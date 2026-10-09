use symgcad::{Problem, algebra::Algebra, output::Status, solver::solve, verify::verify};

fn problem(variables: &[&str], constraints: &[&str], split: &str) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|v| (*v).into()).collect(),
        constraints: constraints.iter().map(|v| (*v).into()).collect(),
        split: vec![split.into()],
        ..Problem::default()
    };
    p.solver.engine = "monotone".into();
    p
}

#[test]
fn a_quadratic_uses_exact_coefficient_proofs_and_the_full_real_root_index() {
    let result = solve(&problem(&["z"], &["z>0"], "z^2+z-1")).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert_eq!(result.cells.len(), 2);
    assert_eq!(result.cells[0].signs, vec![-1]);
    assert_eq!(result.cells[1].signs, vec![1]);
    assert_eq!(result.cells[0].axes[0].upper.as_ref().unwrap().index, 1);
    assert_eq!(result.monotone.as_ref().unwrap().steps.len(), 1);
    assert!(result.monotone.as_ref().unwrap().base.order.is_empty());
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn a_linear_axis_uses_root_index_zero_and_covers_both_signs() {
    let result = solve(&problem(&["z"], &["z>0"], "3*z-2")).unwrap();
    assert_eq!(result.cells.len(), 2);
    assert_eq!(result.cells[0].axes[0].upper.as_ref().unwrap().index, 0);
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn zero_endpoint_is_positive_without_excluding_the_whole_domain() {
    for expression in ["z", "z^2", "z^2+z"] {
        let result = solve(&problem(&["z"], &["z>0"], expression)).unwrap();
        assert_eq!(result.cells.len(), 1);
        assert_eq!(result.cells[0].signs, vec![1]);
        assert!(
            result
                .monotone
                .as_ref()
                .unwrap()
                .base
                .split_polynomials
                .is_empty()
        );
        let a = Algebra::new(&result.order).unwrap();
        assert!(
            result
                .polynomials
                .iter()
                .all(|p| !a.parse(p).unwrap().is_zero())
        );
        assert!(verify(&result).unwrap().verified);
    }
}

#[test]
fn negative_root_collisions_do_not_split_a_positive_axis() {
    // On c>0 the discriminant 1-4*c can change sign, but q>0 on z>0.
    let result = solve(&problem(&["c", "z"], &["c>0", "z>0"], "z^2+z+c")).unwrap();
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![1]);
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn arbitrary_base_constraints_and_changing_endpoint_sign_are_preserved() {
    let p = problem(&["t", "z"], &["t^2<4", "z>0"], "z^2+z+t");
    let result = solve(&p).unwrap();
    assert_eq!(result.cells.len(), 3);
    assert_eq!(
        result.cells.iter().filter(|c| c.signs == vec![-1]).count(),
        1
    );
    let a = Algebra::new(&result.order).unwrap();
    assert!(
        result
            .polynomials
            .iter()
            .any(|p| a.parse(p).unwrap() == a.parse("t").unwrap())
    );
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn inadmissible_axis_constraints_or_coefficients_use_the_exact_base_fallback() {
    for p in [
        problem(&["z"], &["z>0", "z<2"], "z^2+z-1"),
        problem(&["z"], &["z>0"], "z^2-z-1"),
        problem(&["t", "z"], &["z>0"], "t*z^2+z-1"),
        problem(&["z"], &["z>0"], "z^3+z-1"),
    ] {
        let result = solve(&p).unwrap();
        assert!(result.monotone.as_ref().unwrap().steps.is_empty());
        assert!(verify(&result).unwrap().verified);
    }
}

#[test]
fn a_strict_quadratic_leading_coefficient_may_use_an_original_shifted_bound() {
    let p = problem(&["t", "z"], &["t>4", "z>0"], "(t-4)*z^2+z-1");
    let result = solve(&p).unwrap();
    assert_eq!(result.monotone.as_ref().unwrap().steps.len(), 1);
    assert_eq!(result.cells.len(), 2);
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn monotone_chain_adds_one_cell_per_negative_endpoint_branch() {
    let p = problem(
        &["x", "y", "a", "b", "c", "d"],
        &["x>0", "y>0", "a>0", "b>0", "c>0", "d>0"],
        "x*(x+1)+y*(x^2-8*x+1)+a^2+a+b^2+b+c^2+c+d^2+d",
    );
    let result = solve(&p).unwrap();
    assert_eq!(result.monotone.as_ref().unwrap().steps.len(), 4);
    assert_eq!(result.cells.len(), 8);
    assert_eq!(
        result.cells.iter().filter(|c| c.signs == vec![-1]).count(),
        1
    );
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn fixed_orders_are_preserved_and_dynamic_orders_respect_groups() {
    let mut p = problem(&["z", "t"], &["z>0"], "z^2+z+t");
    let fixed = solve(&p).unwrap();
    assert_eq!(fixed.order, vec!["z", "t"]);
    assert!(fixed.monotone.as_ref().unwrap().steps.is_empty());
    p.solver.ordering = "brown".into();
    p.variable_groups = vec![vec!["t".into()], vec!["z".into()]];
    let reordered = solve(&p).unwrap();
    assert_eq!(reordered.order, vec!["t", "z"]);
    assert_eq!(reordered.monotone.as_ref().unwrap().steps.len(), 1);
    assert!(verify(&reordered).unwrap().verified);
}

#[test]
fn homogeneous_and_monotone_certificates_compose() {
    let mut p = problem(&["r", "z"], &["r>0", "z>0"], "z^2+r*z-r^2");
    p.solver.homogeneous_block = vec!["r".into(), "z".into()];
    p.solver.homogeneous_anchor = Some("r".into());
    let result = solve(&p).unwrap();
    assert!(
        result
            .homogeneous
            .as_ref()
            .unwrap()
            .reduced
            .monotone
            .is_some()
    );
    assert_eq!(result.cells.len(), 2);
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn incomplete_base_or_reconstruction_never_becomes_complete() {
    let mut p = problem(&["z"], &["z>0"], "z^2+z-1");
    p.limits.max_cells = Some(1);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].axes.len(), 1);
    assert!(result.diagnostic.as_deref().unwrap().contains("cell limit"));
    assert!(verify(&result).is_err());
}

#[test]
fn a_tiny_library_deadline_is_not_reported_as_complete() {
    let mut p = problem(&["z"], &["z>0"], "z^2+z-1");
    p.limits.wall_time_secs = 0.000_000_001;
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(verify(&result).is_err());
}

#[test]
fn verifier_rejects_missing_children_or_tampered_exact_proofs() {
    let result = solve(&problem(&["t", "z"], &["z>0"], "z^2+z+t")).unwrap();
    let mut bad = result.clone();
    bad.cells.pop();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().steps[0].endpoint = "t+1".into();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().steps[0].leading.sign = -1;
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().steps[0].linear = None;
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.exceptional_polynomials.pop();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().base.cells.pop();
    assert!(verify(&bad).is_err());
    let text = serde_json::to_string(&result).unwrap();
    let decoded = serde_json::from_str(&text).unwrap();
    assert!(verify(&decoded).unwrap().verified);
}

#[test]
fn local_endpoint_proofs_compose_with_monotone_and_positive_scale_reconstruction() {
    let mut p = problem(
        &["r", "x", "y", "z"],
        &["r>0", "x>0", "y>0", "z>0"],
        "z^2+r*z+x^2-3*x*y+y^2-r^2",
    );
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.homogeneous_anchor = Some("r".into());
    let reference = solve(&p).unwrap();
    p.solver.monotone_base_engine = "local".into();
    let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    let result = solve(&restored).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let monotone = reduced.monotone.as_ref().unwrap();
    assert_eq!(monotone.steps.len(), 1);
    assert_eq!(monotone.steps[0].variable, "z");
    assert!(monotone.base.local.is_some());
    assert!(verify(&result).unwrap().verified);
    let compared = symgcad::comparison::compare_generic(&result, &reference, &p.limits).unwrap();
    assert_eq!(compared.equivalent, Some(true));
    let mut bad = result.clone();
    bad.homogeneous
        .as_mut()
        .unwrap()
        .reduced
        .monotone
        .as_mut()
        .unwrap()
        .base
        .cells
        .pop();
    assert!(verify(&bad).is_err());
    p.solver.local_max_refinements = Some(0);
    p.solver.local_max_nodes = Some(1);
    let incomplete = solve(&p).unwrap();
    assert_eq!(incomplete.status, Status::Incomplete);
    assert!(verify(&incomplete).is_err());
}

#[test]
fn monotone_base_engine_is_explicit_and_cannot_recurse() {
    let mut p = problem(&["z"], &["z>0"], "z^2+z-1");
    assert!(
        serde_json::to_value(&p).unwrap()["solver"]
            .get("monotone_base_engine")
            .is_none()
    );
    for engine in ["monotone", "baseline", "unknown"] {
        p.solver.monotone_base_engine = engine.into();
        assert!(p.validate().is_err());
    }
}
