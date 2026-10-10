//! Exact prefix evaluation with a bounded integer numerator fast path.
//!
//! For integer coefficients and rational coordinates `n_i/d_i`, multiply every
//! term by the common positive denominator `product(d_i^degree_i)`. Symbolica
//! performs integer arithmetic until one final rational normalization. The
//! ordinary Symbolica evaluator handles inputs outside the fast-path bounds.

use anyhow::{Result, ensure};
use symbolica::prelude::*;

use super::Poly;

const MAX_TERMS: usize = 4096;
const MAX_ACTIVE_AXES: usize = 16;
const MAX_AXIS_DEGREE: usize = 64;
const MAX_COEFFICIENT_BITS: u64 = 4096;
const MAX_POWER_BITS: u128 = 16384;
const MAX_SCRATCH_ESTIMATE: u128 = 1024 * 1024;
const MAX_EXPONENT_VISITS: usize = 1024 * 1024;

/// Evaluate a polynomial whose *entire support* is already assigned. A later
/// coordinate must not be ignored even if substituting an earlier zero could
/// make its coefficient vanish. Prefix coordinates use the polynomial's exact
/// variable-map order; no compaction or remapping takes place here.
pub(crate) fn evaluate(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
    ensure!(prefix.len() <= p.nvars(), "prefix exceeds variable map");
    ensure!(
        (prefix.len()..p.nvars()).all(|i| p.degree(i) == 0),
        "guard depends on a later coordinate"
    );
    Ok(integer_value(p, prefix).unwrap_or_else(|| p.replace_all(prefix)))
}

/// `evaluate` validates complete support before calling this private helper.
/// These bounds constrain temporary powers and arithmetic work, not the
/// allocation size of the input polynomial or the fallback backend operation.
fn integer_value(p: &Poly, prefix: &[Rational]) -> Option<Rational> {
    if p.is_constant() {
        return Some(p.get_constant());
    }
    if p.nterms() > MAX_TERMS
        || prefix.len().checked_mul(p.nterms())? > MAX_EXPONENT_VISITS
        || p.coefficients
            .iter()
            .any(|c| !c.is_integer() || c.numerator_ref().significant_bits() > MAX_COEFFICIENT_BITS)
    {
        return None;
    }

    let mut active = Vec::new();
    for (i, value) in prefix.iter().enumerate() {
        let degree = p.degree(i) as usize;
        if degree == 0 {
            continue;
        }
        if degree > MAX_AXIS_DEGREE
            || active.len() == MAX_ACTIVE_AXES
            || value.denominator_ref().is_zero()
            || value.denominator_ref().is_negative()
        {
            return None;
        }
        active.push((i, degree));
    }

    // u128 accounting prevents overflow even for a very large input integer.
    // At most 16 degrees, each <=64, enter these sums. Include all three power
    // arrays in the estimate, although numerator/denominator powers for earlier
    // axes have been dropped by the time a later axis is constructed.
    let mut power_bits = 0u128;
    let mut storage = 0u128;
    for &(i, degree) in &active {
        let bits = u128::from(prefix[i].numerator_ref().significant_bits())
            + u128::from(prefix[i].denominator_ref().significant_bits())
            + 2;
        let axis_bits = bits * degree as u128;
        power_bits += axis_bits;
        storage += 3 * (degree + 1) as u128 * (64 + axis_bits.div_ceil(8));
    }
    if power_bits > MAX_POWER_BITS || storage > MAX_SCRATCH_ESTIMATE {
        return None;
    }

    let mut denominator = Integer::one();
    let mut powers = Vec::with_capacity(active.len());
    for &(i, degree) in &active {
        let mut numerators = vec![Integer::one(); degree + 1];
        let mut denominators = vec![Integer::one(); degree + 1];
        for exponent in 1..=degree {
            numerators[exponent] = Z.mul(&numerators[exponent - 1], prefix[i].numerator_ref());
            denominators[exponent] =
                Z.mul(&denominators[exponent - 1], prefix[i].denominator_ref());
        }
        Z.mul_assign(&mut denominator, &denominators[degree]);
        powers.push(
            (0..=degree)
                .map(|exponent| Z.mul(&numerators[exponent], &denominators[degree - exponent]))
                .collect::<Vec<_>>(),
        );
    }

    let mut numerator = Integer::zero();
    for term in p {
        let mut value = term.coefficient.numerator_ref().clone();
        for (axis, &(i, _)) in active.iter().enumerate() {
            let power = &powers[axis][term.exponents[i] as usize];
            if !power.is_one() {
                Z.mul_assign(&mut value, power);
            }
            if value.is_zero() {
                break;
            }
        }
        Z.add_assign(&mut numerator, &value);
    }
    Some(Rational::from((numerator, denominator)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{Algebra, rational};

    #[test]
    fn exact_values_preserve_negative_zero_and_rational_coordinates() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into(), "later".into()]).unwrap();
        for expression in ["x^64+y^2", "x^2+2*x*y+y^2", "3*x^4*y^3-7*x*y+9", "-5*x*y-2"] {
            let p = a.parse(expression).unwrap();
            for point in [
                ["0", "3", "-2/3"],
                ["-3/2", "0", "3/2"],
                ["999999999999999999999/17", "9", "-1234567/19"],
            ] {
                let point = point.map(|s| rational(s).unwrap());
                let expected = p.replace_all(&point);
                assert_eq!(integer_value(&p, &point), Some(expected.clone()));
                assert_eq!(evaluate(&p, &point).unwrap(), expected);
            }
        }
    }

    #[test]
    fn empty_prefix_constants_and_zero_dimensional_maps_are_exact() {
        for variables in [vec![], vec!["x".into(), "later".into()]] {
            let a = Algebra::new(&variables).unwrap();
            for expression in ["0", "7", "-5/9"] {
                let p = a.parse(expression).unwrap();
                assert_eq!(evaluate(&p, &[]).unwrap(), rational(expression).unwrap());
            }
        }
    }

    #[test]
    fn support_is_checked_before_zero_specialization_or_fallback() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let zero = [Rational::zero()];
        for expression in ["x*y", "x/7+y/3", "y", "x^65*y"] {
            assert!(evaluate(&a.parse(expression).unwrap(), &zero).is_err());
        }
        assert!(evaluate(&a.parse("x").unwrap(), &[]).is_err());
        assert!(evaluate(&a.parse("0").unwrap(), &vec![Rational::zero(); 3]).is_err());
        assert!(evaluate(&Algebra::new(&[]).unwrap().parse("1").unwrap(), &zero).is_err());
    }

    #[test]
    fn rational_degree_height_and_term_limits_fall_back_exactly() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let point = [rational("-2/3").unwrap(), rational("1/5").unwrap()];
        for expression in ["x/7+1/3", "x^65-2*x"] {
            let p = a.parse(expression).unwrap();
            assert!(integer_value(&p, &point).is_none());
            assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
        }
        let huge_coefficient = Integer::one() << MAX_COEFFICIENT_BITS as usize;
        let p = a.parse(&format!("{huge_coefficient}*x+1")).unwrap();
        assert!(integer_value(&p, &point).is_none());
        assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));

        let huge_point = [Rational::from((
            Integer::one() << 17000usize,
            Integer::from(3),
        ))];
        let p = a.parse("x^2+1").unwrap();
        assert!(integer_value(&p, &huge_point).is_none());
        assert_eq!(
            evaluate(&p, &huge_point).unwrap(),
            p.replace_all(&huge_point)
        );

        let expression = (0..64)
            .flat_map(|x| (0..65).map(move |y| format!("x^{x}*y^{y}")))
            .collect::<Vec<_>>()
            .join("+");
        let p = a.parse(&expression).unwrap();
        assert_eq!(p.nterms(), 4160);
        assert!(integer_value(&p, &point).is_none());
        assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
    }

    #[test]
    fn active_axis_and_work_caps_preserve_full_variable_order() {
        let names = (0..17).map(|i| format!("x{i}")).collect::<Vec<_>>();
        let a = Algebra::new(&names).unwrap();
        let p = a.parse(&names.join("+")).unwrap();
        let point = (0..17)
            .map(|i| Rational::from((i - 8, 7)))
            .collect::<Vec<_>>();
        assert!(integer_value(&p, &point).is_none());
        assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));

        let names = (0..1025).map(|i| format!("v{i}")).collect::<Vec<_>>();
        let a = Algebra::new(&names).unwrap();
        let expression = (0..32)
            .flat_map(|x| (0..32).map(move |y| format!("v0^{x}*v1024^{y}")))
            .collect::<Vec<_>>()
            .join("+");
        let p = a.parse(&expression).unwrap();
        let point = vec![Rational::from(-1); names.len()];
        assert!(integer_value(&p, &point).is_none());
        assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
    }

    #[test]
    fn deterministic_integer_polynomial_family_matches_symbolica() {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
        for seed in 0..100 {
            let expression = (0..12)
                .map(|k| {
                    format!(
                        "({})*x^{}*y^{}*z^{}",
                        (seed * 17 + k * 13) % 43 - 21,
                        (seed + k) % 5,
                        (seed + 2 * k) % 4,
                        (2 * seed + k) % 3,
                    )
                })
                .collect::<Vec<_>>()
                .join("+");
            let p = a.parse(&expression).unwrap();
            let point = [
                Rational::from((seed - 50, 7)),
                Rational::from((-seed, 11)),
                Rational::from((seed + 1, 13)),
            ];
            assert_eq!(integer_value(&p, &point), Some(p.replace_all(&point)));
            assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
        }
    }
}
