//! Bounded exact pairwise shears before native factorization.
//!
//! Replacing x_i by x_i +/- x_j is a polynomial-ring automorphism. A sparse
//! leading coefficient can make native modular reconstruction much cheaper,
//! even if this change increases an individual degree. Discovery is only a
//! heuristic: original coordinates and all zero sets are recovered by an exact
//! inverse and signed-product replay, including units and multiplicities.
//! The native factor call still requires the caller's killable worker deadline.

use super::Poly;
use anyhow::{Context, Result, ensure};
use symbolica::{poly::factor::Factorize, prelude::*};

const MAX_VARIABLES: usize = 64;
const MAX_SOURCE_TERMS: usize = 4096;
const MAX_ACTIVE: usize = 4;
const MAX_INPUT_AXIS_DEGREE: u32 = 32;
const MAX_TOTAL_DEGREE: u64 = 48;
const MAX_INPUT_BITS: u64 = 4096;
const MAX_WORK_BITS: u64 = 8192;
const MAX_WORK_TERMS: usize = 65_536;
const MAX_EXPANSION_WORK: usize = 2_097_152;
// Keep a quarter of the expansion budget for the selected inverse and factors.
const MAX_DISCOVERY_WORK: usize = MAX_EXPANSION_WORK * 3 / 4;
const MAX_PRODUCT_WORK: usize = 4_194_304;

#[cfg(test)]
thread_local! {
    pub(super) static NATIVE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shear {
    target: usize,
    other: usize,
    sign: i8,
}

impl Shear {
    fn inverse(self) -> Self {
        Self {
            sign: -self.sign,
            ..self
        }
    }
}

struct Admission {
    shear: Shear,
    image: Poly,
    expansion_work: usize,
}

fn bounded(p: &Poly, terms: usize, bits: u64) -> bool {
    p.nvars() <= MAX_VARIABLES
        && p.nterms() <= terms
        && p.exponents_iter().all(|exponents| {
            exponents
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

fn active(p: &Poly) -> Vec<usize> {
    (0..p.nvars()).filter(|&i| p.degree(i) > 0).collect()
}

fn leading_min(p: &Poly) -> Option<usize> {
    active(p)
        .iter()
        .map(|&i| {
            let degree = p.degree(i);
            p.exponents_iter().filter(|e| e[i] == degree).count()
        })
        .min()
}

fn coefficient_bits(p: &Poly) -> u64 {
    p.coefficients
        .iter()
        .map(|c| {
            c.numerator_ref()
                .significant_bits()
                .max(c.denominator_ref().significant_bits())
        })
        .max()
        .unwrap_or(0)
}

/// Every x_i^d expands into at most d+1 terms. The total degree is preserved,
/// hence the admitted degree bound also rules out exponent overflow before
/// Symbolica constructs any intermediate polynomial. Work is shared by all
/// candidates and later inverse-factor transformations, not reset per call.
fn transform(p: &Poly, shear: Shear, work: &mut usize, limit: usize) -> Option<Poly> {
    if !bounded(p, MAX_WORK_TERMS, MAX_WORK_BITS)
        || shear.target >= p.nvars()
        || shear.other >= p.nvars()
        || shear.target == shear.other
        || ![-1, 1].contains(&shear.sign)
        || limit > MAX_EXPANSION_WORK
    {
        return None;
    }
    let expansion = p.exponents_iter().try_fold(0usize, |sum, e| {
        sum.checked_add(e[shear.target] as usize + 1)
            .filter(|&n| n <= MAX_WORK_TERMS)
    })?;
    let next = work.checked_add(expansion)?;
    if next > limit {
        return None;
    }
    *work = next;
    let mut exponent = vec![0; p.nvars()];
    exponent[shear.target] = 1;
    let target = p.monomial(Rational::one(), exponent.clone());
    exponent[shear.target] = 0;
    exponent[shear.other] = 1;
    let other = p.monomial(Rational::from(shear.sign as i64), exponent);
    let result = p.replace_with_poly(shear.target, &(&target + &other));
    (result.variables() == p.variables() && bounded(&result, MAX_WORK_TERMS, MAX_WORK_BITS))
        .then_some(result)
}

fn admission(p: &Poly) -> Option<Admission> {
    if p.is_constant()
        || !bounded(p, MAX_SOURCE_TERMS, MAX_INPUT_BITS)
        || (0..p.nvars()).any(|i| p.degree(i) > MAX_INPUT_AXIS_DEGREE)
        || leading_min(p)? <= 1
    {
        return None;
    }
    let axes = active(p);
    if !(2..=MAX_ACTIVE).contains(&axes.len()) {
        return None;
    }
    let mut work = 0;
    let mut best = None;
    for &target in &axes {
        for &other in &axes {
            if target == other {
                continue;
            }
            for sign in [-1, 1] {
                let shear = Shear {
                    target,
                    other,
                    sign,
                };
                let Some(image) = transform(p, shear, &mut work, MAX_DISCOVERY_WORK) else {
                    continue;
                };
                if image.nterms() > p.nterms() || leading_min(&image) != Some(1) {
                    continue;
                }
                let rank = (
                    image.nterms(),
                    (0..image.nvars())
                        .map(|i| image.degree(i))
                        .max()
                        .unwrap_or(0),
                    coefficient_bits(&image),
                    target,
                    other,
                    sign,
                );
                if best.as_ref().is_none_or(|(old, _, _)| rank < *old) {
                    best = Some((rank, shear, image));
                }
            }
        }
    }
    best.map(|(_, shear, image)| Admission {
        shear,
        image,
        expansion_work: work,
    })
}

/// Signed reconstruction includes constant factors. Bounds are cumulative
/// across both the transformed and original products.
fn reconstructed(original: &Poly, factors: &[(Poly, usize)], work: &mut usize) -> Option<Poly> {
    if factors.len() > MAX_TOTAL_DEGREE as usize + 1 {
        return None;
    }
    let mut product = original.one();
    let mut degree_sum = 0u64;
    for (factor, multiplicity) in factors {
        if factor.variables() != original.variables()
            || factor.is_zero()
            || *multiplicity == 0
            || *multiplicity > MAX_TOTAL_DEGREE as usize
            || !bounded(factor, MAX_WORK_TERMS, MAX_WORK_BITS)
        {
            return None;
        }
        let degree = factor
            .exponents_iter()
            .map(|e| e.iter().map(|&x| u64::from(x)).sum::<u64>())
            .max()
            .unwrap_or(0);
        degree_sum = degree_sum.checked_add(degree.checked_mul(*multiplicity as u64)?)?;
        if degree_sum > MAX_TOTAL_DEGREE {
            return None;
        }
        for _ in 0..*multiplicity {
            *work = work.checked_add(product.nterms().checked_mul(factor.nterms())?)?;
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

pub(super) fn try_factor(original: &Poly) -> Result<Option<Vec<(Poly, usize)>>> {
    let Some(mut plan) = admission(original) else {
        return Ok(None);
    };
    let Some(restored) = transform(
        &plan.image,
        plan.shear.inverse(),
        &mut plan.expansion_work,
        MAX_EXPANSION_WORK,
    ) else {
        return Ok(None);
    };
    ensure!(
        restored == *original,
        "shear preconditioner inverse mismatch"
    );
    // Only the best admitted candidate reaches this single native call.
    #[cfg(test)]
    NATIVE_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut factors = plan.image.factor();
    let mut product_work = 0;
    let product = reconstructed(&plan.image, &factors, &mut product_work)
        .context("shear signed reconstruction exceeded limits after native factoring")?;
    ensure!(
        product == plan.image,
        "sheared signed factor product mismatch"
    );
    for (factor, _) in &mut factors {
        *factor = transform(
            factor,
            plan.shear.inverse(),
            &mut plan.expansion_work,
            MAX_EXPANSION_WORK,
        )
        .context("shear factor inverse exceeded shared expansion limits after native factoring")?;
    }
    let product = reconstructed(original, &factors, &mut product_work)
        .context("original shear reconstruction exceeded limits after native factoring")?;
    ensure!(
        product == *original,
        "restored shear signed factor product mismatch"
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
    fn generic_support_cancellation_creates_a_monomial_leading_layer() {
        let a = algebra();
        let p = a.parse("(x+y)^2-4*x^2*y^2").unwrap();
        assert_eq!(leading_min(&p), Some(2));
        assert!(super::super::affine_factorization::admission(&p).is_none());
        let plan = admission(&p).unwrap();
        assert_eq!(leading_min(&plan.image), Some(1));
        assert_eq!(plan.image.nterms(), p.nterms());
        assert!(plan.expansion_work <= MAX_DISCOVERY_WORK);
        let fs = try_factor(&p).unwrap().unwrap();
        assert_eq!(reconstructed(&p, &fs, &mut 0).unwrap(), p);
        for q in ["x+y-2*x*y", "x+y+2*x*y"] {
            assert!(
                fs.iter()
                    .any(|(f, m)| *m == 1 && canonical(f) == canonical(&a.parse(q).unwrap()))
            );
        }
    }

    #[test]
    fn repeated_factors_negative_unit_and_full_inactive_map_are_preserved() {
        let a = algebra();
        let p = a.parse("-3/7*((x+y)^2-4*x^2*y^2)^2").unwrap();
        let fs = try_factor(&p).unwrap().unwrap();
        assert_eq!(reconstructed(&p, &fs, &mut 0).unwrap(), p);
        for q in ["x+y-2*x*y", "x+y+2*x*y"] {
            assert!(
                fs.iter()
                    .any(|(f, m)| *m == 2 && canonical(f) == canonical(&a.parse(q).unwrap()))
            );
        }
        assert!(
            fs.iter().all(|(f, _)| f.variables() == &a.variables
                && f.degree(1) == 0
                && f.degree(3) == 0)
        );
    }

    #[test]
    fn renamed_permuted_axes_have_the_same_exact_factorization() {
        let a = Algebra::new(&["second".into(), "spare".into(), "first".into()]).unwrap();
        let p = a
            .parse("-5/11*((first+second)^2-9*first^2*second^2)")
            .unwrap();
        let fs = try_factor(&p).unwrap().unwrap();
        assert_eq!(reconstructed(&p, &fs, &mut 0).unwrap(), p);
        assert!(
            fs.iter()
                .all(|(f, _)| f.variables() == &a.variables && f.degree(1) == 0)
        );
        for q in ["first+second-3*first*second", "first+second+3*first*second"] {
            assert!(
                fs.iter()
                    .any(|(f, m)| *m == 1 && canonical(f) == canonical(&a.parse(q).unwrap()))
            );
        }
    }

    #[test]
    fn admission_requires_a_strict_leading_layer_improvement_without_support_growth() {
        let a = algebra();
        for expression in ["0", "-3/7", "x^2+y^2+z^2", "x^2*y^2+x^2*y+x*y^2+x+y+1"] {
            assert!(
                admission(&a.parse(expression).unwrap()).is_none(),
                "{expression}"
            );
        }
        let p = a.parse("(x+y)^2-4*x^2*y^2").unwrap();
        let first = admission(&p).unwrap();
        let again = admission(&p).unwrap();
        assert_eq!(first.shear, again.shear);
        assert_eq!(first.image, again.image);
    }

    #[test]
    fn input_and_shared_expansion_caps_decline_before_native_factorization() {
        let a = algebra();
        let p = a.parse("(x+y)^2-4*x^2*y^2").unwrap();
        let shear = Shear {
            target: 0,
            other: 2,
            sign: -1,
        };
        let mut used = MAX_DISCOVERY_WORK;
        assert!(transform(&p, shear, &mut used, MAX_DISCOVERY_WORK).is_none());
        assert_eq!(used, MAX_DISCOVERY_WORK);
        let mut used = MAX_EXPANSION_WORK;
        assert!(transform(&p, shear, &mut used, MAX_EXPANSION_WORK).is_none());
        assert!(admission(&a.parse("(x+y)^33+z*x^33+z*y^33").unwrap()).is_none());
        let overflow = p.monomial(Rational::one(), vec![u32::MAX, 0, 1, 0]);
        assert!(admission(&overflow).is_none());
        let many = Algebra::new(&(0..5).map(|i| format!("a{i}")).collect::<Vec<_>>()).unwrap();
        assert!(
            admission(
                &many
                    .parse("(a0+a1+a2+a3+a4)^2+a0^2*a1^2*a2^2*a3^2*a4^2")
                    .unwrap()
            )
            .is_none()
        );
        let too_many = Algebra::new(&(0..65).map(|i| format!("b{i}")).collect::<Vec<_>>()).unwrap();
        assert!(admission(&too_many.parse("(b0+b1)^2-4*b0^2*b1^2").unwrap()).is_none());
        let mut large = p.zero();
        'outer: for i in 0..16 {
            for j in 0..16 {
                for k in 0..17 {
                    large.append_monomial(Rational::one(), &[i, 0, j, k]);
                    if large.nterms() > MAX_SOURCE_TERMS {
                        break 'outer;
                    }
                }
            }
        }
        assert_eq!(large.nterms(), MAX_SOURCE_TERMS + 1);
        assert!(admission(&large).is_none());
        let huge = a
            .parse(&format!("({})*((x+y)^2-4*x^2*y^2)", "9".repeat(1300)))
            .unwrap();
        assert!(admission(&huge).is_none());
    }

    #[test]
    fn malformed_shears_maps_units_multiplicities_and_products_are_rejected() {
        let a = algebra();
        let p = a.parse("-3/7*((x+y)^2-4*x^2*y^2)^2").unwrap();
        for shear in [
            Shear {
                target: 0,
                other: 0,
                sign: 1,
            },
            Shear {
                target: 0,
                other: 4,
                sign: 1,
            },
            Shear {
                target: 4,
                other: 0,
                sign: 1,
            },
            Shear {
                target: 0,
                other: 2,
                sign: 0,
            },
        ] {
            assert!(transform(&p, shear, &mut 0, MAX_EXPANSION_WORK).is_none());
        }
        let fs = try_factor(&p).unwrap().unwrap();
        let mut bad = fs.clone();
        bad[0].1 = 0;
        assert!(reconstructed(&p, &bad, &mut 0).is_none());
        let mut bad = fs.clone();
        bad[0].1 = usize::MAX;
        assert!(reconstructed(&p, &bad, &mut 0).is_none());
        let mut bad = fs.clone();
        bad[0].0 = bad[0].0.clone().mul_coeff(Rational::from(2));
        assert_ne!(reconstructed(&p, &bad, &mut 0).unwrap(), p);
        let wrong = Algebra::new(&["y".into(), "unused".into(), "x".into(), "z".into()]).unwrap();
        assert!(reconstructed(&p, &[(wrong.parse("x+y").unwrap(), 1)], &mut 0).is_none());
        let mut exhausted = MAX_PRODUCT_WORK;
        assert!(reconstructed(&p, &fs, &mut exhausted).is_none());
        assert!(reconstructed(&p, &[(p.zero(), 1)], &mut 0).is_none());
    }
}
