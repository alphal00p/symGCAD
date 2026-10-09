use symgcad::{Limits, Problem, comparison::compare_generic, solver::solve};

#[test]
fn different_orders_agree_through_certified_common_refinement() {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "y>0".into(), "x<2".into(), "y<2".into()],
        split: vec!["x^2+y^2-1".into()],
        ..Default::default()
    };
    let a = solve(&p).unwrap();
    p.solver.engine = "local".into();
    p.solver.order = vec!["y".into(), "x".into()];
    let b = solve(&p).unwrap();
    let comparison = compare_generic(&a, &b, &Limits::default()).unwrap();
    assert_eq!(comparison.equivalent, Some(true));
    assert!(comparison.common_regions >= a.cells.len().max(b.cells.len()));
}

#[test]
fn homogeneous_and_monotone_cells_match_the_unreduced_baseline() {
    let mut p = Problem {
        variables: vec!["r".into(), "x".into(), "y".into()],
        constraints: vec!["r>0".into(), "x>0".into(), "y>0".into()],
        split: vec!["x^2+y^2-r^2".into()],
        ..Default::default()
    };
    let a = solve(&p).unwrap();
    p.solver.engine = "monotone".into();
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    let b = solve(&p).unwrap();
    assert_eq!(
        compare_generic(&a, &b, &Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn limited_refinement_is_unknown_and_different_domains_are_rejected() {
    let p = Problem {
        variables: vec!["x".into()],
        constraints: vec!["x^2>0".into()],
        ..Default::default()
    };
    let a = solve(&p).unwrap();
    let limits = Limits {
        max_cells: Some(1),
        ..Default::default()
    };
    assert_eq!(compare_generic(&a, &a, &limits).unwrap().equivalent, None);
    let mut different = p;
    different.constraints = vec!["x>0".into()];
    assert!(compare_generic(&a, &solve(&different).unwrap(), &Limits::default()).is_err());
}
