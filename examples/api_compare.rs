//! Independent API compatibility: all extracted ordering/projection invocations.
use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
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
    Limits, Problem,
    algebra::{Algebra, canonical},
    benchmark::{ProcessOutcome, reference_budget, run_bounded},
    corpus::import_core_entries,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Suite {
    All,
    Fast,
    Slow,
}

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
    /// Original API identifiers, matched exactly; repeat to select several.
    #[arg(long = "case")]
    cases: Vec<String>,
    #[arg(long, value_enum, default_value = "all")]
    suite: Suite,
    #[arg(long, default_value = "tests/api_classification.json")]
    classification: PathBuf,
}

fn process_succeeded(process: &ProcessOutcome) -> bool {
    process.status == "completed" && process.exit_code == Some(0) && !process.output_truncated
}

fn completed_api_wall(
    process: &ProcessOutcome,
    result: Option<&Value>,
    problem: &Problem,
    operation: &str,
) -> Option<f64> {
    if !process_succeeded(process)
        || !process.wall_time_secs.is_finite()
        || process.wall_time_secs <= 0.
    {
        return None;
    }
    let result = result?;
    if result["schema_version"] != 1 || result.get("status").is_some_and(|s| s != "complete") {
        return None;
    }
    let order: Vec<String> = serde_json::from_value(result["order"].clone()).ok()?;
    if order.len() != problem.variables.len() {
        return None;
    }
    let mut ordered = problem.clone();
    ordered.solver.order = order.clone();
    ordered.validate().ok()?;
    if (!problem.solver.order.is_empty() || problem.solver.ordering == "given")
        && order != problem.effective_order()
    {
        return None;
    }
    if operation == "project" {
        if result["levels"].as_array()?.len() != order.len()
            || !result["polynomials"].is_array()
            || canonical_levels(result, false).is_err()
        {
            return None;
        }
    }
    Some(process.wall_time_secs)
}

fn selected_names(
    entries: &[symgcad::corpus::CoreEntry],
    requested: &[String],
    suite: Suite,
    classification: &Value,
) -> Result<BTreeSet<String>> {
    ensure!(
        classification["schema_version"] == 1,
        "unsupported API classification"
    );
    let available: BTreeSet<_> = entries
        .iter()
        .map(|e| e.problem.name.clone().context("missing API name"))
        .collect::<Result<_>>()?;
    ensure!(
        available.len() == entries.len(),
        "duplicate extracted API identifiers"
    );
    let mut classified = BTreeSet::new();
    let mut selected = BTreeSet::new();
    for row in classification["cases"]
        .as_array()
        .context("missing API classification cases")?
    {
        let name = row["name"]
            .as_str()
            .context("missing classified API name")?;
        ensure!(
            classified.insert(name.to_string()),
            "duplicate classified API identifier"
        );
        let tag = row["suite"].as_str().context("missing API suite")?;
        ensure!(tag == "fast" || tag == "slow", "unknown API suite");
        if suite == Suite::All
            || (suite == Suite::Fast && tag == "fast")
            || (suite == Suite::Slow && tag == "slow")
        {
            selected.insert(name.to_string());
        }
    }
    ensure!(
        classified == available,
        "API classification differs from extracted inventory"
    );
    let requests: BTreeSet<_> = requested.iter().cloned().collect();
    ensure!(
        requests.len() == requested.len(),
        "duplicate requested API identifier"
    );
    ensure!(
        requests.is_subset(&selected),
        "requested API identifier is absent from the selected suite"
    );
    if !requests.is_empty() {
        selected = requests;
    }
    ensure!(!selected.is_empty(), "empty API selection");
    Ok(selected)
}

fn save_report(directory: &Path, report: &Value) -> Result<()> {
    let temporary = directory.join("report.json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(report)?)?;
    fs::rename(temporary, directory.join("report.json"))?;
    Ok(())
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
    ensure!(
        fs::read_dir(&args.output)?.next().is_none(),
        "API output directory must be empty; preserve previous evidence"
    );
    let entries: Vec<_> = import_core_entries(&args.core)?
        .into_iter()
        .filter(|entry| entry.operation != "solve")
        .collect();
    let classification: Value = serde_json::from_reader(fs::File::open(&args.classification)?)?;
    let selected = selected_names(&entries, &args.cases, args.suite, &classification)?;
    let reference_metadata: Value = serde_json::from_reader(fs::File::open(
        args.reference_root
            .parent()
            .unwrap()
            .join("provenance.json"),
    )?)?;
    let python = reference_metadata["python_executable"]
        .as_str()
        .context("reference interpreter missing")?;
    let source_solver = args.solver.canonicalize()?;
    let solver = args.output.canonicalize()?.join("symgcad-frozen");
    fs::copy(&source_solver, &solver)?;
    ensure!(
        hash_file(&solver)? == hash_file(&source_solver)?,
        "solver changed while snapshotting"
    );
    let adapter = args.adapter.canonicalize()?;
    let binary_hash = hash_file(&solver)?;
    let mut report = json!({"schema_version":1,"operation":"API compatibility, not a solve benchmark","provenance":{
        "solver":solver,"source_solver":source_solver,"solver_sha256":binary_hash,"harness_sha256":hash_file(Path::new("/proc/self/exe"))?,"adapter_sha256":hash_file(&adapter)?,
        "shared_adapter_sha256":hash_file(&adapter.parent().unwrap().join("adapter.py"))?,
        "core_inputs_sha256":hash_file(&args.core)?,"reference_revision":reference_metadata["revision"],
        "workers":1,"memory_mib":args.memory_mib,"timeout_secs":args.timeout,
        "campaign_time_secs":args.campaign_time_secs,
        "classification_sha256":hash_file(&args.classification)?,
        "reference_budget_policy":"Rust first; native at most min(configured timeout, remaining campaign, 3600s, max(300s, 3*completed Rust API process wall)). A timeout supplies no Rust timing. Single compatibility attempts, not benchmark medians.",
        "selected_cases":selected
    },"cases":[],"elapsed_secs":0.0});
    let started = Instant::now();
    for (index, entry) in entries.into_iter().enumerate() {
        if !selected.contains(entry.problem.name.as_ref().context("missing API name")?) {
            continue;
        }
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
        let result_file = args.output.join(format!("{index:04}-rust-result.json"));
        let mut rust = Command::new(&solver);
        rust.arg("_worker")
            .arg(&entry.operation)
            .arg(&input_file)
            .arg("--output")
            .arg(&result_file);
        row["rust_budget_secs"] = json!(limits.wall_time_secs);
        row["status"] = json!("rust_running");
        report["active_case"] = row.clone();
        save_report(&args.output, &report)?;
        let rust = run_bounded(&mut rust, &limits)?;
        let actual: Option<Value> = if process_succeeded(&rust) {
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
        row["rust_wall_time_secs"] = json!(rust.wall_time_secs);
        row["rust_peak_rss_kib"] = json!(rust.peak_rss_kib);
        let completed_wall =
            completed_api_wall(&rust, actual.as_ref(), &entry.problem, &entry.operation);
        row["rust_api_complete"] = json!(completed_wall.is_some());
        if remaining() < 0.05 {
            row["status"] = json!(if completed_wall.is_some() {
                "rust_complete_reference_pending"
            } else {
                "rust_incomplete_reference_pending"
            });
            row["reference_status"] = json!("campaign_exhausted");
            report["cases"].as_array_mut().unwrap().push(row);
            report["active_case"] = Value::Null;
            save_report(&args.output, &report)?;
            continue;
        }
        let budget = reference_budget(
            completed_wall,
            args.timeout,
            remaining(),
            "api_compatibility",
            None,
        )?;
        limits.wall_time_secs = budget.effective_timeout_secs;
        row["reference_budget"] = serde_json::to_value(&budget)?;
        row["status"] = json!("reference_running");
        report["active_case"] = row.clone();
        save_report(&args.output, &report)?;
        let mut reference = Command::new(python);
        reference
            .env("OPENBLAS_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1")
            .arg(&adapter)
            .arg(&entry.operation)
            .arg("--reference-root")
            .arg(&args.reference_root)
            .arg("--problem-file")
            .arg(&input_file);
        let reference = run_bounded(&mut reference, &limits)?;
        let expected: Option<Value> = if process_succeeded(&reference) {
            serde_json::from_str(&reference.stdout).ok()
        } else {
            None
        };
        fs::write(
            args.output.join(format!("{index:04}-reference.json")),
            serde_json::to_vec_pretty(
                &json!({"process":reference,"result":expected,"reference_budget":budget}),
            )?,
        )?;
        row["reference_status"] = json!(reference.status);
        row["reference_wall_time_secs"] = json!(reference.wall_time_secs);
        row["reference_peak_rss_kib"] = json!(reference.peak_rss_kib);
        row["status"] = json!(if expected.is_none() {
            format!("reference_{}", reference.status)
        } else if completed_wall.is_none() {
            format!("rust_{}", rust.status)
        } else {
            "compared".into()
        });
        if let (Some(expected), Some(actual), Some(_)) = (expected, actual, completed_wall) {
            ensure!(
                expected["schema_version"] == 1 && expected["status"] == "complete",
                "invalid completed reference API response"
            );
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
        report["active_case"] = Value::Null;
        report["elapsed_secs"] = json!(started.elapsed().as_secs_f64());
        save_report(&args.output, &report)?;
    }
    report["elapsed_secs"] = json!(started.elapsed().as_secs_f64());
    report["active_case"] = Value::Null;
    save_report(&args.output, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    ensure!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["status"] == "matched"),
        "some API checks remain incomplete or mismatched; see report.json"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process() -> ProcessOutcome {
        ProcessOutcome {
            status: "completed".into(),
            wall_time_secs: 150.,
            user_cpu_secs: 149.,
            system_cpu_secs: 1.,
            peak_rss_kib: 100,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            output_truncated: false,
        }
    }

    #[test]
    fn only_valid_complete_api_responses_raise_the_reference_cap() -> Result<()> {
        let problem = Problem {
            variables: vec!["s".into(), "x".into()],
            variable_groups: vec![vec!["s".into()], vec!["x".into()]],
            ..Problem::default()
        };
        let result = json!({"schema_version":1,"order":["s","x"]});
        let p = process();
        let wall = completed_api_wall(&p, Some(&result), &problem, "order");
        assert_eq!(wall, Some(150.));
        assert_eq!(
            reference_budget(wall, 3600., 3600., "test", None)?.effective_timeout_secs,
            450.
        );
        for (status, code, truncated) in [
            ("timeout", None, false),
            ("completed", Some(1), false),
            ("completed", Some(0), true),
        ] {
            let mut failed = p.clone();
            failed.status = status.into();
            failed.exit_code = code;
            failed.output_truncated = truncated;
            assert_eq!(
                completed_api_wall(&failed, Some(&result), &problem, "order"),
                None
            );
        }
        for invalid in [
            json!({"schema_version":1,"order":["x","s"]}),
            json!({"schema_version":1,"order":["s","s"]}),
            json!({"schema_version":1,"order":["s","x"],"status":"incomplete"}),
        ] {
            assert_eq!(
                completed_api_wall(&p, Some(&invalid), &problem, "order"),
                None
            );
        }
        assert_eq!(
            completed_api_wall(&p, Some(&result), &problem, "project"),
            None
        );
        let mut heuristic = problem.clone();
        heuristic.solver.ordering = "mods".into();
        let empty = json!({"schema_version":1,"order":[]});
        assert_eq!(
            completed_api_wall(&p, Some(&empty), &heuristic, "order"),
            None
        );
        let invalid_projection = json!({"schema_version":1,"order":["s","x"],"levels":[]});
        assert_eq!(
            completed_api_wall(&p, Some(&invalid_projection), &heuristic, "project"),
            None
        );
        assert_eq!(
            reference_budget(None, 3600., 3600., "test", None)?.effective_timeout_secs,
            300.
        );
        assert_eq!(
            reference_budget(wall, 3600., 17., "test", None)?.effective_timeout_secs,
            17.
        );
        Ok(())
    }

    #[test]
    fn api_classification_keeps_six_original_timeouts_separate() -> Result<()> {
        let classification: Value =
            serde_json::from_str(include_str!("../tests/api_classification.json"))?;
        let entries: Vec<_> = classification["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| symgcad::corpus::CoreEntry {
                problem: Problem {
                    name: Some(row["name"].as_str().unwrap().into()),
                    ..Problem::default()
                },
                source: row["source"].as_str().unwrap().into(),
                operation: row["operation"].as_str().unwrap().into(),
            })
            .collect();
        assert_eq!(
            selected_names(&entries, &[], Suite::All, &classification)?.len(),
            23
        );
        assert_eq!(
            selected_names(&entries, &[], Suite::Fast, &classification)?.len(),
            17
        );
        let slow = selected_names(&entries, &[], Suite::Slow, &classification)?;
        assert_eq!(slow.len(), 6);
        let name = slow.first().unwrap().clone();
        assert_eq!(
            selected_names(&entries, &[name.clone()], Suite::Slow, &classification)?.len(),
            1
        );
        assert!(
            selected_names(
                &entries,
                &[name.clone(), name.clone()],
                Suite::Slow,
                &classification
            )
            .is_err()
        );
        assert!(selected_names(&entries, &[name], Suite::Fast, &classification).is_err());
        assert!(
            selected_names(&entries, &["unknown".into()], Suite::All, &classification).is_err()
        );
        assert!(selected_names(&entries[..22], &[], Suite::All, &classification).is_err());
        Ok(())
    }
}
