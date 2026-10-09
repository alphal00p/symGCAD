//! Bounded, per-union reuse of exact coprimality decisions.
//!
//! Keys identify input fibers inside one union call, not user polynomial IDs.
//! A few exact bisections can separate broad intervals without computing a
//! polynomial GCD. After a finite number of attempts the ordinary exact test
//! is mandatory, so coincident irrational roots still terminate.
use super::{FiberRoot, roots_in_open_interval};
use std::collections::{HashSet, VecDeque};

const MAX_PAIRS: usize = 8192;

pub(super) struct CollisionCache {
    pairs: HashSet<(usize, usize)>,
    fifo: VecDeque<(usize, usize)>,
    limit: usize,
}

impl CollisionCache {
    pub(super) fn new() -> Self {
        Self {
            pairs: HashSet::new(),
            fifo: VecDeque::new(),
            limit: MAX_PAIRS,
        }
    }

    fn remember(&mut self, key: (usize, usize)) {
        if self.pairs.contains(&key) {
            return;
        }
        if self.pairs.len() == self.limit {
            let oldest = self.fifo.pop_front().expect("nonempty collision cache");
            self.pairs.remove(&oldest);
        }
        self.fifo.push_back(key);
        self.pairs.insert(key);
    }

    pub(super) fn same_root(
        &mut self,
        a: &FiberRoot,
        a_source: usize,
        b: &FiberRoot,
        b_source: usize,
        defer_gcd: bool,
    ) -> bool {
        let lo = a.interval.0.clone().max(b.interval.0.clone());
        let hi = a.interval.1.clone().min(b.interval.1.clone());
        if lo > hi {
            return false;
        }
        if lo == hi {
            return super::integer_sign::sign(&a.fiber, &lo) == 0
                && super::integer_sign::sign(&b.fiber, &lo) == 0
                && (a.interval.0 == a.interval.1 || (a.interval.0 < lo && lo < a.interval.1))
                && (b.interval.0 == b.interval.1 || (b.interval.0 < lo && lo < b.interval.1));
        }
        // Isolation gives distinct indexed roots of each individual fiber.
        // The same source/index is never inserted twice in the pending heap.
        if a_source == b_source {
            debug_assert_ne!(a.index, b.index);
            return false;
        }
        let key = (a_source.min(b_source), a_source.max(b_source));
        if self.pairs.contains(&key) || defer_gcd {
            return false;
        }
        let gcd = a.fiber.gcd(&b.fiber);
        if gcd.degree() == 0 {
            self.remember(key);
            false
        } else {
            // A common polynomial factor need not define these two roots.
            roots_in_open_interval(&gcd, &lo, &hi) != 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_storage_stays_bounded_and_hits_do_not_grow_it() {
        let mut cache = CollisionCache::new();
        cache.limit = 4;
        for i in 0..32 {
            cache.remember((i, i + 1));
            assert!(cache.pairs.len() <= 4);
            assert_eq!(cache.pairs.len(), cache.fifo.len());
        }
        for _ in 0..100 {
            cache.remember((31, 32));
        }
        assert_eq!(cache.pairs.len(), 4);
        assert_eq!(cache.fifo.len(), 4);
        assert!(!cache.pairs.contains(&(0, 1)));
    }
}
