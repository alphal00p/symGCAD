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
}

struct Plan {
    order: Vec<String>,
    steps: Vec<MonotoneStep>,
    base: Problem,
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

fn plan(problem: &Problem) -> Result<Plan> {
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
    let mut base = base_problem(problem, &normalized, &steps, &p)?;
    let mut order = crate::solver::selected_order(&base)?;
    base.solver.order = order.clone();
    base.solver.ordering = "given".into();
    order.extend(steps.iter().rev().map(|s| s.variable.clone()));
    check_order(problem, &order)?;
    Ok(Plan { order, steps, base })
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

struct Restorer<'a> {
    algebra: &'a Algebra,
    stages: &'a [Restoration],
    started: Option<Instant>,
    limits: &'a crate::Limits,
    cells: Vec<Cell>,
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

    fn visit(&mut self, depth: usize, cell: Cell) -> Result<()> {
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
            self.cells.push(cell);
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

fn reconstruct(
    problem: &Problem,
    order: Vec<String>,
    steps: Vec<MonotoneStep>,
    mut base: SolveResult,
    started: Option<Instant>,
) -> Result<SolveResult> {
    let normalized = normalize(problem, &order)?;
    let algebra = &normalized.algebra;
    let base_dimension = order.len() - steps.len();
    ensure!(
        base.order == order[..base_dimension],
        "base certificate uses an incompatible order"
    );
    if base.status != Status::CompleteGeneric && base.projection_levels.is_empty() {
        ensure!(
            base.cells.is_empty(),
            "partial cells lack projection levels"
        );
        base.projection_levels.resize(base_dimension, vec![]);
    }
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
    let mut restorer = Restorer {
        algebra,
        stages: &stages,
        started,
        limits: &problem.limits,
        cells: vec![],
        stopped: None,
        stacks: 0,
    };
    for cell in &base.cells {
        let mut cell = cell.clone();
        if base.split_polynomials.is_empty() {
            cell.signs = vec![0];
        }
        restorer.visit(0, cell)?;
        if restorer.stopped.is_some() {
            break;
        }
    }
    let cells = restorer.cells;
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
        normalized_constraints: normalized
            .positives
            .iter()
            .map(|p| algebra.format(p))
            .collect(),
        split_polynomials: normalized.split.iter().map(|p| algebra.format(p)).collect(),
        exceptional_polynomials: (0..table.len()).collect(),
        polynomials: table,
        projection_levels: levels,
        cells,
        trace: None,
        statistics,
        diagnostic: restorer.stopped.or_else(|| base.diagnostic.clone()),
        merged_cells,
        homogeneous: None,
        monotone: Some(Box::new(MonotoneEvidence {
            steps,
            base: Box::new(base),
        })),
        local: None,
    })
}

pub fn solve_monotone(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let plan = plan(problem)?;
    let planning_seconds = started.elapsed().as_secs_f64();
    let base = crate::solver::solve(&plan.base)?;
    let mut result = reconstruct(problem, plan.order, plan.steps, base, Some(started))?;
    result.statistics.ordering_seconds += planning_seconds;
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
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
    let expected_base = check_steps(&result.problem, &result.order, &evidence.steps)?;
    ensure!(
        serde_json::to_value(&evidence.base.problem)? == serde_json::to_value(&expected_base)?,
        "base problem differs from exact endpoint reduction"
    );
    ensure!(
        evidence.base.monotone.is_none() && evidence.base.homogeneous.is_none(),
        "unexpected nested base reduction"
    );
    let base_verification = crate::verify::verify(&evidence.base)?;
    let expected = reconstruct(
        &result.problem,
        result.order.clone(),
        evidence.steps.clone(),
        (*evidence.base).clone(),
        None,
    )?;
    ensure!(
        expected.status == Status::CompleteGeneric,
        "monotone reconstruction is incomplete"
    );
    ensure!(
        result.normalized_constraints == expected.normalized_constraints
            && result.split_polynomials == expected.split_polynomials,
        "original monotone problem metadata was modified"
    );
    ensure!(
        result.polynomials == expected.polynomials
            && result.projection_levels == expected.projection_levels,
        "monotone boundary table was modified"
    );
    ensure!(
        result.exceptional_polynomials == expected.exceptional_polynomials,
        "monotone endpoint or singular seams were modified"
    );
    ensure!(
        result.cells == expected.cells,
        "monotone branches are missing or modified"
    );
    ensure!(
        result.merged_cells == expected.merged_cells,
        "monotone compatibility merge differs"
    );
    ensure!(
        result.trace.is_none() && result.homogeneous.is_none(),
        "unexpected alternate monotone coverage evidence"
    );
    Ok(Verification {
        verified: true,
        cells: result.cells.len(),
        stacks: base_verification.stacks,
        rejected_branches: base_verification.rejected_branches,
    })
}
