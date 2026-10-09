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
            let count =
                roots_in_open_interval(&fiber, &(-cauchy).min(x.clone() - Rational::one()), x);
            // count roots strictly below x: x is above root i iff count > i.
            if (count > bound.index) != lower {
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
