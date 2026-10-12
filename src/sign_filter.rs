//! Exact restriction of a complete sign arrangement to an input inequality.
//!
//! Moving `p > 0` from the domain to an oriented split lets the monotone engine inspect
//! the whole fiber. The original problem is recovered by an exhaustive cell
//! selection; its excluded algebraic seams remain those of the arrangement.
use crate::{
    Problem,
    algebra::Poly,
    output::{Cell, SolveResult, Status},
    problem::normalize,
    verify::Verification,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignFilterEvidence {
    /// Index in the original, ordered strict constraint list.
    pub constraint_index: usize,
    /// Primitive polynomial defining the original strict positive constraint,
    /// expressed in the original variable map without changing its sign.
    pub normalized_target: String,
    /// The arrangement split is exactly orientation * normalized_target.
    #[serde(default = "positive_orientation")]
    pub orientation: i8,
    /// To recover target > 0, this must equal orientation. Retained explicitly
    /// so the selected sign is visible without inferring it from a heuristic.
    #[serde(default = "positive_orientation")]
    pub selected_sign: i8,
    pub arrangement: Box<SolveResult>,
    /// Every arrangement cell with selected_sign, in original cell order.
    pub selected_cells: Vec<usize>,
}

struct Plan {
    constraint_index: usize,
    normalized_target: String,
    orientation: i8,
    arrangement: Problem,
}

fn positive_orientation() -> i8 {
    1
}

fn coordinate_affine(polynomial: &Poly) -> bool {
    let mut active = None;
    for variable in 0..polynomial.nvars() {
        let degree = polynomial.degree(variable);
        if degree > 1 || (degree == 1 && active.is_some()) {
            return false;
        }
        if degree == 1 {
            active = Some(variable);
        }
    }
    true
}

fn oriented_plan(problem: &Problem, orientation: i8) -> Result<Plan> {
    ensure!(
        [-1, 1].contains(&orientation),
        "sign-filter orientation must be +1 or -1"
    );
    ensure!(
        problem.solver.engine == "monotone" && problem.split.is_empty(),
        "sign filtering requires an unsplit monotone problem"
    );
    let normalized = normalize(problem, &problem.variables)?;
    let (constraint_index, target) = normalized
        .positives
        .iter()
        .enumerate()
        .find(|(_, p)| !coordinate_affine(p))
        .ok_or_else(|| {
            anyhow!(
                "unsplit monotone sign filtering requires a non-coordinate-affine strict constraint"
            )
        })?;
    let normalized_target = normalized.algebra.format(target);
    let mut arrangement = problem.clone();
    arrangement.constraints.remove(constraint_index);
    arrangement.split = vec![if orientation == 1 {
        normalized_target.clone()
    } else {
        normalized.algebra.format(&(-target.clone()))
    }];
    arrangement.validate()?;
    Ok(Plan {
        constraint_index,
        normalized_target,
        orientation,
        arrangement,
    })
}

fn plan(problem: &Problem) -> Result<Plan> {
    let original = oriented_plan(problem, 1)?;
    if crate::monotone::has_first_peel(&original.arrangement)? {
        return Ok(original);
    }
    let reversed = oriented_plan(problem, -1)?;
    if crate::monotone::has_first_peel(&reversed.arrangement)? {
        Ok(reversed)
    } else {
        // Orientation never changes the mathematical problem. Preserve the
        // original sign when neither conservative probe proves an advantage.
        Ok(original)
    }
}

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    crate::solver::selected_order(&plan(problem)?.arrangement)
}

/// The arrangement problem that `solve` delegates. Used only by heuristic
/// projection probes; the solved certificate is selected and checked anew.
pub(crate) fn planned_arrangement(problem: &Problem) -> Result<Problem> {
    Ok(plan(problem)?.arrangement)
}

fn selected_indices(arrangement: &SolveResult, selected_sign: i8) -> Result<Vec<usize>> {
    ensure!(
        [-1, 1].contains(&selected_sign),
        "selected arrangement sign must be +1 or -1"
    );
    let mut selected = Vec::new();
    for (index, cell) in arrangement.cells.iter().enumerate() {
        ensure!(
            cell.signs.as_slice() == [1] || cell.signs.as_slice() == [-1],
            "sign arrangement cell lacks one nonzero target sign"
        );
        if cell.signs[0] == selected_sign {
            selected.push(index);
        }
    }
    Ok(selected)
}

fn filtered_cells(arrangement: &SolveResult, selected: &[usize]) -> Vec<Cell> {
    selected
        .iter()
        .map(|&index| Cell {
            axes: arrangement.cells[index].axes.clone(),
            signs: Vec::new(),
        })
        .collect()
}

pub fn solve(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let plan = plan(problem)?;
    solve_plan(problem, plan, started)
}

fn solve_plan(problem: &Problem, plan: Plan, started: Instant) -> Result<SolveResult> {
    let planning_seconds = started.elapsed().as_secs_f64();
    if planning_seconds >= problem.limits.wall_time_secs {
        let mut result = SolveResult::failure(
            problem.clone(),
            Status::Incomplete,
            "wall time exhausted during sign-filter planning",
        );
        result.statistics.ordering_seconds = planning_seconds;
        result.statistics.total_seconds = started.elapsed().as_secs_f64();
        return Ok(result);
    }
    let arrangement = crate::solver::solve(&plan.arrangement)?;
    // An incomplete arrangement may contain diagnostic partial cells, but its
    // selection must never turn that result into a completed decomposition.
    let selected_cells = selected_indices(&arrangement, plan.orientation)?;
    let cells = filtered_cells(&arrangement, &selected_cells);
    let normalized = normalize(problem, &arrangement.order)?;
    let mut statistics = arrangement.statistics.clone();
    statistics.ordering_seconds += planning_seconds;
    statistics.raw_cells = cells.len();
    let merge_started = Instant::now();
    let merged_cells = (problem.solver.merge == "aggressive").then(|| crate::solver::merge(&cells));
    statistics.merging_seconds += merge_started.elapsed().as_secs_f64();
    statistics.total_seconds = started.elapsed().as_secs_f64();
    let mut result = SolveResult {
        schema_version: 1,
        status: arrangement.status.clone(),
        problem: problem.clone(),
        order: arrangement.order.clone(),
        normalized_constraints: normalized
            .positives
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect(),
        split_polynomials: vec![],
        polynomials: arrangement.polynomials.clone(),
        projection_levels: arrangement.projection_levels.clone(),
        exceptional_polynomials: arrangement.exceptional_polynomials.clone(),
        cells,
        trace: None,
        statistics,
        diagnostic: arrangement.diagnostic.clone(),
        merged_cells,
        homogeneous: None,
        monotone: None,
        local: None,
        linear_negative: None,
        bilinear_positive: None,
        quadratic_ladder: None,
        bilinear_general: None,
        sign_filter: Some(Box::new(SignFilterEvidence {
            constraint_index: plan.constraint_index,
            normalized_target: plan.normalized_target,
            orientation: plan.orientation,
            selected_sign: plan.orientation,
            arrangement: Box::new(arrangement),
            selected_cells,
        })),
    };
    // Keep the arrangement problem exact and replayable. Its own soft budget
    // starts at the delegated call, so the outer budget must also account for
    // planning and copying/filtering the resulting certificate. A hard bound
    // around an individual algebra call remains the worker supervisor's job.
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    if result.statistics.total_seconds >= problem.limits.wall_time_secs
        && result.status == Status::CompleteGeneric
    {
        result.status = Status::Incomplete;
        result.diagnostic =
            Some("wall time exhausted during sign-filter solve or selection".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_positive_selection_replays_even_when_new_heuristic_would_reverse() {
        let mut p = Problem {
            variables: vec!["z".into()],
            constraints: vec!["z>0".into(), "z^2+z-1<0".into()],
            ..Default::default()
        };
        p.solver.engine = "monotone".into();
        assert_eq!(plan(&p).unwrap().orientation, -1);
        let legacy = solve_plan(&p, oriented_plan(&p, 1).unwrap(), Instant::now()).unwrap();
        assert!(verify(&legacy).unwrap().verified);
        let mut encoded = serde_json::to_value(legacy).unwrap();
        let evidence = encoded["sign_filter"].as_object_mut().unwrap();
        evidence.remove("orientation");
        evidence.remove("selected_sign");
        let restored: SolveResult = serde_json::from_value(encoded).unwrap();
        assert_eq!(restored.sign_filter.as_ref().unwrap().orientation, 1);
        assert!(verify(&restored).unwrap().verified);
    }
}

pub fn verify(result: &SolveResult) -> Result<Verification> {
    ensure!(
        result.schema_version == 1 && result.status == Status::CompleteGeneric,
        "sign-filter certificate is not complete_generic"
    );
    ensure!(
        result.trace.is_none()
            && result.homogeneous.is_none()
            && result.monotone.is_none()
            && result.local.is_none()
            && result.linear_negative.is_none()
            && result.bilinear_positive.is_none()
            && result.bilinear_general.is_none(),
        "unexpected alternate sign-filter coverage evidence"
    );
    let evidence = result
        .sign_filter
        .as_ref()
        .ok_or_else(|| anyhow!("missing sign-filter evidence"))?;
    // Replay the recorded exact identity, never the evolving discovery
    // heuristic. Historical evidence with absent fields means orientation+1.
    let plan = oriented_plan(&result.problem, evidence.orientation)?;
    ensure!(
        evidence.selected_sign == evidence.orientation,
        "selected sign does not recover the original strict constraint"
    );
    ensure!(
        evidence.constraint_index == plan.constraint_index
            && evidence.normalized_target == plan.normalized_target,
        "selected original sign constraint was modified"
    );
    ensure!(
        serde_json::to_value(&evidence.arrangement.problem)?
            == serde_json::to_value(&plan.arrangement)?,
        "arrangement problem differs from the exact sign-constraint transfer"
    );
    ensure!(
        evidence.arrangement.sign_filter.is_none(),
        "unexpected nested sign filtering"
    );
    let inner = crate::verify::verify(&evidence.arrangement)?;
    ensure!(
        result.order == evidence.arrangement.order,
        "sign-filter order differs from the verified arrangement"
    );
    let normalized = normalize(&result.problem, &result.order)?;
    let constraints: Vec<_> = normalized
        .positives
        .iter()
        .map(|p| normalized.algebra.format(p))
        .collect();
    ensure!(
        result.normalized_constraints == constraints && result.split_polynomials.is_empty(),
        "original sign-filter problem metadata was modified"
    );
    ensure!(
        result.polynomials == evidence.arrangement.polynomials
            && result.projection_levels == evidence.arrangement.projection_levels
            && result.exceptional_polynomials == evidence.arrangement.exceptional_polynomials,
        "sign-filter boundary or exceptional-set metadata was modified"
    );
    let expected_indices = selected_indices(&evidence.arrangement, evidence.selected_sign)?;
    ensure!(
        evidence.selected_cells == expected_indices,
        "signed-cell selection is not exhaustive or was modified"
    );
    ensure!(
        result.cells.len() == expected_indices.len(),
        "selected-sign cells are missing or duplicated"
    );
    for (cell, &index) in result.cells.iter().zip(&expected_indices) {
        ensure!(
            cell.axes == evidence.arrangement.cells[index].axes && cell.signs.is_empty(),
            "selected cell geometry or split annotation was modified"
        );
    }
    let expected_merge =
        (result.problem.solver.merge == "aggressive").then(|| crate::solver::merge(&result.cells));
    ensure!(
        result.merged_cells == expected_merge,
        "sign-filter compatibility merge differs"
    );
    ensure!(
        result.statistics.raw_cells == result.cells.len(),
        "sign-filter raw-cell count differs"
    );
    Ok(Verification {
        verified: true,
        cells: result.cells.len(),
        stacks: inner.stacks,
        rejected_branches: inner.rejected_branches + evidence.arrangement.cells.len()
            - expected_indices.len(),
    })
}
