//! Bounded research-only inspection of a captured factor's leading coefficient.
//! Replays the pinned backend's necessary Wang image gates, not an assignment
//! certificate. No production cache or factorization policy is changed.
#[allow(dead_code)]
#[path = "profile_operation.rs"]
mod capture_parser;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    axis: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 8)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write(path: &std::path::Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= 8 * 1024 * 1024, "report byte cap");
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)?;
    Ok(())
}
fn bounded(p: &Poly, terms: usize) -> Result<()> {
    ensure!(
        p.nvars() <= 64 && p.nterms() <= terms && (0..p.nvars()).all(|i| p.degree(i) <= 32),
        "polynomial shape bound"
    );
    ensure!(
        p.coefficients
            .iter()
            .all(|c| c.numerator_ref().significant_bits() <= 8192
                && c.denominator_ref().significant_bits() <= 8192),
        "coefficient height cap"
    );
    Ok(())
}
fn describe(a: &Algebra, p: &Poly) -> Value {
    json!({"expression":a.format(p),"terms":p.nterms(),
        "degrees":(0..p.nvars()).map(|i| p.degree(i)).collect::<Vec<_>>(),
        "coefficient_bits":p.coefficients.iter().map(|c| c.numerator_ref().significant_bits()
            .max(c.denominator_ref().significant_bits())).max().unwrap_or(0)})
}

fn primitive_leading(a: &Algebra, p: &Poly, axis: usize) -> Result<(Poly, Rational)> {
    ensure!(
        p.variables() == &a.variables,
        "full source variable map mismatch"
    );
    bounded(p, 8192)?;
    ensure!(
        axis < p.nvars() && p.degree(axis) > 0,
        "active source axis required"
    );
    ensure!(
        (0..p.nvars()).filter(|&i| p.degree(i) > 0).count() <= 3,
        "active source axis cap"
    );
    let content = p.content();
    ensure!(!content.is_zero(), "zero source content");
    let primitive = p.clone().div_coeff(&content);
    ensure!(
        primitive.coefficients.iter().all(Rational::is_integer)
            && primitive.clone().mul_coeff(content.clone()) == *p,
        "primitive signed source identity"
    );
    let leading = primitive.univariate_lcoeff(axis);
    bounded(&leading, 128)?;
    ensure!(
        !leading.is_zero()
            && (0..leading.nvars())
                .filter(|&i| leading.degree(i) > 0)
                .count()
                <= 2,
        "leading coefficient admission"
    );
    Ok((leading, content))
}

fn checked_factors(a: &Algebra, leading: &Poly) -> Result<Vec<(Poly, usize)>> {
    let factors = leading.factor();
    ensure!(factors.len() <= 32, "leading factor count cap");
    let mut product = leading.one();
    let mut work = 0usize;
    for (f, multiplicity) in &factors {
        ensure!(
            f.variables() == &a.variables && *multiplicity > 0 && *multiplicity <= 32,
            "leading factor map/multiplicity cap"
        );
        bounded(f, 128)?;
        for _ in 0..*multiplicity {
            work = work
                .checked_add(
                    product
                        .nterms()
                        .checked_mul(f.nterms())
                        .context("reconstruction work overflow")?,
                )
                .context("reconstruction work overflow")?;
            ensure!(work <= 1_048_576, "leading reconstruction work cap");
            product = &product * f;
            bounded(&product, 8192)?;
        }
    }
    ensure!(
        product == *leading,
        "signed leading factor product mismatch"
    );
    Ok(factors)
}

fn primitive_variable(p: &Poly) -> bool {
    p.nterms() == 1
        && p.lcoeff().abs().is_one()
        && p.exponents.iter().filter(|&&e| e != 0).count() == 1
        && p.exponents.contains(&1)
}

fn wang_preflight(
    leading: &Poly,
    factors: &[(Poly, usize)],
    point: &[(usize, i64)],
) -> Result<Value> {
    ensure!(
        leading.coefficients.iter().all(Rational::is_integer),
        "integer leading coefficient required"
    );
    let content = leading.content().abs();
    ensure!(content.is_integer(), "integer leading content required");
    let content = content.numerator();
    let mut images = Vec::<Integer>::new();
    let mut factors_report = Vec::new();
    let mut admitted = true;
    for (f, multiplicity) in factors {
        if f.is_constant() {
            continue;
        }
        ensure!(
            f.variables() == leading.variables(),
            "leading factor variable map mismatch"
        );
        let mut evaluated = f.clone();
        for &(axis, value) in point {
            ensure!(
                axis < f.nvars() && value.unsigned_abs() <= 4096,
                "evaluation point bound"
            );
            evaluated = evaluated.replace(axis, &Rational::from(value));
        }
        ensure!(
            evaluated.is_constant() && evaluated.get_constant().is_integer(),
            "complete integer specialization required"
        );
        let image = evaluated.get_constant().numerator().abs();
        let content_gcd = image.gcd(&content);
        let previous_gcds = images.iter().map(|old| old.gcd(&image)).collect::<Vec<_>>();
        let is_variable = primitive_variable(f);
        let passes = image > Integer::one()
            && (!is_variable || content_gcd.is_one())
            && previous_gcds.iter().all(Integer::is_one);
        admitted &= passes;
        factors_report.push(
            json!({"multiplicity":multiplicity,"absolute_image":image.to_string(),
            "primitive_variable":is_variable,"content_gcd":content_gcd.to_string(),
            "previous_image_gcds":previous_gcds.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "passes_necessary_image_gates":passes}),
        );
        images.push(image);
    }
    Ok(
        json!({"point":point,"leading_integer_content":content.to_string(),"factors":factors_report,
        "passes_necessary_image_gates":admitted,"leading_assignment_certified":false,
        "scope":"Necessary pinned-backend image gates only. Factor multiplicities must still be assigned and the exact signed leading product reconstructed."}),
    )
}

fn worker(args: &Args) -> Result<()> {
    let bytes = capture_parser::bounded_capture_bytes(&args.capture)?;
    ensure!(hash(&bytes) == args.capture_sha256, "capture hash mismatch");
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(
        capture.schema_version == 1
            && capture.coefficient_domain == "Q"
            && capture.operation == AlgebraOperation::Factor
            && !capture.operands_omitted_for_size,
        "complete rational factor capture required"
    );
    ensure!(
        !capture.variable_names.is_empty()
            && capture.variable_names.len() <= 64
            && capture.variable_names.iter().all(|n| n.len() <= 64),
        "bounded source variable map required"
    );
    let operands = capture.operands.as_ref().context("missing operands")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == capture.input_hash,
        "operand binding"
    );
    let a = Algebra::new(&capture.variable_names)?;
    let p = capture_parser::plan_operand(&a, &operands[0])?;
    let (leading, removed_content) = primitive_leading(&a, &p, args.axis)?;
    let mut report = json!({"schema_version":1,"capture_sha256":hash(&bytes),"input_hash":capture.input_hash,
        "variables":capture.variable_names,"axis":args.axis,"source":describe(&a,&p),
        "source_removed_content":removed_content.to_string(),"source_primitive_identity_verified":true,
        "leading":describe(&a,&leading),"phase":"factor_leading","original_coverage_claimed":false,
        "cache_insertion":false,"helper_source_sha256":hash(include_bytes!("profile_wang_leading.rs"))});
    let output = args.output.join("worker.json");
    write(&output, &report)?;
    let start = Instant::now();
    let factors = checked_factors(&a, &leading)?;
    report["factor_and_reconstruction_seconds"] = json!(start.elapsed().as_secs_f64());
    report["signed_leading_product_verified"] = json!(true);
    report["factors"] = json!(
        factors
            .iter()
            .map(|(f, m)| json!({"multiplicity":m,"polynomial":describe(&a,f)}))
            .collect::<Vec<_>>()
    );
    report["preflight"] = json!([]);
    let active = (0..p.nvars())
        .filter(|&i| p.degree(i) > 0)
        .collect::<Vec<_>>();
    for (label, k) in [("ones", 0), ("shift1", 1), ("shift-1", -1), ("shift2", 2)] {
        let point = active
            .iter()
            .enumerate()
            .filter(|(_, i)| **i != args.axis)
            .map(|(j, &i)| (i, 1 + k * (j as i64 + 1)))
            .collect::<Vec<_>>();
        let mut row = wang_preflight(&leading, &factors, &point)?;
        row["label"] = json!(label);
        report["preflight"].as_array_mut().unwrap().push(row);
    }
    let evaluated = active
        .into_iter()
        .filter(|&i| i != args.axis)
        .collect::<Vec<_>>();
    for primes in [[2, 3], [3, 5], [5, 7], [7, 11], [11, 13]] {
        let point = evaluated
            .iter()
            .zip(primes)
            .map(|(&i, v)| (i, v))
            .collect::<Vec<_>>();
        let mut row = wang_preflight(&leading, &factors, &point)?;
        row["label"] = json!(format!("primes{},{}", primes[0], primes[1]));
        report["preflight"].as_array_mut().unwrap().push(row);
    }
    report["phase"] = json!("complete");
    write(&output, &report)
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "bounded research deadline"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(!args.output.exists(), "fresh output required");
    fs::create_dir_all(&args.output)?;
    let exe = std::env::current_exe()?;
    let binary_hash = hash(&fs::read(&exe)?);
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .args(std::env::args_os().skip(1))
        .arg("--worker");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let process = run_bounded(&mut command, &limits)?;
    write(
        &args.output.join("process.json"),
        &json!({"process":process,"limits":limits,"cpu":args.cpu,
        "binary_sha256":binary_hash,"capture_sha256":args.capture_sha256}),
    )?;
    ensure!(
        process.status == "completed" && process.exit_code == Some(0) && !process.output_truncated,
        "incomplete leading diagnostic"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primitive_source_and_signed_lc_factor_product_keep_full_map() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into(), "z".into()]).unwrap();
        let p = a.parse("-3/7*((y+1)^2*(z+1)*x^2+y*x+1)").unwrap();
        let (lc, content) = primitive_leading(&a, &p, 0).unwrap();
        assert!(content.abs() == Rational::from((3, 7)));
        let factors = checked_factors(&a, &lc).unwrap();
        assert!(factors.iter().any(|(_, m)| *m == 2));
        assert!(
            factors
                .iter()
                .all(|(f, _)| f.variables() == &a.variables && f.degree(1) == 0)
        );
        let wrong = Algebra::new(&["y".into(), "unused".into(), "x".into(), "z".into()]).unwrap();
        assert!(primitive_leading(&wrong, &p, 0).is_err());
        assert!(primitive_leading(&a, &p, 1).is_err());
    }
    #[test]
    fn wang_necessary_gates_reject_shared_primes_variable_content_and_small_images() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into(), "z".into()]).unwrap();
        let lc = a.parse("2*y*(z+1)").unwrap();
        let factors = checked_factors(&a, &lc).unwrap();
        let yes = wang_preflight(&lc, &factors, &[(2, 3), (3, 3)]).unwrap();
        assert_eq!(yes["passes_necessary_image_gates"], true);
        assert_eq!(yes["leading_assignment_certified"], false);
        for point in [
            [(2, 2), (3, 2)],
            [(2, 3), (3, 2)],
            [(2, 1), (3, 3)],
            [(2, 0), (3, 3)],
        ] {
            assert_eq!(
                wang_preflight(&lc, &factors, &point).unwrap()["passes_necessary_image_gates"],
                false
            );
        }
        assert!(wang_preflight(&lc, &factors, &[(2, 3)]).is_err());
    }
}
