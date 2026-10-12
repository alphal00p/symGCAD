use symgcad::{
    algebra::Algebra,
    domain::{
        BernsteinProof, CertificateMethod, CertificateOptions, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

#[test]
fn explicit_large_certificate_budgets_replay_and_remain_bounded() {
    // A sparse positive polynomial with 4^9 tensor coefficients. Its two
    // coefficients have opposite signs, so the shifted-coefficient shortcut
    // cannot replace the actual larger Bernstein construction.
    let names = (0..9).map(|i| format!("x{i}")).collect::<Vec<_>>();
    let algebra = Algebra::new(&names).unwrap();
    let constraints = names
        .iter()
        .flat_map(|n| {
            [
                algebra.parse(n).unwrap(),
                algebra.parse(&format!("1-{n}")).unwrap(),
            ]
        })
        .collect::<Vec<_>>();
    let polynomial = algebra
        .parse(&format!("2-({})^3", names.join("*")))
        .unwrap();
    let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
    let options = CertificateOptions {
        bernstein_coefficients: 262_144,
        work_limit: 1_000_000_000,
        bernstein_depth: 0,
        bernstein_nodes: 1,
        square_residual_candidates: 0,
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    };
    let mut too_small = options.clone();
    too_small.bernstein_coefficients = 65_536;
    assert!(
        certify_nonzero(&algebra, &polynomial, &domain, &too_small)
            .unwrap()
            .is_none()
    );
    too_small = options.clone();
    too_small.work_limit = 1;
    assert!(
        certify_nonzero(&algebra, &polynomial, &domain, &too_small)
            .unwrap()
            .is_none()
    );

    let certificate = certify_nonzero(&algebra, &polynomial, &domain, &options)
        .unwrap()
        .unwrap();
    assert_eq!(certificate.sign, 1);
    assert_eq!(certificate.limits, options);
    assert!(matches!(
        certificate.method,
        CertificateMethod::Bernstein {
            proof: BernsteinProof::Leaf
        }
    ));
    verify_certificate(&algebra, &polynomial, &constraints, &[], &certificate).unwrap();
    let bytes = serde_json::to_vec(&certificate).unwrap();
    let restored = serde_json::from_slice(&bytes).unwrap();
    verify_certificate(&algebra, &polynomial, &constraints, &[], &restored).unwrap();

    let mut changed = certificate.clone();
    changed.limits.bernstein_coefficients = 65_536;
    assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
    changed = certificate;
    changed.limits.work_limit = 1;
    assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
}

#[test]
fn certificate_budget_defaults_and_validation_stay_explicit() {
    let defaults = CertificateOptions::default();
    assert_eq!(defaults.work_limit, 1_000_000);
    assert_eq!(defaults.bernstein_coefficients, 4096);
    assert_eq!(
        (defaults.bernstein_depth, defaults.bernstein_nodes),
        (4, 63)
    );
    let algebra = Algebra::new(&["x".into()]).unwrap();
    let polynomial = algebra.parse("1+x").unwrap();
    let constraints = [algebra.parse("x").unwrap(), algebra.parse("1-x").unwrap()];
    let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
    let certificate = certify_nonzero(&algebra, &polynomial, &domain, &defaults)
        .unwrap()
        .unwrap();
    for (work, coefficients) in [
        (0, 4096),
        (1_000_000, 0),
        (1_000_000, 2_097_153),
        (usize::MAX, usize::MAX),
    ] {
        let options = CertificateOptions {
            work_limit: work,
            bernstein_coefficients: coefficients,
            ..defaults.clone()
        };
        assert!(certify_nonzero(&algebra, &polynomial, &domain, &options).is_err());
        let mut changed = certificate.clone();
        changed.limits = options;
        assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
    }
}

#[test]
fn tensor_above_previous_ceiling_has_exact_positive_replay() {
    // Exactly 513^2 = 263169 Bernstein coefficients. The polynomial is at
    // least one on the closed unit square; its negative coefficient rules
    // out the earlier coefficient-sign shortcut.
    let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
    let polynomial = algebra.parse("2-(x*y)^512").unwrap();
    let constraints = ["x", "1-x", "y", "1-y"].map(|p| algebra.parse(p).unwrap());
    let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
    let options = CertificateOptions {
        bernstein_coefficients: 1_048_576,
        work_limit: 1_000_000_000,
        bernstein_depth: 0,
        bernstein_nodes: 1,
        square_residual_candidates: 0,
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    };
    let mut previous_limit = options.clone();
    previous_limit.bernstein_coefficients = 262_144;
    assert!(
        certify_nonzero(&algebra, &polynomial, &domain, &previous_limit)
            .unwrap()
            .is_none()
    );
    let certificate = certify_nonzero(&algebra, &polynomial, &domain, &options)
        .unwrap()
        .unwrap();
    assert_eq!(certificate.sign, 1);
    assert_eq!(certificate.limits, options);
    assert!(matches!(
        certificate.method,
        CertificateMethod::Bernstein {
            proof: BernsteinProof::Leaf
        }
    ));
    verify_certificate(&algebra, &polynomial, &constraints, &[], &certificate).unwrap();
    let bytes = serde_json::to_vec(&certificate).unwrap();
    let restored = serde_json::from_slice(&bytes).unwrap();
    verify_certificate(&algebra, &polynomial, &constraints, &[], &restored).unwrap();
    let mut changed = certificate.clone();
    changed.limits = previous_limit;
    assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
    changed = certificate;
    changed.limits.work_limit = 1;
    assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
}

#[test]
#[cfg(target_pointer_width = "64")]
fn work_allowances_above_previous_ceiling_validate_and_replay() {
    let algebra = Algebra::new(&["x".into()]).unwrap();
    let polynomial = algebra.parse("1+x").unwrap();
    let constraints = [algebra.parse("x").unwrap(), algebra.parse("1-x").unwrap()];
    let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
    for work_limit in [
        1_000_000_001,
        16_000_000_000,
        16_000_000_001,
        48_000_000_000,
        64_000_000_000,
    ] {
        let options = CertificateOptions {
            work_limit,
            ..Default::default()
        };
        let certificate = certify_nonzero(&algebra, &polynomial, &domain, &options)
            .unwrap()
            .unwrap();
        verify_certificate(&algebra, &polynomial, &constraints, &[], &certificate).unwrap();
        let mut changed = certificate;
        changed.limits.work_limit = 64_000_000_001;
        assert!(certify_nonzero(&algebra, &polynomial, &domain, &changed.limits).is_err());
        assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
    }
}

#[test]
fn eight_axis_tensor_above_one_million_replays_at_explicit_allowance() {
    // Actual requested degree grid: (6*5*3*3*17*27*3*3)=1,115,370.
    // This sparse control is at least one on the closed unit cube. Opposite
    // coefficient signs require the native Bernstein path rather than the
    // coefficient-sign shortcut; no physical target is asserted positive here.
    let names = ["u", "v", "q", "r", "s", "t", "z", "w"].map(str::to_owned);
    let algebra = Algebra::new(&names).unwrap();
    let polynomial = algebra
        .parse("2-u^5*v^4*q^2*r^2*s^16*t^26*z^2*w^2")
        .unwrap();
    let constraints = names
        .iter()
        .flat_map(|name| {
            [
                algebra.parse(name).unwrap(),
                algebra.parse(&format!("1-{name}")).unwrap(),
            ]
        })
        .collect::<Vec<_>>();
    let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
    let options = CertificateOptions {
        bernstein_coefficients: 1_115_370,
        work_limit: 1_000_000_000,
        bernstein_depth: 0,
        bernstein_nodes: 1,
        square_residual_candidates: 0,
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    };
    for coefficients in [1_048_576, 1_115_369] {
        let mut insufficient = options.clone();
        insufficient.bernstein_coefficients = coefficients;
        assert!(
            certify_nonzero(&algebra, &polynomial, &domain, &insufficient)
                .unwrap()
                .is_none()
        );
    }
    let certificate = certify_nonzero(&algebra, &polynomial, &domain, &options)
        .unwrap()
        .unwrap();
    assert_eq!(certificate.sign, 1);
    assert_eq!(certificate.limits, options);
    assert!(matches!(
        certificate.method,
        CertificateMethod::Bernstein {
            proof: BernsteinProof::Leaf
        }
    ));
    verify_certificate(&algebra, &polynomial, &constraints, &[], &certificate).unwrap();
    let encoded = serde_json::to_vec(&certificate).unwrap();
    let decoded = serde_json::from_slice(&encoded).unwrap();
    verify_certificate(&algebra, &polynomial, &constraints, &[], &decoded).unwrap();
    let mut changed = certificate.clone();
    changed.limits.bernstein_coefficients = 1_115_369;
    assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
    changed = certificate;
    changed.limits.work_limit = 1;
    assert!(verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err());
}

#[test]
fn new_tensor_ceiling_is_finite_and_saved_options_are_checked() {
    let algebra = Algebra::new(&["x".into()]).unwrap();
    let polynomial = algebra.parse("1+x").unwrap();
    let constraints = ["x", "1-x"].map(|s| algebra.parse(s).unwrap());
    let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
    for coefficients in [1_048_577, 2_097_151, 2_097_152] {
        let options = CertificateOptions {
            bernstein_coefficients: coefficients,
            ..Default::default()
        };
        let certificate = certify_nonzero(&algebra, &polynomial, &domain, &options)
            .unwrap()
            .unwrap();
        assert_eq!(certificate.limits, options);
        verify_certificate(&algebra, &polynomial, &constraints, &[], &certificate).unwrap();
        for rejected in [0, 2_097_153, usize::MAX] {
            let mut changed = certificate.clone();
            changed.limits.bernstein_coefficients = rejected;
            assert!(certify_nonzero(&algebra, &polynomial, &domain, &changed.limits).is_err());
            assert!(
                verify_certificate(&algebra, &polynomial, &constraints, &[], &changed).is_err()
            );
        }
    }
}
