use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, canonical, rational},
    homogeneity::{selected_order, solve_homogeneous, verify_homogeneous},
    output::{LiftTrace, Status},
    solver::solve,
    verify::verify,
};

fn problem(
    variables: &[&str],
    constraints: &[&str],
    split: &[&str],
    block: &[&str],
    anchor: &str,
) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|s| (*s).into()).collect(),
        constraints: constraints.iter().map(|s| (*s).into()).collect(),
        split: split.iter().map(|s| (*s).into()).collect(),
        ..Problem::default()
    };
    p.solver.homogeneous_block = block.iter().map(|s| (*s).into()).collect();
    p.solver.homogeneous_anchor = Some(anchor.into());
    p
}

fn basic() -> Problem {
    problem(&["r", "x"], &["r>0", "x>0"], &["x^2-r^2"], &["r", "x"], "r")
}

#[test]
fn positive_scale_reduction_reconstructs_original_cells_and_singular_seams() {
    let p = basic();
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert_eq!(result.cells.len(), 2);
    assert!(result.trace.is_none());
    assert!(verify(&result).unwrap().verified);
    let evidence = result.homogeneous.as_ref().unwrap();
    assert_eq!(evidence.anchor, "r");
    assert_eq!(evidence.block, vec!["r", "x"]);
    assert_eq!(evidence.degrees, vec![1, 1, 2]);
    assert_eq!(evidence.forward_ratios, vec!["x/r"]);
    assert_eq!(evidence.inverse_coordinates, vec!["r*x"]);
    assert_eq!(evidence.reduced.order, vec!["x"]);
    assert!(evidence.reduced.homogeneous.is_none());
    assert!(evidence.reduced.problem.solver.homogeneous_anchor.is_none());
    let a = Algebra::new(&result.order).unwrap();
    assert_eq!(
        a.parse(&result.polynomials[0]).unwrap(),
        a.parse("r").unwrap()
    );
    assert!(result.exceptional_polynomials.contains(&0));
    assert!(
        result
            .polynomials
            .iter()
            .any(|p| canonical(&a.parse(p).unwrap()) == canonical(&a.parse("x-r").unwrap()))
    );
    for (cell, reduced) in result.cells.iter().zip(&evidence.reduced.cells) {
        assert_eq!(cell.axes[0].sample, "1");
        assert_eq!(cell.axes[0].lower.as_ref().unwrap().polynomial, 0);
        assert_eq!(cell.axes[0].lower.as_ref().unwrap().index, 0);
        assert!(cell.axes[0].upper.is_none());
        assert_eq!(cell.axes[1].sample, reduced.axes[0].sample);
        assert_eq!(cell.signs, reduced.signs);
        for (bound, reduced_bound) in [
            (&cell.axes[1].lower, &reduced.axes[0].lower),
            (&cell.axes[1].upper, &reduced.axes[0].upper),
        ] {
            if let (Some(bound), Some(reduced_bound)) = (bound, reduced_bound) {
                assert_eq!(bound.polynomial, reduced_bound.polynomial + 1);
                assert_eq!(bound.index, reduced_bound.index);
                assert_eq!(bound.lower, reduced_bound.lower);
                assert_eq!(bound.upper, reduced_bound.upper);
            }
        }
    }
    let serialized = serde_json::to_string(&result).unwrap();
    let decoded = serde_json::from_str(&serialized).unwrap();
    assert!(verify(&decoded).unwrap().verified);
}

#[test]
fn nonblock_axes_before_and_after_scale_keep_cylindrical_bounds() {
    for p in [
        problem(
            &["s", "r", "x"],
            &["s>0", "r>0", "x>0"],
            &["x-s*r"],
            &["r", "x"],
            "r",
        ),
        problem(
            &["r", "x", "t"],
            &["r>0", "x>0", "t>0"],
            &["x-t*r"],
            &["r", "x"],
            "r",
        ),
    ] {
        let result = solve(&p).unwrap();
        assert!(verify(&result).unwrap().verified);
        assert_eq!(result.cells.len(), 2);
        let a = Algebra::new(&result.order).unwrap();
        for (level, ids) in result.projection_levels.iter().enumerate() {
            for &id in ids {
                let polynomial = a.parse(&result.polynomials[id]).unwrap();
                assert!(polynomial.degree(level) > 0);
                assert!((level + 1..result.order.len()).all(|later| polynomial.degree(later) == 0));
            }
        }
    }
    let p = problem(
        &["r", "x", "t"],
        &["r>0", "x>0", "t>0"],
        &["x-t*r"],
        &["r", "x"],
        "r",
    );
    let result = solve(&p).unwrap();
    let a = Algebra::new(&result.order).unwrap();
    let boundary = result
        .polynomials
        .iter()
        .map(|p| a.parse(p).unwrap())
        .find(|p| p.degree(2) > 0 && p.degree(0) > 0)
        .unwrap();
    // A nonblock root is t=x/r: x scales, t does not.
    assert!(
        boundary
            .replace_all(&[Rational::from(2), Rational::from(6), Rational::from(3)])
            .is_zero()
    );
}

#[test]
fn singleton_block_reduces_to_an_exact_zero_dimensional_problem() {
    let p = problem(&["r"], &["r>0"], &["r", "-r^2"], &["r"], "r");
    let result = solve(&p).unwrap();
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![1, -1]);
    assert!(
        result
            .homogeneous
            .as_ref()
            .unwrap()
            .reduced
            .order
            .is_empty()
    );
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn dynamic_ordering_projects_only_reduced_variables_and_respects_groups() {
    let mut p = problem(
        &["s", "x", "r", "y"],
        &["s>4", "r>0", "x>0", "y>0"],
        &["x+y-s*r"],
        &["r", "x", "y"],
        "r",
    );
    p.variable_groups = vec![vec!["s".into()], vec!["x".into(), "r".into(), "y".into()]];
    p.solver.ordering = "mods".into();
    let order = selected_order(&p).unwrap();
    assert_eq!(&order[..2], &["s", "r"]);
    let result = solve(&p).unwrap();
    assert_eq!(result.order, order);
    assert!(verify(&result).unwrap().verified);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert_eq!(reduced.problem.solver.ordering, "given");
    assert_eq!(
        reduced.order,
        result
            .order
            .iter()
            .filter(|v| *v != "r")
            .cloned()
            .collect::<Vec<_>>()
    );
}

#[test]
fn incompatible_fixed_given_or_group_orders_are_rejected() {
    let mut p = basic();
    p.solver.order = vec!["x".into(), "r".into()];
    assert!(solve_homogeneous(&p).is_err());
    p.solver.order.clear();
    p.variables.reverse();
    assert!(solve_homogeneous(&p).is_err());
    p.solver.ordering = "brown".into();
    p.variable_groups = vec![vec!["x".into()], vec!["r".into()]];
    assert!(selected_order(&p).is_err());
    assert!(solve(&p).is_err());
}

#[test]
fn nonhomogeneous_constraints_or_splits_and_unproved_positive_scales_are_rejected() {
    let mut p = basic();
    p.constraints.push("1-x>0".into());
    assert!(solve(&p).is_err());
    p = basic();
    p.split.push("x-r+1".into());
    assert!(solve(&p).is_err());
    p = basic();
    p.constraints = vec!["r*x>0".into(), "x>0".into()];
    assert!(solve(&p).is_err());
    p = basic();
    p.constraints[0] = "r<0".into();
    assert!(solve(&p).is_err());
}

#[test]
fn parameters_are_substituted_exactly_before_the_bijection_is_checked() {
    let mut p = basic();
    p.parameters.insert("m".into(), "3/2".into());
    p.split = vec!["x-m*r".into()];
    let result = solve(&p).unwrap();
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert!(reduced.problem.parameters.is_empty());
    let a = Algebra::new(&reduced.order).unwrap();
    assert_eq!(
        a.parse(&reduced.split_polynomials[0]).unwrap(),
        a.parse("2*x-3").unwrap()
    );
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn optional_merge_preserves_raw_cells_and_even_multiplicity_singular_seams() {
    let mut p = basic();
    p.split = vec!["(x-r)^2".into()];
    p.solver.merge = "aggressive".into();
    let result = solve(&p).unwrap();
    assert_eq!(result.cells.len(), 2);
    assert_eq!(result.merged_cells.as_ref().unwrap().len(), 1);
    assert!(result.cells.iter().all(|c| c.signs == vec![1]));
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn incomplete_reduced_lifting_is_never_reported_as_complete() {
    let mut p = basic();
    p.limits.max_cells = Some(1);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert_eq!(
        result.homogeneous.as_ref().unwrap().reduced.status,
        Status::Incomplete
    );
    assert!(verify(&result).is_err());
    let mut forged = result;
    forged.status = Status::CompleteGeneric;
    assert!(verify(&forged).is_err());
}

#[test]
fn original_false_constraint_still_has_a_complete_empty_decomposition() {
    let mut p = basic();
    p.constraints.push("false".into());
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(result.cells.is_empty());
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn checker_rejects_changed_bijections_boundaries_cells_and_reduced_coverage() {
    let result = solve(&basic()).unwrap();
    let mut bad = result.clone();
    bad.cells.pop();
    assert!(verify_homogeneous(&bad).is_err());
    let mut bad = result.clone();
    bad.cells[0].axes[0].lower.as_mut().unwrap().index = 1;
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.polynomials[0] = "r-1".into();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.exceptional_polynomials.retain(|&id| id != 0);
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.homogeneous.as_mut().unwrap().degrees[0] += 1;
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.homogeneous.as_mut().unwrap().forward_ratios[0] = "x*r".into();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.homogeneous.as_mut().unwrap().reduced.cells.pop();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.homogeneous
        .as_mut()
        .unwrap()
        .reduced
        .problem
        .constraints
        .push("x>2".into());
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.order.reverse();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.trace = Some(LiftTrace::Unfinished {
        reason: "forged".into(),
    });
    assert!(verify(&bad).is_err());
    let mut bad = result;
    bad.cells[0].axes[0].sample = rational("2").unwrap().to_string();
    assert!(verify(&bad).is_err());
}
