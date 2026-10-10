use symgcad::{
    Problem,
    comparison::compare_generic,
    output::{RootIndexDomain, Status},
    solver::{selected_order, solve},
    verify::verify,
};

fn problem(q: &str, b: &str) -> Problem {
    let mut p = Problem {
        variables: vec!["v".into(), "y".into(), "x".into(), "z".into()],
        constraints: vec!["v>4".into(), "y>0".into(), "x>0".into(), "z>0".into()],
        split: vec![format!("({q})*z+({b})")],
        ..Default::default()
    };
    p.solver.engine = "quadratic_ladder".into();
    p.solver.ordering = "given".into();
    p.solver.positivity = true;
    p.limits.wall_time_secs = 20.;
    p
}
fn checked(p: &Problem) -> symgcad::output::SolveResult {
    let result = solve(p).unwrap();
    assert_eq!(
        result.status,
        Status::CompleteGeneric,
        "{:?}",
        result.diagnostic
    );
    assert!(verify(&result).unwrap().verified);
    assert_eq!(selected_order(p).unwrap(), result.order);
    let encoded = serde_json::to_string(&result).unwrap();
    let replay = serde_json::from_str(&encoded).unwrap();
    assert!(verify(&replay).unwrap().verified);
    result
}

#[test]
fn generic_families_open_two_positive_roots_and_keep_the_f_sheet() {
    for (q, b) in [
        ("x^2+(1-(v-2)*y)*x+y^2+y+1", "1"),
        ("x^2+(4-(v-2)*y)*x+y^2+y+1", "x^2+y+1"),
        ("(1+y)*x^2+(5-(v-2)*y)*x+(2+y)", "1+x+y^2"),
        ("(3/2)*x^2+(3/2)*(1-(v-2)*y)*x+(3/2)*(y^2+y+1)", "7/3+x"),
    ] {
        let result = checked(&problem(q, b));
        assert_eq!(result.cells.len(), 5);
        assert_eq!(
            result.cells.iter().map(|c| c.signs[0]).collect::<Vec<_>>(),
            vec![1, 1, 1, -1, 1]
        );
        assert_eq!(result.cells[2].axes[3].upper, result.cells[3].axes[3].lower);
        let cut = result.cells[2].axes[3].upper.as_ref().unwrap();
        assert_eq!(cut.index, 0);
        assert_eq!(cut.index_domain, RootIndexDomain::Real);
        assert!(result.exceptional_polynomials.contains(&cut.polynomial));
        assert_eq!(result.cells[0].axes[1].upper.as_ref().unwrap().index, 1);
        assert_eq!(result.cells[1].axes[2].upper.as_ref().unwrap().index, 0);
        assert_eq!(result.cells[4].axes[2].lower.as_ref().unwrap().index, 1);
    }
}

#[test]
fn exact_common_refinement_matches_ordinary_cad() {
    let p = problem("x^2+(1-4*y)*x+y^2+y+1", "1");
    let ladder = checked(&p);
    let mut baseline = p.clone();
    baseline.solver.engine = "baseline".into();
    baseline.solver.positivity = false;
    let ordinary = checked(&baseline);
    assert_eq!(
        compare_generic(&ladder, &ordinary, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn disconnected_base_and_original_groups_are_preserved() {
    let mut p = problem("x^2+(4-(v-2)*y)*x+y^2+y+1", "1");
    p.constraints.push("(v-5)*(v-6)>0".into());
    p.variable_groups = vec![vec!["v".into()], vec!["y".into(), "x".into(), "z".into()]];
    p.solver.order = p.variables.clone();
    let result = checked(&p);
    assert_eq!(result.cells.len(), 10);
    assert_eq!(
        result.quadratic_ladder.as_ref().unwrap().base.cells.len(),
        2
    );
}

#[test]
fn empty_original_base_is_complete_with_no_cells() {
    let mut p = problem("x^2+(1-(v-2)*y)*x+y^2+y+1", "1");
    p.constraints.push("0>0".into());
    assert!(checked(&p).cells.is_empty());
}

#[test]
fn the_unused_discriminant_root_may_cross_zero_inside_a_base_cell() {
    // D(0)=(v-3)^2-4 changes sign at v=5 in the original v>4 domain.
    // Its smaller root crosses the coordinate boundary, while the larger
    // simple root and the five positive-fiber patterns remain valid.
    let p = problem("x^2+((v-3)-(v-2)*y)*x+y^2+y+1", "1");
    let result = checked(&p);
    assert_eq!(result.cells.len(), 5);
    assert_eq!(
        result.quadratic_ladder.as_ref().unwrap().base.cells.len(),
        1
    );
    assert_eq!(result.cells[0].axes[0].sample, "5");
    assert_eq!(result.cells[0].axes[1].upper.as_ref().unwrap().index, 1);
    assert_eq!(result.cells[2].axes[3].upper, result.cells[3].axes[3].lower);
}

#[test]
fn unsupported_or_nonstrict_premises_never_gain_complete_status() {
    for q in [
        "-x^2+(1-(v-2)*y)*x+y^2+y+1",
        "x^2+(1-(v-2)*y)*x",
        "x^2-(v-2)*y*x+y^2+1",
        "x^2+(1+y)*x+y^2+1",
        "x^2+(1-2*y)*x+y^2+1",
    ] {
        assert!(solve(&problem(q, "1")).is_err(), "{q}");
    }
    let p = problem("x^2+(1-(v-2)*y)*x+y^2+y+1", "1");
    for extra in ["y<10", "x>y", "z<1"] {
        let mut bad = p.clone();
        bad.constraints.push(extra.into());
        assert!(solve(&bad).is_err());
    }
    let mut missing = p.clone();
    missing.constraints.pop();
    assert!(solve(&missing).is_err());
    let mut negative = p.clone();
    negative.split[0] = format!("-({})", negative.split[0]);
    assert!(solve(&negative).is_err());
    let mut multiple = p.clone();
    multiple.split.push("x-1".into());
    assert!(solve(&multiple).is_err());
    let mut wrong_order = p;
    wrong_order.variable_groups = vec![vec!["v".into()], vec!["y".into(), "x".into(), "z".into()]];
    wrong_order.solver.order = vec!["y".into(), "v".into(), "x".into(), "z".into()];
    assert!(solve(&wrong_order).is_err());
}

#[test]
fn exhaustive_replay_rejects_forged_branches_roots_and_exceptions() {
    let original = checked(&problem("x^2+(4-(v-2)*y)*x+y^2+y+1", "1"));
    let mut bad = original.clone();
    bad.cells.pop();
    assert!(verify(&bad).is_err());
    let mut bad = original.clone();
    bad.cells[1] = bad.cells[0].clone();
    assert!(verify(&bad).is_err());
    let mut bad = original.clone();
    bad.cells[0].axes[1].upper.as_mut().unwrap().index = 0;
    assert!(verify(&bad).is_err());
    let mut bad = original.clone();
    bad.cells[3].signs[0] = 1;
    assert!(verify(&bad).is_err());
    let mut bad = original.clone();
    bad.exceptional_polynomials.clear();
    assert!(verify(&bad).is_err());
    let mut bad = original.clone();
    bad.quadratic_ladder
        .as_mut()
        .unwrap()
        .base
        .problem
        .constraints = vec!["v>3".into()];
    assert!(verify(&bad).is_err());
    let mut bad = original.clone();
    bad.quadratic_ladder.as_mut().unwrap().positivity[0].sign = -1;
    assert!(verify(&bad).is_err());
    let mut bad = original;
    bad.quadratic_ladder.as_mut().unwrap().schema_version = 99;
    assert!(verify(&bad).is_err());
}

#[test]
fn cell_and_wall_limits_return_incomplete() {
    let mut p = problem("x^2+(1-(v-2)*y)*x+y^2+y+1", "1");
    p.limits.max_cells = Some(4);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(verify(&result).is_err());
    p.limits.max_cells = None;
    p.limits.wall_time_secs = 1e-12;
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(verify(&result).is_err());
}
