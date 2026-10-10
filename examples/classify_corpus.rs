//! Freeze test-suite membership from an audited full-problem completion ledger.
//! This writes metadata only; generated inputs and large proofs stay untracked.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    aggregate: PathBuf,
    #[arg(long, default_value = "tests/corpus_classification.json")]
    output: PathBuf,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = fs::read(&args.aggregate)?;
    let aggregate: Value = serde_json::from_slice(&bytes)?;
    let groups = aggregate["groups"]
        .as_array()
        .context("missing audited groups")?;
    let unresolved = aggregate["unresolved"]
        .as_array()
        .context("missing unresolved groups")?;
    let pending: BTreeSet<_> = unresolved
        .iter()
        .map(|g| {
            g["identifier"]
                .as_str()
                .context("missing unresolved identifier")
        })
        .collect::<Result<_>>()?;
    ensure!(
        pending.len() == unresolved.len(),
        "duplicate unresolved identifier"
    );
    let mut seen = BTreeSet::new();
    let mut cases = Vec::new();
    for g in groups {
        let identifier = g["identifier"].as_str().context("missing identifier")?;
        ensure!(seen.insert(identifier), "duplicate corpus identifier");
        let complete = g["complete_verified"]
            .as_bool()
            .context("missing completion status")?;
        ensure!(
            complete != pending.contains(identifier),
            "unresolved list differs from audited coverage"
        );
        let attempts = g["attempts"]
            .as_array()
            .context("missing attempt evidence")?;
        for a in attempts {
            ensure!(
                a["audit_errors"].as_array().is_some_and(Vec::is_empty),
                "attempt audit errors prevent suite publication"
            );
        }
        let mut entry = json!({"classification":if complete {"verified"} else {"slow"}});
        for key in [
            "identifier",
            "aliases",
            "scope",
            "mathematical_sha256",
            "original_input",
            "original_input_sha256",
        ] {
            ensure!(!g[key].is_null(), "missing classification field {key}");
            entry[key] = g[key].clone();
        }
        cases.push(entry);
    }
    ensure!(
        pending.iter().all(|id| seen.contains(id)),
        "unresolved identifier missing from corpus"
    );
    cases.sort_by(|a, b| a["identifier"].as_str().cmp(&b["identifier"].as_str()));
    let result = json!({"schema_version":1,"source_aggregate_sha256":format!("{:x}",Sha256::digest(bytes)),
        "slow_solve_timeout_secs":3600.0,"scope":"Every distinct original problem remains listed. Verified is historical evidence, not a promise that rerunning it is fast. Slow marks every unresolved problem and never counts as passed.",
        "verified_cases":cases.len()-pending.len(),"slow_cases":pending.len(),"cases":cases});
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(args.output, serde_json::to_vec_pretty(&result)?)?;
    println!(
        "classified {} original problems: {} verified, {} slow",
        groups.len(),
        groups.len() - pending.len(),
        pending.len()
    );
    Ok(())
}
