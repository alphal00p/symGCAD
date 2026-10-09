//! Per-solve reuse of small completed conditional local proofs.
//!
//! Exact prefixes are the entire key: ancestor intervals may change, but the
//! returned proof carries every obligation needed on the ancestor cell. The
//! registry and problem belong to one Builder and never change meanings.
//!
//! Store bounded serialized payloads, not nested cloned trees. Every entry has
//! at most 128 nodes, 16 levels and 1 MiB of payload; retained payload/key accounting
//! is capped at the caller's <=32 MiB budget and 32768 nodes. FIFO eviction is
//! deliberate: a hit does not grow an unbounded access queue.
use super::{Built, LocalNode, LocalReport};
use anyhow::Result;
use std::{
    collections::{HashMap, VecDeque},
    io::{self, Write},
    sync::Arc,
};
use symbolica::prelude::*;

const MAX_ENTRY_NODES: usize = 128;
// Also keeps cached JSON safely below serde_json's nesting limit.
const MAX_ENTRY_DEPTH: usize = 16;
const MAX_ENTRY_BYTES: usize = 1024 * 1024;
const MAX_RETAINED_NODES: usize = 32768;

struct Entry {
    payload: Box<[u8]>,
    nodes: usize,
    charged_bytes: usize,
}

pub(super) struct CompletedSubtrees {
    entries: HashMap<Arc<[Rational]>, Entry>,
    fifo: VecDeque<Arc<[Rational]>>,
    budget: usize,
    bytes: usize,
    nodes: usize,
    peak_bytes: usize,
    peak_nodes: usize,
    hits: usize,
    misses: usize,
    evictions: usize,
}

struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("subtree cache payload limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn node_count(node: &LocalNode, count: &mut usize, depth: usize) -> bool {
    *count += 1;
    if *count > MAX_ENTRY_NODES || depth > MAX_ENTRY_DEPTH {
        return false;
    }
    if let LocalNode::Stack { sectors, .. } = node {
        return sectors
            .iter()
            .all(|s| node_count(&s.child, count, depth + 1));
    }
    true
}

impl CompletedSubtrees {
    pub(super) fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            fifo: VecDeque::new(),
            budget,
            bytes: 0,
            nodes: 0,
            peak_bytes: 0,
            peak_nodes: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }
    pub(super) fn get(&mut self, prefix: &[Rational]) -> Result<Option<Built>> {
        let Some(entry) = self.entries.get(prefix) else {
            self.misses += 1;
            return Ok(None);
        };
        // Only this process can create these bytes; there is no persistent or
        // external proof cache. Deserialization preserves the complete tree,
        // cell count and required-obligation set without filtering.
        let built = serde_json::from_slice(&entry.payload)?;
        self.hits += 1;
        Ok(Some(built))
    }
    pub(super) fn insert(&mut self, prefix: &[Rational], built: &Built) {
        if self.budget == 0
            || self.entries.contains_key(prefix)
            || !matches!(built.node, LocalNode::Stack { .. })
        {
            return;
        }
        let mut nodes = 0;
        if !node_count(&built.node, &mut nodes, 0) {
            return;
        }
        // Conservative accounting for the Arc, hash/FIFO slots, Rational
        // objects and integer limbs. The FIFO shares its Arc with the map.
        let mut key_bytes = 256usize;
        for coordinate in prefix {
            let bits = coordinate
                .numerator_ref()
                .significant_bits()
                .saturating_add(coordinate.denominator_ref().significant_bits())
                as usize;
            key_bytes = key_bytes
                .saturating_add(128)
                .saturating_add(bits.div_ceil(8));
        }
        let Some(available) = self.budget.checked_sub(key_bytes) else {
            return;
        };
        let mut output = BoundedBytes {
            bytes: Vec::new(),
            limit: available.min(MAX_ENTRY_BYTES),
        };
        if serde_json::to_writer(&mut output, built).is_err() {
            return;
        }
        let charged_bytes = key_bytes + output.bytes.len();
        while self.bytes + charged_bytes > self.budget || self.nodes + nodes > MAX_RETAINED_NODES {
            let Some(key) = self.fifo.pop_front() else {
                return;
            };
            let entry = self.entries.remove(&key).expect("FIFO cache entry");
            self.bytes -= entry.charged_bytes;
            self.nodes -= entry.nodes;
            self.evictions += 1;
        }
        let key: Arc<[Rational]> = prefix.into();
        self.bytes += charged_bytes;
        self.nodes += nodes;
        self.peak_bytes = self.peak_bytes.max(self.bytes);
        self.peak_nodes = self.peak_nodes.max(self.nodes);
        self.fifo.push_back(key.clone());
        self.entries.insert(
            key,
            Entry {
                payload: output.bytes.into_boxed_slice(),
                nodes,
                charged_bytes,
            },
        );
    }
    pub(super) fn report(&self, report: &mut LocalReport) {
        report.subtree_cache_hits = self.hits;
        report.subtree_cache_misses = self.misses;
        report.subtree_cache_evictions = self.evictions;
        report.subtree_cache_retained_bytes = self.bytes;
        report.subtree_cache_peak_bytes = self.peak_bytes;
        report.subtree_cache_retained_nodes = self.nodes;
        report.subtree_cache_peak_nodes = self.peak_nodes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_projection::{LocalProjectionCertificate, LocalSector};
    use std::collections::BTreeSet;

    fn built(children: usize) -> Built {
        Built {
            node: LocalNode::Stack {
                cuts: vec![3],
                projection: LocalProjectionCertificate {
                    base_point: false,
                    root_bearing: vec![3],
                    guards: vec![7],
                    removals: vec![4],
                    positive_ray: None,
                },
                roots: vec![],
                sectors: (0..children)
                    .map(|_| LocalSector {
                        sample: "1/2".into(),
                        child: LocalNode::Accepted { signs: vec![-1] },
                    })
                    .collect(),
                required_below: vec![7, 9],
            },
            required: BTreeSet::from([7, 9]),
            cells: children,
        }
    }
    #[test]
    fn exact_prefix_and_complete_obligations_survive_roundtrip() {
        let mut cache = CompletedSubtrees::new(8192);
        let original = built(2);
        cache.insert(&[Rational::from(2)], &original);
        assert!(cache.get(&[Rational::from(-2)]).unwrap().is_none());
        assert!(cache.get(&[]).unwrap().is_none());
        let restored = cache.get(&[Rational::from(2)]).unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&original).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
    #[test]
    fn fifo_and_payload_node_bounds_limit_retention() {
        let mut cache = CompletedSubtrees::new(1500);
        for i in 0..10 {
            cache.insert(&[Rational::from(i)], &built(2));
        }
        assert!(cache.evictions > 0);
        assert!(cache.bytes <= 1500);
        assert!(cache.get(&[Rational::from(0)]).unwrap().is_none());
        assert!(cache.get(&[Rational::from(9)]).unwrap().is_some());
        let before = cache.entries.len();
        cache.insert(&[Rational::from(10)], &built(128));
        assert_eq!(cache.entries.len(), before);
        let mut deep = built(1);
        for _ in 0..MAX_ENTRY_DEPTH {
            let previous = deep.node;
            deep = built(1);
            if let LocalNode::Stack { sectors, .. } = &mut deep.node {
                sectors[0].child = previous;
            }
        }
        cache.insert(&[Rational::from(12)], &deep);
        assert_eq!(cache.entries.len(), before);
        let mut oversized = built(1);
        if let LocalNode::Stack { sectors, .. } = &mut oversized.node {
            sectors[0].sample = "1".repeat(MAX_ENTRY_BYTES + 1);
        }
        cache.insert(&[Rational::from(11)], &oversized);
        assert_eq!(cache.entries.len(), before);
    }
}
