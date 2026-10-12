//! Deterministic bivariate resultants by exact specialization and interpolation.
//!
//! For degrees m,n in the eliminated axis and total degrees P,Q, the degree
//! in the sole surviving parameter is at most n*P+m*Q-m*n. Every determinant
//! monomial selects n coefficients from the first input and m from the second;
//! their eliminated-axis exponents sum to m*n. Cancellation only lowers this
//! bound. D+1 distinct, full-degree exact specializations determine the result.
//!
//! A second proved bound is the tropical (assignment) bound of the Sylvester
//! matrix: det S = sum over permutations of signed entry products, so its
//! parameter degree is at most the maximum over permutations of the summed
//! entry degrees, a maximum-weight perfect assignment. A permutation through a
//! structurally zero entry contributes nothing. This bound never exceeds the
//! two closed-form bounds and is computed exactly by the Hungarian method.
//!
//! Admission is size-derived. Bounds up to `unconditional_degree_bound` keep
//! the historical static caps. A larger bound is admitted only when an a-priori
//! bit bound of every image (Hadamard: |Res| <= ||p(k)||_1^n ||q(k)||_1^m after
//! clearing denominators) fits the per-image cap and, times the image count,
//! the total image cap, so the dynamic resource stops cannot fire afterwards.
//! The degree bound itself is always proved; nothing here is probabilistic.
//!
//! The callback is checked between exact algebra calls; an individual Symbolica
//! call cannot be preempted here. CLI workers retain their supervisor's hard
//! deadline. This module never accepts partial interpolation or probabilistic
//! candidates, and preserves the complete rational scale and sign.
use crate::algebra::{MAX_DEGREE, Poly};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fmt, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug)]
pub struct Options {
    pub min_degree_product: u64,
    pub max_elimination_degree: u32,
    /// Bounds up to this value are admitted on the historical static caps.
    pub unconditional_degree_bound: usize,
    /// Hard ceiling for larger bounds, which also need the size admission.
    pub max_degree_bound: usize,
    /// Use the exact Sylvester assignment bound in addition to the closed forms.
    pub assignment_bound: bool,
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
            unconditional_degree_bound: 1024,
            max_degree_bound: 65536,
            assignment_bound: true,
            min_input_terms: 64,
            max_input_terms: 4096,
            max_input_coefficient_bits: 4096,
            max_image_bits: 65536,
            max_total_image_bits: 64 * 1024 * 1024 * 8,
        }
    }
}
impl Options {
    /// The historical fixed-cap envelope (closed-form bound, cap 1024), kept
    /// for profiling comparisons and explicit opt-outs.
    pub fn legacy() -> Self {
        Self {
            max_degree_bound: 1024,
            assignment_bound: false,
            ..Self::default()
        }
    }
}

/// A static admission decision; constructing it performs no resultant work.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub parameter: usize,
    /// The proved bound used for the image count.
    pub degree_bound: usize,
    pub closed_form_bound: usize,
    pub assignment_bound: Option<usize>,
    /// A-priori bound on the bits of every image (numerator plus denominator).
    pub image_bit_bound: u64,
    /// True when admission needed the size-derived rule above the
    /// unconditional bound.
    pub size_admitted: bool,
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
    #[serde(default)]
    pub closed_form_bound: usize,
    #[serde(default)]
    pub size_admitted: bool,
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

/// Maximum-weight perfect assignment of an n x n matrix whose None entries are
/// structurally zero. Returns None when every permutation meets such an entry.
/// Classical O(n^3) Hungarian method with potentials on negated weights.
fn maximum_assignment(weights: &[Vec<Option<u64>>]) -> Option<u64> {
    let n = weights.len();
    if n == 0 {
        return Some(0);
    }
    // Entry degrees are at most MAX_DEGREE < 2^31 and n <= 129, so all sums
    // of finite costs stay far below the forbidden cost and i64 range.
    const FORBIDDEN: i64 = 1 << 50;
    let cost = |i: usize, j: usize| -> i64 {
        weights[i][j].map_or(FORBIDDEN, |w| -(w as i64))
    };
    let mut u = vec![0i64; n + 1];
    let mut v = vec![0i64; n + 1];
    let mut assigned = vec![0usize; n + 1];
    let mut way = vec![0usize; n + 1];
    for row in 1..=n {
        assigned[0] = row;
        let mut column = 0usize;
        let mut minimum = vec![i64::MAX; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[column] = true;
            let current = assigned[column];
            let mut delta = i64::MAX;
            let mut next = 0usize;
            for j in 1..=n {
                if !used[j] {
                    let reduced = cost(current - 1, j - 1) - u[current] - v[j];
                    if reduced < minimum[j] {
                        minimum[j] = reduced;
                        way[j] = column;
                    }
                    if minimum[j] < delta {
                        delta = minimum[j];
                        next = j;
                    }
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[assigned[j]] += delta;
                    v[j] -= delta;
                } else {
                    minimum[j] -= delta;
                }
            }
            column = next;
            if assigned[column] == 0 {
                break;
            }
        }
        loop {
            let previous = way[column];
            assigned[column] = assigned[previous];
            column = previous;
            if column == 0 {
                break;
            }
        }
    }
    let mut total = 0u64;
    for j in 1..=n {
        let w = weights[assigned[j] - 1][j - 1]?;
        total += w;
    }
    Some(total)
}

/// Degree of every eliminated-axis coefficient in `parameter`, indexed by the
/// eliminated-axis exponent; None for an absent coefficient.
fn coefficient_degrees(p: &Poly, variable: usize, parameter: usize) -> Vec<Option<u64>> {
    let mut degrees = vec![None; p.degree(variable) as usize + 1];
    for e in p.exponents_iter() {
        let slot = &mut degrees[e[variable] as usize];
        *slot = Some(slot.unwrap_or(0).max(u64::from(e[parameter])));
    }
    degrees
}

/// Exact tropical bound on deg_parameter Res_variable(p,q), from the
/// maximum-weight permutation of the Sylvester matrix's entry degrees. Valid in
/// any variable map: other active variables are simply ignored, so this also
/// bounds each parameter separately for multivariate segmentation.
pub fn assignment_degree_bound(
    p: &Poly,
    q: &Poly,
    variable: usize,
    parameter: usize,
) -> Result<usize> {
    ensure!(
        variable < p.nvars() && parameter < p.nvars() && variable != parameter,
        "invalid resultant axes"
    );
    ensure!(
        p.variables() == q.variables(),
        "resultant variable maps differ"
    );
    let m = p.degree(variable) as usize;
    let n = q.degree(variable) as usize;
    ensure!(
        m + n <= 1024,
        "assignment bound matrix size exceeds supported range"
    );
    let a = coefficient_degrees(p, variable, parameter);
    let b = coefficient_degrees(q, variable, parameter);
    let size = m + n;
    let mut matrix = vec![vec![None; size]; size];
    // Rows 0..n hold p's coefficients a_m..a_0 shifted right by the row;
    // rows n..n+m hold q's coefficients b_n..b_0, as in Sylvester's matrix.
    for row in 0..n {
        for k in 0..=m {
            matrix[row][row + (m - k)] = a[k];
        }
    }
    for row in 0..m {
        for k in 0..=n {
            matrix[n + row][row + (n - k)] = b[k];
        }
    }
    Ok(match maximum_assignment(&matrix) {
        Some(bound) => usize::try_from(bound.min(MAX_DEGREE))?,
        // Every determinant monomial vanishes structurally: Res is zero.
        None => 0,
    })
}

/// Exact a-priori bit bound for every numeric image at |point| <= magnitude.
/// After clearing denominators L, ||L p(k)||_1 <= ||L p||_1 |k|^deg_y(p), and
/// Hadamard's bound gives |Res(Lp p(k), Lq q(k))| <= ||.||_1^n ||.||_1^m. The
/// image's denominator divides Lp^n Lq^m.
fn image_bit_bound(p: &Poly, q: &Poly, variable: usize, parameter: usize, magnitude: u64) -> u64 {
    let point_bits = u64::from(64 - magnitude.max(1).leading_zeros());
    let polynomial_bits = |f: &Poly| {
        let mut denominator = Integer::one();
        for c in &f.coefficients {
            denominator = denominator.lcm(c.denominator_ref());
        }
        let cleared = f.clone().mul_coeff(Rational::from(denominator.clone()));
        let norm = cleared
            .coefficients
            .iter()
            .fold(Integer::zero(), |sum, c| sum + c.numerator_ref().abs());
        (
            norm.significant_bits()
                .saturating_add(u64::from(f.degree(parameter)).saturating_mul(point_bits)),
            denominator.significant_bits(),
        )
    };
    let (p_bits, p_denominator) = polynomial_bits(p);
    let (q_bits, q_denominator) = polynomial_bits(q);
    let m = u64::from(p.degree(variable));
    let n = u64::from(q.degree(variable));
    n.saturating_mul(p_bits.saturating_add(p_denominator))
        .saturating_add(m.saturating_mul(q_bits.saturating_add(q_denominator)))
        .saturating_add(2)
}

/// Integer coefficient table of L*f, grouped by eliminated-axis exponent,
/// with L the positive common denominator. Specializing the single parameter
/// at an integer point is then a dot product with its powers.
struct IntegerImage {
    denominator: Integer,
    terms: Vec<Vec<(usize, Integer)>>,
    parameter_degree: u32,
}

impl IntegerImage {
    fn new(f: &Poly, variable: usize, parameter: usize) -> Self {
        let mut denominator = Integer::one();
        for c in &f.coefficients {
            denominator = denominator.lcm(c.denominator_ref());
        }
        let cleared = f.clone().mul_coeff(Rational::from(denominator.clone()));
        let mut terms = vec![Vec::new(); f.degree(variable) as usize + 1];
        for (c, e) in cleared.coefficients.iter().zip(cleared.exponents_iter()) {
            debug_assert!(c.denominator_ref().is_one());
            terms[e[variable] as usize].push((e[parameter] as usize, c.numerator()));
        }
        Self {
            denominator,
            terms,
            parameter_degree: f.degree(parameter),
        }
    }

    /// None if the formal leading coefficient vanishes at this point.
    fn evaluate(
        &self,
        powers: &[Integer],
        x: &std::sync::Arc<PolyVariable>,
    ) -> Option<UnivariatePolynomial<Z>> {
        let coefficients = self
            .terms
            .iter()
            .map(|terms| {
                terms
                    .iter()
                    .fold(Integer::zero(), |sum, (e, c)| sum + c * &powers[*e])
            })
            .collect::<Vec<_>>();
        if coefficients.last().is_none_or(|c| c.is_zero()) {
            return None;
        }
        Some(UnivariatePolynomial::from_coefficients(
            &Z,
            coefficients,
            x.clone(),
        ))
    }
}

/// Static applicability, with no resultant work. None means a cap declined.
pub fn plan(p: &Poly, q: &Poly, variable: usize, options: &Options) -> Result<Option<Plan>> {
    ensure!(
        p.variables() == q.variables(),
        "resultant variable maps differ"
    );
    ensure!(variable < p.nvars(), "resultant variable out of range");
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
    let closed_form_bound = degree_bound(p, q, variable, parameter)?;
    let assignment = if options.assignment_bound {
        Some(assignment_degree_bound(p, q, variable, parameter)?)
    } else {
        None
    };
    let bound = assignment.map_or(closed_form_bound, |a| a.min(closed_form_bound));
    if bound > options.max_degree_bound {
        return Ok(None);
    }
    let max_skips = u64::from(p.degree(parameter)) + u64::from(q.degree(parameter));
    let attempts = (bound as u64).saturating_add(1).saturating_add(max_skips);
    let image_bit_bound = image_bit_bound(p, q, variable, parameter, attempts.div_ceil(2));
    let size_admitted = bound > options.unconditional_degree_bound;
    if size_admitted
        && (image_bit_bound > options.max_image_bits
            || image_bit_bound.saturating_mul(bound as u64 + 1) > options.max_total_image_bits)
    {
        return Ok(None);
    }
    Ok(Some(Plan {
        parameter,
        degree_bound: bound,
        closed_form_bound,
        assignment_bound: assignment,
        image_bit_bound,
        size_admitted,
    }))
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
    let Some(plan) = plan(p, q, variable, options)? else {
        return Ok(None);
    };
    let (m, n) = (p.degree(variable), q.degree(variable));
    let parameter = plan.parameter;
    let bound = plan.degree_bound;
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
        closed_form_bound: plan.closed_form_bound,
        size_admitted: plan.size_admitted,
    };
    let started = Instant::now();
    let mut total_bits = 0u64;
    let left_table = IntegerImage::new(p, variable, parameter);
    let right_table = IntegerImage::new(q, variable, parameter);
    // Res(P/Lp, Q/Lq) = Lp^-n Lq^-m Res(P, Q) for formal degrees m, n.
    let scale = Rational::one()
        / Rational::from(
            left_table.denominator.pow(u64::from(n)) * right_table.denominator.pow(u64::from(m)),
        );
    let max_power = left_table.parameter_degree.max(right_table.parameter_degree) as usize;
    let x = std::sync::Arc::new(p.variables()[variable].clone());
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
        let mut powers = Vec::with_capacity(max_power + 1);
        powers.push(Integer::one());
        for e in 1..=max_power {
            let next = &powers[e - 1] * &Integer::from(k);
            powers.push(next);
        }
        // Exact integer specializations; a vanishing formal leading
        // coefficient is a degree drop and the point is skipped.
        let (Some(left), Some(right)) = (
            left_table.evaluate(&powers, &x),
            right_table.evaluate(&powers, &x),
        ) else {
            stats.skipped_degree_drops += 1;
            continue;
        };
        ensure!(
            left.degree() == m as usize && right.degree() == n as usize,
            "integer specialization degree mismatch"
        );
        // Deterministic Lazard-Ducos over Z (both degrees are at least two).
        let value = Rational::from(crate::resultant_segmentation::integer_resultant(
            &left, &right,
        )) * &scale;
        check(&mut callback)?;
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
