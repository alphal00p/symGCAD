use symgcad::algebra::{Algebra, Poly};
use symgcad::domain::{
    BernsteinProof, CertificateMethod, CertificateOptions, DomainCertificate, certify_nonzero,
    derive_domain, homogeneous_anchor, homogeneous_degree, restore_homogeneous_bound,
    verify_certificate,
};

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
    let c = certify(&["x"], &["x", "1-x"], "x^2-x+1/3").unwrap();
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
    let p = a.parse("x^2-x+1/3").unwrap();
    let domain = derive_domain(&a, &constraints, &[]).unwrap();
    let options = CertificateOptions {
        bernstein_depth: 0,
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
