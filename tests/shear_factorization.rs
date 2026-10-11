use symgcad::{Problem, algebra::Algebra, output::Status, solver::solve, verify::verify};

#[test]
fn algebra_opt_in_handles_generic_support_cancellation_with_original_maps() {
    let a = Algebra::new(&["x".into(), "unused".into(), "y".into()]).unwrap();
    let p = a.parse("-3/7*((x+y)^2-4*x^2*y^2)^2").unwrap();
    let ordinary = a.factors_checked(&[p.clone()]).unwrap();
    let enabled = a.clone().with_affine_factorization(true);
    assert_eq!(enabled.factors_checked(&[p.clone()]).unwrap(), ordinary);
    assert_eq!(
        enabled.clone().factors_checked(&[p.clone()]).unwrap(),
        ordinary
    );
    assert!(
        ordinary
            .iter()
            .all(|f| f.variables() == &a.variables && f.degree(1) == 0)
    );
    let wrong = Algebra::new(&["y".into(), "unused".into(), "x".into()])
        .unwrap()
        .with_affine_factorization(true);
    assert!(wrong.factors_checked(&[p]).is_err());
}

#[test]
fn repeated_same_sign_interior_seams_survive_shear_and_fresh_verification() {
    for engine in ["baseline", "optimized", "local"] {
        let mut p = Problem {
            variables: vec!["x".into(), "y".into()],
            constraints: vec!["x>0".into(), "x<2".into(), "y>0".into(), "y<2".into()],
            split: vec!["((x+y)^2-4*x^2*y^2)^2".into()],
            ..Default::default()
        };
        p.solver.engine = engine.into();
        let native = solve(&p).unwrap();
        assert_eq!(native.status, Status::CompleteGeneric);
        p.solver.affine_factorization = true;
        let sheared = solve(&p).unwrap();
        assert_eq!(sheared.status, Status::CompleteGeneric);
        assert_eq!(sheared.polynomials, native.polynomials, "{engine}");
        assert_eq!(sheared.cells, native.cells, "{engine}");
        assert!(sheared.cells.len() > 1);
        assert!(sheared.cells.iter().all(|c| c.signs == vec![1]));
        assert!(verify(&sheared).unwrap().verified);
        let encoded = serde_json::to_string(&sheared).unwrap();
        let mut restored: symgcad::output::SolveResult = serde_json::from_str(&encoded).unwrap();
        assert!(restored.problem.solver.affine_factorization);
        assert!(verify(&restored).unwrap().verified);
        restored.problem.solver.affine_factorization = false;
        assert!(verify(&restored).unwrap().verified);
        restored.cells.pop();
        assert!(verify(&restored).is_err());
    }
}
