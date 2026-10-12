//! Positive-multiplier square witnesses `p = m * s^2` with exact replay.
//!
//! Discovery uses Symbolica's square-free decomposition over Q: a nonzero
//! polynomial is a rational multiple of a polynomial square exactly when every
//! nonconstant square-free part has even multiplicity. The decomposition is a
//! proposal only. Every consumer replays the identity by exact expansion.
use crate::algebra::Poly;
use anyhow::{Result, ensure};
use symbolica::{poly::factor::Factorize, prelude::*};

/// Bounded proposal of `p = multiplier * square^2` with `multiplier > 0`.
/// `None` means that no such witness exists or the input exceeds the cap; it
/// is never evidence of a sign.
pub fn positive_square_witness(p: &Poly) -> Option<(Rational, Poly)> {
    if p.is_zero() || p.nterms() > 1 << 16 || (0..p.nvars()).any(|i| p.degree(i) > 4096) {
        return None;
    }
    let mut square = p.one();
    for (factor, multiplicity) in p.square_free_factorization() {
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

/// Exact replay of `lhs == multiplier * square^2` with `multiplier > 0`.
pub fn check_positive_square_identity(
    lhs: &Poly,
    multiplier: &Rational,
    square: &Poly,
) -> Result<()> {
    ensure!(
        *multiplier > Rational::zero(),
        "square witness multiplier must be strictly positive"
    );
    ensure!(!square.is_zero(), "square witness must be nonzero");
    ensure!(
        *lhs == &square.pow(2) * &lhs.constant(multiplier.clone()),
        "positive square identity failed"
    );
    Ok(())
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
}
