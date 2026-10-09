//! Freeze native reference completions before measuring a chosen Rust worker.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use symgcad::{
    Limits, Problem,
    benchmark::{CampaignConfig, run_reference},
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    output: PathBuf,
}
fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.into()
    } else {
        base.join(path)
    }
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    let config_text = fs::read_to_string(&args.config)?;
    let config: CampaignConfig = toml::from_str(&config_text)?;
    ensure!(!config.cases.is_empty(), "no qualification cases");
    ensure!(
        config.campaign_time_secs.is_finite()
            && config.campaign_time_secs > 0.0
            && config.campaign_time_secs <= 3600.0,
        "invalid campaign budget"
    );
    ensure!(
        config.reference_timeout_secs.is_finite() && config.reference_timeout_secs > 0.0,
        "invalid qualification timeout"
    );
    ensure!(
        config.workers > 0
            && config.workers <= 16
            && config.memory_mib > 0
            && config.memory_mib <= 65536,
        "invalid resource allocation"
    );
    fs::create_dir_all(&args.output)?;
    ensure!(
        fs::read_dir(&args.output)?.next().is_none(),
        "qualification output must be empty"
    );
    let config_path = args.config.canonicalize()?;
    let base = config_path.parent().unwrap();
    let reference_root = resolve(base, &config.reference_root);
    let adapter = resolve(base, &config.adapter);
    let metadata: Value = serde_json::from_reader(fs::File::open(
        reference_root.parent().unwrap().join("provenance.json"),
    )?)?;
    let python = config
        .reference_python
        .as_ref()
        .map(|p| {
            if p.components().count() == 1 {
                p.clone()
            } else {
                resolve(base, p)
            }
        })
        .unwrap_or(PathBuf::from(
            metadata["python_executable"]
                .as_str()
                .context("reference interpreter missing")?,
        ));
    let provenance = json!({"config":config_path,"config_sha256":format!("{:x}",Sha256::digest(config_text.as_bytes())),"harness_sha256":format!("{:x}",Sha256::digest(fs::read("/proc/self/exe")?)),"adapter_sha256":format!("{:x}",Sha256::digest(fs::read(&adapter)?)),"reference_revision":metadata["revision"],"workers":config.workers,"memory_mib":config.memory_mib,"reference_timeout_secs":config.reference_timeout_secs,"supervisor":"recursive thread-child tree, wait4 CPU and RSS; no host process scan"});
    let mut problems = Vec::new();
    for path in &config.cases {
        let input = Problem::from_toml(&fs::read_to_string(resolve(base, path))?)?;
        ensure!(input.split.len() <= 12, "too many split signs");
        ensure!(
            input.solver.engine == "baseline"
                && !input.solver.positivity
                && !input.solver.input_positivity
                && input.solver.homogeneous_block.is_empty()
                && input.solver.homogeneous_anchor.is_none(),
            "baseline qualification requires original unreduced inputs"
        );
        for mask in 0..1usize << input.split.len() {
            let mut problem = input.clone();
            let signs: String = input
                .split
                .iter()
                .enumerate()
                .map(|(i, split)| {
                    let positive = mask & (1 << i) != 0;
                    problem
                        .constraints
                        .push(format!("({split}) {} 0", if positive { ">" } else { "<" }));
                    if positive { '+' } else { '-' }
                })
                .collect();
            problem.split.clear();
            let name = input.name.as_deref().unwrap_or("unnamed");
            problem.name = Some(if signs.is_empty() {
                name.into()
            } else {
                format!("{name}/signs={signs}")
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
    let mut rows = Vec::new();
    for (i, problem) in problems.iter_mut().enumerate() {
        let remaining = (config.campaign_time_secs - started.elapsed().as_secs_f64()).max(0.0);
        let mut row = json!({"name":problem.name,"reference_qualified":false,"frozen_order":[],"status":"campaign_exhausted","reference_peak_rss_kib":0,"rust_peak_rss_kib":0});
        if remaining > 0.05 {
            problem.limits.wall_time_secs = config.reference_timeout_secs.min(remaining);
            let record = run_reference(problem, &python, &reference_root, &adapter)?;
            write(
                args.output.join(format!("{i:04}-qualification.json")),
                &record,
            )?;
            row["reference_peak_rss_kib"] = json!(record.process.peak_rss_kib);
            if let Some(result) = record.result.filter(|r| r["status"] == "complete_generic") {
                problem.solver.order = serde_json::from_value(result["order"].clone())?;
                problem.solver.ordering = "given".into();
                row["frozen_order"] = json!(problem.solver.order);
                row["reference_qualified"] = json!(true);
                row["status"] = json!("qualified");
            } else {
                row["status"] = json!(format!("reference_{}", record.process.status));
            }
        }
        rows.push(row);
        write(args.output.join("qualification-progress.json"), &rows)?;
    }
    // Restrict only the worker wire schema to M1 fields, keeping all mathematical
    // input data. New workers accept this schema too. No reductions are enabled.
    let wire: Vec<_> = problems
        .iter()
        .map(|p| {
            let mut raw = serde_json::to_value(p).unwrap();
            raw["solver"].as_object_mut().unwrap().retain(|key, _| {
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
            raw
        })
        .collect();
    let elapsed = started.elapsed().as_secs_f64();
    let remaining = (config.campaign_time_secs - elapsed).max(0.0);
    write(
        args.output.join("frozen-reference-set.json"),
        &json!({"schema_version":1,"cases":rows,"problems":wire,"provenance":provenance,"qualification_elapsed_secs":elapsed,"remaining_campaign_secs":remaining}),
    )?;
    println!(
        "{}",
        json!({"cases":rows.len(),"reference_complete":rows.iter().filter(|r|r["reference_qualified"]==true).count(),"qualification_elapsed_secs":elapsed,"remaining_campaign_secs":remaining})
    );
    Ok(())
}
