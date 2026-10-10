//! Bounded exact translation/support diagnostic; no discriminants or cache writes.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 12)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    ensure!(fs::metadata(path)?.is_file(), "regular capture required");
    let mut b = Vec::new();
    fs::File::open(path)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut b)?;
    ensure!(b.len() <= 2 * 1024 * 1024, "capture byte cap");
    Ok(b)
}
// Accept only signed integer sparse canonical terms; never parse arbitrary powers.
fn parse(a: &Algebra, expression: &str) -> Result<Poly> {
    ensure!(expression.len() <= 1024 * 1024, "expression byte cap");
    let mut p = a.parse("0")?;
    let mut seen = HashSet::new();
    for (i, term) in expression.split('+').enumerate() {
        ensure!(i < 5000, "term cap");
        let (c, tail) = term
            .strip_prefix('(')
            .context("coefficient prefix")?
            .split_once(')')
            .context("coefficient suffix")?;
        let digits = c.strip_prefix('-').unwrap_or(c);
        ensure!(
            !digits.is_empty() && digits.len() <= 620 && digits.bytes().all(|b| b.is_ascii_digit()),
            "integer coefficient grammar/height"
        );
        let c = rational(c)?;
        ensure!(
            !c.is_zero() && c.numerator_ref().significant_bits() <= 2048,
            "coefficient bound"
        );
        let mut e = vec![0; a.names.len()];
        if !tail.is_empty() {
            for factor in tail
                .strip_prefix('*')
                .context("monomial prefix")?
                .split('*')
            {
                let (name, power) = factor.split_once('^').unwrap_or((factor, "1"));
                let v = a
                    .names
                    .iter()
                    .position(|n| n == name)
                    .context("unknown variable")?;
                ensure!(
                    !power.is_empty()
                        && power.len() <= 3
                        && power.bytes().all(|b| b.is_ascii_digit()),
                    "power literal"
                );
                let power: u32 = power.parse()?;
                ensure!(
                    e[v] == 0 && power > 0 && power <= 256,
                    "repeated variable or degree cap"
                );
                e[v] = power;
            }
        }
        ensure!(
            e.iter().sum::<u32>() <= 256 && seen.insert(e.clone()),
            "total degree or duplicate term"
        );
        p.append_monomial(c, &e);
    }
    Ok(p)
}
fn coefficient(p: &Poly, v: usize, power: u32) -> Poly {
    let mut q = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        if e[v] == power {
            let mut e = e.to_vec();
            e[v] = 0;
            q.append_monomial(c.clone(), &e);
        }
    }
    q
}
fn stats(p: &Poly) -> Value {
    json!({"terms":p.nterms(), "degrees":(0..p.nvars()).map(|i| p.degree(i)).collect::<Vec<_>>(), "maximum_rational_height":p.coefficients.iter().map(|c| c.numerator_ref().significant_bits()+c.denominator_ref().significant_bits()).max().unwrap_or(0)})
}
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
fn screen(a: &Algebra, p: &Poly, v: usize) -> Result<Value> {
    let started = Instant::now();
    ensure!(
        v < p.nvars() && (2..=64).contains(&p.degree(v)),
        "fiber degree cap"
    );
    ensure!(
        (0..p.nvars()).filter(|i| p.degree(*i) > 0).count() <= 2,
        "at most two active coordinates"
    );
    let n = p.degree(v);
    let leading = coefficient(p, v, n);
    let next = coefficient(p, v, n - 1);
    let denominator = leading.clone().mul_coeff(Rational::from(n));
    let minus_next = next.mul_coeff(Rational::from(-1));
    let then = Instant::now();
    let (h, remainder) = minus_next.quot_rem(&denominator, false);
    ensure!(
        &denominator * &h + remainder.clone() == minus_next,
        "center division reconstruction"
    );
    let quotient_seconds = then.elapsed().as_secs_f64();
    let mut report = json!({"input":stats(p),"leading":stats(&leading),"next_coefficient":stats(&coefficient(p,v,n-1)),"center_quotient":stats(&h),"center_remainder":stats(&remainder),"exact_quotient_identity":true,"quotient_seconds":quotient_seconds,"coverage_claimed":false,"production_changes":false});
    if !remainder.is_zero() {
        report["outcome"] = json!("declined: center is not polynomial");
        report["total_seconds"] = json!(started.elapsed().as_secs_f64());
        return Ok(report);
    }
    if h.degree(v) != 0
        || h.nterms() > 3
        || (0..p.nvars()).any(|i| h.degree(i) > 2)
        || h.coefficients.iter().any(|c| {
            c.numerator_ref().significant_bits() + c.denominator_ref().significant_bits() > 1024
        })
    {
        report["outcome"] = json!("declined: polynomial center exceeds bounded portfolio");
        report["total_seconds"] = json!(started.elapsed().as_secs_f64());
        return Ok(report);
    }
    // For a degree-two univariate h, every h^k has at most 2k+1 terms.
    let expansion_bound = p
        .nterms()
        .checked_mul((n as usize + 1) * (2 * n as usize + 1))
        .context("expansion overflow")?;
    ensure!(expansion_bound <= 22_000_000, "shift expansion envelope");
    report["raw_term_contribution_bound"] = json!(expansion_bound);
    report["center"] = json!(a.format(&h));
    let axis = a.parse(&a.names[v])?;
    let then = Instant::now();
    let shifted = p.replace_with_poly(v, &(&axis + &h));
    ensure!(shifted.nterms() <= 20000, "shift result term cap");
    ensure!(
        coefficient(&shifted, v, n - 1).is_zero(),
        "centering did not remove penultimate coefficient"
    );
    report["shift_seconds"] = json!(then.elapsed().as_secs_f64());
    let then = Instant::now();
    ensure!(
        shifted.replace_with_poly(v, &(&axis - &h)) == *p,
        "inverse shift identity failed"
    );
    report["inverse_identity_seconds"] = json!(then.elapsed().as_secs_f64());
    report["exact_inverse_identity"] = json!(true);
    report["shifted"] = stats(&shifted);
    let powers: Vec<u32> = shifted
        .exponents_iter()
        .map(|e| e[v])
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let k = powers
        .iter()
        .copied()
        .filter(|e| *e > 0)
        .reduce(gcd)
        .unwrap_or(0);
    report["support_power_gcd"] = json!(k);
    let odd = shifted.exponents_iter().filter(|e| e[v] % 2 == 1).count();
    report["odd_power_terms"] = json!(odd);
    let expression = a.format(&shifted);
    ensure!(expression.len() <= 8 * 1024 * 1024, "shift expression cap");
    report["shifted_sha256"] = json!(hash(expression.as_bytes()));
    if k > 1 {
        let mut q = p.zero();
        for (c, e) in shifted.coefficients.iter().zip(shifted.exponents_iter()) {
            let mut e = e.to_vec();
            e[v] /= k;
            q.append_monomial(c.clone(), &e);
        }
        ensure!(
            q.replace_with_poly(v, &axis.pow(k as usize)) == shifted,
            "power support reconstruction failed"
        );
        report["compressed"] = stats(&q);
        report["compressed_polynomial"] = json!(a.format(&q));
        report["exact_power_identity"] = json!(true);
        report["outcome"] = json!("exact polynomial translation and power-support compression");
    } else {
        report["outcome"] = json!("no nontrivial power-support compression after exact centering");
    }
    report["total_seconds"] = json!(started.elapsed().as_secs_f64());
    Ok(report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "deadline must be (0,60]"
    );
    if args.worker {
        let bytes = read(&args.capture)?;
        let capture: OperationCapture = serde_json::from_slice(&bytes)?;
        ensure!(
            capture.schema_version == 1 && capture.operation == AlgebraOperation::Discriminant,
            "capture schema/operation"
        );
        let operands = capture.operands.context("missing operands")?;
        ensure!(
            operands.len() == 1 && capture.input_hash == hash(&serde_json::to_vec(&operands)?),
            "capture operand hash"
        );
        ensure!(
            !capture.variable_names.is_empty()
                && capture.variable_names.len() <= 16
                && capture.variable_names.iter().all(|n| !n.is_empty()
                    && n.len() <= 32
                    && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')),
            "variable map bound"
        );
        ensure!(
            capture.variable_names.iter().collect::<HashSet<_>>().len()
                == capture.variable_names.len(),
            "duplicate variables"
        );
        let a = Algebra::new(&capture.variable_names)?;
        let p = parse(&a, &operands[0])?;
        let v = capture.variable.context("missing axis")?;
        println!(
            "{}",
            json!({"kind":"research_centered_discriminant_support","capture_sha256":hash(&bytes),"input_hash":capture.input_hash,"variables":a.names,"variable":v,"screen":screen(&a,&p,v)?})
        );
        return Ok(());
    }
    ensure!(!args.output.exists(), "output must be fresh");
    let before = hash(&read(&args.capture)?);
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new("taskset");
    cmd.arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
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
    let mut process = run_bounded(&mut cmd, &limits)?;
    let result = process
        .stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .next_back()
        .and_then(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["kind"] == "research_centered_discriminant_support");
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.is_some();
    if result.is_some() {
        process.stdout.clear();
    }
    let after = hash(&read(&args.capture)?);
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    file.write_all(&serde_json::to_vec_pretty(&json!({"schema_version":1,"capture_sha256":before,"capture_unchanged":before==after,"binary_sha256":hash(&fs::read(exe)?),"limits":limits,"cpu":args.cpu,"process":process,"result":result}))?)?;
    ensure!(
        success && before == after,
        "bounded diagnostic incomplete or source changed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_even_polynomial_reconstructs() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        let p = a.parse("3*(t-s-1)^4+2*(t-s-1)^2+s").unwrap();
        let r = screen(&a, &p, 1).unwrap();
        assert_eq!(
            a.parse(r["center"].as_str().unwrap()).unwrap(),
            a.parse("s+1").unwrap()
        );
        assert_eq!(r["support_power_gcd"], 2);
        assert_eq!(r["exact_inverse_identity"], true);
        let q = a
            .parse(r["compressed_polynomial"].as_str().unwrap())
            .unwrap();
        assert_eq!(q, a.parse("3*t^2+2*t+s").unwrap());
    }
    #[test]
    fn asymmetric_and_nonpolynomial_centers_decline() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        let r = screen(&a, &a.parse("t^4+t+s").unwrap(), 1).unwrap();
        assert_eq!(r["support_power_gcd"], 1);
        let r = screen(&a, &a.parse("s*t^4+t^3+1").unwrap(), 1).unwrap();
        assert_eq!(r["outcome"], "declined: center is not polynomial");
    }
    #[test]
    fn parser_bounds_and_signed_sparse_roundtrip() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        assert_eq!(
            parse(&a, "(2)+(-3)*t^2+(4)*s*t").unwrap(),
            a.parse("2-3*t^2+4*s*t").unwrap()
        );
        for bad in [
            "(2^1000000)",
            "(1)*s^999999999",
            "(1)*t*t",
            "(1)*t+(1)*t",
            "(0)",
            "(1)*unknown",
        ] {
            assert!(parse(&a, bad).is_err(), "{bad}");
        }
    }
}
