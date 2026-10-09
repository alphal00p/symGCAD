//! Bounded exact symbolic expansion of the second pair cancellation.
//! This is a diagnostic, not a production coverage certificate.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command, time::Instant};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    optimized_projection::OperationCapture,
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    prefix: Vec<String>,
    #[arg(long)]
    reference: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    #[arg(long)]
    factor_coefficients: bool,
    #[arg(long, hide = true)]
    worker: bool,
}

fn cancellation(high: &Poly, low: &Poly, variable: usize) -> Result<(Poly, usize, f64)> {
    ensure!(high.degree(variable) >= low.degree(variable));
    let a = high.univariate_lcoeff(variable);
    let b = low.univariate_lcoeff(variable);
    let estimate = high
        .nterms()
        .saturating_mul(b.nterms())
        .saturating_add(low.nterms().saturating_mul(a.nterms()));
    ensure!(estimate <= 100_000, "symbolic multiplication cap exceeded");
    let mut powers = vec![0; high.nvars()];
    powers[variable] = high.degree(variable) - low.degree(variable);
    let start = Instant::now();
    let result = &b * high - &a * &low.clone().mul_monomial(&Rational::one(), &powers);
    Ok((result, estimate, start.elapsed().as_secs_f64()))
}

fn worker(args: &Args) -> Result<serde_json::Value> {
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("capture omitted operands"))?;
    ensure!(operands.len() == 2, "two operands required");
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture checksum mismatch"
    );
    let variable = capture
        .variable
        .ok_or_else(|| anyhow::anyhow!("capture omitted variable"))?;
    ensure!(args.prefix.len() == variable, "prefix has wrong length");
    let algebra = Algebra::new(&capture.variable_names)?;
    let prefix = args
        .prefix
        .iter()
        .map(|x| rational(x))
        .collect::<Result<Vec<_>>>()?;
    let mut p = algebra.parse(&operands[0])?;
    let mut q = algebra.parse(&operands[1])?;
    if p.degree(variable) < q.degree(variable) {
        std::mem::swap(&mut p, &mut q);
    }
    let (first, first_estimate, first_seconds) = cancellation(&p, &q, variable)?;
    ensure!(
        !first.is_zero() && first.degree(variable) < q.degree(variable),
        "first cancellation is not a completed nonzero remainder"
    );
    let (second, second_estimate, second_seconds) = cancellation(&q, &first, variable)?;
    let fiber = algebra.specialize_univariate(&second, variable, &prefix)?;
    let coefficients = fiber
        .coefficients()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let reference_bytes = std::fs::read(&args.reference)?;
    let reference: serde_json::Value = serde_json::from_slice(&reference_bytes)?;
    ensure!(
        json!(coefficients) == reference["numeric_prs"][1]["fiber"]["coefficients_low_to_high"],
        "symbolic specialization does not match independent numeric cancellation"
    );
    let sign = if fiber.lcoeff().is_negative() { -1 } else { 1 };
    let same_sign = fiber
        .coefficients()
        .iter()
        .all(|c| !c.is_zero() && if c.is_negative() { -1 } else { 1 } == sign);
    let mut guards = Vec::new();
    for (coefficient, power) in second.to_univariate_polynomial_list(variable) {
        let start = Instant::now();
        let factors = args.factor_coefficients.then(|| {
            coefficient
                .factor()
                .into_iter()
                .map(|(p, multiplicity)| {
                    json!({"expression":algebra.format(&p),"terms":p.nterms(),"multiplicity":multiplicity})
                })
                .collect::<Vec<_>>()
        });
        guards.push(json!({"power":power,"terms":coefficient.nterms(),
            "expression":algebra.format(&coefficient),"factor_seconds":start.elapsed().as_secs_f64(),"factors":factors}));
    }
    Ok(
        json!({"status":"symbolic_cancellation_diagnostic_only", "capture":args.capture,
        "capture_sha256":format!("{:x}",Sha256::digest(bytes)),
        "reference_sha256":format!("{:x}",Sha256::digest(reference_bytes)),
        "variables":capture.variable_names,"variable":variable,"prefix":args.prefix,
        "input_terms":[p.nterms(),q.nterms()],"input_degrees":[p.degree(variable),q.degree(variable)],
        "first":{"terms":first.nterms(),"degree":first.degree(variable),"leading_terms":first.univariate_lcoeff(variable).nterms(),"product_estimate":first_estimate,"seconds":first_seconds},
        "second":{"terms":second.nterms(),"degree":second.degree(variable),"product_estimate":second_estimate,"seconds":second_seconds,"expression":algebra.format(&second)},
        "coefficients_low_to_high":coefficients,"same_strict_sign_at_prefix":same_sign,
        "specialization_matches_numeric_reference":true,"coefficient_guards":guards}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        println!("{}", serde_json::to_string(&worker(&args)?)?);
        return Ok(());
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new(&executable);
    command
        .arg(&args.capture)
        .arg(format!("--prefix={}", args.prefix.join(",")))
        .arg("--reference")
        .arg(&args.reference)
        .arg("--output")
        .arg(&args.output)
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    if args.factor_coefficients {
        command.arg("--factor-coefficients");
    }
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut outcome = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<serde_json::Value>(&outcome.stdout).ok();
    if result.is_some() {
        outcome.stdout.clear();
    }
    let report = json!({"process":outcome,"result":result,"limits":limits,
        "binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?))});
    std::fs::write(&args.output, serde_json::to_string_pretty(&report)? + "\n")?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
