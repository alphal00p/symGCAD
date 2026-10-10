use symgcad::algebra::{Algebra, Poly};
use symgcad::domain::{
    BernsteinProof, CertificateMethod, CertificateOptions, DomainCertificate, certify_nonzero,
    derive_domain, homogeneous_anchor, homogeneous_degree, restore_homogeneous_bound,
    verify_certificate,
};

const PHYSICAL_LINEAR: &str = "(x3-x6)^2+(4-pp4)*x3*x6+2*(x0+x1)*(x3+x5+x6)+2*x3*x5+(2-t)*x5*x6";
const PHYSICAL_LINEAR_X6_ONE: &str = "(x3-1)^2+(4-pp4)*x3+2*(x0+x1)*(x3+x5+1)+2*x3*x5+(2-t)*x5";

#[test]
fn affine_square_certifies_homogeneous_physical_coefficient() {
    let (a, constraints) = setup(
        &["pp4", "t", "x0", "x1", "x3", "x5"],
        &["pp4", "4-pp4", "-t", "x0", "x1", "x3", "x5"],
    );
    let p = a.parse(PHYSICAL_LINEAR_X6_ONE).unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let disabled = CertificateOptions {
        square_residual_candidates: 0,
        ..Default::default()
    };
    assert!(
        certify_nonzero(&a, &p, &domain, &disabled)
            .unwrap()
            .is_none()
    );
    for (p, sign) in [
        (p.clone(), 1),
        (
            a.parse(&format!("-({PHYSICAL_LINEAR_X6_ONE})")).unwrap(),
            -1,
        ),
    ] {
        let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
            .unwrap()
            .unwrap();
        assert_eq!(c.sign, sign);
        assert_eq!(c.domain, domain);
        let CertificateMethod::AffineSquarePlusPositiveResidual {
            upper_variables,
            variable,
            weight,
            center,
            residual,
        } = &c.method
        else {
            panic!("expected affine square proof")
        };
        assert_eq!(upper_variables, &[0]);
        assert_eq!(*variable, 4);
        assert_eq!(weight, "1");
        assert_eq!(center, "1");
        assert_eq!(
            a.parse(residual).unwrap(),
            a.parse("pp4*x3+2*(x0+x1)*(x3+x5+1)+2*x3*x5+(2+t)*x5")
                .unwrap()
        );
        verify_certificate(&a, &p, &constraints, &[], &c).unwrap();
        let decoded = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        verify_certificate(&a, &p, &constraints, &[], &decoded).unwrap();
    }
}

#[test]
fn affine_square_replay_checks_center_axis_bounds_and_identity() {
    let (a, constraints) = setup(
        &["pp4", "t", "x0", "x1", "x3", "x5"],
        &["pp4", "4-pp4", "-t", "x0", "x1", "x3", "x5"],
    );
    let p = a.parse(PHYSICAL_LINEAR_X6_ONE).unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
        .unwrap()
        .unwrap();
    let alter = |change: &dyn Fn(&mut CertificateMethod)| {
        let mut wrong = c.clone();
        change(&mut wrong.method);
        assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    };
    for axes in [vec![], vec![0, 0], vec![999], vec![2]] {
        alter(&|m| {
            if let CertificateMethod::AffineSquarePlusPositiveResidual {
                upper_variables, ..
            } = m
            {
                *upper_variables = axes.clone();
            }
        });
    }
    for index in [999, 3] {
        alter(&|m| {
            if let CertificateMethod::AffineSquarePlusPositiveResidual { variable, .. } = m {
                *variable = index;
            }
        });
    }
    for value in ["0", "-1", "2"] {
        alter(&|m| {
            if let CertificateMethod::AffineSquarePlusPositiveResidual { weight, .. } = m {
                *weight = value.into();
            }
        });
    }
    for value in ["0", "-1", "3/2"] {
        alter(&|m| {
            if let CertificateMethod::AffineSquarePlusPositiveResidual { center, .. } = m {
                *center = value.into();
            }
        });
    }
    alter(&|m| {
        if let CertificateMethod::AffineSquarePlusPositiveResidual { residual, .. } = m {
            *residual = "1".into();
        }
    });
    let mut missing_upper = constraints.clone();
    missing_upper.remove(1);
    let mut wrong = c.clone();
    wrong.domain = derive_domain(&a, &missing_upper, &[]).unwrap();
    assert!(verify_certificate(&a, &p, &missing_upper, &[], &wrong).is_err());
    let mut wrong = c;
    wrong.limits.square_residual_candidates = 0;
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
}

#[test]
fn affine_square_requires_strict_residual_and_bounded_rational_data() {
    let (a, constraints) = setup(&["x", "z"], &[]);
    for (expression, residual) in [("(x-1)^2", "0"), ("(x-1)^2+z^2", "z^2"), ("0", "-(x-1)^2")] {
        let p = a.parse(expression).unwrap();
        let c = DomainCertificate {
            version: 1,
            polynomial: a.format(&p),
            domain: derive_domain(&a, &constraints, &[]).unwrap(),
            sign: 1,
            method: CertificateMethod::AffineSquarePlusPositiveResidual {
                upper_variables: vec![],
                variable: 0,
                weight: "1".into(),
                center: "1".into(),
                residual: residual.into(),
            },
            limits: CertificateOptions::default(),
        };
        assert!(verify_certificate(&a, &p, &constraints, &[], &c).is_err());
        assert!(
            certify_nonzero(&a, &p, &c.domain, &c.limits)
                .unwrap()
                .is_none()
        );
    }
    let (a, constraints) = setup(&["x", "z"], &["z"]);
    let p = a.parse("(x-1)^2+z*x^2+1").unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
        .unwrap()
        .unwrap();
    let enormous = a.parse("2^300").unwrap().get_constant().to_string();
    for field in ["weight", "center"] {
        let mut wrong = c.clone();
        let CertificateMethod::AffineSquarePlusPositiveResidual { weight, center, .. } =
            &mut wrong.method
        else {
            panic!("expected affine square proof")
        };
        if field == "weight" {
            *weight = enormous.clone();
        } else {
            *center = enormous.clone();
        }
        let error = verify_certificate(&a, &p, &constraints, &[], &wrong).unwrap_err();
        assert!(error.to_string().contains("height budget"));
    }
    let too_tall = a.parse("(x-2^300)^2+z*x^2+1").unwrap();
    assert!(
        certify_nonzero(&a, &too_tall, &domain, &CertificateOptions::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn affine_square_uses_shifted_coordinates_and_exact_rational_centers() {
    let c = certify(&["x", "z"], &["x-2", "z"], "((x-2)-3/2)^2+z*(x-2)^2+1").unwrap();
    let CertificateMethod::AffineSquarePlusPositiveResidual {
        variable,
        center,
        residual,
        ..
    } = c.method
    else {
        panic!("expected rational affine square proof")
    };
    assert_eq!(variable, 0);
    assert_eq!(center, "3/2");
    let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
    assert_eq!(a.parse(&residual).unwrap(), a.parse("z*x^2+1").unwrap());
}

#[test]
fn affine_square_replay_is_independent_of_discovered_weight_and_center() {
    let (a, constraints) = setup(&["x", "z"], &["z"]);
    let p = a.parse("x^2-x+1+z*x^2").unwrap();
    let c = DomainCertificate {
        version: 1,
        polynomial: a.format(&p),
        domain: derive_domain(&a, &constraints, &[]).unwrap(),
        sign: 1,
        method: CertificateMethod::AffineSquarePlusPositiveResidual {
            upper_variables: vec![],
            variable: 0,
            weight: "1/2".into(),
            center: "1".into(),
            residual: "x^2/2+1/2+z*x^2".into(),
        },
        limits: CertificateOptions::default(),
    };
    verify_certificate(&a, &p, &constraints, &[], &c).unwrap();
}

#[test]
fn square_residual_certifies_physical_box_with_exact_upper_orientation() {
    let (a, constraints) = setup(
        &["pp4", "t", "x0", "x1", "x3", "x5", "x6"],
        &["pp4", "4-pp4", "-t", "x0", "x1", "x3", "x5", "x6"],
    );
    let p = a.parse(PHYSICAL_LINEAR).unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let mut options = CertificateOptions::default();
    options.square_residual_candidates = 0;
    assert!(
        certify_nonzero(&a, &p, &domain, &options)
            .unwrap()
            .is_none()
    );
    options.square_residual_candidates = 16;
    for (p, sign) in [
        (p.clone(), 1),
        (a.parse(&format!("-({PHYSICAL_LINEAR})")).unwrap(), -1),
    ] {
        let c = certify_nonzero(&a, &p, &domain, &options).unwrap().unwrap();
        assert_eq!(c.sign, sign);
        assert_eq!(c.domain, domain);
        let CertificateMethod::SquarePlusPositiveResidual {
            upper_variables,
            variables,
            weight,
            residual,
        } = &c.method
        else {
            panic!("expected square residual proof")
        };
        assert_eq!(upper_variables, &[0]);
        assert_eq!(variables, &[4, 6]);
        assert_eq!(weight, "1");
        assert_eq!(
            a.parse(residual).unwrap(),
            a.parse("pp4*x3*x6+2*(x0+x1)*(x3+x5+x6)+2*x3*x5+(2+t)*x5*x6")
                .unwrap()
        );
        verify_certificate(&a, &p, &constraints, &[], &c).unwrap();
        let serialized = serde_json::to_string(&c).unwrap();
        let decoded = serde_json::from_str(&serialized).unwrap();
        verify_certificate(&a, &p, &constraints, &[], &decoded).unwrap();
    }
}

#[test]
fn square_residual_replay_checks_identity_strictness_and_bound_premises() {
    let (a, constraints) = setup(
        &["pp4", "t", "x0", "x1", "x3", "x5", "x6"],
        &["pp4", "4-pp4", "-t", "x0", "x1", "x3", "x5", "x6"],
    );
    let p = a.parse(PHYSICAL_LINEAR).unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
        .unwrap()
        .unwrap();
    let alter = |change: &dyn Fn(&mut CertificateMethod)| {
        let mut wrong = c.clone();
        change(&mut wrong.method);
        assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    };
    for axes in [vec![0, 0], vec![999], vec![2], vec![]] {
        alter(&|m| {
            if let CertificateMethod::SquarePlusPositiveResidual {
                upper_variables, ..
            } = m
            {
                *upper_variables = axes.clone();
            }
        });
    }
    for indices in [[4, 4], [6, 4], [4, 999]] {
        alter(&|m| {
            if let CertificateMethod::SquarePlusPositiveResidual { variables, .. } = m {
                *variables = indices;
            }
        });
    }
    for value in ["0", "-1", "2"] {
        alter(&|m| {
            if let CertificateMethod::SquarePlusPositiveResidual { weight, .. } = m {
                *weight = value.into();
            }
        });
    }
    alter(&|m| {
        if let CertificateMethod::SquarePlusPositiveResidual { residual, .. } = m {
            *residual = "1".into();
        }
    });
    let mut weakened = constraints.clone();
    weakened.remove(1);
    assert!(verify_certificate(&a, &p, &weakened, &[], &c).is_err());
    let mut forged = c;
    forged.domain = derive_domain(&a, &weakened, &[]).unwrap();
    assert!(verify_certificate(&a, &p, &weakened, &[], &forged).is_err());
    assert!(certify(&["x", "y"], &["x", "y"], "(x-y)^2").is_none());
    assert!(certify(&["x", "y", "z"], &[], "(x-y)^2+z^2").is_none());
}

#[test]
fn square_residual_replay_does_not_depend_on_discovery_weight() {
    let (a, constraints) = setup(&["x", "y", "z"], &["x", "y", "z"]);
    // Degree-three z*x*y makes rational LDL inapplicable. Both positive
    // rational square weights 1 and 1/2 yield valid strict residual proofs.
    let p = a.parse("x^2-x*y+y^2+z*x*y+1").unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let certificate = DomainCertificate {
        version: 1,
        polynomial: a.format(&p),
        domain,
        sign: 1,
        method: CertificateMethod::SquarePlusPositiveResidual {
            upper_variables: vec![],
            variables: [0, 1],
            weight: "1/2".into(),
            residual: "x^2/2+y^2/2+z*x*y+1".into(),
        },
        limits: CertificateOptions::default(),
    };
    verify_certificate(&a, &p, &constraints, &[], &certificate).unwrap();
}

#[test]
fn square_residual_identity_uses_shifted_not_original_coordinates() {
    let c = certify(
        &["x", "y", "z"],
        &["x-1", "y-2", "z"],
        "((x-1)-(y-2))^2+z*(x-1)*(y-2)+1",
    )
    .unwrap();
    let CertificateMethod::SquarePlusPositiveResidual {
        upper_variables,
        variables,
        residual,
        ..
    } = c.method
    else {
        panic!("expected shifted square residual")
    };
    assert!(upper_variables.is_empty());
    assert_eq!(variables, [0, 1]);
    let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
    assert_eq!(a.parse(&residual).unwrap(), a.parse("z*x*y+1").unwrap());
}

#[test]
fn square_residual_option_preserves_historical_proofs_and_hard_caps() {
    let mut old = serde_json::to_value(CertificateOptions::default()).unwrap();
    old.as_object_mut()
        .unwrap()
        .remove("square_residual_candidates");
    let old: CertificateOptions = serde_json::from_value(old).unwrap();
    assert_eq!(old.square_residual_candidates, 0);
    let (a, constraints) = setup(&["x", "y"], &["x", "y"]);
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let invalid = CertificateOptions {
        square_residual_candidates: 65,
        ..Default::default()
    };
    assert!(certify_nonzero(&a, &a.parse("x+y").unwrap(), &domain, &invalid).is_err());
}

fn setup(names: &[&str], constraints: &[&str]) -> (Algebra, Vec<Poly>) {
    let a = Algebra::new(&names.iter().map(|s| (*s).into()).collect::<Vec<_>>()).unwrap();
    let constraints = constraints.iter().map(|s| a.parse(s).unwrap()).collect();
    (a, constraints)
}

fn certify(names: &[&str], constraints: &[&str], expression: &str) -> Option<DomainCertificate> {
    let (a, constraints) = setup(names, constraints);
    let p = a.parse(expression).unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let certificate = certify_nonzero(&a, &p, &domain, &CertificateOptions::default()).unwrap();
    if let Some(c) = &certificate {
        verify_certificate(&a, &p, &constraints, &[], c).unwrap();
    }
    certificate
}

#[test]
fn bounds_use_only_original_coordinate_affine_constraints_and_exclude_targets() {
    let (a, constraints) = setup(
        &["x", "y", "s"],
        &["2*x-2", "x-3", "7-x", "x+y-50", "y^2-1", "s-4", "y"],
    );
    let domain = derive_domain(&a, &constraints, &[6]).unwrap();
    assert_eq!(domain.bounds[0].lower.as_deref(), Some("3"));
    assert_eq!(domain.bounds[0].upper.as_deref(), Some("7"));
    assert_eq!(domain.bounds[0].lower_constraint, Some(1));
    assert!(domain.bounds[1].lower.is_none());
    assert!(domain.bounds[1].upper.is_none());
    assert_eq!(domain.bounds[2].lower.as_deref(), Some("4"));
    assert!(
        certify_nonzero(
            &a,
            &a.parse("y").unwrap(),
            &domain,
            &CertificateOptions::default()
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn coefficient_certificates_cover_shifts_upper_bounds_and_negative_signs() {
    let shifted = certify(&["s", "x"], &["s-4", "x"], "(s-4)^2+x").unwrap();
    assert_eq!(shifted.sign, 1);
    assert!(matches!(
        shifted.method,
        CertificateMethod::ShiftedCoefficients
    ));
    let negative = certify(&["x"], &["-x-2"], "x^3-1").unwrap();
    assert_eq!(negative.sign, -1);
}

#[test]
fn semidefinite_is_not_strictly_nonzero_on_an_unrestricted_domain() {
    assert!(certify(&["x", "t"], &[], "x^2+t^2").is_none());
    assert!(certify(&["x", "t"], &["x"], "x+t^2").is_some());
    assert!(certify(&["x"], &[], "1+x^2").is_some());
    assert!(certify(&["x"], &[], "x^2").is_none());
    assert!(certify(&["x"], &["x"], "x^2").is_some());
}

#[test]
fn polya_certifies_a_positive_homogeneous_form_with_mixed_coefficients() {
    let c = certify(&["x", "y"], &["x", "y"], "x^2-x*y+y^2").unwrap();
    assert_eq!(c.sign, 1);
    assert!(matches!(c.method, CertificateMethod::Polya { order: 1 }));
    let negative = certify(&["x", "y"], &["x", "y"], "-x^2+x*y-y^2").unwrap();
    assert_eq!(negative.sign, -1);
}

#[test]
fn polya_does_not_remove_an_interior_zero() {
    assert!(certify(&["x", "y"], &["x", "y"], "(x-y)^2").is_none());
}

#[test]
fn sturm_certifies_mixed_coefficient_univariate_factors_on_unbounded_domains() {
    for (bounds, expression, sign) in [
        (vec![], "x^4-x^2+1", 1),
        (vec!["x"], "-x^4+x^2-1", -1),
        (vec!["x-100"], "x^4-x^2+1", 1),
        (vec!["-100-x"], "x^4-x^2+1", 1),
    ] {
        let c = certify(&["unused", "x"], &bounds, expression).unwrap();
        assert_eq!(c.sign, sign);
        // Shifting far outside the roots can also expose a coefficient proof.
        if bounds.is_empty() || bounds == ["x"] {
            assert!(matches!(
                c.method,
                CertificateMethod::UnivariateSturm { variable: 1 }
            ));
        }
    }
}

#[test]
fn sturm_keeps_repeated_and_close_roots_and_checks_strict_endpoints() {
    let (a, constraints) = setup(&["x"], &["x", "1-x"]);
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    // Force replay of the Sturm method to test endpoint conventions directly,
    // independently of the earlier coefficient/Bernstein portfolio choices.
    for (expression, valid, sign) in [
        ("x*(1-x)", true, 1),
        ("-x^2*(1-x)^2", true, -1),
        ("(x-1/3)^2", false, 1),
        ("(x-1/3)*(x-1/3-1/1000000000000)", false, 1),
    ] {
        let p = a.parse(expression).unwrap();
        let certificate = DomainCertificate {
            version: 1,
            polynomial: a.format(&p),
            domain: domain.clone(),
            sign,
            method: CertificateMethod::UnivariateSturm { variable: 0 },
            limits: CertificateOptions::default(),
        };
        assert_eq!(
            verify_certificate(&a, &p, &constraints, &[], &certificate).is_ok(),
            valid
        );
    }
}

#[test]
fn sturm_replay_rejects_wrong_axis_sign_and_budget() {
    let (a, constraints) = setup(&["x", "y"], &[]);
    let p = a.parse("y^4-y^2+1").unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let certificate = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
        .unwrap()
        .unwrap();
    let mut wrong = certificate.clone();
    wrong.method = CertificateMethod::UnivariateSturm { variable: 0 };
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    wrong = certificate.clone();
    wrong.sign = -1;
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    wrong = certificate;
    wrong.limits.work_limit = 1;
    assert!(verify_certificate(&a, &p, &constraints, &[], &wrong).is_err());
    assert!(certify(&["x"], &[], "x^66-x^2+1").is_none());
}

#[test]
fn bernstein_allows_zeros_only_on_original_strict_endpoints() {
    let c = certify(&["x"], &["x", "1-x"], "x*(1-x)").unwrap();
    assert!(matches!(
        c.method,
        CertificateMethod::Bernstein {
            proof: BernsteinProof::Leaf
        }
    ));
    let c = certify(&["x", "y"], &["x", "1-x", "y", "1-y"], "x*(1-x)*y*(1-y)").unwrap();
    assert_eq!(c.sign, 1);
}

#[test]
fn bernstein_subdivision_proves_strict_positivity_across_the_new_seam() {
    let c = certify(&["x"], &["x", "1-x"], "(x^2-x+1/3)^2").unwrap();
    assert!(matches!(
        c.method,
        CertificateMethod::Bernstein {
            proof: BernsteinProof::Bisect { .. }
        }
    ));
    // An actual interior zero cannot be covered by positive leaf interiors.
    assert!(certify(&["x"], &["x", "1-x"], "(x-1/3)^2").is_none());
}

#[test]
fn checker_rejects_a_forged_bernstein_certificate_that_hides_a_zero_on_a_seam() {
    let (a, constraints) = setup(&["x"], &["x", "1-x"]);
    let p = a.parse("(x-1/2)^2").unwrap();
    let forged = DomainCertificate {
        version: 1,
        polynomial: a.format(&p),
        domain: derive_domain(&a, &constraints, &[]).unwrap(),
        sign: 1,
        method: CertificateMethod::Bernstein {
            proof: BernsteinProof::Bisect {
                variable: 0,
                left: Box::new(BernsteinProof::Leaf),
                right: Box::new(BernsteinProof::Leaf),
            },
        },
        limits: CertificateOptions::default(),
    };
    assert!(verify_certificate(&a, &p, &constraints, &[], &forged).is_err());
}

#[test]
fn checker_rebuilds_domain_and_checks_sign_and_expected_polynomial() {
    let (a, constraints) = setup(&["x"], &["x"]);
    let p = a.parse("x+1").unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let c = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
        .unwrap()
        .unwrap();
    let mut changed = c.clone();
    changed.sign = -1;
    assert!(verify_certificate(&a, &p, &constraints, &[], &changed).is_err());
    let mut changed = c.clone();
    changed.domain.bounds[0].lower = Some("2".into());
    assert!(verify_certificate(&a, &p, &constraints, &[], &changed).is_err());
    assert!(verify_certificate(&a, &a.parse("x-1").unwrap(), &constraints, &[], &c).is_err());
    let serialized = serde_json::to_string(&c).unwrap();
    let decoded = serde_json::from_str(&serialized).unwrap();
    verify_certificate(&a, &p, &constraints, &[], &decoded).unwrap();
}

#[test]
fn empty_domain_and_zero_polynomials_do_not_generate_vacuous_pruning_proofs() {
    assert!(certify(&["x"], &["x-1", "1-x"], "x").is_none());
    assert!(certify(&["x"], &["x"], "0").is_none());
}

#[test]
fn insufficient_certificate_budget_is_inconclusive() {
    let (a, constraints) = setup(&["x"], &["x", "1-x"]);
    let p = a.parse("(x^2-x+1/3)^2").unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let options = CertificateOptions {
        bernstein_depth: 0,
        univariate_sturm_degree: 0,
        ..CertificateOptions::default()
    };
    assert!(
        certify_nonzero(&a, &p, &domain, &options)
            .unwrap()
            .is_none()
    );
    let options = CertificateOptions {
        work_limit: 1,
        ..CertificateOptions::default()
    };
    assert!(
        certify_nonzero(&a, &p, &domain, &options)
            .unwrap()
            .is_none()
    );
}

#[test]
fn exact_ldl_certifies_the_talks_mixed_sign_positive_quadratic() {
    // HP2 talk slide 25, written in the matrix order x4, x3, x2. One active
    // coordinate being nonzero suffices; the other two may range over R.
    let c = certify(
        &["x4", "x3", "x2"],
        &["x4"],
        "1849*x4^2-4080*x4*x3+496*x4*x2+2600*x3^2-248*x3*x2+104*x2^2",
    )
    .unwrap();
    assert_eq!(c.sign, 1);
    assert_eq!(
        c.method,
        CertificateMethod::QuadraticLdl {
            augmented: false,
            diagonal: vec!["1849".into(), "645800/1849".into(), "536342/80725".into()],
        }
    );
    let serialized = serde_json::to_string(&c).unwrap();
    let decoded: DomainCertificate = serde_json::from_str(&serialized).unwrap();
    assert_eq!(decoded, c);
}

#[test]
fn exact_ldl_requires_the_origin_of_active_coordinates_to_be_excluded() {
    let q = "x^2-x*y+y^2";
    assert!(certify(&["x", "y"], &[], q).is_none());
    assert!(certify(&["x", "y", "z"], &["z"], q).is_none());
    assert!(certify(&["x", "y"], &["x+1", "1-x"], q).is_none());
    // Coupled inequalities are deliberately absent from the derived box.
    assert!(certify(&["x", "y"], &["x+y"], q).is_none());
    for constraints in [["x"], ["x-2"], ["-x"], ["-x-2"]] {
        let c = certify(&["x", "y"], &constraints, q).unwrap();
        assert!(matches!(
            c.method,
            CertificateMethod::QuadraticLdl {
                augmented: false,
                ..
            }
        ));
    }
}

#[test]
fn exact_ldl_augmented_matrix_proves_global_strict_sign() {
    let c = certify(&["x", "y"], &[], "(x-y)^2+x^2+2*x+2").unwrap();
    assert_eq!(c.sign, 1);
    assert_eq!(
        c.method,
        CertificateMethod::QuadraticLdl {
            augmented: true,
            diagonal: vec!["2".into(), "1/2".into(), "1".into()],
        }
    );
    let negative = certify(&["x", "y"], &[], "-((x-y)^2+x^2+2*x+2)").unwrap();
    assert_eq!(negative.sign, -1);
    assert!(matches!(
        negative.method,
        CertificateMethod::QuadraticLdl {
            augmented: true,
            ..
        }
    ));
    // The augmented matrix is only semidefinite: the zero at (-1,-1) remains.
    assert!(certify(&["x", "y"], &[], "(x-y)^2+x^2+2*x+1").is_none());
    assert!(certify(&["x", "y"], &[], "(x-y)^2+x^2+2*x+1/2").is_none());
}

#[test]
fn exact_ldl_keeps_rational_coefficients_and_rejects_wrong_inertia() {
    let c = certify(&["x", "y"], &["-x"], "-x^2/3+x*y/5-y^2/7").unwrap();
    assert_eq!(c.sign, -1);
    assert_eq!(
        c.method,
        CertificateMethod::QuadraticLdl {
            augmented: false,
            diagonal: vec!["1/3".into(), "79/700".into()],
        }
    );
    assert!(certify(&["x", "y"], &["x"], "x^2-3*x*y+y^2").is_none());
    assert!(certify(&["x", "y"], &["x"], "x*y").is_none());
    assert!(certify(&["x", "y"], &["x"], "(x-y)^2").is_none());
}

#[test]
fn exact_ldl_checker_recomputes_pivots_orientation_and_origin_exclusion() {
    let (a, constraints) = setup(&["x", "y"], &["x"]);
    let p = a.parse("x^2-x*y+y^2").unwrap();
    let c = certify_nonzero(
        &a,
        &p,
        &derive_domain(&a, &constraints, &[]).unwrap(),
        &CertificateOptions::default(),
    )
    .unwrap()
    .unwrap();
    let mut forged = c.clone();
    forged.method = CertificateMethod::QuadraticLdl {
        augmented: false,
        diagonal: vec!["1".into(), "1".into()],
    };
    assert!(verify_certificate(&a, &p, &constraints, &[], &forged).is_err());
    let mut forged = c.clone();
    forged.sign = -1;
    assert!(verify_certificate(&a, &p, &constraints, &[], &forged).is_err());
    let mut forged = c.clone();
    if let CertificateMethod::QuadraticLdl { augmented, .. } = &mut forged.method {
        *augmented = true;
    }
    assert!(verify_certificate(&a, &p, &constraints, &[], &forged).is_err());
    let mut forged = c.clone();
    forged.domain = derive_domain(&a, &[], &[]).unwrap();
    assert!(verify_certificate(&a, &p, &[], &[], &forged).is_err());
    let tiny_budget = CertificateOptions {
        work_limit: 1,
        ..CertificateOptions::default()
    };
    assert!(
        certify_nonzero(&a, &p, &c.domain, &tiny_budget)
            .unwrap()
            .is_none()
    );
}

#[test]
fn homogeneous_anchor_checks_all_constraints_and_provides_exact_reconstruction() {
    let (a, constraints) = setup(&["s", "r", "x", "y"], &["s-4", "r", "x", "y"]);
    let mut all = constraints.clone();
    all.push(a.parse("s*x*y-r^2").unwrap());
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let certificate = homogeneous_anchor(&a, &all, &[1, 2, 3], 1, &domain)
        .unwrap()
        .unwrap();
    assert_eq!(certificate.degrees, vec![0, 1, 1, 1, 2]);
    assert_eq!(
        a.parse(certificate.normalized_polynomials.last().unwrap())
            .unwrap(),
        a.parse("s*x*y-1").unwrap()
    );
    assert_eq!(certificate.forward_ratios, vec!["x/r", "y/r"]);
    assert_eq!(certificate.inverse_coordinates, vec!["r*x", "r*y"]);
    let boundary = a.parse("s*x*y+x-1").unwrap();
    assert!(homogeneous_degree(&boundary, &[2, 2]).is_none());
    assert_eq!(
        restore_homogeneous_bound(&boundary, &[2, 3], 1).unwrap(),
        a.parse("s*x*y+r*x-r^2").unwrap()
    );
    all.push(a.parse("1-x").unwrap());
    assert!(
        homogeneous_anchor(&a, &all, &[1, 2, 3], 1, &domain)
            .unwrap()
            .is_none()
    );
}

#[test]
fn homogeneity_rejects_an_anchor_without_a_strict_positive_domain_bound() {
    let (a, constraints) = setup(&["r", "x"], &["x"]);
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    assert!(
        homogeneous_anchor(&a, &[a.parse("r+x").unwrap()], &[0, 1], 0, &domain)
            .unwrap()
            .is_none()
    );
    assert!(restore_homogeneous_bound(&a.parse("r+x").unwrap(), &[1], 0).is_err());
}
