//! Same-build root-union ablation with positive rational content removed.
//! Fibers are reconstructed from retained exact factor records at a saved
//! prefix, not claimed to be the historical worker's actual simultaneous cuts.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet, fs, hint::black_box, path::PathBuf, process::Command, time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, rational},
    benchmark::run_bounded,
    roots::{FiberRoot, isolate_union, refine_once},
};

#[derive(Parser)]
struct Args {
    directory: PathBuf,
    progress: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 24)]
    cases: usize,
    #[arg(long, default_value_t = 10)]
    iterations: usize,
    #[arg(long, default_value_t = 3)]
    repeats: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 0)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Deserialize)]
struct Operation {
    operation: String,
    variable_names: Vec<String>,
    output: Vec<String>,
    multiplicities: Option<Vec<usize>>,
    output_hash: String,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn integer_fiber(p: UnivariatePolynomial<Q>) -> UnivariatePolynomial<Q> {
    if p.coefficients().iter().all(|c| c.is_integer()) {
        return p;
    }
    let positive_content = p.content().abs();
    p.div_coeff(&positive_content)
}

fn signature(roots: &[FiberRoot]) -> Vec<(usize, usize, (Rational, Rational))> {
    roots
        .iter()
        .map(|r| (r.polynomial, r.index, r.interval.clone()))
        .collect()
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}

fn work(args: &Args) -> Result<serde_json::Value> {
    let progress_bytes = fs::read(&args.progress)?;
    let progress: serde_json::Value = serde_json::from_slice(&progress_bytes)?;
    let prefix_text: Vec<String> = serde_json::from_value(progress["current_prefix"].clone())?;
    let prefix = prefix_text
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let level = progress["current_level"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing axis"))? as usize;
    ensure!(level == prefix.len() && level <= 16, "invalid saved prefix");
    // Bounds apply before reading/parsing individual records. The retained
    // source list fixes enumeration choices without assuming directory order.
    let mut inputs = Vec::new();
    let mut sources = Vec::new();
    let mut seen = BTreeSet::new();
    let mut variables = None;
    for entry in fs::read_dir(&args.directory)?.take(10000) {
        if inputs.len() == args.cases {
            break;
        }
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") || fs::metadata(&path)?.len() > 65536 {
            continue;
        }
        let bytes = fs::read(&path)?;
        let record: Operation = serde_json::from_slice(&bytes)?;
        if record.operation != "factor"
            || record.variable_names.len() > 16
            || level >= record.variable_names.len()
        {
            continue;
        }
        ensure!(
            hash(&serde_json::to_vec(&(
                &record.output,
                &record.multiplicities
            ))?) == record.output_hash,
            "factor output hash mismatch"
        );
        if variables
            .as_ref()
            .is_some_and(|v| v != &record.variable_names)
        {
            continue;
        }
        let algebra = Algebra::new(&record.variable_names)?;
        for expression in &record.output {
            if inputs.len() == args.cases {
                break;
            }
            let p = algebra.parse(expression)?;
            if p.nterms() > 4096
                || !(2..=16).contains(&p.degree(level))
                || (level + 1..p.nvars()).any(|i| p.degree(i) > 0)
            {
                continue;
            }
            let fiber = algebra.specialize_univariate(&p, level, &prefix)?;
            if fiber.degree() != p.degree(level) as usize
                || fiber.coefficients().iter().all(|c| c.is_integer())
            {
                continue;
            }
            let coefficients: Vec<_> = fiber
                .coefficients()
                .iter()
                .map(ToString::to_string)
                .collect();
            if coefficients.iter().map(String::len).sum::<usize>() > 65536
                || !seen.insert(coefficients.clone())
            {
                continue;
            }
            let normalized = integer_fiber(fiber.clone());
            ensure!(
                normalized.coefficients().iter().all(|c| c.is_integer()),
                "content did not clear denominators"
            );
            let scale = fiber.lcoeff() / normalized.lcoeff();
            ensure!(
                scale > Rational::zero() && normalized.clone().mul_coeff(&scale) == fiber,
                "positive scaling identity failed"
            );
            variables = Some(record.variable_names.clone());
            let id = inputs.len();
            sources.push(json!({"path":path,"record_sha256":hash(&bytes),"expression":expression,"fiber_coefficients":coefficients,"id":id,"degree":fiber.degree(),"positive_scale":scale.to_string()}));
            inputs.push((id, fiber));
        }
    }
    ensure!(
        !inputs.is_empty(),
        "no eligible rational fibers at saved prefix"
    );
    let normalized: Vec<_> = inputs
        .iter()
        .map(|(id, p)| (*id, integer_fiber(p.clone())))
        .collect();
    let initial_roots = isolate_union(inputs.clone())?;
    let expected = signature(&initial_roots);
    ensure!(
        signature(&isolate_union(normalized.clone())?) == expected,
        "root owners, ordinals or exact intervals changed"
    );
    let mut original_times = Vec::new();
    let mut normalized_times = Vec::new();
    for repetition in 0..args.repeats {
        for normalize in if repetition % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let started = Instant::now();
            for _ in 0..args.iterations {
                let data = black_box(inputs.clone());
                let data = if normalize {
                    data.into_iter()
                        .map(|(id, p)| (id, integer_fiber(p)))
                        .collect()
                } else {
                    data
                };
                let roots = isolate_union(data)?;
                ensure!(
                    signature(black_box(&roots)) == expected,
                    "timed result changed"
                );
            }
            let elapsed = started.elapsed().as_secs_f64();
            if normalize {
                normalized_times.push(elapsed);
            } else {
                original_times.push(elapsed);
            }
        }
    }
    // Leave VAS and square-free reduction untouched in this second experiment.
    // Charge normalization and cloning on each pass before 32 exact bisections.
    let mut refined = initial_roots.clone();
    for root in &mut refined {
        for _ in 0..32 {
            refine_once(root);
        }
    }
    let refined_signature = signature(&refined);
    let mut refinement_original = Vec::new();
    let mut refinement_normalized = Vec::new();
    for repetition in 0..args.repeats {
        for normalize in if repetition % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let started = Instant::now();
            for _ in 0..args.iterations {
                let mut roots = black_box(initial_roots.clone());
                for root in &mut roots {
                    if normalize {
                        root.fiber = integer_fiber(root.fiber.as_ref().clone()).into();
                    }
                    for _ in 0..32 {
                        refine_once(root);
                    }
                }
                ensure!(
                    signature(black_box(&roots)) == refined_signature,
                    "exact refinement changed"
                );
            }
            if normalize {
                refinement_normalized.push(started.elapsed().as_secs_f64());
            } else {
                refinement_original.push(started.elapsed().as_secs_f64());
            }
        }
    }
    Ok(
        json!({"schema_version":1,"scope":"Same-build reconstructed-fiber root-union microbenchmark, not captured simultaneous cuts or whole-solve timings","progress":args.progress,"progress_sha256":hash(&progress_bytes),"prefix":prefix_text,"variables":variables,"sources":sources,"roots":expected.len(),"exact_signatures_equal":true,"iterations":args.iterations,"repeats":args.repeats,"normalization_included":true,"original_median_seconds":median(&original_times),"normalized_median_seconds":median(&normalized_times),"original_seconds":original_times,"normalized_seconds":normalized_times,
        "refinement":{"steps":32,"nonexact_roots":initial_roots.iter().filter(|r|r.interval.0!=r.interval.1).count(),"original_median_seconds":median(&refinement_original),"normalized_median_seconds":median(&refinement_normalized),"original_seconds":refinement_original,"normalized_seconds":refinement_normalized},
        "helper_source_sha256":hash(include_bytes!("profile_rational_fibers.rs")),"roots_source_sha256":hash(include_bytes!("../src/roots.rs"))}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        (1..=64).contains(&args.cases)
            && (1..=100).contains(&args.iterations)
            && (1..=5).contains(&args.repeats),
        "research bounds exceeded"
    );
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "invalid worker deadline"
    );
    if args.worker {
        println!("{}", work(&args)?);
        return Ok(());
    }
    ensure!(!args.output.exists(), "output already exists");
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
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
        memory_mib: 2048,
        workers: 1,
        max_cells: None,
    };
    let mut process = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<serde_json::Value>(&process.stdout).ok();
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.is_some();
    if result.is_some() {
        process.stdout.clear();
    }
    let report = json!({"process":process,"result":result,"limits":limits,"cpu":args.cpu,"binary_sha256":hash(&fs::read(executable)?)});
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    ensure!(success, "rational-fiber profile did not complete");
    println!("{}", args.output.display());
    Ok(())
}
