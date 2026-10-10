use symgcad::{
    Problem,
    comparison::compare_generic,
    output::{RootIndexDomain, Status},
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
    p.limits.wall_time_secs = 15.0;
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
fn changing_linear_sign_has_the_same_exact_endpoint_only_coverage() {
    let p = problem();
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert_eq!(result.cells.len(), 3);
    let step = &result.monotone.as_ref().unwrap().steps[0];
    assert_eq!(step.variable, "z");
    assert!(step.linear.is_none());
    assert!(step.conditional_linear.is_some());
    let negative = result.cells.iter().find(|c| c.signs == [-1]).unwrap();
    let bound = negative.axes[1].upper.as_ref().unwrap();
    assert_eq!(bound.index, 1);
    assert_eq!(bound.index_domain, RootIndexDomain::Real);
    compare_baseline(&p, &result);
}

#[test]
fn negative_target_uses_the_original_domain_and_only_lower_fibers() {
    let mut p = problem();
    let polynomial = p.split.pop().unwrap();
    p.constraints.push(format!("({polynomial})<0"));
    p.solver.monotone_negative_target = true;
    let result = solve(&p).unwrap();
    let evidence = result.monotone.as_ref().unwrap();
    assert!(evidence.target.is_some());
    assert!(evidence.steps[0].conditional_linear.is_some());
    assert_eq!(result.cells.len(), 1);
    assert!(result.cells[0].signs.is_empty());
    compare_baseline(&p, &result);
}

#[test]
fn conditional_proof_replays_identities_support_signs_and_exclusivity() {
    let result = solve(&problem()).unwrap();
    let mutate = |case: usize| {
        let mut bad = result.clone();
        let step = &mut bad.monotone.as_mut().unwrap().steps[0];
        match case {
            0 => step.conditional_linear = None,
            1 => step.conditional_linear.as_mut().unwrap().multiplier = "z+x".into(),
            2 => step.conditional_linear.as_mut().unwrap().multiplier = "x+2".into(),
            3 => {
                step.conditional_linear
                    .as_mut()
                    .unwrap()
                    .positive_multiplier
                    .sign = -1
            }
            4 => {
                step.conditional_linear
                    .as_mut()
                    .unwrap()
                    .positive_difference
                    .sign = -1
            }
            5 => {
                step.conditional_linear
                    .as_mut()
                    .unwrap()
                    .positive_difference
                    .polynomial = "x+1".into()
            }
            6 => {
                step.linear = Some(
                    step.conditional_linear
                        .as_ref()
                        .unwrap()
                        .positive_multiplier
                        .clone(),
                )
            }
            7 => step.endpoint = "x^2-2*x-1".into(),
            _ => unreachable!(),
        }
        bad
    };
    for case in 0..8 {
        assert!(verify(&mutate(case)).is_err(), "tamper case {case}");
    }
    let text = serde_json::to_string(&result).unwrap();
    assert!(
        verify(&serde_json::from_str(&text).unwrap())
            .unwrap()
            .verified
    );
}

#[test]
fn historical_absence_retains_the_direct_coefficient_rule() {
    let mut p = problem();
    p.split = vec!["z^2+z+x-1".into()];
    let result = solve(&p).unwrap();
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("conditional_linear"));
    assert!(
        verify(&serde_json::from_str(&serialized).unwrap())
            .unwrap()
            .verified
    );
    let mut bad = result.clone();
    bad.monotone.as_mut().unwrap().steps[0].linear = None;
    assert!(verify(&bad).is_err());

    p.split = vec!["z^2-z+x".into()];
    let two_roots = solve(&p).unwrap();
    assert!(two_roots.monotone.as_ref().unwrap().steps.is_empty());
    assert!(verify(&two_roots).unwrap().verified);
}

#[test]
fn homogeneous_scale_and_limits_preserve_conditional_evidence() {
    let mut p = problem();
    p.variables.insert(0, "r".into());
    p.constraints.insert(0, "r>0".into());
    p.split = vec!["z^2+(x-r)*z+x^2-2*x*r".into()];
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.homogeneous_anchor = Some("r".into());
    let result = solve(&p).unwrap();
    assert!(
        result
            .homogeneous
            .as_ref()
            .unwrap()
            .reduced
            .monotone
            .as_ref()
            .unwrap()
            .steps[0]
            .conditional_linear
            .is_some()
    );
    compare_baseline(&p, &result);
    p.limits.max_cells = Some(1);
    let partial = solve(&p).unwrap();
    assert_eq!(partial.status, Status::Incomplete);
    assert!(verify(&partial).is_err());
}

#[test]
fn fixed_order_and_coordinate_domain_restrictions_remain_mandatory() {
    let mut p = problem();
    p.solver.order = vec!["z".into(), "x".into()];
    let result = solve(&p).unwrap();
    assert_eq!(result.order, p.solver.order);
    assert!(verify(&result).unwrap().verified);
    p.solver.order.clear();
    p.constraints.push("z<3".into());
    let result = solve(&p).unwrap();
    assert!(result.monotone.as_ref().unwrap().steps.is_empty());
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn strictly_negative_constant_coefficients_and_zero_endpoint_keep_exact_rules() {
    let mut p = Problem {
        variables: vec!["z".into()],
        constraints: vec!["z>0".into()],
        split: vec!["z^2-z-2".into()],
        ..Problem::default()
    };
    p.solver.engine = "monotone".into();
    let result = solve(&p).unwrap();
    let evidence = result.monotone.as_ref().unwrap();
    assert_eq!(evidence.steps.len(), 1);
    assert!(evidence.steps[0].conditional_linear.is_some());
    assert_eq!(result.cells.len(), 2);
    assert_eq!(result.cells[0].axes[0].upper.as_ref().unwrap().index, 1);
    compare_baseline(&p, &result);

    p.split = vec!["z^2-z".into()];
    let result = solve(&p).unwrap();
    assert!(result.monotone.as_ref().unwrap().steps.is_empty());
    assert!(verify(&result).unwrap().verified);
    p.split = vec!["z^2+z".into()];
    let result = solve(&p).unwrap();
    assert!(
        result.monotone.as_ref().unwrap().steps[0]
            .conditional_linear
            .is_none()
    );
    assert_eq!(result.cells.len(), 1);
    assert!(verify(&result).unwrap().verified);
}
