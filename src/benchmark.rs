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
            reference_timeout_secs: 300.0,
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
        "timing_policy":"fixed order; projection + lifting + merging; normalization and startup reported separately"
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
        };
        if remaining() < 0.05 {
            exhausted = true;
            summary.status = "campaign_exhausted".into();
            summaries.push(summary);
            continue;
        }
        problem.limits.wall_time_secs = config.reference_timeout_secs.min(remaining());
        let record = run_reference(problem, &python, &reference_root, &adapter)?;
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
        }
        summaries.push(summary);
    }
    fs::write(
        output.join("frozen-reference-set.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"schema_version":1,"cases":summaries,"problems":problems}),
        )?,
    )?;
    // Phase 2: only the already-frozen set contributes to parity. The manifest
    // above records reference timeouts as unresolved inputs rather than losses.
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
            let record = run_reference(problem, &python, &reference_root, &adapter)?;
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
            if let Some(result) = result
                .as_ref()
                .filter(|r| r["status"] == "complete_generic")
            {
                rust_wall.push(process.wall_time_secs);
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
            };
            fs::write(
                output.join(format!("{i:04}-rust-{repetition}.json")),
                serde_json::to_vec_pretty(&record)?,
            )?;
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
            s.status = if exhausted {
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
                };
                fs::write(
                    output.join(format!("{i:04}-verification-process.json")),
                    serde_json::to_vec_pretty(&record)?,
                )?;
            }
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
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let cap = libc::rlimit {
                rlim_cur: memory,
                rlim_max: memory,
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
        }
        let mut raw_status = 0;
        let mut usage = unsafe { std::mem::zeroed::<libc::rusage>() };
        let waited = unsafe { libc::wait4(pid, &mut raw_status, libc::WNOHANG, &mut usage) };
        if waited == pid {
            peak = peak.max(usage.ru_maxrss.max(0) as u64);
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
    let (stderr, err_truncated) = err_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stderr reader panicked"))??;
    Ok(ProcessOutcome {
        status: forced_status
            .unwrap_or(if status.success() {
                "completed"
            } else {
                "failed"
            })
            .into(),
        wall_time_secs: elapsed,
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

#[cfg(target_os = "linux")]
fn process_group_rss(group: libc::pid_t) -> u64 {
    let Ok(entries) = fs::read_dir("/proc") else {
        return 0;
    };
    let mut rss = 0;
    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        // comm can contain spaces and parentheses; fields after its final ')'
        // begin with state (3), ppid (4), pgrp (5).
        let Some((_, tail)) = stat.rsplit_once(')') else {
            continue;
        };
        if tail
            .split_whitespace()
            .nth(2)
            .and_then(|s| s.parse::<i32>().ok())
            != Some(group)
        {
            continue;
        }
        if let Ok(status) = fs::read_to_string(entry.path().join("status")) {
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

/// Invoke only the independently built native upstream package. The adapter
/// does no production computation and its output is compatibility evidence.
pub fn run_reference(
    problem: &Problem,
    python: &Path,
    reference_root: &Path,
    adapter: &Path,
) -> Result<BenchmarkRecord> {
    let mut command = Command::new(python);
    command
        .arg(adapter)
        .arg("solve")
        .arg("--reference-root")
        .arg(reference_root);
    // JSON command argument avoids temp-file races and shell interpolation.
    command
        .arg("--problem-json")
        .arg(serde_json::to_string(problem)?);
    command
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let process = run_bounded(&mut command, &problem.limits)?;
    let result = if process.status == "completed" && !process.output_truncated {
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
    })
}

pub fn append_record(path: &Path, record: &BenchmarkRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    serde_json::to_writer(&mut file, record)?;
    writeln!(file)?;
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
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
    fn supervisor_captures_both_streams() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf result; printf diagnostic >&2"]);
        let outcome = run_bounded(&mut command, &Limits::default()).unwrap();
        assert_eq!(outcome.status, "completed");
        assert_eq!(outcome.stdout, "result");
        assert_eq!(outcome.stderr, "diagnostic");
    }
}
