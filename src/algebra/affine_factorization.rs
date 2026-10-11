//! Optional exact scaling/translation preconditioning for native factorization.
//!
//! A degree drop on a small integer fiber can make modular evaluation and
//! lifting expensive. This heuristic tests a fixed finite shift menu, selects
//! at most one improved fiber, and factors one transformed polynomial once.
//! Nonzero diagonal scaling reaches that fiber while preserving support;
//! integer translation remains the fallback when scaling is inadmissible.
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
const MAX_TRANSFORM_WORK: usize = 1_048_576;
const MAX_PRODUCT_WORK: usize = 4_194_304;
const SHIFT_MENU: [i64; 3] = [1, -1, 2];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Admission {
    /// Original full-map indices, never compressed or reordered.
    pub shifts: Vec<(usize, i64)>,
    pub original_degree_drops: usize,
    pub translated_degree_drops: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TransformKind {
    DiagonalScaling,
    IntegerTranslation,
}

impl TransformKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::DiagonalScaling => "diagonal_scaling",
            Self::IntegerTranslation => "integer_translation",
        }
    }
}

#[cfg(test)]
thread_local! {
    static NATIVE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
fn translate(p: &Poly, shifts: &[(usize, i64)], work: &mut usize) -> Option<Poly> {
    if !bounded(p, MAX_WORK_TERMS, MAX_WORK_BITS) {
        return None;
    }
    let mut q = p.clone();
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
        *work = work.checked_add(expanded)?;
        if *work > MAX_TRANSFORM_WORK {
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

/// Nonzero diagonal Q-automorphism. Every monomial keeps its exponents. The
/// conservative height preflight precedes coefficient/power allocation; inverse
/// scaling is exact rational arithmetic, never division by a polynomial.
fn scale(p: &Poly, scales: &[(usize, i64)], inverse: bool, work: &mut usize) -> Option<Poly> {
    if !bounded(p, MAX_WORK_TERMS, MAX_WORK_BITS) || scales.len() > MAX_ACTIVE {
        return None;
    }
    let mut seen = vec![false; p.nvars()];
    let mut growth = Vec::new();
    for &(axis, value) in scales {
        if axis >= p.nvars() || seen[axis] || value == 0 || value.unsigned_abs() > 9 {
            return None;
        }
        seen[axis] = true;
        // ceil(log2(abs(value))); 1 and -1 do not increase bit length.
        growth.push((
            axis,
            u64::from(64 - (value.unsigned_abs() - 1).leading_zeros()),
        ));
    }
    for (coefficient, exponents) in p.coefficients.iter().zip(p.exponents_iter()) {
        let extra = growth.iter().try_fold(0u64, |sum, &(axis, bits)| {
            sum.checked_add(bits.checked_mul(u64::from(exponents[axis]))?)
        })?;
        let height = if inverse {
            coefficient.denominator_ref().significant_bits()
        } else {
            coefficient.numerator_ref().significant_bits()
        };
        if height.checked_add(extra)? > MAX_WORK_BITS {
            return None;
        }
    }
    let power_work = scales.iter().try_fold(0usize, |sum, &(axis, _)| {
        sum.checked_add(p.degree(axis) as usize)
    })?;
    *work = work.checked_add(
        p.nterms()
            .checked_mul(scales.len())?
            .checked_add(power_work)?,
    )?;
    if *work > MAX_TRANSFORM_WORK {
        return None;
    }
    let powers = scales
        .iter()
        .map(|&(axis, value)| {
            let base = if inverse {
                Rational::from((1, value))
            } else {
                Rational::from(value)
            };
            let mut table = Vec::with_capacity(p.degree(axis) as usize + 1);
            table.push(Rational::one());
            for exponent in 1..=p.degree(axis) as usize {
                table.push(&table[exponent - 1] * &base);
            }
            (axis, table)
        })
        .collect::<Vec<_>>();
    let mut q = p.clone();
    for (coefficient, exponents) in q.coefficients.iter_mut().zip(p.exponents_iter()) {
        for (axis, table) in &powers {
            *coefficient = &*coefficient * &table[exponents[*axis] as usize];
        }
    }
    if q.variables() != p.variables()
        || !bounded(&q, MAX_WORK_TERMS, MAX_WORK_BITS)
        || q.nterms() != p.nterms()
        || !q.exponents_iter().eq(p.exponents_iter())
    {
        return None;
    }
    Some(q)
}

/// Complete signed reconstruction, including constants and multiplicities.
fn reconstructed(original: &Poly, factors: &[(Poly, usize)], work: &mut usize) -> Option<Poly> {
    if factors.len() > MAX_TOTAL_DEGREE as usize + 1 {
        return None;
    }
    let mut product = original.one();
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
            *work = next_work;
            if *work > MAX_PRODUCT_WORK {
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
    reconstructed(original, factors, &mut 0).is_some_and(|p| p == *original)
}

pub(super) fn try_factor(original: &Poly) -> Result<Option<Vec<(Poly, usize)>>> {
    Ok(try_factor_with_kind(original, |_| Ok(()))?.map(|(factors, _)| factors))
}

/// The observer runs only after an exact prepared transform is selected and
/// before the single native call; it cannot mislabel a pending scale attempt.
pub(super) fn try_factor_with_kind(
    original: &Poly,
    mut selected: impl FnMut(TransformKind) -> Result<()>,
) -> Result<Option<(Vec<(Poly, usize)>, TransformKind)>> {
    let Some(plan) = admission(original) else {
        return Ok(None);
    };
    let mut transform_work = 0;
    let scales = plan
        .shifts
        .iter()
        .map(|&(i, s)| Some((i, s.checked_add(1)?)))
        .collect::<Option<Vec<_>>>();
    let scaled = scales.as_ref().and_then(|scales| {
        let image = scale(original, scales, false, &mut transform_work)?;
        let restored = scale(&image, scales, true, &mut transform_work)?;
        Some((image, restored))
    });
    let inverse = plan
        .shifts
        .iter()
        .map(|&(i, s)| (i, -s))
        .collect::<Vec<_>>();
    let (shifted, restored, kind) = if let Some((image, restored)) = scaled {
        (image, restored, TransformKind::DiagonalScaling)
    } else {
        let Some(image) = translate(original, &plan.shifts, &mut transform_work) else {
            return Ok(None);
        };
        let Some(restored) = translate(&image, &inverse, &mut transform_work) else {
            return Ok(None);
        };
        (image, restored, TransformKind::IntegerTranslation)
    };
    ensure!(
        restored == *original,
        "affine factor preconditioner inverse mismatch"
    );
    selected(kind)?;
    // One native call only: a stalled call cannot trigger timed fallback.
    // Native irreducibility is preserved by the inverse Q-ring automorphism.
    #[cfg(test)]
    NATIVE_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut factors = shifted.factor();
    ensure!(
        factors
            .iter()
            .all(|(p, m)| p.variables() == original.variables() && *m > 0 && !p.is_zero()),
        "invalid native factors in affine preconditioner"
    );
    let mut product_work = 0;
    let product = reconstructed(&shifted, &factors, &mut product_work)
        .context("affine factor reconstruction exceeded work limits after native factoring")?;
    ensure!(
        product == shifted,
        "transformed signed factor reconstruction mismatch"
    );
    for (factor, _) in &mut factors {
        *factor = match kind {
            TransformKind::DiagonalScaling => {
                scale(factor, scales.as_ref().unwrap(), true, &mut transform_work)
            }
            TransformKind::IntegerTranslation => translate(factor, &inverse, &mut transform_work),
        }
        .context("affine factor inverse exceeded work limits after native factoring")?;
    }
    let product = reconstructed(original, &factors, &mut product_work)
        .context("original factor reconstruction exceeded work limits after native factoring")?;
    ensure!(
        product == *original,
        "original signed factor reconstruction mismatch"
    );
    Ok(Some((factors, kind)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{Algebra, canonical};

    fn algebra() -> Algebra {
        Algebra::new(&["x".into(), "unused".into(), "y".into(), "z".into()]).unwrap()
    }

    #[test]
    fn signed_repeated_factors_and_inactive_coordinates_survive_scaling() {
        let a = algebra();
        let p = a.parse("-5*((y-1)*x^2+z+2)^2*(x+y+z)^3/7").unwrap();
        let plan = admission(&p).unwrap();
        assert_eq!(plan.shifts, vec![(0, 1), (2, 2), (3, 3)]);
        assert!(plan.translated_degree_drops < plan.original_degree_drops);
        NATIVE_CALLS.with(|calls| calls.set(0));
        let mut observed = Vec::new();
        let (fs, kind) = try_factor_with_kind(&p, |kind| {
            observed.push(kind);
            Ok(())
        })
        .unwrap()
        .unwrap();
        assert_eq!(kind, TransformKind::DiagonalScaling);
        assert_eq!(observed, vec![kind]);
        NATIVE_CALLS.with(|calls| assert_eq!(calls.get(), 1));
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
        // The selected regular point has a zero first coordinate. Diagonal
        // scaling would not be invertible, so retain the old translation.
        NATIVE_CALLS.with(|calls| calls.set(0));
        let (factors, kind) = try_factor_with_kind(&p, |_| Ok(())).unwrap().unwrap();
        assert_eq!(kind, TransformKind::IntegerTranslation);
        assert!(reconstructs(&p, &factors));
        NATIVE_CALLS.with(|calls| assert_eq!(calls.get(), 1));
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
        assert!(translate(&p, &[(0, 1), (0, -1)], &mut 0).is_none());
        assert!(translate(&p, &[(4, 1)], &mut 0).is_none());
        let plan = admission(&p).unwrap();
        let shifted = translate(&p, &plan.shifts, &mut 0).unwrap();
        assert_ne!(shifted, p);
        let inverse = plan
            .shifts
            .iter()
            .map(|&(i, s)| (i, -s))
            .collect::<Vec<_>>();
        assert_eq!(translate(&shifted, &inverse, &mut 0).unwrap(), p);
    }

    #[test]
    fn diagonal_scaling_preserves_support_and_native_initial_leading_layers() {
        let a = algebra();
        let p = a.parse("-3*((y-1)*x^2+z)^2/7").unwrap();
        let scales = [(0, 2), (2, 3), (3, 4)];
        let mut work = 0;
        let q = scale(&p, &scales, false, &mut work).unwrap();
        assert!(q.exponents_iter().eq(p.exponents_iter()));
        assert_eq!(q.variables(), p.variables());
        for axis in [0, 2, 3] {
            assert_eq!(q.degree(axis), p.degree(axis));
            assert_eq!(
                q.univariate_lcoeff(axis).nterms(),
                p.univariate_lcoeff(axis).nterms()
            );
        }
        let preparation_work = work;
        assert_eq!(scale(&q, &scales, true, &mut work).unwrap(), p);
        assert!(work > preparation_work);
        assert_ne!(
            scale(&q, &[(0, 2), (2, 2), (3, 4)], true, &mut 0).unwrap(),
            p
        );
    }

    #[test]
    fn scaling_keeps_permuted_full_maps_and_negative_rational_units() {
        for names in [vec!["x", "unused", "y", "z"], vec!["z", "y", "unused", "x"]] {
            let a = Algebra::new(&names.into_iter().map(String::from).collect::<Vec<_>>()).unwrap();
            let p = a.parse("-11/13*((y-1)*x^2+z+2)^2*(x+y+z)^3").unwrap();
            let (factors, kind) = try_factor_with_kind(&p, |_| Ok(())).unwrap().unwrap();
            assert_eq!(kind, TransformKind::DiagonalScaling);
            assert!(reconstructs(&p, &factors));
            assert!(factors.iter().all(|(f, _)| f.variables() == p.variables()));
        }
        let a = algebra();
        let p = a.parse("-11/13*(x-y)^2*(x+z)").unwrap();
        let scales = [(0, -2), (2, 3), (3, -4)];
        let q = scale(&p, &scales, false, &mut 0).unwrap();
        assert_eq!(scale(&q, &scales, true, &mut 0).unwrap(), p);
        let wrong = Algebra::new(&["y".into(), "unused".into(), "x".into(), "z".into()]).unwrap();
        assert!(!reconstructs(
            &p,
            &[(wrong.parse("-11/13*(x-y)^2*(x+z)").unwrap(), 1)]
        ));
    }

    #[test]
    fn scaling_and_both_replays_share_finite_budgets() {
        let a = algebra();
        let p = a.parse("(y-1)*x^2+z").unwrap();
        for scales in [
            vec![(0, 0)],
            vec![(0, 10)],
            vec![(0, i64::MIN)],
            vec![(0, 2), (0, 3)],
            vec![(4, 2)],
        ] {
            assert!(scale(&p, &scales, false, &mut 0).is_none());
        }
        let mut exhausted = MAX_TRANSFORM_WORK;
        assert!(scale(&p, &[(0, 2)], false, &mut exhausted).is_none());
        let mut exhausted = MAX_TRANSFORM_WORK;
        assert!(translate(&p, &[(0, 1)], &mut exhausted).is_none());
        let tall = p
            .clone()
            .mul_coeff(Rational::from(Integer::from(2).pow(MAX_WORK_BITS - 1)));
        assert!(scale(&tall, &[(0, 9)], false, &mut 0).is_none());
        let tiny = p.clone().mul_coeff(Rational::from((
            Integer::one(),
            Integer::from(2).pow(MAX_WORK_BITS - 1),
        )));
        assert!(scale(&tiny, &[(0, 9)], true, &mut 0).is_none());
        let factors = vec![(p.clone(), 1)];
        let mut work = MAX_PRODUCT_WORK - p.nterms();
        assert_eq!(reconstructed(&p, &factors, &mut work).unwrap(), p);
        assert!(reconstructed(&p, &factors, &mut work).is_none());
        NATIVE_CALLS.with(|calls| calls.set(0));
        assert!(try_factor_with_kind(&p, |_| anyhow::bail!("observer failed")).is_err());
        NATIVE_CALLS.with(|calls| assert_eq!(calls.get(), 0));
    }
}
