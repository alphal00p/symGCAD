//! Completed sibling proofs retained across refinement rounds of ONE stack visit.
//!
//! Within a visit the exact ancestor prefix is fixed and the cut set only grows,
//! so every new sector lies inside exactly one sector of an earlier round. A
//! completed child `Built` at `(prefix, s)` is a conditional proof: it holds on
//! every connected sector containing `s` on which its returned obligations are
//! sign-invariant (the conditional extension lemma in
//! `reference/LOCAL_PROJECTION_LIVENESS.md`). The caller merges the child's
//! obligations exactly as for a freshly visited child, so the final projection
//! must contain all of its same-axis requirements and the parent must discharge
//! all of its lower-axis ones. Only strict membership of the unchanged rational
//! sample in a newly isolated sector selects a child; an uncertain comparison
//! declines reuse. The exported tree keeps the original sample, so this needs the
//! strict-interior sample policy (`stable_sector_samples`) in the checker, which
//! independently replays every reused subtree at its exact prefix.
use super::{Built, stack_samples};
use crate::roots::FiberRoot;
use std::{collections::BTreeMap, ops::Bound};
use symbolica::prelude::*;

/// Scheduling bound only: a full pool drops further completed children, which
/// are then simply revisited. It never limits which sectors must be proved.
const MAX_RETAINED: usize = 65_536;

#[derive(Default)]
pub(super) struct SiblingPool {
    children: BTreeMap<Rational, Built>,
}

impl SiblingPool {
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.children.len()
    }

    /// Keep one complete child at its exact visited sample.
    pub(super) fn retain(&mut self, sample: Rational, built: Built) {
        if self.children.len() < MAX_RETAINED {
            self.children.entry(sample).or_insert(built);
        }
    }

    /// Move out a retained child whose sample lies strictly inside the open
    /// sector `(lower, upper)`. Candidates outside the coarse interval hull are
    /// skipped without refinement; each remaining one is checked exactly.
    pub(super) fn take_inside(
        &mut self,
        lower: Option<&FiberRoot>,
        upper: Option<&FiberRoot>,
    ) -> Option<(Rational, Built)> {
        let low = lower.map_or(Bound::Unbounded, |root| Bound::Excluded(&root.interval.0));
        let high = upper.map_or(Bound::Unbounded, |root| Bound::Excluded(&root.interval.1));
        if let (Bound::Excluded(low), Bound::Excluded(high)) = (low, high) {
            if low >= high {
                return None;
            }
        }
        let sample = self
            .children
            .range::<Rational, _>((low, high))
            .map(|(sample, _)| sample)
            .find(|sample| stack_samples::strictly_inside(sample, lower, upper))?
            .clone();
        let built = self.children.remove(&sample)?;
        Some((sample, built))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_projection::LocalNode;
    use std::{collections::BTreeSet, sync::Arc};

    fn root(polynomial: usize, low: i64, high: i64) -> FiberRoot {
        // Fiber (z-low)(z-high) is irrelevant for point roots; use a linear
        // polynomial with the exact root for point intervals.
        let algebra = crate::algebra::Algebra::new(&["z".into()]).unwrap();
        let fiber = algebra
            .parse(&format!("z-{low}"))
            .unwrap()
            .to_univariate_from_univariate(0);
        FiberRoot {
            polynomial,
            index: 0,
            interval: (Rational::from(low), Rational::from(high)),
            fiber: Arc::new(fiber),
        }
    }

    fn leaf(cells: usize) -> Built {
        Built {
            node: LocalNode::Accepted { signs: vec![] },
            required: BTreeSet::new(),
            cells,
        }
    }

    #[test]
    fn strict_membership_selects_one_child_and_moves_it() {
        let mut pool = SiblingPool::default();
        pool.retain(Rational::from(1), leaf(1));
        pool.retain(Rational::from(5), leaf(5));
        // Point roots at 0 and 3: only sample 1 lies strictly inside (0,3).
        let (zero, three) = (root(0, 0, 0), root(1, 3, 3));
        let (sample, built) = pool.take_inside(Some(&zero), Some(&three)).unwrap();
        assert_eq!(sample, Rational::from(1));
        assert_eq!(built.cells, 1);
        assert!(pool.take_inside(Some(&zero), Some(&three)).is_none());
        // A sample exactly on a new boundary is never reused.
        let five = root(2, 5, 5);
        assert!(pool.take_inside(Some(&three), Some(&five)).is_none());
        assert!(pool.take_inside(Some(&five), None).is_none());
        assert_eq!(pool.len(), 1);
        let (sample, _) = pool.take_inside(Some(&three), None).unwrap();
        assert_eq!(sample, Rational::from(5));
    }
}
