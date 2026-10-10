//! Exact negative fibers of linear polynomials on a positive orthant.
//!
//! For q=A*z+B and U=C*z+D, C,D>0 and B*C-A*D>=0 imply
//! q<0 iff A<0 and z>max(0,-B/A). Alternatively, a global A<0
//! certificate gives the same fiber without a residual sign constraint.
//! The boundary is the largest distinct real root of z*q. It is continuous
//! even where B=0; this theorem does not assert simple-root delineability.

use crate::{
    Problem,
    algebra::{Poly, factor_polynomial, rational},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    output::{Axis, Cell, RootBound, RootIndexDomain, SolveResult, Statistics, Status},
    problem::{NormalizedProblem, normalize},
    solver::{evaluate, merge},
    verify::Verification,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LinearNegativeRule {
    NegativeLeading {
        certificate: DomainCertificate,
    },
    PositiveGuide {
        guide: String,
        leading: DomainCertificate,
        endpoint: DomainCertificate,
        /// The exact identity is B*C-A*D = multiplier * square^2.
        multiplier: String,
        square: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearNegativeStep {
    pub variable: String,
    pub polynomial: String,
    pub leading: String,
    pub endpoint: String,
    pub rule: LinearNegativeRule,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearNegativeEvidence {
    pub target_constraint: usize,
    /// Elimination order; each removed axis is restored after its base.
    pub steps: Vec<LinearNegativeStep>,
    pub base: Box<SolveResult>,
}

fn coefficient(p: &Poly, axis: usize, degree: u32) -> Poly {
    let mut result = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        if e[axis] == degree {
            let mut exponents = e.to_vec();
            exponents[axis] = 0;
            result.append_monomial(c.clone(), &exponents);
        }
    }
    result
}

fn positive_axis(p: &Poly) -> Option<usize> {
    if p.nterms() != 1 || p.coefficients[0] <= Rational::zero() {
        return None;
    }
    let exponent = p.exponents_iter().next()?;
    let mut active = exponent.iter().enumerate().filter(|(_, e)| **e != 0);
    let (axis, &degree) = active.next()?;
    (degree == 1 && active.next().is_none()).then_some(axis)
}

struct Input {
    normalized: NormalizedProblem,
    target: usize,
    guide: Option<Poly>,
}

fn input(problem: &Problem) -> Result<Input> {
    problem.validate()?;
    ensure!(
        problem.solver.engine == "linear_negative",
        "wrong negative linear engine"
    );
    ensure!(
        problem.split.is_empty(),
        "negative linear reduction requires an unsplit strict target"
    );
    ensure!(
        problem.solver.homogeneous_anchor.is_none(),
        "negative linear guide transport through homogeneity is not implemented"
    );
    let normalized = normalize(problem, &problem.variables)?;
    let targets: Vec<_> = normalized
        .positives
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_constant() && positive_axis(p).is_none())
        .map(|(i, _)| i)
        .collect();
    ensure!(
        targets.len() == 1,
        "negative linear reduction requires exactly one non-coordinate strict target on a positive orthant"
    );
    let target = targets[0];
    let constrained: BTreeSet<_> = normalized
        .positives
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != target)
        .filter_map(|(_, p)| positive_axis(p))
        .collect();
    ensure!(
        constrained.len() == problem.variables.len(),
        "negative linear reduction requires every coordinate strictly positive"
    );
    let guide = problem
        .solver
        .negative_linear_guide
        .as_ref()
        .map(|guide| {
            let mut proposed = problem.clone();
            proposed.split = vec![guide.clone()];
            Ok::<_, anyhow::Error>(normalize(&proposed, &problem.variables)?.split.remove(0))
        })
        .transpose()?;
    Ok(Input {
        normalized,
        target,
        guide,
    })
}

fn initial_order(problem: &Problem) -> Vec<String> {
    if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
        problem.variable_groups.iter().flatten().cloned().collect()
    } else {
        problem.effective_order()
    }
}

fn check_time(problem: &Problem, started: Option<Instant>) -> Result<()> {
    if started.is_some_and(|s| s.elapsed().as_secs_f64() >= problem.limits.wall_time_secs) {
        return Err(crate::resultant_interpolation::Stopped(
            "negative linear reduction reached its wall budget".into(),
        )
        .into());
    }
    Ok(())
}

/// Bounded proposal only. Replay multiplies the supplied witness exactly and
/// never trusts factorization output or a sampled nonnegative value.
fn square_witness(p: &Poly) -> Result<Option<(Rational, Poly)>> {
    if p.is_zero() {
        return Ok(Some((Rational::zero(), p.one())));
    }
    if p.nterms() > 4096 || (0..p.nvars()).any(|i| p.degree(i) > 4096) {
        return Ok(None);
    }
    let mut multiplier = Rational::one();
    let mut square = p.one();
    for (factor, count) in factor_polynomial(p, true)? {
        if factor.is_constant() {
            multiplier *= factor.pow(count).get_constant();
        } else {
            if count % 2 != 0 {
                return Ok(None);
            }
            square = &square * &factor.pow(count / 2);
        }
    }
    if multiplier < Rational::zero() {
        return Ok(None);
    }
    ensure!(
        &square.pow(2) * &p.constant(multiplier.clone()) == *p,
        "nonnegative square reconstruction failed"
    );
    Ok(Some((multiplier, square)))
}

fn candidate(
    input: &Input,
    p: &Poly,
    guide: Option<&Poly>,
    axis: usize,
) -> Result<Option<LinearNegativeStep>> {
    if p.degree(axis) != 1 {
        return Ok(None);
    }
    let algebra = &input.normalized.algebra;
    let a = coefficient(p, axis, 1);
    let b = coefficient(p, axis, 0);
    let domain = derive_domain(algebra, &input.normalized.positives, &[input.target])?;
    let options = CertificateOptions::default();
    let rule = if let Some(certificate) =
        certify_nonzero(algebra, &a, &domain, &options)?.filter(|c| c.sign == -1)
    {
        LinearNegativeRule::NegativeLeading { certificate }
    } else {
        let Some(guide) = guide.filter(|g| g.degree(axis) == 1) else {
            return Ok(None);
        };
        let c = coefficient(guide, axis, 1);
        let d = coefficient(guide, axis, 0);
        let Some(leading) =
            certify_nonzero(algebra, &c, &domain, &options)?.filter(|c| c.sign == 1)
        else {
            return Ok(None);
        };
        let Some(endpoint) =
            certify_nonzero(algebra, &d, &domain, &options)?.filter(|c| c.sign == 1)
        else {
            return Ok(None);
        };
        if b.nterms()
            .saturating_mul(c.nterms())
            .saturating_add(a.nterms().saturating_mul(d.nterms()))
            > 2_000_000
            || [p, guide]
                .iter()
                .any(|p| (0..p.nvars()).any(|i| p.degree(i) > 2048))
        {
            return Ok(None);
        }
        let residual = &(&b * &c) - &(&a * &d);
        let Some((multiplier, square)) = square_witness(&residual)? else {
            return Ok(None);
        };
        LinearNegativeRule::PositiveGuide {
            guide: algebra.format(guide),
            leading,
            endpoint,
            multiplier: multiplier.to_string(),
            square: algebra.format(&square),
        }
    };
    Ok(Some(LinearNegativeStep {
        variable: algebra.names[axis].clone(),
        polynomial: algebra.format(p),
        leading: algebra.format(&a),
        endpoint: algebra.format(&b),
        rule,
    }))
}

struct Plan {
    target: usize,
    order: Vec<String>,
    steps: Vec<LinearNegativeStep>,
    base: Problem,
}

fn discover(problem: &Problem, started: Option<Instant>) -> Result<Plan> {
    let input = input(problem)?;
    let algebra = &input.normalized.algebra;
    let mut remaining = initial_order(problem);
    let mut polynomial = -input.normalized.positives[input.target].clone();
    let mut guide = input.guide.clone();
    let fixed = !problem.solver.order.is_empty() || problem.solver.ordering == "given";
    let mut steps = Vec::new();
    loop {
        check_time(problem, started)?;
        let Some(last) = remaining.last() else {
            break;
        };
        let group = problem
            .variable_groups
            .iter()
            .position(|g| g.contains(last));
        let candidates = remaining
            .iter()
            .rev()
            .filter(|v| !fixed || *v == last)
            .filter(|v| group.is_none_or(|g| problem.variable_groups[g].contains(v)))
            .cloned()
            .collect::<Vec<_>>();
        let mut found = None;
        for variable in candidates {
            check_time(problem, started)?;
            let axis = algebra.names.iter().position(|v| *v == variable).unwrap();
            if let Some(step) = candidate(&input, &polynomial, guide.as_ref(), axis)? {
                found = Some((axis, step));
                break;
            }
        }
        let Some((axis, step)) = found else {
            break;
        };
        remaining.retain(|v| *v != step.variable);
        polynomial = coefficient(&polynomial, axis, 1);
        guide = guide.map(|g| coefficient(&g, axis, 1));
        let terminal = matches!(step.rule, LinearNegativeRule::NegativeLeading { .. });
        steps.push(step);
        if terminal {
            break;
        }
    }
    let mut order = remaining;
    order.extend(steps.iter().rev().map(|s| s.variable.clone()));
    let base = check_steps(problem, &order, input.target, &steps)?;
    Ok(Plan {
        target: input.target,
        order,
        steps,
        base,
    })
}

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    Ok(discover(problem, None)?.order)
}

/// Check supplied obligations; no search, factorization, or projection is
/// invoked to replace a missing witness. The resulting base problem follows
/// from the exact one-axis equivalences composed in elimination order.
fn check_steps(
    problem: &Problem,
    order: &[String],
    target: usize,
    steps: &[LinearNegativeStep],
) -> Result<Problem> {
    let input = input(problem)?;
    ensure!(
        target == input.target,
        "negative linear target constraint changed"
    );
    let mut ordered = problem.clone();
    ordered.solver.order = order.to_vec();
    ordered.validate()?;
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        ensure!(
            order == initial_order(problem),
            "negative linear order violates fixed order"
        );
    }
    ensure!(steps.len() <= order.len(), "too many negative linear steps");
    let algebra = &input.normalized.algebra;
    let mut p = -input.normalized.positives[target].clone();
    let mut guide = input.guide;
    let mut removed = BTreeSet::new();
    let mut terminal = false;
    for (index, step) in steps.iter().enumerate() {
        ensure!(
            !terminal,
            "negative linear step follows a terminal certificate"
        );
        ensure!(
            step.variable == order[order.len() - 1 - index],
            "negative linear steps differ from cylindrical suffix"
        );
        let axis = algebra
            .names
            .iter()
            .position(|v| *v == step.variable)
            .ok_or_else(|| anyhow!("unknown peeled variable"))?;
        ensure!(
            removed.iter().all(|&i| p.degree(i) == 0) && p.degree(axis) == 1,
            "negative linear target has invalid support or degree"
        );
        let a = coefficient(&p, axis, 1);
        let b = coefficient(&p, axis, 0);
        ensure!(
            algebra.parse(&step.polynomial)? == p
                && algebra.parse(&step.leading)? == a
                && algebra.parse(&step.endpoint)? == b,
            "negative linear coefficient identity changed"
        );
        match &step.rule {
            LinearNegativeRule::NegativeLeading { certificate } => {
                verify_certificate(
                    algebra,
                    &a,
                    &input.normalized.positives,
                    &[target],
                    certificate,
                )?;
                ensure!(
                    certificate.sign == -1,
                    "negative linear leading sign is not negative"
                );
                terminal = true;
            }
            LinearNegativeRule::PositiveGuide {
                guide: recorded,
                leading,
                endpoint,
                multiplier,
                square,
            } => {
                let expected = guide
                    .as_ref()
                    .ok_or_else(|| anyhow!("comparison guide missing from input"))?;
                ensure!(
                    algebra.parse(recorded)? == *expected
                        && expected.degree(axis) == 1
                        && removed.iter().all(|&i| expected.degree(i) == 0),
                    "negative linear guide identity or support changed"
                );
                let c = coefficient(expected, axis, 1);
                let d = coefficient(expected, axis, 0);
                verify_certificate(algebra, &c, &input.normalized.positives, &[target], leading)?;
                verify_certificate(
                    algebra,
                    &d,
                    &input.normalized.positives,
                    &[target],
                    endpoint,
                )?;
                ensure!(
                    leading.sign == 1 && endpoint.sign == 1,
                    "comparison guide coefficients must be strictly positive"
                );
                let multiplier = rational(multiplier)?;
                let square = algebra.parse(square)?;
                ensure!(
                    multiplier >= Rational::zero()
                        && square.degree(axis) == 0
                        && removed.iter().all(|&i| square.degree(i) == 0),
                    "invalid nonnegative square witness"
                );
                ensure!(
                    &(&b * &c) - &(&a * &d) == &square.pow(2) * &p.constant(multiplier),
                    "negative linear nonnegative identity failed"
                );
            }
        }
        p = a;
        guide = guide.map(|g| coefficient(&g, axis, 1));
        removed.insert(axis);
    }
    let mut base = problem.clone();
    base.variables = order[..order.len() - steps.len()].to_vec();
    base.parameters.clear();
    base.constraints = input
        .normalized
        .positives
        .iter()
        .enumerate()
        .filter(|(i, p)| {
            *i != target && positive_axis(p).is_none_or(|axis| !removed.contains(&axis))
        })
        .map(|(_, p)| format!("({})>0", algebra.format(p)))
        .collect();
    if !terminal {
        base.constraints.push(format!("({})<0", algebra.format(&p)));
    }
    base.variable_groups = problem
        .variable_groups
        .iter()
        .map(|g| {
            g.iter()
                .filter(|v| base.variables.contains(v))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|g| !g.is_empty())
        .collect();
    base.solver.engine = "optimized".into();
    base.solver.ordering = "given".into();
    base.solver.order = base.variables.clone();
    base.solver.negative_linear_guide = None;
    base.solver.homogeneous_block.clear();
    base.solver.homogeneous_anchor = None;
    base.solver.merge = "none".into();
    base.validate()?;
    Ok(base)
}

struct Reconstructed {
    normalized: NormalizedProblem,
    table: Vec<String>,
    levels: Vec<Vec<usize>>,
    cells: Vec<Cell>,
}

fn reconstruct(
    problem: &Problem,
    order: &[String],
    steps: &[LinearNegativeStep],
    base: &SolveResult,
    started: Option<Instant>,
) -> Result<Reconstructed> {
    let normalized = normalize(problem, order)?;
    let algebra = &normalized.algebra;
    let base_dim = order.len() - steps.len();
    ensure!(
        base.order == order[..base_dim],
        "negative linear base order differs"
    );
    let mut table = base
        .polynomials
        .iter()
        .map(|p| Ok(algebra.format(&algebra.parse(p)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut levels = base.projection_levels.clone();
    ensure!(
        levels.len() == base_dim,
        "negative linear base level count differs"
    );
    let mut stages = Vec::new();
    for step in steps.iter().rev() {
        let axis = base_dim + stages.len();
        let p = algebra.parse(&step.polynomial)?;
        let z = algebra.parse(&step.variable)?;
        let id = table.len();
        table.push(algebra.format(&(&z * &p)));
        levels.push(vec![id]);
        stages.push((
            axis,
            id,
            algebra.parse(&step.leading)?,
            algebra.parse(&step.endpoint)?,
        ));
    }
    let mut cells = Vec::with_capacity(base.cells.len());
    for cell in &base.cells {
        check_time(problem, started)?;
        ensure!(
            cell.axes.len() == base_dim && cell.signs.is_empty(),
            "invalid negative linear base cell"
        );
        let mut cell = cell.clone();
        let mut point = cell
            .axes
            .iter()
            .map(|axis| rational(&axis.sample))
            .collect::<Result<Vec<_>>>()?;
        point.resize(order.len(), Rational::zero());
        for (axis, id, a, b) in &stages {
            let a = evaluate(a, &point);
            let b = evaluate(b, &point);
            ensure!(
                a < Rational::zero(),
                "negative linear base does not imply negative slope"
            );
            let lower = (-b / a).max(Rational::zero());
            let sample = &lower + &Rational::one();
            cell.axes.push(Axis {
                variable: order[*axis].clone(),
                sample: sample.to_string(),
                lower: Some(RootBound {
                    polynomial: *id,
                    index: 0,
                    index_domain: RootIndexDomain::RealDescending,
                    lower: lower.to_string(),
                    upper: lower.to_string(),
                }),
                upper: None,
            });
            point[*axis] = sample;
        }
        cells.push(cell);
    }
    Ok(Reconstructed {
        normalized,
        table,
        levels,
        cells,
    })
}

pub fn solve(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let planned = discover(problem, Some(started)).and_then(|mut plan| {
        check_time(problem, Some(started))?;
        plan.base.limits.wall_time_secs =
            problem.limits.wall_time_secs - started.elapsed().as_secs_f64();
        if plan.base.limits.wall_time_secs <= 0.0 {
            return Err(crate::resultant_interpolation::Stopped(
                "negative linear delegated wall budget exhausted".into(),
            )
            .into());
        }
        Ok(plan)
    });
    // As in the monotone wrapper, ordering includes normalization and exact
    // guide/peel planning. The delegated base's phases remain additive.
    let planning_seconds = started.elapsed().as_secs_f64();
    let plan = match planned {
        Ok(plan) => plan,
        Err(error) => {
            return stopped_result(
                problem,
                started,
                Statistics {
                    ordering_seconds: planning_seconds,
                    ..Statistics::default()
                },
                error,
            );
        }
    };
    let base = crate::solver::solve(&plan.base)?;
    finish_base(problem, plan, base, started, planning_seconds)
}

/// Preserve measurements for a resource stop without classifying ordinary
/// algebra/input errors as incomplete mathematical work.
fn stopped_result(
    problem: &Problem,
    started: Instant,
    mut statistics: Statistics,
    error: anyhow::Error,
) -> Result<SolveResult> {
    if error
        .downcast_ref::<crate::resultant_interpolation::Stopped>()
        .is_none()
    {
        return Err(error);
    }
    let mut result = SolveResult::failure(problem.clone(), Status::Incomplete, error.to_string());
    statistics.total_seconds = started.elapsed().as_secs_f64();
    statistics.raw_cells = 0;
    result.statistics = statistics;
    Ok(result)
}

fn retain_base(mut result: SolveResult, plan: Plan, base: SolveResult) -> SolveResult {
    result.order = plan.order;
    result.linear_negative = Some(Box::new(LinearNegativeEvidence {
        target_constraint: plan.target,
        steps: plan.steps,
        base: Box::new(base),
    }));
    result
}

fn finish_base(
    problem: &Problem,
    plan: Plan,
    base: SolveResult,
    started: Instant,
    planning_seconds: f64,
) -> Result<SolveResult> {
    let mut statistics = base.statistics.clone();
    statistics.ordering_seconds += planning_seconds;
    if base.status != Status::CompleteGeneric
        && base.projection_levels.len() != plan.base.variables.len()
    {
        let mut result = SolveResult::failure(
            problem.clone(),
            base.status.clone(),
            base.diagnostic
                .clone()
                .unwrap_or_else(|| "negative linear base did not complete projection".into()),
        );
        statistics.total_seconds = started.elapsed().as_secs_f64();
        statistics.raw_cells = 0;
        result.statistics = statistics;
        return Ok(retain_base(result, plan, base));
    }
    let restoration_started = Instant::now();
    let restored =
        reconstruct(problem, &plan.order, &plan.steps, &base, Some(started)).map(|data| {
            let normalized_constraints = data
                .normalized
                .positives
                .iter()
                .map(|p| data.normalized.algebra.format(p))
                .collect();
            (data, normalized_constraints)
        });
    statistics.lifting_seconds += restoration_started.elapsed().as_secs_f64();
    let (data, normalized_constraints) = match restored {
        Ok(data) => data,
        Err(error) => {
            let result = stopped_result(problem, started, statistics, error)?;
            return Ok(retain_base(result, plan, base));
        }
    };
    statistics.raw_cells = data.cells.len();
    let merge_started = Instant::now();
    let merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&data.cells));
    statistics.merging_seconds += merge_started.elapsed().as_secs_f64();
    let mut result = SolveResult {
        schema_version: 1,
        status: base.status.clone(),
        problem: problem.clone(),
        order: plan.order,
        normalized_constraints,
        split_polynomials: vec![],
        polynomials: data.table,
        projection_levels: data.levels,
        exceptional_polynomials: base.exceptional_polynomials.clone(),
        cells: data.cells,
        trace: None,
        statistics,
        diagnostic: base.diagnostic.clone(),
        merged_cells,
        homogeneous: None,
        monotone: None,
        local: None,
        sign_filter: None,
        linear_negative: Some(Box::new(LinearNegativeEvidence {
            target_constraint: plan.target,
            steps: plan.steps,
            base: Box::new(base),
        })),
        bilinear_positive: None,
    };
    // Keep a completed base and any reconstructed data for diagnostics while
    // preventing a late or empty reconstruction from escaping the wall cap.
    if let Err(error) = check_time(problem, Some(started)) {
        result.status = Status::Incomplete;
        result.diagnostic = Some(error.to_string());
    }
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(result)
}

pub fn verify(result: &SolveResult) -> Result<Verification> {
    ensure!(
        result.schema_version == 1 && result.status == Status::CompleteGeneric,
        "negative linear result is not complete_generic"
    );
    ensure!(
        result.trace.is_none()
            && result.homogeneous.is_none()
            && result.monotone.is_none()
            && result.local.is_none()
            && result.sign_filter.is_none()
            && result.bilinear_positive.is_none(),
        "unexpected alternate negative linear coverage evidence"
    );
    let evidence = result
        .linear_negative
        .as_ref()
        .ok_or_else(|| anyhow!("missing negative linear evidence"))?;
    let mut expected = check_steps(
        &result.problem,
        &result.order,
        evidence.target_constraint,
        &evidence.steps,
    )?;
    let delegated_wall = evidence.base.problem.limits.wall_time_secs;
    ensure!(
        delegated_wall.is_finite()
            && delegated_wall > 0.0
            && delegated_wall <= expected.limits.wall_time_secs,
        "negative linear delegated wall budget exceeds original"
    );
    expected.limits.wall_time_secs = delegated_wall;
    ensure!(
        serde_json::to_value(&evidence.base.problem)? == serde_json::to_value(&expected)?,
        "negative linear base problem changed"
    );
    ensure!(
        evidence.base.linear_negative.is_none()
            && evidence.base.bilinear_positive.is_none()
            && evidence.base.homogeneous.is_none()
            && evidence.base.monotone.is_none()
            && evidence.base.sign_filter.is_none()
            && evidence.base.local.is_none(),
        "unexpected nested negative linear base evidence"
    );
    let verification = crate::verify::verify(&evidence.base)?;
    let data = reconstruct(
        &result.problem,
        &result.order,
        &evidence.steps,
        &evidence.base,
        None,
    )?;
    ensure!(
        result.normalized_constraints
            == data
                .normalized
                .positives
                .iter()
                .map(|p| data.normalized.algebra.format(p))
                .collect::<Vec<_>>()
            && result.split_polynomials.is_empty(),
        "negative linear original metadata changed"
    );
    ensure!(
        result.polynomials == data.table && result.projection_levels == data.levels,
        "negative linear root-boundary table changed"
    );
    ensure!(
        result.exceptional_polynomials == evidence.base.exceptional_polynomials,
        "negative linear exceptional set changed"
    );
    ensure!(
        result.cells == data.cells && result.statistics.raw_cells == result.cells.len(),
        "negative linear exhaustive restoration changed"
    );
    ensure!(
        result.merged_cells
            == (result.problem.solver.merge == "aggressive").then(|| merge(&result.cells)),
        "negative linear compatibility merge changed"
    );
    Ok(Verification {
        verified: true,
        cells: result.cells.len(),
        stacks: verification.stacks,
        rejected_branches: verification.rejected_branches,
    })
}

#[cfg(test)]
mod statistics_tests {
    use super::*;
    use std::time::Duration;

    fn problem() -> Problem {
        let mut problem = Problem {
            variables: vec!["x".into(), "z".into()],
            constraints: vec!["x>0".into(), "z>0".into(), "(x-1)*z+x<0".into()],
            ..Default::default()
        };
        problem.solver.engine = "linear_negative".into();
        problem.solver.ordering = "given".into();
        problem.solver.negative_linear_guide = Some("z+1".into());
        problem.limits.wall_time_secs = 10.0;
        problem
    }

    #[test]
    fn successful_phases_include_planning_restoration_and_merge() {
        let mut problem = problem();
        problem.solver.merge = "aggressive".into();
        let result = solve(&problem).unwrap();
        assert!(verify(&result).unwrap().verified);
        let base = &result.linear_negative.as_ref().unwrap().base.statistics;
        let actual = &result.statistics;
        assert!(actual.ordering_seconds > base.ordering_seconds);
        assert!(actual.lifting_seconds > base.lifting_seconds);
        assert!(actual.merging_seconds > base.merging_seconds);
        assert_eq!(actual.projection_seconds, base.projection_seconds);
        assert_eq!(actual.search_seconds, base.search_seconds);
        assert_eq!(actual.raw_cells, result.cells.len());
        assert!(actual.total_seconds >= base.total_seconds);
        assert!(
            actual.total_seconds
                >= actual.ordering_seconds
                    + actual.projection_seconds
                    + actual.lifting_seconds
                    + actual.merging_seconds
        );
    }

    #[test]
    fn planning_stop_keeps_elapsed_cost_and_does_not_hide_ordinary_errors() {
        let mut problem = problem();
        problem.limits.wall_time_secs = 1e-12;
        let result = solve(&problem).unwrap();
        assert_eq!(result.status, Status::Incomplete);
        assert!(result.statistics.ordering_seconds > 0.0);
        assert!(result.statistics.total_seconds >= result.statistics.ordering_seconds);
        assert!(result.cells.is_empty());
        assert!(result.linear_negative.is_none());
        assert!(verify(&result).is_err());
        assert!(
            stopped_result(
                &problem,
                Instant::now(),
                Statistics::default(),
                anyhow!("ordinary algebra failure")
            )
            .is_err()
        );
    }

    #[test]
    fn missing_projection_stop_retains_base_statistics_and_evidence() {
        let problem = problem();
        let plan = discover(&problem, None).unwrap();
        let mut base =
            SolveResult::failure(plan.base.clone(), Status::Incomplete, "projection cap");
        base.statistics.ordering_seconds = 0.01;
        base.statistics.projection_seconds = 0.02;
        base.statistics.visited_stacks = 17;
        let result = finish_base(
            &problem,
            plan,
            base,
            Instant::now() - Duration::from_millis(100),
            0.03,
        )
        .unwrap();
        assert_eq!(result.status, Status::Incomplete);
        assert_eq!(result.diagnostic.as_deref(), Some("projection cap"));
        assert_eq!(result.statistics.ordering_seconds, 0.01 + 0.03);
        assert_eq!(result.statistics.projection_seconds, 0.02);
        assert_eq!(result.statistics.visited_stacks, 17);
        assert!(result.statistics.total_seconds >= 0.1);
        assert_eq!(result.statistics.raw_cells, 0);
        assert!(result.cells.is_empty());
        assert_eq!(
            result.linear_negative.as_ref().unwrap().base.status,
            Status::Incomplete
        );
        assert!(verify(&result).is_err());
    }

    #[test]
    fn restoration_deadline_keeps_completed_base_and_handles_empty_output() {
        for empty in [false, true] {
            let mut problem = problem();
            problem.limits.wall_time_secs = 1.0;
            if empty {
                problem.constraints.push("false".into());
            }
            let plan = discover(&problem, None).unwrap();
            let base = crate::solver::solve(&plan.base).unwrap();
            assert!(crate::verify::verify(&base).unwrap().verified);
            assert_eq!(base.cells.is_empty(), empty);
            let inherited = base.statistics.clone();
            let result = finish_base(
                &problem,
                plan,
                base,
                Instant::now() - Duration::from_secs(2),
                0.01,
            )
            .unwrap();
            assert_eq!(result.status, Status::Incomplete);
            assert_eq!(
                result.statistics.ordering_seconds,
                inherited.ordering_seconds + 0.01
            );
            assert_eq!(
                result.statistics.projection_seconds,
                inherited.projection_seconds
            );
            assert!(result.statistics.lifting_seconds > inherited.lifting_seconds);
            assert!(result.statistics.total_seconds >= 2.0);
            assert_eq!(result.statistics.raw_cells, 0);
            assert!(result.cells.is_empty());
            assert!(
                crate::verify::verify(&result.linear_negative.as_ref().unwrap().base)
                    .unwrap()
                    .verified
            );
            assert!(verify(&result).is_err());
        }
    }
}
