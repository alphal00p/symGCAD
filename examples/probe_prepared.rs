//! Bounded direct-worker diagnostics for an already prepared mathematical problem.
//! Only resource limits, explicit local caps and a fresh checkpoint path change.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use symgcad::{
    Limits, Problem,
    benchmark::{ProcessOutcome, run_bounded, run_bounded_observed},
};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    executable: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, default_value_t = 120.0)]
    verification_seconds: f64,
    #[arg(long)]
    campaign_seconds: Option<f64>,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
    #[arg(long)]
    cpu: usize,
    #[arg(long)]
    max_refinements: Option<usize>,
    #[arg(long)]
    max_nodes: Option<usize>,
    /// Record the preceding proposal-generation cost separately from the solve.
    #[arg(long)]
    discovery_report: Option<PathBuf>,
}

fn validate(args: &Args) -> Result<f64> {
    let campaign = args
        .campaign_seconds
        .unwrap_or(args.seconds + args.verification_seconds + 30.0);
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0,
        "invalid solve deadline"
    );
    ensure!(
        args.verification_seconds.is_finite() && args.verification_seconds > 0.0,
        "invalid verification deadline"
    );
    ensure!(
        campaign.is_finite()
            && campaign >= args.seconds + args.verification_seconds
            && campaign <= 3600.0,
        "campaign must cover both workers and remain within one hour"
    );
    ensure!(
        (1..=65536).contains(&args.memory_mib),
        "invalid worker memory limit"
    );
    ensure!(
        args.max_refinements != Some(0) && args.max_nodes != Some(0),
        "local caps must be positive"
    );
    ensure!(
        args.cpu < libc::CPU_SETSIZE as usize,
        "CPU index outside affinity mask"
    );
    Ok(campaign)
}

fn fresh_directory(path: &Path) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(path).context("output must be a fresh directory")?;
    Ok(path.canonicalize()?)
}

fn pin_cpu(cpu: usize) -> Result<()> {
    let mut allowed = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    ensure!(
        unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&allowed), &mut allowed) } == 0,
        "cannot read CPU affinity"
    );
    ensure!(
        unsafe { libc::CPU_ISSET(cpu, &allowed) },
        "requested CPU is unavailable in this process affinity"
    );
    let mut selected = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_SET(cpu, &mut selected);
    }
    ensure!(
        unsafe { libc::sched_setaffinity(0, std::mem::size_of_val(&selected), &selected) } == 0,
        "cannot set CPU affinity"
    );
    Ok(())
}

fn hash(path: &Path, deadline: Instant) -> Result<String> {
    let mut input = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        ensure!(
            Instant::now() < deadline,
            "campaign deadline during hashing"
        );
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn mathematical_fields(problem: &Problem) -> Result<Value> {
    let mut value = serde_json::to_value(problem)?;
    value.as_object_mut().unwrap().remove("solver");
    value.as_object_mut().unwrap().remove("limits");
    Ok(value)
}

fn prepare(source: &Problem, args: &Args, checkpoints: PathBuf) -> Result<Problem> {
    let mut problem = source.clone();
    problem.limits.wall_time_secs = args.seconds;
    problem.limits.memory_mib = args.memory_mib;
    problem.limits.workers = 1;
    problem.solver.checkpoint_dir = Some(checkpoints);
    if let Some(cap) = args.max_refinements {
        problem.solver.local_max_refinements = Some(cap);
    }
    if let Some(cap) = args.max_nodes {
        problem.solver.local_max_nodes = Some(cap);
    }
    problem.validate()?;
    ensure!(
        mathematical_fields(source)? == mathematical_fields(&problem)?,
        "mathematical input changed"
    );
    ensure!(
        source.solver.order == problem.solver.order
            && source.solver.ordering == problem.solver.ordering,
        "ordering changed"
    );
    Ok(problem)
}

const SMALL_LIMIT: u64 = 16 * 1024 * 1024;
fn read_small<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let meta = fs::metadata(path)?;
    ensure!(
        meta.is_file() && meta.len() <= SMALL_LIMIT,
        "diagnostic file exceeds 16 MiB"
    );
    let mut reader = BufReader::new(fs::File::open(path)?.take(SMALL_LIMIT + 1));
    let value = serde_json::from_reader(&mut reader)?;
    ensure!(
        reader.into_inner().limit() > 0,
        "diagnostic file grew beyond byte cap"
    );
    Ok(value)
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Progress {
    active_phase: String,
    elapsed_seconds: f64,
    current_level: Option<usize>,
    current_prefix: Vec<String>,
    speculative_nodes: usize,
    refinements: usize,
    refinement_cuts: usize,
    projected_pairs: usize,
    strict_bezout_hints_accepted: usize,
    strict_bezout_hints_rejected: usize,
    strict_bezout_resultants_omitted: usize,
    strict_bezout_resultant_fallbacks: usize,
    strict_bezout_preparation_seconds: f64,
    subtree_cache_hits: usize,
    subtree_cache_misses: usize,
    subtree_cache_evictions: usize,
    subtree_cache_retained_bytes: usize,
    learned_cut_cache_hits: usize,
    learned_cut_seeded_cuts: usize,
    learned_cut_cache_evictions: usize,
    learned_cut_cache_retained_bytes: usize,
    stable_sample_hits: usize,
    stable_sample_recorded: usize,
    stable_sample_evictions: usize,
    cache: Value,
    phases: Value,
}

#[derive(Deserialize, Serialize)]
struct Summary {
    status: String,
    #[serde(default)]
    cells: Option<usize>,
    #[serde(default)]
    diagnostic: Option<String>,
    #[serde(default)]
    statistics: Option<Value>,
}

fn successful_summary(process: &ProcessOutcome) -> Result<Option<Summary>> {
    if process.status != "completed" || process.exit_code != Some(0) {
        return Ok(None);
    }
    ensure!(
        !process.output_truncated && process.stdout.len() <= SMALL_LIMIT as usize,
        "worker summary is truncated or oversized"
    );
    let summary: Summary =
        serde_json::from_str(&process.stdout).context("read bounded worker stdout summary")?;
    ensure!(
        summary.status != "complete_generic" || summary.cells.is_some(),
        "complete worker summary lacks cell count"
    );
    Ok(Some(summary))
}

fn worker(executable: &Path, operation: &str, input: &Path, output: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["_worker", operation])
        .arg(input)
        .arg("--output")
        .arg(output)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1")
        .env_remove("SYMGCAD_CAPTURE_DIR")
        .env_remove("SYMGCAD_PROGRESS_PATH")
        .env_remove("SYMGCAD_LIFTING_PROGRESS_PATH");
    command
}

fn main() -> Result<()> {
    let args = Args::parse();
    let campaign_seconds = validate(&args)?;
    let started = Instant::now();
    let deadline = started + Duration::from_secs_f64(campaign_seconds);
    pin_cpu(args.cpu)?;
    let executable = args.executable.canonicalize()?;
    let source_path = args.input.canonicalize()?;
    ensure!(
        executable.is_file() && source_path.is_file(),
        "input and executable must be regular files"
    );
    let output = fresh_directory(&args.output)?;
    let source_bytes = fs::read(&source_path)?;
    let source = Problem::from_toml(std::str::from_utf8(&source_bytes)?)?;
    let checkpoint = output.join("checkpoints");
    let capture = output.join("capture");
    fs::create_dir(&checkpoint)?;
    fs::create_dir(&capture)?;
    let problem = prepare(&source, &args, checkpoint.clone())?;
    fs::write(output.join("source.toml"), &source_bytes)?;
    write(&output.join("input.json"), &problem)?;
    fs::write(output.join("input.toml"), toml::to_string_pretty(&problem)?)?;
    let discovery = if let Some(path) = &args.discovery_report {
        let report: Value = read_small(path)?;
        Some(
            json!({"path":path.canonicalize()?,"sha256":hash(path,deadline)?,"elapsed_seconds":report.get("elapsed_seconds"),"included_in_probe_wall_time":false}),
        )
    } else {
        None
    };
    let provenance = json!({"schema_version":1,"executable":executable,"executable_sha256":hash(&executable,deadline)?,"harness_sha256":hash(&std::env::current_exe()?,deadline)?,"source":source_path,"source_sha256":format!("{:x}",Sha256::digest(&source_bytes)),"input_json_sha256":hash(&output.join("input.json"),deadline)?,"original_mathematical_fields_preserved":true,"original_order_preserved":true,"cold_checkpoints":checkpoint,"capture":capture,"cpu":args.cpu,"limits":problem.limits,"max_refinements":problem.solver.local_max_refinements,"max_nodes":problem.solver.local_max_nodes,"verification_seconds":args.verification_seconds,"campaign_seconds":campaign_seconds,"discovery":discovery});
    write(&output.join("provenance.json"), &provenance)?;
    let progress_path = output.join("progress.json");
    let lifting_path = output.join("lifting-progress.json");
    let result_path = output.join("result.json");
    let mut command = worker(
        &executable,
        "solve",
        &output.join("input.json"),
        &result_path,
    );
    command
        .env("SYMGCAD_PROGRESS_PATH", &progress_path)
        .env("SYMGCAD_LIFTING_PROGRESS_PATH", &lifting_path)
        .env("SYMGCAD_CAPTURE_DIR", &capture);
    let mut history = fs::File::create(output.join("progress-history.jsonl"))?;
    let mut next_snapshot = Duration::ZERO;
    let mut snapshot_errors = 0usize;
    let mut solve_limits = problem.limits.clone();
    solve_limits.wall_time_secs = solve_limits.wall_time_secs.min(
        deadline
            .saturating_duration_since(Instant::now())
            .as_secs_f64(),
    );
    let process = run_bounded_observed(&mut command, &solve_limits, |elapsed| {
        if elapsed >= next_snapshot {
            next_snapshot = elapsed + Duration::from_secs(1);
            if let Ok(progress) = read_small::<Progress>(&progress_path) {
                let snapshot =
                    json!({"observer_seconds":elapsed.as_secs_f64(),"progress":progress});
                if serde_json::to_writer(&mut history, &snapshot)
                    .and_then(|_| history.write_all(b"\n").map_err(serde_json::Error::io))
                    .is_err()
                {
                    snapshot_errors += 1;
                }
            }
        }
        None
    })?;
    write(&output.join("solve-process.json"), &process)?;
    let final_progress = read_small::<Progress>(&progress_path).ok();
    if let Some(progress) = &final_progress {
        serde_json::to_writer(
            &mut history,
            &json!({"observer_seconds":process.wall_time_secs,"final":true,"progress":progress}),
        )?;
        history.write_all(b"\n")?;
    }
    history.flush()?;
    let selection = successful_summary(&process);
    let summary_error = selection.as_ref().err().map(ToString::to_string);
    let summary = selection.unwrap_or(None);
    let complete = summary
        .as_ref()
        .is_some_and(|s| s.status == "complete_generic");
    let mut verification_process = None;
    let mut verification = None;
    let mut proof_sha256 = None;
    let mut proof_unchanged = false;
    if complete && Instant::now() < deadline {
        proof_sha256 = hash(&result_path, deadline).ok();
        if proof_sha256.is_some() && Instant::now() < deadline {
            let path = output.join("verification.json");
            let mut command = worker(&executable, "verify", &result_path, &path);
            let limits = Limits {
                wall_time_secs: args.verification_seconds.min(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .as_secs_f64(),
                ),
                memory_mib: args.memory_mib,
                workers: 1,
                max_cells: None,
            };
            let process = run_bounded(&mut command, &limits)?;
            write(&output.join("verification-process.json"), &process)?;
            verification = read_small::<Value>(&path).ok();
            verification_process = Some(process);
            proof_unchanged = hash(&result_path, deadline).ok().as_ref() == proof_sha256.as_ref();
        }
    }
    let verified = proof_unchanged
        && verification_process
            .as_ref()
            .is_some_and(|p| p.status == "completed" && p.exit_code == Some(0))
        && verification.as_ref().is_some_and(|v| {
            v["verified"] == true
                && v["cells"].as_u64() == summary.as_ref().and_then(|s| s.cells).map(|n| n as u64)
        });
    let report = json!({"schema_version":1,"provenance":provenance,"solve_process":process,"summary":summary,"summary_error":summary_error,"final_progress":final_progress,"progress_snapshot_write_errors":snapshot_errors,"verification_process":verification_process,"verification":verification,"proof_sha256":proof_sha256,"proof_unchanged":proof_unchanged,"verified":verified,"elapsed_seconds":started.elapsed().as_secs_f64(),"scope":"Direct prepared-problem diagnostic. A resource exit or partial proof is never a completed decomposition."});
    write(&output.join("report.json"), &report)?;
    println!("verified={verified}, output={}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args() -> Args {
        Args {
            input: "input.toml".into(),
            executable: "symgcad".into(),
            output: "out".into(),
            seconds: 120.,
            verification_seconds: 120.,
            campaign_seconds: Some(270.),
            memory_mib: 8192,
            cpu: 8,
            max_refinements: Some(1_000_000),
            max_nodes: Some(10_000_000),
            discovery_report: None,
        }
    }
    #[test]
    fn preparation_preserves_every_mathematical_and_ordering_field() -> Result<()> {
        let mut source = Problem::from_toml(
            "variables=['s','x']\nconstraints=['s-4','x','s*x-1']\nvariable_groups=[['s'],['x']]\n[solver]\nengine='local'\norder=['s','x']\n",
        )?;
        source.solver.positivity = true;
        let prepared = prepare(&source, &args(), "cold".into())?;
        assert_eq!(
            mathematical_fields(&source)?,
            mathematical_fields(&prepared)?
        );
        assert_eq!(source.solver.order, prepared.solver.order);
        assert_eq!(
            source.solver.homogeneous_anchor,
            prepared.solver.homogeneous_anchor
        );
        let mut expected = source.clone();
        expected.limits = prepared.limits.clone();
        expected.solver.checkpoint_dir = prepared.solver.checkpoint_dir.clone();
        expected.solver.local_max_refinements = prepared.solver.local_max_refinements;
        expected.solver.local_max_nodes = prepared.solver.local_max_nodes;
        assert_eq!(
            serde_json::to_value(expected)?,
            serde_json::to_value(prepared)?
        );
        Ok(())
    }
    #[test]
    fn resource_bounds_and_fresh_outputs_are_enforced() -> Result<()> {
        let mut a = args();
        assert!(validate(&a).is_ok());
        a.seconds = f64::NAN;
        assert!(validate(&a).is_err());
        a = args();
        a.campaign_seconds = Some(3601.);
        assert!(validate(&a).is_err());
        a = args();
        a.memory_mib = 0;
        assert!(validate(&a).is_err());
        a = args();
        a.max_nodes = Some(0);
        assert!(validate(&a).is_err());
        let path =
            std::env::temp_dir().join(format!("symgcad-prepared-probe-{}", std::process::id()));
        fresh_directory(&path)?;
        fs::write(path.join("sentinel"), "keep")?;
        assert!(fresh_directory(&path).is_err());
        assert_eq!(fs::read_to_string(path.join("sentinel"))?, "keep");
        fs::remove_dir_all(path)?;
        Ok(())
    }
    #[test]
    fn killed_workers_never_receive_completion_from_stdout() -> Result<()> {
        let mut process = ProcessOutcome {
            status: "timeout".into(),
            wall_time_secs: 1.,
            user_cpu_secs: 0.,
            system_cpu_secs: 0.,
            peak_rss_kib: 0,
            exit_code: None,
            stdout: r#"{"status":"complete_generic","cells":1}"#.into(),
            stderr: String::new(),
            output_truncated: false,
        };
        assert!(successful_summary(&process)?.is_none());
        process.status = "completed".into();
        process.exit_code = Some(0);
        assert_eq!(successful_summary(&process)?.unwrap().cells, Some(1));
        process.stdout = r#"{"status":"complete_generic"}"#.into();
        assert!(successful_summary(&process).is_err());
        Ok(())
    }
}
