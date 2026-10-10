//! Research-only divide-and-conquer shifts; no production dispatch.
//! Each product has a complete centered decode and exact re-encoding.
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use symbolica::prelude::*;

const MAX_DEGREE: usize = 4096;
const MAX_HEIGHT: u64 = 65536;
const MAX_INPUT_BITS: u64 = 64_000_000;
const MAX_PACKED_BITS: u64 = 256_000_000;
const MAX_LOGICAL_BITS: u64 = 2_000_000_000;

#[derive(Default, Serialize)]
struct Statistics {
    native_leaves: usize,
    convolutions: usize,
    cached_powers: usize,
    maximum_digit_bits: u64,
    maximum_product_bits: u64,
    leaf_seconds: f64,
    power_seconds: f64,
    packing_seconds: f64,
    multiplication_seconds: f64,
    decoding_and_roundtrip_seconds: f64,
    addition_seconds: f64,
}

fn log2_ceil(n: u64) -> u64 {
    if n <= 1 {
        0
    } else {
        64 - (n - 1).leading_zeros() as u64
    }
}

/// This bounds mathematical payload, not transient allocator/GMP RSS. The
/// calling research worker must retain its independent process supervisor.
fn preflight(p: &UnivariatePolynomial<Z>, shift: &Integer, leaf: usize) -> Result<u64> {
    ensure!(
        leaf.is_power_of_two() && (8..=512).contains(&leaf),
        "recursive leaf cap"
    );
    let size = p.coefficients().len();
    ensure!(
        size <= MAX_DEGREE + 1 && shift.significant_bits() <= 256,
        "recursive degree/shift cap"
    );
    let height = p
        .coefficients()
        .iter()
        .map(Integer::significant_bits)
        .max()
        .unwrap_or(0);
    ensure!(height <= MAX_HEIGHT, "recursive coefficient height cap");
    ensure!(
        p.coefficients()
            .iter()
            .map(Integer::significant_bits)
            .sum::<u64>()
            <= MAX_INPUT_BITS,
        "recursive input storage cap"
    );
    // ||p(x+a)||_1 <= ||p||_1*(1+|a|)^n. Every subproblem has
    // no greater degree, and its unshifted L1 norm is at most ||p||_1.
    let degree = size.saturating_sub(1) as u64;
    let output_height =
        height + log2_ceil(size.max(1) as u64) + degree * (shift.significant_bits() + 1) + 2;
    let packed = (size.max(1) as u64 + 1) * (output_height + 2);
    ensure!(
        packed <= MAX_PACKED_BITS && 32 * packed <= MAX_LOGICAL_BITS,
        "recursive packed/logical storage cap"
    );
    Ok(packed)
}

fn norm(values: &[Integer]) -> Integer {
    values.iter().fold(Integer::zero(), |sum, c| sum + c.abs())
}

fn convolution(
    left: &[Integer],
    right: &[Integer],
    stats: &mut Statistics,
) -> Result<Vec<Integer>> {
    ensure!(
        !left.is_empty() && !right.is_empty(),
        "empty convolution operand"
    );
    let count = left.len() + right.len() - 1;
    ensure!(count <= MAX_DEGREE + 1, "recursive convolution degree cap");
    let bound = norm(left) * norm(right);
    if bound.is_zero() {
        return Ok(vec![Integer::zero(); count]);
    }
    // R=2^bits>2*bound excludes all centered-digit ambiguity, even for
    // negative coefficients, missing low terms and a negative packed value.
    let bits = bound.significant_bits() + 1;
    ensure!(
        (count as u64 + 1) * bits <= MAX_PACKED_BITS,
        "recursive product cap"
    );
    let started = Instant::now();
    let a = super::pack(left, bits as usize);
    let b = super::pack(right, bits as usize);
    stats.packing_seconds += started.elapsed().as_secs_f64();
    let started = Instant::now();
    let product = &a * &b;
    stats.multiplication_seconds += started.elapsed().as_secs_f64();
    stats.maximum_digit_bits = stats.maximum_digit_bits.max(bits);
    stats.maximum_product_bits = stats.maximum_product_bits.max(product.significant_bits());
    let started = Instant::now();
    let mut result = vec![Integer::zero(); count];
    super::unpack(product.clone(), &mut result, bits as usize, &bound)?;
    ensure!(
        super::pack(&result, bits as usize) == product,
        "recursive full product re-encoding"
    );
    stats.decoding_and_roundtrip_seconds += started.elapsed().as_secs_f64();
    stats.convolutions += 1;
    Ok(result)
}

/// Exact binomial recurrence, including a=0 and signed a. Construction is
/// linear in m and includes every coefficient of (x+a)^m.
fn power(shift: &Integer, m: usize) -> Result<Vec<Integer>> {
    ensure!(m <= MAX_DEGREE, "recursive power degree cap");
    let mut result = vec![Integer::zero(); m + 1];
    result[m] = Integer::one();
    for i in (1..=m).rev() {
        let numerator = &result[i] * Integer::from(i as u64) * shift;
        let (quotient, remainder) = numerator.quot_rem(&Integer::from((m - i + 1) as u64));
        ensure!(remainder.is_zero(), "nonintegral binomial recurrence");
        result[i - 1] = quotient;
    }
    Ok(result)
}

fn recurse(
    coefficients: &[Integer],
    template: &UnivariatePolynomial<Z>,
    shift: &Integer,
    leaf: usize,
    cache: &mut BTreeMap<usize, Vec<Integer>>,
    stats: &mut Statistics,
) -> Result<Vec<Integer>> {
    if coefficients.len() <= leaf {
        let started = Instant::now();
        let polynomial = UnivariatePolynomial::from_coefficients(
            &Z,
            coefficients.to_vec(),
            Arc::new(template.get_vars_ref().clone()),
        );
        let shifted = polynomial.shift_var(shift);
        let mut result = shifted.coefficients().to_vec();
        result.resize(coefficients.len(), Integer::zero());
        stats.native_leaves += 1;
        stats.leaf_seconds += started.elapsed().as_secs_f64();
        return Ok(result);
    }
    let m = coefficients.len() / 2;
    let low = recurse(&coefficients[..m], template, shift, leaf, cache, stats)?;
    let high = recurse(&coefficients[m..], template, shift, leaf, cache, stats)?;
    if let std::collections::btree_map::Entry::Vacant(entry) = cache.entry(m) {
        let started = Instant::now();
        entry.insert(power(shift, m)?);
        stats.power_seconds += started.elapsed().as_secs_f64();
    }
    let mut result = convolution(&high, &cache[&m], stats)?;
    ensure!(
        result.len() == coefficients.len(),
        "recursive output length"
    );
    let started = Instant::now();
    for (coefficient, addend) in result.iter_mut().zip(low) {
        *coefficient += addend;
    }
    stats.addition_seconds += started.elapsed().as_secs_f64();
    Ok(result)
}

pub(super) fn recursive_shift(
    polynomial: &UnivariatePolynomial<Z>,
    shift: &Integer,
    leaf: usize,
) -> Result<(UnivariatePolynomial<Z>, Value)> {
    let started = Instant::now();
    let packed_bound = preflight(polynomial, shift, leaf)?;
    let mut stats = Statistics::default();
    let mut cache = BTreeMap::new();
    let coefficients = recurse(
        polynomial.coefficients(),
        polynomial,
        shift,
        leaf,
        &mut cache,
        &mut stats,
    )?;
    stats.cached_powers = cache.len();
    let output = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(polynomial.get_vars_ref().clone()),
    );
    ensure!(
        output.degree() == polynomial.degree() && output.lcoeff() == polynomial.lcoeff(),
        "recursive shift changed degree/leading coefficient"
    );
    let metadata = json!({
        "method": "recursive_cached_binomial_convolution", "leaf": leaf,
        "packed_bits_bound": packed_bound, "logical_bits_bound": 32 * packed_bound,
        "statistics": stats, "total_seconds": started.elapsed().as_secs_f64(),
        "complete_signed_decode_roundtrip": true, "binomial_divisions_exact": true,
        "production_modified": false,
    });
    Ok((output, metadata))
}

/// Isolate the cost of multiplying by a unit inside the native triangular
/// algorithm. This deliberately does not alter the recursive method's leaves.
pub(super) fn unit_shift(
    polynomial: &UnivariatePolynomial<Z>,
    shift: &Integer,
) -> Result<(UnivariatePolynomial<Z>, Value)> {
    let started = Instant::now();
    ensure!(
        shift.is_one() || *shift == Integer::from(-1),
        "unit shift requires +1 or -1"
    );
    let packed_bound = preflight(polynomial, shift, 512)?;
    let mut coefficients = polynomial.coefficients().to_vec();
    let degree = coefficients.len().saturating_sub(1);
    let positive = shift.is_one();
    if positive {
        for k in 0..degree {
            for j in (k..degree).rev() {
                let (left, right) = coefficients.split_at_mut(j + 1);
                left[j] += &right[0];
            }
        }
    } else {
        for k in 0..degree {
            for j in (k..degree).rev() {
                let (left, right) = coefficients.split_at_mut(j + 1);
                left[j] -= &right[0];
            }
        }
    }
    let output = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(polynomial.get_vars_ref().clone()),
    );
    ensure!(
        output.degree() == polynomial.degree() && output.lcoeff() == polynomial.lcoeff(),
        "unit shift changed degree/leading coefficient"
    );
    Ok((
        output,
        json!({
            "method": "triangular_unit_add_or_subtract",
            "total_seconds": started.elapsed().as_secs_f64(),
            "coefficient_updates": degree * (degree + 1) / 2,
            "conservative_shared_recursive_packed_bound": packed_bound,
            "production_modified": false,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::algebra::Algebra;

    fn poly(values: Vec<Integer>) -> UnivariatePolynomial<Z> {
        let a = Algebra::new(&["taylor_recursive_x".into()]).unwrap();
        UnivariatePolynomial::from_coefficients(&Z, values, Arc::new(a.variables[0].clone()))
    }

    #[test]
    fn recursive_full_coefficients_signed_sparse_uneven_and_zero() -> Result<()> {
        for size in [0, 1, 2, 8, 9, 17, 31, 64, 65, 129] {
            let p = poly(
                (0..size)
                    .map(|i| Integer::from((i * 17 % 29) as i64 - 14))
                    .collect(),
            );
            let sparse = poly(
                (0..size)
                    .map(|i| Integer::from(if i % 7 == 0 { -3 } else { 0 }))
                    .collect(),
            );
            for polynomial in [&p, &sparse] {
                for shift in [-7, -1, 0, 1, 4] {
                    let shift = Integer::from(shift);
                    let (output, _) = recursive_shift(polynomial, &shift, 8)?;
                    assert_eq!(output, polynomial.shift_var(&shift));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn recursive_binomial_and_signed_product_identities() -> Result<()> {
        for shift in [-3, 0, 1, 8] {
            let shift = Integer::from(shift);
            for degree in [0, 1, 7, 32] {
                let mut coefficients = vec![Integer::zero(); degree + 1];
                coefficients[degree] = Integer::one();
                assert_eq!(
                    poly(power(&shift, degree)?),
                    poly(coefficients).shift_var(&shift)
                );
            }
        }
        let left = [-7, 0, 8, -3].map(Integer::from);
        let right = [0, 3, -2].map(Integer::from);
        let output = convolution(&left, &right, &mut Statistics::default())?;
        assert_eq!(poly(output), &poly(left.to_vec()) * &poly(right.to_vec()));
        Ok(())
    }

    #[test]
    fn recursive_multilimb_coefficients_and_leaf_choices() -> Result<()> {
        let large = super::super::shl(Integer::one(), 700);
        let p = poly(
            (0..65)
                .map(|i| {
                    &large * Integer::from((i * 13 % 23) as i64 - 11) + Integer::from(i as i64 - 37)
                })
                .collect(),
        );
        for shift in [-2, 1, 4] {
            let shift = Integer::from(shift);
            let native = p.shift_var(&shift);
            for leaf in [8, 16, 32, 64, 128, 256, 512] {
                assert_eq!(recursive_shift(&p, &shift, leaf)?.0, native);
            }
        }
        Ok(())
    }

    #[test]
    fn unit_addition_signed_large_cancellation_and_invalid_shift() -> Result<()> {
        let large = super::super::shl(Integer::one(), 700);
        let inputs = [
            poly(Vec::new()),
            poly(vec![Integer::from(-3)]),
            poly(vec![large.clone(), -large.clone(), Integer::one()]),
            poly(
                (0..65)
                    .map(|i| &large * Integer::from(i as i64 - 32))
                    .collect(),
            ),
        ];
        for p in inputs {
            for shift in [-1, 1] {
                let shift = Integer::from(shift);
                assert_eq!(unit_shift(&p, &shift)?.0, p.shift_var(&shift));
            }
            for shift in [-2, 0, 2] {
                assert!(unit_shift(&p, &Integer::from(shift)).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn recursive_preflight_bounds_before_large_algebra() -> Result<()> {
        let p = poly(vec![Integer::one(); 17]);
        for leaf in [0, 1, 7, 9, 1024] {
            assert!(recursive_shift(&p, &Integer::one(), leaf).is_err());
        }
        let oversized = poly(vec![super::super::shl(
            Integer::one(),
            MAX_HEIGHT as usize + 1,
        )]);
        assert!(preflight(&oversized, &Integer::one(), 8).is_err());
        let too_many = poly(vec![Integer::one(); MAX_DEGREE + 2]);
        assert!(preflight(&too_many, &Integer::one(), 8).is_err());
        let large_shift = super::super::shl(Integer::one(), 255);
        let costly = poly(vec![Integer::one(); MAX_DEGREE + 1]);
        assert!(preflight(&costly, &large_shift, 8).is_err());
        Ok(())
    }
}
