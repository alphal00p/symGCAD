use symgcad::{
    Problem,
    algebra::Algebra,
    comparison::compare_generic,
    output::{RootIndexDomain, Status},
    solver::{selected_order, solve},
    verify::verify,
};

fn problem(variables: &[&str], constraints: &[&str]) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|s| (*s).into()).collect(),
        constraints: constraints.iter().map(|s| (*s).into()).collect(),
        ..Default::default()
    };
    p.solver.engine = "monotone".into();
    p.solver.monotone_negative_target = true;
    p.limits.wall_time_secs = 20.0;
    p
}

fn compare_baseline(p: &Problem, result: &symgcad::output::SolveResult) {
    let mut baseline = p.clone();
    baseline.solver.engine = "baseline".into();
    baseline.solver.monotone_negative_target = false;
    baseline.solver.homogeneous_anchor = None;
    baseline.solver.homogeneous_block.clear();
    let reference = solve(&baseline).unwrap();
    assert!(verify(result).unwrap().verified);
    assert!(verify(&reference).unwrap().verified);
    assert_eq!(
        compare_generic(result, &reference, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn linear_and_quadratic_targets_keep_only_negative_endpoint_fibers() {
    for (target, degree, index) in [("3*z+t<0", 1, 0), ("z^2+z+t<0", 2, 1)] {
        let p = problem(&["t", "z"], &["t>-2", "t<1", "z>0", target]);
        let result = solve(&p).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric);
        assert_eq!(result.order, selected_order(&p).unwrap());
        assert_eq!(result.cells.len(), 1);
        assert!(result.cells[0].signs.is_empty());
        let root = result.cells[0].axes[1].upper.as_ref().unwrap();
        assert_eq!(root.index, index);
        assert_eq!(root.index_domain, RootIndexDomain::Real);
        let evidence = result.monotone.as_ref().unwrap();
        assert_eq!(evidence.steps.len(), 1);
        assert_eq!(evidence.steps[0].degree, degree);
        let target = evidence.target.as_ref().unwrap();
        assert_eq!(target.constraint_index, 3);
        assert_eq!(target.orientation, -1);
        assert_eq!(target.selected_sign, -1);
        assert!(evidence.base.problem.split.is_empty());
        assert!(evidence.base.cells.iter().all(|c| c.signs.is_empty()));
        assert!(evidence.base.problem.limits.wall_time_secs < p.limits.wall_time_secs);
        assert_eq!(result.problem.constraints, p.constraints);
        assert!(result.sign_filter.is_none());
        compare_baseline(&p, &result);
    }
}

#[test]
fn a_two_peel_negative_chain_restores_one_child_at_each_stage() {
    let p = problem(&["y", "z"], &["y>0", "z>0", "y^2+y+z^2+z-1<0"]);
    let result = solve(&p).unwrap();
    let evidence = result.monotone.as_ref().unwrap();
    assert_eq!(evidence.steps.len(), 2);
    assert_eq!(evidence.steps[0].variable, "z");
    assert_eq!(evidence.steps[1].variable, "y");
    assert!(evidence.base.order.is_empty());
    assert_eq!(evidence.base.cells.len(), 1);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].axes.len(), 2);
    compare_baseline(&p, &result);

    let mut previous = p.clone();
    previous.solver.monotone_negative_target = false;
    let previous = solve(&previous).unwrap();
    let arrangement = &previous.sign_filter.as_ref().unwrap().arrangement;
    assert_eq!(arrangement.cells.len(), 3);
    assert_eq!(
        compare_generic(&result, &previous, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn terminal_zero_positive_and_negative_constants_have_exact_coverage() {
    for (target, count) in [("z^2+z<0", 0), ("z^2+z+1<0", 0), ("z^2+z-1<0", 1)] {
        let p = problem(&["z"], &["z>0", target]);
        let result = solve(&p).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric);
        assert_eq!(result.cells.len(), count);
        assert!(result.monotone.as_ref().unwrap().target.is_some());
        let algebra = Algebra::new(&result.order).unwrap();
        assert!(
            result
                .polynomials
                .iter()
                .all(|p| !algebra.parse(p).unwrap().is_zero())
        );
        compare_baseline(&p, &result);
    }
    // Coordinate-affine upper inequalities are also eligible when a separate
    // original positivity premise supplies the complete positive ray.
    let p = problem(&["z"], &["z>0", "3*z-2<0"]);
    let result = solve(&p).unwrap();
    assert_eq!(result.monotone.as_ref().unwrap().steps.len(), 1);
    compare_baseline(&p, &result);
}

#[test]
fn contradictory_remaining_constraints_stay_in_the_exact_base() {
    for contradiction in ["false", "0>0", "-1>0"] {
        let p = problem(&["z"], &["z>0", contradiction, "z^2+z-1<0"]);
        let result = solve(&p).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric);
        assert!(result.cells.is_empty());
        assert!(result.monotone.as_ref().unwrap().target.is_some());
        assert!(verify(&result).unwrap().verified);
    }
}

#[test]
fn no_peel_and_unsupported_axis_domains_keep_the_historical_fallback() {
    for constraints in [
        vec!["z>0", "z^3+z-1<0"],
        vec!["z>0", "z^2-z-1<0"],
        vec!["z>0", "z<2", "z^2+z-1<0"],
        vec!["z>0", "z^2+z-1>0"],
    ] {
        let p = problem(&["z"], &constraints);
        let result = solve(&p).unwrap();
        assert!(result.sign_filter.is_some());
        assert!(result.monotone.is_none());
        assert_eq!(result.problem.constraints, p.constraints);
        compare_baseline(&p, &result);
    }
}

#[test]
fn parameters_fixed_orders_groups_and_other_constraints_are_preserved() {
    let mut p = problem(&["z", "t"], &["z>0", "a*z^2+z+t<0", "t>-1"]);
    p.parameters.insert("a".into(), "3/2".into());
    p.solver.order = vec!["t".into(), "z".into()];
    p.variable_groups = vec![vec!["t".into()], vec!["z".into()]];
    let result = solve(&p).unwrap();
    assert_eq!(result.order, p.solver.order);
    assert_eq!(
        result
            .monotone
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .constraint_index,
        1
    );
    assert_eq!(result.problem.parameters, p.parameters);
    assert!(verify(&result).unwrap().verified);
    let mut bad = result.clone();
    bad.order.reverse();
    assert!(verify(&bad).is_err());

    // A fixed final variable without a positive-axis domain cannot be peeled.
    let mut blocked = problem(&["z", "t"], &["z>0", "z^2+z+t<0"]);
    blocked.solver.order = vec!["z".into(), "t".into()];
    let result = solve(&blocked).unwrap();
    assert_eq!(result.order, blocked.solver.order);
    assert!(result.sign_filter.is_some());
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn local_endpoint_and_original_homogeneous_domain_compose() {
    let mut p = problem(
        &["r", "x", "z"],
        &["r>0", "x>0", "z>0", "z^2+r*z+x^2-r^2<0"],
    );
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.monotone_base_engine = "local".into();
    let result = solve(&p).unwrap();
    assert_eq!(result.problem.constraints, p.constraints);
    assert!(result.split_polynomials.is_empty());
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let evidence = reduced.monotone.as_ref().unwrap();
    assert!(evidence.target.is_some());
    assert!(evidence.base.local.is_some());
    assert_eq!(evidence.steps.len(), 2);
    assert_eq!(result.cells.len(), 1);
    compare_baseline(&p, &result);
}

#[test]
fn target_identity_base_geometry_and_coverage_tampering_are_rejected() {
    let p = problem(&["t", "z"], &["t>-2", "t<1", "z>0", "z^2+z+t<0"]);
    let result = solve(&p).unwrap();
    let mut attacks = Vec::new();
    let mut bad = result.clone();
    bad.monotone
        .as_mut()
        .unwrap()
        .target
        .as_mut()
        .unwrap()
        .constraint_index = 0;
    attacks.push(bad);
    let mut bad = result.clone();
    bad.monotone
        .as_mut()
        .unwrap()
        .target
        .as_mut()
        .unwrap()
        .normalized_target = "z^2+z+t".into();
    attacks.push(bad);
    for field in [0, 1] {
        let mut bad = result.clone();
        let target = bad.monotone.as_mut().unwrap().target.as_mut().unwrap();
        if field == 0 {
            target.orientation = 1;
        } else {
            target.selected_sign = 1;
        }
        attacks.push(bad);
    }
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().target = None;
    attacks.push(bad);
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().steps[0].endpoint = "t-1".into();
    attacks.push(bad);
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().steps[0].leading.sign = -1;
    attacks.push(bad);
    for new_endpoint in ["t>0", "t<-1", "true"] {
        let mut bad = result.clone();
        *bad.monotone
            .as_mut()
            .unwrap()
            .base
            .problem
            .constraints
            .last_mut()
            .unwrap() = new_endpoint.into();
        attacks.push(bad);
    }
    let mut bad = result.clone();
    bad.monotone
        .as_mut()
        .unwrap()
        .base
        .problem
        .limits
        .wall_time_secs = p.limits.wall_time_secs + 1.0;
    attacks.push(bad);
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().base.cells.clear();
    attacks.push(bad);
    let mut bad = result.clone();
    bad.cells.clear();
    attacks.push(bad);
    let mut bad = result.clone();
    bad.cells.push(bad.cells[0].clone());
    attacks.push(bad);
    let mut bad = result.clone();
    bad.cells[0].axes[1].upper.as_mut().unwrap().index = 0;
    attacks.push(bad);
    let mut bad = result.clone();
    bad.cells[0].signs = vec![-1];
    attacks.push(bad);
    let mut bad = result.clone();
    bad.exceptional_polynomials.pop();
    attacks.push(bad);
    for (index, bad) in attacks.iter().enumerate() {
        assert!(verify(bad).is_err(), "accepted tamper {index}");
    }
    let decoded = serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    assert!(verify(&decoded).unwrap().verified);
}

#[test]
fn incomplete_base_and_cumulative_deadlines_never_become_complete() {
    let mut p = problem(&["t", "z"], &["t>-2", "t<1", "z>0", "z^2+z+t<0"]);
    p.solver.monotone_base_engine = "local".into();
    p.solver.local_max_nodes = Some(0);
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(result.monotone.as_ref().unwrap().target.is_some());
    assert!(verify(&result).is_err());
    let mut forged = result.clone();
    forged.status = Status::CompleteGeneric;
    assert!(verify(&forged).is_err());

    p.limits.wall_time_secs = 1e-9;
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(result.statistics.total_seconds >= p.limits.wall_time_secs);
    assert!(verify(&result).is_err());
}

#[test]
fn historical_arrangement_evidence_missing_target_retains_both_signs() {
    let mut p = problem(&["z"], &["z>0"]);
    p.split = vec!["z^2+z-1".into()];
    p.solver.monotone_negative_target = false;
    let result = solve(&p).unwrap();
    assert_eq!(result.cells.len(), 2);
    assert!(result.monotone.as_ref().unwrap().target.is_none());
    let encoded = serde_json::to_value(&result).unwrap();
    assert!(encoded["monotone"].get("target").is_none());
    let decoded = serde_json::from_value(encoded).unwrap();
    assert!(verify(&decoded).unwrap().verified);
}
