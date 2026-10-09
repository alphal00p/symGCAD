//! Independent API compatibility: all extracted ordering/projection invocations.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symgcad::{
    Limits,
    algebra::{Algebra, canonical},
    benchmark::run_bounded,
    corpus::import_core_entries,
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "artifacts/corpus/core-inputs.json")]
    core: PathBuf,
    #[arg(long, default_value = ".reference/gcad")]
    reference_root: PathBuf,
    #[arg(long, default_value = "reference/api_adapter.py")]
    adapter: PathBuf,
    #[arg(long, default_value = "artifacts/bin/symgcad-baseline-3125469")]
    solver: PathBuf,
    #[arg(long, default_value = "artifacts/bench/core-api")]
    output: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    timeout: f64,
    #[arg(long, default_value_t = 1200.0)]
    campaign_time_secs: f64,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn canonical_levels(result: &Value, native: bool) -> Result<Vec<BTreeSet<String>>> {
    let order: Vec<String> = serde_json::from_value(result["order"].clone())?;
    let algebra = Algebra::new(&order)?;
    result["levels"]
        .as_array()
        .context("projection levels missing")?
        .iter()
        .map(|level| {
            level
                .as_array()
                .context("projection level is not a list")?
                .iter()
                .map(|entry| {
                    let text = if native {
                        entry
                            .as_str()
                            .context("reference polynomial is not a string")?
                    } else {
                        result["polynomials"][entry
                            .as_u64()
                            .context("projection index is not an integer")?
                            as usize]
                            .as_str()
                            .context("projection polynomial missing")?
                    };
                    Ok(algebra.format(&canonical(&algebra.parse(text)?)))
                })
                .collect()
        })
        .collect()
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.timeout.is_finite() && args.timeout > 0.0 && args.timeout <= 3600.0,
        "invalid timeout"
    );
    ensure!(
        args.campaign_time_secs.is_finite()
            && args.campaign_time_secs > 0.0
            && args.campaign_time_secs <= 3600.0,
        "invalid campaign budget"
    );
    ensure!(
        args.memory_mib > 0 && args.memory_mib <= 65536,
        "invalid memory cap"
    );
    fs::create_dir_all(&args.output)?;
    let reference_metadata: Value = serde_json::from_reader(fs::File::open(
        args.reference_root
            .parent()
            .unwrap()
            .join("provenance.json"),
    )?)?;
    let python = reference_metadata["python_executable"]
        .as_str()
        .context("reference interpreter missing")?;
    let solver = args.solver.canonicalize()?;
    let adapter = args.adapter.canonicalize()?;
    let binary_hash = hash_file(&solver)?;
    let mut report = json!({"schema_version":1,"operation":"API compatibility, not a solve benchmark","provenance":{
        "solver":solver,"solver_sha256":binary_hash,"adapter_sha256":hash_file(&adapter)?,
        "shared_adapter_sha256":hash_file(&adapter.parent().unwrap().join("adapter.py"))?,
        "core_inputs_sha256":hash_file(&args.core)?,"reference_revision":reference_metadata["revision"],
        "workers":1,"memory_mib":args.memory_mib,"timeout_secs":args.timeout
    },"cases":[],"elapsed_secs":0.0});
    let started = Instant::now();
    let entries = import_core_entries(&args.core)?
        .into_iter()
        .filter(|entry| entry.operation != "solve");
    for (index, entry) in entries.enumerate() {
        let mut row = json!({"source":entry.source,"name":entry.problem.name,"operation":entry.operation,"status":"not_started"});
        let remaining = || (args.campaign_time_secs - started.elapsed().as_secs_f64()).max(0.0);
        if remaining() < 0.05 {
            row["status"] = json!("campaign_exhausted");
            report["cases"].as_array_mut().unwrap().push(row);
            continue;
        }
        let mut input = serde_json::to_value(&entry.problem)?;
        // Input extraction provides baseline defaults. Retain the M1 schema so
        // the committed baseline executable can be tested after new options land.
        input["solver"].as_object_mut().unwrap().retain(|key, _| {
            [
                "engine",
                "ordering",
                "order",
                "merge",
                "seed",
                "max_rollouts",
                "search_time_secs",
            ]
            .contains(&key.as_str())
        });
        input["limits"] = serde_json::to_value(Limits {
            wall_time_secs: args.timeout,
            memory_mib: args.memory_mib,
            workers: 1,
            max_cells: None,
        })?;
        let input_file = args.output.join(format!("{index:04}-input.json"));
        fs::write(&input_file, serde_json::to_vec_pretty(&input)?)?;
        row["input_sha256"] = json!(hash_file(&input_file)?);
        let mut limits = Limits {
            wall_time_secs: args.timeout.min(remaining()),
            memory_mib: args.memory_mib,
            workers: 1,
            max_cells: None,
        };
        let mut reference = Command::new(python);
        reference
            .env("OPENBLAS_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1");
        reference
            .arg(&adapter)
            .arg(&entry.operation)
            .arg("--reference-root")
            .arg(&args.reference_root)
            .arg("--problem-file")
            .arg(&input_file);
        let reference = run_bounded(&mut reference, &limits)?;
        let expected: Option<Value> = if reference.status == "completed" {
            serde_json::from_str(&reference.stdout).ok()
        } else {
            None
        };
        fs::write(
            args.output.join(format!("{index:04}-reference.json")),
            serde_json::to_vec_pretty(&json!({"process":reference,"result":expected}))?,
        )?;
        row["reference_status"] = json!(reference.status);
        if remaining() < 0.05 {
            row["status"] = json!("campaign_exhausted");
            report["cases"].as_array_mut().unwrap().push(row);
            continue;
        }
        let result_file = args.output.join(format!("{index:04}-rust-result.json"));
        if result_file.exists() {
            fs::remove_file(&result_file)?;
        }
        let mut rust = Command::new(&solver);
        rust.arg("_worker")
            .arg(&entry.operation)
            .arg(&input_file)
            .arg("--output")
            .arg(&result_file);
        limits.wall_time_secs = args.timeout.min(remaining());
        let rust = run_bounded(&mut rust, &limits)?;
        let actual: Option<Value> = if rust.status == "completed" {
            fs::read(&result_file)
                .ok()
                .and_then(|s| serde_json::from_slice(&s).ok())
        } else {
            None
        };
        fs::write(
            args.output.join(format!("{index:04}-rust.json")),
            serde_json::to_vec_pretty(&json!({"process":rust,"result":actual}))?,
        )?;
        row["rust_status"] = json!(rust.status);
        row["status"] = json!(if expected.is_none() {
            format!("reference_{}", reference.status)
        } else if actual.is_none() {
            format!("rust_{}", rust.status)
        } else {
            "compared".into()
        });
        if let (Some(expected), Some(actual)) = (expected, actual) {
            let mut matches = expected["order"] == actual["order"];
            row["reference_order"] = expected["order"].clone();
            row["rust_order"] = actual["order"].clone();
            if entry.operation == "project" {
                let expected = canonical_levels(&expected, true)?;
                let actual = canonical_levels(&actual, false)?;
                matches &= expected == actual;
                row["reference_levels"] = serde_json::to_value(expected)?;
                row["rust_levels"] = serde_json::to_value(actual)?;
            }
            row["status"] = json!(if matches { "matched" } else { "mismatch" });
        }
        report["cases"].as_array_mut().unwrap().push(row);
        report["elapsed_secs"] = json!(started.elapsed().as_secs_f64());
        fs::write(
            args.output.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
    }
    report["elapsed_secs"] = json!(started.elapsed().as_secs_f64());
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
