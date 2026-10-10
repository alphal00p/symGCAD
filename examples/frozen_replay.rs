//! Re-measure an immutable reference qualification set without requalifying it.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symgcad::{
    Limits, Problem,
    benchmark::{
        completed_solve_wall as completed_wall, reference_budget, run_bounded,
        run_reference_measured,
    },
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    frozen: PathBuf,
    #[arg(long)]
    solver: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = ".reference/gcad")]
    reference_root: PathBuf,
    #[arg(long, default_value = "reference/adapter.py")]
    adapter: PathBuf,
    #[arg(long, default_value_t = 3)]
    repeats: usize,
    #[arg(long, default_value_t = 60.0)]
    timeout: f64,
    #[arg(long)]
    verification_timeout: Option<f64>,
    #[arg(long, default_value_t = 3600.0)]
    campaign_time_secs: f64,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
}
fn hash(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn median(mut xs: Vec<f64>) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(f64::total_cmp);
    let n = xs.len();
    Some(if n % 2 == 0 {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    } else {
        xs[n / 2]
    })
}
fn clear_measurements(row: &mut Value) {
    for key in [
        "verified",
        "end_to_end_ratio",
        "solver_ratio",
        "raw_cells_match",
        "merged_cells_match",
        "reference_median_secs",
        "rust_median_secs",
        "reference_solver_median_secs",
        "rust_solver_median_secs",
        "reference_preparation_median_secs",
        "rust_preparation_median_secs",
    ] {
        row[key] = Value::Null;
    }
}
fn retain_verified_ratios(row: &mut Value) {
    if row["verified"] != true
        || row["status"] != "complete"
        || row["raw_cells_match"] != true
        || row["merged_cells_match"] == false
    {
        row["end_to_end_ratio"] = Value::Null;
        row["solver_ratio"] = Value::Null;
    }
}
fn geometric(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() || xs.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        None
    } else {
        Some((xs.iter().map(|x| x.ln()).sum::<f64>() / xs.len() as f64).exp())
    }
}
fn algorithm(result: &Value, reference: bool) -> Option<f64> {
    let (stats, keys) = if reference {
        (&result["phases"], ["projection", "lifting", "merging"])
    } else {
        (
            &result["statistics"],
            ["projection_seconds", "lifting_seconds", "merging_seconds"],
        )
    };
    Some(stats[keys[0]].as_f64()? + stats[keys[1]].as_f64()? + stats[keys[2]].as_f64()?)
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.repeats > 0
            && args.timeout.is_finite()
            && args.timeout > 0.0
            && args.timeout <= 3600.0,
        "invalid repetition/deadline"
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
    let verification_timeout = args.verification_timeout.unwrap_or(args.timeout);
    ensure!(
        verification_timeout.is_finite()
            && verification_timeout > 0.0
            && verification_timeout <= 3600.0,
        "invalid verification deadline"
    );
    fs::create_dir_all(&args.output)?;
    ensure!(
        fs::read_dir(&args.output)?.next().is_none(),
        "replay output directory must be empty to preserve previous evidence"
    );
    let frozen: Value = serde_json::from_reader(fs::File::open(&args.frozen)?)?;
    let mut rows = frozen["cases"]
        .as_array()
        .context("frozen cases missing")?
        .clone();
    let problems = frozen["problems"]
        .as_array()
        .context("frozen problems missing")?;
    ensure!(
        rows.len() == problems.len(),
        "frozen case/problem count mismatch"
    );
    let solver = args.solver.canonicalize()?;
    let solver_hash = hash(&solver)?;
    let metadata: Value = serde_json::from_reader(fs::File::open(
        args.reference_root
            .parent()
            .unwrap()
            .join("provenance.json"),
    )?)?;
    let python: PathBuf = metadata["python_executable"]
        .as_str()
        .context("reference interpreter missing")?
        .into();
    let provenance = json!({"executable":solver,"executable_sha256":solver_hash,"harness_sha256":hash(Path::new("/proc/self/exe"))?,"frozen_reference_set":args.frozen,"frozen_reference_set_sha256":hash(&args.frozen)?,"adapter_sha256":hash(&args.adapter)?,"reference_revision":metadata["revision"],"reference_compiler":metadata["compiler"],"workers":1,"memory_mib":args.memory_mib,"solve_timeout_secs":args.timeout,"verification_timeout_secs":verification_timeout,"timing_policy":"frozen qualification/orders; matched projection + lifting + merging; startup/export in fresh-process wall; independent verification outside solve timing"});
    let mut provenance = provenance;
    provenance["reference_budget_policy"] = json!(
        "Qualification set and original wire schema unchanged; Rust repeats first; native cap=min(explicit timeout, remaining campaign, max(300s,3*median completed Rust process wall)); incomplete or unverified repetitions have no exact ratio"
    );
    write(args.output.join("frozen-reference-set.json"), &frozen)?;
    let started = Instant::now();
    let mut exhausted = false;
    for (index, original) in problems.iter().enumerate() {
        let remaining = || (args.campaign_time_secs - started.elapsed().as_secs_f64()).max(0.0);
        let row = &mut rows[index];
        ensure!(
            row["name"] == original["name"],
            "frozen problem identity mismatch"
        );
        if row["reference_qualified"] != true {
            continue;
        }
        clear_measurements(row);
        row["reference_budgets"] = json!([]);
        row["reference_censored"] = json!(false);
        ensure!(
            row["frozen_order"] == original["solver"]["order"]
                && original["solver"]["ordering"] == "given",
            "frozen order is not pinned"
        );
        if remaining() < 0.05 {
            exhausted = true;
            row["status"] = json!("campaign_exhausted");
            continue;
        }
        let mut reference_wall = Vec::new();
        let mut rust_wall = Vec::new();
        let mut reference_kernel = Vec::new();
        let mut rust_kernel = Vec::new();
        let mut ref_prep = Vec::new();
        let mut rust_prep = Vec::new();
        let mut ref_cells = Vec::new();
        let mut rust_cells = Vec::new();
        let mut ref_merged = Vec::new();
        let mut rust_merged = Vec::new();
        let mut last_result = None;
        let mut reference_rss = row["reference_peak_rss_kib"].as_u64().unwrap_or(0);
        let mut rust_rss = 0;
        for repeat in 0..args.repeats {
            if remaining() < 0.05 {
                exhausted = true;
                break;
            }
            // Preserve original worker schema, including omitted post-M1 fields.
            let mut raw = original.clone();
            raw["limits"]["wall_time_secs"] = json!(args.timeout.min(remaining()));
            raw["limits"]["memory_mib"] = json!(args.memory_mib);
            raw["limits"]["workers"] = json!(1);
            let input: Problem = serde_json::from_value(raw.clone())?;
            input.validate()?;
            let input_file = args.output.join(format!("{index:04}-input.json"));
            write(&input_file, &raw)?;
            let result_file = args
                .output
                .join(format!("{index:04}-rust-{repeat}-result.json"));
            let limits: Limits = serde_json::from_value(raw["limits"].clone())?;
            let mut command = Command::new(&solver);
            command
                .arg("_worker")
                .arg("solve")
                .arg(&input_file)
                .arg("--output")
                .arg(&result_file);
            let process = run_bounded(&mut command, &limits)?;
            rust_rss = rust_rss.max(process.peak_rss_kib);
            let result: Option<Value> =
                if process.status == "completed" && !process.output_truncated {
                    serde_json::from_str(&process.stdout).ok()
                } else {
                    None
                };
            write(
                args.output.join(format!("{index:04}-rust-{repeat}.json")),
                &json!({"schema_version":1,"engine":"symgcad","problem":raw["name"],"process":process,"result":result}),
            )?;
            if let Some(wall) = completed_wall(&process, result.as_ref()) {
                let result = result.as_ref().unwrap();
                rust_wall.push(wall);
                if let Some(t) = algorithm(&result, false) {
                    rust_kernel.push(t);
                }
                if let Some(total) = result["statistics"]["total_seconds"].as_f64() {
                    rust_prep.push(
                        (total
                            - algorithm(&result, false).unwrap_or(total)
                            - result["statistics"]["ordering_seconds"]
                                .as_f64()
                                .unwrap_or(0.0))
                        .max(0.0),
                    );
                }
                rust_cells.push(
                    result["cells"]
                        .as_u64()
                        .context("Rust cell count missing")?,
                );
                rust_merged.push(result["merged_cells"].as_u64());
                last_result = Some(result_file);
            }
        }
        // Qualification remains immutable; these new native repetitions are
        // budgeted only after observing completed Rust solve processes.
        let completed_rust_median = median(rust_wall.clone());
        for repeat in 0..args.repeats {
            if remaining() < 0.05 {
                exhausted = true;
                break;
            }
            let budget = reference_budget(
                completed_rust_median,
                args.timeout,
                remaining(),
                "frozen_replay",
                Some(repeat),
            )?;
            let mut raw = original.clone();
            raw["limits"]["wall_time_secs"] = json!(budget.effective_timeout_secs);
            raw["limits"]["memory_mib"] = json!(args.memory_mib);
            raw["limits"]["workers"] = json!(1);
            let input: Problem = serde_json::from_value(raw)?;
            input.validate()?;
            let mut record = run_reference_measured(
                &input,
                &python,
                &args.reference_root,
                &args.adapter,
                completed_rust_median,
            )?;
            record.reference_budget = Some(budget.clone());
            row["reference_budgets"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::to_value(budget)?);
            if record.process.status == "timeout" {
                row["reference_censored"] = json!(true);
            }
            reference_rss = reference_rss.max(record.process.peak_rss_kib);
            write(
                args.output
                    .join(format!("{index:04}-reference-{repeat}.json")),
                &record,
            )?;
            if let Some(wall) = completed_wall(&record.process, record.result.as_ref()) {
                let result = record.result.as_ref().unwrap();
                reference_wall.push(wall);
                if let Some(t) = algorithm(result, true) {
                    reference_kernel.push(t);
                }
                if let Some(t) = result["preparation_seconds"].as_f64() {
                    ref_prep.push(t);
                }
                let result_rows = result["results"]
                    .as_array()
                    .context("reference result rows missing")?;
                ref_cells.push(
                    result_rows
                        .iter()
                        .map(|r| r["raw_cells"].as_u64().unwrap_or(0))
                        .sum::<u64>(),
                );
                ref_merged.push(
                    result_rows
                        .iter()
                        .map(|r| r["merged_cells"].as_u64().unwrap_or(0))
                        .sum::<u64>(),
                );
            }
        }
        let complete = reference_wall.len() == args.repeats
            && rust_wall.len() == args.repeats
            && reference_kernel.len() == args.repeats
            && rust_kernel.len() == args.repeats;
        row["status"] = json!(if complete {
            "complete"
        } else {
            "incomplete_repetitions"
        });
        row["reference_median_secs"] = json!(median(reference_wall));
        row["rust_median_secs"] = json!(median(rust_wall));
        row["reference_solver_median_secs"] = json!(median(reference_kernel));
        row["rust_solver_median_secs"] = json!(median(rust_kernel));
        row["reference_preparation_median_secs"] = json!(median(ref_prep));
        row["rust_preparation_median_secs"] = json!(median(rust_prep));
        row["reference_peak_rss_kib"] = json!(reference_rss);
        row["rust_peak_rss_kib"] = json!(rust_rss);
        if complete {
            row["raw_cells_match"] = json!(ref_cells == rust_cells);
            row["merged_cells_match"] = if original["solver"]["merge"] == "aggressive" {
                json!(ref_merged.iter().map(|v| Some(*v)).collect::<Vec<_>>() == rust_merged)
            } else {
                Value::Null
            };
            if row["raw_cells_match"] != true || row["merged_cells_match"] == false {
                row["status"] = json!("cell_count_difference");
            }
            row["end_to_end_ratio"] = json!(
                row["rust_median_secs"].as_f64().unwrap()
                    / row["reference_median_secs"].as_f64().unwrap()
            );
            row["solver_ratio"] = json!(
                row["rust_solver_median_secs"].as_f64().unwrap()
                    / row["reference_solver_median_secs"].as_f64().unwrap()
            );
            if remaining() >= 0.05 {
                let verification_file = args.output.join(format!("{index:04}-verification.json"));
                let mut command = Command::new(&solver);
                command
                    .arg("_worker")
                    .arg("verify")
                    .arg(last_result.unwrap())
                    .arg("--output")
                    .arg(&verification_file);
                let limits = Limits {
                    wall_time_secs: verification_timeout.min(remaining()),
                    memory_mib: args.memory_mib,
                    workers: 1,
                    max_cells: None,
                };
                let process = run_bounded(&mut command, &limits)?;
                let verified = process.status == "completed"
                    && process.exit_code == Some(0)
                    && !process.output_truncated
                    && fs::read(&verification_file)
                        .ok()
                        .and_then(|s| serde_json::from_slice::<Value>(&s).ok())
                        .is_some_and(|v| v["verified"] == true);
                write(
                    args.output
                        .join(format!("{index:04}-verification-process.json")),
                    &process,
                )?;
                row["verified"] = json!(verified);
                if !verified {
                    row["status"] = json!("verification_failed");
                }
            } else {
                exhausted = true;
                row["status"] = json!("verification_budget_exhausted");
            }
        }
        retain_verified_ratios(row);
        write(args.output.join("progress.json"), &rows)?;
    }
    let qualified = rows
        .iter()
        .filter(|r| r["reference_qualified"] == true)
        .collect::<Vec<_>>();
    let complete = qualified.iter().all(|r| {
        r["status"] == "complete"
            && r["raw_cells_match"] == true
            && r["verified"] == true
            && r["merged_cells_match"] != false
    });
    let ratios = qualified
        .iter()
        .filter_map(|r| r["solver_ratio"].as_f64())
        .collect::<Vec<_>>();
    let walls = qualified
        .iter()
        .filter_map(|r| r["end_to_end_ratio"].as_f64())
        .collect::<Vec<_>>();
    let geo = (ratios.len() == qualified.len())
        .then(|| geometric(&ratios))
        .flatten();
    let wall_geo = (walls.len() == qualified.len())
        .then(|| geometric(&walls))
        .flatten();
    let report = json!({"schema_version":1,"cases":rows,"repeats":args.repeats,"frozen_reference_count":qualified.len(),"geometric_mean_solver_ratio":geo,"geometric_mean_end_to_end_ratio":wall_geo,"parity_target":1.25,"parity_passed":complete&&!exhausted&&geo.is_some_and(|x|x<=1.25),"campaign_exhausted":exhausted,"elapsed_secs":started.elapsed().as_secs_f64(),"provenance":provenance});
    write(args.output.join("report.json"), &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::benchmark::ProcessOutcome;
    fn process(status: &str, wall: f64) -> ProcessOutcome {
        ProcessOutcome {
            status: status.into(),
            wall_time_secs: wall,
            user_cpu_secs: 0.,
            system_cpu_secs: 0.,
            peak_rss_kib: 0,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            output_truncated: false,
        }
    }
    #[test]
    fn only_complete_processes_supply_adaptive_reference_median() -> Result<()> {
        let complete = json!({"status":"complete_generic"});
        let incomplete = json!({"status":"incomplete"});
        let mut failed = process("completed", 900.);
        failed.exit_code = Some(1);
        let mut truncated = process("completed", 800.);
        truncated.output_truncated = true;
        let samples = [
            (process("completed", 120.), &complete),
            (process("timeout", 999.), &complete),
            (failed, &complete),
            (truncated, &complete),
            (process("completed", 700.), &incomplete),
            (process("completed", 160.), &complete),
            (process("completed", 140.), &complete),
        ]
        .into_iter()
        .filter_map(|(p, r)| completed_wall(&p, Some(r)))
        .collect::<Vec<_>>();
        assert_eq!(samples, vec![120., 160., 140.]);
        let budget = reference_budget(median(samples), 1000., 800., "frozen_replay", Some(0))?;
        assert_eq!(budget.effective_timeout_secs, 420.);
        assert_eq!(
            reference_budget(None, 999., 800., "frozen_replay", Some(0))?.effective_timeout_secs,
            300.
        );
        Ok(())
    }
    #[test]
    fn censored_or_unverified_replay_never_retains_historical_ratios() {
        let mut row = json!({"reference_qualified":true,"frozen_order":["x"],"status":"complete",
            "verified":true,"raw_cells_match":true,"end_to_end_ratio":0.5,"solver_ratio":0.4});
        clear_measurements(&mut row);
        assert_eq!(row["reference_qualified"], true);
        assert_eq!(row["frozen_order"], json!(["x"]));
        assert!(row["solver_ratio"].is_null());
        assert!(row["verified"].is_null());
        for (status, verified) in [
            ("incomplete_repetitions", true),
            ("verification_failed", false),
            ("complete", false),
        ] {
            row["status"] = json!(status);
            row["verified"] = json!(verified);
            row["raw_cells_match"] = json!(true);
            row["solver_ratio"] = json!(0.4);
            row["end_to_end_ratio"] = json!(0.5);
            retain_verified_ratios(&mut row);
            assert!(row["solver_ratio"].is_null());
            assert!(row["end_to_end_ratio"].is_null());
        }
    }
    #[test]
    fn explicit_and_remaining_ceilings_bind_new_reference_repeats() -> Result<()> {
        assert_eq!(
            reference_budget(Some(150.), 60., 900., "frozen_replay", Some(1))?
                .effective_timeout_secs,
            60.
        );
        assert_eq!(
            reference_budget(Some(150.), 900., 25., "frozen_replay", Some(2))?
                .effective_timeout_secs,
            25.
        );
        let mut row = json!({"status":"complete","verified":true,"raw_cells_match":true,"merged_cells_match":null,"solver_ratio":0.4,"end_to_end_ratio":0.5});
        retain_verified_ratios(&mut row);
        assert_eq!(row["solver_ratio"], 0.4);
        row["merged_cells_match"] = json!(false);
        retain_verified_ratios(&mut row);
        assert!(row["solver_ratio"].is_null());
        Ok(())
    }
}
