use symgcad::{
    Problem,
    algebra::Algebra,
    optimized_projection::{OptimizedProjectionOptions, ProjectionCache},
    output::Status,
    problem::normalize,
    solver::solve,
    verify::verify,
};

fn problem() -> Problem {
    Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "x<3".into(), "y>0".into(), "y<3".into()],
        // At x=1 the leading y coefficient vanishes. The repeated factor has
        // an interior zero sheet even though its sign is positive on both sides.
        split: vec!["((x-1)*y+x)^2".into()],
        ..Default::default()
    }
}

#[test]
fn projection_and_fresh_verification_preserve_repeated_interior_seams() {
    for engine in ["baseline", "optimized", "local"] {
        let mut p = problem();
        p.solver.engine = engine.into();
        let native = solve(&p).unwrap();
        assert_eq!(native.status, Status::CompleteGeneric);
        assert!(verify(&native).unwrap().verified);

        p.solver.affine_factorization = true;
        let shifted = solve(&p).unwrap();
        assert_eq!(shifted.status, Status::CompleteGeneric);
        assert_eq!(shifted.polynomials, native.polynomials, "{engine}");
        assert_eq!(shifted.cells, native.cells, "{engine}");
        assert!(shifted.cells.len() > 1);
        assert!(shifted.cells.iter().all(|cell| cell.signs == vec![1]));

        let text = serde_json::to_string(&shifted).unwrap();
        let mut restored: symgcad::output::SolveResult = serde_json::from_str(&text).unwrap();
        assert!(restored.problem.solver.affine_factorization);
        assert!(verify(&restored).unwrap().verified);
        // The setting changes computation, never the mathematical certificate.
        restored.problem.solver.affine_factorization = false;
        assert!(verify(&restored).unwrap().verified);
        restored.cells.pop();
        assert!(verify(&restored).is_err());
    }
}

#[test]
fn policy_is_per_algebra_and_does_not_change_factor_cache_semantics() {
    let mut p = problem();
    let native = normalize(&p, &p.variables).unwrap();
    p.solver.affine_factorization = true;
    let affine = normalize(&p, &p.variables).unwrap();
    let source = affine.algebra.parse("-3/7*((x-1)*y+x)^2").unwrap();
    let expected = native.algebra.factors_checked(&[source.clone()]).unwrap();
    assert_eq!(
        affine.algebra.factors_checked(&[source.clone()]).unwrap(),
        expected
    );
    assert_eq!(
        affine
            .algebra
            .clone()
            .factors_checked(&[source.clone()])
            .unwrap(),
        expected
    );

    let options = OptimizedProjectionOptions::default();
    let mut cache = ProjectionCache::new(&affine.algebra, &options);
    assert_eq!(cache.factor_basis(&[source.clone()]).unwrap(), expected);
    assert_eq!(cache.factor_basis(&[source.clone()]).unwrap(), expected);
    assert!(cache.report.cache.factor_hits > 0);

    let wrong_map = Algebra::new(&["y".into(), "x".into()]).unwrap();
    assert!(wrong_map.factors_checked(&[source]).is_err());
}

#[test]
fn toml_flag_defaults_off_and_roundtrips_without_changing_mathematics() {
    let p = problem();
    let ordinary = toml::to_string(&p).unwrap();
    assert!(!ordinary.contains("affine_factorization"));
    let mut enabled: Problem = toml::from_str(&ordinary).unwrap();
    enabled.solver.affine_factorization = true;
    let encoded = toml::to_string(&enabled).unwrap();
    let restored: Problem = toml::from_str(&encoded).unwrap();
    assert!(restored.solver.affine_factorization);
    let a = normalize(&p, &p.variables).unwrap();
    let b = normalize(&restored, &restored.variables).unwrap();
    assert_eq!(a.positives, b.positives);
    assert_eq!(a.split, b.split);
}
