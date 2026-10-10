//! Bounded exact boundary-shift screen; never factors or accepts a CAD proof.
use anyhow::{ensure, Result};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    algebra::{rational, Algebra, Poly},
    benchmark::run_bounded,
    Limits,
};

#[derive(Parser)]
struct Args {
    candidate: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Deserialize)]
struct Candidate {
    operation: String,
    input_hash: String,
    result: String,
    result_sha256: String,
    result_terms: usize,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        fs::metadata(path)?.is_file(),
        "candidate is not a regular file"
    );
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() <= 16 * 1024 * 1024, "candidate byte cap");
    Ok(data)
}
// Accept the saved canonical integer-univariate grammar only. Bound every
// exponent/literal before any Symbolica parse or power allocation.
fn parse_canonical(algebra: &Algebra, expression: &str) -> Result<Poly> {
    let mut remaining = expression;
    let mut p = algebra.parse("0")?;
    let mut last = None;
    let mut count = 0usize;
    loop {
        remaining = remaining
            .strip_prefix('(')
            .ok_or_else(|| anyhow::anyhow!("expected coefficient"))?;
        let (coefficient, tail) = remaining
            .split_once(')')
            .ok_or_else(|| anyhow::anyhow!("coefficient terminator"))?;
        let digits = coefficient.strip_prefix('-').unwrap_or(coefficient);
        ensure!(
            !digits.is_empty()
                && digits.len() <= 6000
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "integer coefficient cap/grammar"
        );
        let tail = tail
            .strip_prefix("*s^")
            .ok_or_else(|| anyhow::anyhow!("only explicit s powers supported"))?;
        let end = tail.find('+').unwrap_or(tail.len());
        let power = &tail[..end];
        ensure!(
            !power.is_empty() && power.len() <= 4 && power.bytes().all(|b| b.is_ascii_digit()),
            "exponent grammar"
        );
        let exponent: u32 = power.parse()?;
        ensure!(
            exponent <= 4096 && last.is_none_or(|x| exponent > x),
            "degree/order cap"
        );
        last = Some(exponent);
        count += 1;
        ensure!(count <= 4096, "term cap");
        let coefficient = rational(coefficient)?;
        ensure!(!coefficient.is_zero(), "zero input coefficient");
        p.append_monomial(coefficient, &[exponent]);
        if end == tail.len() {
            break;
        }
        remaining = &tail[end + 1..];
    }
    Ok(p)
}
fn sign(q: &Rational) -> i8 {
    if q.is_zero() {
        0
    } else if q.is_negative() {
        -1
    } else {
        1
    }
}
fn worker_report(stdout: &str) -> Option<serde_json::Value> {
    // Symbolica may print a startup banner before our compact JSON line.
    serde_json::from_str(stdout.lines().rev().find(|line| !line.trim().is_empty())?).ok()
}
fn screen(algebra: &Algebra, p: &Poly) -> Result<serde_json::Value> {
    let started = Instant::now();
    let power = p.exponents_iter().map(|m| m[0]).min().unwrap();
    let mut stripped = p.zero();
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        stripped.append_monomial(c.clone(), &[m[0] - power]);
    }
    ensure!(
        stripped.clone().mul_monomial(&Rational::one(), &[power]) == *p,
        "monomial stripping identity"
    );
    let univariate = stripped.to_univariate_from_univariate(0);
    let degree = univariate.degree();
    ensure!(
        degree <= 4096 && degree.saturating_mul(degree + 1) / 2 <= 9_000_000,
        "shift work cap"
    );
    let input_bits = univariate
        .coefficients()
        .iter()
        .map(|c| c.numerator_ref().significant_bits())
        .max()
        .unwrap_or(0);
    // A coefficient of q(s+4) is bounded by ||q||_1*5^degree.
    let output_bits = input_bits + 3 * degree as u64 + 13;
    ensure!(
        output_bits <= 32768 && output_bits.saturating_mul((degree + 1) as u64) <= 128_000_000,
        "shift coefficient storage bound"
    );
    let samples = ["4", "9/2", "5", "6", "8", "10", "16", "32", "64", "128"]
        .into_iter()
        .map(|s| Ok(json!({"s":s,"stripped_sign":sign(&univariate.evaluate(&rational(s)?))})))
        .collect::<Result<Vec<_>>>()?;
    let domain_samples = &samples[1..]; // s=4 is outside the strict domain.
    let brackets = domain_samples
        .windows(2)
        .filter(|pair| {
            pair[0]["stripped_sign"].as_i64().unwrap() * pair[1]["stripped_sign"].as_i64().unwrap()
                == -1
        })
        .map(|pair| json!([pair[0]["s"], pair[1]["s"]]))
        .collect::<Vec<_>>();
    let sampled_signs = domain_samples
        .iter()
        .map(|sample| sample["stripped_sign"].as_i64().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let refuted =
        sampled_signs.contains(&0) || (sampled_signs.contains(&-1) && sampled_signs.contains(&1));
    let shift_started = Instant::now();
    let shifted = univariate.shift_var(&Rational::from(4));
    let shift_seconds = shift_started.elapsed().as_secs_f64();
    let inverse_started = Instant::now();
    ensure!(
        shifted.shift_var(&Rational::from(-4)) == univariate,
        "exact inverse-shift identity"
    );
    let inverse_seconds = inverse_started.elapsed().as_secs_f64();
    let signs = shifted.coefficients().iter().map(sign).collect::<Vec<_>>();
    let positive = signs.iter().filter(|&&s| s > 0).count();
    let negative = signs.iter().filter(|&&s| s < 0).count();
    let boundary_power = signs.iter().take_while(|&&s| s == 0).count();
    let certified_sign = match (positive, negative) {
        (0, n) if n > 0 => Some(-1),
        (n, 0) if n > 0 => Some(1),
        _ => None,
    };
    let shifted_poly = shifted.clone().to_multivariate::<u32>();
    Ok(
        json!({"input_terms":p.nterms(),"input_degree":p.degree(0),"removed_s_power":power,
        "stripped_degree":degree,"input_max_coefficient_bits":input_bits,"shift_output_bit_bound":output_bits,
        "shift_work_add_multiply_count":degree*(degree+1)/2,"shifted_terms":positive+negative,
        "positive_coefficients":positive,"negative_coefficients":negative,"s_minus_four_multiplicity":boundary_power,
        "shifted_signs_low_to_high":signs,"shifted_polynomial_sha256":hash(algebra.format(&shifted_poly).as_bytes()),
        "exact_stripping_identity":true,"exact_inverse_shift_identity":true,"samples":samples,
        "strict_sign_on_s_greater_four":certified_sign,
        "nonvanishing_refuted_by_exact_samples":refuted,"interior_root_brackets":brackets,
        "outcome":if refuted {"global nonvanishing refuted by exact interior samples"}else if certified_sign.is_some(){"uniform shifted coefficients prove candidate nonvanishing on s>4"}else{"inconclusive mixed shifted coefficients"},
        "shift_seconds":shift_seconds,"inverse_identity_seconds":inverse_seconds,"total_seconds":started.elapsed().as_secs_f64(),
        "limitations":["No factorization, root count, CAD completion or production pruning", "Candidate discriminant identity requires separately verified full-grid evidence", "Sample signs can refute a sign claim but cannot establish one"]}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "deadline must be in (0,60]"
    );
    if args.worker {
        let bytes = read(&args.candidate)?;
        let candidate: Candidate = serde_json::from_slice(&bytes)?;
        ensure!(
            candidate.operation == "discriminant"
                && candidate.result_sha256 == hash(candidate.result.as_bytes()),
            "candidate hash/operation mismatch"
        );
        let algebra = Algebra::new(&["s".into()])?;
        let p = parse_canonical(&algebra, &candidate.result)?;
        ensure!(
            p.nterms() == candidate.result_terms,
            "candidate term count mismatch"
        );
        println!(
            "{}",
            json!({"candidate_sha256":hash(&bytes),"discriminant_sha256":candidate.result_sha256,"input_hash":candidate.input_hash,"screen":screen(&algebra,&p)?})
        );
        return Ok(());
    }
    ensure!(!args.output.exists(), "output must be fresh");
    let executable = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut process = run_bounded(&mut command, &limits)?;
    let result = worker_report(&process.stdout);
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.is_some();
    if result.is_some() {
        process.stdout.clear();
    }
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    file.write_all(&serde_json::to_vec_pretty(&json!({"limits":limits,"cpu":args.cpu,"process":process,"result":result,"binary_sha256":hash(&fs::read(executable)?)}))?)?;
    ensure!(success, "bounded boundary screen did not complete");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_negative_boundary_and_mixed_sign_cases() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let p = a.parse("-s^2*(s-4)^3*(s+1)").unwrap();
        let r = screen(&a, &p).unwrap();
        assert_eq!(r["removed_s_power"], 2);
        assert_eq!(r["s_minus_four_multiplicity"], 3);
        assert_eq!(r["strict_sign_on_s_greater_four"], -1);
        let r = screen(&a, &a.parse("s^2*(s-5)").unwrap()).unwrap();
        assert!(r["strict_sign_on_s_greater_four"].is_null());
    }
    #[test]
    fn parser_caps_powers_and_exactly_retains_sign() {
        let a = Algebra::new(&["s".into()]).unwrap();
        assert_eq!(
            parse_canonical(&a, "(2)*s^2+(-3)*s^4").unwrap(),
            a.parse("2*s^2-3*s^4").unwrap()
        );
        for bad in [
            "(1)*s^999999999",
            "(2^3)*s^2",
            "(1)*s^2+(1)*s^2",
            "(0)*s^1",
            "(1)*t^1",
        ] {
            assert!(parse_canonical(&a, bad).is_err());
        }
    }
    #[test]
    fn banner_cannot_hide_or_fabricate_final_worker_report() {
        assert_eq!(
            worker_report("banner\n{\"screen\":{}}\n"),
            Some(json!({"screen":{}}))
        );
        assert!(worker_report("{\"screen\":{}}\ntrailing error\n").is_none());
    }
}
