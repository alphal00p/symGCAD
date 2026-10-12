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
//! two closed-form bounds. The Hungarian method proposes LP potentials and the
//! bound is their checked dual value, so it does not rely on that search.
//!
//! Admission: a plan whose closed-form bound is at most
//! `unconditional_degree_bound` lies in the historical envelope and keeps the
//! historical static caps (and their dynamic image stops). Every other plan,
//! including one brought under that value only by the assignment bound, is
//! admitted only when an a-priori bit bound of every image (Hadamard:
//! |Res| <= ||p(k)||_1^n ||q(k)||_1^m after clearing denominators) fits the
//! per-image cap and, times the image count, the total image cap, so the
//! dynamic resource stops cannot fire for it. The degree bound itself is always
//! proved; nothing here is probabilistic.
//!
//! Images are exact integer specializations followed by the deterministic
//! integer resultant, which decides a vanishing image by an exact gcd test
//! before Lazard-Ducos (see `resultant_segmentation::integer_resultant`). The
//! interpolated result is then checked at one further point through a
//! different route: Symbolica's own specialization and its Brown subresultant
//! resultant over Q (`spot_check`). The check adds no premise; a mismatch is an
//! error.
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
    /// True when the plan lies outside the historical closed-form envelope
    /// and was admitted by the a-priori image size check.
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
    /// Whether the independent spot check found a full-degree point.
    #[serde(default)]
    pub spot_checked: bool,
}

/// Independent exact spot check of a kernel result `result` = Res_x(p, q).
///
/// Candidate points put parameter j (the j-th active variable other than x)
/// at first+(j+1)*t+j for t = 0, 1, ...; the first point where both formal
/// leading coefficients in x stay nonzero is used, so resultant and
/// specialization commute there. At that point Symbolica's own `replace`
/// specializes p and q and its Brown subresultant resultant over Q computes
/// the value; the interpolation and segmentation kernels use neither. A
/// mismatch is an error. Returns false when no candidate in a bounded
/// sequence keeps both degrees (nothing is then claimed), or when an operand
/// is linear: those use the explicit signed identity elsewhere.
pub(crate) fn spot_check(
    p: &Poly,
    q: &Poly,
    variable: usize,
    first: i64,
    result: &Poly,
) -> Result<bool> {
    ensure!(
        p.variables() == q.variables() && result.variables() == p.variables(),
        "spot check variable maps differ"
    );
    let (m, n) = (p.degree(variable), q.degree(variable));
    if m < 2 || n < 2 {
        return Ok(false);
    }
    let parameters: Vec<usize> = (0..p.nvars())
        .filter(|&v| v != variable && (p.degree(v) > 0 || q.degree(v) > 0))
        .collect();
    ensure!(
        (0..result.nvars()).all(|v| parameters.contains(&v) || result.degree(v) == 0),
        "spot check result has extra support"
    );
    let leading = [p.univariate_lcoeff(variable), q.univariate_lcoeff(variable)];
    // Along the candidate line each leading coefficient is a univariate
    // polynomial of at most its total degree: unless it vanishes on the
    // whole line, this many candidates contain a full-degree point.
    let total = |c: &Poly| {
        c.exponents_iter()
            .map(|e| e.iter().map(|&d| u64::from(d)).sum::<u64>())
            .max()
            .unwrap_or(0)
    };
    let candidates = (total(&leading[0]) + total(&leading[1]) + 1).min(256);
    let specialize = |f: &Poly, point: &[(usize, Rational)]| -> Poly {
        point
            .iter()
            .fold(f.clone(), |acc, (v, value)| acc.replace(*v, value))
    };
    for t in 0..candidates as i64 {
        let point = parameters
            .iter()
            .enumerate()
            .map(|(j, &v)| {
                let j = j as i64;
                let value = first
                    .checked_add(t.checked_mul(j + 1).unwrap_or(i64::MAX / 4))
                    .and_then(|x| x.checked_add(j))
                    .ok_or_else(|| anyhow::anyhow!("spot check point overflow"))?;
                Ok((v, Rational::from(value)))
            })
            .collect::<Result<Vec<_>>>()?;
        if leading.iter().any(|c| specialize(c, &point).is_zero()) {
            continue;
        }
        let left = specialize(p, &point);
        let right = specialize(q, &point);
        ensure!(
            left.degree(variable) == m && right.degree(variable) == n,
            "spot check specialization lost the formal degree"
        );
        let expected = left
            .to_univariate_from_univariate(variable)
            .resultant_brown(&right.to_univariate_from_univariate(variable));
        let value = specialize(result, &point);
        ensure!(value.is_constant(), "spot check value is not constant");
        let value = if value.is_zero() {
            Rational::zero()
        } else {
            value.get_constant()
        };
        ensure!(
            value == expected,
            "independent resultant spot check failed at {:?}",
            point.iter().map(|(_, v)| v.to_string()).collect::<Vec<_>>()
        );
        return Ok(true);
    }
    Ok(false)
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

/// Result of a certified assignment bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssignmentBound {
    Finite(u64),
    /// No permutation avoids a structurally zero entry: the determinant is
    /// identically zero.
    StructurallyZero,
}

/// Certified maximum-weight assignment of an n x n matrix whose None entries
/// are structurally zero.
///
/// The classical O(n^3) Hungarian method on negated weights only proposes
/// row/column potentials. By weak LP duality, any U, V with
/// U_i + V_j >= w_ij on every allowed entry bound the weight of every
/// permutation avoiding None entries by sum(U) + sum(V). That inequality is
/// checked here entry by entry, so the returned bound never relies on the
/// Hungarian implementation. Weights are nonnegative, so a negative certified
/// bound proves that no allowed permutation exists. Returns None only if the
/// proposed potentials fail the check (never expected; callers then keep the
/// closed-form bound).
fn certified_maximum_assignment(weights: &[Vec<Option<u64>>]) -> Option<AssignmentBound> {
    let n = weights.len();
    if n == 0 {
        return Some(AssignmentBound::Finite(0));
    }
    // Entry degrees are at most MAX_DEGREE < 2^31 and n <= 1024; the forbidden
    // cost dominates every finite assignment and all sums fit in i128.
    const FORBIDDEN: i128 = 1 << 60;
    let cost = |i: usize, j: usize| -> i128 { weights[i][j].map_or(FORBIDDEN, |w| -i128::from(w)) };
    let mut u = vec![0i128; n + 1];
    let mut v = vec![0i128; n + 1];
    let mut assigned = vec![0usize; n + 1];
    let mut way = vec![0usize; n + 1];
    for row in 1..=n {
        assigned[0] = row;
        let mut column = 0usize;
        let mut minimum = vec![i128::MAX; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[column] = true;
            let current = assigned[column];
            let mut delta = i128::MAX;
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
    // Independent dual check: U_i = -u_i, V_j = -v_j must dominate every
    // allowed weight.
    for i in 1..=n {
        for j in 1..=n {
            if let Some(w) = weights[i - 1][j - 1]
                && -u[i] - v[j] < i128::from(w)
            {
                return None;
            }
        }
    }
    let bound = -(u[1..].iter().sum::<i128>() + v[1..].iter().sum::<i128>());
    Some(if bound < 0 {
        AssignmentBound::StructurallyZero
    } else {
        AssignmentBound::Finite(u64::try_from(bound).ok()?)
    })
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

/// Certified tropical bound on deg_parameter Res_variable(p,q), from the
/// maximum-weight permutation of the Sylvester matrix's entry degrees. Valid in
/// any variable map: other active variables are simply ignored, so this also
/// bounds each parameter separately for multivariate segmentation.
///
/// The bound is the dual value of checked LP potentials (weak duality), so it
/// is proved independently of the Hungarian search that proposes them. Some(0)
/// with a certified structural zero means Res vanishes identically. None means
/// no certified bound (failed check or above MAX_DEGREE): callers keep the
/// closed-form bounds.
pub fn assignment_degree_bound(
    p: &Poly,
    q: &Poly,
    variable: usize,
    parameter: usize,
) -> Result<Option<usize>> {
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
    Ok(match certified_maximum_assignment(&matrix) {
        Some(AssignmentBound::Finite(bound)) if bound <= MAX_DEGREE => {
            Some(usize::try_from(bound)?)
        }
        Some(AssignmentBound::Finite(_)) | None => None,
        // Every determinant monomial vanishes structurally: Res is zero.
        Some(AssignmentBound::StructurallyZero) => Some(0),
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
        assignment_degree_bound(p, q, variable, parameter)?
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
    // The historical envelope is decided by the closed-form bound alone, so
    // it admits exactly what the fixed-cap rule admitted. Everything else,
    // including a bound brought under the threshold only by the assignment
    // bound, needs the a-priori size check and can never hit an image stop.
    let historical = closed_form_bound <= options.unconditional_degree_bound;
    let size_admitted = !historical;
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
        spot_checked: false,
    };
    let started = Instant::now();
    let mut total_bits = 0u64;
    let mut largest_magnitude = 0i64;
    let left_table = IntegerImage::new(p, variable, parameter);
    let right_table = IntegerImage::new(q, variable, parameter);
    // Res(P/Lp, Q/Lq) = Lp^-n Lq^-m Res(P, Q) for formal degrees m, n.
    let scale = Rational::one()
        / Rational::from(
            left_table.denominator.pow(u64::from(n)) * right_table.denominator.pow(u64::from(m)),
        );
    let max_power = left_table
        .parameter_degree
        .max(right_table.parameter_degree) as usize;
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
        largest_magnitude = largest_magnitude.max(magnitude);
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
    // Beyond every sample: an independent route must reproduce the result.
    stats.spot_checked = spot_check(p, q, variable, largest_magnitude + 1, &result)?;
    check(&mut callback)?;
    Ok(Some((result, stats)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{algebra::Algebra, projection::exact_resultant_ducos};

    #[test]
    fn spot_check_accepts_exact_results_and_rejects_altered_ones() {
        let a = Algebra::new(&["s".into(), "x".into(), "t".into()]).unwrap();
        for (p, q) in [
            ("(2/3)*s*x^3+x^2/5+s^2*x+1", "(s+1)*x^2/7-s*x+3"),
            ("x^4+s*t*x^2+t", "(s-t)*x^3+x+s^2"),
        ] {
            let p = a.parse(p).unwrap();
            let q = a.parse(q).unwrap();
            let r = exact_resultant_ducos(&p, &q, 1).unwrap();
            assert!(spot_check(&p, &q, 1, 3, &r).unwrap());
            assert!(
                spot_check(&q, &p, 1, 3, &r).is_ok_and(|checked| checked)
                    == (p.degree(1) * q.degree(1) % 2 == 0)
            );
            assert!(spot_check(&p, &q, 1, 3, &-r.clone()).is_err());
            assert!(spot_check(&p, &q, 1, 3, &r.clone().mul_coeff(Rational::from(2))).is_err());
            let shifted = &r + &a.parse("s+t+1").unwrap();
            assert!(spot_check(&p, &q, 1, 3, &shifted).is_err());
        }
        // A shared factor: the zero result is confirmed through Brown.
        let p = a.parse("(x^2+s)*(x+1)").unwrap();
        let q = a.parse("(x^2+s)*(x-s)").unwrap();
        assert!(spot_check(&p, &q, 1, 3, &p.zero()).unwrap());
        assert!(spot_check(&p, &q, 1, 3, &p.one()).is_err());
        // Linear operands use the explicit identity elsewhere.
        let linear = a.parse("x+s").unwrap();
        assert!(!spot_check(&p, &linear, 1, 3, &p.zero()).unwrap());
        // Leading coefficients vanishing at the first candidates are skipped.
        let p = a.parse("(s-3)*(s-4)*x^2+x+1").unwrap();
        let q = a.parse("x^2+s").unwrap();
        let r = exact_resultant_ducos(&p, &q, 1).unwrap();
        assert!(spot_check(&p, &q, 1, 3, &r).unwrap());
        assert!(spot_check(&p, &q, 1, 3, &(&r + &a.parse("(s-3)*(s-4)").unwrap())).is_err());
        // Extra support is refused outright.
        assert!(spot_check(&p, &q, 1, 3, &a.parse("t").unwrap()).is_err());
    }

    /// Brute-force maximum over permutations avoiding None entries.
    fn brute_force(weights: &[Vec<Option<u64>>]) -> Option<u64> {
        fn walk(
            row: usize,
            used: &mut [bool],
            weights: &[Vec<Option<u64>>],
            sum: u64,
            best: &mut Option<u64>,
        ) {
            if row == weights.len() {
                *best = Some(best.map_or(sum, |b| b.max(sum)));
                return;
            }
            for column in 0..weights.len() {
                if !used[column]
                    && let Some(w) = weights[row][column]
                {
                    used[column] = true;
                    walk(row + 1, used, weights, sum + w, best);
                    used[column] = false;
                }
            }
        }
        let mut best = None;
        walk(0, &mut vec![false; weights.len()], weights, 0, &mut best);
        best
    }

    #[test]
    fn certified_assignment_matches_brute_force_and_certifies_structural_zeros() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = move |bound: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % bound
        };
        let (mut finite, mut zero) = (0, 0);
        for _ in 0..4000 {
            let n = 1 + next(7) as usize;
            let holes = next(5);
            let weights = (0..n)
                .map(|_| {
                    (0..n)
                        .map(|_| (next(8) >= holes).then(|| next(40)))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let certified = certified_maximum_assignment(&weights).expect("dual check");
            match brute_force(&weights) {
                Some(best) => {
                    assert_eq!(certified, AssignmentBound::Finite(best), "{weights:?}");
                    finite += 1;
                }
                None => {
                    assert_eq!(certified, AssignmentBound::StructurallyZero, "{weights:?}");
                    zero += 1;
                }
            }
        }
        assert!(finite > 1000 && zero > 100, "{finite} {zero}");
        // Public bound: x divides both operands, so Sylvester's last column is
        // structurally zero and the certified bound is Some(0).
        let a = Algebra::new(&["s".into(), "x".into()]).unwrap();
        let p = a.parse("x^3+s*x").unwrap();
        let q = a.parse("x^2+s^5*x").unwrap();
        assert_eq!(assignment_degree_bound(&p, &q, 1, 0).unwrap(), Some(0));
    }
}
