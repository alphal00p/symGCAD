//! Bounded exact experiments for the six unresolved original core domains.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Problem, SolverOptions,
    benchmark::run_bounded,
    campaign::{CaseRun, CompletionReport, ScheduledCase, Trial},
    domain::{derive_domain, homogeneous_anchor},
    ordering::cheap_seed_orders,
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    family: String,
    #[arg(long)]
    executable: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "artifacts/completion-aggregate-296.json")]
    completion: PathBuf,
    #[arg(long, default_value_t = 20.0)]
    initial_secs: f64,
    #[arg(long, default_value_t = 120.0)]
    promotion_secs: f64,
    #[arg(long, default_value_t = 240.0)]
    verify_secs: f64,
    #[arg(long, default_value_t = 1200.0)]
    campaign_secs: f64,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
    /// Transfer an unsplit sign constraint and compare both endpoint engines.
    #[arg(long)]
    monotone: bool,
}
#[derive(Clone)]
struct Job {
    problem: Problem,
    source: PathBuf,
    aliases: Vec<String>,
    label: String,
}
#[derive(Deserialize)]
struct Summary {
    status: String,
    #[serde(default)]
    statistics: Option<Value>,
    #[serde(default)]
    diagnostic: Option<String>,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
fn write(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}
fn worker(binary: &Path, operation: &str, input: &Path, output: &Path) -> Command {
    let mut c = Command::new(binary);
    c.arg("_worker")
        .arg(operation)
        .arg(input)
        .arg("--output")
        .arg(output)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    c
}
fn jobs(args: &Args) -> Result<Vec<Job>> {
    let report: Value = serde_json::from_slice(&fs::read(&args.completion)?)?;
    let mut jobs = Vec::new();
    for group in report["unresolved"]
        .as_array()
        .context("unresolved groups missing")?
    {
        let name = group["identifier"].as_str().context("name missing")?;
        let selected = match args.family.as_str() {
            "gluza30" => name.starts_with("test_gluza_ex30/"),
            "gluza33" => name.starts_with("test_gluza_ex33/"),
            "symbolic" => {
                name.starts_with("test_massless_planar_elliptic_symb/")
                    || name.starts_with("test_elliptic2l_physical_symb/")
            }
            _ => anyhow::bail!("family must be gluza30, gluza33, or symbolic"),
        };
        if !selected {
            continue;
        }
        let source = PathBuf::from(group["original_input"].as_str().context("input missing")?)
            .canonicalize()?;
        ensure!(
            hash(&source)? == group["original_input_sha256"],
            "original input hash mismatch"
        );
        let original = Problem::from_toml(&fs::read_to_string(&source)?)?;
        let n = normalize(&original, &original.variables)?;
        let all = n
            .positives
            .iter()
            .chain(&n.split)
            .cloned()
            .collect::<Vec<_>>();
        let domain = derive_domain(&n.algebra, &n.positives, &[])?;
        let block = original
            .variables
            .iter()
            .enumerate()
            .filter(|(_, v)| v.starts_with('x'))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let proposed: &[&str] = match args.family.as_str() {
            "gluza30" => &[],
            "gluza33" => &["x0", "x1", "x6"],
            _ if name.contains("massless") => &["x1", "x5"],
            _ => &["x0", "x1"],
        };
        let mut anchors = Vec::new();
        if proposed.is_empty() {
            anchors.push(None);
        }
        for candidate in proposed {
            let index = original
                .variables
                .iter()
                .position(|v| v == candidate)
                .context("unknown anchor")?;
            if homogeneous_anchor(&n.algebra, &all, &block, index, &domain)?.is_some() {
                anchors.push(Some(index));
            }
        }
        ensure!(!anchors.is_empty(), "no certified anchors for {name}");
        for anchor in anchors {
            let reduced = all
                .iter()
                .map(|p| anchor.map_or_else(|| p.clone(), |v| p.replace(v, &Rational::one())))
                .filter(|p| !p.is_constant())
                .collect::<Vec<_>>();
            let mut groups = if original.variable_groups.is_empty() {
                vec![(0..original.variables.len()).collect::<Vec<_>>()]
            } else {
                original
                    .variable_groups
                    .iter()
                    .map(|g| {
                        g.iter()
                            .map(|v| original.variables.iter().position(|x| x == v).unwrap())
                            .collect()
                    })
                    .collect()
            };
            if let Some(a) = anchor {
                let at = groups.iter().position(|g| g.contains(&a)).unwrap();
                groups[at].retain(|&v| v != a);
                groups.insert(at, vec![a]);
                groups.retain(|g| !g.is_empty());
            }
            let orders = if original.solver.order.is_empty() {
                cheap_seed_orders(&reduced, &groups)
            } else {
                vec![
                    original
                        .solver
                        .order
                        .iter()
                        .map(|v| original.variables.iter().position(|x| x == v).unwrap())
                        .collect(),
                ]
            };
            for (seed, order) in orders
                .into_iter()
                .take(if args.monotone { 1 } else { 2 })
                .enumerate()
            {
                for engine in ["local", "optimized"] {
                    let mut p = original.clone();
                    let dynamic = args.monotone && original.solver.order.is_empty();
                    p.solver = SolverOptions {
                        engine: if args.monotone { "monotone" } else { engine }.into(),
                        ordering: if dynamic { "brown" } else { "given" }.into(),
                        order: if dynamic {
                            vec![]
                        } else {
                            order
                                .iter()
                                .map(|&i| original.variables[i].clone())
                                .collect()
                        },
                        positivity: true,
                        input_positivity: true,
                        prune_operations: engine == "optimized",
                        monotone_base_engine: if args.monotone { engine } else { "optimized" }
                            .into(),
                        ..Default::default()
                    };
                    if let Some(a) = anchor {
                        p.solver.homogeneous_block = block
                            .iter()
                            .map(|&i| original.variables[i].clone())
                            .collect();
                        p.solver.homogeneous_anchor = Some(original.variables[a].clone());
                    }
                    p.limits.workers = 1;
                    p.limits.memory_mib = args.memory_mib;
                    p.limits.wall_time_secs = args.initial_secs;
                    p.validate()?;
                    if !original.solver.order.is_empty() {
                        ensure!(
                            p.solver.order == original.solver.order,
                            "fixed order changed"
                        );
                    }
                    jobs.push(Job {
                        problem: p,
                        source: source.clone(),
                        aliases: serde_json::from_value(group["aliases"].clone())?,
                        label: format!(
                            "{}{}-{}-seed{}",
                            if args.monotone { "monotone-" } else { "" },
                            engine,
                            anchor.map_or("unscaled", |a| original.variables[a].as_str()),
                            seed
                        ),
                    });
                }
            }
        }
    }
    Ok(jobs)
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.campaign_secs > 0.0
            && args.campaign_secs <= 3600.0
            && args.initial_secs > 0.0
            && args.promotion_secs > 0.0
            && args.verify_secs > 0.0
            && args.memory_mib <= 8192,
        "invalid bounds"
    );
    ensure!(!args.output.exists(), "use a fresh output directory");
    let started = Instant::now();
    let jobs = jobs(&args)?;
    fs::create_dir_all(args.output.join("executables"))?;
    let output = args.output.canonicalize()?;
    let binary_hash = hash(&args.executable)?;
    let binary = output
        .join("executables")
        .join(format!("symgcad-{binary_hash}"));
    fs::copy(&args.executable, &binary)?;
    ensure!(
        hash(&binary)? == binary_hash,
        "worker changed during snapshot"
    );
    let planned=jobs.iter().map(|j|json!({"name":j.problem.name,"variant":j.label,"order":j.problem.solver.order,"anchor":j.problem.solver.homogeneous_anchor,"source":j.source})).collect::<Vec<_>>();
    write(
        &output.join("plan.json"),
        &json!({"family":args.family,"jobs":planned,"initial_secs":args.initial_secs,"promotion_secs":args.promotion_secs,"campaign_secs":args.campaign_secs,"input_audit":args.completion,"input_audit_sha256":hash(&args.completion)?}),
    )?;
    let mut report = CompletionReport {
        schema_version: 1,
        mode: "completion".into(),
        provenance: json!({"executable":args.executable.canonicalize()?,"frozen_executable":binary,"executable_sha256":binary_hash,"operation":"direct solve worker preserving original fixed orders and variable groups","monotone":args.monotone,"workers":1,"memory_mib":args.memory_mib,"initial_secs":args.initial_secs,"promotion_secs":args.promotion_secs,"verify_timeout_secs":args.verify_secs,"campaign_secs":args.campaign_secs,"harness_sha256":hash(&std::env::current_exe()?)?}),
        scheduled: vec![],
        runs: vec![],
        complete_verified: 0,
        resumed: 0,
        campaign_exhausted: false,
        elapsed_secs: 0.0,
    };
    let mut solved = BTreeSet::new();
    let mut promotions = Vec::<(usize, f64, Job)>::new();
    let mut queue = jobs
        .into_iter()
        .map(|j| (j, args.initial_secs, false))
        .collect::<std::collections::VecDeque<_>>();
    let mut promoted = false;
    loop {
        if queue.is_empty() && !promoted {
            promoted = true;
            promotions.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.total_cmp(&b.1)));
            let mut picked = BTreeSet::new();
            for (_, _, j) in promotions.drain(..) {
                let key = (
                    j.problem.name.clone(),
                    j.problem.solver.engine.clone(),
                    j.problem.solver.monotone_base_engine.clone(),
                );
                if !solved.contains(j.problem.name.as_ref().unwrap()) && picked.insert(key) {
                    queue.push_back((j, args.promotion_secs, true));
                }
            }
        }
        let Some((job, seconds, promotion)) = queue.pop_front() else {
            break;
        };
        let name = job.problem.name.clone().unwrap();
        if solved.contains(&name) {
            continue;
        }
        let remaining = args.campaign_secs - started.elapsed().as_secs_f64();
        if remaining <= 0.05 {
            report.campaign_exhausted = true;
            break;
        }
        let index = report.runs.len();
        let directory = output.join(format!(
            "{index:04}-{}{}",
            job.label,
            if promotion { "-promotion" } else { "" }
        ));
        fs::create_dir(&directory)?;
        let mut problem = job.problem.clone();
        problem.limits.wall_time_secs = seconds.min(remaining);
        let input_file = directory.join("input.toml");
        fs::write(&input_file, toml::to_string_pretty(&problem)?)?;
        let wire = directory.join("input.json");
        write(&wire, &problem)?;
        let input_hash = hash(&input_file)?;
        let identity = digest(format!("{input_hash}:{binary_hash}:{index}").as_bytes());
        let result_file = directory.join("result.json");
        let progress_file = directory.join("progress.json");
        let verification_file = directory.join("verification.json");
        let mut command = worker(&binary, "solve", &wire, &result_file);
        command.env("SYMGCAD_PROGRESS_PATH", &progress_file);
        let solve = run_bounded(&mut command, &problem.limits)?;
        let summary = fs::File::open(&result_file)
            .ok()
            .and_then(|f| serde_json::from_reader::<_, Summary>(BufReader::new(f)).ok());
        let complete = solve.status == "completed"
            && solve.exit_code == Some(0)
            && summary
                .as_ref()
                .is_some_and(|s| s.status == "complete_generic");
        let remaining = args.campaign_secs - started.elapsed().as_secs_f64();
        let verification = if complete && remaining > 0.05 {
            let mut limits = problem.limits.clone();
            limits.wall_time_secs = args.verify_secs.min(remaining);
            Some(run_bounded(
                &mut worker(&binary, "verify", &result_file, &verification_file),
                &limits,
            )?)
        } else {
            None
        };
        let verified = verification
            .as_ref()
            .is_some_and(|v| v.status == "completed" && v.exit_code == Some(0))
            && fs::read(&verification_file)
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .is_some_and(|v| v["verified"] == true);
        let status = if verified {
            "complete_verified".to_owned()
        } else if complete {
            "verification_failed".into()
        } else if solve.status != "completed" {
            solve.status.clone()
        } else {
            summary
                .as_ref()
                .map_or("invalid_result".into(), |s| s.status.clone())
        };
        let progress = fs::read(&progress_file)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        let levels = progress
            .as_ref()
            .and_then(|v| v["checkpoints"].as_array())
            .map_or_else(
                || {
                    progress
                        .as_ref()
                        .and_then(|v| v["current_level"].as_u64())
                        .unwrap_or(0) as usize
                },
                Vec::len,
            );
        let terms = progress
            .as_ref()
            .and_then(|v| v["checkpoints"].as_array())
            .and_then(|v| v.last())
            .and_then(|v| v["term_count"].as_f64())
            .unwrap_or(0.0);
        write(
            &directory.join("diagnostic.json"),
            &json!({"name":name,"variant":job.label,"promotion":promotion,"status":status,"levels":levels,"last_terms":terms,"progress_file":progress_file,"diagnostic":summary.as_ref().and_then(|s|s.diagnostic.as_ref())}),
        )?;
        if verified {
            solved.insert(name.clone());
        } else if !promotion
            && (solve.status == "timeout"
                || summary.as_ref().is_some_and(|s| s.status == "incomplete"))
        {
            promotions.push((levels, terms, job.clone()));
        }
        let trial = Trial {
            repetition: 0,
            input_file: Some(input_file.clone()),
            input_sha256: Some(input_hash.clone()),
            status: status.clone(),
            solve: solve.clone(),
            result_file: result_file.clone(),
            result_sha256: hash(&result_file).ok(),
            result_status: summary.as_ref().map(|s| s.status.clone()),
            statistics: summary.and_then(|s| s.statistics),
            verification: verification.clone(),
            verification_file: verification_file.clone(),
            verification_sha256: hash(&verification_file).ok(),
            verified,
        };
        report.runs.push(CaseRun {
            identity_sha256: identity.clone(),
            input_sha256: input_hash,
            executable_sha256: binary_hash.clone(),
            name: name.clone(),
            aliases: job.aliases.clone(),
            equivalence_sha256: None,
            source: job.source.clone(),
            input_file,
            variant: format!("{}{}", job.label, if promotion { "-promotion" } else { "" }),
            mode: "completion".into(),
            requested_repeats: 1,
            status: status.clone(),
            solve_median_secs: verified.then_some(solve.wall_time_secs),
            peak_rss_kib: solve
                .peak_rss_kib
                .max(verification.as_ref().map_or(0, |v| v.peak_rss_kib)),
            trials: vec![trial],
        });
        report.scheduled.push(ScheduledCase {
            identity_sha256: identity,
            name: name.clone(),
            aliases: job.aliases,
            variant: job.label.clone(),
            status: status.clone(),
            resumed: false,
            run_index: Some(index),
        });
        report.complete_verified = solved.len();
        report.elapsed_secs = started.elapsed().as_secs_f64();
        write(&output.join("ledger.json"), &report)?;
        println!(
            "{}",
            json!({"index":index,"name":name,"variant":job.label,"promotion":promotion,"status":status,"levels":levels,"terms":terms,"elapsed":report.elapsed_secs})
        );
    }
    report.elapsed_secs = started.elapsed().as_secs_f64();
    write(&output.join("ledger.json"), &report)?;
    Ok(())
}
