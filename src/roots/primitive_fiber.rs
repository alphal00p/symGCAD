//! Bound the one-time cost of clearing a retained root fiber's denominators.
//! This only changes its internal positive scale, never its root intervals.
use symbolica::prelude::*;

const MAX_COEFFICIENTS: usize = 257;
const MAX_COEFFICIENT_BITS: u128 = 65_536;
const MAX_TOTAL_BITS: u128 = 8 * 1024 * 1024;

pub(super) fn prepare(poly: UnivariatePolynomial<Q>) -> UnivariatePolynomial<Q> {
    let coefficients = poly.coefficients();
    if coefficients.len() < 3 || coefficients.len() > MAX_COEFFICIENTS {
        return poly;
    }
    let mut denominator_bits = 0u128;
    let mut numerator_bits = 0u128;
    let mut rational = false;
    for coefficient in coefficients {
        numerator_bits = numerator_bits.max(coefficient.numerator_ref().significant_bits().into());
        if !coefficient.is_integer() {
            rational = true;
            // The product of the positive denominators bounds their LCM.
            denominator_bits += u128::from(coefficient.denominator_ref().significant_bits());
        }
    }
    let cleared_bits = numerator_bits + denominator_bits;
    if !rational
        || cleared_bits > MAX_COEFFICIENT_BITS
        || cleared_bits * coefficients.len() as u128 > MAX_TOTAL_BITS
    {
        return poly;
    }
    let positive_content = poly.content().abs();
    let primitive = poly.div_coeff(&positive_content);
    debug_assert!(primitive.coefficients().iter().all(|c| c.is_integer()));
    primitive
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{Algebra, rational};

    fn p(expression: &str) -> UnivariatePolynomial<Q> {
        Algebra::new(&["x".into()])
            .unwrap()
            .parse(expression)
            .unwrap()
            .to_univariate_from_univariate(0)
    }

    #[test]
    fn positive_scaling_preserves_mixed_denominator_signs_and_zeros() {
        for expression in [
            "-7/13*(x^2-2)",
            "x^3/3-x^2/5-2*x/7+4/11",
            "(x-1/7)^3*(x+2/3)^2",
            "(x^2+1)^2*(x^2-2)/17",
        ] {
            let original = p(expression);
            let prepared = prepare(original.clone());
            assert!(prepared.coefficients().iter().all(|c| c.is_integer()));
            let scale = original.lcoeff() / prepared.lcoeff();
            assert!(scale > Rational::zero());
            assert_eq!(prepared.clone().mul_coeff(&scale), original);
            for x in ["-3/2", "-2/3", "0", "1/7", "7/5", "2"] {
                let x = rational(x).unwrap();
                assert_eq!(prepared.evaluate(&x) * &scale, original.evaluate(&x));
            }
        }
    }

    #[test]
    fn bounded_fallbacks_preserve_original_fibers() {
        for expression in ["0", "-7/13", "x/5+1/7", "-6*x^2+12", "x^257/3+1"] {
            let original = p(expression);
            assert_eq!(prepare(original.clone()), original);
        }
        let denominator = Integer::from(2).pow(65_537);
        let original = p(&format!("x^2/{denominator}+x/3+1"));
        assert_eq!(prepare(original.clone()), original);
        // Each candidate coefficient is below the height cap, but retaining
        // all 257 cleared coefficients would exceed the total-bit cap.
        let denominator = Integer::from(2).pow(33_000);
        let original = p(&format!("x^256/{denominator}+x/3+1"));
        assert_eq!(prepare(original.clone()), original);
    }
}
