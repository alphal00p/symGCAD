use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    sync::atomic::{AtomicU64, Ordering},
};
use symgcad::{
    Problem,
    benchmark::{self, ProcessOutcome},
    output::{SolveResult, Status},
    solver::{prepare_projection, projection_table, selected_order, solve},
};

#[derive(Parser)]
#[command(
    version,
    about = "Exact generic cylindrical algebraic decomposition using Symbolica"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Solve {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Project {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Order {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Verify {
        input: PathBuf,
    },
    Bench {
        input: PathBuf,
    },
    Corpus {
        #[arg(long, default_value = "DO_NOT_PUSH_REFERENCES/gcad")]
        source: PathBuf,
        #[arg(long)]
        fixed: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        core: Option<PathBuf>,
    },
    #[command(name = "_worker", hide = true)]
    Worker {
        operation: String,
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

fn emit(value: &impl serde::Serialize, output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = io::BufWriter::new(fs::File::create(path)?);
            serde_json::to_writer(&mut file, value)?;
            file.flush()?;
        }
        None => {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            serde_json::to_writer_pretty(&mut lock, value)?;
            writeln!(lock)?;
        }
    }
    Ok(())
}

fn read_problem(input: &Path, output: Option<&Path>) -> Result<Option<Problem>> {
    let text = fs::read_to_string(input)?;
    match Problem::from_toml(&text) {
        Ok(problem) => Ok(Some(problem)),
        Err(error) => {
            let problem = Problem {
                name: Some(input.display().to_string()),
                ..Default::default()
            };
            emit(
                &SolveResult::failure(problem, Status::InvalidInput, format!("{error:#}")),
                output,
            )?;
            Ok(None)
        }
    }
}

fn temporary(label: &str) -> Result<PathBuf> {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    // Worker control/progress traffic must stay off a potentially remote
    // workspace filesystem. Durable algebra checkpoints and final results
    // still use the caller's requested paths.
    let directory = std::env::temp_dir().join("symgcad-workers");
    fs::create_dir_all(&directory)?;
    loop {
        let path = directory.join(format!(
            "{}-{}-{label}.json",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
}

fn bounded(
    problem: &Problem,
    operation: &str,
    search_report: Option<&Value>,
) -> Result<(ProcessOutcome, PathBuf, Option<Value>)> {
    bounded_trial(problem, operation, search_report, false)
}

fn bounded_trial(
    problem: &Problem,
    operation: &str,
    search_report: Option<&Value>,
    prune: bool,
) -> Result<(ProcessOutcome, PathBuf, Option<Value>)> {
    let input = temporary("input")?;
    let output = temporary("output")?;
    let progress = temporary("progress")?;
    let search_path = search_report
        .map(|report| -> Result<PathBuf> {
            let path = temporary("search")?;
            fs::write(&path, serde_json::to_vec(report)?)?;
            Ok(path)
        })
        .transpose()?;
    fs::write(&input, serde_json::to_vec(problem)?)?;
    let mut command = ProcessCommand::new(std::env::current_exe()?);
    command
        .arg("_worker")
        .arg(operation)
        .arg(&input)
        .arg("--output")
        .arg(&output);
    command
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1")
        .env("SYMGCAD_PROGRESS_PATH", &progress);
    if let Some(path) = &search_path {
        command.env("SYMGCAD_SEARCH_REPORT", path);
    }
    let mut last_check = std::time::Duration::ZERO;
    let mut last_metadata = None;
    let projection_variables =
        problem.variables.len() - usize::from(problem.solver.homogeneous_anchor.is_some());
    let outcome = benchmark::run_bounded_observed(&mut command, &problem.limits, |elapsed| {
        if !prune || elapsed.saturating_sub(last_check).as_millis() < 100 {
            return None;
        }
        last_check = elapsed;
        let metadata = fs::metadata(&progress).ok()?;
        let signature = (metadata.modified().ok()?, metadata.len());
        if last_metadata == Some(signature) || metadata.len() > 16 * 1024 * 1024 {
            return None;
        }
        last_metadata = Some(signature);
        let report: symgcad::optimized_projection::OptimizedProjectionReport =
            serde_json::from_slice(&fs::read(&progress).ok()?).ok()?;
        symgcad::search::growth_abandonment(&report.checkpoints, projection_variables)
            .map(|(_, reason)| reason)
    });
    let evidence = serde_json::from_slice::<Value>(&fs::read(&progress)?).ok();
    fs::remove_file(progress)?;
    fs::remove_file(input)?;
    if let Some(path) = search_path {
        fs::remove_file(path)?;
    }
    Ok((outcome?, output, evidence))
}

fn forward(
    problem: Problem,
    operation: &str,
    output: Option<PathBuf>,
    search: Option<&Value>,
) -> Result<()> {
    let (process, result_path, progress) = bounded(&problem, operation, search)?;
    if process.status == "completed"
        && !process.output_truncated
        && fs::metadata(&result_path)?.len() > 0
    {
        if let Some(output) = output {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&result_path, &output)?;
        } else {
            io::copy(&mut fs::File::open(&result_path)?, &mut io::stdout().lock())?;
            println!();
        }
    } else {
        let failure = interrupted_result(problem, &process, progress, search);
        emit(&failure, output.as_deref())?;
    }
    fs::remove_file(result_path)?;
    Ok(())
}

fn interrupted_result(
    problem: Problem,
    process: &ProcessOutcome,
    progress: Option<Value>,
    search: Option<&Value>,
) -> SolveResult {
    let status = if ["timeout", "memory_limit"].contains(&process.status.as_str()) {
        Status::Incomplete
    } else {
        Status::ComputationFailure
    };
    let mut failure = SolveResult::failure(
        problem,
        status,
        format!("worker {}: {}", process.status, process.stderr),
    );
    // The killed worker cannot supply a final solver timer. Retain its measured
    // wall cost, including initialization/export, without guessing phase costs.
    // Search happened in the parent before this worker, so attach it once below.
    failure.statistics.total_seconds = process.wall_time_secs;
    if let Some(progress) = progress {
        // LocalReport has serde defaults. Require its own identifying fields
        // instead of interpreting arbitrary JSON as an empty local snapshot.
        let local_shape = progress["speculative_nodes"].is_u64()
            && progress["refinements"].is_u64()
            && progress["active_phase"].is_string()
            && progress["current_prefix"].is_array();
        let optimized_shape = progress["checkpoints"].is_array() && progress["cache"].is_object();
        match (local_shape, optimized_shape) {
            (true, false) => {
                failure.statistics.interrupted_local_progress =
                    serde_json::from_value(progress).ok();
            }
            (false, true) => {
                failure.statistics.optimized_projection = serde_json::from_value(progress).ok();
            }
            _ => {} // Unknown, malformed or ambiguous diagnostics are optional.
        }
    }
    if let Some(search) = search {
        attach_search(&mut failure, search.clone());
    }
    failure
}

fn run_search(problem: &Problem) -> Result<Value> {
    use symgcad::search::{
        CheckpointReport, SearchCheckpointOptions, SearchOptions, TrialOutcome, TrialStatus,
        search, search_checkpointed,
    };
    let n = problem.variables.len();
    let mut groups: Vec<Vec<usize>> = if problem.variable_groups.is_empty() {
        vec![(0..n).collect()]
    } else {
        problem
            .variable_groups
            .iter()
            .map(|g| {
                g.iter()
                    .map(|v| problem.variables.iter().position(|n| n == v).unwrap())
                    .collect()
            })
            .collect()
    };
    if let Some(anchor) = &problem.solver.homogeneous_anchor {
        let anchor_index = problem.variables.iter().position(|v| v == anchor).unwrap();
        let group_index = groups
            .iter()
            .position(|g| g.contains(&anchor_index))
            .unwrap();
        anyhow::ensure!(
            groups[..group_index].iter().flatten().all(|&i| !problem
                .solver
                .homogeneous_block
                .contains(&problem.variables[i])),
            "homogeneous block conflicts with variable groups"
        );
        groups[group_index].retain(|&i| i != anchor_index);
        groups.insert(group_index, vec![anchor_index]);
        groups.retain(|g| !g.is_empty());
    }
    let start = std::time::Instant::now();
    let mut options = SearchOptions::for_campaign(problem.limits.wall_time_secs);
    options.seed = problem.solver.seed;
    options.max_rollouts = problem.solver.max_rollouts;
    if let Some(seconds) = problem.solver.search_time_secs {
        options.wall_time_secs = seconds.min(options.wall_time_secs);
    }
    let search_budget = options.wall_time_secs;
    let default_order = groups.iter().flatten().copied().collect::<Vec<_>>();
    let mut seeds = vec![default_order.clone()];
    let mut normalized_context = None;
    {
        let mut seed = problem.clone();
        seed.solver.order.clear();
        seed.variable_groups = groups
            .iter()
            .map(|g| g.iter().map(|&i| problem.variables[i].clone()).collect())
            .collect();
        seed.limits.wall_time_secs = (options.wall_time_secs * 0.2).min(5.0).max(0.01);
        let (out, path, _) = bounded(&seed, "seed_orders", None)?;
        if out.status == "completed" {
            if let Ok(value) = serde_json::from_slice::<Value>(&fs::read(&path)?) {
                normalized_context = value.get("search_context").cloned();
                if let Some(orders) = value["orders"].as_array() {
                    for order in orders {
                        let indices: Vec<usize> = serde_json::from_value(order.clone())?;
                        if !seeds.contains(&indices) {
                            seeds.push(indices);
                        }
                    }
                }
            }
        }
        fs::remove_file(path)?;
    }
    let priority = seeds
        .last()
        .unwrap()
        .iter()
        .rev()
        .copied()
        .collect::<Vec<_>>();
    let mut projection_progress = Vec::new();
    let evaluate = |request: &symgcad::search::TrialRequest| {
        let mut trial = problem.clone();
        trial.solver.order = request
            .order
            .iter()
            .map(|&i| problem.variables[i].clone())
            .collect();
        trial.solver.ordering = "given".into();
        trial.limits.wall_time_secs = request.timeout_secs;
        let prune = trial.solver.search_pruning && trial.solver.engine == "optimized";
        let (process, path, progress) = bounded_trial(&trial, "solve", None, prune)?;
        let summary = serde_json::from_str::<Value>(&process.stdout).ok();
        let complete = summary
            .as_ref()
            .is_some_and(|v| v["status"] == "complete_generic");
        let status = match process.status.as_str() {
            "completed" if complete => TrialStatus::Completed,
            "timeout" => TrialStatus::Timeout,
            "memory_limit" => TrialStatus::MemoryLimit,
            "pruned" => TrialStatus::Pruned,
            _ => TrialStatus::Failed,
        };
        let cells = summary
            .as_ref()
            .and_then(|v| v["cells"].as_u64())
            .map(|n| n as usize);
        fs::remove_file(path)?;
        let progress_score = progress.as_ref().and_then(|p| {
            let checkpoints = p["checkpoints"].as_array()?;
            let last = checkpoints.last()?;
            let cost = [
                "term_count",
                "polynomial_count",
                "max_degree",
                "max_coefficient_bits",
            ]
            .iter()
            .map(|k| (last[*k].as_f64().unwrap_or(0.0) + 1.0).ln())
            .sum::<f64>();
            Some((1.0 + cost) / checkpoints.len().max(1) as f64)
        });
        let promotion_score = problem
            .solver
            .search_local_progress
            .then(|| {
                let report: symgcad::local_projection::LocalReport =
                    serde_json::from_value(progress.as_ref()?.clone()).ok()?;
                symgcad::search::local_progress::promotion_score(
                    report.search_inventory.as_ref()?,
                    report.refinements,
                )
            })
            .flatten();
        let abandoned_prefix = if status == TrialStatus::Pruned {
            progress.as_ref().and_then(|p| {
                let report: symgcad::optimized_projection::OptimizedProjectionReport =
                    serde_json::from_value(p.clone()).ok()?;
                let variables =
                    trial.variables.len() - usize::from(trial.solver.homogeneous_anchor.is_some());
                let (depth, _) =
                    symgcad::search::growth_abandonment(&report.checkpoints, variables)?;
                Some(request.order.iter().rev().copied().take(depth).collect())
            })
        } else {
            None
        };
        projection_progress.push(json!({"order":request.order,"progress":progress}));
        Ok(TrialOutcome {
            status,
            elapsed_secs: process.wall_time_secs,
            cells,
            progress_score,
            promotion_score,
            message: if complete { None } else { Some(process.stderr) },
            abandoned_prefix,
        })
    };
    let mut context_problem = serde_json::to_value(problem)?;
    context_problem.as_object_mut().unwrap().remove("name");
    context_problem["limits"]
        .as_object_mut()
        .unwrap()
        .remove("wall_time_secs");
    for key in ["checkpoint_dir", "search_time_secs", "max_rollouts"] {
        context_problem["solver"]
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    options.wall_time_secs = (search_budget - start.elapsed().as_secs_f64()).max(f64::MIN_POSITIVE);
    let report = if let (Some(directory), Some(normalized)) =
        (&problem.solver.checkpoint_dir, normalized_context)
    {
        let mut context =
            json!({"problem":context_problem,"normalized":normalized,"scheduler":"uct-journal-v1"});
        if problem.solver.search_local_progress {
            context["timeout_promotion_policy"] = json!(symgcad::search::local_progress::POLICY);
        }
        let checkpoint = SearchCheckpointOptions {
            directory: directory.clone(),
            context: serde_json::to_string(&context)?,
        };
        search_checkpointed(&groups, &priority, &seeds, &options, &checkpoint, evaluate)?
    } else {
        let mut report = search(&groups, &priority, &seeds, &options, evaluate)?;
        if let Some(directory) = &problem.solver.checkpoint_dir {
            report.checkpoint=Some(CheckpointReport{path:directory.join("searches"),rejected:Some("bounded normalization/seed worker did not return an exact context; search persistence disabled".into()),..Default::default()});
        }
        report
    };
    Ok(
        json!({"schema_version":1,"problem":problem,"search":report,"projection_progress":projection_progress,"total_seconds":start.elapsed().as_secs_f64()}),
    )
}

fn enable_search_checkpoints(problem: &mut Problem) {
    if problem.solver.checkpoint_dir.is_none() {
        problem.solver.checkpoint_dir = Some(PathBuf::from("artifacts/checkpoints"));
    }
}

fn attach_search(result: &mut SolveResult, report: Value) {
    let seconds = report["total_seconds"].as_f64().unwrap_or(0.0);
    result.statistics.search_seconds = seconds;
    result.statistics.ordering_seconds += seconds;
    result.statistics.total_seconds += seconds;
    result.statistics.ordering_search = Some(report);
}

fn worker(operation: &str, input: &Path, output: &Path) -> Result<()> {
    if operation == "verify" {
        // Large lifting traces can exceed a gigabyte. Stream their JSON so the
        // verifier does not retain a second, complete copy of the serialized
        // evidence alongside the parsed cell/trace structures.
        let result: SolveResult =
            serde_json::from_reader(io::BufReader::new(fs::File::open(input)?))?;
        let verification = symgcad::verify::verify(&result)?;
        emit(&verification, Some(output))?;
        return emit(&verification, None);
    }
    let problem: Problem = serde_json::from_slice(&fs::read(input)?)?;
    if let Err(error) = symgcad::problem::normalize(&problem, &problem.variables) {
        let failure = SolveResult::failure(problem, Status::InvalidInput, format!("{error:#}"));
        emit(&failure, Some(output))?;
        return emit(
            &json!({"status":"invalid_input","diagnostic":failure.diagnostic}),
            None,
        );
    }
    match operation {
        "solve" => {
            let mut result = match solve(&problem) {
                Ok(result) => result,
                Err(e) => {
                    SolveResult::failure(problem, Status::ComputationFailure, format!("{e:#}"))
                }
            };
            if let Some(path) = std::env::var_os("SYMGCAD_SEARCH_REPORT") {
                attach_search(&mut result, serde_json::from_slice(&fs::read(path)?)?);
            }
            emit(&result, Some(output))?;
            let mut statistics = serde_json::to_value(&result.statistics)?;
            if let Some(report) = statistics
                .get_mut("optimized_projection")
                .and_then(Value::as_object_mut)
            {
                for field in ["removals", "initial_removals"] {
                    if let Some(certificates) = report.remove(field) {
                        report.insert(
                            format!("{field}_count"),
                            json!(certificates.as_array().map_or(0, Vec::len)),
                        );
                    }
                }
            }
            emit(
                &json!({"status":result.status,"cells":result.cells.len(),"merged_cells":result.merged_cells.as_ref().map(Vec::len),"statistics":statistics}),
                None,
            )?;
        }
        "project" => {
            let order = selected_order(&problem)?;
            let (normalized, projection, optimization) = prepare_projection(&problem, &order)?;
            let (polynomials, levels) = projection_table(&normalized.algebra, &projection);
            emit(
                &json!({"schema_version":1,"order":order,"polynomials":polynomials,"levels":levels,"steps":projection.steps,"optimization":optimization}),
                Some(output),
            )?;
        }
        "order" => {
            emit(
                &json!({"schema_version":1,"order":selected_order(&problem)?}),
                Some(output),
            )?;
        }
        "seed_orders" => {
            let normalized = symgcad::problem::normalize(&problem, &problem.variables)?;
            // Normalize and hash inside the same killable seed worker. A parent
            // without this complete context must not load a search checkpoint.
            use sha2::{Digest, Sha256};
            let mut executable = fs::File::open(std::env::current_exe()?)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 65536];
            loop {
                let n = std::io::Read::read(&mut executable, &mut buffer)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            let search_context = json!({"variables":normalized.algebra.names,"positive":normalized.positives.iter().map(ToString::to_string).collect::<Vec<_>>(),"split":normalized.split.iter().map(ToString::to_string).collect::<Vec<_>>(),"backend":symbolica::prelude::LicenseManager::get_version(),"executable_sha256":format!("{:x}",hasher.finalize())});
            let mut all = normalized.positives;
            all.extend(normalized.split);
            let groups = if problem.variable_groups.is_empty() {
                vec![(0..problem.variables.len()).collect()]
            } else {
                problem
                    .variable_groups
                    .iter()
                    .map(|g| {
                        g.iter()
                            .map(|v| problem.variables.iter().position(|n| n == v).unwrap())
                            .collect()
                    })
                    .collect()
            };
            emit(
                &json!({"orders":symgcad::ordering::cheap_seed_orders(&all, &groups),"search_context":search_context}),
                Some(output),
            )?;
        }
        _ => anyhow::bail!("unknown worker operation"),
    }
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Worker {
            operation,
            input,
            output,
        } => worker(&operation, &input, &output),
        Command::Solve { input, output } => {
            let Some(mut problem) = read_problem(&input, output.as_deref())? else {
                return Ok(());
            };
            let mut search_metadata = None;
            if problem.solver.ordering == "mcts" && problem.solver.order.is_empty() {
                enable_search_checkpoints(&mut problem);
                let start = std::time::Instant::now();
                let report = run_search(&problem)?;
                let indices: Vec<usize> =
                    serde_json::from_value(report["search"]["suggested_order"].clone())?;
                problem.solver.order = indices
                    .into_iter()
                    .map(|i| problem.variables[i].clone())
                    .collect();
                problem.solver.ordering = "given".into();
                problem.limits.wall_time_secs =
                    (problem.limits.wall_time_secs - start.elapsed().as_secs_f64()).max(0.001);
                search_metadata = Some(report);
            }
            forward(problem, "solve", output, search_metadata.as_ref())
        }
        Command::Project { input, output } => {
            let Some(problem) = read_problem(&input, output.as_deref())? else {
                return Ok(());
            };
            forward(problem, "project", output, None)
        }
        Command::Order { input, output } => {
            let Some(mut problem) = read_problem(&input, output.as_deref())? else {
                return Ok(());
            };
            if problem.solver.ordering == "mcts" && problem.solver.order.is_empty() {
                enable_search_checkpoints(&mut problem);
                emit(&run_search(&problem)?, output.as_deref())
            } else {
                forward(problem, "order", output, None)
            }
        }
        Command::Verify { input } => {
            let output = temporary("verification")?;
            let mut command = ProcessCommand::new(std::env::current_exe()?);
            command
                .arg("_worker")
                .arg("verify")
                .arg(input)
                .arg("--output")
                .arg(&output);
            let outcome = benchmark::run_bounded(&mut command, &symgcad::Limits::default())?;
            let _ = fs::remove_file(output);
            anyhow::ensure!(
                outcome.status == "completed",
                "verification {}: {}",
                outcome.status,
                outcome.stderr
            );
            print!("{}", outcome.stdout);
            Ok(())
        }
        Command::Bench { input } => {
            let configuration: toml::Value = toml::from_str(&fs::read_to_string(&input)?)?;
            if configuration
                .get("mode")
                .and_then(toml::Value::as_str)
                .is_some_and(|mode| ["completion", "ablation"].contains(&mode))
            {
                return emit(
                    &symgcad::campaign::run_campaign(&input, &std::env::current_exe()?)?,
                    None,
                );
            }
            let report = benchmark::run_campaign(&input, &std::env::current_exe()?)?;
            emit(&report, None)
        }
        Command::Corpus {
            source,
            fixed,
            output,
            core,
        } => {
            let inventory = symgcad::corpus::inventory(&source)?;
            if let Some(directory) = output {
                let mut problems = symgcad::corpus::import_zz(&source, fixed)?;
                if let Some(core) = core {
                    problems.extend(symgcad::corpus::import_core_json(&core)?);
                }
                let files = symgcad::corpus::export_toml(&problems, &directory)?;
                let groups = symgcad::corpus::write_equivalence_manifest(
                    &problems,
                    &directory.join("equivalence.json"),
                )?;
                emit(&inventory, Some(&directory.join("inventory.json")))?;
                emit(
                    &json!({"collected_tests":inventory.collected_count,"disabled_tests":inventory.disabled_count,"fixtures":inventory.fixtures.len(),"exported":files.len(),"distinct_problems":groups.len(),"directory":directory}),
                    None,
                )
            } else {
                emit(&inventory, None)
            }
        }
    }
}

#[cfg(test)]
mod interrupted_tests {
    use super::*;

    fn process(status: &str) -> ProcessOutcome {
        ProcessOutcome {
            status: status.into(),
            wall_time_secs: 1200.25,
            user_cpu_secs: 1100.0,
            system_cpu_secs: 10.0,
            peak_rss_kib: 1024,
            exit_code: None,
            stdout: "{\"status\":\"complete_generic\"}".into(),
            stderr: "interrupted".into(),
            output_truncated: false,
        }
    }

    fn assert_no_proof(result: &SolveResult) {
        assert!(result.cells.is_empty());
        assert!(result.trace.is_none());
        assert!(result.local.is_none());
        assert!(result.statistics.projection_steps.is_empty());
        assert_eq!(result.statistics.projection_seconds, 0.0);
        assert_eq!(result.statistics.lifting_seconds, 0.0);
        assert_eq!(result.statistics.merging_seconds, 0.0);
        assert!(symgcad::verify::verify(result).is_err());
    }

    #[test]
    fn interrupted_wall_and_search_are_counted_once_without_phase_inference() {
        let search = json!({"total_seconds":300.5,"search":{"trials":[]}});
        for status in ["timeout", "memory_limit", "failed"] {
            let plain = interrupted_result(Problem::default(), &process(status), None, None);
            assert_eq!(plain.statistics.total_seconds, 1200.25);
            assert_eq!(plain.statistics.search_seconds, 0.0);
            assert_eq!(plain.statistics.ordering_seconds, 0.0);
            assert_no_proof(&plain);
            let searched =
                interrupted_result(Problem::default(), &process(status), None, Some(&search));
            assert_eq!(searched.statistics.total_seconds, 1500.75);
            assert_eq!(searched.statistics.search_seconds, 300.5);
            assert_eq!(searched.statistics.ordering_seconds, 300.5);
            assert_eq!(searched.statistics.ordering_search, Some(search.clone()));
            assert_eq!(
                searched.status,
                if status == "failed" {
                    Status::ComputationFailure
                } else {
                    Status::Incomplete
                }
            );
            assert_no_proof(&searched);
        }
    }

    #[test]
    fn interrupted_progress_retains_typed_diagnostics_without_coverage() {
        let local = symgcad::local_projection::LocalReport {
            speculative_nodes: 23,
            refinements: 7,
            active_phase: "resultant".into(),
            current_prefix: vec!["3/2".into()],
            elapsed_seconds: 1190.0,
            ..Default::default()
        };
        let result = interrupted_result(
            Problem::default(),
            &process("timeout"),
            Some(serde_json::to_value(local).unwrap()),
            None,
        );
        let snapshot = result
            .statistics
            .interrupted_local_progress
            .as_ref()
            .unwrap();
        assert_eq!(snapshot.speculative_nodes, 23);
        assert_eq!(snapshot.refinements, 7);
        assert_eq!(snapshot.current_prefix, ["3/2"]);
        assert!(result.statistics.optimized_projection.is_none());
        assert_eq!(result.statistics.total_seconds, 1200.25);
        assert_no_proof(&result);

        let optimized = symgcad::optimized_projection::OptimizedProjectionReport::default();
        let result = interrupted_result(
            Problem::default(),
            &process("timeout"),
            Some(serde_json::to_value(optimized).unwrap()),
            None,
        );
        assert!(result.statistics.interrupted_local_progress.is_none());
        assert!(result.statistics.optimized_projection.is_some());
        assert_no_proof(&result);
    }

    #[test]
    fn malformed_or_ambiguous_progress_is_not_reinterpreted() {
        for progress in [
            json!({}),
            json!({"status":"complete_generic","cells":[{}]}),
            json!({"speculative_nodes":0,"refinements":0,"active_phase":"x","current_prefix":[3]}),
            json!({"checkpoints":[],"cache":false}),
            json!({"speculative_nodes":0,"refinements":0,"active_phase":"x","current_prefix":[],"checkpoints":[],"cache":{}}),
        ] {
            let result = interrupted_result(
                Problem::default(),
                &process("timeout"),
                Some(progress),
                None,
            );
            assert!(result.statistics.interrupted_local_progress.is_none());
            assert!(result.statistics.optimized_projection.is_none());
            assert_no_proof(&result);
        }
    }

    #[test]
    fn historical_statistics_default_to_no_interrupted_progress() {
        let statistics: symgcad::output::Statistics =
            serde_json::from_value(json!({"total_seconds":12.0})).unwrap();
        assert!(statistics.interrupted_local_progress.is_none());
        assert!(
            serde_json::to_value(statistics)
                .unwrap()
                .get("interrupted_local_progress")
                .is_none()
        );
    }
}
