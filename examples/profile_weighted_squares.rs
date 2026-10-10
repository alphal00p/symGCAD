//! Research-only bounded weighted binomial squares; no production certificate.
//! Run under a killable process deadline: algebra calls are not interruptible.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    domain::{CertificateOptions, certify_nonzero, derive_domain, verify_certificate},
};

const MAX_TERMS: usize = 256;
const MAX_VARIABLES: usize = 16;
const MAX_EXPONENT: u32 = 32;
const MAX_HEIGHT: u64 = 512;
const MAX_SQUARES: usize = 32;
const MAX_WORK: usize = 1_000_000;
const STRATEGIES: usize = 36;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Deserialize)]
struct Input {
    variables: Vec<String>,
    /// Original strict constraints, each expression interpreted as p > 0.
    constraints: Vec<String>,
    polynomials: Vec<NamedPolynomial>,
}

#[derive(Deserialize)]
struct NamedPolynomial {
    label: String,
    polynomial: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Shift {
    variable: usize,
    /// x_original = offset + orientation*x_positive.
    offset: String,
    orientation: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Square {
    weight: String,
    common: Vec<u32>,
    left: Vec<u32>,
    right: Vec<u32>,
    ratio: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Certificate {
    sign: i8,
    shifts: Vec<Shift>,
    squares: Vec<Square>,
    residual: String,
}

#[derive(Default, Serialize)]
struct Stats {
    strategies: usize,
    pair_scans: usize,
    squares_proposed: usize,
    best_negative_terms: usize,
    budget_exhausted: bool,
}

fn height(c: &Rational) -> u64 {
    c.numerator_ref().significant_bits() as u64 + c.denominator_ref().significant_bits() as u64
}

fn bounded(p: &Poly) -> bool {
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

fn count_negative(p: &Poly) -> usize {
    p.coefficients
        .iter()
        .filter(|c| **c < Rational::zero())
        .count()
}

fn positive_support(p: &Poly, positive: &[bool]) -> bool {
    p.exponents_iter().all(|e| {
        e.iter()
            .zip(positive)
            .all(|(&power, &is_positive)| power == 0 || is_positive)
    })
}

fn strict_residual(p: &Poly, positive: &[bool]) -> bool {
    !p.is_zero()
        && p.coefficients.iter().all(|c| *c > Rational::zero())
        && positive_support(p, positive)
}

/// Conservative scope: every active coordinate must have an exact affine bound.
/// Other original constraints can only narrow the certified orthant superset.
pub(crate) fn shift(
    algebra: &Algebra,
    p: &Poly,
    constraints: &[Poly],
) -> Result<Option<(Poly, Vec<Shift>, Vec<bool>)>> {
    if !bounded(p) {
        return Ok(None);
    }
    let domain = derive_domain(algebra, constraints, &[])?;
    if domain.empty {
        return Ok(None);
    }
    let mut shifts = Vec::new();
    let mut positive = vec![false; algebra.names.len()];
    let mut shifted = p.clone();
    for (variable, bound) in domain.bounds.iter().enumerate() {
        if p.degree(variable) == 0 {
            continue;
        }
        let (offset, orientation) = if let Some(lower) = &bound.lower {
            (lower.clone(), 1i8)
        } else if let Some(upper) = &bound.upper {
            (upper.clone(), -1i8)
        } else {
            return Ok(None);
        };
        let offset_value = rational(&offset)?;
        if height(&offset_value) > MAX_HEIGHT {
            return Ok(None);
        }
        let mut exponent = vec![0; algebra.names.len()];
        exponent[variable] = 1;
        let coordinate = monomial(p, Rational::from(orientation as i64), &exponent);
        let replacement = coordinate + p.constant(offset_value);
        shifted = shifted.replace_with_poly(variable, &replacement);
        if !bounded(&shifted) {
            return Ok(None);
        }
        positive[variable] = true;
        shifts.push(Shift {
            variable,
            offset,
            orientation,
        });
    }
    Ok(Some((shifted, shifts, positive)))
}

/// Independent replay does not trust discovery's coefficient map or mutations.
fn replay_square(template: &Poly, s: &Square, positive: &[bool]) -> Result<Poly> {
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
            "unrestricted coordinate in research square"
        );
        ensure!(
            s.common[i] + 2 * s.left[i] <= MAX_EXPONENT
                && s.common[i] + 2 * s.right[i] <= MAX_EXPONENT,
            "replayed square exponent budget"
        );
    }
    let weight = rational(&s.weight)?;
    let ratio = rational(&s.ratio)?;
    ensure!(
        weight > Rational::zero()
            && ratio > Rational::zero()
            && height(&weight) <= MAX_HEIGHT
            && height(&ratio) <= MAX_HEIGHT,
        "positive rational square weights"
    );
    let binomial =
        monomial(template, Rational::one(), &s.left) - monomial(template, ratio, &s.right);
    let square = monomial(template, weight, &s.common) * &binomial.pow(2);
    ensure!(bounded(&square), "replayed square size budget");
    Ok(square)
}

fn verify(
    algebra: &Algebra,
    original: &Poly,
    constraints: &[Poly],
    certificate: &Certificate,
) -> Result<()> {
    ensure!(
        matches!(certificate.sign, -1 | 1),
        "nonzero sign orientation"
    );
    ensure!(
        certificate.squares.len() <= MAX_SQUARES,
        "square count budget"
    );
    let (shifted, shifts, positive) = shift(algebra, original, constraints)?
        .context("original domain has no allowed positive-coordinate map")?;
    ensure!(certificate.shifts == shifts, "shift/domain binding");
    let residual = algebra.parse(&certificate.residual)?;
    ensure!(bounded(&residual), "residual size budget");
    ensure!(
        strict_residual(&residual, &positive),
        "strict positive residual required"
    );
    let mut reconstructed = residual;
    for square in &certificate.squares {
        reconstructed = reconstructed + replay_square(original, square, &positive)?;
        ensure!(bounded(&reconstructed), "reconstruction size budget");
    }
    ensure!(
        reconstructed == shifted.mul_coeff(Rational::from(certificate.sign as i64)),
        "full signed polynomial reconstruction"
    );
    Ok(())
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
    stats: &mut Stats,
    work_limit: usize,
) -> Result<Option<(Vec<Square>, Poly)>> {
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
            stats.pair_scans += 1;
            if stats.pair_scans > work_limit {
                stats.budget_exhausted = true;
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
        let square = Square {
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
        let _ = replay_square(p, &square, positive)?;
        coefficients.insert(pair.u, &pair.a - &weight);
        coefficients.insert(pair.v, &pair.b - &used_b);
        coefficients.insert(e, Rational::zero());
        squares.push(square);
        stats.squares_proposed += 1;
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
    stats.best_negative_terms = stats.best_negative_terms.min(count_negative(&residual));
    if !strict_residual(&residual, positive) {
        return Ok(None);
    }
    Ok(Some((squares, residual)))
}

fn discover(
    algebra: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    work_limit: usize,
) -> Result<(Option<Certificate>, Stats)> {
    let mut stats = Stats {
        best_negative_terms: count_negative(p),
        ..Stats::default()
    };
    let Some((shifted, shifts, positive)) = shift(algebra, p, constraints)? else {
        return Ok((None, stats));
    };
    for sign in [1i8, -1] {
        let signed = shifted.clone().mul_coeff(Rational::from(sign as i64));
        for strategy in 0..STRATEGIES {
            stats.strategies += 1;
            if let Some((squares, residual)) =
                extract(&signed, &positive, strategy, &mut stats, work_limit)?
            {
                let certificate = Certificate {
                    sign,
                    shifts,
                    squares,
                    residual: algebra.format(&residual),
                };
                verify(algebra, p, constraints, &certificate)?;
                return Ok((Some(certificate), stats));
            }
            if stats.budget_exhausted {
                return Ok((None, stats));
            }
        }
    }
    Ok((None, stats))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "fresh output required");
    let bytes = fs::read(&args.input)?;
    ensure!(bytes.len() <= 4 * 1024 * 1024, "input byte budget");
    let input: Input = serde_json::from_slice(&bytes)?;
    ensure!(
        input.variables.len() <= MAX_VARIABLES && input.polynomials.len() <= 32,
        "input item budget"
    );
    let algebra = Algebra::new(&input.variables)?;
    let constraints = input
        .constraints
        .iter()
        .map(|s| algebra.parse(s))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&algebra, &constraints, &[])?;
    let mut rows = Vec::new();
    for item in input.polynomials {
        ensure!(
            item.polynomial.len() <= 65536,
            "polynomial input byte budget"
        );
        let p = algebra.parse(&item.polynomial)?;
        let baseline_start = Instant::now();
        let baseline = certify_nonzero(&algebra, &p, &domain, &CertificateOptions::default())?;
        let baseline_seconds = baseline_start.elapsed().as_secs_f64();
        if let Some(c) = &baseline {
            verify_certificate(&algebra, &p, &constraints, &[], c)?;
        }
        let started = Instant::now();
        let (certificate, stats) = discover(&algebra, &p, &constraints, MAX_WORK)?;
        let discovery_seconds = started.elapsed().as_secs_f64();
        let verify_start = Instant::now();
        if let Some(c) = &certificate {
            let roundtrip: Certificate = serde_json::from_slice(&serde_json::to_vec(c)?)?;
            verify(&algebra, &p, &constraints, &roundtrip)?;
        }
        rows.push(json!({"label":item.label,"polynomial":algebra.format(&p),"terms":p.nterms(),
            "current_portfolio":baseline,"current_portfolio_seconds":baseline_seconds,
            "certificate":certificate,"verified":certificate.is_some(),"stats":stats,
            "discovery_seconds":discovery_seconds,"replay_seconds":verify_start.elapsed().as_secs_f64()}));
    }
    let report = json!({"schema_version":1,"kind":"research_weighted_binomial_squares",
        "original_completion":false,"production_certificate":false,
        "input":args.input,"input_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "variables":input.variables,"constraints":input.constraints,
        "limits":{"terms":MAX_TERMS,"variables":MAX_VARIABLES,"exponent":MAX_EXPONENT,
            "rational_height":MAX_HEIGHT,"squares_per_trial":MAX_SQUARES,
            "strategies_per_sign":STRATEGIES,"pair_scans_per_polynomial":MAX_WORK},"rows":rows});
    fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Result<(Algebra, Vec<Poly>)> {
        let names = ["x", "y", "w", "z", "h"].map(String::from).to_vec();
        let a = Algebra::new(&names)?;
        let constraints = names
            .iter()
            .map(|s| a.parse(s))
            .collect::<Result<Vec<_>>>()?;
        Ok((a, constraints))
    }

    #[test]
    fn weighted_family_and_negative_orientation() -> Result<()> {
        let (a, constraints) = setup()?;
        for text in [
            "w*(x-y)^2+z*(x-2*y)^2+h*x*y+1",
            "w*x*(x^2-y^2)^2+z*y*(x-3*y)^2+h",
            "-(w*(x-y)^2+z*(x-2*y)^2+h*x*y+1)",
        ] {
            let p = a.parse(text)?;
            let (c, _) = discover(&a, &p, &constraints, MAX_WORK)?;
            let c = c.context("synthetic weighted family should certify")?;
            verify(&a, &p, &constraints, &c)?;
        }
        Ok(())
    }

    #[test]
    fn rejects_zero_seam_and_unrestricted_weight() -> Result<()> {
        let (a, constraints) = setup()?;
        assert!(
            discover(&a, &a.parse("(x-y)^2")?, &constraints, MAX_WORK)?
                .0
                .is_none()
        );
        let unrestricted = vec![a.parse("x")?, a.parse("y")?];
        assert!(
            discover(&a, &a.parse("w*(x-y)^2+1")?, &unrestricted, MAX_WORK)?
                .0
                .is_none()
        );
        assert!(
            discover(&a, &a.parse("1-w*(x-y)^2")?, &constraints, MAX_WORK)?
                .0
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn replay_rejects_forged_identity_weight_and_domain() -> Result<()> {
        let (a, constraints) = setup()?;
        let p = a.parse("w*(x-y)^2+h")?;
        let c = discover(&a, &p, &constraints, MAX_WORK)?
            .0
            .context("proof")?;
        assert!(!c.squares.is_empty());
        let mut wrong = c.clone();
        wrong.residual = format!("({})+1", wrong.residual);
        assert!(verify(&a, &p, &constraints, &wrong).is_err());
        let mut wrong = c.clone();
        wrong.squares[0].weight = "-1".into();
        assert!(verify(&a, &p, &constraints, &wrong).is_err());
        let unrestricted = vec![a.parse("x")?, a.parse("y")?, a.parse("h")?];
        assert!(verify(&a, &p, &unrestricted, &c).is_err());
        Ok(())
    }

    #[test]
    fn explicit_upper_shift_and_work_budget() -> Result<()> {
        let (a, mut constraints) = setup()?;
        constraints[4] = a.parse("4-h")?;
        let p = a.parse("w*(x-y)^2+(4-h)*x*y+z")?;
        let c = discover(&a, &p, &constraints, MAX_WORK)?
            .0
            .context("upper shift proof")?;
        assert!(
            c.shifts
                .iter()
                .any(|s| s.variable == 4 && s.orientation == -1 && s.offset == "4")
        );
        verify(&a, &p, &constraints, &c)?;
        let (c, stats) = discover(&a, &p, &constraints, 0)?;
        assert!(c.is_none() && stats.budget_exhausted);
        Ok(())
    }
}
