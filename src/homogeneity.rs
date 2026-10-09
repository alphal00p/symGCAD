//! Exact positive-scale reduction and reconstruction of cylindrical cells.
//!
//! For an explicit block B and anchor r>0, the map x_i=r*y_i (i in B\{r})
//! is a bijection with inverse y_i=x_i/r. Homogeneous inequalities and split
//! signs change only by positive powers of r. Placing r before every y_i
//! makes this map triangular in the lifting order, so complete reduced cells
//! lift to complete original cylindrical cells, including all excluded seams.

use crate::{
    Problem,
    algebra::Poly,
    domain::{derive_domain, homogeneous_anchor, homogeneous_degree, restore_homogeneous_bound},
    output::{Axis, Cell, RootBound, SolveResult, Status},
    problem::{NormalizedProblem, normalize},
    solver::{merge, selected_order as select_polynomial_order},
    verify::Verification,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HomogeneousEvidence {
    pub anchor: String,
    /// Block names in the original result's lifting order.
    pub block: Vec<String>,
    /// Exact block degrees, original constraints first, then split polynomials.
    pub degrees: Vec<u32>,
    pub forward_ratios: Vec<String>,
    pub inverse_coordinates: Vec<String>,
    /// The full exhaustive certificate in ratio coordinates, without the scale.
    pub reduced: Box<SolveResult>,
}

struct Reduction {
    reduced: Problem,
    anchor: String,
}

/// Build the reduction in the input variable map, independently of a heuristic
/// projection ordering. This avoids computing a hard unreduced MODS projection
/// merely to decide the order of the substantially smaller problem.
fn prepare(problem: &Problem) -> Result<Reduction> {
    problem.validate()?;
    let anchor = problem
        .solver
        .homogeneous_anchor
        .as_ref()
        .ok_or_else(|| anyhow!("homogeneous reduction requires an explicit anchor"))?;
    let normalized = normalize(problem, &problem.variables)?;
    let anchor_index = problem.variables.iter().position(|n| n == anchor).unwrap();
    let block = problem
        .solver
        .homogeneous_block
        .iter()
        .map(|n| problem.variables.iter().position(|v| v == n).unwrap())
        .collect::<Vec<_>>();
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    let domain = derive_domain(&normalized.algebra, &normalized.positives, &[])?;
    let certificate = homogeneous_anchor(
        &normalized.algebra,
        &all,
        &block,
        anchor_index,
        &domain,
    )?
    .ok_or_else(|| anyhow!(
        "homogeneous reduction requires every constraint and split to be homogeneous in the declared block and an original strict positive anchor bound"
    ))?;
    let mut reduced = problem.clone();
    reduced.variables.retain(|v| v != anchor);
    reduced.constraints = certificate.normalized_polynomials[..problem.constraints.len()]
        .iter()
        .map(|p| format!("({p})>0"))
        .collect();
    reduced.split = certificate.normalized_polynomials[problem.constraints.len()..].to_vec();
    // Parameter values were substituted exactly before normalization.
    reduced.parameters.clear();
    reduced.variable_groups = problem
        .variable_groups
        .iter()
        .map(|g| {
            g.iter()
                .filter(|v| *v != anchor)
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|g| !g.is_empty())
        .collect();
    reduced.solver.order.retain(|v| v != anchor);
    reduced.solver.homogeneous_anchor = None;
    reduced.solver.homogeneous_block.clear();
    reduced.validate()?;
    Ok(Reduction {
        reduced,
        anchor: anchor.clone(),
    })
}

fn check_order(problem: &Problem, order: &[String]) -> Result<usize> {
    let mut check = problem.clone();
    check.solver.order = order.to_vec();
    // validate() allows an empty order as shorthand. Here it must be explicit.
    ensure!(
        order.len() == problem.variables.len(),
        "incomplete homogeneous lifting order"
    );
    check.validate()?;
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        ensure!(
            given_order(problem) == order,
            "homogeneous order overrides the requested order"
        );
    }
    let anchor = problem.solver.homogeneous_anchor.as_ref().unwrap();
    let anchor_index = order.iter().position(|v| v == anchor).unwrap();
    ensure!(
        problem
            .solver
            .homogeneous_block
            .iter()
            .all(|v| { v == anchor || order.iter().position(|n| n == v).unwrap() > anchor_index }),
        "homogeneous anchor must precede every other block variable in the lifting order"
    );
    Ok(anchor_index)
}

fn given_order(problem: &Problem) -> Vec<String> {
    if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
        problem.variable_groups.iter().flatten().cloned().collect()
    } else {
        problem.effective_order()
    }
}

fn choose_original_order(problem: &Problem, reduction: &Reduction) -> Result<Vec<String>> {
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        let order = given_order(problem);
        check_order(problem, &order)?;
        return Ok(order);
    }
    let mut order = select_polynomial_order(&reduction.reduced)?;
    let first_ratio = order
        .iter()
        .position(|v| problem.solver.homogeneous_block.contains(v))
        .unwrap_or(order.len());
    let insertion = if problem.variable_groups.is_empty() {
        first_ratio
    } else {
        let anchor_group = problem
            .variable_groups
            .iter()
            .position(|g| g.contains(&reduction.anchor))
            .unwrap();
        order
            .iter()
            .position(|v| {
                problem
                    .variable_groups
                    .iter()
                    .position(|g| g.contains(v))
                    .unwrap()
                    >= anchor_group
            })
            .unwrap_or(order.len())
    };
    ensure!(
        insertion <= first_ratio,
        "variable groups require a block variable before its homogeneous anchor"
    );
    order.insert(insertion, reduction.anchor.clone());
    check_order(problem, &order)?;
    Ok(order)
}

/// Select an admissible order using only the reduced problem's projections.
/// Fixed/given orders retain their exact meaning and fail if incompatible.
pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    choose_original_order(problem, &prepare(problem)?)
}

fn fixed_reduced_problem(mut reduction: Reduction, order: &[String]) -> Problem {
    reduction.reduced.solver.order = order
        .iter()
        .filter(|n| *n != &reduction.anchor)
        .cloned()
        .collect();
    reduction.reduced.solver.ordering = "given".into();
    reduction.reduced
}

struct Reconstruction {
    anchor: usize,
    block: Vec<usize>,
    ratios: Vec<usize>,
    normalized: NormalizedProblem,
    degrees: Vec<u32>,
    forward_ratios: Vec<String>,
    inverse_coordinates: Vec<String>,
}

fn reconstruction(problem: &Problem, order: &[String]) -> Result<Reconstruction> {
    let anchor = check_order(problem, order)?;
    let normalized = normalize(problem, order)?;
    let block: Vec<_> = order
        .iter()
        .enumerate()
        .filter_map(|(i, n)| problem.solver.homogeneous_block.contains(n).then_some(i))
        .collect();
    let ratios: Vec<_> = block.iter().copied().filter(|&v| v != anchor).collect();
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    let domain = derive_domain(&normalized.algebra, &normalized.positives, &[])?;
    let certificate = homogeneous_anchor(&normalized.algebra, &all, &block, anchor, &domain)?
        .ok_or_else(|| {
            anyhow!("original problem does not admit the claimed positive-scale bijection")
        })?;
    let scale = normalized.algebra.parse(&order[anchor])?;
    for (p, &degree) in all.iter().zip(&certificate.degrees) {
        let q = p.replace(anchor, &Rational::one());
        let restored = restore_checked(&q, &block, &ratios, anchor)?;
        let restored_degree = homogeneous_degree(&restored, &block).unwrap();
        ensure!(
            degree >= restored_degree,
            "incorrect original homogeneous degree"
        );
        let identity = &restored * &scale.pow((degree - restored_degree) as usize);
        ensure!(
            identity == *p,
            "original polynomial fails its exact scale identity"
        );
    }
    Ok(Reconstruction {
        anchor,
        block,
        ratios,
        normalized,
        degrees: certificate.degrees,
        forward_ratios: certificate.forward_ratios,
        inverse_coordinates: certificate.inverse_coordinates,
    })
}

fn restore_checked(p: &Poly, block: &[usize], ratios: &[usize], anchor: usize) -> Result<Poly> {
    let restored = restore_homogeneous_bound(p, ratios, anchor)?;
    ensure!(
        homogeneous_degree(&restored, block).is_some(),
        "reconstructed polynomial is not homogeneous"
    );
    ensure!(
        restored.replace(anchor, &Rational::one()) == *p,
        "reconstructed polynomial fails its exact ratio identity"
    );
    Ok(restored)
}

fn shifted_bound(bound: &Option<RootBound>) -> Option<RootBound> {
    bound.as_ref().map(|b| RootBound {
        polynomial: b.polynomial + 1,
        index: b.index,
        lower: b.lower.clone(),
        upper: b.upper.clone(),
    })
}

fn reconstructed_cells(
    reduced: &SolveResult,
    order: &[String],
    anchor: usize,
) -> Result<Vec<Cell>> {
    reduced
        .cells
        .iter()
        .map(|cell| {
            ensure!(
                cell.axes.len() + 1 == order.len(),
                "reduced cell has incorrect dimension"
            );
            let mut axes = Vec::with_capacity(order.len());
            let mut next = 0;
            for (i, variable) in order.iter().enumerate() {
                if i == anchor {
                    axes.push(Axis {
                        variable: variable.clone(),
                        sample: "1".into(),
                        lower: Some(RootBound {
                            polynomial: 0,
                            index: 0,
                            lower: "0".into(),
                            upper: "0".into(),
                        }),
                        upper: None,
                    });
                } else {
                    let axis = &cell.axes[next];
                    ensure!(
                        &axis.variable == variable,
                        "reduced cell order does not match the original order"
                    );
                    for bound in [&axis.lower, &axis.upper].into_iter().flatten() {
                        ensure!(
                            bound.polynomial < reduced.polynomials.len(),
                            "invalid reduced bound polynomial id"
                        );
                    }
                    axes.push(Axis {
                        variable: variable.clone(),
                        // Scale sample is exactly 1: all ratio/nonblock samples,
                        // isolating intervals and distinct root indices coincide.
                        sample: axis.sample.clone(),
                        lower: shifted_bound(&axis.lower),
                        upper: shifted_bound(&axis.upper),
                    });
                    next += 1;
                }
            }
            Ok(Cell {
                axes,
                signs: cell.signs.clone(),
            })
        })
        .collect()
}

fn reconstruct(problem: &Problem, order: Vec<String>, reduced: SolveResult) -> Result<SolveResult> {
    let data = reconstruction(problem, &order)?;
    let algebra = &data.normalized.algebra;
    let reduced_order: Vec<_> = order
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (i != data.anchor).then_some(n.clone()))
        .collect();
    ensure!(
        reduced.order == reduced_order,
        "reduced certificate uses a different order"
    );
    ensure!(
        reduced.projection_levels.len() == reduced_order.len(),
        "reduced projection has incorrect dimension"
    );
    let mut polynomials = vec![algebra.format(&algebra.parse(&order[data.anchor])?)];
    let mut restored = vec![algebra.parse(&order[data.anchor])?];
    for expression in &reduced.polynomials {
        let polynomial = restore_checked(
            &algebra.parse(expression)?,
            &data.block,
            &data.ratios,
            data.anchor,
        )?;
        polynomials.push(algebra.format(&polynomial));
        restored.push(polynomial);
    }
    let mut levels = Vec::with_capacity(order.len());
    let mut next = 0;
    for i in 0..order.len() {
        if i == data.anchor {
            levels.push(vec![0]);
        } else {
            let level = reduced.projection_levels[next]
                .iter()
                .map(|&id| {
                    ensure!(
                        id < reduced.polynomials.len(),
                        "invalid reduced projection polynomial id"
                    );
                    Ok(id + 1)
                })
                .collect::<Result<Vec<_>>>()?;
            levels.push(level);
            next += 1;
        }
        for &id in &levels[i] {
            ensure!(
                restored[id].degree(i) > 0
                    && (i + 1..order.len()).all(|j| restored[id].degree(j) == 0),
                "scale reconstruction would make a bound noncylindrical"
            );
        }
    }
    let cells = reconstructed_cells(&reduced, &order, data.anchor)?;
    let merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&cells));
    let mut statistics = reduced.statistics.clone();
    statistics.raw_cells = cells.len();
    let evidence = HomogeneousEvidence {
        anchor: order[data.anchor].clone(),
        block: data.block.iter().map(|&i| order[i].clone()).collect(),
        degrees: data.degrees,
        forward_ratios: data.forward_ratios,
        inverse_coordinates: data.inverse_coordinates,
        reduced: Box::new(reduced),
    };
    Ok(SolveResult {
        schema_version: 1,
        status: evidence.reduced.status.clone(),
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
        exceptional_polynomials: (0..polynomials.len()).collect(),
        polynomials,
        projection_levels: levels,
        cells,
        trace: None,
        statistics,
        diagnostic: evidence.reduced.diagnostic.clone(),
        merged_cells,
        homogeneous: Some(Box::new(evidence)),
        monotone: None,
    })
}

pub fn solve_homogeneous(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let reduction = prepare(problem)?;
    let order = choose_original_order(problem, &reduction)?;
    let ordering_seconds = started.elapsed().as_secs_f64();
    let reduced_problem = fixed_reduced_problem(reduction, &order);
    let reduced = crate::solver::solve(&reduced_problem)?;
    let mut result = reconstruct(problem, order, reduced)?;
    result.statistics.ordering_seconds += ordering_seconds;
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(result)
}

/// Verify complete reduced coverage, the exact positive-scale bijection, and
/// every deterministic output field. This proves coverage of every original
/// point outside the explicit exceptional set, not merely of chosen samples.
pub fn verify_homogeneous(result: &SolveResult) -> Result<Verification> {
    ensure!(result.schema_version == 1, "unsupported result schema");
    ensure!(
        result.status == Status::CompleteGeneric,
        "homogeneous result is not complete_generic"
    );
    let evidence = result
        .homogeneous
        .as_ref()
        .ok_or_else(|| anyhow!("missing homogeneous evidence"))?;
    let reduction = prepare(&result.problem)?;
    check_order(&result.problem, &result.order)?;
    let expected_reduced = fixed_reduced_problem(reduction, &result.order);
    ensure!(
        serde_json::to_value(&evidence.reduced.problem)?
            == serde_json::to_value(&expected_reduced)?,
        "reduced problem does not follow from the original positive-scale substitution"
    );
    ensure!(
        evidence.reduced.homogeneous.is_none(),
        "unexpected nested homogeneous reduction"
    );
    let verified = crate::verify::verify(&evidence.reduced)?;
    let expected = reconstruct(
        &result.problem,
        result.order.clone(),
        (*evidence.reduced).clone(),
    )?;
    let expected_evidence = expected.homogeneous.as_ref().unwrap();
    ensure!(
        evidence.anchor == expected_evidence.anchor
            && evidence.block == expected_evidence.block
            && evidence.degrees == expected_evidence.degrees
            && evidence.forward_ratios == expected_evidence.forward_ratios
            && evidence.inverse_coordinates == expected_evidence.inverse_coordinates,
        "homogeneous scale identities or metadata were modified"
    );
    ensure!(
        result.normalized_constraints == expected.normalized_constraints
            && result.split_polynomials == expected.split_polynomials,
        "original normalized constraints or split polynomials were modified"
    );
    ensure!(
        result.polynomials == expected.polynomials
            && result.projection_levels == expected.projection_levels,
        "reconstructed boundary table or levels were modified"
    );
    ensure!(
        result.exceptional_polynomials == expected.exceptional_polynomials,
        "reconstructed exceptional set or scale seam was modified"
    );
    ensure!(
        result.cells == expected.cells,
        "reconstructed cells are missing or modified"
    );
    ensure!(
        result.merged_cells == expected.merged_cells,
        "reconstructed compatibility merge was modified"
    );
    ensure!(
        result.trace.is_none(),
        "homogeneous coverage must use its reduced exhaustive certificate"
    );
    // A new positive-scale stack is inserted once per distinct reduced prefix
    // preceding the anchor. This statistic is descriptive; coverage is proved
    // by the reduced trace plus the bijection, independent of these counts.
    let anchor = result
        .order
        .iter()
        .position(|v| v == &evidence.anchor)
        .unwrap();
    let prefixes = result
        .cells
        .iter()
        .map(|cell| serde_json::to_string(&cell.axes[..anchor]))
        .collect::<Result<BTreeSet<_>, _>>()?;
    Ok(Verification {
        verified: true,
        cells: result.cells.len(),
        stacks: verified.stacks + prefixes.len(),
        rejected_branches: verified.rejected_branches,
    })
}
