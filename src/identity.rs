//! Square witnesses `p = m * s^2` with exact replay.
//!
//! Discovery only combines an exact factorization supplied by Symbolica: a
//! nonzero polynomial is a rational multiple of a polynomial square exactly
//! when every nonconstant factor has even multiplicity. The factorization is a
//! proposal only. Every consumer replays the identity by exact expansion.
//!
//! Two callers share the combination step:
//! * [`nonnegative_square_witness`] (linear_negative) uses the common checked
//!   irreducible factorization path and admits `m >= 0`, including `p = 0`.
//! * [`positive_square_witness`] (bilinear_general) needs only Symbolica's
//!   square-free decomposition, requires `m > 0` and normalizes the square.
use crate::algebra::{Poly, factor_polynomial};
use anyhow::{Result, ensure};
use symbolica::{poly::factor::Factorize, prelude::*};

/// Combine an exact factorization of nonzero `p` into `p = multiplier *
/// square^2`, or `None` when some nonconstant factor has odd multiplicity.
/// The result is replayed exactly; the factorization is never trusted.
fn square_from_factors(
    p: &Poly,
    factors: impl IntoIterator<Item = (Poly, usize)>,
) -> Option<(Rational, Poly)> {
    let mut square = p.one();
    for (factor, multiplicity) in factors {
        if factor.is_constant() {
            continue;
        }
        if multiplicity % 2 != 0 {
            return None;
        }
        square = &square * &factor.pow(multiplicity / 2);
    }
    let quotient = p.try_div(&square.pow(2))?;
    if !quotient.is_constant() {
        return None;
    }
    let multiplier = quotient.get_constant();
    (*p == &square.pow(2) * &p.constant(multiplier.clone())).then_some((multiplier, square))
}

/// Bounded proposal of `p = multiplier * square^2` with `multiplier >= 0`.
/// `p = 0` gives `(0, 1)`. `Ok(None)` means that no witness exists or the
/// input exceeds the cap; it is never evidence of a sign.
pub fn nonnegative_square_witness(p: &Poly) -> Result<Option<(Rational, Poly)>> {
    if p.is_zero() {
        return Ok(Some((Rational::zero(), p.one())));
    }
    if p.nterms() > 4096 || (0..p.nvars()).any(|i| p.degree(i) > 4096) {
        return Ok(None);
    }
    let Some((multiplier, square)) = square_from_factors(p, factor_polynomial(p, true)?) else {
        return Ok(None);
    };
    if multiplier < Rational::zero() {
        return Ok(None);
    }
    check_square_identity(p, &multiplier, &square, false)?;
    Ok(Some((multiplier, square)))
}

/// Bounded proposal of `p = multiplier * square^2` with `multiplier > 0`.
/// `None` means that no such witness exists or the input exceeds the cap; it
/// is never evidence of a sign.
pub fn positive_square_witness(p: &Poly) -> Option<(Rational, Poly)> {
    if p.is_zero() || p.nterms() > 1 << 16 || (0..p.nvars()).any(|i| p.degree(i) > 4096) {
        return None;
    }
    let (multiplier, square) = square_from_factors(p, p.square_free_factorization())?;
    if multiplier <= Rational::zero() {
        return None;
    }
    // Deterministic witness normalization: a primitive square whose leading
    // coefficient is positive. The multiplier absorbs the positive content.
    let content = square.content().abs();
    let mut square = square.div_coeff(&content);
    if square.lcoeff() < Rational::zero() {
        square = -square;
    }
    let multiplier = multiplier * &content * &content;
    check_positive_square_identity(p, &multiplier, &square).ok()?;
    Some((multiplier, square))
}

/// Exact replay of `lhs == multiplier * square^2`, with `multiplier > 0` when
/// `strict` and `multiplier >= 0` otherwise.
pub fn check_square_identity(
    lhs: &Poly,
    multiplier: &Rational,
    square: &Poly,
    strict: bool,
) -> Result<()> {
    ensure!(
        *multiplier > Rational::zero() || (!strict && multiplier.is_zero()),
        "square witness multiplier has the wrong sign"
    );
    ensure!(!square.is_zero(), "square witness must be nonzero");
    ensure!(
        *lhs == &square.pow(2) * &lhs.constant(multiplier.clone()),
        "square identity failed"
    );
    Ok(())
}

/// Exact replay of `lhs == multiplier * square^2` with `multiplier > 0`.
pub fn check_positive_square_identity(
    lhs: &Poly,
    multiplier: &Rational,
    square: &Poly,
) -> Result<()> {
    check_square_identity(lhs, multiplier, square, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::Algebra;

    #[test]
    fn square_free_witnesses_are_exact_and_positive() {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = algebra.parse("10*(x-2*y)^2*(x+y+1)^4").unwrap();
        let (m, s) = positive_square_witness(&p).unwrap();
        assert!(m > Rational::zero());
        check_positive_square_identity(&p, &m, &s).unwrap();
        for text in ["-(x-y)^2", "x*(x-y)^2", "x^2+y^2", "0"] {
            assert!(positive_square_witness(&algebra.parse(text).unwrap()).is_none());
        }
        let constant = algebra.parse("4").unwrap();
        let (m, s) = positive_square_witness(&constant).unwrap();
        assert_eq!(m, Rational::from(4));
        assert!(s.is_one());
        assert!(check_positive_square_identity(&p, &Rational::zero(), &s).is_err());
        assert!(check_positive_square_identity(&p, &-m, &s).is_err());
    }

    #[test]
    fn nonnegative_witnesses_keep_the_linear_negative_contract() {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let zero = algebra.parse("0").unwrap();
        let (m, s) = nonnegative_square_witness(&zero).unwrap().unwrap();
        assert!(m.is_zero() && s.is_one());
        check_square_identity(&zero, &m, &s, false).unwrap();
        assert!(check_square_identity(&zero, &m, &s, true).is_err());
        let p = algebra.parse("3/7*(x-2*y)^2*(x+y+1)^4").unwrap();
        let (m, s) = nonnegative_square_witness(&p).unwrap().unwrap();
        assert!(m > Rational::zero());
        check_square_identity(&p, &m, &s, false).unwrap();
        for text in ["-(x-y)^2", "x*(x-y)^2", "x^2+y^2"] {
            assert!(
                nonnegative_square_witness(&algebra.parse(text).unwrap())
                    .unwrap()
                    .is_none()
            );
        }
        // Both discovery paths agree on the square up to a constant factor.
        let (pm, ps) = positive_square_witness(&p).unwrap();
        assert_eq!(&ps.pow(2) * &p.constant(pm), &s.pow(2) * &p.constant(m));
    }
}
