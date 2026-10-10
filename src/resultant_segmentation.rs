//! Exact multivariate resultants by bounded binary segmentation.
//!
//! After clearing positive coefficient denominators, a Sylvester determinant
//! has coefficient l1 norm at most ||p||_1^n ||q||_1^m: expand the determinant,
//! discard signs, and bound the permanent by the product of row l1 norms.
//! Its degree in parameter y is bounded both by n deg_y(p)+m deg_y(q) and by
//! n max(e_x+e_y in p)+m max(e_x+e_y in q)-mn. The latter follows because
//! every determinant monomial has total eliminated-axis weight mn.
//!
//! Widening that box to include input support gives an injective mixed-radix
//! monomial code. Evaluate its digits at B=2^k with B>2 times both the output
//! coefficient bound and every input coefficient. No nonzero leading input
//! coefficient vanishes under this encoding. The deterministic integer
//! resultant is therefore the encoding of the desired polynomial; centered
//! block decoding recovers it uniquely, including sign and rational scale.
//!
//! Static caps decline this optional strategy. Once execution begins, caller
//! cancellation, resource stops and algebraic inconsistencies propagate, never
//! silently starting another expensive backend. Callbacks cannot preempt an
//! individual integer resultant: supervised workers remain necessary to bound
//! its time and intermediate memory, which output size alone does not bound.
use crate::{
    algebra::{MAX_DEGREE, Poly},
    resultant_interpolation::Stopped,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug)]
pub struct Options {
    pub min_degree_product: u64,
    pub min_input_terms: usize,
    pub max_input_terms: usize,
    pub min_parameters: usize,
    pub max_parameters: usize,
    pub max_variables: usize,
    pub max_elimination_degree: u32,
    pub max_input_coefficient_bits: u64,
    pub max_denominator_bits: u64,
    pub max_cleared_coefficient_bits: u64,
    pub max_coefficient_bound_bits: u64,
    pub max_digits: usize,
    pub max_output_bits: usize,
    pub max_total_input_bits: usize,
    pub max_output_terms: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            min_degree_product: 60,
            min_input_terms: 64,
            max_input_terms: 8192,
            min_parameters: 2,
            max_parameters: 2,
            max_variables: 128,
            max_elimination_degree: 64,
            max_input_coefficient_bits: 4096,
            max_denominator_bits: 16384,
            max_cleared_coefficient_bits: 32768,
            max_coefficient_bound_bits: 1_000_000,
            max_digits: 65536,
            // Bounds the encoded output, not intermediate native memory.
            // Callers must still supervise the whole operation's resources.
            max_output_bits: 256_000_000,
            max_total_input_bits: 512_000_000,
            max_output_terms: 65536,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub parameters: Vec<usize>,
    pub degree_bounds: Vec<usize>,
    pub weights: Vec<usize>,
    pub digits: usize,
    pub digit_bits: usize,
    pub output_bit_bound: usize,
    pub total_input_bit_bound: usize,
    pub coefficient_bound_bits: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Statistics {
    pub plan: Plan,
    pub encoding_seconds: f64,
    pub resultant_seconds: f64,
    pub decoding_seconds: f64,
    pub numeric_result_bits: u64,
    pub output_terms: usize,
}

fn check(callback: &mut impl FnMut() -> Result<()>) -> Result<()> {
    callback().map_err(|e| anyhow::Error::new(Stopped(e.to_string())))
}
fn stopped(message: &str) -> anyhow::Error {
    anyhow::Error::new(Stopped(format!("binary segmentation: {message}")))
}
fn coefficient_bits(c: &Rational) -> u64 {
    c.numerator_ref()
        .significant_bits()
        .saturating_add(c.denominator_ref().significant_bits())
}

// Explicit arbitrary precision also works with the pristine pinned backend,
// whose small-representation Integer left shift has an optional upstream fix.
fn shift(value: Integer, bits: usize) -> Integer {
    Integer::from(value.to_multi_prec() << bits)
}

struct Prepared {
    left: Poly,
    right: Poly,
    left_denominator: Integer,
    right_denominator: Integer,
    bound: Integer,
    plan: Plan,
}

fn clear(
    p: &Poly,
    options: &Options,
    callback: &mut impl FnMut() -> Result<()>,
) -> Result<Option<(Poly, Integer)>> {
    let mut denominator = Integer::one();
    for c in &p.coefficients {
        check(callback)?;
        denominator = denominator.lcm(c.denominator_ref());
        if denominator.significant_bits() > options.max_denominator_bits {
            return Ok(None);
        }
    }
    // Bound each multiplication before constructing the cleared polynomial.
    if p.coefficients.iter().any(|c| {
        c.numerator_ref()
            .significant_bits()
            .saturating_add(denominator.significant_bits())
            > options.max_cleared_coefficient_bits
    }) {
        return Ok(None);
    }
    let cleared = p.clone().mul_coeff(Rational::from(denominator.clone()));
    ensure!(
        cleared
            .coefficients
            .iter()
            .all(|c| c.denominator_ref().is_one()),
        "denominator clearing failed"
    );
    Ok(Some((cleared, denominator)))
}

fn code(exponents: &[u32], plan: &Plan) -> Result<usize> {
    ensure!(
        exponents.len() == plan.weights.len(),
        "encoding variable map differs"
    );
    let mut result = 0usize;
    for (&e, &weight) in exponents.iter().zip(&plan.weights) {
        result = result
            .checked_add(
                (e as usize)
                    .checked_mul(weight)
                    .ok_or_else(|| anyhow::anyhow!("encoding exponent overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("encoding exponent overflow"))?;
    }
    ensure!(result < plan.digits, "encoding exponent exceeds box");
    Ok(result)
}

fn prepare(
    p: &Poly,
    q: &Poly,
    x: usize,
    options: &Options,
    callback: &mut impl FnMut() -> Result<()>,
) -> Result<Option<Prepared>> {
    ensure!(
        p.variables() == q.variables(),
        "resultant variable maps differ"
    );
    ensure!(x < p.nvars(), "resultant variable out of range");
    check(callback)?;
    let m = p.degree(x) as usize;
    let n = q.degree(x) as usize;
    let terms = p.nterms().saturating_add(q.nterms());
    if m < 2
        || n < 2
        || m > options.max_elimination_degree as usize
        || n > options.max_elimination_degree as usize
        || (m as u64) * (n as u64) < options.min_degree_product
        || terms < options.min_input_terms
        || terms > options.max_input_terms
        || p.nvars() > options.max_variables
        || p.coefficients
            .iter()
            .chain(&q.coefficients)
            .any(|c| coefficient_bits(c) > options.max_input_coefficient_bits)
    {
        return Ok(None);
    }
    let parameters: Vec<_> = (0..p.nvars())
        .filter(|&i| i != x && (p.degree(i) > 0 || q.degree(i) > 0))
        .collect();
    if parameters.len() < options.min_parameters || parameters.len() > options.max_parameters {
        return Ok(None);
    }
    let mut degrees = vec![0usize; p.nvars()];
    for &i in &parameters {
        let weighted = |f: &Poly| {
            f.exponents_iter()
                .map(|e| u64::from(e[x]) + u64::from(e[i]))
                .max()
                .unwrap_or(0)
        };
        let rectangular = (n as u64)
            .checked_mul(u64::from(p.degree(i)))
            .and_then(|a| a.checked_add((m as u64).checked_mul(u64::from(q.degree(i)))?));
        let diagonal = (n as u64)
            .checked_mul(weighted(p))
            .and_then(|a| a.checked_add((m as u64).checked_mul(weighted(q))?))
            .and_then(|a| a.checked_sub((m as u64).checked_mul(n as u64)?));
        let Some((rectangular, diagonal)) = rectangular.zip(diagonal) else {
            return Ok(None);
        };
        let d = rectangular
            .min(diagonal)
            .max(u64::from(p.degree(i)))
            .max(u64::from(q.degree(i)));
        if d > MAX_DEGREE {
            return Ok(None);
        }
        degrees[i] = usize::try_from(d)?;
    }
    let mut weights = vec![0usize; p.nvars()];
    let mut digits = 1usize;
    for &i in parameters.iter().rev() {
        weights[i] = digits;
        let Some(next) = digits.checked_mul(degrees[i] + 1) else {
            return Ok(None);
        };
        digits = next;
        if digits > options.max_digits {
            return Ok(None);
        }
    }
    let Some((left, left_denominator)) = clear(p, options, callback)? else {
        return Ok(None);
    };
    let Some((right, right_denominator)) = clear(q, options, callback)? else {
        return Ok(None);
    };
    let norm = |f: &Poly| {
        f.coefficients
            .iter()
            .fold(Integer::zero(), |sum, c| sum + c.numerator_ref().abs())
    };
    let pn = norm(&left);
    let qn = norm(&right);
    let estimated_bits = pn
        .significant_bits()
        .checked_mul(n as u64)
        .and_then(|a| a.checked_add(qn.significant_bits().checked_mul(m as u64)?));
    if estimated_bits.is_none_or(|b| b > options.max_coefficient_bound_bits) {
        return Ok(None);
    }
    let bound = pn.pow(n as u64) * qn.pow(m as u64);
    let input_bound = left
        .coefficients
        .iter()
        .chain(&right.coefficients)
        .map(|c| c.numerator_ref().abs())
        .max()
        .unwrap_or_else(Integer::zero);
    let Some(digit_bits) =
        usize::try_from(bound.clone().max(input_bound).significant_bits())?.checked_add(1)
    else {
        return Ok(None);
    };
    let Some(output_bit_bound) = digits.checked_mul(digit_bits) else {
        return Ok(None);
    };
    if output_bit_bound > options.max_output_bits {
        return Ok(None);
    }
    let mut plan = Plan {
        parameters,
        degree_bounds: degrees,
        weights,
        digits,
        digit_bits,
        output_bit_bound,
        total_input_bit_bound: 0,
        coefficient_bound_bits: bound.significant_bits(),
    };
    // Every encoded coefficient occupies at most (highest digit+1)*k bits,
    // since all input digits are strict centered digits. Cap the complete
    // dense univariate input allocation, not just one output integer.
    for input in [&left, &right] {
        let mut coefficient_bounds = vec![0usize; input.degree(x) as usize + 1];
        for e in input.exponents_iter() {
            let bits = (code(e, &plan)? + 1)
                .checked_mul(digit_bits)
                .ok_or_else(|| anyhow::anyhow!("encoded input bit bound overflow"))?;
            coefficient_bounds[e[x] as usize] = coefficient_bounds[e[x] as usize].max(bits);
        }
        for bits in coefficient_bounds {
            let Some(total) = plan.total_input_bit_bound.checked_add(bits) else {
                return Ok(None);
            };
            plan.total_input_bit_bound = total;
            if total > options.max_total_input_bits {
                return Ok(None);
            }
        }
    }
    Ok(Some(Prepared {
        left,
        right,
        left_denominator,
        right_denominator,
        bound,
        plan,
    }))
}

/// Inspect bounded applicability without constructing the encoded integers.
pub fn plan(p: &Poly, q: &Poly, variable: usize, options: &Options) -> Result<Option<Plan>> {
    Ok(prepare(p, q, variable, options, &mut || Ok(()))?.map(|p| p.plan))
}

fn encode(
    p: &Poly,
    x: usize,
    plan: &Plan,
    callback: &mut impl FnMut() -> Result<()>,
) -> Result<UnivariatePolynomial<Z>> {
    let mut terms = vec![Vec::<(usize, Integer)>::new(); p.degree(x) as usize + 1];
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        ensure!(
            c.denominator_ref().is_one(),
            "encoding requires integer coefficients"
        );
        terms[e[x] as usize].push((code(e, plan)?, c.numerator()));
    }
    let mut coefficients = Vec::with_capacity(terms.len());
    for mut terms in terms {
        check(callback)?;
        terms.sort_unstable_by_key(|(e, _)| *e);
        let mut previous = terms.last().map(|(e, _)| *e).unwrap_or(0);
        let mut value = Integer::zero();
        for (e, c) in terms.into_iter().rev() {
            value = shift(
                value,
                (previous - e)
                    .checked_mul(plan.digit_bits)
                    .ok_or_else(|| anyhow::anyhow!("encoding shift overflow"))?,
            );
            value += c;
            previous = e;
        }
        coefficients.push(shift(
            value,
            previous
                .checked_mul(plan.digit_bits)
                .ok_or_else(|| anyhow::anyhow!("encoding shift overflow"))?,
        ));
    }
    let result = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(p.variables()[x].clone()),
    );
    ensure!(
        result.degree() == p.degree(x) as usize && !result.is_zero(),
        "encoding lost leading degree"
    );
    Ok(result)
}

fn integer_resultant(p: &UnivariatePolynomial<Z>, q: &UnivariatePolynomial<Z>) -> Integer {
    fn linear_second(p: &UnivariatePolynomial<Z>, q: &UnivariatePolynomial<Z>) -> Integer {
        let neg_constant = -q.coefficients()[0].clone();
        let leading = &q.coefficients()[1];
        let mut power = leading.clone();
        let mut result = p.lcoeff();
        for c in p.coefficients()[..p.degree()].iter().rev() {
            result = result * &neg_constant + c * &power;
            power *= leading;
        }
        if p.degree() % 2 == 1 { -result } else { result }
    }
    if q.degree() == 1 {
        return linear_second(p, q);
    }
    if p.degree() == 1 {
        let result = linear_second(q, p);
        return if q.degree() % 2 == 1 { -result } else { result };
    }
    // Z is Euclidean: this is deterministic Lazard-Ducos, not the separate
    // multivariate auto/Monte Carlo CRT implementation.
    p.resultant(q)
}

fn decode(
    value: Integer,
    offset: usize,
    count: usize,
    plan: &Plan,
    bound: &Integer,
    output: &mut Poly,
    max_terms: usize,
    callback: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    check(callback)?;
    if value.is_zero() {
        return Ok(());
    }
    if count == 1 {
        ensure!(
            value.abs() <= *bound,
            "decoded coefficient exceeds certified bound"
        );
        if output.nterms() >= max_terms {
            return Err(stopped("decoded term cap"));
        }
        let mut exponents = vec![0u32; output.nvars()];
        let mut index = offset;
        for &i in &plan.parameters {
            exponents[i] = u32::try_from(index / plan.weights[i])?;
            ensure!(
                exponents[i] as usize <= plan.degree_bounds[i],
                "decoded exponent exceeds bound"
            );
            index %= plan.weights[i];
        }
        ensure!(index == 0, "invalid decoded exponent");
        output.append_monomial_back(Rational::from(value), &exponents);
        return Ok(());
    }
    let low_count = count / 2;
    let radix = shift(
        Integer::one(),
        low_count
            .checked_mul(plan.digit_bits)
            .ok_or_else(|| anyhow::anyhow!("decode shift overflow"))?,
    );
    let (mut high, mut low) = value.quot_rem(&radix);
    // Symbolica quot_rem uses Euclidean remainder for this positive radix.
    if &low + &low >= radix {
        low -= &radix;
        high += Integer::one();
    }
    decode(
        low, offset, low_count, plan, bound, output, max_terms, callback,
    )?;
    decode(
        high,
        offset + low_count,
        count - low_count,
        plan,
        bound,
        output,
        max_terms,
        callback,
    )
}

/// Explicit optional entry point. None is a static decline; errors must not
/// trigger an expensive fallback. Exact rational scale and variable map remain
/// unchanged. An enclosing worker must bound the native integer operation.
pub fn try_resultant(
    p: &Poly,
    q: &Poly,
    variable: usize,
    options: &Options,
    mut callback: impl FnMut() -> Result<()>,
) -> Result<Option<(Poly, Statistics)>> {
    let started = Instant::now();
    let Some(prepared) = prepare(p, q, variable, options, &mut callback)? else {
        return Ok(None);
    };
    let Prepared {
        left,
        right,
        left_denominator,
        right_denominator,
        bound,
        plan,
    } = prepared;
    let ep = encode(&left, variable, &plan, &mut callback)?;
    let eq = encode(&right, variable, &plan, &mut callback)?;
    let encoding_seconds = started.elapsed().as_secs_f64();
    check(&mut callback)?;
    let numeric_started = Instant::now();
    let encoded = integer_resultant(&ep, &eq);
    let resultant_seconds = numeric_started.elapsed().as_secs_f64();
    check(&mut callback)?;
    ensure!(
        encoded.significant_bits() <= plan.output_bit_bound as u64,
        "numeric resultant exceeds certified output size"
    );
    let decoding_started = Instant::now();
    let mut result = left.zero();
    decode(
        encoded.clone(),
        0,
        plan.digits,
        &plan,
        &bound,
        &mut result,
        options.max_output_terms,
        &mut callback,
    )?;
    let reencoded = if result.is_zero() {
        Integer::zero()
    } else {
        encode(&result, variable, &plan, &mut callback)?.get_constant()
    };
    ensure!(
        reencoded == encoded,
        "resultant encoding reconstruction failed"
    );
    let scale = left_denominator.pow(q.degree(variable) as u64)
        * right_denominator.pow(p.degree(variable) as u64);
    result = result.mul_coeff(Rational::one() / Rational::from(scale));
    check(&mut callback)?;
    ensure!(
        result.variables() == p.variables() && result.degree(variable) == 0,
        "decoded resultant map or support changed"
    );
    let statistics = Statistics {
        plan,
        encoding_seconds,
        resultant_seconds,
        decoding_seconds: decoding_started.elapsed().as_secs_f64(),
        numeric_result_bits: encoded.significant_bits(),
        output_terms: result.nterms(),
    };
    Ok(Some((result, statistics)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{algebra::Algebra, projection::exact_resultant_ducos};

    fn algebra(names: &[&str]) -> Algebra {
        Algebra::new(&names.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()).unwrap()
    }
    fn small() -> Options {
        Options {
            min_degree_product: 1,
            min_input_terms: 0,
            ..Options::default()
        }
    }
    fn compare(a: &Algebra, p: &str, q: &str, x: usize) -> (Poly, Statistics) {
        let p = a.parse(p).unwrap();
        let q = a.parse(q).unwrap();
        let result = try_resultant(&p, &q, x, &small(), || Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(result.0, exact_resultant_ducos(&p, &q, x).unwrap());
        assert_eq!(result.0.variables(), p.variables());
        assert_eq!(result.0.degree(x), 0);
        assert!(result.1.numeric_result_bits <= result.1.plan.output_bit_bound as u64);
        result
    }

    #[test]
    fn exact_rational_scale_sign_repeated_and_shared_factors() {
        let a = algebra(&["a", "b", "x"]);
        for (p, q) in [
            ("x^3+a*x+b", "x^2-a*b*x+2"),
            ("x^3/7-a*x/3+b/11", "x^2/5+b*x/13-a/17"),
            ("-6*(x-a)^2*(x+b)/35", "14*(x^2+a*b+2)/33"),
            ("(x-a)*(x+b)", "(x-a)*(x^2+b)"),
            ("a*x^3+b*x+1", "b*x^2-a*x+3"),
            ("x^2-a", "x^2-b"),
            ("x^5-a*b*x+b^3", "x^3+a^2*x-b"),
        ] {
            compare(&a, p, q, 2);
        }
        let (zero, stats) = compare(&a, "(x-a)*(x+b)", "(x-a)*(x^2+b)", 2);
        assert!(zero.is_zero());
        assert_eq!(stats.numeric_result_bits, 0);
    }

    #[test]
    fn varied_original_maps_and_inactive_axes_remain_exact() {
        for (names, x) in [
            (vec!["x", "a", "unused", "b"], 0),
            (vec!["unused", "b", "x", "a", "tail"], 2),
            (vec!["b", "unused", "a", "tail", "x"], 4),
        ] {
            let a = algebra(&names);
            let (_, stats) = compare(&a, "a*x^3+b*x+1", "b*x^2-a*x+3", x);
            for (axis, name) in names.iter().enumerate() {
                if *name == "unused" || *name == "tail" || axis == x {
                    assert_eq!(stats.plan.weights[axis], 0);
                    assert_eq!(stats.plan.degree_bounds[axis], 0);
                }
            }
        }
    }

    #[test]
    fn deterministic_polynomial_family_matches_independent_ducos() {
        let a = algebra(&["a", "x", "b"]);
        for i in 0..12 {
            let c = i - 5;
            let d = 2 * i - 9;
            compare(
                &a,
                &format!("({c})*a*x^{}+b*x^2+a*b*x+{}/7", 3 + i % 2, i + 1),
                &format!("b*x^3+({d})*a*x^2+(a-b)*x+{}/5", i + 2),
                1,
            );
        }
    }

    #[test]
    fn default_envelope_includes_the_measured_ten_by_six_degree_pair() {
        let a = algebra(&["s", "x", "t"]);
        let options = Options::default();
        let p = a.parse("(x+s+t+1)^10+s*x+1").unwrap();
        let q = a.parse("(x+s+t+2)^6+t*x+1").unwrap();
        let admitted = plan(&p, &q, 1, &options).unwrap().unwrap();
        assert_eq!(admitted.parameters, vec![0, 2]);
        assert!(admitted.output_bit_bound <= options.max_output_bits);
        assert!(admitted.total_input_bit_bound <= options.max_total_input_bits);
        // Preserve a lower degree-product cutoff after admitting the known
        // 10×6 capture; smaller pairs still use the existing exact backend.
        let smaller = a.parse("(x+s+t+1)^9+s*x+1").unwrap();
        assert!(plan(&smaller, &q, 1, &options).unwrap().is_none());
    }

    #[test]
    fn bounded_dense_inputs_fit_within_the_new_term_ceiling() {
        let a = algebra(&["s", "x", "unused"]);
        let options = Options {
            min_parameters: 1,
            ..Options::default()
        };
        let powers = (0..=32)
            .map(|k| format!("x^{k}"))
            .collect::<Vec<_>>()
            .join("+");
        let p = a.parse(&format!("({powers})*(s+1)^123")).unwrap();
        let q = p.derivative(1);
        assert!(p.nterms() + q.nterms() > 4096);
        assert!(p.nterms() + q.nterms() <= options.max_input_terms);
        let admitted = plan(&p, &q, 1, &options).unwrap().unwrap();
        assert!(admitted.output_bit_bound < 96_000_000);
        assert!(
            plan(
                &p,
                &q,
                1,
                &Options {
                    max_input_terms: 4096,
                    ..options.clone()
                }
            )
            .unwrap()
            .is_none()
        );
        let oversized = a.parse(&format!("({powers})*(s+1)^126")).unwrap();
        assert!(oversized.nterms() + oversized.derivative(1).nterms() > options.max_input_terms);
        assert!(
            plan(&oversized, &oversized.derivative(1), 1, &options)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn larger_encoding_still_obeys_independent_output_input_and_digit_caps() {
        let a = algebra(&["s", "x", "unused"]);
        let options = Options {
            min_parameters: 1,
            ..Options::default()
        };
        let p = a.parse("-2/3*(x^10+2^3500*(s+1)^500)").unwrap();
        let q = p.derivative(1);
        let admitted = plan(&p, &q, 1, &options).unwrap().unwrap();
        assert!(admitted.output_bit_bound > 96_000_000);
        assert!(admitted.output_bit_bound < options.max_output_bits);
        for restricted in [
            Options {
                max_output_bits: 96_000_000,
                ..options.clone()
            },
            Options {
                max_total_input_bits: admitted.total_input_bit_bound - 1,
                ..options.clone()
            },
            Options {
                max_digits: admitted.digits - 1,
                ..options.clone()
            },
        ] {
            assert!(plan(&p, &q, 1, &restricted).unwrap().is_none());
        }
        let oversized = a.parse("x^10+(s+1)^1900").unwrap();
        let derivative = oversized.derivative(1);
        assert!(
            plan(&oversized, &derivative, 1, &options)
                .unwrap()
                .is_none()
        );
        let larger = plan(
            &oversized,
            &derivative,
            1,
            &Options {
                max_output_bits: 512_000_000,
                ..options
            },
        )
        .unwrap()
        .unwrap();
        assert!(larger.output_bit_bound > 256_000_000);
    }

    #[test]
    fn all_static_caps_decline_without_an_algebra_fallback() {
        let a = algebra(&["a", "b", "x"]);
        let p = a.parse("a*x^3/3+b*x/5+1/7").unwrap();
        let q = a.parse("b*x^2-a*x+3").unwrap();
        assert!(
            try_resultant(&p, &q, 2, &Options::default(), || Ok(()))
                .unwrap()
                .is_none()
        );
        for opts in [
            Options {
                max_variables: 1,
                ..small()
            },
            Options {
                max_input_terms: 1,
                ..small()
            },
            Options {
                max_elimination_degree: 1,
                ..small()
            },
            Options {
                max_input_coefficient_bits: 1,
                ..small()
            },
            Options {
                max_denominator_bits: 4,
                ..small()
            },
            Options {
                max_cleared_coefficient_bits: 1,
                ..small()
            },
            Options {
                max_coefficient_bound_bits: 1,
                ..small()
            },
            Options {
                max_digits: 1,
                ..small()
            },
            Options {
                max_output_bits: 1,
                ..small()
            },
            Options {
                max_total_input_bits: 1,
                ..small()
            },
            Options {
                max_parameters: 1,
                ..small()
            },
            Options {
                min_parameters: 3,
                ..small()
            },
        ] {
            assert!(
                try_resultant(&p, &q, 2, &opts, || Ok(()))
                    .unwrap()
                    .is_none(),
                "{opts:?}"
            );
        }
        let huge = a.parse("a^1000000*x^2+b").unwrap();
        assert!(
            try_resultant(&huge, &q, 2, &small(), || Ok(()))
                .unwrap()
                .is_none()
        );
        for text in ["0", "a+b", "x+a+b"] {
            assert!(
                try_resultant(&a.parse(text).unwrap(), &q, 2, &small(), || Ok(()))
                    .unwrap()
                    .is_none()
            );
        }
        let one_parameter = a.parse("x^3+a*x+1").unwrap();
        assert!(
            try_resultant(&one_parameter, &one_parameter, 2, &small(), || Ok(()))
                .unwrap()
                .is_none()
        );
        assert!(plan(&p, &q, 3, &small()).is_err());
        let other = algebra(&["a", "x", "b"]);
        assert!(plan(&p, &other.parse("a*x^2+b").unwrap(), 2, &small()).is_err());
    }

    #[test]
    fn cancellation_and_runtime_caps_propagate_typed_stops() {
        let a = algebra(&["a", "b", "x"]);
        let p = a.parse("x^2-a").unwrap();
        let q = a.parse("x^2-b").unwrap();
        let e = try_resultant(&p, &q, 2, &small(), || anyhow::bail!("cancelled")).unwrap_err();
        assert!(e.downcast_ref::<Stopped>().is_some());
        let mut checks = 0;
        let e = try_resultant(&p, &q, 2, &small(), || {
            checks += 1;
            ensure!(checks < 9, "cancel during operation");
            Ok(())
        })
        .unwrap_err();
        assert!(e.downcast_ref::<Stopped>().is_some());
        let e = try_resultant(
            &p,
            &q,
            2,
            &Options {
                max_output_terms: 0,
                ..small()
            },
            || Ok(()),
        )
        .unwrap_err();
        assert!(e.downcast_ref::<Stopped>().is_some());
    }

    #[test]
    fn centered_decode_boundaries_and_large_shifts_are_exact() {
        let a = algebra(&["a", "b", "x"]);
        let p = a.parse("x^2+a^2+b^2-2").unwrap();
        let q = a.parse("x^2-a*b-3").unwrap();
        let prepared = prepare(&p, &q, 2, &small(), &mut || Ok(()))
            .unwrap()
            .unwrap();
        let plan = &prepared.plan;
        let bound = &prepared.bound;
        for text in ["-a^2*b^2+7*a*b-3", "a^2*b^2-7*a*b+3", "-3", "0"] {
            let target = a.parse(text).unwrap();
            let value = if target.is_zero() {
                Integer::zero()
            } else {
                encode(&target, 2, plan, &mut || Ok(()))
                    .unwrap()
                    .get_constant()
            };
            let mut output = p.zero();
            decode(
                value,
                0,
                plan.digits,
                plan,
                bound,
                &mut output,
                65536,
                &mut || Ok(()),
            )
            .unwrap();
            assert_eq!(output, target);
        }
        for sign in [-1, 1] {
            let mut target = p.zero();
            target.append_monomial_back(Rational::from(sign), &[0, 0, 0]);
            target.append_monomial_back(
                Rational::from(bound.clone() * Integer::from(-sign)),
                &[
                    plan.degree_bounds[0] as u32,
                    plan.degree_bounds[1] as u32,
                    0,
                ],
            );
            let value = encode(&target, 2, plan, &mut || Ok(()))
                .unwrap()
                .get_constant();
            let mut output = p.zero();
            decode(
                value,
                0,
                plan.digits,
                plan,
                bound,
                &mut output,
                65536,
                &mut || Ok(()),
            )
            .unwrap();
            assert_eq!(output, target);
        }
        assert_eq!(shift(Integer::one(), 130).significant_bits(), 131);
        let mut output = p.zero();
        assert!(
            decode(
                bound + Integer::one(),
                0,
                1,
                plan,
                bound,
                &mut output,
                65536,
                &mut || Ok(())
            )
            .is_err()
        );
    }

    #[test]
    fn integer_linear_identity_is_signed_for_both_operand_orders() {
        let a = algebra(&["x"]);
        let make = |text| {
            let p = a.parse(text).unwrap().to_univariate(0);
            UnivariatePolynomial::from_coefficients(
                &Z,
                p.coefficients()
                    .iter()
                    .map(|c| c.get_constant().numerator())
                    .collect(),
                Arc::new(a.variables[0].clone()),
            )
        };
        let p = make("x^3+2");
        let q = make("2*x-3");
        assert_eq!(integer_resultant(&p, &q), Integer::from(-43));
        assert_eq!(integer_resultant(&q, &p), Integer::from(43));
    }
}
