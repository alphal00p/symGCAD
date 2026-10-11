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
        (1_000_000_001, 4096),
        (1_000_000, 262_145),
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
