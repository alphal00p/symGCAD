//! Check projection obligations and every branch of a lifting certificate.
use crate::{
    algebra::{Algebra, Poly, rational},
    output::{Axis, Cell, LiftTrace, RootBound, SolveResult, Status},
    problem::normalize,
    projection::project_checked,
    roots::{isolate_union, roots_in_open_interval},
    solver::{projection_table, sign},
};
use anyhow::{Result, anyhow, ensure};
use serde::Serialize;
use std::collections::BTreeSet;
use symbolica::prelude::*;

#[derive(Debug, Serialize)]
pub struct Verification {
    pub verified: bool,
    pub cells: usize,
    pub stacks: usize,
    pub rejected_branches: usize,
}

fn check_root(
    root: &RootBound,
    algebra: &Algebra,
    polynomials: &[Poly],
    level: usize,
    prefix: &[Rational],
) -> Result<(Rational, Rational)> {
    ensure!(
        root.index_domain == crate::output::RootIndexDomain::Real,
        "ordinary global projection requires full-real root indices"
    );
    let polynomial = polynomials
        .get(root.polynomial)
        .ok_or_else(|| anyhow!("invalid polynomial id"))?;
    let fiber = algebra.specialize_univariate(polynomial, level, prefix)?;
    ensure!(
        !fiber.is_zero() && fiber.degree() > 0,
        "invalid root polynomial"
    );
    let lo = rational(&root.lower)?;
    let hi = rational(&root.upper)?;
    ensure!(lo <= hi, "reversed root interval");
    if lo == hi {
        ensure!(
            fiber.evaluate(&lo).is_zero(),
            "claimed exact root does not vanish"
        );
    } else {
        ensure!(
            roots_in_open_interval(&fiber, &lo, &hi) == 1,
            "interval ({lo},{hi}) does not isolate exactly one root of {} at prefix {prefix:?}",
            algebra.format(polynomial)
        );
    }
    let lc = fiber.coefficients().last().unwrap().abs();
    let cauchy = fiber
        .coefficients()
        .iter()
        .take(fiber.degree())
        .map(|c| c.abs() / &lc)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let left = (-cauchy).min(lo.clone()) - Rational::one();
    let before = roots_in_open_interval(&fiber, &left, &lo)
        + usize::from(lo != hi && fiber.evaluate(&lo).is_zero());
    ensure!(before == root.index, "incorrect real-root index");
    Ok((lo, hi))
}

struct Checker<'a> {
    result: &'a SolveResult,
    algebra: &'a Algebra,
    positives: &'a [Poly],
    split: &'a [Poly],
    polynomials: &'a [Poly],
    seen: BTreeSet<usize>,
    stacks: usize,
    rejected: usize,
}
impl Checker<'_> {
    fn walk(
        &mut self,
        trace: &LiftTrace,
        prefix: &mut Vec<Rational>,
        axes: &mut Vec<Axis>,
    ) -> Result<()> {
        let level = prefix.len();
        match trace {
            LiftTrace::Unfinished { .. } => {
                anyhow::bail!("unfinished branch in a complete certificate")
            }
            LiftTrace::Rejected { constraint } => {
                let mut p = self
                    .positives
                    .get(*constraint)
                    .ok_or_else(|| anyhow!("invalid rejecting constraint"))?
                    .clone();
                for (i, sample) in prefix.iter().enumerate() {
                    p = p.replace(i, sample);
                }
                ensure!(
                    p.is_constant() && p.get_constant() <= Rational::zero(),
                    "unjustified rejected branch"
                );
                self.rejected += 1;
            }
            LiftTrace::Accepted { cell } => {
                ensure!(
                    level == self.algebra.names.len(),
                    "accepted branch before all axes"
                );
                ensure!(self.seen.insert(*cell), "cell used twice");
                let actual = self
                    .result
                    .cells
                    .get(*cell)
                    .ok_or_else(|| anyhow!("invalid cell reference"))?;
                ensure!(
                    self.positives
                        .iter()
                        .all(|p| crate::solver::evaluate(p, prefix) > Rational::zero()),
                    "accepted sample violates strict inequalities"
                );
                let expected = Cell {
                    axes: axes.clone(),
                    signs: self
                        .split
                        .iter()
                        .map(|p| sign(&crate::solver::evaluate(p, prefix)))
                        .collect(),
                };
                ensure!(
                    actual == &expected,
                    "cell does not match its exhaustive lifting path"
                );
            }
            LiftTrace::Stack { roots, sectors } => {
                ensure!(level < self.algebra.names.len(), "excess lifting level");
                let ids = &self.result.projection_levels[level];
                let fibers = self.algebra.specialize_univariate_batch(
                    ids.iter().map(|&id| &self.polynomials[id]),
                    level,
                    prefix,
                )?;
                let expected = isolate_union(ids.iter().copied().zip(fibers).collect())?;
                ensure!(roots.len() == expected.len(), "root union is incomplete");
                ensure!(
                    sectors.len() == roots.len() + 1,
                    "missing or extra open sectors"
                );
                let mut intervals = Vec::new();
                for (r, e) in roots.iter().zip(expected) {
                    ensure!(
                        r.polynomial == e.polynomial && r.index == e.index,
                        "root union/order differs"
                    );
                    intervals.push(check_root(
                        r,
                        self.algebra,
                        self.polynomials,
                        level,
                        prefix,
                    )?);
                }
                ensure!(
                    intervals.windows(2).all(|w| w[0].1 < w[1].0),
                    "root intervals not ordered and disjoint"
                );
                self.stacks += 1;
                for (i, sector) in sectors.iter().enumerate() {
                    let sample = rational(&sector.sample)?;
                    if i > 0 {
                        let (a, b) = &intervals[i - 1];
                        ensure!(
                            sample >= *b && (a != b || sample > *b),
                            "sample not above lower root"
                        );
                    }
                    if i < intervals.len() {
                        let (a, b) = &intervals[i];
                        ensure!(
                            sample <= *a && (a != b || sample < *a),
                            "sample not below upper root"
                        );
                    }
                    let lower = if i > 0 {
                        Some(roots[i - 1].clone())
                    } else {
                        None
                    };
                    axes.push(Axis {
                        variable: self.algebra.names[level].clone(),
                        sample: sector.sample.clone(),
                        lower,
                        upper: roots.get(i).cloned(),
                    });
                    prefix.push(sample);
                    self.walk(&sector.child, prefix, axes)?;
                    prefix.pop();
                    axes.pop();
                }
            }
        }
        Ok(())
    }
}

pub fn verify(result: &SolveResult) -> Result<Verification> {
    if result.linear_negative.is_some() {
        return crate::linear_negative::verify(result);
    }
    if result.sign_filter.is_some() {
        return crate::sign_filter::verify(result);
    }
    if let Some(evidence) = &result.local {
        return crate::local_projection::verify_local_parts(result, evidence);
    }
    verify_nonlocal(result)
}

/// A local engine's explicitly labeled global fallback uses the ordinary
/// certificate checker without redispatching through its fallback metadata.
pub(crate) fn verify_nonlocal(result: &SolveResult) -> Result<Verification> {
    ensure!(
        result.linear_negative.is_none(),
        "unexpected negative linear coverage evidence"
    );
    if result.homogeneous.is_some() {
        return crate::homogeneity::verify_homogeneous(result);
    }
    if result.monotone.is_some() {
        return crate::monotone::verify_monotone(result);
    }
    ensure!(result.schema_version == 1, "unsupported result schema");
    ensure!(
        result.status == Status::CompleteGeneric,
        "result is not complete_generic"
    );
    let mut check_problem = result.problem.clone();
    if !result.problem.solver.order.is_empty() || result.problem.solver.ordering == "given" {
        ensure!(
            crate::solver::selected_order(&result.problem)? == result.order,
            "result order differs from the requested fixed order"
        );
    }
    check_problem.solver.order = result.order.clone();
    check_problem.validate()?;
    let normalized = normalize(&result.problem, &result.order)?;
    let algebra = &normalized.algebra;
    ensure!(
        result
            .normalized_constraints
            .iter()
            .map(|p| algebra.parse(p).map(crate::problem::positive_primitive))
            .collect::<Result<Vec<_>>>()?
            == normalized.positives,
        "normalized constraints differ from input"
    );
    ensure!(
        result
            .split_polynomials
            .iter()
            .map(|p| algebra.parse(p).map(crate::problem::positive_primitive))
            .collect::<Result<Vec<_>>>()?
            == normalized.split,
        "split polynomials differ from input"
    );
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    let expected = if result.problem.solver.positivity {
        let original = normalize(&result.problem, &result.problem.variables)?;
        let indices = result
            .order
            .iter()
            .map(|name| {
                result
                    .problem
                    .variables
                    .iter()
                    .position(|n| n == name)
                    .expect("validated order")
            })
            .collect::<Vec<_>>();
        let evidence = result
            .statistics
            .optimized_projection
            .as_ref()
            .ok_or_else(|| anyhow!("missing certified projection evidence"))?;
        ensure!(
            result.problem.solver.input_positivity || evidence.initial_removals.is_empty(),
            "input factor removals require input_positivity mode"
        );
        ensure!(
            result.problem.solver.prune_operations || evidence.operation_removals.is_empty(),
            "whole-operation removals require prune_operations mode"
        );
        crate::optimized_projection::replay_certified_projection_with_operations(
            &original.algebra,
            &original.positives,
            &original.split,
            &indices,
            &[],
            &evidence.initial_removals,
            &evidence.removals,
            &evidence.operation_removals,
        )?
    } else {
        ensure!(
            result
                .statistics
                .optimized_projection
                .as_ref()
                .is_none_or(|r| r.removals.is_empty()
                    && r.initial_removals.is_empty()
                    && r.operation_removals.is_empty()),
            "nonvanishing removals require certified projection mode"
        );
        project_checked(algebra, &all)?
    };
    let (table, levels) = projection_table(algebra, &expected);
    ensure!(
        table == result.polynomials && levels == result.projection_levels,
        "projection obligations are missing or modified"
    );
    ensure!(
        result.exceptional_polynomials == (0..table.len()).collect::<Vec<_>>(),
        "exceptional-set metadata differs"
    );
    let polynomials = table
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let mut checker = Checker {
        result,
        algebra,
        positives: &normalized.positives,
        split: &normalized.split,
        polynomials: &polynomials,
        seen: BTreeSet::new(),
        stacks: 0,
        rejected: 0,
    };
    checker.walk(
        result
            .trace
            .as_ref()
            .ok_or_else(|| anyhow!("missing coverage trace"))?,
        &mut vec![],
        &mut vec![],
    )?;
    ensure!(
        checker.seen.len() == result.cells.len(),
        "unaccounted output cells"
    );
    if let Some(merged) = &result.merged_cells {
        ensure!(
            result.problem.solver.merge == "aggressive",
            "merged cells present without compatibility mode"
        );
        ensure!(
            merged == &crate::solver::merge(&result.cells),
            "compatibility merge does not match certified raw cells"
        );
    }
    Ok(Verification {
        verified: true,
        cells: result.cells.len(),
        stacks: checker.stacks,
        rejected_branches: checker.rejected,
    })
}
