use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    domain::{
        CertificateMethod, CertificateOptions, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

const C0: &str = "(x+y+z+w)^2-u*(x+y)*(z+w)";

fn setup(names: &[&str], constraints: &[&str]) -> (Algebra, Vec<Poly>) {
    let algebra = Algebra::new(&names.iter().map(|s| (*s).into()).collect::<Vec<_>>()).unwrap();
    let constraints = constraints
        .iter()
        .map(|s| algebra.parse(s).unwrap())
        .collect();
    (algebra, constraints)
}
fn enabled() -> CertificateOptions {
    CertificateOptions {
        quadratic_component_candidates: 32,
        ..Default::default()
    }
}
fn isolated() -> CertificateOptions {
    CertificateOptions {
        square_residual_candidates: 0,
        quadratic_no_real_roots_candidates: 0,
        ..enabled()
    }
}
fn certify(
    a: &Algebra,
    p: &Poly,
    c: &[Poly],
    options: &CertificateOptions,
) -> Option<DomainCertificate> {
    certify_nonzero(a, p, &derive_domain(a, c, &[]).unwrap(), options).unwrap()
}

#[test]
fn authentic_threshold_components_extend_the_weighted_portfolio() {
    let (a, c) = setup(
        &["u", "x", "y", "z", "w", "v"],
        &["4-u", "x", "y", "z", "w", "v"],
    );
    for text in [
        C0.to_owned(),
        format!("({C0})+v^2+(2*(x+y)+(31/18)*(z+w))*v"),
    ] {
        let p = a.parse(&text).unwrap();
        assert!(certify(&a, &p, &c, &CertificateOptions::default()).is_none());
        assert!(
            certify(
                &a,
                &p,
                &c,
                &CertificateOptions {
                    weighted_square_candidates: 72,
                    ..Default::default()
                }
            )
            .is_none()
        );
        let proof = certify(&a, &p, &c, &enabled()).unwrap();
        let CertificateMethod::QuadraticComponentsPlusPositiveResidual { squares, .. } =
            &proof.method
        else {
            panic!("expected PSD component")
        };
        assert_eq!(squares.len(), 1);
        assert_eq!(
            squares[0].coefficients.iter().filter(|s| *s != "0").count(),
            4
        );
        let decoded = serde_json::from_slice(&serde_json::to_vec(&proof).unwrap()).unwrap();
        verify_certificate(&a, &p, &c, &[], &decoded).unwrap();
    }
    // Weighted homogeneous cubic components are a distinct proof class.
    let (a, c) = setup(
        &["u", "x1", "x2", "x3", "x4", "x5", "x7"],
        &["4-u", "x1", "x2", "x3", "x4", "x5", "x7"],
    );
    let p=a.parse("(x4+x5+x7)*((x1+x2+x3)^2-u*x1*x3-(5/18)*x2*x3)+(x1+x2+x3)*((x4+x5+x7)^2-u*x5*x7-(5/18)*x4*x7)").unwrap();
    assert!(certify(&a, &p, &c, &enabled()).is_none());
    let proof = certify(
        &a,
        &p,
        &c,
        &CertificateOptions {
            weighted_square_candidates: 72,
            ..enabled()
        },
    )
    .unwrap();
    assert!(matches!(
        proof.method,
        CertificateMethod::WeightedSquaresPlusPositiveResidual { .. }
    ));
    verify_certificate(&a, &p, &c, &[], &proof).unwrap();
}

#[test]
fn rational_rank_one_and_rank_two_forms_on_unrelated_supports() {
    let (a, c) = setup(&["x", "y", "z", "w", "h"], &["x", "y", "z", "w", "h"]);
    for (text, count) in [
        ("(2*x-3*y+z)^2+h*x*y*z+w", 1),
        ("(x-y)^2+(y-z)^2+h*x*z+w", 2),
        ("(1/7)*(x-(3/5)*y+2*z)^2+h*x*y+w", 1),
    ] {
        for sign in [1, -1] {
            let p = a.parse(&format!("({sign})*({text})")).unwrap();
            let proof = certify(&a, &p, &c, &isolated()).unwrap();
            assert_eq!(proof.sign, sign);
            let CertificateMethod::QuadraticComponentsPlusPositiveResidual { squares, .. } =
                &proof.method
            else {
                panic!("expected PSD component")
            };
            assert_eq!(squares.len(), count);
            let decoded = serde_json::from_slice(&serde_json::to_vec(&proof).unwrap()).unwrap();
            verify_certificate(&a, &p, &c, &[], &decoded).unwrap();
        }
    }
}

#[test]
fn preserves_zero_seams_and_rejects_indefinite_components() {
    let (a, c) = setup(&["x", "y", "z", "w", "h"], &["x", "y", "z", "w", "h"]);
    for text in [
        "(x+y-z-w)^2",
        "(x-y)^2+(y-z)^2",
        "x^2+y^2-3*x*y+1",
        "-2*x*y+y^2+1",
        "-x^2-2*x*y+y^2+1",
        "(x+y-z-w)^2-h*x*y",
    ] {
        assert!(
            certify(&a, &a.parse(text).unwrap(), &c, &isolated()).is_none(),
            "{text}"
        );
    }
    let p = a.parse("(x+y-z-w)^2+h*x*y").unwrap();
    assert!(certify(&a, &p, &c[..4], &isolated()).is_none());
    let (a, c) = setup(&["u", "x", "y", "z", "w"], &["u-4", "x", "y", "z", "w"]);
    assert!(certify(&a, &a.parse(C0).unwrap(), &c, &enabled()).is_none());
}

#[test]
fn upper_orientation_and_original_strict_bounds_are_replayed() {
    let (a, c) = setup(
        &["u", "x", "y", "z", "w"],
        &["u", "4-u", "x", "y", "z", "w"],
    );
    let p = a.parse(C0).unwrap();
    let proof = certify(&a, &p, &c, &enabled()).unwrap();
    let CertificateMethod::QuadraticComponentsPlusPositiveResidual {
        upper_variables, ..
    } = &proof.method
    else {
        panic!("expected PSD")
    };
    assert_eq!(upper_variables, &vec![0]);
    verify_certificate(&a, &p, &c, &[], &proof).unwrap();
    let changed = c
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (i != 1).then_some(p.clone()))
        .collect::<Vec<_>>();
    assert!(verify_certificate(&a, &p, &changed, &[], &proof).is_err());
}

#[test]
fn rejects_forged_typed_evidence_before_expression_expansion() {
    let (a, c) = setup(&["u", "x", "y", "z", "w"], &["4-u", "x", "y", "z", "w"]);
    let p = a.parse(C0).unwrap();
    let proof = certify(&a, &p, &c, &enabled()).unwrap();
    for mutation in 0..20 {
        let mut wrong = proof.clone();
        let CertificateMethod::QuadraticComponentsPlusPositiveResidual {
            squares, residual, ..
        } = &mut wrong.method
        else {
            unreachable!()
        };
        match mutation {
            0 => squares[0].weight = "-1".into(),
            1 => squares[0].weight = "0".into(),
            2 => squares[0].weight = "1/0".into(),
            3 => squares[0].weight = "2^1000000000".into(),
            4 => squares[0].coefficients[1] = "-2^(2^1000000000)".into(),
            5 => squares[0].coefficients[1] = "9".repeat(1000),
            6 => squares[0].coefficients[1] = "--1".into(),
            7 => squares[0].coefficients[1] = "1/000".into(),
            8 => squares[0].coefficients.fill("0".into()),
            9 => {
                squares[0].coefficients.pop();
            }
            10 => squares[0].coefficients[1] = "12345".into(),
            11 => residual[0].coefficient = "2^(2^1000000000)".into(),
            12 => residual[0].exponents[0] = u32::MAX,
            13 => residual.extend(vec![residual[0].clone(); 257]),
            14 => residual.clear(),
            15 => wrong.polynomial = "2^(2^1000000000)".into(),
            16 => wrong.sign = -wrong.sign,
            17 => wrong.limits.quadratic_component_candidates = 0,
            18 => squares.extend(vec![squares[0].clone(); 17]),
            _ => squares.clear(),
        }
        assert!(
            verify_certificate(&a, &p, &c, &[], &wrong).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn validates_full_variable_map_and_resource_budgets() {
    let (a, c) = setup(&["x", "y", "z", "w", "h"], &["x", "y", "z", "w", "h"]);
    let p = a.parse("(x+y-z-w)^2+h*x*y").unwrap();
    let proof = certify(&a, &p, &c, &enabled()).unwrap();
    let (foreign, _) = setup(&["a", "b", "c", "d", "e"], &[]);
    let q = foreign.parse("(a+b-c-d)^2+e*a*b").unwrap();
    assert_eq!(a.format(&p), a.format(&q));
    assert!(verify_certificate(&a, &q, &c, &[], &proof).is_err());
    assert!(
        certify(
            &a,
            &p,
            &c,
            &CertificateOptions {
                work_limit: 1,
                ..isolated()
            }
        )
        .is_none()
    );
    let bad = CertificateOptions {
        quadratic_component_candidates: 33,
        ..Default::default()
    };
    assert!(certify_nonzero(&a, &p, &derive_domain(&a, &c, &[]).unwrap(), &bad).is_err());
    let p = a.parse("(x+y-z-w)^2+h^33*x*y").unwrap();
    assert!(certify(&a, &p, &c, &isolated()).is_none());
}

#[test]
fn independent_options_roundtrip_and_optimized_result_verify() {
    let old = serde_json::to_value(CertificateOptions::default()).unwrap();
    assert!(old.get("quadratic_component_candidates").is_none());
    assert_eq!(
        serde_json::from_value::<CertificateOptions>(old)
            .unwrap()
            .quadratic_component_candidates,
        0
    );
    let text = r#"
variables=["u","x","y","z","w"]
constraints=["u<4","x>0","y>0","z>0","w>0"]
split=["(x+y+z+w)^2-u*(x+y)*(z+w)"]
[solver]
engine="optimized"
positivity=true
input_positivity=true
quadratic_component_positivity=true
"#;
    let p = Problem::from_toml(text).unwrap();
    assert!(!p.solver.weighted_square_positivity);
    assert_eq!(
        p.solver
            .certificate_options()
            .quadratic_component_candidates,
        32
    );
    let decoded: Problem = toml::from_str(&toml::to_string(&p).unwrap()).unwrap();
    assert!(decoded.solver.quadratic_component_positivity);
    let mut disabled = p.clone();
    disabled.solver.quadratic_component_positivity = false;
    assert!(
        !toml::to_string(&disabled)
            .unwrap()
            .contains("quadratic_component_positivity")
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
            .contains("quadratic_components_plus_positive_residual")
    );
}

#[test]
fn local_proof_preserves_both_sides_of_the_interior_root() {
    let mut p = Problem::default();
    p.variables = ["u", "x", "y", "z", "w", "v"].map(str::to_owned).to_vec();
    p.constraints = ["u<4", "x>0", "y>0", "z>0", "w>0", "v>0"]
        .map(str::to_owned)
        .to_vec();
    p.split = vec![format!("({C0})*v-1")];
    p.solver.engine = "local".into();
    p.solver.positivity = true;
    p.solver.quadratic_component_positivity = true;
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
        CertificateMethod::QuadraticComponentsPlusPositiveResidual { .. }
    )));
    let decoded = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    symgcad::verify::verify(&decoded).unwrap();
    assert!(result.cells.iter().any(|cell| cell.signs == vec![-1]));
    assert!(result.cells.iter().any(|cell| cell.signs == vec![1]));
}
