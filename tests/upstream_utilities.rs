//! Explicit equivalents of the eleven upstream non-GCAD utility tests.
//! Python/Cython object conversion is replaced by exact Rust/Symbolica input
//! and serialization boundaries; no Python runtime or FLINT is used here.
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, rational},
    roots::simplest_between,
};

fn q(text: &str) -> Rational {
    rational(text).unwrap()
}

fn sample(first: &str, second: &str, expected: &str) {
    let first = q(first);
    let second = q(second);
    let expected = q(expected);
    assert_eq!(simplest_between(&first, &second), expected);
    assert_eq!(simplest_between(&second, &first), expected);
}

#[test]
fn test_shortest_fraction_zero() {
    sample("-1/3", "2/7", "0");
}

#[test]
fn test_shortest_fraction_int() {
    sample("5/3", "21/7", "2");
}

#[test]
fn test_shortest_fraction_negint() {
    sample("-50/3", "-8/7", "-2");
}

#[test]
fn test_shortest_fraction_close() {
    sample("31/107", "32/107", "5/17");
}

#[test]
fn test_shortest_fraction_closer() {
    // Upstream's reversed-endpoint assertion expects 9/332, which is outside
    // the interval: 9*7121 - 193*332 = 13 > 0. Preserve an actual sample.
    sample("193/7121", "191/7109", "1/37");
    assert!(q("9/332") > q("193/7121"));
    assert!(q("9/332") > q("191/7109"));
    assert!(q("191/7109") < q("1/37"));
    assert!(q("1/37") < q("193/7121"));
}

#[test]
fn test_shortest_fraction_bigint() {
    for exponent in (0..150).step_by(3) {
        let first = q(&format!("2^{exponent}"));
        let second = &first + &Rational::one();
        assert_eq!(simplest_between(&first, &second), first);
        assert_eq!(simplest_between(&second, &first), first);
        assert_eq!(simplest_between(&first, &first), first);
    }
}

fn roundtrip(text: &str) -> Rational {
    let value = q(text);
    assert_eq!(rational(&value.to_string()).unwrap(), value);
    value
}

#[test]
fn test_identity_int() {
    assert_eq!(roundtrip("3"), Rational::from(3));
    assert_eq!(roundtrip("-7"), Rational::from(-7));
    assert_eq!(roundtrip("-2^90"), q("-1237940039285380274899124224"));
    assert!(rational("zzz").is_err());
}

#[test]
fn test_identity_fraction() {
    assert_eq!(roundtrip("3/4") * Rational::from(4), Rational::from(3));
    assert_eq!(roundtrip("-9/1"), Rational::from(-9));
    assert_eq!(roundtrip("2^150/3") * Rational::from(3), q("2^150"));
    assert_eq!(roundtrip("7/2^160") * q("2^160"), Rational::from(7));
}

#[test]
fn test_identity_fraction_conversions() {
    assert_eq!(roundtrip("17"), Rational::from(17));
    assert_eq!(roundtrip("23/71") * Rational::from(71), Rational::from(23));
}

#[test]
fn test_identity_univariate_poly() {
    let algebra = Algebra::new(&["x".into()]).unwrap();
    let polynomial = algebra.parse("x**2-2*x+7").unwrap();
    assert_eq!(
        algebra.parse(&algebra.format(&polynomial)).unwrap(),
        polynomial
    );
    let univariate = algebra.specialize_univariate(&polynomial, 0, &[]).unwrap();
    assert_eq!(univariate.coefficients(), &[q("7"), q("-2"), q("1")]);
}

#[test]
fn test_identity_multivariate_poly() {
    for names in [["x", "y"], ["y", "x"]] {
        let algebra = Algebra::new(&names.map(str::to_owned)).unwrap();
        let polynomial = algebra.parse("x*y**2-2*x+4*y-7").unwrap();
        assert_eq!(
            algebra.parse(&algebra.format(&polynomial)).unwrap(),
            polynomial
        );
        let mut terms = polynomial
            .exponents_iter()
            .zip(&polynomial.coefficients)
            .map(|(powers, coefficient)| {
                let x = powers[names.iter().position(|&name| name == "x").unwrap()];
                let y = powers[names.iter().position(|&name| name == "y").unwrap()];
                (x, y, coefficient.to_string())
            })
            .collect::<Vec<_>>();
        terms.sort();
        assert_eq!(
            terms,
            vec![
                (0, 0, "-7".into()),
                (0, 1, "4".into()),
                (1, 0, "-2".into()),
                (1, 2, "1".into()),
            ]
        );
    }
}
