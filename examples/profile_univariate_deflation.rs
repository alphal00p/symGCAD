//! Research-only exact deflation/SQF/factor stages, each in one bounded worker.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Stage {
    Deflate,
    SquareFree,
    Factor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Operand {
    Original,
    Residual,
}
#[derive(Parser)]
struct Args {
    candidate: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_enum)]
    stage: Stage,
    #[arg(long, value_enum, default_value = "residual")]
    operand: Operand,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    /// Save exact signed reconstruction factors after the complete identity check.
    #[arg(long)]
    save_factors: bool,
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
    ensure!(fs::metadata(path)?.is_file(), "candidate is not regular");
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 16 * 1024 * 1024, "candidate byte cap");
    Ok(bytes)
}
// The captured signed integer grammar is checked before any algebra parse or power.
fn parse(algebra: &Algebra, expression: &str) -> Result<Poly> {
    let mut tail = expression;
    let mut p = algebra.parse("0")?;
    let mut last = None;
    let mut count = 0;
    loop {
        let text = tail
            .strip_prefix('(')
            .ok_or_else(|| anyhow::anyhow!("coefficient"))?;
        let (coefficient, text) = text
            .split_once(')')
            .ok_or_else(|| anyhow::anyhow!("coefficient end"))?;
        let digits = coefficient.strip_prefix('-').unwrap_or(coefficient);
        ensure!(
            !digits.is_empty()
                && digits.len() <= 6000
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "integer height/grammar"
        );
        let text = text
            .strip_prefix("*s^")
            .ok_or_else(|| anyhow::anyhow!("power"))?;
        let end = text.find('+').unwrap_or(text.len());
        let power = &text[..end];
        ensure!(
            !power.is_empty() && power.len() <= 4 && power.bytes().all(|b| b.is_ascii_digit()),
            "power grammar"
        );
        let power: u32 = power.parse()?;
        ensure!(
            power <= 4096 && last.is_none_or(|old| power > old),
            "degree/order cap"
        );
        count += 1;
        ensure!(count <= 4096, "term cap");
        last = Some(power);
        let coefficient = rational(coefficient)?;
        ensure!(!coefficient.is_zero(), "zero term");
        p.append_monomial(coefficient, &[power]);
        if end == text.len() {
            break;
        }
        tail = &text[end + 1..];
    }
    Ok(p)
}
fn summary(a: &Algebra, p: &Poly) -> Value {
    json!({"degree":p.degree(0),"terms":p.nterms(),"sha256":hash(a.format(p).as_bytes()),
        "max_numerator_bits":p.coefficients.iter().map(|c|c.numerator_ref().significant_bits()).max().unwrap_or(0),
        "max_denominator_bits":p.coefficients.iter().map(|c|c.denominator_ref().significant_bits()).max().unwrap_or(0)})
}
fn reconstruct(p: &Poly, factors: &[(Poly, usize)]) -> Result<()> {
    ensure!(!factors.is_empty() && factors.len() <= 4097, "factor count");
    let mut degree = 0u64;
    for (f, m) in factors {
        ensure!(!f.is_zero() && *m > 0 && *m <= 4096, "factor/multiplicity");
        degree = degree
            .checked_add(u64::from(f.degree(0)) * (*m as u64))
            .ok_or_else(|| anyhow::anyhow!("degree overflow"))?;
    }
    ensure!(degree == u64::from(p.degree(0)), "reconstruction degree");
    let mut product = p.one();
    for (f, m) in factors {
        product = &product * &f.pow(*m);
    }
    ensure!(product == *p, "signed factor reconstruction");
    Ok(())
}
fn factor_bundle(
    a: &Algebra,
    p: &Poly,
    factors: &[(Poly, usize)],
    candidate: &str,
) -> Result<Value> {
    let mut expressions = Vec::new();
    let mut total_bytes = 0usize;
    for (f, multiplicity) in factors {
        let mut expression = String::new();
        for (index, (c, e)) in f.coefficients.iter().zip(f.exponents_iter()).enumerate() {
            ensure!(
                c.is_integer(),
                "factor export requires integer coefficients"
            );
            let coefficient = c.to_string();
            ensure!(
                coefficient.trim_start_matches('-').len() <= 6000,
                "factor literal cap"
            );
            if index != 0 {
                expression.push('+');
            }
            expression.push_str(&format!("({coefficient})*s^{}", e[0]));
        }
        total_bytes = total_bytes
            .checked_add(expression.len())
            .ok_or_else(|| anyhow::anyhow!("export size overflow"))?;
        ensure!(total_bytes <= 16 * 1024 * 1024, "factor export byte cap");
        ensure!(parse(a, &expression)? == *f, "factor export roundtrip");
        expressions.push(json!({"multiplicity":multiplicity,"polynomial":summary(a,f),"expression_sha256":hash(expression.as_bytes()),"expression":expression}));
    }
    Ok(
        json!({"schema_version":1,"candidate_sha256":candidate,"original":summary(a,p),"variable":"s",
        "exact_signed_original_reconstruction":true,"factors":expressions,
        "note":"Research proposal only: consumer must independently reconstruct the candidate; no irreducibility or root-count authority."}),
    )
}
fn deflate(a: &Algebra, p: &Poly) -> Result<(Poly, Vec<(Poly, usize)>)> {
    let zero_power = p
        .exponents_iter()
        .map(|e| e[0])
        .min()
        .ok_or_else(|| anyhow::anyhow!("empty input"))?;
    let mut q = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        q.append_monomial(c.clone(), &[e[0] - zero_power]);
    }
    ensure!(
        q.clone().mul_monomial(&Rational::one(), &[zero_power]) == *p,
        "coordinate identity"
    );
    let divisor = a.parse("s-4")?;
    let linear = divisor.to_univariate_from_univariate(0);
    let mut univariate = q.to_univariate_from_univariate(0);
    let mut four_power = 0;
    loop {
        if !univariate.evaluate(&Rational::from(4)).is_zero() {
            break;
        }
        ensure!(
            four_power < 64 && univariate.degree() > 0,
            "threshold multiplicity cap"
        );
        let quotient = univariate
            .try_div(&linear)
            .ok_or_else(|| anyhow::anyhow!("native exact linear division failed"))?;
        ensure!(
            &quotient * &linear == univariate,
            "linear division identity"
        );
        univariate = quotient;
        four_power += 1;
    }
    let residual = univariate.to_multivariate::<u32>();
    let mut factors = Vec::new();
    if zero_power > 0 {
        factors.push((a.parse("s")?, zero_power as usize));
    }
    if four_power > 0 {
        factors.push((divisor, four_power));
    }
    let mut complete = factors.clone();
    complete.push((residual.clone(), 1));
    reconstruct(p, &complete)?;
    Ok((residual, factors))
}
fn progress(path: &Path, value: &Value) -> Result<()> {
    let temp = path.with_extension("progress.tmp");
    let target = path.with_extension("progress.json");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, target)?;
    Ok(())
}
fn worker(args: &Args) -> Result<()> {
    let bytes = read(&args.candidate)?;
    let candidate: Candidate = serde_json::from_slice(&bytes)?;
    ensure!(
        candidate.operation == "discriminant"
            && hash(candidate.result.as_bytes()) == candidate.result_sha256,
        "candidate binding"
    );
    let a = Algebra::new(&["s".into()])?;
    let p = parse(&a, &candidate.result)?;
    ensure!(p.nterms() == candidate.result_terms, "term count");
    let mut report = json!({"candidate_sha256":hash(&bytes),"input_hash":candidate.input_hash,"discriminant_sha256":candidate.result_sha256,
        "stage":format!("{:?}",args.stage),"operand":format!("{:?}",args.operand),"original":summary(&a,&p),"phase":"parsed"});
    progress(&args.output, &report)?;
    let started = Instant::now();
    let (input, removed) = if args.operand == Operand::Residual || args.stage == Stage::Deflate {
        deflate(&a, &p)?
    } else {
        (p.clone(), vec![])
    };
    report["preparation_seconds"] = json!(started.elapsed().as_secs_f64());
    report["operand_summary"] = summary(&a, &input);
    report["removed"] = json!(
        removed
            .iter()
            .map(|(f, m)| json!({"polynomial":a.format(f),"multiplicity":m}))
            .collect::<Vec<_>>()
    );
    report["exact_preparation_identity"] = json!(true);
    report["phase"] = json!("prepared");
    progress(&args.output, &report)?;
    if args.stage != Stage::Deflate {
        report["phase"] = json!("native_stage");
        progress(&args.output, &report)?;
        let started = Instant::now();
        let factors = match args.stage {
            Stage::SquareFree => input.square_free_factorization(),
            Stage::Factor => input.factor(),
            Stage::Deflate => unreachable!(),
        };
        report["native_stage_seconds"] = json!(started.elapsed().as_secs_f64());
        report["factors"] = json!(
            factors
                .iter()
                .map(|(f, m)| json!({"polynomial":summary(&a,f),"multiplicity":m}))
                .collect::<Vec<_>>()
        );
        report["phase"] = json!("reconstruction");
        progress(&args.output, &report)?;
        let started = Instant::now();
        reconstruct(&input, &factors)?;
        let mut original_factors = removed;
        original_factors.extend(factors);
        reconstruct(&p, &original_factors)?;
        report["reconstruction_seconds"] = json!(started.elapsed().as_secs_f64());
        report["exact_signed_original_reconstruction"] = json!(true);
        if args.save_factors {
            report["phase"] = json!("factor_export");
            progress(&args.output, &report)?;
            let bundle = factor_bundle(&a, &p, &original_factors, &hash(&bytes))?;
            let encoded = serde_json::to_vec(&bundle)?;
            ensure!(
                encoded.len() <= 17 * 1024 * 1024,
                "serialized factor export cap"
            );
            let path = args.output.with_extension("factors.json");
            let temporary = args.output.with_extension("factors.tmp");
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&encoded)?;
            ensure!(!path.exists(), "factor export destination is not fresh");
            fs::rename(temporary, &path)?;
            report["factor_bundle"] =
                json!({"path":path,"sha256":hash(&encoded),"bytes":encoded.len()});
        }
    }
    report["phase"] = json!("complete");
    progress(&args.output, &report)?;
    println!("{report}");
    Ok(())
}
fn worker_report(stdout: &str) -> Option<Value> {
    serde_json::from_str(stdout.lines().rev().find(|s| !s.trim().is_empty())?).ok()
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "deadline (0,60]"
    );
    ensure!(
        !args.save_factors || args.stage != Stage::Deflate,
        "factor export requires the square-free or factor stage"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists()
            && !args.output.with_extension("progress.json").exists()
            && !args.output.with_extension("progress.tmp").exists()
            && !args.output.with_extension("factors.json").exists()
            && !args.output.with_extension("factors.tmp").exists(),
        "fresh output required"
    );
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
        .arg(&args.candidate)
        .arg("--output")
        .arg(&args.output)
        .arg("--stage")
        .arg(args.stage.to_possible_value().unwrap().get_name())
        .arg("--operand")
        .arg(args.operand.to_possible_value().unwrap().get_name())
        .arg("--worker");
    if args.save_factors {
        command.arg("--save-factors");
    }
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
        && result.as_ref().is_some_and(|v| v["phase"] == "complete");
    if result.is_some() {
        process.stdout.clear();
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    file.write_all(&serde_json::to_vec_pretty(&json!({"schema_version":1,"limits":limits,"cpu":args.cpu,"binary_sha256":hash(&fs::read(executable)?),"process":process,"result":result,
        "note":"Research-only exact factor preparation. No guard omission, root count, proof acceptance or original CAD completion."}))?)?;
    ensure!(
        success,
        "bounded stage incomplete; process and progress retained"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_deflation_retains_sign_and_all_multiplicities() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let p = a.parse("-6*s^3*(s-4)^8*(s^2+1)^2").unwrap();
        let (q, removed) = deflate(&a, &p).unwrap();
        assert_eq!(q, a.parse("-6*(s^2+1)^2").unwrap());
        assert_eq!(
            removed,
            vec![(a.parse("s").unwrap(), 3), (a.parse("s-4").unwrap(), 8)]
        );
        let sqf = q.square_free_factorization();
        reconstruct(&q, &sqf).unwrap();
        assert!(sqf.iter().any(|(f, m)| f.degree(0) == 2 && *m == 2));
        let fs = q.factor();
        reconstruct(&q, &fs).unwrap();
        let mut all = removed;
        all.extend(fs);
        reconstruct(&p, &all).unwrap();
        let bundle = factor_bundle(&a, &p, &all, "test-candidate").unwrap();
        let replay = bundle["factors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    parse(&a, v["expression"].as_str().unwrap()).unwrap(),
                    v["multiplicity"].as_u64().unwrap() as usize,
                )
            })
            .collect::<Vec<_>>();
        reconstruct(&p, &replay).unwrap();
    }
    #[test]
    fn declined_threshold_does_not_divide_or_change_polynomial() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let p = a.parse("-3*s^2+s-1").unwrap();
        let (q, removed) = deflate(&a, &p).unwrap();
        assert_eq!(q, p);
        assert!(removed.is_empty());
        assert!(reconstruct(&p, &[(a.parse("s").unwrap(), 2)]).is_err());
        assert!(deflate(&a, &a.parse("(s-4)^65").unwrap()).is_err());
    }
    #[test]
    fn untrusted_input_and_worker_report_are_bounded() {
        let a = Algebra::new(&["s".into()]).unwrap();
        assert_eq!(
            parse(&a, "(-2)*s^0+(1)*s^3").unwrap(),
            a.parse("s^3-2").unwrap()
        );
        for bad in [
            "(2^3)*s^0",
            "(1)*s^99999",
            "(1)*s^2+(1)*s^1",
            "(0)*s^0",
            "(1)*t^0",
        ] {
            assert!(parse(&a, bad).is_err());
        }
        assert_eq!(
            worker_report("banner\n{\"phase\":\"complete\"}\n"),
            Some(json!({"phase":"complete"}))
        );
        assert!(worker_report("{}\ntrailing error").is_none());
    }
}
