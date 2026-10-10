//! Certified positive-axis linear/quadratic peeling.
//!
//! If q(z)=a*z^2+b*z+c with a>0,b>=0 on the complete base domain,
//! q is strictly increasing and unbounded on z>0. For c<0 its unique
//! positive root separates exactly two sectors; for c>=0 the entire positive
//! axis has positive sign. Linear q uses the same argument with a>0.
//! Endpoint sign cells and exact coefficient certificates therefore replace
//! global discriminants of irrelevant negative roots without losing coverage.

use crate::{
    Problem,
    algebra::{Algebra, Poly, rational},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    output::{Axis, Cell, RootBound, SolveResult, Status},
    problem::{NormalizedProblem, normalize},
    roots::{FiberRoot, isolate_union, sector_sample},
    solver::{bound, merge},
    verify::Verification,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    time::Instant,
};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonotoneStep {
    pub variable: String,
    pub polynomial: String,
    pub endpoint: String,
    pub degree: u32,
    pub leading: DomainCertificate,
    /// Quadratic linear coefficient: None means exactly zero. Linear
    /// polynomials have no separate linear-coefficient obligation.
    pub linear: Option<DomainCertificate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonotoneEvidence {
    /// Elimination order: the last lifted coordinate is peeled first.
    pub steps: Vec<MonotoneStep>,
    pub base: Box<SolveResult>,
    /// Absent in historical, complete sign-arrangement certificates. When
    /// present, the base proves only the exact negative endpoint inequality
    /// and reconstruction restores only the admissible lower fiber.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<MonotoneTarget>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonotoneTarget {
    /// Index in the original ordered strict constraint list.
    pub constraint_index: usize,
    /// Exact normalized Q defining the original constraint Q > 0.
    pub normalized_target: String,
    /// The peeled polynomial is orientation * Q. This theorem requires -1.
    pub orientation: i8,
    /// The reconstructed fibers have this sign; it must also be -1.
    pub selected_sign: i8,
}

struct Plan {
    order: Vec<String>,
    steps: Vec<MonotoneStep>,
    base: Problem,
}

struct NegativePlan {
    plan: Plan,
    target: MonotoneTarget,
}

fn coefficient(p: &Poly, variable: usize, degree: u32) -> Poly {
    let mut result = p.zero();
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        if m[variable] == degree {
            let mut exponent = m.to_vec();
            exponent[variable] = 0;
            result.append_monomial(c.clone(), &exponent);
        }
    }
    result
}

fn positive_axis_constraint(p: &Poly, variable: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter().next().is_some_and(|m| {
            m.iter()
                .enumerate()
                .all(|(i, &e)| e == u32::from(i == variable))
        })
}

fn admissible_axis(positives: &[Poly], variable: usize) -> bool {
    let constraints: Vec<_> = positives
        .iter()
        .filter(|p| p.degree(variable) > 0)
        .collect();
    !constraints.is_empty()
        && constraints
            .iter()
            .all(|p| positive_axis_constraint(p, variable))
}

fn candidate(
    normalized: &NormalizedProblem,
    p: &Poly,
    variable: usize,
) -> Result<Option<MonotoneStep>> {
    let degree = p.degree(variable);
    if ![1, 2].contains(&degree) || !admissible_axis(&normalized.positives, variable) {
        return Ok(None);
    }
    let algebra = &normalized.algebra;
    let domain = derive_domain(algebra, &normalized.positives, &[])?;
    let options = CertificateOptions::default();
    let Some(leading) = certify_nonzero(
        algebra,
        &coefficient(p, variable, degree),
        &domain,
        &options,
    )?
    else {
        return Ok(None);
    };
    if leading.sign != 1 {
        return Ok(None);
    }
    let linear = if degree == 2 {
        let b = coefficient(p, variable, 1);
        if b.is_zero() {
            None
        } else {
            let Some(certificate) = certify_nonzero(algebra, &b, &domain, &options)? else {
                return Ok(None);
            };
            if certificate.sign != 1 {
                return Ok(None);
            }
            Some(certificate)
        }
    } else {
        None
    };
    Ok(Some(MonotoneStep {
        variable: algebra.names[variable].clone(),
        polynomial: algebra.format(p),
        endpoint: algebra.format(&coefficient(p, variable, 0)),
        degree,
        leading,
        linear,
    }))
}

fn given_order(problem: &Problem) -> Vec<String> {
    if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
        problem.variable_groups.iter().flatten().cloned().collect()
    } else {
        problem.effective_order()
    }
}

/// A conservative, projection-free orientation heuristic for sign filtering.
/// Test the complete unscaled domain, excluding the transferred inequality.
/// A homogeneous solve may have additional candidates after reduction, but
/// this probe never needs to plan its order or solve its endpoint problem.
pub(crate) fn has_first_peel(problem: &Problem) -> Result<bool> {
    ensure!(
        problem.split.len() == 1,
        "first-peel probe requires one split"
    );
    let normalized = normalize(problem, &problem.variables)?;
    let fixed = !problem.solver.order.is_empty() || problem.solver.ordering == "given";
    let axes = if fixed {
        given_order(problem)
            .last()
            .cloned()
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        problem
            .variable_groups
            .last()
            .cloned()
            .unwrap_or_else(|| problem.variables.clone())
    };
    for name in axes {
        if problem.solver.homogeneous_anchor.as_ref() == Some(&name) {
            continue;
        }
        let variable = problem.variables.iter().position(|v| v == &name).unwrap();
        if candidate(&normalized, &normalized.split[0], variable)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn check_order(problem: &Problem, order: &[String]) -> Result<()> {
    ensure!(
        order.len() == problem.variables.len(),
        "incomplete monotone lifting order"
    );
    let mut checked = problem.clone();
    checked.solver.order = order.to_vec();
    checked.validate()?;
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        ensure!(
            given_order(problem) == order,
            "monotone order overrides the requested order"
        );
    }
    Ok(())
}

fn input(problem: &Problem) -> Result<NormalizedProblem> {
    problem.validate()?;
    ensure!(
        problem.solver.homogeneous_anchor.is_none(),
        "apply homogeneous reduction before monotone peeling"
    );
    ensure!(
        problem.split.len() == 1,
        "monotone engine currently requires one split polynomial"
    );
    normalize(problem, &problem.variables)
}

fn base_problem(
    problem: &Problem,
    normalized: &NormalizedProblem,
    steps: &[MonotoneStep],
    endpoint: &Poly,
) -> Result<Problem> {
    let peeled: BTreeSet<_> = steps.iter().map(|s| s.variable.as_str()).collect();
    let mut base = problem.clone();
    base.variables.retain(|v| !peeled.contains(v.as_str()));
    base.constraints = normalized
        .positives
        .iter()
        .filter(|p| {
            !steps.iter().any(|s| {
                p.degree(
                    problem
                        .variables
                        .iter()
                        .position(|v| v == &s.variable)
                        .unwrap(),
                ) > 0
            })
        })
        .map(|p| format!("({})>0", normalized.algebra.format(p)))
        .collect();
    base.split = if endpoint.is_zero() {
        vec![]
    } else {
        vec![normalized.algebra.format(endpoint)]
    };
    base.parameters.clear();
    base.variable_groups = base
        .variable_groups
        .iter()
        .map(|g| {
            g.iter()
                .filter(|v| !peeled.contains(v.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|g| !g.is_empty())
        .collect();
    base.solver.order.retain(|v| !peeled.contains(v.as_str()));
    base.solver.engine = problem.solver.monotone_base_engine.clone();
    base.solver.positivity = true;
    base.solver.homogeneous_anchor = None;
    base.solver.homogeneous_block.clear();
    base.validate()?;
    Ok(base)
}

fn discover_steps(problem: &Problem) -> Result<(NormalizedProblem, Vec<MonotoneStep>, Poly)> {
    let normalized = input(problem)?;
    let mut p = normalized.split[0].clone();
    let mut steps = Vec::new();
    let fixed = !problem.solver.order.is_empty() || problem.solver.ordering == "given";
    if fixed {
        for name in given_order(problem).iter().rev() {
            let variable = problem.variables.iter().position(|v| v == name).unwrap();
            let Some(step) = candidate(&normalized, &p, variable)? else {
                break;
            };
            p = coefficient(&p, variable, 0);
            steps.push(step);
        }
    } else {
        let mut groups = if problem.variable_groups.is_empty() {
            vec![(0..problem.variables.len()).collect::<Vec<_>>()]
        } else {
            problem
                .variable_groups
                .iter()
                .map(|g| {
                    g.iter()
                        .map(|n| problem.variables.iter().position(|v| v == n).unwrap())
                        .collect()
                })
                .collect()
        };
        loop {
            groups.retain(|g| !g.is_empty());
            let Some(group) = groups.last() else {
                break;
            };
            let mut candidates = Vec::new();
            for &v in group {
                if let Some(step) = candidate(&normalized, &p, v)? {
                    candidates.push((coefficient(&p, v, 0).nterms(), v, step));
                }
            }
            candidates.sort_by_key(|(terms, v, _)| (*terms, *v));
            let Some((_, variable, step)) = candidates.into_iter().next() else {
                break;
            };
            groups.last_mut().unwrap().retain(|&v| v != variable);
            p = coefficient(&p, variable, 0);
            steps.push(step);
        }
    }
    Ok((normalized, steps, p))
}

fn restrict_negative_endpoint(base: &mut Problem, endpoint: &str) -> Result<()> {
    // A constant zero endpoint deliberately becomes a false constraint. It
    // must never be turned into a zero exceptional polynomial that excludes
    // the entire domain instead of proving that the inequality is empty.
    base.split.clear();
    base.constraints.push(format!("(-({endpoint}))>0"));
    base.validate()
}

fn finish_plan(
    problem: &Problem,
    normalized: &NormalizedProblem,
    steps: Vec<MonotoneStep>,
    endpoint: &Poly,
    negative_target: bool,
) -> Result<Plan> {
    let mut base = base_problem(problem, normalized, &steps, endpoint)?;
    if negative_target {
        restrict_negative_endpoint(&mut base, &normalized.algebra.format(endpoint))?;
    }
    let mut order = crate::solver::selected_order(&base)?;
    base.solver.order = order.clone();
    base.solver.ordering = "given".into();
    order.extend(steps.iter().rev().map(|s| s.variable.clone()));
    check_order(problem, &order)?;
    Ok(Plan { order, steps, base })
}

fn plan(problem: &Problem) -> Result<Plan> {
    let (normalized, steps, endpoint) = discover_steps(problem)?;
    finish_plan(problem, &normalized, steps, &endpoint, false)
}

/// Reconstruct the exact target-free domain used for coefficient proofs.
/// The original inequality cannot justify its own monotonicity obligations.
fn target_problem(problem: &Problem, target: &MonotoneTarget) -> Result<Problem> {
    problem.validate()?;
    ensure!(
        problem.solver.engine == "monotone"
            && problem.solver.monotone_negative_target
            && problem.split.is_empty()
            && problem.solver.homogeneous_anchor.is_none(),
        "negative-target peeling requires an unsplit, unscaled opt-in monotone problem"
    );
    ensure!(
        target.orientation == -1 && target.selected_sign == -1,
        "negative-target orientation and selected sign must both be -1"
    );
    let normalized = normalize(problem, &problem.variables)?;
    let q = normalized
        .positives
        .get(target.constraint_index)
        .ok_or_else(|| anyhow!("negative-target constraint index is out of range"))?;
    ensure!(
        normalized.algebra.format(q) == target.normalized_target,
        "original negative-target constraint was modified"
    );
    let mut transferred = problem.clone();
    transferred.constraints.remove(target.constraint_index);
    transferred.split = vec![normalized.algebra.format(&(-q.clone()))];
    transferred.validate()?;
    Ok(transferred)
}

fn deadline_reached(problem: &Problem, started: Option<Instant>) -> bool {
    started.is_some_and(|t| t.elapsed().as_secs_f64() >= problem.limits.wall_time_secs)
}

fn negative_plan(problem: &Problem, started: Option<Instant>) -> Result<Option<NegativePlan>> {
    problem.validate()?;
    ensure!(
        problem.solver.engine == "monotone"
            && problem.solver.monotone_negative_target
            && problem.split.is_empty()
            && problem.solver.homogeneous_anchor.is_none(),
        "negative-target planning requires an unsplit, unscaled opt-in monotone problem"
    );
    let normalized = normalize(problem, &problem.variables)?;
    for (constraint_index, q) in normalized.positives.iter().enumerate() {
        if deadline_reached(problem, started) {
            break;
        }
        if q.is_constant() {
            continue;
        }
        let target = MonotoneTarget {
            constraint_index,
            normalized_target: normalized.algebra.format(q),
            orientation: -1,
            selected_sign: -1,
        };
        let transferred = target_problem(problem, &target)?;
        let (transferred_normalized, steps, endpoint) = discover_steps(&transferred)?;
        if steps.is_empty() {
            continue;
        }
        if deadline_reached(problem, started) {
            break;
        }
        return Ok(Some(NegativePlan {
            plan: finish_plan(
                &transferred,
                &transferred_normalized,
                steps,
                &endpoint,
                true,
            )?,
            target,
        }));
    }
    Ok(None)
}

/// Select the same original lifting order as the exact inequality solver.
/// Inapplicable problems retain the historical full-arrangement fallback.
pub fn selected_negative_order(problem: &Problem) -> Result<Vec<String>> {
    match negative_plan(problem, None)? {
        Some(plan) => Ok(plan.plan.order),
        None => crate::sign_filter::selected_order(problem),
    }
}

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    Ok(plan(problem)?.order)
}

fn check_steps(problem: &Problem, order: &[String], steps: &[MonotoneStep]) -> Result<Problem> {
    let normalized = input(problem)?;
    check_order(problem, order)?;
    ensure!(steps.len() <= order.len(), "too many monotone steps");
    let mut p = normalized.split[0].clone();
    for (step, name) in steps.iter().zip(order.iter().rev()) {
        ensure!(
            &step.variable == name,
            "peeled variables are not a lifting-order suffix"
        );
        let variable = problem.variables.iter().position(|v| v == name).unwrap();
        ensure!(
            admissible_axis(&normalized.positives, variable),
            "peeled axis has unsupported domain constraints"
        );
        ensure!(
            normalized.algebra.parse(&step.polynomial)? == p,
            "monotone polynomial identity was modified"
        );
        ensure!(
            [1, 2].contains(&step.degree) && p.degree(variable) == step.degree,
            "invalid monotone degree"
        );
        ensure!(
            step.leading.sign == 1,
            "leading coefficient must be strictly positive"
        );
        verify_certificate(
            &normalized.algebra,
            &coefficient(&p, variable, step.degree),
            &normalized.positives,
            &[],
            &step.leading,
        )?;
        if step.degree == 2 {
            let b = coefficient(&p, variable, 1);
            if b.is_zero() {
                ensure!(
                    step.linear.is_none(),
                    "zero linear coefficient has unexpected evidence"
                );
            } else {
                let certificate = step
                    .linear
                    .as_ref()
                    .ok_or_else(|| anyhow!("missing nonnegative linear coefficient proof"))?;
                ensure!(
                    certificate.sign == 1,
                    "linear coefficient must be nonnegative"
                );
                verify_certificate(
                    &normalized.algebra,
                    &b,
                    &normalized.positives,
                    &[],
                    certificate,
                )?;
            }
        } else {
            ensure!(
                step.linear.is_none(),
                "linear polynomial has a quadratic-only proof"
            );
        }
        p = coefficient(&p, variable, 0);
        ensure!(
            normalized.algebra.parse(&step.endpoint)? == p,
            "endpoint restriction is not the exact z=0 polynomial"
        );
    }
    let mut base = base_problem(problem, &normalized, steps, &p)?;
    base.solver.order = order[..order.len() - steps.len()].to_vec();
    base.solver.ordering = "given".into();
    Ok(base)
}

fn insert_polynomial(
    algebra: &Algebra,
    p: Poly,
    table: &mut Vec<String>,
    ids: &mut HashMap<Poly, usize>,
    levels: &mut [Vec<usize>],
) -> Result<usize> {
    ensure!(
        !p.is_constant(),
        "only nonconstant polynomials belong to the boundary table"
    );
    if let Some(&id) = ids.get(&p) {
        return Ok(id);
    }
    let id = table.len();
    let level = (0..algebra.names.len())
        .rev()
        .find(|&v| p.degree(v) > 0)
        .unwrap();
    levels[level].push(id);
    table.push(algebra.format(&p));
    ids.insert(p, id);
    Ok(id)
}

fn zero_bound(polynomial: usize) -> RootBound {
    RootBound {
        polynomial,
        index: 0,
        index_domain: crate::output::RootIndexDomain::Real,
        lower: "0".into(),
        upper: "0".into(),
    }
}

struct Restoration {
    variable: String,
    degree: u32,
    polynomial: Poly,
    coordinate: Poly,
    zero_endpoint: bool,
    polynomial_id: usize,
    coordinate_id: usize,
}

/// Solving retains cells; verification compares each deterministic leaf as it
/// is generated. The latter never owns a second complete output cell table.
enum RestorationCells<'a> {
    Collect(Vec<Cell>),
    Check {
        expected: &'a [Cell],
        matched: usize,
    },
}

impl RestorationCells<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Collect(cells) => cells.len(),
            Self::Check { matched, .. } => *matched,
        }
    }

    fn push(&mut self, cell: Cell) -> Result<()> {
        match self {
            Self::Collect(cells) => cells.push(cell),
            Self::Check { expected, matched } => {
                ensure!(
                    expected.get(*matched) == Some(&cell),
                    "monotone branches are missing or modified"
                );
                *matched = matched
                    .checked_add(1)
                    .ok_or_else(|| anyhow!("monotone cell count overflow"))?;
            }
        }
        Ok(())
    }
}

struct Restorer<'a> {
    algebra: &'a Algebra,
    stages: &'a [Restoration],
    started: Option<Instant>,
    limits: &'a crate::Limits,
    negative_only: bool,
    cells: RestorationCells<'a>,
    stopped: Option<String>,
    stacks: usize,
}

impl Restorer<'_> {
    fn children(&self, depth: usize, cell: Cell) -> Result<Vec<Cell>> {
        let stage = &self.stages[depth];
        let level = cell.axes.len();
        ensure!(
            self.algebra.names[level] == stage.variable && cell.signs.len() == 1,
            "base cell has incorrect dimensions or signs"
        );
        let sign = cell.signs[0];
        ensure!(
            !self.negative_only || sign == -1,
            "negative-target restoration requires a certified negative endpoint"
        );
        ensure!(
            [-1, 0, 1].contains(&sign) && (sign != 0 || stage.zero_endpoint),
            "uncertified endpoint sign zero"
        );
        if sign >= 0 {
            let mut child = cell;
            child.axes.push(Axis {
                variable: stage.variable.clone(),
                sample: "1".into(),
                lower: Some(zero_bound(stage.coordinate_id)),
                upper: None,
            });
            child.signs = vec![1];
            return Ok(vec![child]);
        }
        let prefix = cell
            .axes
            .iter()
            .map(|a| rational(&a.sample))
            .collect::<Result<Vec<_>>>()?;
        let roots = isolate_union(vec![
            (
                stage.coordinate_id,
                self.algebra
                    .specialize_univariate(&stage.coordinate, level, &prefix)?,
            ),
            (
                stage.polynomial_id,
                self.algebra
                    .specialize_univariate(&stage.polynomial, level, &prefix)?,
            ),
        ])?;
        let zero = roots
            .iter()
            .find(|r| r.polynomial == stage.coordinate_id)
            .ok_or_else(|| anyhow!("missing positive-axis zero boundary"))?;
        let positive: Vec<&FiberRoot> = roots
            .iter()
            .filter(|r| r.polynomial == stage.polynomial_id && r.interval.0 > Rational::zero())
            .collect();
        ensure!(
            positive.len() == 1,
            "monotone fiber lacks exactly one positive root"
        );
        let root = positive[0];
        ensure!(
            root.index == usize::from(stage.degree == 2),
            "monotone positive root has incorrect full real-root index"
        );
        let mut lower = cell.clone();
        lower.axes.push(Axis {
            variable: stage.variable.clone(),
            sample: sector_sample(Some(zero), Some(root)).to_string(),
            lower: Some(zero_bound(stage.coordinate_id)),
            upper: Some(bound(root)),
        });
        lower.signs = vec![-1];
        if self.negative_only {
            return Ok(vec![lower]);
        }
        let mut upper = cell;
        upper.axes.push(Axis {
            variable: stage.variable.clone(),
            sample: sector_sample(Some(root), None).to_string(),
            lower: Some(bound(root)),
            upper: None,
        });
        upper.signs = vec![1];
        Ok(vec![lower, upper])
    }

    fn visit(&mut self, depth: usize, mut cell: Cell) -> Result<()> {
        if self.stopped.is_some() {
            return Ok(());
        }
        if self
            .started
            .is_some_and(|t| t.elapsed().as_secs_f64() >= self.limits.wall_time_secs)
        {
            self.stopped = Some("monotone reconstruction wall time limit".into());
            return Ok(());
        }
        if self
            .limits
            .max_cells
            .is_some_and(|limit| self.cells.len() >= limit)
        {
            self.stopped = Some("monotone reconstruction cell limit".into());
            return Ok(());
        }
        if depth == self.stages.len() {
            ensure!(
                cell.axes.len() == self.algebra.names.len(),
                "incomplete reconstructed cell dimension"
            );
            // Intermediate target fibers retain -1 for the next monotone
            // step; only an emitted original-problem cell has no split sign.
            if self.negative_only {
                cell.signs.clear();
            }
            self.cells.push(cell)?;
            return Ok(());
        }
        self.stacks += 1;
        for child in self.children(depth, cell)? {
            self.visit(depth + 1, child)?;
            if self.stopped.is_some() {
                break;
            }
        }
        Ok(())
    }
}

struct Reconstruction {
    normalized: NormalizedProblem,
    table: Vec<String>,
    levels: Vec<Vec<usize>>,
    stages: Vec<Restoration>,
}

/// Shared exact metadata and restoration stages. Borrowing the already
/// verified base avoids copying its exhaustive trace and cell table.
fn prepare_reconstruction(
    problem: &Problem,
    order: &[String],
    steps: &[MonotoneStep],
    base: &SolveResult,
    negative_only: bool,
) -> Result<Reconstruction> {
    let normalized = normalize(problem, order)?;
    let algebra = &normalized.algebra;
    let base_dimension = order
        .len()
        .checked_sub(steps.len())
        .ok_or_else(|| anyhow!("too many monotone restoration stages"))?;
    ensure!(
        !negative_only || (!steps.is_empty() && base.split_polynomials.is_empty()),
        "negative-target reconstruction needs at least one peel and an unsplit endpoint domain"
    );
    ensure!(
        base.order == order[..base_dimension],
        "base certificate uses an incompatible order"
    );
    ensure!(
        base.projection_levels.len() == base_dimension,
        "base projection has incorrect dimension"
    );
    let mut table = base.polynomials.clone();
    let mut ids = table
        .iter()
        .enumerate()
        .map(|(i, p)| Ok((algebra.parse(p)?, i)))
        .collect::<Result<HashMap<_, _>>>()?;
    let mut levels = base.projection_levels.clone();
    levels.resize(order.len(), vec![]);
    let mut stages = Vec::new();
    for (offset, step) in steps.iter().rev().enumerate() {
        let level = base_dimension + offset;
        ensure!(
            step.variable == order[level],
            "monotone reconstruction order differs"
        );
        let p = algebra.parse(&step.polynomial)?;
        ensure!(
            (level + 1..order.len()).all(|v| p.degree(v) == 0),
            "monotone polynomial is not cylindrical"
        );
        let endpoint = algebra.parse(&step.endpoint)?;
        if !endpoint.is_constant() {
            insert_polynomial(algebra, endpoint.clone(), &mut table, &mut ids, &mut levels)?;
        }
        let coordinate = algebra.parse(&step.variable)?;
        let coordinate_id = insert_polynomial(
            algebra,
            coordinate.clone(),
            &mut table,
            &mut ids,
            &mut levels,
        )?;
        let polynomial_id =
            insert_polynomial(algebra, p.clone(), &mut table, &mut ids, &mut levels)?;
        stages.push(Restoration {
            variable: step.variable.clone(),
            degree: step.degree,
            polynomial: p,
            coordinate,
            zero_endpoint: endpoint.is_zero(),
            coordinate_id,
            polynomial_id,
        });
    }
    Ok(Reconstruction {
        normalized,
        table,
        levels,
        stages,
    })
}

fn restoration_seed(cell: &Cell, base: &SolveResult, negative_only: bool) -> Result<Cell> {
    let mut cell = cell.clone();
    if negative_only {
        ensure!(
            cell.signs.is_empty(),
            "endpoint inequality has unexpected split signs"
        );
        cell.signs = vec![-1];
    } else if base.split_polynomials.is_empty() {
        cell.signs = vec![0];
    }
    Ok(cell)
}

fn reconstruct(
    problem: &Problem,
    order: Vec<String>,
    steps: Vec<MonotoneStep>,
    mut base: SolveResult,
    started: Option<Instant>,
    target: Option<MonotoneTarget>,
) -> Result<SolveResult> {
    let base_dimension = order
        .len()
        .checked_sub(steps.len())
        .ok_or_else(|| anyhow!("too many monotone restoration stages"))?;
    if base.status != Status::CompleteGeneric && base.projection_levels.is_empty() {
        ensure!(
            base.cells.is_empty(),
            "partial cells lack projection levels"
        );
        base.projection_levels.resize(base_dimension, vec![]);
    }
    let data = prepare_reconstruction(problem, &order, &steps, &base, target.is_some())?;
    let algebra = &data.normalized.algebra;
    let mut restorer = Restorer {
        algebra,
        stages: &data.stages,
        started,
        limits: &problem.limits,
        negative_only: target.is_some(),
        cells: RestorationCells::Collect(vec![]),
        stopped: None,
        stacks: 0,
    };
    for cell in &base.cells {
        restorer.visit(0, restoration_seed(cell, &base, target.is_some())?)?;
        if restorer.stopped.is_some() {
            break;
        }
    }
    let RestorationCells::Collect(cells) = restorer.cells else {
        unreachable!("the solving restorer collects its output")
    };
    let merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&cells));
    let mut statistics = base.statistics.clone();
    statistics.raw_cells = cells.len();
    statistics.visited_stacks += restorer.stacks;
    Ok(SolveResult {
        schema_version: 1,
        status: if restorer.stopped.is_some() {
            Status::Incomplete
        } else {
            base.status.clone()
        },
        problem: problem.clone(),
        order,
        normalized_constraints: data
            .normalized
            .positives
            .iter()
            .map(|p| algebra.format(p))
            .collect(),
        split_polynomials: data
            .normalized
            .split
            .iter()
            .map(|p| algebra.format(p))
            .collect(),
        exceptional_polynomials: (0..data.table.len()).collect(),
        polynomials: data.table,
        projection_levels: data.levels,
        cells,
        trace: None,
        statistics,
        diagnostic: restorer.stopped.or_else(|| base.diagnostic.clone()),
        merged_cells,
        homogeneous: None,
        monotone: Some(Box::new(MonotoneEvidence {
            steps,
            base: Box::new(base),
            target,
        })),
        local: None,
        sign_filter: None,
    })
}

pub fn solve_monotone(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let plan = plan(problem)?;
    let planning_seconds = started.elapsed().as_secs_f64();
    let base = crate::solver::solve(&plan.base)?;
    let mut result = reconstruct(problem, plan.order, plan.steps, base, Some(started), None)?;
    result.statistics.ordering_seconds += planning_seconds;
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(result)
}

/// Solve an original strict inequality by retaining only its negative
/// monotone fibers. This is a separate coverage theorem, not a partial sign
/// arrangement: the exact terminal endpoint inequality is a base constraint.
pub fn solve_negative_target(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let plan = negative_plan(problem, Some(started))?;
    let planning_seconds = started.elapsed().as_secs_f64();
    if deadline_reached(problem, Some(started)) {
        let mut result = SolveResult::failure(
            problem.clone(),
            Status::Incomplete,
            "wall time exhausted during negative-target planning",
        );
        result.statistics.ordering_seconds = planning_seconds;
        result.statistics.total_seconds = started.elapsed().as_secs_f64();
        return Ok(result);
    }
    let mut result = if let Some(mut negative) = plan {
        // Narrow only the delegated resource budget. Verification permits this
        // explicitly checked narrowing while reconstructing every mathematical
        // and solver field of the endpoint problem exactly.
        negative.plan.base.limits.wall_time_secs =
            problem.limits.wall_time_secs - started.elapsed().as_secs_f64();
        if negative.plan.base.limits.wall_time_secs <= 0.0 {
            let mut result = SolveResult::failure(
                problem.clone(),
                Status::Incomplete,
                "wall time exhausted before the negative endpoint solve",
            );
            result.statistics.ordering_seconds = planning_seconds;
            result.statistics.total_seconds = started.elapsed().as_secs_f64();
            return Ok(result);
        }
        let base = crate::solver::solve(&negative.plan.base)?;
        let reconstruction_started = Instant::now();
        let mut result = reconstruct(
            problem,
            negative.plan.order,
            negative.plan.steps,
            base,
            Some(started),
            Some(negative.target),
        )?;
        result.statistics.lifting_seconds += reconstruction_started.elapsed().as_secs_f64();
        result
    } else {
        // The historical path receives the unchanged original problem so its
        // complete-arrangement proof remains byte-for-byte reconstructible.
        crate::sign_filter::solve(problem)?
    };
    result.statistics.ordering_seconds += planning_seconds;
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    // Includes planning, delegated work, and reconstruction (even an empty
    // result). Individual algebra calls still require the worker's hard cap.
    if result.status == Status::CompleteGeneric && deadline_reached(problem, Some(started)) {
        result.status = Status::Incomplete;
        result.diagnostic = Some("negative-target cumulative wall time limit".into());
    }
    Ok(result)
}

pub fn verify_monotone(result: &SolveResult) -> Result<Verification> {
    ensure!(
        result.schema_version == 1 && result.status == Status::CompleteGeneric,
        "monotone certificate is not complete_generic"
    );
    ensure!(
        result.problem.solver.engine == "monotone",
        "monotone evidence has a different engine"
    );
    let evidence = result
        .monotone
        .as_ref()
        .ok_or_else(|| anyhow!("missing monotone evidence"))?;
    let expected_base = if let Some(target) = &evidence.target {
        ensure!(
            !evidence.steps.is_empty(),
            "negative-target evidence requires at least one certified peel"
        );
        let transferred = target_problem(&result.problem, target)?;
        let mut base = check_steps(&transferred, &result.order, &evidence.steps)?;
        restrict_negative_endpoint(&mut base, &evidence.steps.last().unwrap().endpoint)?;
        let delegated_wall = evidence.base.problem.limits.wall_time_secs;
        ensure!(
            delegated_wall.is_finite()
                && delegated_wall > 0.0
                && delegated_wall <= result.problem.limits.wall_time_secs,
            "negative endpoint wall budget exceeds the original budget"
        );
        base.limits.wall_time_secs = delegated_wall;
        base
    } else {
        check_steps(&result.problem, &result.order, &evidence.steps)?
    };
    ensure!(
        serde_json::to_value(&evidence.base.problem)? == serde_json::to_value(&expected_base)?,
        "base problem differs from exact endpoint reduction"
    );
    ensure!(
        evidence.base.monotone.is_none() && evidence.base.homogeneous.is_none(),
        "unexpected nested base reduction"
    );
    let base_verification = crate::verify::verify(&evidence.base)?;
    let data = prepare_reconstruction(
        &result.problem,
        &result.order,
        &evidence.steps,
        &evidence.base,
        evidence.target.is_some(),
    )?;
    let algebra = &data.normalized.algebra;
    ensure!(
        result.normalized_constraints
            == data
                .normalized
                .positives
                .iter()
                .map(|p| algebra.format(p))
                .collect::<Vec<_>>()
            && result.split_polynomials
                == data
                    .normalized
                    .split
                    .iter()
                    .map(|p| algebra.format(p))
                    .collect::<Vec<_>>(),
        "original monotone problem metadata was modified"
    );
    ensure!(
        result.polynomials == data.table && result.projection_levels == data.levels,
        "monotone boundary table was modified"
    );
    ensure!(
        result
            .exceptional_polynomials
            .iter()
            .copied()
            .eq(0..data.table.len()),
        "monotone endpoint or singular seams were modified"
    );
    let mut restorer = Restorer {
        algebra,
        stages: &data.stages,
        started: None,
        limits: &result.problem.limits,
        negative_only: evidence.target.is_some(),
        cells: RestorationCells::Check {
            expected: &result.cells,
            matched: 0,
        },
        stopped: None,
        stacks: 0,
    };
    for cell in &evidence.base.cells {
        restorer.visit(
            0,
            restoration_seed(cell, &evidence.base, evidence.target.is_some())?,
        )?;
        if restorer.stopped.is_some() {
            break;
        }
    }
    ensure!(
        restorer.stopped.is_none(),
        "monotone reconstruction is incomplete"
    );
    ensure!(
        restorer.cells.len() == result.cells.len(),
        "monotone branches are missing or modified"
    );
    ensure!(
        result.merged_cells
            == (result.problem.solver.merge == "aggressive").then(|| merge(&result.cells)),
        "monotone compatibility merge differs"
    );
    ensure!(
        result.trace.is_none()
            && result.homogeneous.is_none()
            && result.local.is_none()
            && result.sign_filter.is_none(),
        "unexpected alternate monotone coverage evidence"
    );
    if evidence.target.is_some() {
        ensure!(
            result.statistics.raw_cells == result.cells.len(),
            "negative-target raw-cell count was modified"
        );
    }
    Ok(Verification {
        verified: true,
        cells: result.cells.len(),
        stacks: base_verification.stacks,
        rejected_branches: base_verification.rejected_branches,
    })
}
