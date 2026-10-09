//! Same-build exact-evaluation ablation on a captured univariate integer fiber.
//! Includes the production evaluator verbatim; no diagnostic production API.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{hint::black_box, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{Limits, algebra, benchmark::run_bounded};

#[path = "../src/roots/integer_sign.rs"]
mod integer_sign;

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 1000)]
    iterations: usize,
    #[arg(long, default_value_t = 5)]
    repeats: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 7)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Deserialize)]
struct Capture {
    degree: usize,
    coefficients_low_to_high: Vec<String>,
}

fn rational_sign(poly: &UnivariatePolynomial<Q>, x: &Rational) -> i8 {
    let value = poly.evaluate(x);
    if value.is_zero() {
        0
    } else if value.is_negative() {
        -1
    } else {
        1
    }
}

fn timed(
    poly: &UnivariatePolynomial<Q>,
    points: &[Rational],
    iterations: usize,
    evaluate: fn(&UnivariatePolynomial<Q>, &Rational) -> i8,
) -> (f64, i64) {
    let started = Instant::now();
    let mut checksum = 0i64;
    for _ in 0..iterations {
        for point in points {
            checksum += black_box(evaluate(black_box(poly), black_box(point))) as i64;
        }
    }
    (started.elapsed().as_secs_f64(), black_box(checksum))
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn work(args: &Args) -> Result<serde_json::Value> {
    let bytes = std::fs::read(&args.capture)?;
    let capture: Capture = serde_json::from_slice(&bytes)?;
    ensure!(capture.degree <= 256, "research profile degree exceeds 256");
    let prototype = algebra::Algebra::new(&["x".into()])?
        .parse("x")?
        .to_univariate_from_univariate(0);
    let coefficients = capture
        .coefficients_low_to_high
        .iter()
        .map(|value| algebra::rational(value))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        coefficients.iter().all(|value| value.is_integer()),
        "capture must have integer coefficients"
    );
    let poly = UnivariatePolynomial::from_coefficients(&Q, coefficients, prototype.get_vars());
    ensure!(poly.degree() == capture.degree, "capture degree differs");
    let mut groups = Vec::new();
    // Positive and negative points near integers at several dyadic precisions.
    // These are controlled sign-evaluation inputs, not a root-isolation timing.
    for bits in [0u64, 8, 32, 128, 512] {
        let denominator = Integer::from(2).pow(bits);
        let mut points = Vec::new();
        for base in -4..=4 {
            for offset in [-1, 1] {
                points.push(
                    Rational::from(base)
                        + Rational::new(Integer::from(offset), denominator.clone()),
                );
            }
        }
        let signs: Vec<_> = points.iter().map(|x| rational_sign(&poly, x)).collect();
        for (point, &expected) in points.iter().zip(&signs) {
            ensure!(
                integer_sign::sign(&poly, point) == expected,
                "exact sign differs at {point}"
            );
        }
        let expected_checksum =
            signs.iter().map(|&v| v as i64).sum::<i64>() * i64::try_from(args.iterations)?;
        let mut rational_seconds = Vec::new();
        let mut integer_seconds = Vec::new();
        for repeat in 0..args.repeats {
            // Alternate execution order to reduce a systematic warm-cache bias.
            let evaluators: [(bool, fn(&UnivariatePolynomial<Q>, &Rational) -> i8); 2] =
                if repeat % 2 == 0 {
                    [(false, rational_sign), (true, integer_sign::sign)]
                } else {
                    [(true, integer_sign::sign), (false, rational_sign)]
                };
            for (integer, evaluate) in evaluators {
                let (seconds, checksum) = timed(&poly, &points, args.iterations, evaluate);
                ensure!(checksum == expected_checksum, "timed sign checksum differs");
                if integer {
                    integer_seconds.push(seconds);
                } else {
                    rational_seconds.push(seconds);
                }
            }
        }
        groups.push(json!({
            "dyadic_denominator_exponent":bits,
            "points":points.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "signs":signs,
            "calls_per_repeat":points.len()*args.iterations,
            "rational_seconds":rational_seconds,
            "integer_seconds":integer_seconds,
            "rational_median_seconds":median(&rational_seconds),
            "integer_median_seconds":median(&integer_seconds),
            "rational_over_integer_ratio":median(&rational_seconds)/median(&integer_seconds),
        }));
    }
    Ok(json!({
        "capture":args.capture,
        "capture_sha256":format!("{:x}",Sha256::digest(bytes)),
        "integer_sign_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("../src/roots/integer_sign.rs"))),
        "degree":poly.degree(),
        "coefficient_bits_sum":poly.coefficients().iter().map(|v|v.numerator_ref().significant_bits()).sum::<u64>(),
        "iterations":args.iterations,
        "repeats":args.repeats,
        "all_signs_equal":true,
        "groups":groups,
        "scope":"Same-build sign-evaluation microbenchmark only; does not measure whole root isolation or GCAD.",
    }))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        (1..=1_000_000).contains(&args.iterations) && (1..=9).contains(&args.repeats),
        "iterations or repeats exceed profile bounds"
    );
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "profile wall deadline must be within 120 seconds"
    );
    if args.worker {
        println!("{}", work(&args)?);
        return Ok(());
    }
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
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
    let mut outcome = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<serde_json::Value>(&outcome.stdout).ok();
    if result.is_some() {
        outcome.stdout.clear();
    }
    let report = json!({
        "process":outcome,"result":result,"limits":limits,"cpu":args.cpu,
        "binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?)),
    });
    std::fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}
