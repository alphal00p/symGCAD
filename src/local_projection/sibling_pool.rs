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
use std::{cell::Cell, collections::BTreeMap, ops::Bound, rc::Rc};
use symbolica::prelude::*;

/// Scheduling bound only: a full pool drops further completed children, which
/// are then simply revisited. It never limits which sectors must be proved.
const MAX_RETAINED: usize = 65_536;

/// One budget shared by every pool on the active path. A child's weight is the
/// number of proof-tree nodes it retains. A declined child is simply dropped and
/// later revisited (the pre-retention behaviour), so the budget bounds memory
/// without affecting any proof.
pub(super) struct RetentionBudget {
    limit: usize,
    used: Cell<usize>,
    peak: Cell<usize>,
    declined: Cell<usize>,
}

impl RetentionBudget {
    pub(super) fn new(limit: usize) -> Rc<Self> {
        Rc::new(Self {
            limit,
            used: Cell::new(0),
            peak: Cell::new(0),
            declined: Cell::new(0),
        })
    }
    pub(super) fn report(&self, report: &mut super::LocalReport) {
        report.retained_sibling_budget = self.limit;
        report.retained_sibling_peak_weight = self.peak.get();
        report.retained_sibling_budget_drops = self.declined.get();
    }
}

pub(super) struct SiblingPool {
    children: BTreeMap<Rational, (Built, usize)>,
    weight: usize,
    budget: Rc<RetentionBudget>,
}

impl Drop for SiblingPool {
    fn drop(&mut self) {
        self.budget.used.set(self.budget.used.get() - self.weight);
    }
}

impl SiblingPool {
    pub(super) fn new(budget: Rc<RetentionBudget>) -> Self {
        Self {
            children: BTreeMap::new(),
            weight: 0,
            budget,
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.children.len()
    }

    /// Keep one complete child at its exact visited sample, within budget.
    pub(super) fn retain(&mut self, sample: Rational, built: Built) {
        let weight = built.nodes.max(1);
        let used = self.budget.used.get();
        if self.children.len() >= MAX_RETAINED
            || self.children.contains_key(&sample)
            || used.saturating_add(weight) > self.budget.limit
        {
            self.budget.declined.set(self.budget.declined.get() + 1);
            return;
        }
        self.children.insert(sample, (built, weight));
        self.weight += weight;
        self.budget.used.set(used + weight);
        self.budget.peak.set(self.budget.peak.get().max(used + weight));
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
        let (built, weight) = self.children.remove(&sample)?;
        self.weight -= weight;
        self.budget.used.set(self.budget.used.get() - weight);
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

    fn leaf(nodes: usize) -> Built {
        Built {
            node: LocalNode::Accepted { signs: vec![] },
            required: BTreeSet::new(),
            cells: nodes,
            nodes,
        }
    }

    #[test]
    fn strict_membership_selects_one_child_and_moves_it() {
        let mut pool = SiblingPool::new(RetentionBudget::new(usize::MAX));
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

    #[test]
    fn shared_budget_declines_children_and_is_released_on_take_and_drop() {
        let budget = RetentionBudget::new(10);
        let mut outer = SiblingPool::new(budget.clone());
        outer.retain(Rational::from(1), leaf(5));
        {
            let mut inner = SiblingPool::new(budget.clone());
            inner.retain(Rational::from(2), leaf(5));
            // 5 + 5 = 10 is within the limit; one more child is declined.
            inner.retain(Rational::from(3), leaf(1));
            assert_eq!(inner.len(), 1);
            assert_eq!(budget.used.get(), 10);
        }
        // Dropping the inner pool releases its weight.
        assert_eq!(budget.used.get(), 5);
        let (zero, three) = (root(0, 0, 0), root(1, 3, 3));
        assert!(outer.take_inside(Some(&zero), Some(&three)).is_some());
        assert_eq!(budget.used.get(), 0);
        assert_eq!(budget.peak.get(), 10);
        assert_eq!(budget.declined.get(), 1);
    }
}
