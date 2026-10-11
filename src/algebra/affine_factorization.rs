//! Optional exact integer-translation preconditioning for native factorization.
//!
//! A degree drop on a small integer fiber can make modular evaluation and
//! lifting expensive. This heuristic tests a fixed finite shift menu, selects
//! at most one improved fiber, and factors its translated polynomial once.
//! It is an algebra-only polynomial-ring automorphism: every factor is mapped
//! back before use, including signed content and multiplicities. Original CAD
//! coordinates, domains, roots and seams are unchanged.
//!
//! Admission is deliberately conservative and is not a factorization theorem.
//! Exact inverse and signed-product checks supply the mathematical obligation.
//! Symbolica's native factor call still needs the caller's killable deadline;
//! bounded discovery does not make that call interruptible.

use super::Poly;
use anyhow::{Context, Result, ensure};
use symbolica::{poly::factor::Factorize, prelude::*};

const MAX_VARIABLES: usize = 64;
const MAX_INPUT_TERMS: usize = 2048;
const MAX_ACTIVE: usize = 4;
const MAX_AXIS_DEGREE: u32 = 32;
const MAX_TOTAL_DEGREE: u64 = 48;
const MAX_INPUT_BITS: u64 = 4096;
const MAX_WORK_BITS: u64 = 8192;
const MAX_WORK_TERMS: usize = 65_536;
const MAX_TRANSLATION_WORK: usize = 1_048_576;
const MAX_PRODUCT_WORK: usize = 4_194_304;
const SHIFT_MENU: [i64; 3] = [1, -1, 2];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Admission {
    /// Original full-map indices, never compressed or reordered.
    pub shifts: Vec<(usize, i64)>,
    pub original_degree_drops: usize,
    pub translated_degree_drops: usize,
}

fn bounded(p: &Poly, terms: usize, bits: u64) -> bool {
    p.nvars() <= MAX_VARIABLES
        && p.nterms() <= terms
        && p.exponents_iter().all(|exponents| {
            exponents.iter().all(|&e| e <= MAX_AXIS_DEGREE)
                && exponents
                    .iter()
                    .try_fold(0u64, |sum, &e| {
                        sum.checked_add(u64::from(e))
                            .filter(|&s| s <= MAX_TOTAL_DEGREE)
                    })
                    .is_some()
        })
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= bits
                && c.denominator_ref().significant_bits() <= bits
        })
}

/// Testing a leading coefficient suffices: the fiber loses degree iff this
/// exact value is zero. Specializing constants cannot increase term support.
fn degree_drops(leading: &[(usize, Poly)], point: &[(usize, i64)]) -> usize {
    leading
        .iter()
        .filter(|(axis, coefficient)| {
            let mut value = coefficient.clone();
            for &(other, coordinate) in point {
                if other != *axis {
                    value = value.replace(other, &Rational::from(coordinate));
                }
            }
            value.is_zero()
        })
        .count()
}

pub(super) fn admission(p: &Poly) -> Option<Admission> {
    if p.is_constant() || !bounded(p, MAX_INPUT_TERMS, MAX_INPUT_BITS) {
        return None;
    }
    let active = (0..p.nvars())
        .filter(|&i| p.degree(i) > 0)
        .collect::<Vec<_>>();
    if !(2..=MAX_ACTIVE).contains(&active.len()) {
        return None;
    }
    let leading = active
        .iter()
        .map(|&i| (i, p.univariate_lcoeff(i)))
        .collect::<Vec<_>>();
    let ones = active.iter().map(|&i| (i, 1)).collect::<Vec<_>>();
    let original_degree_drops = degree_drops(&leading, &ones);
    if original_degree_drops == 0 {
        return None;
    }
    for k in SHIFT_MENU {
        let shifts = active
            .iter()
            .enumerate()
            .map(|(j, &i)| (i, k * (j as i64 + 1)))
            .collect::<Vec<_>>();
        let point = shifts.iter().map(|&(i, s)| (i, 1 + s)).collect::<Vec<_>>();
        let translated_degree_drops = degree_drops(&leading, &point);
        if translated_degree_drops < original_degree_drops {
            return Some(Admission {
                shifts,
                original_degree_drops,
                translated_degree_drops,
            });
        }
    }
    None
}

/// Bound every intermediate expansion before Symbolica allocates it. A shift
/// replaces x^d by at most d+1 monomials and never raises any original degree.
fn translate(p: &Poly, shifts: &[(usize, i64)]) -> Option<Poly> {
    if !bounded(p, MAX_WORK_TERMS, MAX_WORK_BITS) {
        return None;
    }
    let mut q = p.clone();
    let mut work = 0usize;
    let mut seen = vec![false; p.nvars()];
    for &(axis, shift) in shifts {
        if axis >= p.nvars() || seen[axis] || shift.unsigned_abs() > 8 {
            return None;
        }
        seen[axis] = true;
        let mut expanded = 0usize;
        for exponents in q.exponents_iter() {
            expanded = expanded.checked_add(exponents[axis] as usize + 1)?;
            if expanded > MAX_WORK_TERMS {
                return None;
            }
        }
        work = work.checked_add(expanded)?;
        if work > MAX_TRANSLATION_WORK {
            return None;
        }
        let mut powers = vec![0; p.nvars()];
        powers[axis] = 1;
        let replacement = &p.monomial(Rational::one(), powers) + &p.constant(Rational::from(shift));
        q = q.replace_with_poly(axis, &replacement);
        if q.variables() != p.variables() || !bounded(&q, MAX_WORK_TERMS, MAX_WORK_BITS) {
            return None;
        }
    }
    Some(q)
}

/// Complete signed reconstruction, including constants and multiplicities.
fn reconstructed(original: &Poly, factors: &[(Poly, usize)]) -> Option<Poly> {
    if factors.len() > MAX_TOTAL_DEGREE as usize + 1 {
        return None;
    }
    let mut product = original.one();
    let mut work = 0usize;
    let mut total_degree = 0u64;
    for (factor, multiplicity) in factors {
        if factor.variables() != original.variables()
            || *multiplicity == 0
            || *multiplicity > MAX_TOTAL_DEGREE as usize
            || factor.is_zero()
            || !bounded(factor, MAX_WORK_TERMS, MAX_WORK_BITS)
        {
            return None;
        }
        let degree = factor
            .exponents_iter()
            .map(|e| e.iter().map(|&x| u64::from(x)).sum::<u64>())
            .max()
            .unwrap_or(0);
        let Some(next_degree) = degree
            .checked_mul(*multiplicity as u64)
            .and_then(|d| total_degree.checked_add(d))
        else {
            return None;
        };
        total_degree = next_degree;
        if total_degree > MAX_TOTAL_DEGREE {
            return None;
        }
        for _ in 0..*multiplicity {
            let Some(next_work) = product
                .nterms()
                .checked_mul(factor.nterms())
                .and_then(|n| work.checked_add(n))
            else {
                return None;
            };
            work = next_work;
            if work > MAX_PRODUCT_WORK {
                return None;
            }
            product = &product * factor;
            if !bounded(&product, MAX_WORK_TERMS, MAX_WORK_BITS) {
                return None;
            }
        }
    }
    Some(product)
}

#[cfg(test)]
fn reconstructs(original: &Poly, factors: &[(Poly, usize)]) -> bool {
    reconstructed(original, factors).is_some_and(|p| p == *original)
}

pub(super) fn try_factor(original: &Poly) -> Result<Option<Vec<(Poly, usize)>>> {
    let Some(plan) = admission(original) else {
        return Ok(None);
    };
    let inverse = plan
        .shifts
        .iter()
        .map(|&(i, s)| (i, -s))
        .collect::<Vec<_>>();
    let Some(shifted) = translate(original, &plan.shifts) else {
        return Ok(None);
    };
    let Some(restored) = translate(&shifted, &inverse) else {
        return Ok(None);
    };
    ensure!(
        restored == *original,
        "affine factor preconditioner inverse mismatch"
    );
    // One native call only. Native irreducibility is preserved by the inverse
    // polynomial-ring automorphism, with no rational-function denominators.
    let mut factors = shifted.factor();
    ensure!(
        factors
            .iter()
            .all(|(p, m)| p.variables() == original.variables() && *m > 0 && !p.is_zero()),
        "invalid native factors in affine preconditioner"
    );
    let product = reconstructed(&shifted, &factors)
        .context("affine factor reconstruction exceeded work limits after native factoring")?;
    ensure!(
        product == shifted,
        "translated signed factor reconstruction mismatch"
    );
    for (factor, _) in &mut factors {
        *factor = translate(factor, &inverse)
            .context("affine factor inverse exceeded work limits after native factoring")?;
    }
    let product = reconstructed(original, &factors)
        .context("original factor reconstruction exceeded work limits after native factoring")?;
    ensure!(
        product == *original,
        "original signed factor reconstruction mismatch"
    );
    Ok(Some(factors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{Algebra, canonical};

    fn algebra() -> Algebra {
        Algebra::new(&["x".into(), "unused".into(), "y".into(), "z".into()]).unwrap()
    }

    #[test]
    fn signed_repeated_factors_and_inactive_coordinates_survive_translation() {
        let a = algebra();
        let p = a.parse("-5*((y-1)*x^2+z+2)^2*(x+y+z)^3/7").unwrap();
        let plan = admission(&p).unwrap();
        assert_eq!(plan.shifts, vec![(0, 1), (2, 2), (3, 3)]);
        assert!(plan.translated_degree_drops < plan.original_degree_drops);
        let fs = try_factor(&p).unwrap().unwrap();
        assert!(reconstructs(&p, &fs));
        for (expression, multiplicity) in [("(y-1)*x^2+z+2", 2), ("x+y+z", 3)] {
            assert!(fs.iter().any(|(f, m)| canonical(f)
                == canonical(&a.parse(expression).unwrap())
                && *m == multiplicity));
        }
        assert!(
            fs.iter()
                .all(|(f, _)| f.variables() == &a.variables && f.degree(1) == 0)
        );
    }

    #[test]
    fn first_shift_that_improves_exact_fibers_is_selected() {
        let a = algebra();
        let p = a.parse("(y-1)*(y-3)*(x^2+1)+z+1").unwrap();
        let plan = admission(&p).unwrap();
        assert_eq!(plan.shifts, vec![(0, -1), (2, -2), (3, -3)]);
        assert_eq!(plan.original_degree_drops, 1);
        assert_eq!(plan.translated_degree_drops, 0);
        assert!(reconstructs(&p, &try_factor(&p).unwrap().unwrap()));
    }

    #[test]
    fn every_finite_menu_candidate_may_decline() {
        let a = algebra();
        let p = a.parse("(y-1)*(y-3)*(y+1)*(y-5)*x^2+z+1").unwrap();
        assert!(admission(&p).is_none());
    }

    #[test]
    fn ordinary_and_out_of_budget_inputs_decline_before_native_factoring() {
        let a = algebra();
        for expr in ["0", "-5/7", "x^2-1", "x+y+z", "x^2+y^2+z^2", "(y-1)*x^33+z"] {
            assert!(admission(&a.parse(expr).unwrap()).is_none(), "{expr}");
        }
        let mut p = a.parse("(y-1)*x^2+z").unwrap();
        p.coefficients[0] = Rational::from(Integer::from(2).pow(4097));
        assert!(admission(&p).is_none());
        let wide = Algebra::new(&(0..65).map(|i| format!("v{i}")).collect::<Vec<_>>()).unwrap();
        assert!(admission(&wide.parse("(v1-1)*v0^2+v2").unwrap()).is_none());
        let many = Algebra::new(&(0..5).map(|i| format!("v{i}")).collect::<Vec<_>>()).unwrap();
        assert!(admission(&many.parse("(v1-1)*v0^2+v2+v3+v4").unwrap()).is_none());
    }

    #[test]
    fn inverse_and_signed_product_checks_reject_false_evidence() {
        let a = algebra();
        let p = a.parse("-3*((y-1)*x^2+z)^2/7").unwrap();
        let mut fs = try_factor(&p).unwrap().unwrap();
        assert!(reconstructs(&p, &fs));
        let i = fs.iter().position(|(f, _)| !f.is_constant()).unwrap();
        fs[i].1 += 1;
        assert!(!reconstructs(&p, &fs));
        assert!(!reconstructs(&p, &[(p.clone(), 0)]));
        assert!(!reconstructs(&p, &[(p.one(), 1)]));
        assert!(!reconstructs(&p, &[(p.clone(), usize::MAX)]));
        assert!(translate(&p, &[(0, 1), (0, -1)]).is_none());
        assert!(translate(&p, &[(4, 1)]).is_none());
        let plan = admission(&p).unwrap();
        let shifted = translate(&p, &plan.shifts).unwrap();
        assert_ne!(shifted, p);
        let inverse = plan
            .shifts
            .iter()
            .map(|&(i, s)| (i, -s))
            .collect::<Vec<_>>();
        assert_eq!(translate(&shifted, &inverse).unwrap(), p);
    }
}
