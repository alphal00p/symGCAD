use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    domain::{
        CertificateMethod, CertificateOptions, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

const FAMILY: &str = "z*(x-y)^2+x^2*(y-z)^2+y^3*(z-x)^2+1";

fn setup(names: &[&str], constraints: &[&str]) -> (Algebra, Vec<Poly>) {
    let a = Algebra::new(&names.iter().map(|s| (*s).into()).collect::<Vec<_>>()).unwrap();
    let c = constraints.iter().map(|s| a.parse(s).unwrap()).collect();
    (a, c)
}
fn enabled() -> CertificateOptions {
    CertificateOptions {
        weighted_square_candidates: 72,
        ..Default::default()
    }
}
fn certify(
    a: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    options: &CertificateOptions,
) -> Option<DomainCertificate> {
    certify_nonzero(a, p, &derive_domain(a, constraints, &[]).unwrap(), options).unwrap()
}

#[test]
fn authentic_endpoint_requires_multiple_positive_monomial_weights() {
    let names = ["u", "x1", "x2", "x3", "x4", "x5", "x7"];
    let (a, c) = setup(&names, &["4-u", "x1", "x2", "x3", "x4", "x5", "x7"]);
    let p=a.parse("(x4+x5+x7)*((x1+x2+x3)^2-u*x1*x3-(5/18)*x2*x3)+(x1+x2+x3)*((x4+x5+x7)^2-u*x5*x7-(5/18)*x4*x7)").unwrap();
    assert!(certify(&a, &p, &c, &CertificateOptions::default()).is_none());
    let proof = certify(&a, &p, &c, &enabled()).unwrap();
    let CertificateMethod::WeightedSquaresPlusPositiveResidual { squares, .. } = &proof.method
    else {
        panic!("expected weighted square evidence");
    };
    assert_eq!(squares.len(), 6);
    assert!(squares.iter().all(|s| s.common.iter().sum::<u32>() == 1));
    verify_certificate(&a, &p, &c, &[], &proof).unwrap();
}

#[test]
fn broad_degree_supports_and_both_signs_replay_after_serialization() {
    let (a, c) = setup(&["x", "y", "z"], &["x", "y", "z"]);
    for text in [FAMILY, "z*(x-y)^2+x^3*(y-z)^2+y^5*(z-x)^2+1"] {
        for sign in [1, -1] {
            let p = a.parse(&format!("({sign})*({text})")).unwrap();
            assert!(certify(&a, &p, &c, &CertificateOptions::default()).is_none());
            let proof = certify(&a, &p, &c, &enabled()).unwrap();
            assert_eq!(proof.sign, sign);
            let CertificateMethod::WeightedSquaresPlusPositiveResidual { squares, .. } =
                &proof.method
            else {
                panic!("expected weighted evidence");
            };
            assert_eq!(squares.len(), 3);
            let decoded: DomainCertificate =
                serde_json::from_slice(&serde_json::to_vec(&proof).unwrap()).unwrap();
            verify_certificate(&a, &p, &c, &[], &decoded).unwrap();
        }
    }
    // A merged -6xyz needs partial multi-pair cancellation, outside this rule.
    let cyclic = a.parse("z*(x-y)^2+x*(y-z)^2+y*(z-x)^2+1").unwrap();
    assert!(certify(&a, &cyclic, &c, &enabled()).is_none());
}

#[test]
fn preserves_interior_zero_seams_and_declines_unproved_weights() {
    let (a, c) = setup(&["x", "y", "z", "w"], &["x", "y", "z", "w"]);
    for text in [
        "(x-y)^2",
        "z*(x-y)^2+x^2*(y-z)^2+y^3*(z-x)^2",
        "(x+y-z-w)^2",
        "1-w*(x-y)^2",
    ] {
        assert!(
            certify(&a, &a.parse(text).unwrap(), &c, &enabled()).is_none(),
            "{text}"
        );
    }
    let restricted = vec![
        a.parse("x").unwrap(),
        a.parse("y").unwrap(),
        a.parse("z").unwrap(),
    ];
    assert!(
        certify(
            &a,
            &a.parse("w*(x-y)^2+z").unwrap(),
            &restricted,
            &enabled()
        )
        .is_none()
    );
}

#[test]
fn positive_box_orientation_is_recorded_and_rechecked() {
    let (a, c) = setup(&["x", "y", "z", "w"], &["x", "y", "z", "w", "1-w"]);
    let p = a.parse("(1-w)*(x-y)^2+z").unwrap();
    let options = CertificateOptions {
        square_residual_candidates: 0,
        quadratic_no_real_roots_candidates: 0,
        ..enabled()
    };
    let proof = certify(&a, &p, &c, &options).unwrap();
    let CertificateMethod::WeightedSquaresPlusPositiveResidual {
        upper_variables, ..
    } = &proof.method
    else {
        panic!("expected weighted upper orientation");
    };
    assert_eq!(upper_variables, &vec![3]);
    verify_certificate(&a, &p, &c, &[], &proof).unwrap();
    assert!(verify_certificate(&a, &p, &c[..4], &[], &proof).is_err());
}

#[test]
fn rejects_forged_weights_exponents_residuals_and_disabled_proofs() {
    let (a, c) = setup(&["x", "y", "z"], &["x", "y", "z"]);
    let p = a.parse(FAMILY).unwrap();
    let proof = certify(&a, &p, &c, &enabled()).unwrap();
    for mutation in 0..15 {
        let mut wrong = proof.clone();
        let CertificateMethod::WeightedSquaresPlusPositiveResidual {
            squares, residual, ..
        } = &mut wrong.method
        else {
            unreachable!();
        };
        match mutation {
            0 => squares[0].weight = "-1".into(),
            1 => squares[0].ratio = "0".into(),
            2 => squares[0].common[0] = u32::MAX,
            3 => squares[0].left.pop().map(|_| ()).unwrap(),
            4 => residual[0].coefficient = "12345".into(),
            5 => wrong.sign = -wrong.sign,
            6 => wrong.limits.weighted_square_candidates = 0,
            7 => squares[0].weight = "2^1000000000".into(),
            8 => squares[0].ratio = "2^(2^1000000000)".into(),
            9 => residual[0].coefficient = "2^(2^1000000000)".into(),
            10 => residual[0].exponents[0] = u32::MAX,
            11 => residual[0].coefficient = "9".repeat(1000),
            12 => wrong.polynomial = "2^(2^1000000000)".into(),
            13 => residual.extend(vec![residual[0].clone(); 257]),
            _ => squares[0].weight = "1/000".into(),
        }
        assert!(
            verify_certificate(&a, &p, &c, &[], &wrong).is_err(),
            "mutation {mutation}"
        );
    }
    let tiny = CertificateOptions {
        work_limit: 1,
        ..enabled()
    };
    assert!(certify(&a, &p, &c, &tiny).is_none());
    let invalid = CertificateOptions {
        weighted_square_candidates: 73,
        ..Default::default()
    };
    assert!(certify_nonzero(&a, &p, &derive_domain(&a, &c, &[]).unwrap(), &invalid).is_err());
}

#[test]
fn expected_polynomial_map_is_checked_before_canonical_spelling() {
    let (a, constraints) = setup(&["x", "y", "z"], &["x", "y", "z"]);
    let p = a.parse(FAMILY).unwrap();
    let proof = certify(&a, &p, &constraints, &enabled()).unwrap();
    let (foreign, _) = setup(&["a", "b", "c"], &[]);
    let same_shape = foreign
        .parse("c*(a-b)^2+a^2*(b-c)^2+b^3*(c-a)^2+1")
        .unwrap();
    assert_eq!(a.format(&p), a.format(&same_shape));
    assert_ne!(p.variables(), same_shape.variables());
    assert!(verify_certificate(&a, &same_shape, &constraints, &[], &proof).is_err());
}

#[test]
fn serde_defaults_and_toml_preserve_independent_opt_in() {
    let old = serde_json::to_value(CertificateOptions::default()).unwrap();
    assert!(old.get("weighted_square_candidates").is_none());
    assert_eq!(
        serde_json::from_value::<CertificateOptions>(old)
            .unwrap()
            .weighted_square_candidates,
        0
    );
    let text = r#"
variables=["x","y","z"]
constraints=["x>0","y>0","z>0"]
split=["z*(x-y)^2+x^2*(y-z)^2+y^3*(z-x)^2+1"]
[solver]
engine="optimized"
positivity=true
input_positivity=true
weighted_square_positivity=true
"#;
    let p = Problem::from_toml(text).unwrap();
    assert_eq!(
        p.solver.certificate_options().weighted_square_candidates,
        72
    );
    let decoded: Problem = toml::from_str(&toml::to_string(&p).unwrap()).unwrap();
    assert!(decoded.solver.weighted_square_positivity);
    let mut disabled = p.clone();
    disabled.solver.weighted_square_positivity = false;
    assert!(
        !toml::to_string(&disabled)
            .unwrap()
            .contains("weighted_square_positivity")
    );
    let mut invalid = p.clone();
    invalid.solver.positivity = false;
    invalid.solver.input_positivity = false;
    assert!(invalid.validate().is_err());
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, symgcad::output::Status::CompleteGeneric);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![1]);
    let decoded = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    symgcad::verify::verify(&decoded).unwrap();
    assert!(
        serde_json::to_string(&result)
            .unwrap()
            .contains("weighted_squares_plus_positive_residual")
    );
}

#[test]
fn local_guards_use_opt_in_proofs_and_replay_independently() {
    let mut p = Problem::default();
    p.variables = ["x", "y", "z", "w"].map(str::to_owned).to_vec();
    p.constraints = ["x>0", "y>0", "z>0", "w>0"].map(str::to_owned).to_vec();
    p.split = vec![format!("({FAMILY})*w-1")];
    p.solver.engine = "local".into();
    p.solver.positivity = true;
    p.solver.weighted_square_positivity = true;
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, symgcad::output::Status::CompleteGeneric);
    let symgcad::local_projection::LocalEvidence::Local {
        guard_certificates, ..
    } = result.local.as_deref().unwrap()
    else {
        panic!("expected local evidence")
    };
    assert!(guard_certificates.iter().any(|r| matches!(
        r.certificate.method,
        CertificateMethod::WeightedSquaresPlusPositiveResidual { .. }
    )));
    let decoded = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    symgcad::verify::verify(&decoded).unwrap();
    // Both sides of the actual w=1/F interior boundary must remain.
    assert!(result.cells.iter().any(|cell| cell.signs == vec![-1]));
    assert!(result.cells.iter().any(|cell| cell.signs == vec![1]));
}
