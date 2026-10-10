use super::*;

#[test]
fn independent_opt_in_discriminant_replay_preserves_the_signed_polynomial() {
    let algebra = Algebra::new(&["s".into(), "x".into()]).unwrap();
    let p = algebra.parse("x^10+(s+1)^120").unwrap();
    let expected = algebra.parse("-10^10*(s+1)^1080").unwrap();
    let (_, statistics) = crate::projection::discriminant_with_binary_statistics(&p, 1).unwrap();
    assert!(statistics.is_some());
    let mut independent = IndependentOperations {
        binary_resultants: true,
    };
    let actual = independent.discriminant(&p, 1).unwrap();
    assert_eq!(actual, expected);
    assert_ne!(actual, canonical(&expected));
}

struct Cached<'a>(ProjectionCache<'a>);

impl ProjectionOperations for Cached<'_> {
    fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly> {
        self.0.discriminant(p, variable)
    }

    fn resultant(&mut self, _: &Poly, _: &Poly, _: usize) -> Result<Poly> {
        panic!("quadratic classification must not request a pair resultant")
    }
}

#[test]
fn signed_quadratic_values_do_not_inherit_the_canonical_guard_sign() {
    let algebra = Algebra::new(&["x".into(), "z".into()]).unwrap();
    let mut operations = Cached(ProjectionCache::new(
        &algebra,
        &OptimizedProjectionOptions::default(),
    ));
    for text in ["z^2-2*z+x", "-3/7*(z^2-2*z+x)"] {
        let p = algebra.parse(text).unwrap();
        let exact = discriminant(&p, 1).unwrap();
        let a = p.univariate_lcoeff(1);
        let b = p.derivative(1).replace(1, &Rational::zero());
        let c = p.replace(1, &Rational::zero());
        for x in [Rational::from((1, 2)), Rational::one(), Rational::from(2)] {
            let prefix = [x];
            let (guard, value) = quadratic_discriminant_guard(
                &p,
                1,
                &prefix_value(&a, &prefix).unwrap(),
                &prefix_value(&b, &prefix).unwrap(),
                &prefix_value(&c, &prefix).unwrap(),
                &mut operations,
            )
            .unwrap();
            assert_eq!(value, prefix_value(&exact, &prefix).unwrap());
            assert_eq!(guard, canonical(&exact));
            // The cache guard is x-1, opposite in sign to the actual D.
            assert_eq!(sign(&value), -sign(&prefix_value(&guard, &prefix).unwrap()));
        }
    }
    assert_eq!(operations.0.report.cache.discriminant_misses, 2);
    assert_eq!(operations.0.report.cache.discriminant_hits, 4);
}

#[test]
fn quadratic_guard_matches_independent_replay_with_rational_coefficients() {
    let algebra = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
    let p = algebra
        .parse("(x^2+1)/3*z^2-(x*y+2)/7*z+(y^2+3)/11")
        .unwrap();
    let prefix = [Rational::from((-2, 3)), Rational::from((5, 4))];
    let av = prefix_value(&p.univariate_lcoeff(2), &prefix).unwrap();
    let bv = prefix_value(&p.derivative(2).replace(2, &Rational::zero()), &prefix).unwrap();
    let cv = prefix_value(&p.replace(2, &Rational::zero()), &prefix).unwrap();
    let mut cached = Cached(ProjectionCache::new(
        &algebra,
        &OptimizedProjectionOptions::default(),
    ));
    let mut independent = IndependentOperations::default();
    let (guard, value) = quadratic_discriminant_guard(&p, 2, &av, &bv, &cv, &mut cached).unwrap();
    let (checked_guard, checked_value) =
        quadratic_discriminant_guard(&p, 2, &av, &bv, &cv, &mut independent).unwrap();
    assert_eq!(value, checked_value);
    assert_eq!(guard, canonical(&checked_guard));
    assert_eq!(value, prefix_value(&checked_guard, &prefix).unwrap());
}

#[test]
fn identically_zero_quadratic_discriminants_are_rejected() {
    let algebra = Algebra::new(&["x".into(), "z".into()]).unwrap();
    let p = algebra.parse("(z-x)^2").unwrap();
    let mut operations = IndependentOperations::default();
    assert!(
        quadratic_discriminant_guard(
            &p,
            1,
            &Rational::one(),
            &Rational::from(-4),
            &Rational::from(4),
            &mut operations,
        )
        .unwrap_err()
        .to_string()
        .contains("square-free")
    );
}
