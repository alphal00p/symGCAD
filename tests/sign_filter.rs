use symgcad::{
    Problem,
    comparison::compare_generic,
    output::{LiftTrace, Status},
    solver::{selected_order, solve},
    verify::verify,
};

fn problem(variables: &[&str], constraints: &[&str]) -> Problem {
    let mut problem = Problem {
        variables: variables.iter().map(|s| (*s).into()).collect(),
        constraints: constraints.iter().map(|s| (*s).into()).collect(),
        ..Problem::default()
    };
    problem.solver.engine = "monotone".into();
    problem
}

#[test]
fn complete_arrangement_is_exhaustively_restricted_to_the_original_domain() {
    let p = problem(&["z"], &["z>0", "z^2+z-1>0"]);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert_eq!(selected_order(&p).unwrap(), result.order);
    let evidence = result.sign_filter.as_ref().unwrap();
    assert_eq!(evidence.constraint_index, 1);
    assert_eq!(evidence.arrangement.cells.len(), 2);
    assert_eq!(evidence.selected_cells, [1]);
    assert_eq!(result.cells.len(), 1);
    assert!(result.cells[0].signs.is_empty());
    assert_eq!(result.cells[0].axes, evidence.arrangement.cells[1].axes);
    assert_eq!(result.polynomials, evidence.arrangement.polynomials);
    assert_eq!(
        result.exceptional_polynomials,
        evidence.arrangement.exceptional_polynomials
    );
    assert!(result.split_polynomials.is_empty());
    assert_eq!(result.problem.constraints, p.constraints);
    assert!(verify(&result).unwrap().verified);
    let decoded = serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    assert!(verify(&decoded).unwrap().verified);
}

#[test]
fn both_strict_orientations_match_the_baseline_by_exact_common_refinement() {
    for relation in [">", "<"] {
        let target = format!("z^2+z+t {relation} 0");
        let p = problem(&["t", "z"], &["t>-2", "t<1", "z>0", &target]);
        let restricted = solve(&p).unwrap();
        let evidence = restricted.sign_filter.as_ref().unwrap();
        let expected_sign = if relation == ">" { 1 } else { -1 };
        assert_eq!(evidence.orientation, expected_sign);
        assert_eq!(evidence.selected_sign, expected_sign);
        assert_eq!(
            evidence.arrangement.monotone.as_ref().unwrap().steps.len(),
            1
        );
        let mut baseline = p.clone();
        baseline.solver.engine = "baseline".into();
        let baseline = solve(&baseline).unwrap();
        assert!(verify(&restricted).unwrap().verified);
        let common = compare_generic(&restricted, &baseline, &p.limits).unwrap();
        assert_eq!(common.equivalent, Some(true));
        assert!(common.common_regions > 0);
    }
}

#[test]
fn homogeneous_and_local_endpoint_certificates_keep_the_original_input() {
    let mut p = problem(
        &["r", "x", "z"],
        &["r>0", "x>0", "z>0", "z^2+r*z+x^2-r^2>0"],
    );
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.monotone_base_engine = "local".into();
    let result = solve(&p).unwrap();
    assert_eq!(result.problem.variables, p.variables);
    let arrangement = &result.sign_filter.as_ref().unwrap().arrangement;
    let reduced = &arrangement.homogeneous.as_ref().unwrap().reduced;
    assert!(reduced.monotone.is_some());
    assert!(reduced.monotone.as_ref().unwrap().base.local.is_some());
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn parameter_substitution_constraint_order_and_fixed_order_are_preserved() {
    let mut p = problem(&["z", "t"], &["z>0", "a*z^2+z+t>0", "t>-1"]);
    p.parameters.insert("a".into(), "3/2".into());
    p.solver.order = vec!["t".into(), "z".into()];
    p.variable_groups = vec![vec!["t".into()], vec!["z".into()]];
    let result = solve(&p).unwrap();
    assert_eq!(result.order, p.solver.order);
    let evidence = result.sign_filter.as_ref().unwrap();
    assert_eq!(evidence.constraint_index, 1);
    assert_eq!(evidence.arrangement.problem.constraints, ["z>0", "t>-1"]);
    assert_eq!(evidence.arrangement.problem.parameters, p.parameters);
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn false_and_zero_constraints_remain_in_the_arrangement_domain() {
    for false_constraint in ["false", "0>0", "-1>0"] {
        let p = problem(&["z"], &["z>0", false_constraint, "z^2+z-1>0"]);
        let result = solve(&p).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric);
        assert!(result.cells.is_empty());
        let evidence = result.sign_filter.as_ref().unwrap();
        assert_eq!(evidence.constraint_index, 2);
        assert!(evidence.arrangement.cells.is_empty());
        assert!(verify(&result).unwrap().verified);
    }
    let p = problem(&["z"], &["z>0", "true", "z^2+z-1>0"]);
    assert!(verify(&solve(&p).unwrap()).unwrap().verified);
}

#[test]
fn unsupported_no_target_input_is_an_explicit_applicability_error() {
    for constraints in [vec!["z>0"], vec!["false"], vec!["0>0", "z<2"]] {
        let p = problem(&["z"], &constraints);
        let before = serde_json::to_string(&p).unwrap();
        let error = solve(&p).unwrap_err().to_string();
        assert!(error.contains("non-coordinate-affine strict constraint"));
        assert!(selected_order(&p).is_err());
        assert_eq!(before, serde_json::to_string(&p).unwrap());
    }
}

#[test]
fn incomplete_arrangement_never_becomes_a_complete_selected_subset() {
    let mut p = problem(&["z"], &["z>0", "z^2+z-1>0"]);
    p.limits.max_cells = Some(1);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert_eq!(
        result.sign_filter.as_ref().unwrap().arrangement.status,
        Status::Incomplete
    );
    assert!(verify(&result).is_err());
    let mut forged = result;
    forged.status = Status::CompleteGeneric;
    assert!(verify(&forged).is_err());
}

#[test]
fn the_library_deadline_includes_sign_filter_planning_and_selection() {
    let mut p = problem(&["z"], &["z>0", "z^2+z-1>0"]);
    p.limits.wall_time_secs = 0.000_000_001;
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(result.diagnostic.as_deref().unwrap().contains("wall time"));
    assert!(result.statistics.total_seconds >= p.limits.wall_time_secs);
    assert!(verify(&result).is_err());
}

#[test]
fn verifier_rejects_selection_problem_geometry_and_seam_tampering() {
    let p = problem(&["z"], &["z>0", "z^2+z-1>0"]);
    let result = solve(&p).unwrap();
    let mut bad = result.clone();
    bad.sign_filter.as_mut().unwrap().selected_cells.clear();
    bad.cells.clear();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.sign_filter.as_mut().unwrap().selected_cells = vec![0];
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.sign_filter.as_mut().unwrap().constraint_index = 0;
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.sign_filter.as_mut().unwrap().normalized_target = "z^2+z+1".into();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.sign_filter
        .as_mut()
        .unwrap()
        .arrangement
        .problem
        .constraints
        .clear();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.cells[0].axes[0].sample = "0".into();
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.cells[0].signs = vec![1];
    assert!(verify(&bad).is_err());
    let mut bad = result.clone();
    bad.exceptional_polynomials.clear();
    assert!(verify(&bad).is_err());
    let mut bad = result;
    bad.trace = Some(LiftTrace::Accepted { cell: 0 });
    assert!(verify(&bad).is_err());
}

#[test]
fn compatibility_merge_is_recomputed_after_sign_selection() {
    let mut p = problem(&["t", "z"], &["t^2<1", "z>0", "z^2+z+t>0"]);
    p.solver.merge = "aggressive".into();
    let result = solve(&p).unwrap();
    assert!(result.merged_cells.is_some());
    assert!(verify(&result).unwrap().verified);
    let mut bad = result;
    bad.merged_cells = Some(vec![]);
    assert!(verify(&bad).is_err());
}

#[test]
fn negative_original_constraint_peels_increasing_split_and_selects_negative_cells() {
    let p = problem(&["z"], &["z>0", "-3/7*(z^2+z-1)>0"]);
    let result = solve(&p).unwrap();
    assert_eq!(selected_order(&p).unwrap(), result.order);
    let evidence = result.sign_filter.as_ref().unwrap();
    assert_eq!(evidence.orientation, -1);
    assert_eq!(evidence.selected_sign, -1);
    assert_eq!(evidence.selected_cells, [0]);
    assert_eq!(
        evidence.arrangement.monotone.as_ref().unwrap().steps.len(),
        1
    );
    assert_eq!(evidence.arrangement.cells[0].signs, [-1]);
    assert_eq!(result.cells[0].axes, evidence.arrangement.cells[0].axes);
    assert!(verify(&result).unwrap().verified);
    let restored = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(verify(&restored).unwrap().verified);
    for which in 0..7 {
        let mut bad = result.clone();
        let evidence = bad.sign_filter.as_mut().unwrap();
        match which {
            0 => evidence.orientation = 0,
            1 => evidence.orientation = 2,
            2 => evidence.orientation = 1,
            3 => evidence.selected_sign = 0,
            4 => evidence.selected_sign = 1,
            5 => {
                evidence.orientation = 1;
                evidence.selected_sign = 1;
            }
            _ => evidence.arrangement.problem.split = vec![evidence.normalized_target.clone()],
        }
        assert!(verify(&bad).is_err(), "tamper{which}");
    }
    let mut incomplete = p;
    incomplete.limits.max_cells = Some(1);
    let incomplete = solve(&incomplete).unwrap();
    assert_eq!(incomplete.status, Status::Incomplete);
    assert!(verify(&incomplete).is_err());
}

#[test]
fn orientation_probes_respect_fixed_orders_and_group_constraints() {
    let mut p = problem(&["z", "w"], &["z>0", "w>0", "z^2+z-1<0"]);
    p.solver.order = vec!["z".into(), "w".into()];
    let fixed = solve(&p).unwrap();
    assert_eq!(fixed.sign_filter.as_ref().unwrap().orientation, 1);
    assert_eq!(fixed.order, p.solver.order);
    assert!(verify(&fixed).unwrap().verified);
    p.solver.order.reverse();
    let legal = solve(&p).unwrap();
    assert_eq!(legal.sign_filter.as_ref().unwrap().orientation, -1);
    assert_eq!(legal.order, p.solver.order);
    assert!(verify(&legal).unwrap().verified);
    p.solver.order.clear();
    p.solver.ordering = "brown".into();
    p.variable_groups = vec![vec!["z".into()], vec!["w".into()]];
    let grouped = solve(&p).unwrap();
    assert_eq!(grouped.sign_filter.as_ref().unwrap().orientation, 1);
    assert_eq!(grouped.order, vec!["z", "w"]);
    assert!(verify(&grouped).unwrap().verified);
}

#[test]
fn negative_selection_composes_with_homogeneous_and_local_endpoint_proofs() {
    let mut p = problem(
        &["r", "x", "z"],
        &["r>0", "x>0", "z>0", "a*z^2+r*z+x^2-r^2<0"],
    );
    p.parameters.insert("a".into(), "3/2".into());
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.monotone_base_engine = "local".into();
    let result = solve(&p).unwrap();
    let evidence = result.sign_filter.as_ref().unwrap();
    assert_eq!(evidence.orientation, -1);
    assert_eq!(evidence.selected_sign, -1);
    assert_eq!(evidence.arrangement.problem.parameters, p.parameters);
    let reduced = &evidence.arrangement.homogeneous.as_ref().unwrap().reduced;
    assert!(!reduced.monotone.as_ref().unwrap().steps.is_empty());
    assert!(reduced.monotone.as_ref().unwrap().base.local.is_some());
    assert!(verify(&result).unwrap().verified);
    assert_eq!(result.problem.constraints, p.constraints);
}
