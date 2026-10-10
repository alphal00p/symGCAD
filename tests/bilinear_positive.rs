use symgcad::{
    Problem,
    comparison::compare_generic,
    output::{LiftTrace, RootIndexDomain, Status},
    solver::{selected_order, solve},
    verify::verify,
};

fn problem(f: &str) -> Problem {
    let mut p = Problem {
        variables: vec!["y".into(), "x".into()],
        constraints: vec!["y>0".into(), "x>0".into(), format!("({f})>0")],
        ..Default::default()
    };
    p.solver.engine = "bilinear_positive".into();
    p.solver.ordering = "given".into();
    p.solver.bilinear_positive_guide = Some("(x+1)*(y+1)".into());
    p.limits.wall_time_secs = 20.;
    p
}
fn checked(p: &Problem) -> symgcad::output::SolveResult {
    let r = solve(p).unwrap();
    assert_eq!(r.status, Status::CompleteGeneric, "{:?}", r.diagnostic);
    assert!(verify(&r).unwrap().verified);
    assert_eq!(selected_order(p).unwrap(), r.order);
    r
}
fn parameter_problem() -> Problem {
    let mut p = problem("(t-1)*x*y+t*x+(t+1)*y+t+2");
    p.variables = vec!["x".into(), "t".into(), "y".into()];
    p.constraints.insert(0, "t>0".into());
    p.variable_groups = vec![vec!["t".into()], vec!["x".into(), "y".into()]];
    p.solver.order = vec!["t".into(), "y".into(), "x".into()];
    p
}

#[test]
fn all_five_sign_patterns_have_exhaustive_nonempty_standard_fibers() {
    for (f, count) in [
        ("x*y+2*x+3*y+4", 1),
        ("-2*x*y-x+y+2", 1),
        ("-3*x*y-2*x-y+1", 1),
        ("-x*y+x+y+2", 2),
        ("-2*x*y+x-y+2", 2),
    ] {
        let p = problem(f);
        let r = checked(&p);
        assert_eq!(r.cells.len(), count);
        for cell in &r.cells {
            for axis in &cell.axes {
                for bound in [&axis.lower, &axis.upper].into_iter().flatten() {
                    assert_eq!(bound.index_domain, RootIndexDomain::Real);
                    assert_eq!(bound.index, 0);
                }
            }
        }
        let mut baseline = p.clone();
        baseline.solver.engine = "baseline".into();
        baseline.solver.bilinear_positive_guide = None;
        let reference = solve(&baseline).unwrap();
        assert!(verify(&reference).unwrap().verified);
        assert_eq!(
            compare_generic(&r, &reference, &p.limits)
                .unwrap()
                .equivalent,
            Some(true)
        );
    }
}

#[test]
fn original_groups_and_parameter_base_are_replayed_for_all_base_engines() {
    for engine in ["baseline", "optimized", "local"] {
        let mut p = parameter_problem();
        p.solver.bilinear_positive_base_engine = engine.into();
        let r = checked(&p);
        assert_eq!(r.cells.len(), 3);
        assert_eq!(r.problem.constraints, p.constraints);
        let proof = r.bilinear_positive.as_ref().unwrap();
        assert_eq!(proof.base.problem.constraints, vec!["((1)*t)>0"]);
        assert_eq!(proof.base.problem.solver.engine, engine);
        assert_eq!(proof.base.problem.variable_groups, vec![vec!["t"]]);
        assert!(
            proof
                .positivity
                .iter()
                .all(|c| c.sign == 1 && c.domain.excluded_sign_constraints == vec![3])
        );
        let mut baseline = p;
        baseline.solver.engine = "baseline".into();
        baseline.solver.bilinear_positive_guide = None;
        let reference = solve(&baseline).unwrap();
        assert_eq!(
            compare_generic(&r, &reference, &baseline.limits)
                .unwrap()
                .equivalent,
            Some(true)
        );
    }
}

#[test]
fn proof_replay_rejects_domain_identity_sign_inventory_and_bound_tampering() {
    let r = checked(&parameter_problem());
    let reject = |p: &symgcad::output::SolveResult| assert!(verify(p).is_err());
    let mut p = r.clone();
    p.cells.pop();
    reject(&p);
    let mut p = r.clone();
    p.cells.push(p.cells[0].clone());
    reject(&p);
    let mut p = r.clone();
    p.cells[0].axes[2].sample = "0".into();
    reject(&p);
    let mut p = r.clone();
    p.cells[0].axes[1].lower.as_mut().unwrap().index = 1;
    reject(&p);
    let mut p = r.clone();
    p.exceptional_polynomials.pop();
    reject(&p);
    let mut p = r.clone();
    p.polynomials[0] = "1".into();
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive.as_mut().unwrap().positivity[3].sign = -1;
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive.as_mut().unwrap().positivity[3]
        .domain
        .excluded_sign_constraints
        .clear();
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive.as_mut().unwrap().guide = "x+y+1".into();
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive
        .as_mut()
        .unwrap()
        .base
        .problem
        .constraints
        .clear();
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive
        .as_mut()
        .unwrap()
        .base
        .problem
        .split
        .pop();
    reject(&p);
    let mut p = r.clone();
    p.trace = Some(LiftTrace::Accepted { cell: 0 });
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive.as_mut().unwrap().base.status = Status::Incomplete;
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive.as_mut().unwrap().target_constraint = 0;
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive.as_mut().unwrap().positivity[6].polynomial = "2".into();
    reject(&p);
    let mut p = r.clone();
    p.bilinear_positive
        .as_mut()
        .unwrap()
        .base
        .problem
        .limits
        .wall_time_secs = f64::INFINITY;
    reject(&p);
}

#[test]
fn zero_width_without_strict_guide_is_rejected_and_zero_coefficients_are_not_seams() {
    // Rayleigh determinant zero is insufficient: both proposed y bounds equal1.
    assert!(solve(&problem("-(x+1)*(y-1)")).is_err());
    assert!(solve(&problem("x+y+1")).is_err());
    assert!(solve(&problem("x*y+y+1")).is_err());
    assert!(solve(&problem("x*y+x+1")).is_err());
}

#[test]
fn extra_constraints_missing_positive_premises_wrong_order_and_wrong_guide_decline() {
    let original = parameter_problem();
    let mut p = original.clone();
    p.constraints.push("t<2".into());
    assert!(solve(&p).is_err());
    let mut p = original.clone();
    p.constraints.push("y<2".into());
    assert!(solve(&p).is_err());
    let mut p = original.clone();
    p.constraints.remove(0);
    assert!(solve(&p).is_err());
    let mut p = original.clone();
    p.solver.order = vec!["x".into(), "t".into(), "y".into()];
    assert!(solve(&p).is_err());
    let mut p = original.clone();
    p.solver.order.clear();
    p.solver.ordering = "brown".into();
    assert!(solve(&p).is_err());
    let mut p = original.clone();
    p.solver.bilinear_positive_guide = Some("x+y".into());
    assert!(solve(&p).is_err());
    let mut p = original.clone();
    p.split.push("t-1".into());
    assert!(solve(&p).is_err());
    let mut p = original;
    p.solver.homogeneous_anchor = Some("t".into());
    p.solver.homogeneous_block = p.variables.clone();
    assert!(solve(&p).is_err());
}

#[test]
fn reconstruction_cell_and_wall_budgets_never_claim_complete() {
    let mut late = problem("-x*y+x+y+2");
    late.limits.max_cells = Some(1);
    let r = solve(&late).unwrap();
    assert_eq!(
        r.bilinear_positive.as_ref().unwrap().base.status,
        Status::CompleteGeneric
    );
    assert_eq!(r.status, Status::Incomplete);
    assert_eq!(r.cells.len(), 1);
    assert!(verify(&r).is_err());
    let mut p = parameter_problem();
    p.limits.max_cells = Some(2);
    let r = solve(&p).unwrap();
    assert_eq!(r.status, Status::Incomplete);
    assert!(r.cells.len() <= 2);
    assert!(verify(&r).is_err());
    let mut p = parameter_problem();
    p.limits.wall_time_secs = 1e-12;
    let r = solve(&p).unwrap();
    assert_eq!(r.status, Status::Incomplete);
    assert!(verify(&r).is_err());
}

#[test]
fn baseline_base_clears_optimized_flags_without_weakening_wrapper_certificates() {
    let mut p = parameter_problem();
    p.solver.bilinear_positive_base_engine = "baseline".into();
    p.solver.positivity = true;
    p.solver.input_positivity = true;
    p.solver.prune_operations = true;
    let r = checked(&p);
    let proof = r.bilinear_positive.as_ref().unwrap();
    assert!(
        !proof.base.problem.solver.positivity
            && !proof.base.problem.solver.input_positivity
            && !proof.base.problem.solver.prune_operations
    );
    assert!(proof.positivity.iter().all(|c| c.sign == 1));
}

#[test]
fn incomplete_local_base_never_receives_original_completion_credit() {
    let mut p = parameter_problem();
    p.solver.bilinear_positive_base_engine = "local".into();
    p.solver.local_max_nodes = Some(1);
    let r = solve(&p).unwrap();
    assert_eq!(r.status, Status::Incomplete);
    assert_eq!(
        r.bilinear_positive.as_ref().unwrap().base.status,
        Status::Incomplete
    );
    assert!(r.cells.is_empty());
    assert_eq!(r.statistics.raw_cells, 0);
    assert!(verify(&r).is_err());
}

#[test]
fn compatibility_merge_retains_raw_cells_and_exact_exceptional_seams() {
    let mut p = parameter_problem();
    p.solver.merge = "aggressive".into();
    let r = checked(&p);
    assert_eq!(r.cells.len(), 3);
    assert!(r.merged_cells.is_some());
    let mut changed = r.clone();
    changed.merged_cells.as_mut().unwrap().clear();
    assert!(verify(&changed).is_err());
    let text = serde_json::to_vec(&r).unwrap();
    let restored: symgcad::output::SolveResult = serde_json::from_slice(&text).unwrap();
    assert!(verify(&restored).unwrap().verified);
}
