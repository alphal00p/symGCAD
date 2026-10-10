//! Read-only, per-Builder observations of failed visits returning Stop::Guard.
//!
//! A propagated failure is observed at each returning visit, so these are not
//! counts of independent originating failures or of discarded descendant work.
//! Exact keys are retained in a bounded FIFO window. An evicted key may later
//! be admitted again; only `repeated_while_retained` is a repeat guarantee.
//! Nothing in this module supplies a guard, cut, cached proof, or solver choice.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use symbolica::prelude::*;

const BYTE_LIMIT: usize = 1024 * 1024;
const ENTRY_LIMIT: usize = 4096;
const PREFIX_LIMIT: usize = 64;
const PREFIX_BITS: u64 = 16_384;
const SAMPLE_LIMIT: usize = 32;
const SAMPLE_BYTE_LIMIT: usize = 64 * 1024;
const SAMPLE_PREFIX_BITS: u64 = 1024;
static NEXT_SCOPE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerObservations {
    pub owner_axis: usize,
    pub failed_visits: usize,
    pub first_seen_or_evicted: usize,
    pub repeated_while_retained: usize,
    pub skipped_prefix_limit: usize,
    pub evictions: usize,
    pub retained_keys: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObstructionSample {
    /// This ID has meaning only within this report's single Builder scope.
    pub polynomial: usize,
    pub owner_axis: usize,
    pub failed_level: usize,
    pub prefix: Vec<String>,
    pub first_event: usize,
    pub last_event: usize,
    /// Further matching events while this particular admission was retained.
    pub repeated_while_retained: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObstructionReport {
    pub schema_version: u32,
    pub process_id: u32,
    pub builder_scope: u64,
    pub semantics: String,
    pub failed_visits: usize,
    pub first_seen_or_evicted: usize,
    pub repeated_while_retained: usize,
    pub skipped_prefix_limit: usize,
    pub invalid_owner: usize,
    pub evictions: usize,
    pub retained_keys: usize,
    pub peak_keys: usize,
    /// Conservative charged key/entry bytes, not a process-RSS measurement.
    pub retained_bytes: usize,
    pub peak_bytes: usize,
    pub byte_limit: usize,
    pub entry_limit: usize,
    pub prefix_limit: usize,
    pub prefix_bit_limit: u64,
    pub sample_bytes: usize,
    pub sample_byte_limit: usize,
    pub sample_limit: usize,
    pub samples_skipped: usize,
    pub by_owner: Vec<OwnerObservations>,
    pub samples: Vec<ObstructionSample>,
    pub writes: usize,
    pub write_failures: usize,
    pub final_snapshot: bool,
}

struct Entry {
    charged_bytes: usize,
    owner_axis: usize,
    sample: Option<usize>,
}

pub(super) struct Observer {
    // HashMap uses full Rational slice equality even on hash collisions.
    // Different IDs at an identical prefix remain separate observations.
    entries: HashMap<Arc<[Rational]>, BTreeMap<usize, Entry>>,
    fifo: VecDeque<(Arc<[Rational]>, usize)>,
    report: ObstructionReport,
    path: Option<PathBuf>,
}

fn prefix_bits(prefix: &[Rational]) -> Option<u64> {
    if prefix.len() > PREFIX_LIMIT {
        return None;
    }
    let mut bits = 0u64;
    for value in prefix {
        bits = bits
            .checked_add(value.numerator_ref().significant_bits())?
            .checked_add(value.denominator_ref().significant_bits())?;
        if bits > PREFIX_BITS {
            return None;
        }
    }
    Some(bits)
}

impl Observer {
    pub(super) fn new(path: Option<PathBuf>) -> Self {
        Self::with_limits(path, BYTE_LIMIT, ENTRY_LIMIT)
    }

    fn with_limits(path: Option<PathBuf>, byte_limit: usize, entry_limit: usize) -> Self {
        Self {
            entries: HashMap::new(),
            fifo: VecDeque::new(),
            report: ObstructionReport {
                schema_version: 1,
                process_id: std::process::id(),
                builder_scope: NEXT_SCOPE.fetch_add(1, Ordering::Relaxed),
                semantics: "failed visits including propagated guards; repeats only while retained; no discarded-work estimate".into(),
                failed_visits: 0,
                first_seen_or_evicted: 0,
                repeated_while_retained: 0,
                skipped_prefix_limit: 0,
                invalid_owner: 0,
                evictions: 0,
                retained_keys: 0,
                peak_keys: 0,
                retained_bytes: 0,
                peak_bytes: 0,
                byte_limit,
                entry_limit,
                prefix_limit: PREFIX_LIMIT,
                prefix_bit_limit: PREFIX_BITS,
                sample_bytes: 0,
                sample_byte_limit: SAMPLE_BYTE_LIMIT,
                sample_limit: SAMPLE_LIMIT,
                samples_skipped: 0,
                by_owner: Vec::new(),
                samples: Vec::new(),
                writes: 0,
                write_failures: 0,
                final_snapshot: false,
            },
            path,
        }
    }

    fn owner(&mut self, axis: usize) -> &mut OwnerObservations {
        // All callers enforce axis < PREFIX_LIMIT. No unbounded map of axes.
        let index = match self
            .report
            .by_owner
            .binary_search_by_key(&axis, |row| row.owner_axis)
        {
            Ok(index) => index,
            Err(index) => {
                self.report.by_owner.insert(
                    index,
                    OwnerObservations {
                        owner_axis: axis,
                        ..Default::default()
                    },
                );
                index
            }
        };
        &mut self.report.by_owner[index]
    }

    /// `polynomial` is an existing nonzero canonical registry ID; the caller
    /// supplies its exact highest axis. The observer does not reevaluate it or
    /// infer a new obstruction: this is solely an already-returned Guard event.
    pub(super) fn observe(&mut self, prefix: &[Rational], polynomial: usize, axis: usize) {
        self.report.failed_visits = self.report.failed_visits.saturating_add(1);
        if axis >= prefix.len() || axis >= PREFIX_LIMIT {
            self.report.invalid_owner = self.report.invalid_owner.saturating_add(1);
            return;
        }
        let row = self.owner(axis);
        row.failed_visits = row.failed_visits.saturating_add(1);
        let Some(bits) = prefix_bits(prefix) else {
            self.skip(axis);
            return;
        };
        if let Some(entry) = self
            .entries
            .get(prefix)
            .and_then(|ids| ids.get(&polynomial))
        {
            let sample = entry.sample;
            self.report.repeated_while_retained =
                self.report.repeated_while_retained.saturating_add(1);
            let row = self.owner(axis);
            row.repeated_while_retained = row.repeated_while_retained.saturating_add(1);
            if let Some(index) = sample {
                let row = &mut self.report.samples[index];
                row.last_event = self.report.failed_visits;
                row.repeated_while_retained = row.repeated_while_retained.saturating_add(1);
            }
            return;
        }
        // Charge even shared prefix storage per admitted key. Includes Rational
        // objects/limbs, Arc/map/FIFO and per-ID tree entries conservatively.
        let charged_bytes = 256 + 128 * prefix.len() + (bits as usize).div_ceil(8);
        if charged_bytes > self.report.byte_limit || self.report.entry_limit == 0 {
            self.skip(axis);
            return;
        }
        while self.report.retained_keys >= self.report.entry_limit
            || self.report.retained_bytes > self.report.byte_limit - charged_bytes
        {
            self.evict();
        }
        let key = self
            .entries
            .get_key_value(prefix)
            .map(|(key, _)| Arc::clone(key))
            .unwrap_or_else(|| Arc::from(prefix));
        let sample = self.sample(prefix, polynomial, axis, bits);
        self.entries.entry(Arc::clone(&key)).or_default().insert(
            polynomial,
            Entry {
                charged_bytes,
                owner_axis: axis,
                sample,
            },
        );
        self.fifo.push_back((key, polynomial));
        self.report.first_seen_or_evicted = self.report.first_seen_or_evicted.saturating_add(1);
        self.report.retained_keys += 1;
        self.report.retained_bytes += charged_bytes;
        self.report.peak_keys = self.report.peak_keys.max(self.report.retained_keys);
        self.report.peak_bytes = self.report.peak_bytes.max(self.report.retained_bytes);
        let row = self.owner(axis);
        row.first_seen_or_evicted = row.first_seen_or_evicted.saturating_add(1);
        row.retained_keys += 1;
    }

    fn skip(&mut self, axis: usize) {
        self.report.skipped_prefix_limit = self.report.skipped_prefix_limit.saturating_add(1);
        let row = self.owner(axis);
        row.skipped_prefix_limit = row.skipped_prefix_limit.saturating_add(1);
    }

    fn evict(&mut self) {
        let (prefix, polynomial) = self.fifo.pop_front().expect("observer FIFO entry");
        let ids = self
            .entries
            .get_mut(prefix.as_ref())
            .expect("observer prefix");
        let entry = ids.remove(&polynomial).expect("observer polynomial");
        if ids.is_empty() {
            self.entries.remove(prefix.as_ref());
        }
        self.report.retained_keys -= 1;
        self.report.retained_bytes -= entry.charged_bytes;
        self.report.evictions = self.report.evictions.saturating_add(1);
        let row = self.owner(entry.owner_axis);
        row.retained_keys -= 1;
        row.evictions = row.evictions.saturating_add(1);
    }

    fn sample(
        &mut self,
        prefix: &[Rational],
        polynomial: usize,
        axis: usize,
        bits: u64,
    ) -> Option<usize> {
        if self.report.samples.len() >= SAMPLE_LIMIT || bits > SAMPLE_PREFIX_BITS {
            self.report.samples_skipped = self.report.samples_skipped.saturating_add(1);
            return None;
        }
        // Rational text is formed only after the tight bit bound above.
        let values: Vec<String> = prefix.iter().map(ToString::to_string).collect();
        let bytes = 256 + prefix.len() * 64 + values.iter().map(String::len).sum::<usize>();
        if bytes > SAMPLE_BYTE_LIMIT - self.report.sample_bytes {
            self.report.samples_skipped = self.report.samples_skipped.saturating_add(1);
            return None;
        }
        self.report.sample_bytes += bytes;
        let index = self.report.samples.len();
        self.report.samples.push(ObstructionSample {
            polynomial,
            owner_axis: axis,
            failed_level: prefix.len(),
            prefix: values,
            first_event: self.report.failed_visits,
            last_event: self.report.failed_visits,
            repeated_while_retained: 0,
        });
        Some(index)
    }

    pub(super) fn snapshot(&self) -> ObstructionReport {
        self.report.clone()
    }

    /// Best effort: diagnostic path/serialization failures never change a
    /// solve's Result, cuts, cache contents, or proof. The next snapshot carries
    /// failure counts. The payload is bounded by the fixed sample/owner caps.
    pub(super) fn publish(&mut self, final_snapshot: bool) {
        self.report.final_snapshot = final_snapshot;
        let Some(path) = &self.path else { return };
        let temporary = path.with_extension(format!(
            "obstruction-{}-{}.tmp",
            self.report.process_id, self.report.builder_scope
        ));
        let result = (|| -> anyhow::Result<()> {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&temporary, serde_json::to_vec(&self.report)?)?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_ok() {
            self.report.writes = self.report.writes.saturating_add(1);
        } else {
            self.report.write_failures = self.report.write_failures.saturating_add(1);
            let _ = std::fs::remove_file(temporary);
        }
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        self.publish(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_prefix_and_polynomial_are_both_part_of_the_key() {
        let mut observer = Observer::new(None);
        let a = [Rational::from((1, 2)), Rational::from(3)];
        let b = [Rational::from((1, 3)), Rational::from(3)];
        observer.observe(&a, 7, 0);
        observer.observe(&b, 7, 0);
        observer.observe(&a, 8, 1);
        observer.observe(&[Rational::from((2, 4)), Rational::from(3)], 7, 0);
        let report = observer.snapshot();
        assert_eq!(report.first_seen_or_evicted, 3);
        assert_eq!(report.repeated_while_retained, 1);
        assert_eq!(report.by_owner[0].repeated_while_retained, 1);
        assert_eq!(report.by_owner[1].repeated_while_retained, 0);
        assert_eq!(report.samples[0].last_event, 4);
    }

    #[test]
    fn fifo_eviction_does_not_claim_global_distinct_counts() {
        let mut observer = Observer::with_limits(None, 4096, 2);
        let prefix = [Rational::one()];
        for id in [0, 1, 0, 2, 0] {
            observer.observe(&prefix, id, 0);
        }
        let report = observer.snapshot();
        assert_eq!(report.failed_visits, 5);
        assert_eq!(report.first_seen_or_evicted, 4);
        assert_eq!(report.repeated_while_retained, 1);
        assert_eq!(report.evictions, 2);
        assert_eq!(report.retained_keys, 2);
        assert_eq!(report.by_owner[0].retained_keys, 2);
        assert!(report.peak_bytes <= 4096);
    }

    #[test]
    fn bytes_prefixes_owners_and_samples_remain_bounded() {
        let mut observer = Observer::with_limits(None, 1024, 10);
        for id in 0..100 {
            observer.observe(&[Rational::from(id)], id, 0);
        }
        observer.observe(&vec![Rational::one(); PREFIX_LIMIT + 1], 100, 0);
        observer.observe(&[Rational::from(2).pow(PREFIX_BITS)], 101, 0);
        observer.observe(&[], 102, 0);
        let report = observer.snapshot();
        assert!(report.peak_bytes <= 1024);
        assert!(report.peak_keys <= 10);
        assert!(report.samples.len() <= SAMPLE_LIMIT);
        assert!(report.sample_bytes <= SAMPLE_BYTE_LIMIT);
        assert_eq!(report.skipped_prefix_limit, 2);
        assert_eq!(report.invalid_owner, 1);
        assert_eq!(report.by_owner.len(), 1);
        assert_eq!(observer.fifo.len(), report.retained_keys);
    }

    #[test]
    fn final_atomic_diagnostic_and_io_failure_are_best_effort() {
        let dir = std::env::temp_dir().join(format!(
            "symgcad-obstruction-{}-{}",
            std::process::id(),
            NEXT_SCOPE.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join("report.json");
        {
            let mut observer = Observer::new(Some(path.clone()));
            observer.observe(&[Rational::one()], 1, 0);
            observer.publish(false);
            let report: ObstructionReport =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            assert!(!report.final_snapshot);
        }
        let report: ObstructionReport =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(report.final_snapshot);
        assert_eq!(report.failed_visits, 1);
        let mut failed = Observer::new(Some(path.join("cannot-be-child-of-file")));
        failed.publish(false);
        assert_eq!(failed.snapshot().write_failures, 1);
        drop(failed);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
