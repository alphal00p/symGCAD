//! Fresh-process matched controls for optional affine factor preconditioning.
//! No operation cache or CAD-completion authority is written by this helper.
#[path = "../src/algebra/affine_factorization.rs"]
mod affine_factorization;
#[allow(dead_code)]
#[path = "profile_operation.rs"]
mod capture_parser;
#[path = "../src/algebra/shear_factorization.rs"]
mod shear_factorization;

use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
#[cfg(test)]
use symgcad::algebra;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Mode {
    Native,
    Policy,
    /// Inspect only the integer-translation admission gate; does not inspect shears.
    Admission,
}
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Synthetic {
    Repeated,
    Bivariate,
    FourVariables,
    MenuSecond,
    MenuExhausted,
    Ordinary,
    AxisCap,
    CoefficientCap,
    Shear,
}
#[derive(Parser)]
struct Args {
    #[arg(required_unless_present = "synthetic", conflicts_with = "synthetic")]
    capture: Option<PathBuf>,
    #[arg(long, requires = "capture")]
    capture_sha256: Option<String>,
    #[arg(long, value_enum)]
    synthetic: Option<Synthetic>,
    #[arg(long, value_enum)]
    mode: Mode,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    #[arg(long, default_value_t = 8)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, cap: u64) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "regular file required"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(cap + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= cap, "input size cap");
    Ok(bytes)
}
fn write(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= 32 * 1024 * 1024, "report size cap");
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn source(args: &Args) -> Result<(Algebra, Poly, Value)> {
    if let Some(kind) = args.synthetic {
        let a = Algebra::new(&[
            "x".into(),
            "unused".into(),
            "y".into(),
            "z".into(),
            "w".into(),
        ])?;
        let expression = match kind {
            Synthetic::Repeated => "-5*((y-1)*x^2+z+2)^2*(x+y+z)^3/7",
            Synthetic::Bivariate => "((y-1)*x^5+y^3+2)*(x^2+y+1)",
            Synthetic::FourVariables => "((y-1)*x^2+z+w)*(x+y*z+w^2)",
            Synthetic::MenuSecond => "(y-1)*(y-3)*(x^2+1)+z+1",
            Synthetic::MenuExhausted => "(y-1)*(y-3)*(y+1)*(y-5)*x^2+z+1",
            Synthetic::Ordinary => "(x^2+y^2+z^2+1)*(x+y+z+2)",
            Synthetic::AxisCap => "(y-1)*x^33+z+1",
            Synthetic::CoefficientCap => "(y-1)*x^2+z+1",
            Synthetic::Shear => "-3/7*((x+y)^2-4*x^2*y^2)^2",
        };
        let mut p = a.parse(expression)?;
        if matches!(kind, Synthetic::CoefficientCap) {
            p.coefficients[0] = Rational::from(Integer::from(2).pow(4097));
        }
        return Ok((
            a,
            p,
            json!({"synthetic":format!("{kind:?}"),"description":expression}),
        ));
    }
    let path = args.capture.as_ref().context("capture required")?;
    let bytes = read(path, 8 * 1024 * 1024)?;
    ensure!(
        args.capture_sha256.as_deref() == Some(hash(&bytes).as_str()),
        "capture hash mismatch"
    );
    let c: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(
        c.schema_version == 1
            && c.coefficient_domain == "Q"
            && c.operation == AlgebraOperation::Factor
            && !c.operands_omitted_for_size,
        "factor capture required"
    );
    let operands = c.operands.as_ref().context("operands required")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == c.input_hash,
        "operand binding"
    );
    let a = Algebra::new(&c.variable_names)?;
    let p = capture_parser::plan_operand(&a, &operands[0])?;
    Ok((
        a,
        p,
        json!({"capture_sha256":hash(&bytes),"input_hash":c.input_hash}),
    ))
}

/// Match the production order of these optional preconditioners. Each declined
/// gate continues; an error after an admitted native factor call propagates.
/// Primitive-linear and equal-derivative sum shortcuts are outside this
/// operand-level experiment and are not claimed to have been profiled here.
fn factor_with_policy(
    p: &Poly,
    mode: Mode,
    mut phase: impl FnMut(&str) -> Result<()>,
) -> Result<(Vec<(Poly, usize)>, &'static str)> {
    if matches!(mode, Mode::Policy) {
        phase("translation_attempt")?;
        if let Some(factors) = affine_factorization::try_factor(p)? {
            return Ok((factors, "integer_translation"));
        }
        phase("shear_attempt")?;
        if let Some(factors) = shear_factorization::try_factor(p)? {
            return Ok((factors, "pairwise_shear"));
        }
    }
    phase("native")?;
    Ok((p.factor(), "native"))
}

fn worker(args: &Args) -> Result<()> {
    let (a, p, origin) = source(args)?;
    ensure!(!p.is_zero(), "zero has no finite factorization");
    let output = args.output.join("worker.json");
    let started = Instant::now();
    let plan = affine_factorization::admission(&p);
    let admission_seconds = started.elapsed().as_secs_f64();
    let mut report = json!({"schema_version":1,"origin":origin,"mode":format!("{:?}",args.mode),
        "source_sha256":hash(a.format(&p).as_bytes()),"variables":a.names,"source_terms":p.nterms(),
        "source_degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),
        "module_sha256":hash(include_bytes!("../src/algebra/affine_factorization.rs")),
        "translation_module_sha256":hash(include_bytes!("../src/algebra/affine_factorization.rs")),
        "shear_module_sha256":hash(include_bytes!("../src/algebra/shear_factorization.rs")),
        "admission_scope":"integer_translation_only; shear admission is not evaluated by Admission mode",
        "policy_order":["integer_translation","pairwise_shear","native"],
        "policy_scope":"optional-preconditioner operand probe; excludes earlier primitive-linear and equal-derivative-sum shortcuts",
        "preconditioner_kind":null,"policy_stage":"not_started",
        "backend":LicenseManager::get_version(),"admitted":plan.is_some(),"admission_seconds":admission_seconds,
        "admission":plan.as_ref().map(|v|json!({"shifts":v.shifts,"original_degree_drops":v.original_degree_drops,"translated_degree_drops":v.translated_degree_drops})),
        "phase":"factor","original_coverage_claimed":false,"cache_insertion":false});
    write(&output, &report)?;
    if matches!(args.mode, Mode::Admission) {
        report["phase"] = json!("admission_complete");
        return write(&output, &report);
    }
    let start = Instant::now();
    let (fs, kind) = factor_with_policy(&p, args.mode, |stage| {
        report["policy_stage"] = json!(stage);
        // A pending preconditioner attempt is not evidence it was admitted.
        // Only the explicit native fallback can be named before completion.
        if stage == "native" {
            report["preconditioner_kind"] = json!("native");
        }
        write(&output, &report)
    })?;
    report["factor_seconds"] = json!(start.elapsed().as_secs_f64());
    report["preconditioner_kind"] = json!(kind);
    report["preconditioner_applied"] = json!(kind != "native");
    report["policy_stage"] = json!("signed_replay");
    write(&output, &report)?;
    let replay_started = Instant::now();
    let mut product = p.one();
    let mut products = 0usize;
    for (f, m) in &fs {
        ensure!(
            f.variables() == p.variables() && !f.is_zero() && *m > 0 && *m <= 64,
            "factor map/multiplicity"
        );
        for _ in 0..*m {
            products = products
                .checked_add(
                    product
                        .nterms()
                        .checked_mul(f.nterms())
                        .context("product overflow")?,
                )
                .context("work overflow")?;
            ensure!(products <= 8_000_000, "independent signed replay work cap");
            product = &product * f;
        }
    }
    ensure!(product == p, "independent signed product mismatch");
    report["independent_signed_replay_seconds"] = json!(replay_started.elapsed().as_secs_f64());
    report["signed_product_verified"] = json!(true);
    report["factors"] = json!(
        fs.iter()
            .map(|(f, m)| json!({"expression":a.format(f),"multiplicity":m,"terms":f.nterms()}))
            .collect::<Vec<_>>()
    );
    report["phase"] = json!("complete");
    write(&output, &report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 30.0,
        "bounded trial cap"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(!args.output.exists(), "fresh output required");
    let pin = if let Some(path) = &args.capture {
        let b = read(path, 8 * 1024 * 1024)?;
        ensure!(
            args.capture_sha256.as_deref() == Some(hash(&b).as_str()),
            "source pin mismatch"
        );
        Some(hash(&b))
    } else {
        None
    };
    fs::create_dir_all(&args.output)?;
    let exe = std::env::current_exe()?;
    let binary = hash(&read(&exe, 1024 * 1024 * 1024)?);
    let mut command = Command::new("taskset");
    command
        .arg("-c")
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
    let process = run_bounded(&mut command, &limits)?;
    write(
        &args.output.join("process.json"),
        &json!({"binary_sha256":binary,"capture_sha256":pin,"process":process,"limits":limits,"cpu":args.cpu,"no_cad_claim":true}),
    )?;
    if let Some(path) = &args.capture {
        ensure!(
            Some(hash(&read(path, 8 * 1024 * 1024)?)) == pin,
            "source changed"
        );
    }
    ensure!(
        hash(&read(&exe, 1024 * 1024 * 1024)?) == binary,
        "binary changed"
    );
    // Timeouts remain durable incomplete evidence, not a claim of success.
    ensure!(
        process.status == "completed" && process.exit_code == Some(0) && !process.output_truncated,
        "incomplete factor trial; evidence retained"
    );
    Ok(())
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn automatic_policy_selects_translation_then_shear_then_native() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into(), "z".into()]).unwrap();
        for (expression, expected, stages) in [
            (
                "(y-1)*x^2+z+2",
                "integer_translation",
                vec!["translation_attempt"],
            ),
            (
                "-3/7*((x+y)^2-4*x^2*y^2)^2",
                "pairwise_shear",
                vec!["translation_attempt", "shear_attempt"],
            ),
            (
                "x^2+y^2+1",
                "native",
                vec!["translation_attempt", "shear_attempt", "native"],
            ),
        ] {
            let p = a.parse(expression).unwrap();
            let mut observed = Vec::new();
            let (factors, kind) = factor_with_policy(&p, Mode::Policy, |stage| {
                observed.push(stage.to_string());
                Ok(())
            })
            .unwrap();
            assert_eq!(kind, expected);
            assert_eq!(observed, stages);
            let mut product = p.one();
            for (factor, multiplicity) in factors {
                assert_eq!(factor.variables(), p.variables());
                assert_eq!(factor.degree(1), 0);
                product = &product * &factor.pow(multiplicity);
            }
            assert_eq!(product, p);
        }
    }

    #[test]
    fn native_control_never_enters_either_preconditioner() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = a.parse("(x+y)^2-4*x^2*y^2").unwrap();
        let mut observed = Vec::new();
        let (_, kind) = factor_with_policy(&p, Mode::Native, |stage| {
            observed.push(stage.to_string());
            Ok(())
        })
        .unwrap();
        assert_eq!(kind, "native");
        assert_eq!(observed, ["native"]);
    }
}
