use symbolica::prelude::*;
fn main() {}

// Keep the pre-optimization fused recurrence as an independent comparator for
// both the unit path and the unchanged non-unit path.
fn unit_shift_fused_reference<F: Ring>(
    p: &UnivariatePolynomial<F>,
    shift: &F::Element,
) -> UnivariatePolynomial<F> {
    let mut coefficients = p.coefficients().to_vec();
    for k in 0..p.degree() {
        for j in (k..p.degree()).rev() {
            let (left, right) = coefficients.split_at_mut(j + 1);
            p.coefficient_ring().add_mul_assign(&mut left[j], &right[0], shift);
        }
    }
    UnivariatePolynomial::from_coefficients(p.coefficient_ring(), coefficients, p.get_vars_ref().clone())
}

#[test]
fn unit_shift_integer_large_signed_zero_and_nonunit_equivalence() {
    use symbolica::domains::integer::Integer;
    for expression in [
        "x",
        "-7",
        "0",
        "x^3*(x-1)^4",
        "(2^700+3)*x^17-2^701*x^3+5*x-11",
        "(x-1)^32*(2^130*x+1)",
    ] {
        let seed = parse!("x")
            .to_polynomial::<_, u16>(&Z, None)
            .to_univariate_from_univariate(0);
        let p = if expression == "0" {
            seed.zero()
        } else if expression == "-7" {
            seed.constant(Integer::from(-7))
        } else {
            parse!(expression)
                .to_polynomial::<_, u16>(&Z, None)
                .to_univariate_from_univariate(0)
        };
        for value in [-3, -1, 0, 1, 2] {
            let shift = Integer::from(value);
            let actual = p.shift_var(&shift);
            assert_eq!(actual, unit_shift_fused_reference(&p, &shift));
            for point in [-2, 0, 3] {
                let point = Integer::from(point);
                assert_eq!(actual.evaluate(&point), p.evaluate(&(&point + &shift)));
            }
        }
    }
}

#[test]
fn unit_shift_rational_content_and_fractional_fallback() {
    use symbolica::domains::rational::Rational;
    let p = parse!("-3/7*x^4+2/13*x^2-1/5*x+2^700/19")
        .to_polynomial::<_, u16>(&Q, None)
        .to_univariate_from_univariate(0);
    for p in [&p, &p.zero(), &p.one()] {
        for shift in [
            Rational::from(-1),
            Rational::zero(),
            Rational::one(),
            (1, 3).into(),
            (-7, 2).into(),
        ] {
            let actual = p.shift_var(&shift);
            assert_eq!(actual, unit_shift_fused_reference(p, &shift));
            for point in [Rational::from(-2), (1, 5).into(), Rational::from(3)] {
                assert_eq!(actual.evaluate(&point), p.evaluate(&(&point + &shift)));
            }
        }
    }
}
