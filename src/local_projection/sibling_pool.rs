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
use super::{Builder, Built, stack_samples};
use crate::roots::FiberRoot;
use std::{cell::Cell, collections::BTreeMap, ops::Bound, rc::Rc};
use symbolica::prelude::*;

/// Scheduling bound only: a full pool drops further completed children, which
/// are then simply revisited. It never limits which sectors must be proved.
const MAX_RETAINED: usize = 65_536;

/// Outcome of offering one completed child to a pool. No outcome affects which
/// sectors are proved: a child that is not kept is revisited when needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Retention {
    Kept,
    /// A child at the same exact sample is already retained.
    Duplicate,
    /// The per-pool count bound is reached.
    Capacity,
    /// The shared node budget cannot hold this child.
    OverBudget,
}

impl Retention {
    /// Memory refusals: the child will be revisited, so the round must not
    /// keep collecting siblings beyond the ordinary discovery thresholds.
    pub(super) fn refused(self) -> bool {
        matches!(self, Self::Capacity | Self::OverBudget)
    }
}

/// One budget shared by every pool on the active path. A child's weight is the
/// number of proof-tree nodes it retains. A declined child is simply dropped and
/// later revisited (the pre-retention behaviour), so the budget bounds memory
/// without affecting any proof.
pub(super) struct RetentionBudget {
    limit: usize,
    used: Cell<usize>,
    peak: Cell<usize>,
    over_budget: Cell<usize>,
    duplicates: Cell<usize>,
    capacity: Cell<usize>,
}

impl RetentionBudget {
    pub(super) fn new(limit: usize) -> Rc<Self> {
        Rc::new(Self {
            limit,
            used: Cell::new(0),
            peak: Cell::new(0),
            over_budget: Cell::new(0),
            duplicates: Cell::new(0),
            capacity: Cell::new(0),
        })
    }
    fn has_room(&self) -> bool {
        self.used.get() < self.limit
    }
    fn count(cell: &Cell<usize>) {
        cell.set(cell.get() + 1);
    }
    pub(super) fn report(&self, report: &mut super::LocalReport) {
        report.retained_sibling_budget = self.limit;
        report.retained_sibling_peak_weight = self.peak.get();
        report.retained_sibling_budget_drops = self.over_budget.get();
        report.retained_sibling_duplicate_drops = self.duplicates.get();
        report.retained_sibling_capacity_drops = self.capacity.get();
    }
}

pub(super) struct SiblingPool {
    children: BTreeMap<Rational, (Built, usize)>,
    weight: usize,
    capacity: usize,
    budget: Rc<RetentionBudget>,
}

impl Drop for SiblingPool {
    fn drop(&mut self) {
        self.budget.used.set(self.budget.used.get() - self.weight);
    }
}

impl SiblingPool {
    pub(super) fn new(budget: Rc<RetentionBudget>) -> Self {
        Self::with_capacity(budget, MAX_RETAINED)
    }

    fn with_capacity(budget: Rc<RetentionBudget>, capacity: usize) -> Self {
        Self {
            children: BTreeMap::new(),
            weight: 0,
            capacity,
            budget,
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.children.len()
    }

    /// Whether a further child of positive weight could still be kept.
    pub(super) fn accepting(&self) -> bool {
        self.children.len() < self.capacity && self.budget.has_room()
    }

    /// Keep one complete child at its exact visited sample, within budget.
    pub(super) fn retain(&mut self, sample: Rational, built: Built) -> Retention {
        let weight = built.nodes.max(1);
        let used = self.budget.used.get();
        let outcome = if self.children.contains_key(&sample) {
            Retention::Duplicate
        } else if self.children.len() >= self.capacity {
            Retention::Capacity
        } else if used.saturating_add(weight) > self.budget.limit {
            Retention::OverBudget
        } else {
            Retention::Kept
        };
        match outcome {
            Retention::Kept => {
                self.children.insert(sample, (built, weight));
                self.weight += weight;
                self.budget.used.set(used + weight);
                self.budget.peak.set(self.budget.peak.get().max(used + weight));
            }
            Retention::Duplicate => RetentionBudget::count(&self.budget.duplicates),
            Retention::Capacity => RetentionBudget::count(&self.budget.capacity),
            Retention::OverBudget => RetentionBudget::count(&self.budget.over_budget),
        }
        outcome
    }

    /// Move out a retained child whose sample lies strictly inside the open
    /// sector `(lower, upper)`. Candidates outside the coarse interval hull are
    /// skipped without refinement; each remaining one is checked exactly, since
    /// a candidate inside the hull may still lie on the far side of a root.
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

impl Builder<'_> {
    /// A complete child of a round that will be restarted. With retention it
    /// is offered to the pool; otherwise, or when the pool declines it, it is
    /// discarded and its sector is visited again in a later round. Returns
    /// whether the round may keep collecting siblings past the ordinary
    /// discovery thresholds: only while children are actually being retained.
    pub(super) fn set_aside(
        &mut self,
        pool: Option<&mut SiblingPool>,
        sample: Rational,
        child: Built,
    ) -> bool {
        let cells = child.cells;
        let (kept, retaining) = match pool {
            Some(pool) => {
                let outcome = pool.retain(sample, child);
                (
                    outcome == Retention::Kept,
                    !outcome.refused() && pool.accepting(),
                )
            }
            None => (false, false),
        };
        if let Some(diag) = &mut self.thrash {
            if !kept {
                diag.report.discarded_completed_children += 1;
                diag.report.discarded_completed_cells += cells;
            }
        }
        retaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_projection::LocalNode;
    use std::{collections::BTreeSet, sync::Arc};

    /// Root of `fiber` (in `z`) isolated by `(low, high)`.
    fn isolated(polynomial: usize, fiber: &str, low: Rational, high: Rational) -> FiberRoot {
        let algebra = crate::algebra::Algebra::new(&["z".into()]).unwrap();
        let fiber = algebra
            .parse(fiber)
            .unwrap()
            .to_univariate_from_univariate(0);
        FiberRoot {
            polynomial,
            index: 0,
            interval: (low, high),
            fiber: Arc::new(fiber),
        }
    }

    /// Exact point root `value` of `z - value`.
    fn root(polynomial: usize, value: i64) -> FiberRoot {
        isolated(
            polynomial,
            &format!("z-{value}"),
            Rational::from(value),
            Rational::from(value),
        )
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
        assert_eq!(pool.retain(Rational::from(1), leaf(1)), Retention::Kept);
        assert_eq!(pool.retain(Rational::from(5), leaf(5)), Retention::Kept);
        // Point roots at 0 and 3: only sample 1 lies strictly inside (0,3).
        let (zero, three) = (root(0, 0), root(1, 3));
        let (sample, built) = pool.take_inside(Some(&zero), Some(&three)).unwrap();
        assert_eq!(sample, Rational::from(1));
        assert_eq!(built.cells, 1);
        assert!(pool.take_inside(Some(&zero), Some(&three)).is_none());
        // A sample exactly on a new boundary is never reused.
        let five = root(2, 5);
        assert!(pool.take_inside(Some(&three), Some(&five)).is_none());
        assert!(pool.take_inside(Some(&five), None).is_none());
        assert_eq!(pool.len(), 1);
        let (sample, _) = pool.take_inside(Some(&three), None).unwrap();
        assert_eq!(sample, Rational::from(5));
    }

    #[test]
    fn candidates_inside_a_wide_isolating_interval_are_checked_exactly() {
        // sqrt(5) ~ 2.2361 isolated only by (2,3): both retained samples lie
        // inside the coarse hull (2, inf), but 11/5 is below the root.
        let sqrt5 = || isolated(0, "z^2-5", Rational::from(2), Rational::from(3));
        let (below, above) = (Rational::from((11, 5)), Rational::from((9, 4)));
        let mut pool = SiblingPool::new(RetentionBudget::new(usize::MAX));
        pool.retain(below.clone(), leaf(1));
        pool.retain(above.clone(), leaf(2));
        let (sample, built) = pool.take_inside(Some(&sqrt5()), None).unwrap();
        assert_eq!(sample, above);
        assert_eq!(built.cells, 2);
        // The remaining candidate is inside the hull but not inside the sector.
        assert!(pool.take_inside(Some(&sqrt5()), None).is_none());
        assert_eq!(pool.len(), 1);
        // Symmetrically for an upper root: only 11/5 lies below sqrt(5).
        let (sample, _) = pool.take_inside(None, Some(&sqrt5())).unwrap();
        assert_eq!(sample, below);
        // A sample at an isolating endpoint is ordered by the interval alone.
        pool.retain(Rational::from(2), leaf(1));
        assert!(pool.take_inside(Some(&sqrt5()), None).is_none());
        assert!(pool.take_inside(None, Some(&sqrt5())).is_some());
    }

    #[test]
    fn shared_budget_declines_children_and_is_released_on_take_and_drop() {
        let budget = RetentionBudget::new(10);
        let mut outer = SiblingPool::new(budget.clone());
        assert_eq!(outer.retain(Rational::from(1), leaf(5)), Retention::Kept);
        {
            let mut inner = SiblingPool::new(budget.clone());
            assert_eq!(inner.retain(Rational::from(2), leaf(5)), Retention::Kept);
            // 5 + 5 = 10 is the limit: no room for anything else.
            assert!(!inner.accepting());
            assert_eq!(
                inner.retain(Rational::from(3), leaf(1)),
                Retention::OverBudget
            );
            assert!(Retention::OverBudget.refused());
            assert_eq!(inner.len(), 1);
            assert_eq!(budget.used.get(), 10);
        }
        // Dropping the inner pool releases its weight.
        assert_eq!(budget.used.get(), 5);
        assert!(outer.accepting());
        let (zero, three) = (root(0, 0), root(1, 3));
        assert!(outer.take_inside(Some(&zero), Some(&three)).is_some());
        assert_eq!(budget.used.get(), 0);
        assert_eq!(budget.peak.get(), 10);
        assert_eq!(budget.over_budget.get(), 1);
        assert_eq!(budget.duplicates.get(), 0);
        assert_eq!(budget.capacity.get(), 0);
    }

    #[test]
    fn duplicate_and_capacity_declines_are_reported_separately() {
        let budget = RetentionBudget::new(usize::MAX);
        let mut pool = SiblingPool::with_capacity(budget.clone(), 2);
        assert_eq!(pool.retain(Rational::from(1), leaf(1)), Retention::Kept);
        // A duplicate sample is not a memory refusal: the pool keeps a proof
        // at that sample and can still accept others.
        let duplicate = pool.retain(Rational::from(1), leaf(1));
        assert_eq!(duplicate, Retention::Duplicate);
        assert!(!duplicate.refused());
        assert!(pool.accepting());
        assert_eq!(pool.retain(Rational::from(2), leaf(1)), Retention::Kept);
        assert!(!pool.accepting());
        let full = pool.retain(Rational::from(3), leaf(1));
        assert_eq!(full, Retention::Capacity);
        assert!(full.refused());
        let mut report = crate::local_projection::LocalReport::default();
        budget.report(&mut report);
        assert_eq!(report.retained_sibling_duplicate_drops, 1);
        assert_eq!(report.retained_sibling_capacity_drops, 1);
        assert_eq!(report.retained_sibling_budget_drops, 0);
    }
}
