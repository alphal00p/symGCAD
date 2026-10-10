//! Algebra-only factor preconditioning for p(..., u+v, ...).
//!
//! In characteristic zero, equal partial derivatives certify dependence on the
//! sum. Setting u=0 gives q, and we additionally check q(v+u)=p exactly. Factoring
//! q in the smaller polynomial ring and adjoining the unused u preserves
//! irreducibility; the inverse shear is a polynomial-ring automorphism. These
//! substitutions never change CAD coordinates or polynomial variable maps.
//! Caps bound discovery, substitution and reconstruction work. The native
//! reduced factorization still relies on the caller's killable worker deadline,
//! just like the original backend factorization.

use super::Poly;
use symbolica::{poly::factor::Factorize, prelude::*};

const MAX_INPUT_TERMS: usize = 1024;
const MAX_VARIABLES: usize = 64;
const MAX_ACTIVE: usize = 16;
const MAX_TOTAL_DEGREE: u64 = 32;
const MAX_COEFFICIENT_BITS: u64 = 4096;
const MAX_EXPONENT_VISITS: usize = 65_536;
const MAX_REDUCTIONS: usize = 2;
const MAX_EXPANDED_TERMS: usize = 65_536;
const MAX_PRODUCTS: usize = 262_144;

struct Reduction {
    polynomial: Poly,
    /// Each entry restores the eliminated variable into the retained one.
    substitutions: Vec<(usize, Poly)>,
}

fn total_degree(exponents: &[u32]) -> Option<u64> {
    exponents.iter().try_fold(0u64, |degree, &e| {
        degree
            .checked_add(u64::from(e))
            .filter(|&d| d <= MAX_TOTAL_DEGREE)
    })
}

fn eligible(p: &Poly) -> bool {
    !p.is_constant()
        && p.nterms() <= MAX_INPUT_TERMS
        && p.nvars() <= MAX_VARIABLES
        && p.nterms()
            .checked_mul(p.nvars())
            .is_some_and(|work| work <= MAX_EXPONENT_VISITS)
        && p.exponents_iter()
            .all(|exponents| total_degree(exponents).is_some())
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= MAX_COEFFICIENT_BITS
                && c.denominator_ref().significant_bits() <= MAX_COEFFICIENT_BITS
        })
}

fn active_variables(p: &Poly) -> Vec<usize> {
    (0..p.nvars()).filter(|&axis| p.degree(axis) > 0).collect()
}

fn sum(p: &Poly, first: usize, second: usize) -> Poly {
    let mut exponents = vec![0; p.nvars()];
    exponents[first] = 1;
    let first = p.monomial(Rational::one(), exponents.clone());
    exponents.fill(0);
    exponents[second] = 1;
    &first + &p.monomial(Rational::one(), exponents)
}

/// A two-term linear substitution expands each monomial into at most d+1
/// terms. Check that bound before calling Symbolica, including exponent sums.
fn restore(p: &Poly, variable: usize, replacement: &Poly) -> Option<Poly> {
    if p.nvars() != replacement.nvars() || p.variables() != replacement.variables() {
        return None;
    }
    let mut expanded = 0usize;
    for exponents in p.exponents_iter() {
        total_degree(exponents)?;
        expanded = expanded.checked_add((exponents[variable] as usize).checked_add(1)?)?;
        if expanded > MAX_EXPANDED_TERMS {
            return None;
        }
    }
    let restored = p.replace_with_poly(variable, replacement);
    (restored.nterms() <= MAX_EXPANDED_TERMS).then_some(restored)
}

fn reduce(original: &Poly) -> Option<Reduction> {
    if !eligible(original) {
        return None;
    }
    let mut p = original.clone();
    let mut substitutions = Vec::new();
    for _ in 0..MAX_REDUCTIONS {
        let active = active_variables(&p);
        if !(3..=MAX_ACTIVE).contains(&active.len()) {
            break;
        }
        // Each derivative is calculated only once per bounded reduction.
        let derivatives: Vec<_> = active.iter().map(|&v| p.derivative(v)).collect();
        let mut found = None;
        'pairs: for (i, &first) in active.iter().enumerate() {
            for (j, &second) in active.iter().enumerate().skip(i + 1) {
                if derivatives[i] != derivatives[j] {
                    continue;
                }
                let q = p.replace(first, &Rational::zero());
                let inverse = sum(&p, first, second);
                if q.degree(first) != 0
                    || active_variables(&q).len() >= active.len()
                    || restore(&q, second, &inverse)? != p
                {
                    return None;
                }
                found = Some((q, second, inverse));
                break 'pairs;
            }
        }
        let Some((q, second, inverse)) = found else {
            break;
        };
        substitutions.push((second, inverse));
        p = q;
    }
    (!substitutions.is_empty()).then_some(Reduction {
        polynomial: p,
        substitutions,
    })
}

/// Resource-bounded product verification includes constants and multiplicities.
/// Declining this optional path leaves the original complete factorizer intact.
fn reconstructs(original: &Poly, factors: &[(Poly, usize)]) -> bool {
    let mut product = original.one();
    let mut degree = 0u64;
    let mut work = 0usize;
    for (factor, multiplicity) in factors {
        if factor.variables() != original.variables() || *multiplicity == 0 {
            return false;
        }
        let Some(factor_degree) = factor
            .exponents_iter()
            .try_fold(0, |degree, e| total_degree(e).map(|d| degree.max(d)))
        else {
            return false;
        };
        let Some(contribution) = factor_degree.checked_mul(*multiplicity as u64) else {
            return false;
        };
        let Some(next_degree) = degree.checked_add(contribution) else {
            return false;
        };
        degree = next_degree;
        if degree > MAX_TOTAL_DEGREE || *multiplicity > MAX_TOTAL_DEGREE as usize + 1 {
            return false;
        }
        for _ in 0..*multiplicity {
            let Some(next_work) = product
                .nterms()
                .checked_mul(factor.nterms())
                .and_then(|v| work.checked_add(v))
            else {
                return false;
            };
            work = next_work;
            if work > MAX_PRODUCTS {
                return false;
            }
            product = &product * factor;
            if product.nterms() > MAX_EXPANDED_TERMS {
                return false;
            }
        }
    }
    product == *original
}

pub(super) fn try_factor(original: &Poly) -> Option<Vec<(Poly, usize)>> {
    let reduction = reduce(original)?;
    // This is the same complete Symbolica backend as the ordinary path.
    let mut factors = reduction.polynomial.factor();
    if !reconstructs(&reduction.polynomial, &factors) {
        return None;
    }
    for (factor, _) in &mut factors {
        for (variable, replacement) in reduction.substitutions.iter().rev() {
            *factor = restore(factor, *variable, replacement)?;
        }
    }
    // Mandatory even for ordinary unchecked solves: the preconditioner never
    // returns factors until the original signed rational polynomial is rebuilt.
    reconstructs(original, &factors).then_some(factors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{Algebra, canonical, factor_polynomial};

    fn algebra() -> Algebra {
        Algebra::new(&["s".into(), "t".into(), "x".into(), "unused".into()]).unwrap()
    }

    #[test]
    fn captured_a446_sum_dependent_factor_is_reconstructed_exactly() {
        let a = algebra();
        // Exact captured 41-term operand, written in its checked compact form.
        let p = a.parse("32400+67680*x+67020*x^2+37340*x^3+11955*x^4+2010*x^5+135*x^6 + (s+t)*(-19440-34128*x-48564*x^2-38028*x^3-15441*x^4-3132*x^5-243*x^6) + (s+t)^2*(-11664*x-3996*x^2+1512*x^3-108*x^4) + (s+t)^3*(-972*x^2+972*x^4)").unwrap();
        assert_eq!(p.nterms(), 41);
        let reduced = reduce(&p).unwrap();
        assert_eq!(active_variables(&reduced.polynomial), vec![1, 2]);
        assert_eq!(reduced.substitutions.len(), 1);
        let fs = try_factor(&p).unwrap();
        assert!(reconstructs(&p, &fs));
        assert_eq!(
            fs.iter()
                .filter(|(p, _)| !p.is_constant())
                .map(|(p, _)| p.nterms())
                .sum::<usize>(),
            26
        );
        assert!(reconstructs(&p, &factor_polynomial(&p, false).unwrap()));
    }

    #[test]
    fn negative_rational_content_and_repeated_factors_survive_inverse_shear() {
        let a = algebra();
        let p = a.parse("-5*(s+t+x)^2*(s+t-x^2)^3/7").unwrap();
        let fs = try_factor(&p).unwrap();
        assert!(reconstructs(&p, &fs));
        for (expression, expected) in [("s+t+x", 2), ("s+t-x^2", 3)] {
            let expected_factor = canonical(&a.parse(expression).unwrap());
            assert!(
                fs.iter()
                    .any(|(f, m)| canonical(f) == expected_factor && *m == expected)
            );
        }
        assert!(
            fs.iter()
                .all(|(f, _)| f.variables() == &a.variables && f.degree(3) == 0)
        );
    }

    #[test]
    fn successive_reductions_reverse_in_the_correct_order_and_stop_at_cap() {
        let a =
            Algebra::new(&["s".into(), "t".into(), "u".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("(s+t+u+v+x^2)^2+1").unwrap();
        let reduced = reduce(&p).unwrap();
        assert_eq!(reduced.substitutions.len(), MAX_REDUCTIONS);
        assert_eq!(active_variables(&reduced.polynomial).len(), 3);
        let mut restored = reduced.polynomial;
        for (axis, replacement) in reduced.substitutions.iter().rev() {
            restored = restore(&restored, *axis, replacement).unwrap();
        }
        assert_eq!(restored, p);
        // The cap is permanent for this call; reduction discovery is not
        // recursively re-entered by factor_polynomial.
    }

    #[test]
    fn almost_invariant_and_small_support_inputs_keep_the_native_path() {
        let a = algebra();
        for expression in ["(s+t+x^2)^2+1+s/1000000", "(s+t)^2+1", "x", "0", "7/3"] {
            assert!(
                reduce(&a.parse(expression).unwrap()).is_none(),
                "{expression}"
            );
        }
        let p = a.parse("(s+t+x^2)^2+1+s/1000000").unwrap();
        assert!(reconstructs(&p, &factor_polynomial(&p, true).unwrap()));
    }

    #[test]
    fn degree_term_map_and_coefficient_caps_decline_without_expansion() {
        let a = algebra();
        assert!(reduce(&a.parse("(s+t+x)^33+1").unwrap()).is_none());
        assert!(reduce(&a.parse("(s+t+x+unused)^20").unwrap()).is_none());
        let mut p = a.parse("s+t+x^2").unwrap();
        p.coefficients[0] = Rational::from(Integer::from(2).pow(4097));
        assert!(reduce(&p).is_none());
        let wide = Algebra::new(&(0..65).map(|i| format!("v{i}")).collect::<Vec<_>>()).unwrap();
        assert!(reduce(&wide.parse("v0+v1+v2^2").unwrap()).is_none());
        let seventeen =
            Algebra::new(&(0..17).map(|i| format!("v{i}")).collect::<Vec<_>>()).unwrap();
        let expression = (0..17)
            .map(|i| format!("v{i}^2"))
            .collect::<Vec<_>>()
            .join("+");
        assert!(reduce(&seventeen.parse(&expression).unwrap()).is_none());
    }

    #[test]
    fn verification_rejects_wrong_content_multiplicity_or_unbounded_work() {
        let a = algebra();
        let p = a.parse("-3*(s+t+x)^2/7").unwrap();
        let mut fs = try_factor(&p).unwrap();
        assert!(reconstructs(&p, &fs));
        let nonconstant = fs.iter().position(|(f, _)| !f.is_constant()).unwrap();
        fs[nonconstant].1 += 1;
        assert!(!reconstructs(&p, &fs));
        assert!(!reconstructs(&p, &[(p.clone(), 0)]));
        assert!(!reconstructs(&p, &[(p.clone(), usize::MAX)]));
        assert!(!reconstructs(&p, &[(p.one(), 1)]));
    }
}
