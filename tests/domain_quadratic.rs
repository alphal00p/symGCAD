use symgcad::{
    algebra::{Algebra, Poly},
    domain::{
        CertificateMethod, CertificateOptions, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

fn setup(names: &[&str], constraints: &[&str]) -> (Algebra, Vec<Poly>) {
    let a = Algebra::new(&names.iter().map(|s| (*s).into()).collect::<Vec<_>>()).unwrap();
    let c = constraints.iter().map(|s| a.parse(s).unwrap()).collect();
    (a, c)
}

fn physical() -> (Algebra, Vec<Poly>, Poly, DomainCertificate) {
    let (a, constraints) = setup(&["m", "x", "y"], &["m", "4-m", "x", "y"]);
    let p = a.parse("(x+y+1)^2-m*(x+y)").unwrap();
    let c = certify_nonzero(
        &a,
        &p,
        &derive_domain(&a, &constraints, &[]).unwrap(),
        &CertificateOptions::default(),
    )
    .unwrap()
    .unwrap();
    (a, constraints, p, c)
}

#[test]
fn parameterized_quadratic_has_exact_negative_discriminant_on_original_box() {
    let (a, constraints, p, _) = physical();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let old = CertificateOptions {
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    };
    for p in [p, a.parse("-((x+y+1)^2-m*(x+y))").unwrap()] {
        assert!(certify_nonzero(&a, &p, &domain, &old).unwrap().is_none());
        let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
            .unwrap()
            .unwrap();
        let CertificateMethod::QuadraticNoRealRoots {
            variable,
            leading,
            negative_discriminant,
        } = &c.method
        else {
            panic!("expected nonrecursive quadratic proof")
        };
        assert_eq!(*variable, 1);
        assert_eq!(leading.sign, c.sign);
        assert_eq!(negative_discriminant.sign, 1);
        assert_eq!(
            a.parse(&negative_discriminant.polynomial).unwrap(),
            a.parse("m*(4-m)").unwrap()
        );
        assert!(matches!(
            negative_discriminant.method,
            CertificateMethod::Bernstein { .. }
        ));
        assert_eq!(leading.limits.quadratic_no_real_roots_candidates, 0);
        assert_eq!(
            negative_discriminant
                .limits
                .quadratic_no_real_roots_candidates,
            0
        );
        verify_certificate(&a, &p, &constraints, &[], &c).unwrap();
        let decoded = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        verify_certificate(&a, &p, &constraints, &[], &decoded).unwrap();
    }
}

#[test]
fn quadratic_no_real_roots_can_use_a_nonconstant_exact_leading_proof() {
    let (a, constraints) = setup(&["s", "m", "x"], &["s-4", "m", "4-m", "x"]);
    let p = a
        .parse("1+2*x+x^2+m*x+2*m*x^2-2*s*x-2*s*x^2-s*m*x^2+s^2*x^2")
        .unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
        .unwrap()
        .unwrap();
    let CertificateMethod::QuadraticNoRealRoots {
        variable,
        leading,
        negative_discriminant,
    } = &c.method
    else {
        panic!("expected quadratic proof with nonconstant leading coefficient")
    };
    assert_eq!(*variable, 2);
    assert!(!a.parse(&leading.polynomial).unwrap().is_constant());
    assert!(matches!(
        leading.method,
        CertificateMethod::AffineSquarePlusPositiveResidual { .. }
    ));
    assert_eq!(
        a.parse(&negative_discriminant.polynomial).unwrap(),
        a.parse("m*(4-m)").unwrap()
    );
    verify_certificate(&a, &p, &constraints, &[], &c).unwrap();
}

#[test]
fn quadratic_replay_rejects_wrong_identity_domain_sign_axis_and_nested_evidence() {
    let (a, constraints, p, c) = physical();
    let alter = |change: &dyn Fn(&mut CertificateMethod)| {
        let mut wrong = c.clone();
        change(&mut wrong.method);
        assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    };
    for bad_axis in [0, 999] {
        alter(&|m| {
            if let CertificateMethod::QuadraticNoRealRoots { variable, .. } = m {
                *variable = bad_axis;
            }
        });
    }
    alter(&|m| {
        if let CertificateMethod::QuadraticNoRealRoots { leading, .. } = m {
            leading.polynomial = "2".into();
        }
    });
    alter(&|m| {
        if let CertificateMethod::QuadraticNoRealRoots {
            negative_discriminant,
            ..
        } = m
        {
            negative_discriminant.polynomial = "1".into();
        }
    });
    alter(&|m| {
        if let CertificateMethod::QuadraticNoRealRoots {
            negative_discriminant,
            ..
        } = m
        {
            negative_discriminant.sign = -1;
        }
    });
    alter(&|m| {
        if let CertificateMethod::QuadraticNoRealRoots { leading, .. } = m {
            leading.domain = derive_domain(&a, &constraints[2..], &[]).unwrap();
        }
    });
    for which in [0, 1] {
        let mut wrong = c.clone();
        let CertificateMethod::QuadraticNoRealRoots {
            leading,
            negative_discriminant,
            ..
        } = &mut wrong.method
        else {
            unreachable!()
        };
        if which == 0 {
            *leading = Box::new(c.clone());
        } else {
            *negative_discriminant = Box::new(c.clone());
        }
        assert!(
            verify_certificate(&a, &p, &constraints, &[], &wrong)
                .unwrap_err()
                .to_string()
                .contains("depth one")
        );
    }
    let mut wrong = c.clone();
    wrong.sign = -1;
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    let mut wrong = c;
    wrong.limits.quadratic_no_real_roots_candidates = 0;
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
}

#[test]
fn quadratic_strict_discriminant_excludes_zeros_and_degree_drops() {
    let (a, constraints) = setup(&["t", "x", "y"], &[]);
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    for expression in ["(x-y)^2", "x^2-y", "t*x^2+x+y", "x^3+y"] {
        let p = a.parse(expression).unwrap();
        assert!(
            certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
                .unwrap()
                .is_none()
        );
    }
    let (a, constraints) = setup(&["m", "x", "y"], &["m", "4-m"]);
    let p = a.parse("(x+y+1)^2-m*(x+y)").unwrap();
    let c = certify_nonzero(
        &a,
        &p,
        &derive_domain(&a, &constraints, &[]).unwrap(),
        &CertificateOptions::default(),
    )
    .unwrap()
    .unwrap();
    verify_certificate(&a, &p, &constraints, &[], &c).unwrap();
    assert!(matches!(
        c.method,
        CertificateMethod::QuadraticNoRealRoots { .. }
    ));
}

#[test]
fn quadratic_options_preserve_historical_ablation_and_replay_budgets() {
    let mut old = serde_json::to_value(CertificateOptions::default()).unwrap();
    old.as_object_mut()
        .unwrap()
        .remove("quadratic_no_real_roots_candidates");
    assert_eq!(
        serde_json::from_value::<CertificateOptions>(old)
            .unwrap()
            .quadratic_no_real_roots_candidates,
        0
    );
    let (a, constraints, p, c) = physical();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let invalid = CertificateOptions {
        quadratic_no_real_roots_candidates: 17,
        ..Default::default()
    };
    assert!(certify_nonzero(&a, &p, &domain, &invalid).is_err());
    let mut wrong = c.clone();
    wrong.limits.max_terms = 1;
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    for expression in ["2^600*((x+y+1)^2-m*(x+y))", "(x+y^33+1)^2-m*(x+y^33)"] {
        let q = a.parse(expression).unwrap();
        let mut wrong = c.clone();
        wrong.polynomial = a.format(&q);
        assert!(verify_certificate(&a, &q, &constraints, &[], &wrong).is_err());
        let found = certify_nonzero(&a, &q, &domain, &CertificateOptions::default()).unwrap();
        assert!(
            !found.is_some_and(|c| matches!(
                c.method,
                CertificateMethod::QuadraticNoRealRoots { .. }
            ))
        );
    }
}
