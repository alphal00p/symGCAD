//! Deterministic bivariate resultants by exact specialization and interpolation.
//!
//! For degrees m,n in the eliminated axis and total degrees P,Q, the degree
//! in the sole surviving parameter is at most n*P+m*Q-m*n. Every determinant
//! monomial selects n coefficients from the first input and m from the second;
//! their eliminated-axis exponents sum to m*n. Cancellation only lowers this
//! bound. D+1 distinct, full-degree exact specializations determine the result.
//!
//! The callback is checked between exact algebra calls; an individual Symbolica
//! call cannot be preempted here. CLI workers retain their supervisor's hard
//! deadline. This module never accepts partial interpolation or probabilistic
//! candidates, and preserves the complete rational scale and sign.
use crate::{algebra::Poly, projection::exact_resultant_ducos};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fmt, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug)]
pub struct Options {
    pub min_degree_product: u64,
    pub max_elimination_degree: u32,
    pub max_degree_bound: usize,
    pub min_input_terms: usize,
    pub max_input_terms: usize,
    pub max_input_coefficient_bits: u64,
    pub max_image_bits: u64,
    pub max_total_image_bits: u64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            min_degree_product: 64,
            max_elimination_degree: 64,
            max_degree_bound: 1024,
            min_input_terms: 64,
            max_input_terms: 4096,
            max_input_coefficient_bits: 4096,
            max_image_bits: 65536,
            max_total_image_bits: 64 * 1024 * 1024 * 8,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Statistics {
    pub parameter: usize,
    pub degree_bound: usize,
    pub images: usize,
    pub skipped_degree_drops: usize,
    pub maximum_image_bits: u64,
    pub numeric_seconds: f64,
    pub interpolation_seconds: f64,
}

/// A resource or caller cancellation must propagate, never start a fallback.
#[derive(Debug)]
pub struct Stopped(pub String);
impl fmt::Display for Stopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resultant interpolation stopped: {}", self.0)
    }
}
impl std::error::Error for Stopped {}
fn check(callback: &mut impl FnMut() -> Result<()>) -> Result<()> {
    callback().map_err(|e| anyhow::Error::new(Stopped(e.to_string())))
}
fn bits(r: &Rational) -> u64 {
    r.numerator_ref()
        .significant_bits()
        .saturating_add(r.denominator_ref().significant_bits())
}

/// Both bounds follow directly from the Sylvester determinant. Expects exactly
/// the two specified active axes; the caller checks this support condition.
pub fn degree_bound(p: &Poly, q: &Poly, variable: usize, parameter: usize) -> Result<usize> {
    ensure!(
        variable < p.nvars() && parameter < p.nvars() && variable != parameter,
        "invalid resultant axes"
    );
    ensure!(
        p.variables() == q.variables(),
        "resultant variable maps differ"
    );
    ensure!(
        (0..p.nvars())
            .all(|v| v == variable || v == parameter || (p.degree(v) == 0 && q.degree(v) == 0)),
        "degree bound requires bivariate support"
    );
    let m = u64::from(p.degree(variable));
    let n = u64::from(q.degree(variable));
    let total = |f: &Poly| {
        f.exponents_iter()
            .map(|e| e.iter().map(|&d| u64::from(d)).sum::<u64>())
            .max()
            .unwrap_or(0)
    };
    let rectangular = m
        .checked_mul(u64::from(q.degree(parameter)))
        .and_then(|d| d.checked_add(n.checked_mul(u64::from(p.degree(parameter)))?));
    let homogeneous = n
        .checked_mul(total(p))
        .and_then(|d| d.checked_add(m.checked_mul(total(q))?))
        .and_then(|d| d.checked_sub(m.checked_mul(n)?));
    let bound = rectangular
        .zip(homogeneous)
        .ok_or_else(|| anyhow::anyhow!("resultant degree bound overflow"))?;
    Ok(usize::try_from(bound.0.min(bound.1))?)
}

/// None means a static applicability cap declined the strategy, permitting the
/// ordinary deterministic resultant. Errors, including cancellation and image
/// resource caps, must propagate without starting that fallback.
pub fn try_resultant(
    p: &Poly,
    q: &Poly,
    variable: usize,
    options: &Options,
    mut callback: impl FnMut() -> Result<()>,
) -> Result<Option<(Poly, Statistics)>> {
    ensure!(
        p.variables() == q.variables(),
        "resultant variable maps differ"
    );
    ensure!(variable < p.nvars(), "resultant variable out of range");
    check(&mut callback)?;
    let m = p.degree(variable);
    let n = q.degree(variable);
    if m < 2
        || n < 2
        || m > options.max_elimination_degree
        || n > options.max_elimination_degree
        || u64::from(m) * u64::from(n) < options.min_degree_product
        || p.nterms().saturating_add(q.nterms()) < options.min_input_terms
        || p.nterms().saturating_add(q.nterms()) > options.max_input_terms
        || p.coefficients
            .iter()
            .chain(&q.coefficients)
            .any(|c| bits(c) > options.max_input_coefficient_bits)
    {
        return Ok(None);
    }
    let parameters: Vec<_> = (0..p.nvars())
        .filter(|&v| v != variable && (p.degree(v) > 0 || q.degree(v) > 0))
        .collect();
    if parameters.len() != 1 {
        return Ok(None);
    }
    let parameter = parameters[0];
    let bound = degree_bound(p, q, variable, parameter)?;
    if bound > options.max_degree_bound {
        return Ok(None);
    }
    let required = bound
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("image count overflow"))?;
    let max_skips = (p.degree(parameter) as usize)
        .checked_add(q.degree(parameter) as usize)
        .ok_or_else(|| anyhow::anyhow!("degree-drop count overflow"))?;
    let attempts = required
        .checked_add(max_skips)
        .ok_or_else(|| anyhow::anyhow!("sample count overflow"))?;
    let mut points = Vec::with_capacity(required);
    let mut values = Vec::with_capacity(required);
    let mut stats = Statistics {
        parameter,
        degree_bound: bound,
        images: 0,
        skipped_degree_drops: 0,
        maximum_image_bits: 0,
        numeric_seconds: 0.0,
        interpolation_seconds: 0.0,
    };
    let started = Instant::now();
    let mut total_bits = 0u64;
    for attempt in 0..attempts {
        if points.len() == required {
            break;
        }
        check(&mut callback)?;
        let magnitude = i64::try_from(attempt.div_ceil(2))?;
        let k = if attempt % 2 == 1 {
            magnitude
        } else {
            -magnitude
        };
        let point = Rational::from(k);
        let left = p.replace(parameter, &point);
        let right = q.replace(parameter, &point);
        if left.degree(variable) != m || right.degree(variable) != n {
            stats.skipped_degree_drops += 1;
            continue;
        }
        let image = exact_resultant_ducos(&left, &right, variable)?;
        check(&mut callback)?;
        ensure!(image.is_constant(), "numeric resultant is not constant");
        let value = image.get_constant();
        let size = bits(&value);
        total_bits = total_bits.saturating_add(size);
        if size > options.max_image_bits || total_bits > options.max_total_image_bits {
            return Err(Stopped("exact numeric image storage cap".into()).into());
        }
        stats.maximum_image_bits = stats.maximum_image_bits.max(size);
        points.push(point);
        // Symbolica can return a zero image with no variables. Restore the
        // original map before Newton interpolation extends its first image.
        values.push(p.constant(value));
    }
    ensure!(
        points.len() == required,
        "insufficient full-degree specializations"
    );
    stats.images = points.len();
    stats.numeric_seconds = started.elapsed().as_secs_f64();
    check(&mut callback)?;
    let started = Instant::now();
    let mut result = Poly::newton_interpolation(&points, &values, parameter);
    if result.is_constant() {
        result = p.constant(result.get_constant());
    }
    stats.interpolation_seconds = started.elapsed().as_secs_f64();
    check(&mut callback)?;
    ensure!(
        result.variables() == p.variables(),
        "interpolation changed the variable map"
    );
    ensure!(
        result.degree(parameter) as usize <= bound && result.nterms() <= required,
        "interpolated resultant exceeds proved support bound"
    );
    ensure!(
        (0..result.nvars()).all(|v| v == parameter || result.degree(v) == 0),
        "interpolated resultant has extra support"
    );
    Ok(Some((result, stats)))
}
