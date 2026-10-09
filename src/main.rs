use anyhow::{Context, Result};
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
    problem::normalize,
    projection::project,
    solver::{projection_table, selected_order, solve},
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
            let file = fs::File::create(path)?;
            serde_json::to_writer(file, value)?;
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

fn temporary(label: &str) -> Result<PathBuf> {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let directory = PathBuf::from("artifacts/workers");
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

fn bounded(problem: &Problem, operation: &str) -> Result<(ProcessOutcome, PathBuf)> {
    let input = temporary("input")?;
    let output = temporary("output")?;
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
        .env("OMP_NUM_THREADS", "1");
    let outcome = benchmark::run_bounded(&mut command, &problem.limits);
    fs::remove_file(input)?;
    Ok((outcome?, output))
}

fn forward(problem: Problem, operation: &str, output: Option<PathBuf>) -> Result<()> {
    let (process, result_path) = bounded(&problem, operation)?;
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
        let status = if ["timeout", "memory_limit"].contains(&process.status.as_str()) {
            Status::Incomplete
        } else {
            Status::ComputationFailure
        };
        let failure = SolveResult::failure(
            problem,
            status,
            format!("worker {}: {}", process.status, process.stderr),
        );
        emit(&failure, output.as_deref())?;
    }
    fs::remove_file(result_path)?;
    Ok(())
}

fn run_search(problem: &Problem) -> Result<Value> {
    use symgcad::search::{SearchOptions, TrialOutcome, TrialStatus, search};
    let n = problem.variables.len();
    let groups: Vec<Vec<usize>> = if problem.variable_groups.is_empty() {
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
    let start = std::time::Instant::now();
    let mut options = SearchOptions::for_campaign(problem.limits.wall_time_secs);
    options.seed = problem.solver.seed;
    options.max_rollouts = problem.solver.max_rollouts;
    if let Some(seconds) = problem.solver.search_time_secs {
        options.wall_time_secs = seconds.min(options.wall_time_secs);
    }
    let default_order = groups.iter().flatten().copied().collect::<Vec<_>>();
    let mut seeds = vec![default_order.clone()];
    for method in ["brown", "sparsity"] {
        if start.elapsed().as_secs_f64() >= options.wall_time_secs * 0.2 {
            break;
        }
        let mut seed = problem.clone();
        seed.solver.order.clear();
        seed.solver.ordering = method.into();
        seed.limits.wall_time_secs = (options.wall_time_secs * 0.1).min(5.0).max(0.01);
        let (out, path) = bounded(&seed, "order")?;
        if out.status == "completed" {
            if let Ok(value) = serde_json::from_slice::<Value>(&fs::read(&path)?) {
                if let Some(order) = value["order"].as_array() {
                    let indices = order
                        .iter()
                        .map(|v| {
                            problem
                                .variables
                                .iter()
                                .position(|n| Some(n.as_str()) == v.as_str())
                                .context("unknown seed variable")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    if !seeds.contains(&indices) {
                        seeds.push(indices);
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
    options.wall_time_secs = (options.wall_time_secs - start.elapsed().as_secs_f64()).max(0.001);
    let report = search(&groups, &priority, &seeds, &options, |request| {
        let mut trial = problem.clone();
        trial.solver.order = request
            .order
            .iter()
            .map(|&i| problem.variables[i].clone())
            .collect();
        trial.solver.ordering = "given".into();
        trial.limits.wall_time_secs = request.timeout_secs;
        let (process, path) = bounded(&trial, "solve")?;
        let summary = serde_json::from_str::<Value>(&process.stdout).ok();
        let complete = summary
            .as_ref()
            .is_some_and(|v| v["status"] == "complete_generic");
        let status = match process.status.as_str() {
            "completed" if complete => TrialStatus::Completed,
            "timeout" => TrialStatus::Timeout,
            "memory_limit" => TrialStatus::MemoryLimit,
            _ => TrialStatus::Failed,
        };
        let cells = summary
            .as_ref()
            .and_then(|v| v["cells"].as_u64())
            .map(|n| n as usize);
        fs::remove_file(path)?;
        Ok(TrialOutcome {
            status,
            elapsed_secs: process.wall_time_secs,
            cells,
            progress_score: None,
            message: if complete { None } else { Some(process.stderr) },
        })
    })?;
    Ok(
        json!({"schema_version":1,"problem":problem,"search":report,"total_seconds":start.elapsed().as_secs_f64()}),
    )
}

fn worker(operation: &str, input: &Path, output: &Path) -> Result<()> {
    if operation == "verify" {
        let result: SolveResult = serde_json::from_slice(&fs::read(input)?)?;
        let verification = symgcad::verify::verify(&result)?;
        emit(&verification, Some(output))?;
        return emit(&verification, None);
    }
    let problem: Problem = serde_json::from_slice(&fs::read(input)?)?;
    match operation {
        "solve" => {
            let result = match solve(&problem) {
                Ok(result) => result,
                Err(e) => {
                    SolveResult::failure(problem, Status::ComputationFailure, format!("{e:#}"))
                }
            };
            emit(&result, Some(output))?;
            emit(
                &json!({"status":result.status,"cells":result.cells.len(),"merged_cells":result.merged_cells.as_ref().map(Vec::len),"statistics":result.statistics}),
                None,
            )?;
        }
        "project" => {
            let order = selected_order(&problem)?;
            let normalized = normalize(&problem, &order)?;
            let mut all = normalized.positives;
            all.extend(normalized.split);
            let projection = project(&normalized.algebra, &all)?;
            let (polynomials, levels) = projection_table(&normalized.algebra, &projection);
            emit(
                &json!({"schema_version":1,"order":order,"polynomials":polynomials,"levels":levels,"steps":projection.steps}),
                Some(output),
            )?;
        }
        "order" => {
            emit(
                &json!({"schema_version":1,"order":selected_order(&problem)?}),
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
            let mut problem = Problem::from_toml(&fs::read_to_string(input)?)?;
            if problem.solver.ordering == "mcts" {
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
                eprintln!("{}", serde_json::to_string(&report)?);
            }
            forward(problem, "solve", output)
        }
        Command::Project { input, output } => forward(
            Problem::from_toml(&fs::read_to_string(input)?)?,
            "project",
            output,
        ),
        Command::Order { input, output } => {
            let problem = Problem::from_toml(&fs::read_to_string(input)?)?;
            if problem.solver.ordering == "mcts" {
                emit(&run_search(&problem)?, output.as_deref())
            } else {
                forward(problem, "order", output)
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
            fs::remove_file(output)?;
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
                emit(&inventory, Some(&directory.join("inventory.json")))?;
                emit(
                    &json!({"collected_tests":inventory.collected_count,"disabled_tests":inventory.disabled_count,"fixtures":inventory.fixtures.len(),"exported":files.len(),"directory":directory}),
                    None,
                )
            } else {
                emit(&inventory, None)
            }
        }
    }
}
