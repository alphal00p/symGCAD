//! Bounded rational sampling hints for one stack at one fixed ancestor prefix.
//!
//! Nothing here is a proof cache. A previously visited rational is reused only
//! after strict membership in a newly isolated sector has been proved exactly.
//! The ordinary child visitor and every conditional obligation still run.
use crate::roots::{FiberRoot, refine_once};
use std::{
    cmp::Ordering,
    collections::{BTreeSet, VecDeque},
};
use symbolica::prelude::*;

const MAX_SAMPLES: usize = 256;
const MAX_BYTES: usize = 32 * 1024;
const MAX_BITS: u64 = 1024;
const MAX_REFINEMENTS: usize = 128;

pub(super) struct StackSamples {
    samples: BTreeSet<Rational>,
    fifo: VecDeque<Rational>,
    bytes: usize,
}

fn charge(sample: &Rational) -> Option<usize> {
    let bits = sample
        .numerator_ref()
        .significant_bits()
        .checked_add(sample.denominator_ref().significant_bits())?;
    if bits > MAX_BITS {
        return None;
    }
    // Two owned Rational values (set/FIFO), their limbs, and container slots.
    Some(256 + 2 * (bits as usize).div_ceil(8))
}

/// Compare to the target root, rather than any other root of its polynomial.
/// Non-point intervals are open at both ends; a different root can lie on an
/// endpoint. Test the interval before evaluating the polynomial for this reason.
fn compare(sample: &Rational, root: &mut FiberRoot, budget: &mut usize) -> Option<Ordering> {
    if root.interval.0 == root.interval.1 {
        return Some(sample.cmp(&root.interval.0));
    }
    if sample <= &root.interval.0 {
        return Some(Ordering::Less);
    }
    if sample >= &root.interval.1 {
        return Some(Ordering::Greater);
    }
    if root.fiber.evaluate(sample).is_zero() {
        return Some(Ordering::Equal);
    }
    loop {
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        refine_once(root);
        if root.interval.0 == root.interval.1 {
            return Some(sample.cmp(&root.interval.0));
        }
        if sample <= &root.interval.0 {
            return Some(Ordering::Less);
        }
        if sample >= &root.interval.1 {
            return Some(Ordering::Greater);
        }
    }
}

/// Independent strict membership check for a supplied rational sample. An
/// exhausted comparison budget rejects the hint; it never accepts uncertainty.
pub(super) fn strictly_inside(
    sample: &Rational,
    lower: Option<&FiberRoot>,
    upper: Option<&FiberRoot>,
) -> bool {
    let mut budget = MAX_REFINEMENTS;
    lower
        .cloned()
        .is_none_or(|mut root| compare(sample, &mut root, &mut budget) == Some(Ordering::Greater))
        && upper
            .cloned()
            .is_none_or(|mut root| compare(sample, &mut root, &mut budget) == Some(Ordering::Less))
}

impl StackSamples {
    pub(super) fn new() -> Self {
        Self {
            samples: BTreeSet::new(),
            fifo: VecDeque::new(),
            bytes: 0,
        }
    }

    pub(super) fn bytes(&self) -> usize {
        self.bytes
    }

    /// The caller invokes this only after actually visiting the sample.
    /// FIFO retention and bit/byte/count limits are independent of proof size.
    pub(super) fn remember(&mut self, sample: &Rational) -> (bool, usize) {
        if self.samples.contains(sample) {
            return (false, 0);
        }
        let Some(bytes) = charge(sample) else {
            return (false, 0);
        };
        let mut evictions = 0;
        while self.samples.len() >= MAX_SAMPLES || self.bytes + bytes > MAX_BYTES {
            let oldest = self.fifo.pop_front().expect("nonempty bounded sample pool");
            self.samples.remove(&oldest);
            self.bytes -= charge(&oldest).expect("admitted sample has a bounded charge");
            evictions += 1;
        }
        self.samples.insert(sample.clone());
        self.fifo.push_back(sample.clone());
        self.bytes += bytes;
        (true, evictions)
    }

    pub(super) fn choose(
        &self,
        lower: Option<&FiberRoot>,
        upper: Option<&FiberRoot>,
    ) -> Option<Rational> {
        if self.samples.is_empty() {
            return None;
        }
        let mut lower = lower.cloned();
        let mut upper = upper.cloned();
        let mut budget = MAX_REFINEMENTS;
        for sample in &self.samples {
            if let Some(root) = &mut lower {
                if compare(sample, root, &mut budget)? != Ordering::Greater {
                    continue;
                }
            }
            if let Some(root) = &mut upper {
                if compare(sample, root, &mut budget)? != Ordering::Less {
                    // Remaining stored rationals are at least this large.
                    return None;
                }
            }
            return Some(sample.clone());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        algebra::{Algebra, rational},
        roots::isolate_union,
    };

    fn roots(expression: &str) -> Vec<FiberRoot> {
        let a = Algebra::new(&["x".into()]).unwrap();
        isolate_union(vec![(
            0,
            a.specialize_univariate(&a.parse(expression).unwrap(), 0, &[])
                .unwrap(),
        )])
        .unwrap()
    }

    #[test]
    fn retains_a_visited_sample_after_new_cuts_tighten_its_sector() {
        let mut samples = StackSamples::new();
        let old = rational("3/2").unwrap();
        assert_eq!(samples.remember(&old), (true, 0));
        let roots = roots("(x^2-2)*(x-2)*(x-3)");
        assert_eq!(samples.choose(Some(&roots[1]), Some(&roots[2])), Some(old));
        assert_eq!(samples.choose(Some(&roots[2]), Some(&roots[3])), None);
    }

    #[test]
    fn samples_on_new_boundaries_are_rejected_exactly() {
        let mut samples = StackSamples::new();
        samples.remember(&rational("3/2").unwrap());
        let roots = roots("2*x-3");
        assert_eq!(samples.choose(None, Some(&roots[0])), None);
        assert_eq!(samples.choose(Some(&roots[0]), None), None);
        assert_eq!(samples.choose(None, None), Some(rational("3/2").unwrap()));
    }

    #[test]
    fn another_root_on_an_open_interval_endpoint_is_not_the_target() {
        let mut root = roots("x*(x-1)").pop().unwrap();
        root.interval = (Rational::zero(), Rational::from(2));
        assert_eq!(
            compare(&Rational::zero(), &mut root, &mut 128),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare(&Rational::one(), &mut root, &mut 128),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn uncertain_membership_falls_back_at_the_refinement_budget() {
        let mut root = roots("x^2-2").pop().unwrap();
        root.interval = (Rational::one(), Rational::from(2));
        assert_eq!(compare(&rational("3/2").unwrap(), &mut root, &mut 0), None);
    }

    #[test]
    fn retained_rationals_obey_all_limits_and_instances_share_no_state() {
        let mut samples = StackSamples::new();
        let mut evictions = 0;
        for i in 0..1024 {
            evictions += samples.remember(&Rational::from(i)).1;
        }
        assert!(evictions > 0);
        assert!(samples.samples.len() <= MAX_SAMPLES);
        assert!(samples.bytes <= MAX_BYTES);
        let huge = Q.pow(&Rational::from(2), 2048);
        assert_eq!(samples.remember(&huge), (false, 0));
        assert_eq!(StackSamples::new().choose(None, None), None);
    }
}
