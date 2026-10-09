//! Resource-bounded exploratory A446 anchor/order campaigns.
//! This is a development experiment, not a release parity benchmark.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    benchmark::{ProcessOutcome, run_bounded},
    ordering::cheap_seed_orders,
    output::{SolveResult, Status},
    problem::normalize,
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        long,
        default_value = "artifacts/corpus/fixed/0093_finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1_fixed.toml"
    )]
    fixture: PathBuf,
    #[arg(long, default_value = "target/debug/symgcad")]
    executable: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value_t = 600.0)]
    campaign_secs: f64,
    #[arg(long, default_value_t = 10.0)]
    initial_secs: f64,
    #[arg(long, default_value_t = 60.0)]
    promotion_secs: f64,
    #[arg(long, default_value_t = 6)]
    promotions: usize,
    #[arg(long, default_value_t = 3)]
    parallel: usize,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
    #[arg(long, value_delimiter = ',')]
    block: Vec<String>,
    #[arg(long)]
    anchor: Option<String>,
    #[arg(long, value_delimiter = ',')]
    order: Vec<String>,
    #[arg(long, default_value = "optimized")]
    engine: String,
    /// Same-build ablation of bounded sibling discovery in the local engine.
    #[arg(long)]
    no_batch_siblings: bool,
    /// Same-build ablation of completed conditional-subtree reuse.
    #[arg(long)]
    no_reuse_subtrees: bool,
    /// Same-build ablation of a second exact pair cancellation.
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=2))]
    pair_cancellations: u8,
    #[arg(long)]
    checkpoint_dir: Option<PathBuf>,
    #[arg(long)]
    inspect_progress: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Trial {
    id: String,
    anchor: String,
    order: Vec<String>,
    seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Record {
    trial: Trial,
    process: ProcessOutcome,
    result_status: Option<Status>,
    diagnostic: Option<String>,
    cells: Option<usize>,
    statistics: Option<Value>,
    progress: Option<Value>,
    verification: Option<ProcessOutcome>,
    verified: bool,
}

struct Campaign {
    problem: Problem,
    executable: PathBuf,
    output: PathBuf,
    checkpoint_dir: PathBuf,
    deadline: Instant,
    memory_mib: u64,
    block: Vec<String>,
    engine: String,
    batch_siblings: bool,
    reuse_subtrees: bool,
    pair_cancellations: usize,
}

fn available_cpus() -> Result<Vec<usize>> {
    let mut mask = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    ensure!(
        unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&mask), &mut mask) } == 0,
        "cannot read CPU affinity"
    );
    Ok((0..libc::CPU_SETSIZE as usize)
        .filter(|&cpu| unsafe { libc::CPU_ISSET(cpu, &mask) })
        .collect())
}

fn pin_current_thread(cpu: usize) -> Result<()> {
    let mut mask = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_SET(cpu, &mut mask);
    }
    ensure!(
        unsafe { libc::sched_setaffinity(0, std::mem::size_of_val(&mask), &mask) } == 0,
        "cannot pin supervisor thread"
    );
    Ok(())
}

fn remaining(campaign: &Campaign) -> f64 {
    campaign
        .deadline
        .saturating_duration_since(Instant::now())
        .as_secs_f64()
}

fn worker_command(campaign: &Campaign, operation: &str, input: &Path, output: &Path) -> Command {
    let mut command = Command::new(&campaign.executable);
    command
        .arg("_worker")
        .arg(operation)
        .arg(input)
        .arg("--output")
        .arg(output)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    command
}

fn run_trial(campaign: &Campaign, trial: Trial) -> Result<Record> {
    let directory = campaign.output.join(&trial.id);
    fs::create_dir_all(&directory)?;
    let input = directory.join("input.json");
    let output = directory.join("result.json");
    let progress_path = directory.join("progress.json");
    let mut problem = campaign.problem.clone();
    problem.solver.engine = campaign.engine.clone();
    problem.solver.local_batch_siblings = campaign.batch_siblings;
    problem.solver.local_reuse_subtrees = campaign.reuse_subtrees;
    problem.solver.local_pair_cancellations = campaign.pair_cancellations;
    problem.solver.positivity = true;
    problem.solver.homogeneous_block = campaign.block.clone();
    problem.solver.homogeneous_anchor = Some(trial.anchor.clone());
    problem.solver.ordering = "given".into();
    problem.solver.order = trial.order.clone();
    problem.solver.merge = "none".into();
    problem.solver.checkpoint_dir = Some(campaign.checkpoint_dir.clone());
    problem.limits.workers = 1;
    problem.limits.memory_mib = campaign.memory_mib;
    problem.limits.wall_time_secs = trial.seconds.min(remaining(campaign)).max(0.001);
    problem.validate()?;
    fs::write(&input, serde_json::to_vec_pretty(&problem)?)?;
    let mut command = worker_command(campaign, "solve", &input, &output);
    command.env("SYMGCAD_PROGRESS_PATH", &progress_path);
    let process = run_bounded(&mut command, &problem.limits)?;
    let result: Option<SolveResult> = fs::read(&output)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let progress = fs::read(&progress_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let complete = process.status == "completed"
        && result
            .as_ref()
            .is_some_and(|r| r.status == Status::CompleteGeneric);
    let verification = if complete && remaining(campaign) > 0.01 {
        let mut limits = problem.limits.clone();
        limits.wall_time_secs = 60.0_f64.min(remaining(campaign));
        let mut command = worker_command(
            campaign,
            "verify",
            &output,
            &directory.join("verification.json"),
        );
        Some(run_bounded(&mut command, &limits)?)
    } else {
        None
    };
    let verified = verification.as_ref().is_some_and(|v| {
        v.status == "completed"
            && fs::read(directory.join("verification.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .is_some_and(|j| j["verified"] == true)
    });
    let record = Record {
        trial,
        result_status: result.as_ref().map(|r| r.status.clone()),
        diagnostic: result.as_ref().and_then(|r| r.diagnostic.clone()),
        cells: result.as_ref().map(|r| r.cells.len()),
        statistics: result
            .as_ref()
            .map(|r| serde_json::to_value(&r.statistics))
            .transpose()?,
        process,
        progress,
        verification,
        verified,
    };
    fs::write(
        directory.join("record.json"),
        serde_json::to_vec_pretty(&record)?,
    )?;
    Ok(record)
}

fn completed_levels(record: &Record) -> usize {
    record
        .progress
        .as_ref()
        .and_then(|p| p["checkpoints"].as_array())
        .map_or(0, Vec::len)
}

fn last_terms(record: &Record) -> u64 {
    record
        .progress
        .as_ref()
        .and_then(|p| p["checkpoints"].as_array())
        .and_then(|c| c.last())
        .and_then(|c| c["term_count"].as_u64())
        .unwrap_or(u64::MAX)
}

fn run_round(campaign: &Campaign, trials: Vec<Trial>, cpus: &[usize]) -> Result<Vec<Record>> {
    let pending = Arc::new(Mutex::new(VecDeque::from(trials)));
    let records = Arc::new(Mutex::new(Vec::new()));
    std::thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for &cpu in cpus {
            let pending = Arc::clone(&pending);
            let records = Arc::clone(&records);
            handles.push(scope.spawn(move || -> Result<()> {
                pin_current_thread(cpu)?;
                loop {
                    if remaining(campaign) <= 0.01 {
                        break;
                    }
                    let Some(trial) = pending.lock().unwrap().pop_front() else {
                        break;
                    };
                    let record = run_trial(campaign, trial)?;
                    let mut records = records.lock().unwrap();
                    println!(
                        "{}: {} {:?}, checkpoints={}, last_terms={}, verified={}",
                        record.trial.id,
                        record.process.status,
                        record.result_status,
                        completed_levels(&record),
                        last_terms(&record),
                        record.verified
                    );
                    let mut journal = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(campaign.output.join("journal.jsonl"))?;
                    writeln!(journal, "{}", serde_json::to_string(&record)?)?;
                    records.push(record);
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("probe supervisor panicked"))??;
        }
        Ok(())
    })?;
    let mut records = records.lock().unwrap().clone();
    records.sort_by(|a, b| a.trial.id.cmp(&b.trial.id));
    Ok(records)
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(progress) = &args.inspect_progress {
        return inspect_prefix(&args, progress);
    }
    ensure!(
        args.parallel > 0 && args.parallel <= 4,
        "parallel must be in 1..=4"
    );
    ensure!(
        args.campaign_secs.is_finite() && args.campaign_secs > 0.0 && args.campaign_secs <= 3600.0,
        "campaign duration must be in (0,3600]"
    );
    ensure!(
        args.initial_secs.is_finite()
            && args.initial_secs > 0.0
            && args.promotion_secs.is_finite()
            && args.promotion_secs > 0.0,
        "trial durations must be positive"
    );
    ensure!(
        args.memory_mib > 0 && args.memory_mib.saturating_mul(args.parallel as u64) <= 65536,
        "aggregate worker memory exceeds 64 GiB"
    );
    let started = Instant::now();
    let fixture = fs::read_to_string(&args.fixture)?;
    let problem = Problem::from_toml(&fixture)?;
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "artifacts/hard-campaigns/a446-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ))
    });
    fs::create_dir_all(&output)?;
    let output = output.canonicalize()?;
    let executable = args
        .executable
        .canonicalize()
        .context("find symgcad worker binary")?;
    let allowed = available_cpus()?;
    let cpus = allowed
        .iter()
        .copied()
        .skip(usize::from(allowed.len() > args.parallel))
        .take(args.parallel)
        .collect::<Vec<_>>();
    let campaign = Campaign {
        checkpoint_dir: args
            .checkpoint_dir
            .clone()
            .unwrap_or_else(|| output.join("checkpoints")),
        block: if args.block.is_empty() {
            problem.variables.clone()
        } else {
            args.block.clone()
        },
        engine: args.engine.clone(),
        batch_siblings: !args.no_batch_siblings,
        reuse_subtrees: !args.no_reuse_subtrees,
        pair_cancellations: args.pair_cancellations as usize,
        problem,
        executable,
        output,
        deadline: started + std::time::Duration::from_secs_f64(args.campaign_secs),
        memory_mib: args.memory_mib,
    };
    let normalized = normalize(&campaign.problem, &campaign.problem.variables)?;
    let mut anchors = (0..campaign.problem.variables.len())
        .filter(|&v| {
            campaign.block.contains(&campaign.problem.variables[v])
                && args
                    .anchor
                    .as_ref()
                    .is_none_or(|a| a == &campaign.problem.variables[v])
        })
        .collect::<Vec<_>>();
    ensure!(
        !anchors.is_empty(),
        "no requested anchor belongs to the explicit block"
    );
    anchors.sort_by_key(|&v| {
        (
            normalized
                .split
                .iter()
                .map(|p| p.degree(v))
                .max()
                .unwrap_or(0),
            v,
        )
    });
    let mut trials = Vec::new();
    for anchor in anchors {
        let mut reduced = normalized
            .positives
            .iter()
            .chain(&normalized.split)
            .map(|p| p.replace(anchor, &Rational::one()))
            .collect::<Vec<_>>();
        reduced.retain(|p| !p.is_constant());
        let mut groups = if campaign.problem.variable_groups.is_empty() {
            vec![(0..campaign.problem.variables.len()).collect::<Vec<_>>()]
        } else {
            campaign
                .problem
                .variable_groups
                .iter()
                .map(|g| {
                    g.iter()
                        .map(|v| {
                            campaign
                                .problem
                                .variables
                                .iter()
                                .position(|n| n == v)
                                .unwrap()
                        })
                        .collect()
                })
                .collect()
        };
        let anchor_group = groups.iter().position(|g| g.contains(&anchor)).unwrap();
        groups[anchor_group].retain(|&v| v != anchor);
        groups.insert(anchor_group, vec![anchor]);
        groups.retain(|g| !g.is_empty());
        let seeds = if args.order.is_empty() {
            cheap_seed_orders(&reduced, &groups)
        } else {
            vec![
                args.order
                    .iter()
                    .map(|n| {
                        campaign
                            .problem
                            .variables
                            .iter()
                            .position(|v| v == n)
                            .context("unknown explicit order variable")
                    })
                    .collect::<Result<Vec<_>>>()?,
            ]
        };
        for (seed, indices) in seeds.into_iter().enumerate() {
            let name = campaign.problem.variables[anchor].clone();
            trials.push(Trial {
                id: format!("initial-{name}-{seed}"),
                anchor: name,
                order: indices
                    .iter()
                    .map(|&i| campaign.problem.variables[i].clone())
                    .collect(),
                seconds: args.initial_secs,
            });
        }
    }
    let provenance = json!({
        "kind":"exploratory_debug_no_parity_claim", "fixture":args.fixture,"engine":args.engine,
        "fixture_sha256":format!("{:x}",Sha256::digest(fixture.as_bytes())),
        "executable":campaign.executable,
        "executable_sha256":format!("{:x}",Sha256::digest(fs::read(&campaign.executable)?)),
        "cpus":cpus,"worker_memory_mib":args.memory_mib,"campaign_secs":args.campaign_secs,
        "checkpoint_dir":campaign.checkpoint_dir,
        "initial_trials":trials,
    });
    fs::write(
        campaign.output.join("provenance.json"),
        serde_json::to_vec_pretty(&provenance)?,
    )?;
    println!(
        "{} initial trials; output={}",
        trials.len(),
        campaign.output.display()
    );
    let mut records = run_round(&campaign, trials, &cpus)?;
    let mut candidates = records
        .iter()
        .filter(|r| r.process.status == "timeout" && !r.verified)
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        completed_levels(b)
            .cmp(&completed_levels(a))
            .then_with(|| last_terms(a).cmp(&last_terms(b)))
            .then_with(|| a.trial.id.cmp(&b.trial.id))
    });
    let promotions = candidates
        .into_iter()
        .take(args.promotions)
        .map(|r| Trial {
            id: format!("promoted-{}", r.trial.id),
            seconds: args.promotion_secs,
            ..r.trial
        })
        .collect();
    if remaining(&campaign) > 0.01 {
        records.extend(run_round(&campaign, promotions, &cpus)?);
    }
    let summary = json!({"provenance":provenance,"elapsed_secs":started.elapsed().as_secs_f64(),
        "verified_complete":records.iter().filter(|r|r.verified).count(),"records":records});
    fs::write(
        campaign.output.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!(
        "campaign finished: {:.3}s, {} verified complete",
        started.elapsed().as_secs_f64(),
        summary["verified_complete"]
    );
    Ok(())
}

/// Heuristic diagnostics only: cached factors are never adopted as a proof.
fn inspect_prefix(args: &Args, progress_path: &Path) -> Result<()> {
    use std::collections::HashSet;
    use symgcad::{
        algebra::Algebra,
        domain::{CertificateOptions, certify_nonzero, derive_domain},
        ordering::{OrderingMethod, rank_candidates},
    };
    let problem = Problem::from_toml(&fs::read_to_string(&args.fixture)?)?;
    let progress: Value = serde_json::from_slice(&fs::read(progress_path)?)?;
    let checkpoints = progress["checkpoints"]
        .as_array()
        .context("missing checkpoints")?;
    let directory = args
        .checkpoint_dir
        .as_ref()
        .context("prefix inspection needs --checkpoint-dir")?;
    let read = |checkpoint: &Value| -> Result<Value> {
        let key = checkpoint["key"]
            .as_str()
            .context("missing checkpoint key")?;
        Ok(serde_json::from_slice(&fs::read(
            directory.join(format!("{key}.json")),
        )?)?)
    };
    let initial = read(checkpoints.first().context("empty progress")?)?;
    let last = read(checkpoints.last().unwrap())?;
    let names: Vec<String> = serde_json::from_value(last["variable_names"].clone())?;
    let algebra = Algebra::new(&names)?;
    let parsed = |value: &Value| -> Result<Vec<symgcad::algebra::Poly>> {
        value["output"]
            .as_array()
            .context("missing cached output")?
            .iter()
            .map(|v| algebra.parse(v.as_str().context("cached polynomial must be text")?))
            .collect()
    };
    let protected = parsed(&initial)?.into_iter().collect::<HashSet<_>>();
    let raw = parsed(&last)?;
    let original = normalize(&problem, &problem.variables)?;
    let anchor = args
        .anchor
        .as_ref()
        .context("prefix inspection needs an anchor")?;
    let index = problem
        .variables
        .iter()
        .position(|v| v == anchor)
        .context("unknown anchor")?;
    let constraints = original
        .positives
        .iter()
        .map(|p| algebra.parse(&original.algebra.format(&p.replace(index, &Rational::one()))))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&algebra, &constraints, &[])?;
    let mut retained = Vec::new();
    let mut removed = Vec::new();
    for p in &raw {
        if !protected.contains(p)
            && let Some(certificate) =
                certify_nonzero(&algebra, p, &domain, &CertificateOptions::default())?
        {
            removed.push(json!({"terms":p.nterms(),"method":certificate.method}));
        } else {
            retained.push(p.clone());
        }
    }
    let eliminated: HashSet<usize> = checkpoints
        .iter()
        .filter_map(|c| c["eliminated_variable"].as_u64().map(|v| v as usize))
        .collect();
    let candidates = (0..names.len())
        .filter(|v| !eliminated.contains(v) && args.block.contains(&names[*v]))
        .collect::<Vec<_>>();
    let mut features = Vec::new();
    for &v in &candidates {
        let active = retained
            .iter()
            .filter(|p| p.degree(v) > 0)
            .collect::<Vec<_>>();
        let degrees = active.iter().map(|p| p.degree(v)).collect::<Vec<_>>();
        let mut max_sylvester_size = 0;
        let mut max_resultant_total_degree_bound = 0u64;
        for (i, a) in active.iter().enumerate() {
            for b in &active[i + 1..] {
                max_sylvester_size = max_sylvester_size.max(a.degree(v) + b.degree(v));
                let bound = (0..names.len())
                    .filter(|&w| w != v)
                    .map(|w| {
                        u64::from(a.degree(v)) * u64::from(b.degree(w))
                            + u64::from(b.degree(v)) * u64::from(a.degree(w))
                    })
                    .sum();
                max_resultant_total_degree_bound = max_resultant_total_degree_bound.max(bound);
            }
        }
        features.push(json!({"variable":names[v],"active_polynomials":active.len(),"active_terms":active.iter().map(|p|p.nterms()).sum::<usize>(),
            "degrees":degrees,"mods_log2":degrees.iter().map(|&d|f64::from(d).log2()).sum::<f64>(),
            "max_sylvester_size":max_sylvester_size,"max_resultant_total_degree_bound":max_resultant_total_degree_bound}));
    }
    let rankings = [OrderingMethod::Mods,OrderingMethod::T1,OrderingMethod::Brown,OrderingMethod::Sparsity].into_iter().map(|method| {
        json!({"method":format!("{method:?}"),"elimination_priority":rank_candidates(&retained,&candidates,method).iter().map(|&v|names[v].clone()).collect::<Vec<_>>()})
    }).collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"diagnostic_only":true,"checkpoint":checkpoints.last(),"variables":names,
        "raw_polynomials":raw.len(),"raw_terms":raw.iter().map(|p|p.nterms()).sum::<usize>(),"retained_polynomials":retained.len(),
        "retained_terms":retained.iter().map(|p|p.nterms()).sum::<usize>(),"removed":removed,"features":features,"rankings":rankings})
        )?
    );
    Ok(())
}
