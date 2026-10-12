//! Exact common refinements of small generic decompositions.
//!
//! Membership checks use rational samples of a *certified common CAD* whose
//! projection includes every boundary and exceptional polynomial of both
//! inputs. They are consequently constant on each connected refinement cell;
//! this is not a random-sampling comparison.
//!
//! The in-process wall budget is checked between verification phases and cell
//! membership checks. A single algebra or verification call cannot be preempted;
//! callers needing a hard deadline must run this API in a bounded worker.
use crate::{
    Limits, Problem,
    algebra::{Algebra, rational},
    output::{Cell, SolveResult, Status},
    problem::normalize,
    roots::roots_in_open_interval,
    solver::solve,
    verify::verify,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericComparison {
    /// None means the exact common refinement exceeded its resource budget.
    pub equivalent: Option<bool>,
    pub common_regions: usize,
    /// Matching raw cell IDs for each common region, in refinement order.
    pub corresponding_cells: Vec<(usize, usize)>,
    pub counterexample: Option<BTreeMap<String, String>>,
    pub refinement: Box<SolveResult>,
}

fn contains(
    result: &SolveResult,
    algebra: &Algebra,
    cell: &Cell,
    point: &BTreeMap<String, Rational>,
) -> Result<bool> {
    let prefix = result
        .order
        .iter()
        .map(|v| point[v].clone())
        .collect::<Vec<_>>();
    for (level, axis) in cell.axes.iter().enumerate() {
        for (bound, lower) in [(&axis.lower, true), (&axis.upper, false)] {
            let Some(bound) = bound else { continue };
            let p = algebra.parse(&result.polynomials[bound.polynomial])?;
            let fiber = algebra.specialize_univariate(&p, level, &prefix[..level])?;
            ensure!(
                !fiber.is_zero(),
                "refinement sample lies on a nullified boundary"
            );
            let x = &prefix[level];
            ensure!(
                !fiber.evaluate(x).is_zero(),
                "refinement sample lies on a boundary"
            );
            let lc = fiber.coefficients().last().unwrap().abs();
            let cauchy = fiber
                .coefficients()
                .iter()
                .take(fiber.degree())
                .map(|c| c.abs() / &lc)
                .max()
                .unwrap_or_else(Rational::zero)
                + Rational::from(2);
            let above = match bound.index_domain {
                crate::output::RootIndexDomain::Real => {
                    roots_in_open_interval(&fiber, &(-cauchy).min(x.clone() - Rational::one()), x)
                        > bound.index
                }
                // x lies above the index-th positive root iff more than index
                // distinct positive roots lie below it. For the opt-in
                // projective selector a missing root is +infinity, which this
                // count already treats as never below x.
                crate::output::RootIndexDomain::Positive
                | crate::output::RootIndexDomain::PositiveOrInfinity
                    if x > &Rational::zero() =>
                {
                    roots_in_open_interval(&fiber, &Rational::zero(), x) > bound.index
                }
                crate::output::RootIndexDomain::Positive
                | crate::output::RootIndexDomain::PositiveOrInfinity => false,
                crate::output::RootIndexDomain::RealDescending => {
                    // For descending root i, x lies above it iff at most i
                    // distinct roots lie above x. The refinement sample is
                    // not a root, including at a repeated-root seam.
                    roots_in_open_interval(&fiber, x, &cauchy.max(x.clone() + Rational::one()))
                        <= bound.index
                }
            };
            if above != lower {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub fn compare_generic(
    left: &SolveResult,
    right: &SolveResult,
    limits: &Limits,
) -> Result<GenericComparison> {
    let started = Instant::now();
    verify(left)?;
    verify(right)?;
    let a = normalize(&left.problem, &left.order)?;
    let b = normalize(&right.problem, &left.order)?;
    let sorted = |polys: &[crate::algebra::Poly]| {
        let mut text = polys
            .iter()
            .map(|p| a.algebra.format(p))
            .collect::<Vec<_>>();
        text.sort();
        text.dedup();
        text
    };
    ensure!(
        sorted(&a.positives) == sorted(&b.positives),
        "comparison requires the same normalized strict domain"
    );
    ensure!(
        a.split == b.split,
        "comparison requires the same ordered split polynomials"
    );
    let mut cuts = a
        .split
        .iter()
        .map(|p| a.algebra.format(p))
        .collect::<Vec<_>>();
    for result in [left, right] {
        for p in &result.polynomials {
            let parsed = a.algebra.parse(p)?;
            if !parsed.is_constant() {
                cuts.push(a.algebra.format(&parsed));
            }
        }
    }
    cuts.sort();
    cuts.dedup();
    let mut problem = Problem {
        name: Some("exact common refinement".into()),
        variables: left.order.clone(),
        constraints: a
            .positives
            .iter()
            .map(|p| format!("({})>0", a.algebra.format(p)))
            .collect(),
        split: cuts,
        limits: limits.clone(),
        ..Default::default()
    };
    let remaining = limits.wall_time_secs - started.elapsed().as_secs_f64();
    let refinement = if remaining > 0.0 {
        problem.limits.wall_time_secs = remaining;
        solve(&problem)?
    } else {
        SolveResult::failure(
            problem,
            Status::Incomplete,
            "comparison budget exhausted checking inputs",
        )
    };
    let mut report = GenericComparison {
        equivalent: None,
        common_regions: 0,
        corresponding_cells: vec![],
        counterexample: None,
        refinement: Box::new(refinement),
    };
    if report.refinement.status != Status::CompleteGeneric {
        return Ok(report);
    }
    let expired = || started.elapsed().as_secs_f64() >= limits.wall_time_secs;
    if expired() {
        return Ok(report);
    }
    verify(&report.refinement)?;
    let left_algebra = Algebra::new(&left.order)?;
    let right_algebra = Algebra::new(&right.order)?;
    for cell in &report.refinement.cells {
        if expired() {
            return Ok(report);
        }
        let point = cell
            .axes
            .iter()
            .map(|axis| Ok((axis.variable.clone(), rational(&axis.sample)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let matching = |result: &SolveResult, algebra: &Algebra| -> Result<Option<Vec<usize>>> {
            let mut ids = vec![];
            for (i, candidate) in result.cells.iter().enumerate() {
                if expired() {
                    return Ok(None);
                }
                if contains(result, algebra, candidate, &point)? {
                    ids.push(i);
                }
            }
            Ok(Some(ids))
        };
        let Some(left_ids) = matching(left, &left_algebra)? else {
            return Ok(report);
        };
        let Some(right_ids) = matching(right, &right_algebra)? else {
            return Ok(report);
        };
        if left_ids.len() != 1
            || right_ids.len() != 1
            || left.cells[left_ids[0]].signs != right.cells[right_ids[0]].signs
        {
            report.equivalent = Some(false);
            report.counterexample =
                Some(point.into_iter().map(|(v, x)| (v, x.to_string())).collect());
            return Ok(report);
        }
        report.corresponding_cells.push((left_ids[0], right_ids[0]));
        report.common_regions += 1;
    }
    if !expired() {
        report.equivalent = Some(true);
    }
    Ok(report)
}

#[cfg(test)]
mod root_selection_tests {
    use super::*;
    use crate::output::{Axis, RootBound, RootIndexDomain};

    // These tests exercise exact membership only. A result's independent
    // coverage verifier must separately establish continuity of its selector.
    fn member(
        polynomial: &str,
        domain: RootIndexDomain,
        index: usize,
        lower: bool,
        t: i64,
        z: i64,
    ) -> Result<bool> {
        let problem = Problem {
            variables: vec!["t".into(), "z".into()],
            ..Default::default()
        };
        let mut result = SolveResult::failure(problem, Status::Incomplete, "membership-only test");
        result.order = vec!["t".into(), "z".into()];
        result.polynomials = vec![polynomial.into()];
        let bound = Some(RootBound {
            polynomial: 0,
            index,
            index_domain: domain,
            lower: "0".into(),
            upper: "0".into(),
        });
        let cell = Cell {
            axes: vec![
                Axis {
                    variable: "t".into(),
                    sample: "0".into(),
                    lower: None,
                    upper: None,
                },
                Axis {
                    variable: "z".into(),
                    sample: "1".into(),
                    lower: lower.then(|| bound.clone()).flatten(),
                    upper: (!lower).then_some(bound).flatten(),
                },
            ],
            signs: vec![],
        };
        contains(
            &result,
            &Algebra::new(&result.order)?,
            &cell,
            &BTreeMap::from([
                ("t".into(), Rational::from(t)),
                ("z".into(), Rational::from(z)),
            ]),
        )
    }

    #[test]
    fn largest_root_crosses_zero_and_remains_distinct_at_collision() {
        for t in [-2, 0, 2] {
            let largest = t.max(0);
            assert!(
                member(
                    "z*(z-t)",
                    RootIndexDomain::RealDescending,
                    0,
                    true,
                    t,
                    largest + 1
                )
                .unwrap()
            );
            assert!(
                !member(
                    "z*(z-t)",
                    RootIndexDomain::RealDescending,
                    0,
                    true,
                    t,
                    largest - 1
                )
                .unwrap()
            );
            assert!(
                member(
                    "z*(z-t)",
                    RootIndexDomain::RealDescending,
                    0,
                    false,
                    t,
                    largest - 1
                )
                .unwrap()
            );
        }
        assert!(member("z*(z-t)", RootIndexDomain::RealDescending, 0, true, 0, 0).is_err());
    }

    #[test]
    fn descending_indices_count_distinct_roots_and_preserve_legacy_domains() {
        let p = "z^2*(z-2)^3*(z+2)";
        for (index, inside, outside) in [(0, 3, 1), (1, 1, -1), (2, -1, -3)] {
            assert!(member(p, RootIndexDomain::RealDescending, index, true, 0, inside).unwrap());
            assert!(!member(p, RootIndexDomain::RealDescending, index, true, 0, outside).unwrap());
        }
        for z in [1, 3] {
            let descending =
                member("z*(z-t)", RootIndexDomain::RealDescending, 0, true, 2, z).unwrap();
            assert_eq!(
                descending,
                member("z*(z-t)", RootIndexDomain::Real, 1, true, 2, z).unwrap()
            );
            assert_eq!(
                descending,
                member("z*(z-t)", RootIndexDomain::Positive, 0, true, 2, z).unwrap()
            );
        }
    }

    #[test]
    fn projective_upper_sections_are_infinite_where_the_positive_root_is_missing() {
        // z < (first positive root of t*z+1, or +infinity): the root -1/t is
        // positive only for t<0.
        let p = "t*z+1";
        let upper = |t, z| member(p, RootIndexDomain::PositiveOrInfinity, 0, false, t, z);
        assert!(upper(-1, 0).unwrap() && !upper(-1, 2).unwrap());
        for t in [1, 2] {
            for z in [1, 5, 1000] {
                assert!(upper(t, z).unwrap(), "t={t} z={z}");
            }
        }
        // A missing root gives a constant nonzero fiber at t=0: still inside.
        assert!(upper(0, 7).unwrap());
        // Non-positive z lies below every positive root and below infinity.
        assert!(upper(-1, -3).unwrap());
        // A second positive root never exists for a linear fiber.
        assert!(member(p, RootIndexDomain::PositiveOrInfinity, 1, false, -1, 9).unwrap());
    }
}
