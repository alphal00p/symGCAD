//! Bounded, proposed polynomial identities for strict positive-root separation.
//!
//! p*L+q*M=k*H^2 with k>0 and H nonzero on the positive stack excludes every
//! shared positive root. This does not establish individual delineability.
//! V1 proves H nonzero only from uniform coefficient signs and original strict
//! coordinate bounds. In particular, no target inequality is a premise.
use super::{highest, positive_coordinate, prefix_value};
use crate::{
    algebra::{Poly, canonical, rational},
    problem::NormalizedProblem,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Instant};
use symbolica::prelude::*;

const MAX_HINTS: usize = 128;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_EXPRESSION: usize = 64 * 1024;
const MAX_PRODUCTS: usize = 65_536;

/// A proposal, never a domain assumption. Polynomial representatives must be
/// canonical in the actual ordered variable map; unmatched proposals decline.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictBezoutHint {
    pub schema_version: u32,
    pub polynomials: [String; 2],
    pub multipliers: [String; 2],
    pub square: String,
    pub multiplier: String,
}

/// Exact separation of this sorted pair on the original positive stack.
/// Supporting indices name literal original k*x>0 constraints, not arbitrary
/// nonlinear constraints that might themselves depend on the target signs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositiveStrictBezout {
    pub polynomials: [usize; 2],
    pub hint: StrictBezoutHint,
    pub supporting_constraints: Vec<usize>,
}

#[derive(Clone)]
struct Prepared {
    hint: StrictBezoutHint,
    pair: [Poly; 2],
    variable: usize,
    supporting_constraints: Vec<usize>,
    /// Only assigned axes; the current positivity is the ray's own premise.
    guards: Vec<Poly>,
}

fn bytes(hint: &StrictBezoutHint) -> usize {
    hint.polynomials
        .iter()
        .chain(&hint.multipliers)
        .chain([&hint.square, &hint.multiplier])
        .fold(0usize, |total, text| total.saturating_add(text.len()))
}

fn bounded(p: &Poly) -> bool {
    p.nterms() <= 4096
        && p.nvars() <= 64
        && p.exponents_iter()
            .all(|powers| powers.iter().map(|&e| u64::from(e)).sum::<u64>() <= 32)
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= 4096
                && c.denominator_ref().significant_bits() <= 4096
        })
}

fn prepare(normalized: &NormalizedProblem, hint: &StrictBezoutHint) -> Result<Prepared> {
    ensure!(hint.schema_version == 1, "unsupported strict Bezout schema");
    ensure!(
        hint.polynomials
            .iter()
            .chain(&hint.multipliers)
            .chain([&hint.square, &hint.multiplier])
            .all(|text| !text.is_empty() && text.len() <= MAX_EXPRESSION),
        "strict Bezout expression byte cap"
    );
    // Parsing, like ordinary input parsing, is inside the killable worker.
    // Expanded-polynomial caps are checked before any identity products.
    let parse = |text: &str| -> Result<Poly> {
        let p = normalized.algebra.parse(text)?;
        ensure!(bounded(&p), "strict Bezout polynomial resource cap");
        Ok(p)
    };
    let pair = [parse(&hint.polynomials[0])?, parse(&hint.polynomials[1])?];
    let multipliers = [parse(&hint.multipliers[0])?, parse(&hint.multipliers[1])?];
    let h = parse(&hint.square)?;
    let k = rational(&hint.multiplier)?;
    ensure!(
        k > Rational::zero()
            && k.numerator_ref().significant_bits() <= 4096
            && k.denominator_ref().significant_bits() <= 4096,
        "strict Bezout multiplier must be bounded and positive"
    );
    ensure!(pair[0] != pair[1], "strict Bezout pair is not distinct");
    let variable = highest(&pair[0]).ok_or_else(|| anyhow!("constant strict Bezout cut"))?;
    ensure!(
        highest(&pair[1]) == Some(variable) && pair.iter().all(|p| canonical(p) == *p),
        "strict Bezout cuts must be canonical with the same main axis"
    );
    ensure!(
        multipliers
            .iter()
            .chain([&h])
            .all(|p| highest(p).is_none_or(|v| v <= variable)),
        "strict Bezout identity uses a later axis"
    );
    ensure!(!h.is_zero(), "strict Bezout square is zero");
    let negative = h.coefficients[0].is_negative();
    ensure!(
        h.coefficients
            .iter()
            .all(|c| !c.is_zero() && c.is_negative() == negative),
        "strict Bezout square lacks uniform nonzero coefficient signs"
    );
    let mut supporting_constraints = Vec::new();
    let mut guards = Vec::new();
    for axis in 0..=variable {
        if h.degree(axis) == 0 {
            continue;
        }
        let index = positive_coordinate(normalized, axis).ok_or_else(|| {
            anyhow!("strict Bezout square lacks original positive coordinate premise")
        })?;
        supporting_constraints.push(index);
        if axis < variable {
            guards.push(normalized.positives[index].clone());
        }
    }
    supporting_constraints.sort_unstable();
    for (p, q) in [
        (&pair[0], &multipliers[0]),
        (&pair[1], &multipliers[1]),
        (&h, &h),
    ] {
        ensure!(
            p.nterms().saturating_mul(q.nterms()) <= MAX_PRODUCTS,
            "strict Bezout identity multiplication cap"
        );
    }
    let left = &(&pair[0] * &multipliers[0]) + &(&pair[1] * &multipliers[1]);
    let right = (&h * &h).mul_coeff(k);
    ensure!(left == right, "strict Bezout polynomial identity is false");
    Ok(Prepared {
        hint: hint.clone(),
        pair,
        variable,
        supporting_constraints,
        guards,
    })
}

impl Prepared {
    fn witness(
        &self,
        polynomials: &[Poly],
        ids: [usize; 2],
        variable: usize,
        prefix: &[Rational],
    ) -> Result<Option<(PositiveStrictBezout, Vec<Poly>)>> {
        if ids[0] >= ids[1] || variable != self.variable {
            return Ok(None);
        }
        let Some(p) = polynomials.get(ids[0]) else {
            return Ok(None);
        };
        let Some(q) = polynomials.get(ids[1]) else {
            return Ok(None);
        };
        if !((p == &self.pair[0] && q == &self.pair[1])
            || (p == &self.pair[1] && q == &self.pair[0]))
        {
            return Ok(None);
        }
        // Guard nonvanishing later proves these sampled positive signs persist
        // throughout the connected ancestor cell. Negative prefixes decline.
        for guard in &self.guards {
            if prefix_value(guard, prefix)? <= Rational::zero() {
                return Ok(None);
            }
        }
        Ok(Some((
            PositiveStrictBezout {
                polynomials: ids,
                hint: self.hint.clone(),
                supporting_constraints: self.supporting_constraints.clone(),
            },
            self.guards.clone(),
        )))
    }
}

#[derive(Default)]
pub(super) struct Registry {
    proposals: HashMap<Poly, Vec<Prepared>>,
    /// Only independently checked evidence is cached by the verifier; never a
    /// producer cache, input proposal registry, or persisted proof decision.
    replayed: HashMap<StrictBezoutHint, Prepared>,
}

impl Registry {
    pub(super) fn from_hints(
        normalized: &NormalizedProblem,
        hints: &[StrictBezoutHint],
        report: &mut super::LocalReport,
        solve_started: Instant,
        wall_time_secs: f64,
    ) -> Self {
        let started = Instant::now();
        let mut registry = Self::default();
        if hints.len() > MAX_HINTS
            || hints.iter().fold(0usize, |n, h| n.saturating_add(bytes(h))) > MAX_BYTES
        {
            report.strict_bezout_hints_rejected = hints.len();
            report
                .strict_bezout_rejections
                .push("strict Bezout hint inventory cap".into());
        } else {
            for (index, hint) in hints.iter().enumerate() {
                if solve_started.elapsed().as_secs_f64() >= wall_time_secs {
                    report.strict_bezout_hints_rejected += hints.len() - index;
                    if report.strict_bezout_rejections.len() < 8 {
                        report
                            .strict_bezout_rejections
                            .push("wall time limit during strict Bezout preparation".into());
                    }
                    break;
                }
                match prepare(normalized, hint) {
                    Ok(prepared) => {
                        registry
                            .proposals
                            .entry(prepared.pair[0].clone())
                            .or_default()
                            .push(prepared);
                        report.strict_bezout_hints_accepted += 1;
                    }
                    Err(error) => {
                        report.strict_bezout_hints_rejected += 1;
                        if report.strict_bezout_rejections.len() < 8 {
                            report.strict_bezout_rejections.push(error.to_string());
                        }
                    }
                }
            }
        }
        report.strict_bezout_preparation_seconds = started.elapsed().as_secs_f64();
        registry
    }

    pub(super) fn attempt(
        &self,
        polynomials: &[Poly],
        ids: [usize; 2],
        variable: usize,
        prefix: &[Rational],
    ) -> Result<Option<(PositiveStrictBezout, Vec<Poly>)>> {
        for &id in &ids {
            if let Some(candidates) = self.proposals.get(&polynomials[id]) {
                for prepared in candidates {
                    if let Some(witness) = prepared.witness(polynomials, ids, variable, prefix)? {
                        return Ok(Some(witness));
                    }
                }
            }
        }
        Ok(None)
    }

    pub(super) fn verify(
        &mut self,
        normalized: &NormalizedProblem,
        polynomials: &[Poly],
        ids: [usize; 2],
        variable: usize,
        prefix: &[Rational],
        supplied: &PositiveStrictBezout,
    ) -> Result<Vec<Poly>> {
        ensure!(
            supplied.polynomials == ids,
            "strict Bezout witness names the wrong pair"
        );
        if !self.replayed.contains_key(&supplied.hint) {
            ensure!(
                self.replayed.len() < MAX_HINTS,
                "strict Bezout replay inventory cap"
            );
            let prepared = prepare(normalized, &supplied.hint)?;
            self.replayed.insert(supplied.hint.clone(), prepared);
        }
        let Some((expected, guards)) =
            self.replayed[&supplied.hint].witness(polynomials, ids, variable, prefix)?
        else {
            anyhow::bail!("strict Bezout witness does not apply on this positive stack");
        };
        ensure!(
            &expected == supplied,
            "strict Bezout coordinate premises were changed"
        );
        Ok(guards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Problem, problem::normalize};

    fn normalized(constraints: &[&str]) -> NormalizedProblem {
        let problem = Problem {
            variables: vec!["a".into(), "z".into()],
            constraints: constraints.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        normalize(&problem, &problem.variables).unwrap()
    }

    fn hint() -> StrictBezoutHint {
        StrictBezoutHint {
            schema_version: 1,
            polynomials: ["a+z".into(), "a^2+a+z".into()],
            multipliers: ["-1".into(), "1".into()],
            square: "a".into(),
            multiplier: "1".into(),
        }
    }

    #[test]
    fn original_positive_coordinates_are_the_only_strict_premises() {
        let n = normalized(&["a>0", "z>0"]);
        let p = prepare(&n, &hint()).unwrap();
        assert_eq!(p.supporting_constraints, vec![0]);
        assert_eq!(p.guards, vec![n.positives[0].clone()]);
        assert!(
            p.witness(&p.pair, [0, 1], 1, &[Rational::one()])
                .unwrap()
                .is_some()
        );
        assert!(
            p.witness(&p.pair, [0, 1], 1, &[Rational::from(-1)])
                .unwrap()
                .is_none()
        );
        assert!(prepare(&normalized(&["a^3>0", "z>0"]), &hint()).is_err());
        assert!(prepare(&normalized(&["z>0"]), &hint()).is_err());
        let mut constant = hint();
        constant.polynomials[1] = "a+z+1".into();
        constant.square = "-1".into();
        assert!(
            prepare(&normalized(&["z>0"]), &constant)
                .unwrap()
                .guards
                .is_empty()
        );
    }

    #[test]
    fn exact_identity_strictness_and_replay_are_not_proposals() {
        let n = normalized(&["a>0", "z>0"]);
        let p = prepare(&n, &hint()).unwrap();
        let (proof, guards) = p
            .witness(&p.pair, [0, 1], 1, &[Rational::one()])
            .unwrap()
            .unwrap();
        let mut independent = Registry::default();
        assert_eq!(
            independent
                .verify(&n, &p.pair, [0, 1], 1, &[Rational::one()], &proof)
                .unwrap(),
            guards
        );
        let mut changed = proof.clone();
        changed.hint.multipliers[0] = "1".into();
        assert!(
            independent
                .verify(&n, &p.pair, [0, 1], 1, &[Rational::one()], &changed)
                .is_err()
        );
        let mut changed = proof;
        changed.supporting_constraints.clear();
        assert!(
            independent
                .verify(&n, &p.pair, [0, 1], 1, &[Rational::one()], &changed)
                .is_err()
        );
        let mut weak = hint();
        weak.polynomials[1] = "a^2-a+z+1".into();
        weak.square = "a-1".into();
        assert!(
            prepare(&n, &weak)
                .err()
                .unwrap()
                .to_string()
                .contains("uniform")
        );
        weak.square = "0".into();
        assert!(prepare(&n, &weak).is_err());
        let mut negative = hint();
        negative.multiplier = "-1".into();
        assert!(prepare(&n, &negative).is_err());
    }

    #[test]
    fn bounded_inventory_and_preprocessing_deadline_decline() {
        let n = normalized(&["a>0", "z>0"]);
        let mut too_large = hint();
        too_large.square = "a".repeat(MAX_EXPRESSION + 1);
        assert!(prepare(&n, &too_large).is_err());
        let mut high_degree = hint();
        high_degree.square = "a^33".into();
        assert!(prepare(&n, &high_degree).is_err());
        let mut report = super::super::LocalReport::default();
        let registry = Registry::from_hints(
            &n,
            &vec![hint(); MAX_HINTS + 1],
            &mut report,
            Instant::now(),
            10.0,
        );
        assert!(registry.proposals.is_empty());
        assert_eq!(report.strict_bezout_hints_rejected, MAX_HINTS + 1);
        let mut report = super::super::LocalReport::default();
        let registry = Registry::from_hints(&n, &[hint()], &mut report, Instant::now(), 0.0);
        assert!(registry.proposals.is_empty());
        assert_eq!(report.strict_bezout_hints_rejected, 1);
        assert!(report.strict_bezout_rejections[0].contains("wall time"));
    }
}
