//! Independent reference invocation and killable worker supervision.
use crate::{Limits, Problem};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CampaignConfig {
    pub cases: Vec<std::path::PathBuf>,
    pub output_dir: std::path::PathBuf,
    pub reference_python: Option<std::path::PathBuf>,
    pub reference_root: std::path::PathBuf,
    pub adapter: std::path::PathBuf,
    pub repeats: usize,
    pub reference_timeout_secs: f64,
    pub solve_timeout_secs: f64,
    pub campaign_time_secs: f64,
    pub memory_mib: u64,
    pub workers: usize,
}

impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            cases: Vec::new(),
            output_dir: "artifacts/bench".into(),
            reference_python: None,
            reference_root: ".reference/gcad".into(),
            adapter: "reference/adapter.py".into(),
            repeats: 3,
            reference_timeout_secs: 3600.0,
            solve_timeout_secs: 300.0,
            campaign_time_secs: 3600.0,
            memory_mib: 65536,
            workers: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseSummary {
    pub name: String,
    pub reference_qualified: bool,
    pub frozen_order: Vec<String>,
    pub status: String,
    pub reference_median_secs: Option<f64>,
    pub rust_median_secs: Option<f64>,
    pub end_to_end_ratio: Option<f64>,
    pub reference_solver_median_secs: Option<f64>,
    pub rust_solver_median_secs: Option<f64>,
    pub reference_preparation_median_secs: Option<f64>,
    pub rust_preparation_median_secs: Option<f64>,
    pub solver_ratio: Option<f64>,
    pub raw_cells_match: Option<bool>,
    pub merged_cells_match: Option<bool>,
    pub verified: Option<bool>,
    pub reference_peak_rss_kib: u64,
    pub rust_peak_rss_kib: u64,
    #[serde(default)]
    pub reference_budgets: Vec<ReferenceBudget>,
    #[serde(default)]
    pub reference_censored: bool,
}

/// Fresh-process wall time, never an algorithm phase time or a timeout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceBudget {
    pub stage: String,
    pub repetition: Option<usize>,
    pub completed_rust_median_wall_secs: Option<f64>,
    pub adaptive_ceiling_secs: f64,
    pub configured_ceiling_secs: f64,
    pub remaining_campaign_secs: f64,
    pub effective_timeout_secs: f64,
}

pub fn reference_budget(
    rust_median: Option<f64>,
    configured: f64,
    remaining: f64,
    stage: &str,
    repetition: Option<usize>,
) -> Result<ReferenceBudget> {
    ensure!(
        configured.is_finite() && configured > 0. && remaining.is_finite() && remaining > 0.,
        "reference ceilings must be finite and positive"
    );
    ensure!(
        rust_median.is_none_or(|t| t.is_finite() && t >= 0.),
        "invalid completed Rust measurement"
    );
    let adaptive = rust_median.map_or(300., |t| (3. * t).min(f64::MAX).max(300.));
    Ok(ReferenceBudget {
        stage: stage.into(),
        repetition,
        completed_rust_median_wall_secs: rust_median,
        adaptive_ceiling_secs: adaptive,
        configured_ceiling_secs: configured,
        remaining_campaign_secs: remaining,
        effective_timeout_secs: adaptive.min(configured).min(remaining).min(3600.),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignReport {
    pub schema_version: u32,
    pub cases: Vec<CaseSummary>,
    pub repeats: usize,
    pub frozen_reference_count: usize,
    pub geometric_mean_end_to_end_ratio: Option<f64>,
    pub geometric_mean_solver_ratio: Option<f64>,
    pub parity_target: f64,
    pub parity_passed: bool,
    pub campaign_exhausted: bool,
    pub elapsed_secs: f64,
    pub provenance: Value,
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn resolve(base: &Path, path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}

fn median(mut numbers: Vec<f64>) -> Option<f64> {
    if numbers.is_empty() {
        return None;
    }
    numbers.sort_by(f64::total_cmp);
    let n = numbers.len();
    Some(if n % 2 == 0 {
        (numbers[n / 2 - 1] + numbers[n / 2]) / 2.0
    } else {
        numbers[n / 2]
    })
}

fn geomean(numbers: &[f64]) -> Option<f64> {
    if numbers.is_empty() || numbers.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return None;
    }
    Some((numbers.iter().map(|x| x.ln()).sum::<f64>() / numbers.len() as f64).exp())
}

fn algorithm_seconds(result: &Value, reference: bool) -> Option<f64> {
    let (phases, keys) = if reference {
        (&result["phases"], ["projection", "lifting", "merging"])
    } else {
        (
            &result["statistics"],
            ["projection_seconds", "lifting_seconds", "merging_seconds"],
        )
    };
    let projection = phases[keys[0]].as_f64()?;
    let lifting = phases[keys[1]].as_f64()?;
    let merging = phases[keys[2]].as_f64()?;
    let total = projection + lifting + merging;
    (total.is_finite() && total >= 0.0).then_some(total)
}

pub fn completed_solve_wall(process: &ProcessOutcome, result: Option<&Value>) -> Option<f64> {
    (process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && process.wall_time_secs.is_finite()
        && process.wall_time_secs > 0.
        && result.is_some_and(|r| r["status"] == "complete_generic"))
    .then_some(process.wall_time_secs)
}

/// Compare the same explicit-order mathematical branches. Reference
/// qualification is frozen before any Rust timings are observed. A failed
/// Rust run stays in the denominator and prevents a passing parity claim.
/// Paths in the campaign TOML are relative to that file.
pub fn run_campaign(config_path: &Path, executable: &Path) -> Result<CampaignReport> {
    let config: CampaignConfig = toml::from_str(&fs::read_to_string(config_path)?)?;
    ensure!(
        !config.cases.is_empty(),
        "benchmark campaign contains no cases"
    );
    ensure!(config.repeats > 0, "benchmark repeats must be positive");
    ensure!(
        config.campaign_time_secs.is_finite()
            && config.campaign_time_secs > 0.0
            && config.campaign_time_secs <= 3600.0,
        "campaign budget must be in (0,3600]"
    );
    ensure!(
        config.reference_timeout_secs.is_finite() && config.reference_timeout_secs > 0.0,
        "invalid reference timeout"
    );
    ensure!(
        config.solve_timeout_secs.is_finite() && config.solve_timeout_secs > 0.0,
        "invalid solve timeout"
    );
    ensure!(
        config.memory_mib > 0
            && config.memory_mib <= 65536
            && config.workers > 0
            && config.workers <= 16,
        "invalid campaign resource allocation"
    );
    let base = config_path.canonicalize()?.parent().unwrap().to_owned();
    let output = resolve(&base, &config.output_dir);
    fs::create_dir_all(&output)?;
    ensure!(
        fs::read_dir(&output)?.next().is_none(),
        "preserve existing benchmark evidence; select a fresh output directory"
    );
    let reference_root = resolve(&base, &config.reference_root);
    let adapter = resolve(&base, &config.adapter);
    let python = if let Some(path) = &config.reference_python {
        if path.components().count() == 1 {
            path.clone()
        } else {
            resolve(&base, path)
        }
    } else {
        let provenance = reference_root.parent().unwrap().join("provenance.json");
        let metadata: Value =
            serde_json::from_str(&fs::read_to_string(&provenance).with_context(|| {
                format!(
                    "read {}; build the reference first or set reference_python",
                    provenance.display()
                )
            })?)?;
        metadata
            .get("python_executable")
            .and_then(Value::as_str)
            .context(
                "reference provenance lacks python_executable; rerun reference/build_reference.py",
            )?
            .into()
    };
    let reference_metadata =
        fs::read_to_string(reference_root.parent().unwrap().join("provenance.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let cpu_model = fs::read_to_string("/proc/cpuinfo").ok().and_then(|text| {
        text.lines()
            .find(|line| line.starts_with("model name"))
            .map(str::to_owned)
    });
    let provenance = serde_json::json!({
        "executable":executable,"executable_sha256":file_sha256(executable)?,
        "profile":if cfg!(debug_assertions){"debug"}else{"release"},
        "config_sha256":file_sha256(config_path)?,"adapter_sha256":file_sha256(&adapter)?,
        "reference_revision":reference_metadata.as_ref().and_then(|m|m.get("revision")),
        "reference_compiler":reference_metadata.as_ref().and_then(|m|m.get("compiler")),
        "reference_python":python,"cpu_model":cpu_model,
        "workers":config.workers,"memory_mib":config.memory_mib,
        "timing_policy":"fixed order; projection + lifting + merging; normalization and startup reported separately",
        "reference_budget_policy":"qualification at most 300s before Rust; paired native repetitions capped at max(300s,3*median completed Rust fresh-process solve wall), further limited by configured and remaining campaign ceilings; timeouts are censored"
    });
    let mut problems = Vec::new();
    for path in &config.cases {
        let path = resolve(&base, path);
        let input = Problem::from_toml(&fs::read_to_string(&path)?)?;
        ensure!(
            input.split.len() <= 12,
            "too many split branches for a benchmark"
        );
        // Do not compare a shared two-sign arrangement against two independent
        // upstream projections: expand into identical individual sign solves.
        for mask in 0..(1usize << input.split.len()) {
            let mut problem = input.clone();
            let mut signs = Vec::new();
            for (i, split) in input.split.iter().enumerate() {
                let positive = mask & (1 << i) != 0;
                problem
                    .constraints
                    .push(format!("({split}) {} 0", if positive { ">" } else { "<" }));
                signs.push(if positive { "+" } else { "-" });
            }
            problem.split.clear();
            let name = input.name.as_deref().unwrap_or("unnamed");
            problem.name = Some(if signs.is_empty() {
                name.to_owned()
            } else {
                format!("{name}/signs={}", signs.join(""))
            });
            problem.limits = Limits {
                wall_time_secs: config.reference_timeout_secs.min(3600.0),
                memory_mib: config.memory_mib,
                workers: config.workers,
                max_cells: None,
            };
            problems.push(problem);
        }
    }
    let started = Instant::now();
    let remaining = || (config.campaign_time_secs - started.elapsed().as_secs_f64()).max(0.0);
    let mut summaries = Vec::new();
    let mut exhausted = false;
    // Phase 1: determine and freeze the reference-completing set.
    for (i, problem) in problems.iter_mut().enumerate() {
        let mut summary = CaseSummary {
            name: problem.name.clone().unwrap(),
            reference_qualified: false,
            frozen_order: vec![],
            status: "not_started".into(),
            reference_median_secs: None,
            rust_median_secs: None,
            end_to_end_ratio: None,
            reference_solver_median_secs: None,
            rust_solver_median_secs: None,
            reference_preparation_median_secs: None,
            rust_preparation_median_secs: None,
            solver_ratio: None,
            raw_cells_match: None,
            merged_cells_match: None,
            verified: None,
            reference_peak_rss_kib: 0,
            rust_peak_rss_kib: 0,
            reference_budgets: vec![],
            reference_censored: false,
        };
        if remaining() < 0.05 {
            exhausted = true;
            summary.status = "campaign_exhausted".into();
            summaries.push(summary);
            continue;
        }
        let budget = reference_budget(
            None,
            config.reference_timeout_secs,
            remaining(),
            "qualification",
            None,
        )?;
        problem.limits.wall_time_secs = budget.effective_timeout_secs;
        summary.reference_budgets.push(budget.clone());
        let mut record = run_reference(problem, &python, &reference_root, &adapter)?;
        record.reference_budget = Some(budget);
        fs::write(
            output.join(format!("{i:04}-qualification.json")),
            serde_json::to_vec_pretty(&record)?,
        )?;
        summary.reference_peak_rss_kib = record.process.peak_rss_kib;
        if record
            .result
            .as_ref()
            .and_then(|r| r.get("status"))
            .and_then(Value::as_str)
            == Some("complete_generic")
        {
            let order: Vec<String> =
                serde_json::from_value(record.result.as_ref().unwrap()["order"].clone())?;
            problem.solver.order = order.clone();
            problem.solver.ordering = "given".into();
            summary.frozen_order = order;
            summary.reference_qualified = true;
            summary.status = "qualified".into();
        } else {
            summary.status = format!("reference_{}", record.process.status);
            summary.reference_censored = true;
        }
        summaries.push(summary);
    }
    fs::write(
        output.join("frozen-reference-set.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"schema_version":1,"cases":summaries,"problems":problems}),
        )?,
    )?;
    // Phase 2: only the already-frozen set contributes to parity. Qualification
    // remains capped at300s before Rust, so it may censor a case that could
    // otherwise finish under a larger measured adaptive reference budget.
    for (i, problem) in problems.iter_mut().enumerate() {
        if !summaries[i].reference_qualified {
            continue;
        }
        let mut reference_wall = Vec::new();
        let mut rust_wall = Vec::new();
        let mut reference_solver = Vec::new();
        let mut rust_solver = Vec::new();
        let mut reference_preparation = Vec::new();
        let mut rust_preparation = Vec::new();
        let mut reference_cells = Vec::new();
        let mut rust_cells = Vec::new();
        let mut reference_merged = Vec::new();
        let mut rust_merged = Vec::new();
        for repetition in 0..config.repeats {
            if remaining() < 0.05 {
                exhausted = true;
                break;
            }
            problem.limits.wall_time_secs = config.solve_timeout_secs.min(remaining());
            let input = output.join(format!("{i:04}-input.json"));
            let result_file = output.join(format!("{i:04}-rust-{repetition}-result.json"));
            fs::write(&input, serde_json::to_vec(problem)?)?;
            let mut command = Command::new(executable);
            command
                .arg("_worker")
                .arg("solve")
                .arg(&input)
                .arg("--output")
                .arg(&result_file);
            let process = run_bounded(&mut command, &problem.limits)?;
            summaries[i].rust_peak_rss_kib =
                summaries[i].rust_peak_rss_kib.max(process.peak_rss_kib);
            let result: Option<Value> =
                if process.status == "completed" && !process.output_truncated {
                    serde_json::from_str(&process.stdout).ok()
                } else {
                    None
                };
            if let Some(wall) = completed_solve_wall(&process, result.as_ref()) {
                let result = result.as_ref().unwrap();
                rust_wall.push(wall);
                if let Some(seconds) = algorithm_seconds(result, false) {
                    rust_solver.push(seconds);
                    if let Some(total) = result["statistics"]["total_seconds"].as_f64() {
                        let ordering = result["statistics"]["ordering_seconds"]
                            .as_f64()
                            .unwrap_or(0.0);
                        rust_preparation.push((total - seconds - ordering).max(0.0));
                    }
                }
                rust_cells.push(result["cells"].as_u64().unwrap_or(0));
                if let Some(merged) = result["merged_cells"].as_u64() {
                    rust_merged.push(merged);
                }
            }
            let record = BenchmarkRecord {
                schema_version: 1,
                engine: "symgcad".into(),
                problem: problem.name.clone(),
                process,
                result,
                limits: Some(problem.limits.clone()),
                reference_budget: None,
            };
            fs::write(
                output.join(format!("{i:04}-rust-{repetition}.json")),
                serde_json::to_vec_pretty(&record)?,
            )?;
        }
        // Complete Rust fresh-process measurements determine one adaptive
        // ceiling per case. Failed/timeout runs never enter this median.
        let rust_median = median(rust_wall.clone());
        for repetition in 0..config.repeats {
            if remaining() < 0.05 {
                exhausted = true;
                break;
            }
            let budget = reference_budget(
                rust_median,
                config.reference_timeout_secs,
                remaining(),
                "measurement",
                Some(repetition),
            )?;
            problem.limits.wall_time_secs = budget.effective_timeout_secs;
            summaries[i].reference_budgets.push(budget.clone());
            let mut record =
                run_reference_measured(problem, &python, &reference_root, &adapter, rust_median)?;
            record.reference_budget = Some(budget);
            let before = reference_wall.len();
            summaries[i].reference_peak_rss_kib = summaries[i]
                .reference_peak_rss_kib
                .max(record.process.peak_rss_kib);
            fs::write(
                output.join(format!("{i:04}-reference-{repetition}.json")),
                serde_json::to_vec_pretty(&record)?,
            )?;
            if let Some(result) = record.result.filter(|r| r["status"] == "complete_generic") {
                reference_wall.push(record.process.wall_time_secs);
                if let Some(seconds) = algorithm_seconds(&result, true) {
                    reference_solver.push(seconds);
                }
                if let Some(seconds) = result["preparation_seconds"].as_f64() {
                    reference_preparation.push(seconds);
                }
                reference_cells.push(
                    result["results"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r["raw_cells"].as_u64().unwrap_or(0))
                        .sum::<u64>(),
                );
                reference_merged.push(
                    result["results"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r["merged_cells"].as_u64().unwrap_or(0))
                        .sum::<u64>(),
                );
            }
            if reference_wall.len() == before {
                summaries[i].reference_censored = true;
            }
        }
        let s = &mut summaries[i];
        s.reference_median_secs = median(reference_wall.clone());
        s.rust_median_secs = median(rust_wall.clone());
        s.reference_solver_median_secs = median(reference_solver.clone());
        s.rust_solver_median_secs = median(rust_solver.clone());
        s.reference_preparation_median_secs = median(reference_preparation);
        s.rust_preparation_median_secs = median(rust_preparation);
        if reference_wall.len() == config.repeats && rust_wall.len() == config.repeats {
            s.end_to_end_ratio =
                Some(s.rust_median_secs.unwrap() / s.reference_median_secs.unwrap());
            s.raw_cells_match = Some(reference_cells.iter().zip(&rust_cells).all(|(a, b)| a == b));
            if problem.solver.merge == "aggressive" {
                s.merged_cells_match = Some(
                    rust_merged.len() == config.repeats
                        && reference_merged
                            .iter()
                            .zip(&rust_merged)
                            .all(|(a, b)| a == b),
                );
            }
            s.status = if s.raw_cells_match == Some(true) && s.merged_cells_match != Some(false) {
                "complete"
            } else {
                "cell_count_difference"
            }
            .into();
            if reference_solver.len() == config.repeats && rust_solver.len() == config.repeats {
                s.solver_ratio = Some(
                    s.rust_solver_median_secs.unwrap() / s.reference_solver_median_secs.unwrap(),
                );
            }
        } else {
            s.status = if s.reference_censored {
                "reference_censored"
            } else if exhausted {
                "campaign_exhausted"
            } else {
                "incomplete_repetitions"
            }
            .into();
        }
        if s.status == "complete" {
            if remaining() < 0.05 {
                exhausted = true;
                s.status = "verification_not_started".into();
            } else {
                // Validation is outside the measured solve interval, but uses
                // the same bounded worker protocol and campaign budget.
                problem.limits.wall_time_secs = config.solve_timeout_secs.min(remaining());
                let result_file =
                    output.join(format!("{i:04}-rust-{}-result.json", config.repeats - 1));
                let verification_file = output.join(format!("{i:04}-verification.json"));
                let mut command = Command::new(executable);
                command
                    .arg("_worker")
                    .arg("verify")
                    .arg(&result_file)
                    .arg("--output")
                    .arg(&verification_file);
                let process = run_bounded(&mut command, &problem.limits)?;
                let result: Option<Value> =
                    if process.status == "completed" && !process.output_truncated {
                        serde_json::from_str(&process.stdout).ok()
                    } else {
                        None
                    };
                s.verified = Some(result.as_ref().is_some_and(|r| r["verified"] == true));
                if s.verified != Some(true) {
                    s.status = "verification_failed_or_incomplete".into();
                }
                let record = BenchmarkRecord {
                    schema_version: 1,
                    engine: "symgcad-verifier".into(),
                    problem: problem.name.clone(),
                    process,
                    result,
                    limits: Some(problem.limits.clone()),
                    reference_budget: None,
                };
                fs::write(
                    output.join(format!("{i:04}-verification-process.json")),
                    serde_json::to_vec_pretty(&record)?,
                )?;
            }
        }
        // A timeout is a censored measurement, never an exact ratio. Likewise
        // a failed proof replay must not leave a provisional ratio behind.
        if s.status != "complete" || s.verified != Some(true) {
            s.end_to_end_ratio = None;
            s.solver_ratio = None;
        }
        fs::write(
            output.join("progress.json"),
            serde_json::to_vec_pretty(&summaries)?,
        )?;
    }
    let frozen_reference_count = summaries.iter().filter(|s| s.reference_qualified).count();
    let complete = summaries
        .iter()
        .filter(|s| s.reference_qualified)
        .all(|s| s.status == "complete" && s.verified == Some(true));
    let wall = summaries
        .iter()
        .filter_map(|s| s.end_to_end_ratio)
        .collect::<Vec<_>>();
    let solver = summaries
        .iter()
        .filter_map(|s| s.solver_ratio)
        .collect::<Vec<_>>();
    let wall_geo = if wall.len() == frozen_reference_count {
        geomean(&wall)
    } else {
        None
    };
    let solver_geo = if solver.len() == frozen_reference_count {
        geomean(&solver)
    } else {
        None
    };
    let report = CampaignReport {
        schema_version: 1,
        cases: summaries,
        repeats: config.repeats,
        frozen_reference_count,
        geometric_mean_end_to_end_ratio: wall_geo,
        geometric_mean_solver_ratio: solver_geo,
        parity_target: 1.25,
        parity_passed: complete && !exhausted && solver_geo.is_some_and(|r| r <= 1.25),
        campaign_exhausted: exhausted,
        elapsed_secs: started.elapsed().as_secs_f64(),
        provenance,
    };
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutcome {
    pub status: String,
    pub wall_time_secs: f64,
    /// CPU consumed by the reaped worker (and children it reaped), from wait4.
    #[serde(default)]
    pub user_cpu_secs: f64,
    #[serde(default)]
    pub system_cpu_secs: f64,
    pub peak_rss_kib: u64,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub output_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRecord {
    pub schema_version: u32,
    pub engine: String,
    pub problem: Option<String>,
    pub process: ProcessOutcome,
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_budget: Option<ReferenceBudget>,
}

const OUTPUT_LIMIT: usize = 64 * 1024 * 1024;
fn read_bounded(mut reader: impl Read) -> std::io::Result<(String, bool)> {
    let mut saved = Vec::new();
    let mut buffer = [0; 8192];
    let mut truncated = false;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let keep = n.min(OUTPUT_LIMIT.saturating_sub(saved.len()));
        saved.extend_from_slice(&buffer[..keep]);
        truncated |= keep != n;
    }
    Ok((String::from_utf8_lossy(&saved).into_owned(), truncated))
}

/// Execute a worker under a wall-clock deadline and aggregate process-group
/// resident-memory cap. On Linux the process group also gets a CPU affinity
/// mask and a per-process virtual-memory backstop. stdout/stderr are drained
/// concurrently so a verbose child cannot deadlock its supervisor.
#[cfg(target_os = "linux")]
pub fn run_bounded(command: &mut Command, limits: &Limits) -> Result<ProcessOutcome> {
    run_bounded_observed(command, limits, |_| None)
}

/// As `run_bounded`, with an optional heuristic cancellation signal. The
/// observer runs in the supervisor and must return promptly. Cancellation is
/// reported as `pruned`; it is never evidence of mathematical emptiness.
#[cfg(target_os = "linux")]
pub fn run_bounded_observed(
    command: &mut Command,
    limits: &Limits,
    mut observer: impl FnMut(Duration) -> Option<String>,
) -> Result<ProcessOutcome> {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    ensure!(
        limits.wall_time_secs.is_finite() && limits.wall_time_secs > 0.0,
        "invalid worker deadline"
    );
    ensure!(
        limits.memory_mib > 0 && limits.workers > 0,
        "invalid worker resource limit"
    );
    let memory = limits
        .memory_mib
        .checked_mul(1024 * 1024)
        .context("memory limit overflow")?;
    let workers = limits.workers;
    let supervisor_pid = unsafe { libc::getpid() };
    // Compute affinity before fork; the pre-exec hook only performs syscalls.
    let mut allowed = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    let affinity_ok =
        unsafe { libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut allowed) }
            == 0;
    let mut selected = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    let mut selected_count = 0;
    if affinity_ok {
        for cpu in 0..libc::CPU_SETSIZE as usize {
            if unsafe { libc::CPU_ISSET(cpu, &allowed) } && selected_count < workers {
                unsafe {
                    libc::CPU_SET(cpu, &mut selected);
                }
                selected_count += 1;
            }
        }
    }
    unsafe {
        command.pre_exec(move || {
            // Nested CLI supervisors create their own worker groups. Kill each
            // direct child if its supervisor disappears, so cancellation also
            // propagates through those nested groups during search campaigns.
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != supervisor_pid {
                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
            }
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut inherited = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::getrlimit(libc::RLIMIT_AS, &mut inherited) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let effective_memory = memory.min(inherited.rlim_max);
            let cap = libc::rlimit {
                rlim_cur: effective_memory,
                rlim_max: effective_memory,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &cap) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if affinity_ok
                && selected_count > 0
                && libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &selected)
                    != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let started = Instant::now();
    let mut child = command.spawn().context("start bounded worker")?;
    let pid = child.id() as libc::pid_t;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out_thread = thread::spawn(move || read_bounded(stdout));
    let err_thread = thread::spawn(move || read_bounded(stderr));
    let mut peak = 0;
    let mut forced_status = None;
    let mut prune_reason = None;
    let worker_usage;
    let status = loop {
        let rss = process_group_rss(pid);
        peak = peak.max(rss);
        if forced_status.is_none() && started.elapsed().as_secs_f64() >= limits.wall_time_secs {
            forced_status = Some("timeout");
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        } else if forced_status.is_none() && rss > limits.memory_mib.saturating_mul(1024) {
            forced_status = Some("memory_limit");
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        } else if forced_status.is_none() {
            if let Some(reason) = observer(started.elapsed()) {
                prune_reason = Some(reason);
                forced_status = Some("pruned");
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
        }
        let mut raw_status = 0;
        let mut usage = unsafe { std::mem::zeroed::<libc::rusage>() };
        let waited = unsafe { libc::wait4(pid, &mut raw_status, libc::WNOHANG, &mut usage) };
        if waited == pid {
            peak = peak.max(usage.ru_maxrss.max(0) as u64);
            worker_usage = usage;
            break std::process::ExitStatus::from_raw(raw_status);
        }
        if waited < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
                bail!("wait for worker: {error}");
            }
        }
        thread::sleep(Duration::from_millis(20));
    };
    // Do not let descendants keep output pipes open after their worker exits.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let elapsed = started.elapsed().as_secs_f64();
    let (stdout, out_truncated) = out_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stdout reader panicked"))??;
    let (mut stderr, err_truncated) = err_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stderr reader panicked"))??;
    if let Some(reason) = prune_reason {
        stderr.push_str("\nsymGCAD search abandonment: ");
        stderr.push_str(&reason);
    }
    Ok(ProcessOutcome {
        status: forced_status
            .unwrap_or(if status.success() {
                "completed"
            } else {
                "failed"
            })
            .into(),
        wall_time_secs: elapsed,
        user_cpu_secs: worker_usage.ru_utime.tv_sec as f64
            + worker_usage.ru_utime.tv_usec as f64 / 1_000_000.0,
        system_cpu_secs: worker_usage.ru_stime.tv_sec as f64
            + worker_usage.ru_stime.tv_usec as f64 / 1_000_000.0,
        peak_rss_kib: peak,
        exit_code: status.code(),
        stdout,
        stderr,
        output_truncated: out_truncated || err_truncated,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn run_bounded(_command: &mut Command, _limits: &Limits) -> Result<ProcessOutcome> {
    bail!("resource supervision currently requires Linux")
}

#[cfg(not(target_os = "linux"))]
pub fn run_bounded_observed(
    _command: &mut Command,
    _limits: &Limits,
    _observer: impl FnMut(Duration) -> Option<String>,
) -> Result<ProcessOutcome> {
    bail!("resource supervision currently requires Linux")
}

#[cfg(target_os = "linux")]
fn process_group_rss(group: libc::pid_t) -> u64 {
    let members = live_process_tree(group);
    let mut rss = 0;
    for pid in members {
        if let Ok(status) = fs::read_to_string(format!("/proc/{pid}/status")) {
            if let Some(line) = status.lines().find(|s| s.starts_with("VmRSS:")) {
                rss += line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
            }
        }
    }
    rss
}

#[cfg(target_os = "linux")]
fn live_process_tree(root: libc::pid_t) -> std::collections::HashSet<libc::pid_t> {
    let mut members = std::collections::HashSet::from([root]);
    let mut pending = vec![root];
    // Follow every thread's children: a child can be created by a nonleader
    // thread, and nested CLI workers deliberately establish new process groups.
    // Do not scan the host's /proc directory: on shared machines that scan can
    // consume half of the worker's assigned CPU and distort its wall deadline.
    // Controlled workers retain their parent tree and use PDEATHSIG. A child
    // orphaned between samples could escape RSS accounting; this is not a
    // general-purpose sandbox for arbitrary daemonizing executables.
    while let Some(pid) = pending.pop() {
        let Ok(tasks) = fs::read_dir(format!("/proc/{pid}/task")) else {
            continue;
        };
        for task in tasks.flatten() {
            let Ok(children) = fs::read_to_string(task.path().join("children")) else {
                continue;
            };
            for child in children.split_whitespace().filter_map(|s| s.parse().ok()) {
                if members.insert(child) {
                    pending.push(child);
                }
            }
        }
    }
    members
}

/// Invoke only the independently built native upstream package. The adapter
/// does no production computation and its output is compatibility evidence.
pub fn run_reference(
    problem: &Problem,
    python: &Path,
    reference_root: &Path,
    adapter: &Path,
) -> Result<BenchmarkRecord> {
    run_reference_measured(problem, python, reference_root, adapter, None)
}

/// The supplied median must contain completed Rust fresh-process solves only.
/// Unknown timing keeps every qualification/standalone reference at most300s.
pub fn run_reference_measured(
    problem: &Problem,
    python: &Path,
    reference_root: &Path,
    adapter: &Path,
    completed_rust_median: Option<f64>,
) -> Result<BenchmarkRecord> {
    let budget = reference_budget(
        completed_rust_median,
        problem.limits.wall_time_secs,
        problem.limits.wall_time_secs,
        "reference_worker",
        None,
    )?;
    let mut problem = problem.clone();
    problem.limits.wall_time_secs = budget.effective_timeout_secs;
    let mut command = Command::new(python);
    command
        .arg(adapter)
        .arg("solve")
        .arg("--reference-root")
        .arg(reference_root);
    // JSON command argument avoids temp-file races and shell interpolation.
    command
        .arg("--problem-json")
        .arg(serde_json::to_string(&problem)?);
    command
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let process = run_bounded(&mut command, &problem.limits)?;
    let result = if process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
    {
        Some(serde_json::from_str(&process.stdout).context("reference emitted invalid JSON")?)
    } else {
        None
    };
    Ok(BenchmarkRecord {
        schema_version: 1,
        engine: "upstream-native-gcad".into(),
        problem: problem.name.clone(),
        process,
        result,
        limits: Some(problem.limits.clone()),
        reference_budget: Some(budget),
    })
}

pub fn append_record(path: &Path, record: &BenchmarkRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = std::io::BufWriter::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?,
    );
    serde_json::to_writer(&mut file, record)?;
    writeln!(file)?;
    file.flush()?;
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn adaptive_reference_budget_respects_floor_median_and_all_ceilings() -> Result<()> {
        assert_eq!(
            reference_budget(None, 3600., 3600., "qualification", None)?.effective_timeout_secs,
            300.
        );
        let measured = median(vec![20., 400., 200.]);
        let b = reference_budget(measured, 3600., 3000., "measurement", Some(0))?;
        assert_eq!(b.completed_rust_median_wall_secs, Some(200.));
        assert_eq!(b.adaptive_ceiling_secs, 600.);
        assert_eq!(b.effective_timeout_secs, 600.);
        assert_eq!(
            reference_budget(measured, 90., 3000., "test", None)?.effective_timeout_secs,
            90.
        );
        assert_eq!(
            reference_budget(measured, 3600., 41., "test", None)?.effective_timeout_secs,
            41.
        );
        assert_eq!(
            reference_budget(Some(2000.), 9000., 9000., "test", None)?.effective_timeout_secs,
            3600.
        );
        assert!(reference_budget(Some(f64::NAN), 300., 300., "test", None).is_err());
        assert!(reference_budget(None, 300., 0., "test", None).is_err());
        Ok(())
    }
    #[test]
    fn failed_or_partial_rust_runs_never_supply_a_completed_measurement() {
        let result =
            serde_json::json!({"status":"complete_generic","statistics":{"total_seconds":999999.}});
        let mut p = ProcessOutcome {
            status: "completed".into(),
            wall_time_secs: 123.,
            user_cpu_secs: 0.,
            system_cpu_secs: 0.,
            peak_rss_kib: 1,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            output_truncated: false,
        };
        assert_eq!(completed_solve_wall(&p, Some(&result)), Some(123.));
        p.status = "timeout".into();
        assert_eq!(completed_solve_wall(&p, Some(&result)), None);
        p.status = "completed".into();
        p.exit_code = Some(1);
        assert_eq!(completed_solve_wall(&p, Some(&result)), None);
        p.exit_code = Some(0);
        p.output_truncated = true;
        assert_eq!(completed_solve_wall(&p, Some(&result)), None);
        p.output_truncated = false;
        assert_eq!(
            completed_solve_wall(&p, Some(&serde_json::json!({"status":"incomplete"}))),
            None
        );
        p.wall_time_secs = f64::INFINITY;
        assert_eq!(completed_solve_wall(&p, Some(&result)), None);
    }
    #[test]
    fn reference_timeout_retains_effective_budget_and_censors_result() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "symgcad-reference-cap-{}-{}.sh",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::write(&path, "sleep 10\n")?;
        let p = Problem {
            limits: Limits {
                wall_time_secs: 0.06,
                memory_mib: 128,
                workers: 1,
                max_cells: None,
            },
            ..Default::default()
        };
        let record = run_reference(&p, Path::new("sh"), Path::new("unused"), &path)?;
        fs::remove_file(path)?;
        assert_eq!(record.process.status, "timeout");
        assert!(record.result.is_none());
        assert_eq!(record.limits.as_ref().unwrap().wall_time_secs, 0.06);
        let budget = record.reference_budget.unwrap();
        assert_eq!(budget.completed_rust_median_wall_secs, None);
        assert_eq!(budget.adaptive_ceiling_secs, 300.);
        assert_eq!(budget.effective_timeout_secs, 0.06);
        assert!(record.process.wall_time_secs < 2.);
        Ok(())
    }
    #[test]
    fn supervisor_kills_a_process_group_on_deadline() {
        let limits = Limits {
            wall_time_secs: 0.1,
            memory_mib: 128,
            workers: 1,
            ..Limits::default()
        };
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10 & wait"]);
        let outcome = run_bounded(&mut command, &limits).unwrap();
        assert_eq!(outcome.status, "timeout");
        assert!(outcome.wall_time_secs < 2.0);
    }
    #[test]
    fn heuristic_observer_kills_worker_without_claiming_completion() {
        let limits = Limits {
            wall_time_secs: 10.0,
            memory_mib: 128,
            workers: 1,
            ..Limits::default()
        };
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10 & wait"]);
        let outcome = run_bounded_observed(&mut command, &limits, |elapsed| {
            (elapsed.as_millis() >= 50).then(|| "measured term growth".into())
        })
        .unwrap();
        assert_eq!(outcome.status, "pruned");
        assert!(outcome.stderr.contains("measured term growth"));
        assert!(outcome.wall_time_secs < 2.0);
    }
    #[test]
    fn supervisor_captures_both_streams() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf result; printf diagnostic >&2"]);
        let outcome = run_bounded(&mut command, &Limits::default()).unwrap();
        assert_eq!(outcome.status, "completed");
        assert_eq!(outcome.stdout, "result");
        assert_eq!(outcome.stderr, "diagnostic");
    }
    #[test]
    fn nested_worker_groups_remain_in_memory_accounting() {
        let mut child = Command::new("sh")
            .args(["-c", "setsid sh -c 'sleep 10 & wait' & wait"])
            .spawn()
            .unwrap();
        let root = child.id() as libc::pid_t;
        let started = Instant::now();
        let members = loop {
            let members = live_process_tree(root);
            if members.len() >= 3 || started.elapsed() > Duration::from_secs(2) {
                break members;
            }
            thread::sleep(Duration::from_millis(10));
        };
        let rss = process_group_rss(root);
        // Clean up before assertions, including the child's distinct group.
        for &pid in &members {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        assert!(members.len() >= 3, "missing child/grandchild: {members:?}");
        assert!(rss > 0);
        assert!(!members.contains(&(std::process::id() as libc::pid_t)));
    }
}
