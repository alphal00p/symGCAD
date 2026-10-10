use symgcad::{
    Problem,
    algebra::Algebra,
    monotone::{PeelInspection, inspect_peels},
    solver::solve,
    verify::verify,
};

fn problem() -> Problem {
    let mut p = Problem {
        variables: vec!["x".into(), "z".into()],
        constraints: vec!["x>0".into(), "z>0".into()],
        split: vec!["z^2+(x-1)*z+x^2-2*x".into()],
        ..Problem::default()
    };
    p.solver.engine = "monotone".into();
    p.solver.ordering = "given".into();
    p
}

#[test]
fn inspection_matches_the_solved_chain_and_exact_endpoint_without_a_coverage_status() {
    let p = problem();
    let before = serde_json::to_value(&p).unwrap();
    let inspected = inspect_peels(&p).unwrap();
    assert_eq!(inspected.provisional_order, vec!["x", "z"]);
    assert_eq!(inspected.steps.len(), 1);
    assert!(inspected.steps[0].conditional_linear.is_some());
    assert_eq!(inspected.base.variables, vec!["x"]);
    let a = Algebra::new(&inspected.base.variables).unwrap();
    assert_eq!(
        a.parse(&inspected.base.split[0]).unwrap(),
        a.parse("x^2-2*x").unwrap()
    );
    let base = symgcad::problem::normalize(&inspected.base, &inspected.base.variables).unwrap();
    assert_eq!(base.positives, vec![a.parse("x").unwrap()]);
    let result = solve(&p).unwrap();
    assert!(verify(&result).unwrap().verified);
    assert_eq!(
        serde_json::to_value(&inspected.steps).unwrap(),
        serde_json::to_value(&result.monotone.as_ref().unwrap().steps).unwrap()
    );
    let json = serde_json::to_value(&inspected).unwrap();
    assert!(json.get("status").is_none() && json.get("cells").is_none());
    let decoded: PeelInspection = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.provisional_order, inspected.provisional_order);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
}

#[test]
fn provisional_order_uses_groups_for_unfixed_inputs_and_preserves_fixed_orders() {
    let mut p = problem();
    p.variables = vec!["z".into(), "u".into(), "x".into()];
    p.constraints.push("u>0".into());
    p.variable_groups = vec![vec!["u".into(), "x".into()], vec!["z".into()]];
    p.solver.ordering = "brown".into();
    let report = inspect_peels(&p).unwrap();
    assert_eq!(report.provisional_order, vec!["u", "x", "z"]);
    assert_eq!(report.base.solver.order, vec!["u", "x"]);
    p.solver.order = vec!["x".into(), "u".into(), "z".into()];
    assert_eq!(inspect_peels(&p).unwrap().provisional_order, p.solver.order);
    p.variable_groups.clear();
    p.solver.order = vec!["z".into(), "u".into(), "x".into()];
    let report = inspect_peels(&p).unwrap();
    assert_eq!(report.provisional_order, p.solver.order);
    assert!(report.steps.iter().all(|s| s.variable != "z"));
}

#[test]
fn unprocessed_scale_and_missing_or_multiple_splits_are_explicit_errors() {
    let mut p = problem();
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.homogeneous_anchor = Some("x".into());
    assert!(
        inspect_peels(&p)
            .unwrap_err()
            .to_string()
            .contains("homogeneous reduction")
    );
    p.solver.homogeneous_anchor = None;
    assert!(inspect_peels(&p).is_err());
    p.solver.homogeneous_block.clear();
    p.split.clear();
    assert!(
        inspect_peels(&p)
            .unwrap_err()
            .to_string()
            .contains("one split")
    );
    p.split = vec!["x".into(), "z".into()];
    assert!(inspect_peels(&p).is_err());
}

#[test]
fn inapplicability_and_zero_endpoint_are_diagnostics_without_running_a_base_solver() {
    let mut p = problem();
    p.split = vec!["z^3+x".into()];
    p.limits.wall_time_secs = 1e-12;
    let report = inspect_peels(&p).unwrap();
    assert!(report.steps.is_empty());
    assert_eq!(report.base.variables, p.variables);
    p.split = vec!["z^2+z".into()];
    let report = inspect_peels(&p).unwrap();
    assert_eq!(report.steps.len(), 1);
    assert!(report.base.split.is_empty());
}
