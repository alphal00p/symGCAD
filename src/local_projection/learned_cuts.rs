//! Bounded cut hints for unfinished stacks, scoped to one append-only registry.
//! These are never proofs: the caller must reproject every seeded cut and
//! regenerate every sector before accepting a stack. Exact rational prefixes
//! prevent an unrelated fiber from changing the discovery heuristic.
use super::LocalReport;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    sync::Arc,
};
use symbolica::prelude::*;

const MAX_ENTRIES: usize = 4096;
const MAX_IDS: usize = 65_536;
const MAX_ENTRY_IDS: usize = 4096;
const MAX_PREFIX_BITS: u64 = 65_536;

struct Entry {
    ids: Box<[usize]>,
    key_bytes: usize,
    charged_bytes: usize,
}

pub(super) struct LearnedCuts {
    entries: HashMap<Arc<[Rational]>, Entry>,
    fifo: VecDeque<Arc<[Rational]>>,
    budget: usize,
    bytes: usize,
    ids: usize,
    peak_bytes: usize,
    hits: usize,
    misses: usize,
    seeded: usize,
    evictions: usize,
}

fn key_bytes(prefix: &[Rational]) -> Option<usize> {
    let mut bits = 0u64;
    for coordinate in prefix {
        bits = bits
            .saturating_add(coordinate.numerator_ref().significant_bits())
            .saturating_add(coordinate.denominator_ref().significant_bits());
        if bits > MAX_PREFIX_BITS {
            return None;
        }
    }
    // Includes Rational objects, Arc/map/FIFO overhead, and integer limbs.
    Some(
        256usize
            .saturating_add(prefix.len().saturating_mul(128))
            .saturating_add((bits as usize).div_ceil(8)),
    )
}

impl LearnedCuts {
    pub(super) fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            fifo: VecDeque::new(),
            budget,
            bytes: 0,
            ids: 0,
            peak_bytes: 0,
            hits: 0,
            misses: 0,
            seeded: 0,
            evictions: 0,
        }
    }

    pub(super) fn seed(&mut self, prefix: &[Rational], cuts: &mut BTreeSet<usize>) {
        let Some(entry) = self.entries.get(prefix) else {
            self.misses += 1;
            return;
        };
        self.hits += 1;
        let before = cuts.len();
        cuts.extend(entry.ids.iter().copied());
        self.seeded += cuts.len() - before;
    }

    /// Union new hints into one FIFO entry. Hits and updates never append
    /// access records, so neither queue length nor integer-key storage grows
    /// beyond the stated bounds. Oversized hints leave existing data intact.
    pub(super) fn remember(&mut self, prefix: &[Rational], cuts: &BTreeSet<usize>) {
        if self.budget == 0 || cuts.is_empty() || cuts.len() > MAX_ENTRY_IDS {
            return;
        }
        let Some(key_bytes) = key_bytes(prefix) else {
            return;
        };
        let mut ids = cuts.iter().copied().collect::<BTreeSet<_>>();
        if let Some(previous) = self.entries.get(prefix) {
            ids.extend(previous.ids.iter().copied());
            if ids.len() == previous.ids.len() || ids.len() > MAX_ENTRY_IDS {
                return;
            }
        }
        let charged_bytes = key_bytes.saturating_add(ids.len().saturating_mul(size_of::<usize>()));
        if charged_bytes > self.budget {
            return;
        }
        // An updated entry retains its original FIFO position unless budget
        // pressure evicts it. The remaining hints can then be reinserted once.
        loop {
            let previous = self.entries.get(prefix);
            let previous_bytes = previous.map_or(0, |entry| entry.charged_bytes);
            let previous_ids = previous.map_or(0, |entry| entry.ids.len());
            let extra_entry = usize::from(previous.is_none());
            if self.bytes - previous_bytes + charged_bytes <= self.budget
                && self.ids - previous_ids + ids.len() <= MAX_IDS
                && self.entries.len() + extra_entry <= MAX_ENTRIES
            {
                break;
            }
            let Some(oldest) = self.fifo.pop_front() else {
                return;
            };
            let removed = self
                .entries
                .remove(&oldest)
                .expect("FIFO learned-cut entry");
            self.bytes -= removed.charged_bytes;
            self.ids -= removed.ids.len();
            self.evictions += 1;
        }
        if let Some(previous) = self.entries.get_mut(prefix) {
            self.bytes -= previous.charged_bytes;
            self.ids -= previous.ids.len();
            previous.ids = ids.into_iter().collect::<Vec<_>>().into_boxed_slice();
            previous.charged_bytes = previous.key_bytes + previous.ids.len() * size_of::<usize>();
            self.bytes += previous.charged_bytes;
            self.ids += previous.ids.len();
        } else {
            let key: Arc<[Rational]> = prefix.into();
            self.ids += ids.len();
            self.bytes += charged_bytes;
            self.fifo.push_back(key.clone());
            self.entries.insert(
                key,
                Entry {
                    ids: ids.into_iter().collect::<Vec<_>>().into_boxed_slice(),
                    key_bytes,
                    charged_bytes,
                },
            );
        }
        self.peak_bytes = self.peak_bytes.max(self.bytes);
    }

    pub(super) fn report(&self, report: &mut LocalReport) {
        report.learned_cut_cache_hits = self.hits;
        report.learned_cut_cache_misses = self.misses;
        report.learned_cut_seeded_cuts = self.seeded;
        report.learned_cut_cache_evictions = self.evictions;
        report.learned_cut_cache_retained_bytes = self.bytes;
        report.learned_cut_cache_peak_bytes = self.peak_bytes;
        report.learned_cut_cache_retained_ids = self.ids;
        report.learned_cut_cache_retained_entries = self.entries.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_union_only_at_the_exact_prefix_and_do_not_omit_existing_cuts() {
        let mut cache = LearnedCuts::new(4096);
        let prefix = [Rational::from(2)];
        cache.remember(&prefix, &BTreeSet::from([3, 7]));
        cache.remember(&prefix, &BTreeSet::from([7, 11]));
        let mut cuts = BTreeSet::from([1]);
        cache.seed(&[Rational::from(-2)], &mut cuts);
        assert_eq!(cuts, BTreeSet::from([1]));
        cache.seed(&prefix, &mut cuts);
        assert_eq!(cuts, BTreeSet::from([1, 3, 7, 11]));
        assert_eq!(cache.seeded, 3);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.fifo.len(), 1);
    }

    #[test]
    fn fifo_bytes_ids_entries_and_prefix_bits_remain_bounded() {
        let mut cache = LearnedCuts::new(1000);
        for i in 0..10 {
            cache.remember(&[Rational::from(i)], &BTreeSet::from([1, 3, 5]));
        }
        assert!(cache.evictions > 0);
        assert!(cache.bytes <= cache.budget);
        assert!(cache.entries.len() <= MAX_ENTRIES && cache.ids <= MAX_IDS);
        assert_eq!(cache.entries.len(), cache.fifo.len());
        let old_entries = cache.entries.len();
        cache.remember(&[], &(0..=MAX_ENTRY_IDS).collect());
        assert_eq!(cache.entries.len(), old_entries);
        let huge = Rational::from(2).pow(MAX_PREFIX_BITS);
        cache.remember(&[huge], &BTreeSet::from([1]));
        assert_eq!(cache.entries.len(), old_entries);
        assert_eq!(cache.entries.len(), cache.fifo.len());
    }

    #[test]
    fn growing_entries_and_independent_id_and_entry_caps_have_bounded_fifo_storage() {
        let mut cache = LearnedCuts::new(1000);
        cache.remember(&[Rational::from(1)], &BTreeSet::from([1]));
        cache.remember(&[Rational::from(2)], &BTreeSet::from([2]));
        cache.remember(&[Rational::from(1)], &(0..60).collect());
        assert!(cache.bytes <= 1000);
        assert_eq!(cache.entries.len(), cache.fifo.len());
        let mut seeded = BTreeSet::new();
        cache.seed(&[Rational::from(1)], &mut seeded);
        assert_eq!(seeded.len(), 60);
        let mut cache = LearnedCuts::new(8 * 1024 * 1024);
        for i in 0..=MAX_ENTRIES {
            cache.remember(&[Rational::from(i as i64)], &BTreeSet::from([0]));
        }
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert_eq!(cache.fifo.len(), MAX_ENTRIES);
        assert_eq!(cache.evictions, 1);
        let mut cache = LearnedCuts::new(8 * 1024 * 1024);
        for i in 0..18 {
            cache.remember(&[Rational::from(i)], &(0..MAX_ENTRY_IDS).collect());
        }
        assert!(cache.ids <= MAX_IDS);
        assert!(cache.evictions > 0);
        assert_eq!(cache.entries.len(), cache.fifo.len());
    }
}
