//! Research-only exact full-real Descartes covers from untrusted interval hints.
//! Every gap and both tails are checked; hints supply no root-count authority.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{Limits, algebra::rational, benchmark::run_bounded};

const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_DEGREE: usize = 512;
const MAX_COEFFICIENT_BITS: u64 = 20_000;
const MAX_ENDPOINT_BITS: u64 = 256;
const MAX_TRANSFORM_BITS: u64 = 131_072;
const MAX_CUTS: usize = 2048;
const MAX_WORK: usize = 128_000_000;
const MAX_DEPTH: usize = 64;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    input_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 3)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    /// Research alternative: exact integer shifts and reversal instead of Horner.
    #[arg(long)]
    shift_transform: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Term {
    exponent: usize,
    coefficient: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hint {
    lower: String,
    upper: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    terms: Vec<Term>,
    hints: Vec<Hint>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cover {
    cuts: Vec<String>,
    endpoint_roots: Vec<bool>,
    // (-infinity,cuts[0]), every intervening open interval, and the right tail.
    variations: Vec<usize>,
}
#[derive(Default, Serialize)]
struct Work {
    shift_transform: bool,
    transforms: usize,
    /// Conservative transform-operation accounting; endpoint Horner evaluations
    /// are separately bounded by the degree, cut-count and endpoint-height caps.
    transform_work_units: usize,
    subdivisions: usize,
    max_transformed_bits: u64,
    #[serde(skip)]
    progress_path: Option<PathBuf>,
    #[serde(skip)]
    started: Option<Instant>,
    phase: String,
}

impl Work {
    fn progress(&self) -> Result<()> {
        if let Some(path) = &self.progress_path {
            let report = json!({"work":self,"complete":false,"original_cad_coverage":false,
                "elapsed_seconds":self.started.map(|s| s.elapsed().as_secs_f64())});
            let temp = path.with_extension("tmp");
            fs::write(&temp, serde_json::to_vec_pretty(&report)?)?;
            fs::rename(temp, path)?;
        }
        Ok(())
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &PathBuf) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    ensure!(file.metadata()?.is_file(), "regular input required");
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= MAX_BYTES, "input byte cap");
    Ok(bytes)
}
fn literal(s: &str, bits: u64, integer: bool) -> Result<Rational> {
    let parts = s.split('/').collect::<Vec<_>>();
    ensure!(
        !parts.is_empty() && parts.len() <= if integer { 1 } else { 2 },
        "literal grammar"
    );
    for (i, part) in parts.iter().enumerate() {
        let digits = if i == 0 {
            part.strip_prefix('-').unwrap_or(part)
        } else {
            part
        };
        ensure!(
            !digits.is_empty()
                && digits.len() <= (bits as usize * 30103).div_ceil(100000) + 1
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "bounded decimal literal required"
        );
        ensure!(
            i == 0 || digits.bytes().any(|b| b != b'0'),
            "zero denominator"
        );
    }
    let q = rational(s)?;
    ensure!(
        q.numerator_ref().significant_bits() as u64 <= bits
            && q.denominator_ref().significant_bits() as u64 <= bits,
        "literal height cap"
    );
    Ok(q)
}
fn load(input: Input) -> Result<(Vec<Integer>, Vec<Rational>)> {
    ensure!(
        !input.terms.is_empty()
            && input.terms.len() <= MAX_DEGREE + 1
            && input.hints.len() <= MAX_CUTS / 2,
        "input counts"
    );
    let degree = input
        .terms
        .iter()
        .map(|t| t.exponent)
        .max()
        .context("degree")?;
    ensure!(degree > 0 && degree <= MAX_DEGREE, "degree cap");
    let mut p = vec![Integer::zero(); degree + 1];
    for term in input.terms {
        let c = literal(&term.coefficient, MAX_COEFFICIENT_BITS, true)?;
        ensure!(
            !c.is_zero() && p[term.exponent].is_zero(),
            "duplicate or zero sparse term"
        );
        p[term.exponent] = c.numerator_ref().clone();
    }
    let mut cuts = vec![Rational::zero()];
    for hint in input.hints {
        let a = literal(&hint.lower, MAX_ENDPOINT_BITS, false)?;
        let b = literal(&hint.upper, MAX_ENDPOINT_BITS, false)?;
        ensure!(a <= b, "reversed interval hint");
        cuts.extend([a, b]);
    }
    cuts.sort();
    cuts.dedup();
    ensure!(cuts.len() <= MAX_CUTS, "cut count");
    Ok((p, cuts))
}

// Exact homogeneous Horner substitution. For finite a=A/D,b=B/E this computes
// (DE)^n(1+y)^n p((AE+BD*y)/(DE*(1+y))). Its positive roots correspond exactly
// to p's roots in (a,b), preserving multiplicity. For a tail use p(a +/- y),
// with positive denominator clearing. No numerical evaluation is involved.
fn variation(
    p: &[Integer],
    a: Option<&Rational>,
    b: Option<&Rational>,
    work: &mut Work,
) -> Result<usize> {
    ensure!(
        a.is_some() || b.is_some(),
        "split the full line at a rational point"
    );
    let (mut linear0, mut linear1, mut denominator, finite) = match (a, b) {
        (Some(a), Some(b)) => {
            ensure!(a < b, "strict ordered endpoints");
            (
                a.numerator_ref() * b.denominator_ref(),
                b.numerator_ref() * a.denominator_ref(),
                a.denominator_ref() * b.denominator_ref(),
                true,
            )
        }
        (Some(a), None) => (
            a.numerator_ref().clone(),
            a.denominator_ref().clone(),
            a.denominator_ref().clone(),
            false,
        ),
        (None, Some(b)) => (
            b.numerator_ref().clone(),
            -b.denominator_ref(),
            b.denominator_ref().clone(),
            false,
        ),
        _ => unreachable!(),
    };
    if work.shift_transform {
        // A common positive projective scale contributes only its nth power.
        let common = linear0.gcd(&linear1).gcd(&denominator).abs();
        linear0 /= &common;
        linear1 /= &common;
        denominator /= &common;
    }
    let degree = p.len() - 1;
    let input_height = p
        .iter()
        .map(|c| c.significant_bits() as u64)
        .max()
        .unwrap_or(0);
    let step_height = [&linear0, &linear1, &denominator]
        .into_iter()
        .map(|c| c.significant_bits() as u64)
        .max()
        .unwrap_or(0)
        + 2;
    // Conservative preflight of every intermediate coefficient, including sums.
    ensure!(
        input_height + degree as u64 * step_height * if work.shift_transform { 3 } else { 1 } + 2
            <= MAX_TRANSFORM_BITS,
        "transform height preflight"
    );
    let cost = 4 * (degree + 1) * (degree + 1);
    ensure!(
        work.transform_work_units
            .checked_add(cost)
            .is_some_and(|n| n <= MAX_WORK),
        "transform work cap"
    );
    work.transform_work_units += cost;
    work.transforms += 1;
    let h = if work.shift_transform {
        shifted_transform(p, &linear0, &linear1, &denominator, finite, work)?
    } else {
        let mut h = vec![p[degree].clone()];
        let mut power = vec![Integer::one()];
        for c in p[..degree].iter().rev() {
            let mut next_power = vec![Integer::zero(); power.len() + 1];
            for (i, v) in power.iter().enumerate() {
                let term = v * &denominator;
                next_power[i] += &term;
                if finite {
                    next_power[i + 1] += &term;
                }
            }
            let mut next = vec![Integer::zero(); h.len() + 1];
            for (i, v) in h.iter().enumerate() {
                next[i] += v * &linear0;
                next[i + 1] += v * &linear1;
            }
            for (v, w) in next.iter_mut().zip(&next_power) {
                *v += c * w;
            }
            let height = next
                .iter()
                .chain(&next_power)
                .map(|c| c.significant_bits() as u64)
                .max()
                .unwrap_or(0);
            ensure!(height <= MAX_TRANSFORM_BITS, "actual transform height cap");
            work.max_transformed_bits = work.max_transformed_bits.max(height);
            h = next;
            power = next_power;
        }
        h
    };
    let mut last = 0;
    let mut count = 0;
    for c in &h {
        let sign = if c.is_zero() {
            continue;
        } else if c.is_negative() {
            -1
        } else {
            1
        };
        count += usize::from(last != 0 && sign != last);
        last = sign;
    }
    if work.transforms % 16 == 0 {
        work.progress()?;
    }
    Ok(count)
}

// For finite endpoints form P(X)=d^n p(X/d), shift by b, multiply its ith
// coefficient by (a-b)^i, reverse all n+1 coefficients, then shift by one.
// This is exactly (1+y)^n P((a+b*y)/(1+y)). Both shifts are Symbolica
// polynomial operations. Tails need only P(a+b*y). Zero endpoint factors
// remain represented by exact zero coefficients, including degree drops.
fn shifted_transform(
    p: &[Integer],
    a: &Integer,
    b: &Integer,
    d: &Integer,
    finite: bool,
    work: &mut Work,
) -> Result<Vec<Integer>> {
    let n = p.len() - 1;
    let variable = std::sync::Arc::new(PolyVariable::Temporary(0));
    let checked = |coefficients: &[Integer], work: &mut Work| -> Result<()> {
        let height = coefficients
            .iter()
            .map(|c| c.significant_bits() as u64)
            .max()
            .unwrap_or(0);
        ensure!(height <= MAX_TRANSFORM_BITS, "shift transform height cap");
        work.max_transformed_bits = work.max_transformed_bits.max(height);
        Ok(())
    };
    let mut coefficients = p.to_vec();
    let mut power = Integer::one();
    for coefficient in coefficients.iter_mut().rev() {
        *coefficient *= &power;
        power *= d;
    }
    checked(&coefficients, work)?;
    let shifted = UnivariatePolynomial::from_coefficients(&Z, coefficients, variable.clone())
        .shift_var(if finite { b } else { a });
    checked(shifted.coefficients(), work)?;
    let mut coefficients = shifted.coefficients().to_vec();
    coefficients.resize(n + 1, Integer::zero());
    let scale = if finite { a - b } else { b.clone() };
    power = Integer::one();
    for coefficient in &mut coefficients {
        *coefficient *= &power;
        power *= &scale;
    }
    checked(&coefficients, work)?;
    if finite {
        coefficients.reverse();
        coefficients = UnivariatePolynomial::from_coefficients(&Z, coefficients, variable)
            .shift_var(&Integer::one())
            .coefficients()
            .to_vec();
        checked(&coefficients, work)?;
    }
    Ok(coefficients)
}
fn endpoint_is_root(p: &[Integer], x: &Rational) -> bool {
    let mut value = Rational::zero();
    for c in p.iter().rev() {
        value = value * x + Rational::from(c.clone());
    }
    value.is_zero()
}
fn subdivide(
    p: &[Integer],
    a: Option<Rational>,
    b: Option<Rational>,
    depth: usize,
    cuts: &mut Vec<Rational>,
    work: &mut Work,
) -> Result<()> {
    if variation(p, a.as_ref(), b.as_ref(), work)? <= 1 {
        return Ok(());
    }
    ensure!(
        depth < MAX_DEPTH && cuts.len() < MAX_CUTS,
        "subdivision depth/cut cap"
    );
    let middle = match (&a, &b) {
        (Some(a), Some(b)) => (a + b) / Rational::from(2),
        (Some(a), None) => a + &Rational::from(Integer::from(2).pow(depth as u64)),
        (None, Some(b)) => b - &Rational::from(Integer::from(2).pow(depth as u64)),
        _ => unreachable!(),
    };
    literal(&middle.to_string(), MAX_ENDPOINT_BITS, false)?;
    work.subdivisions += 1;
    cuts.push(middle.clone());
    subdivide(p, a, Some(middle.clone()), depth + 1, cuts, work)?;
    subdivide(p, Some(middle), b, depth + 1, cuts, work)
}
fn prove(p: &[Integer], mut cuts: Vec<Rational>, work: &mut Work) -> Result<Cover> {
    let initial = cuts.clone();
    for i in 0..=initial.len() {
        subdivide(
            p,
            i.checked_sub(1).map(|j| initial[j].clone()),
            initial.get(i).cloned(),
            0,
            &mut cuts,
            work,
        )?;
    }
    cuts.sort();
    cuts.dedup();
    let mut variations = Vec::new();
    for i in 0..=cuts.len() {
        variations.push(variation(
            p,
            i.checked_sub(1).map(|j| &cuts[j]),
            cuts.get(i),
            work,
        )?);
    }
    Ok(Cover {
        endpoint_roots: cuts.iter().map(|q| endpoint_is_root(p, q)).collect(),
        cuts: cuts.iter().map(ToString::to_string).collect(),
        variations,
    })
}
fn verify(p: &[Integer], cover: &Cover, work: &mut Work) -> Result<usize> {
    ensure!(
        !cover.cuts.is_empty()
            && cover.cuts.len() <= MAX_CUTS
            && cover.endpoint_roots.len() == cover.cuts.len()
            && cover.variations.len() == cover.cuts.len() + 1,
        "complete cover shape"
    );
    let cuts = cover
        .cuts
        .iter()
        .map(|q| literal(q, MAX_ENDPOINT_BITS, false))
        .collect::<Result<Vec<_>>>()?;
    ensure!(cuts.windows(2).all(|w| w[0] < w[1]), "strict cut ordering");
    let mut roots = 0;
    for (x, expected) in cuts.iter().zip(&cover.endpoint_roots) {
        let actual = endpoint_is_root(p, x);
        ensure!(actual == *expected, "endpoint root claim");
        roots += usize::from(actual);
    }
    for (i, expected) in cover.variations.iter().enumerate() {
        let actual = variation(p, i.checked_sub(1).map(|j| &cuts[j]), cuts.get(i), work)?;
        ensure!(
            actual <= 1 && actual == *expected,
            "root-count certificate failed"
        );
        roots += actual;
    }
    Ok(roots)
}
fn worker(args: &Args) -> Result<()> {
    let bytes = read(&args.input)?;
    ensure!(hash(&bytes) == args.input_sha256, "input digest mismatch");
    let (p, cuts) = load(serde_json::from_slice(&bytes)?)?;
    let start = Instant::now();
    let mut discovery_work = Work {
        shift_transform: args.shift_transform,
        progress_path: Some(args.output.with_extension("progress.json")),
        started: Some(start),
        phase: "discovery".into(),
        ..Default::default()
    };
    discovery_work.progress()?;
    let cover = prove(&p, cuts, &mut discovery_work)?;
    let discovery_seconds = start.elapsed().as_secs_f64();
    // Round-trip evidence and replay without referring to input hints.
    let encoded = serde_json::to_vec(&cover)?;
    let decoded = serde_json::from_slice(&encoded)?;
    let start = Instant::now();
    let mut verification_work = Work {
        shift_transform: args.shift_transform,
        progress_path: Some(args.output.with_extension("progress.json")),
        started: Some(start),
        phase: "independent_replay".into(),
        ..Default::default()
    };
    verification_work.progress()?;
    let roots = verify(&p, &decoded, &mut verification_work)?;
    let report = json!({"schema_version":1,"kind":"research_full_real_descartes_cover",
        "input_sha256":args.input_sha256,"complete":true,"degree":p.len()-1,
        "distinct_real_roots":roots,"discovery_seconds":discovery_seconds,
        "verification_seconds":start.elapsed().as_secs_f64(),"discovery_work":discovery_work,
        "verification_work":verification_work,"cover":cover,"original_cad_coverage":false,
        "source_product_replayed":false,"cross_block_coincidences_resolved":false});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 60.,
        "deadline (0,60]"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists() && !args.output.with_extension("process.json").exists(),
        "fresh output paths required"
    );
    let input = read(&args.input)?;
    ensure!(hash(&input) == args.input_sha256, "input hash mismatch");
    let binary = std::env::current_exe()?;
    let binary_sha256 = hash(&fs::read(&binary)?);
    let mut command = Command::new("taskset");
    command
        .args(["-c", &args.cpu.to_string()])
        .arg(&binary)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: args.seconds,
            memory_mib: 2048,
            workers: 1,
            max_cells: None,
        },
    )?;
    // Preserve the terminal outcome before any post-run input check.
    fs::write(
        args.output.with_extension("process.json"),
        serde_json::to_vec_pretty(&json!({"process":process}))?,
    )?;
    let unchanged = read(&args.input).is_ok_and(|b| hash(&b) == args.input_sha256);
    let report = json!({"process":process,"input_unchanged":unchanged,"binary_sha256":binary_sha256,
        "cpu":args.cpu,"wall_time_secs":args.seconds,"memory_mib":2048,"original_cad_coverage":false});
    fs::write(
        args.output.with_extension("process.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    ensure!(
        unchanged && process.status == "completed" && process.exit_code == Some(0),
        "incomplete bounded root-cover experiment"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn polynomial(text: &str) -> Vec<Integer> {
        let a = symgcad::algebra::Algebra::new(&["x".into()]).unwrap();
        a.parse(text)
            .unwrap()
            .to_univariate_from_univariate(0)
            .coefficients()
            .iter()
            .map(|c| {
                assert!(c.denominator_ref().is_one());
                c.numerator_ref().clone()
            })
            .collect()
    }
    fn check(text: &str, points: &[i64], expected: usize) -> Cover {
        let p = polynomial(text);
        let cover = prove(
            &p,
            points.iter().map(|x| Rational::from(*x)).collect(),
            &mut Work::default(),
        )
        .unwrap();
        assert_eq!(verify(&p, &cover, &mut Work::default()).unwrap(), expected);
        cover
    }
    #[test]
    fn full_line_including_negative_roots_and_empty_gaps() {
        check("(x^2-2)*(x^2+1)*(x-3)", &[-4, 0, 4], 3);
        check("x^2+1", &[0], 0);
    }
    #[test]
    fn exact_endpoint_roots_and_repeated_rational_roots() {
        check("(x+2)^2*x^3*(x-1)*(x-3)^2", &[-2, 0, 1, 3], 4);
    }
    #[test]
    fn narrow_roots_and_missing_hints_are_reconstructed() {
        check("(1000*x-1)*(1000*x-2)*(x+7)", &[0], 3);
    }
    #[test]
    fn altered_claims_and_missing_gaps_are_rejected() {
        let p = polynomial("x^2-2");
        let cover = check("x^2-2", &[-2, 0, 2], 2);
        let mut changed = cover.clone();
        changed.variations[1] = 0;
        assert!(verify(&p, &changed, &mut Work::default()).is_err());
        let mut changed = cover.clone();
        changed.variations.pop();
        assert!(verify(&p, &changed, &mut Work::default()).is_err());
        let mut changed = cover;
        changed.endpoint_roots[0] = true;
        assert!(verify(&p, &changed, &mut Work::default()).is_err());
    }
    #[test]
    fn unresolved_multiple_irrational_roots_decline_and_literals_are_bounded() {
        let p = polynomial("(x^2-2)^2");
        assert!(prove(&p, vec![Rational::zero()], &mut Work::default()).is_err());
        for bad in ["1^999999999", "1/0", "1/2/3", "1e999999", "+2"] {
            assert!(literal(bad, 32, false).is_err());
        }
        let p = polynomial("x^2-2");
        let mut work = Work {
            transform_work_units: MAX_WORK,
            ..Default::default()
        };
        assert!(prove(&p, vec![Rational::zero()], &mut work).is_err());
    }

    #[test]
    fn homogeneous_horner_matches_independent_symbolica_expansion() {
        let algebra = symgcad::algebra::Algebra::new(&["x".into()]).unwrap();
        let x = algebra.parse("x").unwrap();
        let one = algebra.parse("1").unwrap();
        for expression in ["x^5-3*x^3+x-7", "(x-1)^2*(x+2)", "x^4+1"] {
            let p = polynomial(expression);
            let a = rational("-5/3").unwrap();
            let b = rational("1/7").unwrap();
            for (lower, upper) in [(Some(&a), Some(&b)), (None, Some(&b)), (Some(&a), None)] {
                let (linear, denominator) = match (lower, upper) {
                    (Some(a), Some(b)) => (
                        &one.clone().mul_coeff(a.clone()) + &x.clone().mul_coeff(b.clone()),
                        &one + &x,
                    ),
                    (Some(a), None) => (&one.clone().mul_coeff(a.clone()) + &x, one.clone()),
                    (None, Some(b)) => (&one.clone().mul_coeff(b.clone()) - &x, one.clone()),
                    _ => unreachable!(),
                };
                let mut expanded = x.zero();
                for (i, coefficient) in p.iter().enumerate() {
                    expanded = &expanded
                        + &(linear.pow(i) * &denominator.pow(p.len() - 1 - i))
                            .mul_coeff(Rational::from(coefficient.clone()));
                }
                let mut signs = Vec::new();
                for coefficient in expanded.to_univariate_from_univariate(0).coefficients() {
                    if !coefficient.is_zero() {
                        signs.push(coefficient.is_negative());
                    }
                }
                let expected = signs.windows(2).filter(|w| w[0] != w[1]).count();
                assert_eq!(
                    variation(&p, lower, upper, &mut Work::default()).unwrap(),
                    expected
                );
                assert_eq!(
                    variation(
                        &p,
                        lower,
                        upper,
                        &mut Work {
                            shift_transform: true,
                            ..Default::default()
                        }
                    )
                    .unwrap(),
                    expected
                );
            }
        }
    }

    #[test]
    fn two_shifts_equal_full_direct_expansion_including_endpoint_zeros() {
        let algebra = symgcad::algebra::Algebra::new(&["x".into()]).unwrap();
        let x = algebra.parse("x").unwrap();
        let one = algebra.parse("1").unwrap();
        for expression in ["x^5-3*x^3+x-7", "(x-1)^2*(x+2)", "x^4+1", "x^3*(x-2)^2"] {
            let p = polynomial(expression);
            for (a, b, d, finite) in [
                (-35, 3, 21, true),
                (-2, 1, 1, true),
                (0, 2, 1, true),
                (3, -7, 5, true),
                (-5, 3, 3, false),
                (1, -2, 2, false),
            ] {
                let linear = &one.clone().mul_coeff(Rational::from(a))
                    + &x.clone().mul_coeff(Rational::from(b));
                let denominator =
                    if finite { &one + &x } else { one.clone() }.mul_coeff(Rational::from(d));
                let mut expected = x.zero();
                for (i, coefficient) in p.iter().enumerate() {
                    expected = &expected
                        + &(linear.pow(i) * &denominator.pow(p.len() - 1 - i))
                            .mul_coeff(Rational::from(coefficient.clone()));
                }
                let mut actual = shifted_transform(
                    &p,
                    &Integer::from(a),
                    &Integer::from(b),
                    &Integer::from(d),
                    finite,
                    &mut Work::default(),
                )
                .unwrap();
                let mut expected = expected
                    .to_univariate_from_univariate(0)
                    .coefficients()
                    .iter()
                    .map(|c| c.numerator_ref().clone())
                    .collect::<Vec<_>>();
                actual.resize(p.len(), Integer::zero());
                expected.resize(p.len(), Integer::zero());
                assert_eq!(
                    actual, expected,
                    "{expression}: a={a}, b={b}, d={d}, finite={finite}"
                );
            }
        }
    }

    #[test]
    fn shift_discovery_replays_with_horner_and_preserves_repeated_endpoint_roots() {
        for (expression, roots) in [
            ("(x^2-2)*(x^2+1)*(x-3)", 3),
            ("(x+2)^2*x^3*(x-1)*(x-3)^2", 4),
        ] {
            let p = polynomial(expression);
            let cover = prove(
                &p,
                [-2, 0, 1, 3].into_iter().map(Rational::from).collect(),
                &mut Work {
                    shift_transform: true,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(verify(&p, &cover, &mut Work::default()).unwrap(), roots);
        }
        let p = polynomial("(x^2-2)^2");
        assert!(
            prove(
                &p,
                vec![Rational::zero()],
                &mut Work {
                    shift_transform: true,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
