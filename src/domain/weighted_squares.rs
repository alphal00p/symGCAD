//! Bounded positive monomial weights times exact binomial squares.
use super::{
    CertificateMethod, CertificateOptions, Domain, ProofBudget, shift_to_positive_oriented,
};
use crate::algebra::{Algebra, Poly, rational};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use symbolica::prelude::*;

const MAX_TERMS: usize = 256;
const MAX_VARIABLES: usize = 16;
const MAX_EXPONENT: u32 = 32;
const MAX_HEIGHT: u64 = 512;
const MAX_SQUARES: usize = 32;
const STRATEGIES: usize = 36;
const MAX_ORIENTATIONS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedSquareTerm {
    pub weight: String,
    pub common: Vec<u32>,
    pub left: Vec<u32>,
    pub right: Vec<u32>,
    pub ratio: String,
}

/// Literal sparse data prevents expression expansion before replay's size caps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedResidualTerm {
    pub coefficient: String,
    pub exponents: Vec<u32>,
}

pub(super) fn positive_literal(text: &str) -> Result<Rational> {
    // A 512-bit numerator/denominator pair needs at most 156 decimal digits.
    // Check grammar and length before handing any data to Symbolica's parser.
    ensure!(text.len() <= 160, "weighted rational literal length");
    let mut parts = text.split('/');
    let numerator = parts.next().unwrap_or("");
    let denominator = parts.next();
    ensure!(
        !numerator.is_empty()
            && numerator.bytes().all(|b| b.is_ascii_digit())
            && denominator.is_none_or(|d| {
                !d.is_empty()
                    && d.bytes().all(|b| b.is_ascii_digit())
                    && d.bytes().any(|b| b != b'0')
            })
            && parts.next().is_none(),
        "weighted rational must be a positive literal"
    );
    let value = rational(text)?;
    ensure!(
        value > Rational::zero() && height(&value) <= MAX_HEIGHT,
        "weighted rational height/sign"
    );
    Ok(value)
}

pub(super) fn replay_residual(
    template: &Poly,
    terms: &[WeightedResidualTerm],
    limit: usize,
) -> Result<Poly> {
    ensure!(
        !terms.is_empty() && terms.len() <= limit,
        "weighted residual term count"
    );
    let mut result = template.zero();
    let mut seen = std::collections::BTreeSet::new();
    for term in terms {
        ensure!(
            term.exponents.len() == template.nvars()
                && term.exponents.iter().all(|e| *e <= MAX_EXPONENT)
                && seen.insert(&term.exponents),
            "weighted residual coordinate/exponent budget"
        );
        result.append_monomial(positive_literal(&term.coefficient)?, &term.exponents);
    }
    Ok(result)
}

pub(super) fn height(c: &Rational) -> u64 {
    c.numerator_ref().significant_bits() as u64 + c.denominator_ref().significant_bits() as u64
}

pub(super) fn bounded(p: &Poly) -> bool {
    p.nterms() <= MAX_TERMS
        && p.nvars() <= MAX_VARIABLES
        && p.exponents_iter()
            .all(|e| e.iter().all(|x| *x <= MAX_EXPONENT))
        && p.coefficients.iter().all(|c| height(c) <= MAX_HEIGHT)
}

fn monomial(template: &Poly, coefficient: Rational, exponents: &[u32]) -> Poly {
    let mut p = template.zero();
    p.append_monomial(coefficient, exponents);
    p
}

pub(super) fn positive_support(p: &Poly, positive: &[bool]) -> bool {
    p.exponents_iter().all(|e| {
        e.iter()
            .zip(positive)
            .all(|(&power, &is_positive)| power == 0 || is_positive)
    })
}

pub(super) fn strict_residual(p: &Poly, positive: &[bool]) -> bool {
    !p.is_zero()
        && p.coefficients.iter().all(|c| *c > Rational::zero())
        && positive_support(p, positive)
}

/// Independent replay does not trust discovery's coefficient map or mutations.
fn replay_square(template: &Poly, s: &WeightedSquareTerm, positive: &[bool]) -> Result<Poly> {
    ensure!(
        [s.common.len(), s.left.len(), s.right.len()]
            .into_iter()
            .all(|n| n == template.nvars()),
        "square coordinate map"
    );
    ensure!(
        s.common
            .iter()
            .chain(&s.left)
            .chain(&s.right)
            .all(|e| *e <= MAX_EXPONENT),
        "square exponent budget"
    );
    for (i, &known_positive) in positive.iter().enumerate() {
        ensure!(
            known_positive || (s.common[i] == 0 && s.left[i] == 0 && s.right[i] == 0),
            "unrestricted coordinate in weighted square"
        );
        ensure!(
            s.common[i] + 2 * s.left[i] <= MAX_EXPONENT
                && s.common[i] + 2 * s.right[i] <= MAX_EXPONENT,
            "replayed square exponent budget"
        );
    }
    let weight = positive_literal(&s.weight)?;
    let ratio = positive_literal(&s.ratio)?;
    let binomial =
        monomial(template, Rational::one(), &s.left) - monomial(template, ratio, &s.right);
    let square = monomial(template, weight, &s.common) * &binomial.pow(2);
    ensure!(bounded(&square), "replayed square size budget");
    Ok(square)
}

#[derive(Clone)]
struct Pair {
    u: Vec<u32>,
    v: Vec<u32>,
    a: Rational,
    b: Rational,
    slack: Rational,
    distance: u32,
}

fn extract(
    p: &Poly,
    positive: &[bool],
    strategy: usize,
    budget: &mut ProofBudget,
) -> Result<Option<(Vec<WeightedSquareTerm>, Poly)>> {
    if !budget.spend(p.nterms().saturating_mul(p.nvars())) {
        return Ok(None);
    }
    let mut coefficients: BTreeMap<_, _> = p
        .coefficients
        .iter()
        .zip(p.exponents_iter())
        .map(|(c, e)| (e.to_vec(), c.clone()))
        .collect();
    let mut negatives: Vec<_> = coefficients
        .iter()
        .filter(|(_, c)| **c < Rational::zero())
        .map(|(e, c)| (e.clone(), -c.clone()))
        .collect();
    match strategy % 4 {
        1 => negatives.reverse(),
        2 => negatives.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0))),
        3 => negatives.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))),
        _ => (),
    }
    let ranking = (strategy / 4) % 3;
    let weight_mode = (strategy / 12) % 3;
    let mut squares = Vec::new();
    for (e, c) in negatives {
        if squares.len() >= MAX_SQUARES {
            break;
        }
        let c2 = &c * &c;
        let mut best: Option<Pair> = None;
        for (u, a) in &coefficients {
            if !budget.spend(p.nvars().saturating_add(4)) {
                return Ok(None);
            }
            if *a <= Rational::zero()
                || height(a) > MAX_HEIGHT
                || u.iter().zip(&e).any(|(u, e)| *u > 2 * e)
            {
                continue;
            }
            let v: Vec<_> = e.iter().zip(u).map(|(e, u)| 2 * e - u).collect();
            if *u >= v {
                continue;
            }
            let Some(b) = coefficients.get(&v).filter(|b| **b > Rational::zero()) else {
                continue;
            };
            if height(b) > MAX_HEIGHT {
                continue;
            }
            let four_ab = (a * b) * Rational::from(4);
            if four_ab < c2 {
                continue;
            }
            let candidate = Pair {
                u: u.clone(),
                v: v.clone(),
                a: a.clone(),
                b: b.clone(),
                slack: four_ab / &c2,
                distance: u.iter().zip(&v).map(|(a, b)| a.abs_diff(*b)).sum(),
            };
            let better = best.as_ref().is_none_or(|old| match ranking {
                0 => candidate.slack > old.slack,
                1 => {
                    candidate.distance < old.distance
                        || (candidate.distance == old.distance && candidate.slack > old.slack)
                }
                _ => candidate.u > old.u,
            });
            if better {
                best = Some(candidate);
            }
        }
        let Some(pair) = best else {
            continue;
        };
        let lower = &c2 / &(&pair.b * &Rational::from(4));
        let weight = match weight_mode {
            0 => pair.a.clone(),
            1 => lower.clone(),
            _ => (&pair.a + &lower) / Rational::from(2),
        };
        let ratio = &c / &(&weight * &Rational::from(2));
        let used_b = &weight * &ratio * &ratio;
        ensure!(
            weight >= lower && weight <= pair.a && used_b <= pair.b,
            "coefficient budget"
        );
        if height(&weight) > MAX_HEIGHT
            || height(&ratio) > MAX_HEIGHT
            || height(&used_b) > MAX_HEIGHT
        {
            continue;
        }
        let common: Vec<_> = pair
            .u
            .iter()
            .zip(&pair.v)
            .map(|(u, v)| (*u).min(*v))
            .collect();
        ensure!(
            pair.u.iter().zip(&common).all(|(u, g)| (u - g) % 2 == 0)
                && pair.v.iter().zip(&common).all(|(v, g)| (v - g) % 2 == 0),
            "even square exponents"
        );
        let square = WeightedSquareTerm {
            weight: weight.to_string(),
            left: pair
                .u
                .iter()
                .zip(&common)
                .map(|(u, g)| (u - g) / 2)
                .collect(),
            right: pair
                .v
                .iter()
                .zip(&common)
                .map(|(v, g)| (v - g) / 2)
                .collect(),
            common,
            ratio: ratio.to_string(),
        };
        coefficients.insert(pair.u, &pair.a - &weight);
        coefficients.insert(pair.v, &pair.b - &used_b);
        coefficients.insert(e, Rational::zero());
        squares.push(square);
    }
    let mut residual = p.zero();
    for (e, c) in coefficients {
        if !c.is_zero() {
            residual.append_monomial(c, &e);
        }
    }
    if !bounded(&residual) {
        return Ok(None);
    }
    if !strict_residual(&residual, positive) {
        return Ok(None);
    }
    Ok(Some((squares, residual)))
}

fn reconstruct(
    signed: &Poly,
    positive: &[bool],
    squares: &[WeightedSquareTerm],
    residual: &Poly,
    budget: &mut ProofBudget,
) -> Result<bool> {
    if squares.len() > MAX_SQUARES
        || !bounded(residual)
        || !strict_residual(residual, positive)
        || !budget.spend(residual.nterms().saturating_mul(residual.nvars()))
    {
        return Ok(false);
    }
    let mut reconstructed = residual.clone();
    for square in squares {
        if !budget.spend(
            signed
                .nvars()
                .saturating_mul(3)
                .saturating_add(reconstructed.nterms()),
        ) {
            return Ok(false);
        }
        reconstructed = reconstructed + replay_square(signed, square, positive)?;
        if !bounded(&reconstructed) {
            return Ok(false);
        }
    }
    Ok(reconstructed == *signed)
}

/// Discovery is finite; failing any bound leaves the polynomial untouched.
pub(super) fn discover(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<(i8, CertificateMethod)>> {
    if options.weighted_square_candidates == 0 || !bounded(polynomial) {
        return Ok(None);
    }
    let limit = options.max_terms.min(MAX_TERMS);
    if polynomial.nterms() > limit {
        return Ok(None);
    }
    let mut alternatives = Vec::new();
    for (v, bounds) in domain.bounds.iter().enumerate() {
        if polynomial.degree(v) == 0 {
            continue;
        }
        for bound in [&bounds.lower, &bounds.upper].into_iter().flatten() {
            if height(&rational(bound)?) > MAX_HEIGHT {
                return Ok(None);
            }
        }
        if bounds.lower.is_some() && bounds.upper.is_some() {
            alternatives.push(v);
        }
    }
    let mut orientations = vec![vec![]];
    for &v in alternatives.iter().take(MAX_ORIENTATIONS - 2) {
        orientations.push(vec![v]);
    }
    if alternatives.len() > 1 {
        orientations.push(alternatives);
    }
    let mut budget = ProofBudget::new(options);
    let mut shifted_forms = Vec::new();
    for upper_variables in orientations {
        if !budget.spend(polynomial.nterms().saturating_mul(polynomial.nvars())) {
            return Ok(None);
        }
        let Some((shifted, positive)) =
            shift_to_positive_oriented(algebra, polynomial, domain, &upper_variables, limit)?
        else {
            continue;
        };
        if bounded(&shifted) && positive_support(&shifted, &positive) {
            shifted_forms.push((upper_variables, shifted, positive));
        }
    }
    let mut attempts = 0usize;
    for strategy in 0..STRATEGIES {
        for sign in [1i8, -1] {
            for (upper_variables, shifted, positive) in &shifted_forms {
                if attempts >= options.weighted_square_candidates || budget.work == 0 {
                    return Ok(None);
                }
                attempts += 1;
                let signed = shifted.clone().mul_coeff(Rational::from(sign as i64));
                let Some((squares, residual)) = extract(&signed, positive, strategy, &mut budget)?
                else {
                    continue;
                };
                if residual.nterms() <= limit
                    && reconstruct(&signed, positive, &squares, &residual, &mut budget)?
                {
                    return Ok(Some((
                        sign,
                        CertificateMethod::WeightedSquaresPlusPositiveResidual {
                            upper_variables: upper_variables.clone(),
                            squares,
                            residual: residual
                                .coefficients
                                .iter()
                                .zip(residual.exponents_iter())
                                .map(|(c, e)| WeightedResidualTerm {
                                    coefficient: c.to_string(),
                                    exponents: e.to_vec(),
                                })
                                .collect(),
                        },
                    )));
                }
            }
        }
    }
    Ok(None)
}

/// Replay starts from the original polynomial and a domain independently
/// reconstructed by verify_certificate; no discovery state or cache is used.
pub(super) fn verify(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    sign: i8,
    upper_variables: &[usize],
    squares: &[WeightedSquareTerm],
    residual: &[WeightedResidualTerm],
    options: &CertificateOptions,
) -> Result<bool> {
    ensure!(
        options.weighted_square_candidates > 0,
        "weighted square certificate is disabled"
    );
    let limit = options.max_terms.min(MAX_TERMS);
    ensure!(
        bounded(polynomial) && polynomial.nterms() <= limit,
        "weighted square polynomial budget"
    );
    ensure!(squares.len() <= MAX_SQUARES, "weighted square count budget");
    for (v, bounds) in domain.bounds.iter().enumerate() {
        if polynomial.degree(v) == 0 {
            continue;
        }
        for bound in [&bounds.lower, &bounds.upper].into_iter().flatten() {
            ensure!(
                height(&rational(bound)?) <= MAX_HEIGHT,
                "weighted square bound height budget"
            );
        }
    }
    let mut budget = ProofBudget::new(options);
    if !budget.spend(polynomial.nterms().saturating_mul(polynomial.nvars())) {
        return Ok(false);
    }
    let signed = polynomial.clone().mul_coeff(Rational::from(sign as i64));
    let Some((shifted, positive)) =
        shift_to_positive_oriented(algebra, &signed, domain, upper_variables, limit)?
    else {
        return Ok(false);
    };
    ensure!(
        bounded(&shifted) && positive_support(&shifted, &positive),
        "weighted square shifted domain"
    );
    let residual = replay_residual(&shifted, residual, limit)?;
    reconstruct(&shifted, &positive, squares, &residual, &mut budget)
}
