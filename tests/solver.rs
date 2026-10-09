use symgcad::{
    Problem,
    output::{LiftTrace, Status},
    solver::solve,
    verify::verify,
};
fn run(variables: &[&str], constraints: &[&str], split: &[&str]) -> symgcad::output::SolveResult {
    let p = Problem {
        variables: variables.iter().map(|s| s.to_string()).collect(),
        constraints: constraints.iter().map(|s| s.to_string()).collect(),
        split: split.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    let result = solve(&p).unwrap();
    verify(&result).unwrap();
    result
}
#[test]
fn strict_zero_and_contradiction_are_empty() {
    assert_eq!(run(&[], &[], &["0"]).cells[0].signs, vec![0]);
    assert!(run(&[], &["0 > 0"], &[]).cells.is_empty());
    assert!(run(&["x"], &["0 > 0"], &[]).cells.is_empty());
    assert!(run(&["x"], &["x>0", "x<0"], &[]).cells.is_empty());
    assert_eq!(run(&["x"], &[], &[]).cells.len(), 1);
}
#[test]
fn repeated_zero_seams_are_retained() {
    let result = run(&["x"], &["x^2>0"], &[]);
    assert_eq!(result.cells.len(), 2);
    assert!(result.merged_cells.is_none());
}
#[test]
fn narrow_disconnected_components_and_close_roots() {
    let result = run(&["x"], &["(x-1)*(1000000000000*x-1000000000001)<0"], &[]);
    assert_eq!(result.cells.len(), 1);
    let result = run(&["x"], &["(x^2-1)*(x^2-4)<0"], &[]);
    assert_eq!(result.cells.len(), 2);
}
#[test]
fn shared_roots_and_inactive_axis() {
    let result = run(&["x", "y"], &["x^2-1>0", "x-1>0"], &[]);
    assert_eq!(result.cells.len(), 1);
    assert!(result.cells[0].axes[1].lower.is_none());
}
#[test]
fn split_square_covers_both_signs() {
    let result = run(&["x", "y"], &["x>0", "x<1", "y>0", "y<1"], &["x+y-1"]);
    assert_eq!(result.cells.len(), 2);
    assert!(result.cells.iter().any(|c| c.signs == vec![-1]));
    assert!(result.cells.iter().any(|c| c.signs == vec![1]));
}
#[test]
fn verifier_detects_missing_branch_and_projection() {
    let mut result = run(&["x", "y"], &["y^2-x>0"], &[]);
    if let Some(LiftTrace::Stack { sectors, .. }) = &mut result.trace {
        sectors.pop();
    } else {
        panic!("expected stack");
    }
    assert!(verify(&result).is_err());
    let mut result = run(&["x", "y"], &["y^2-x>0"], &[]);
    result.projection_levels[0].clear();
    assert!(verify(&result).is_err());
}
#[test]
fn resource_limited_output_never_verifies_complete() {
    let mut p = Problem {
        variables: vec!["x".into()],
        constraints: vec!["x^2>0".into()],
        ..Default::default()
    };
    p.limits.max_cells = Some(1);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(verify(&result).is_err());
}
#[test]
fn exact_parameter_specialization_and_json_roundtrip() {
    let mut p = Problem {
        variables: vec!["x".into()],
        constraints: vec!["x>mass".into(), "x<1".into()],
        ..Default::default()
    };
    p.parameters.insert("mass".into(), "1/3".into());
    let result = solve(&p).unwrap();
    let restored = serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    verify(&restored).unwrap();
}

#[test]
fn optimized_projection_keeps_raw_cells_and_certifies_pruning() {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "y>0".into()],
        split: vec!["y^2+x*y+1".into()],
        ..Default::default()
    };
    let baseline = solve(&p).unwrap();
    p.solver.engine = "optimized".into();
    let optimized = solve(&p).unwrap();
    assert_eq!(baseline.polynomials, optimized.polynomials);
    assert_eq!(baseline.cells, optimized.cells);
    verify(&optimized).unwrap();
    p.solver.positivity = true;
    let pruned = solve(&p).unwrap();
    verify(&pruned).unwrap();
    assert!(
        !pruned
            .statistics
            .optimized_projection
            .as_ref()
            .unwrap()
            .removals
            .is_empty()
    );
    let mut broken = pruned;
    broken
        .statistics
        .optimized_projection
        .as_mut()
        .unwrap()
        .removals
        .clear();
    assert!(verify(&broken).is_err());
}

#[test]
fn ordering_work_is_reused_by_optimized_final_projection() {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into(), "z".into()],
        constraints: vec!["x>0".into(), "y>0".into(), "z>0".into()],
        split: vec!["x+y+z-1".into()],
        ..Default::default()
    };
    p.solver.engine = "optimized".into();
    p.solver.ordering = "mods".into();
    p.solver.positivity = true;
    let result = solve(&p).unwrap();
    verify(&result).unwrap();
    assert!(
        result
            .statistics
            .optimized_projection
            .unwrap()
            .cache
            .prefix_memory_hits
            >= 2
    );
}

#[test]
fn denominator_clearing_preserves_strict_negative_signs() {
    let result = run(&["x"], &["-x/3>0", "x/7>-1/14"], &["-2*x/9"]);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![1]);
    let normalized = symgcad::problem::normalize(&result.problem, &result.order).unwrap();
    assert_eq!(
        normalized.positives[0],
        normalized.algebra.parse("-x").unwrap()
    );
    assert!(
        normalized
            .positives
            .iter()
            .flat_map(|p| &p.coefficients)
            .all(|c| c.is_integer())
    );
}

#[test]
fn verifier_rejects_dropped_unconstrained_dimensions() {
    let mut result = run(&[], &[], &[]);
    result.problem.variables.push("x".into());
    assert!(verify(&result).is_err());
    let problem = Problem {
        variables: vec!["x".into()],
        ..Default::default()
    };
    assert!(symgcad::problem::normalize(&problem, &[]).is_err());
}

#[test]
fn certified_positive_input_is_removed_without_changing_its_sign() {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "y>0".into()],
        split: vec!["-x^2-x*y-y^2".into()],
        ..Default::default()
    };
    p.solver.engine = "optimized".into();
    p.solver.positivity = true;
    p.solver.input_positivity = true;
    let result = solve(&p).unwrap();
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![-1]);
    assert_eq!(
        result
            .statistics
            .optimized_projection
            .as_ref()
            .unwrap()
            .initial_removals
            .len(),
        1
    );
    verify(&result).unwrap();
    let mut broken = result;
    broken
        .statistics
        .optimized_projection
        .as_mut()
        .unwrap()
        .initial_removals
        .clear();
    assert!(verify(&broken).is_err());
}
