//! Preserve full split problems while borrowing successful baseline orders.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};
use symgcad::{Problem, SolverOptions, campaign::CampaignConfig};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    completion: PathBuf,
    #[arg(long, default_value = "artifacts/bench/parity-aggregate-9025.json")]
    parity: PathBuf,
    #[arg(long)]
    output: PathBuf,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn main() -> Result<()> {
    let args = Args::parse();
    let completion_bytes = fs::read(&args.completion)?;
    let completion: Value = serde_json::from_slice(&completion_bytes)?;
    let parity_bytes = fs::read(&args.parity)?;
    let parity: Value = serde_json::from_slice(&parity_bytes)?;
    ensure!(
        parity["summary"]["qualification_coverage_complete"] == true,
        "parity coverage is incomplete"
    );
    let mut solved = BTreeMap::<String, Value>::new();
    for source in parity["reports"].as_array().context("missing reports")? {
        let path = source["path"].as_str().context("missing report path")?;
        let bytes = fs::read(path)?;
        ensure!(
            digest(&bytes) == source["sha256"],
            "parity report hash mismatch"
        );
        let report: Value = serde_json::from_slice(&bytes)?;
        for case in report["cases"].as_array().context("missing cases")? {
            if case["status"] == "complete"
                && case["verified"] == true
                && case["raw_cells_match"] == true
            {
                let mut record = case.clone();
                record["report"] = json!(path);
                record["report_sha256"] = source["sha256"].clone();
                solved.insert(
                    case["name"].as_str().context("case name missing")?.into(),
                    record,
                );
            }
        }
    }
    ensure!(
        !args.output.exists(),
        "retry output already exists; retain previous attempts"
    );
    fs::create_dir_all(args.output.join("inputs"))?;
    let output = args.output.canonicalize()?;
    let mut selected = Vec::new();
    let mut skipped = Vec::new();
    for group in completion["unresolved"]
        .as_array()
        .context("unresolved groups missing")?
    {
        let original_path = group["original_input"]
            .as_str()
            .context("original input missing")?;
        let bytes = fs::read(original_path)?;
        ensure!(
            digest(&bytes) == group["original_input_sha256"],
            "original input hash mismatch"
        );
        let mut problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
        let aliases: Vec<String> = serde_json::from_value(group["aliases"].clone())?;
        let evidence = if problem.split.is_empty() {
            aliases
                .iter()
                .find_map(|name| solved.get(name).map(|case| vec![case.clone()]))
        } else if problem.split.len() == 1 {
            aliases.iter().find_map(|name| {
                Some(vec![
                    solved.get(&format!("{name}/signs=-"))?.clone(),
                    solved.get(&format!("{name}/signs=+"))?.clone(),
                ])
            })
        } else {
            None
        };
        let Some(evidence) = evidence else {
            skipped.push(json!({"identifier":group["identifier"],"reason":"no verified baseline full unsplit case, or no verified pair for the original split polynomial"}));
            continue;
        };
        let order: Vec<String> = serde_json::from_value(evidence[0]["frozen_order"].clone())?;
        if !problem.solver.order.is_empty() && problem.solver.order != order {
            skipped.push(json!({"identifier":group["identifier"],"reason":"baseline order differs from original explicit fixed order"}));
            continue;
        }
        problem.solver = SolverOptions {
            order,
            ..Default::default()
        };
        problem.limits.wall_time_secs = 120.0;
        problem.limits.memory_mib = 8192;
        problem.limits.workers = 1;
        problem.validate()?;
        // Baseline-only wire fields keep immutable older workers readable.
        let mut wire = toml::Value::try_from(&problem)?;
        wire.get_mut("solver")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .retain(|key, _| {
                [
                    "engine",
                    "ordering",
                    "order",
                    "merge",
                    "seed",
                    "max_rollouts",
                    "search_time_secs",
                ]
                .contains(&key)
            });
        let content = toml::to_string_pretty(&wire)?;
        let cost = evidence
            .iter()
            .map(|case| {
                case["rust_solver_median_secs"]
                    .as_f64()
                    .unwrap_or(f64::INFINITY)
            })
            .fold(0.0, f64::max);
        selected.push((cost,content,json!({"identifier":group["identifier"],"aliases":aliases,"original_input":original_path,"original_input_sha256":group["original_input_sha256"],"mathematical_sha256":group["mathematical_sha256"],"baseline_evidence":evidence})));
    }
    selected.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut cases = Vec::new();
    let mut records = Vec::new();
    for (index, (_, content, mut record)) in selected.into_iter().enumerate() {
        let path = output.join("inputs").join(format!("{index:04}.toml"));
        fs::write(&path, &content)?;
        record["input"] = json!(path);
        record["input_sha256"] = json!(digest(content.as_bytes()));
        cases.push(path);
        records.push(record);
    }
    let config = CampaignConfig {
        cases,
        output_dir: output.join("results"),
        equivalence_manifests: vec![
            PathBuf::from("artifacts/corpus/fixed/equivalence.json").canonicalize()?,
            PathBuf::from("artifacts/corpus/symbolic/equivalence.json").canonicalize()?,
        ],
        repeats: 1,
        solve_timeout_secs: 120.0,
        verify_timeout_secs: 240.0,
        campaign_time_secs: 3600.0,
        memory_mib: 8192,
        workers: 1,
        resume: true,
        ..Default::default()
    };
    let config_path = (!config.cases.is_empty()).then(|| output.join("campaign.toml"));
    if let Some(path) = &config_path {
        fs::write(path, toml::to_string_pretty(&config)?)?;
    }
    let report = json!({"schema_version":1,"completion_report":args.completion,"completion_sha256":digest(&completion_bytes),"parity_report":args.parity,"parity_sha256":digest(&parity_bytes),"selected":records,"skipped":skipped,"policy":"Currently unresolved full problems only; borrow a verified full unsplit order or a split polynomial's negative-branch order when both branches verified. Preserve original mathematical input/groups/explicit fixed order, disable all reductions, never count separate sign proofs as a full problem completion."});
    fs::write(
        output.join("preparation.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{}",
        json!({"selected":config.cases.len(),"skipped":report["skipped"].as_array().unwrap().len(),"config":config_path})
    );
    Ok(())
}
