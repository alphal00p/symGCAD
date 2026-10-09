//! Prepare exploratory auxiliary sign annotations without changing the domain.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use symgcad::{Problem, problem::normalize};

#[derive(Parser)]
struct Args {
    fixture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, required = true, allow_hyphen_values = true)]
    split: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.fixture)?;
    let mut problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    let original_count = problem.split.len();
    problem.split.extend(args.split.clone());
    problem.name = Some(format!(
        "{}/auxiliary_split_experiment",
        problem.name.as_deref().unwrap_or("unnamed")
    ));
    let normalized = normalize(&problem, &problem.variables)?;
    ensure!(
        normalized.split[original_count..]
            .iter()
            .all(|p| !p.is_constant()),
        "auxiliary cuts must be nonconstant polynomials"
    );
    std::fs::create_dir_all(&args.output)?;
    std::fs::write(
        args.output.join("seeded.toml"),
        toml::to_string_pretty(&problem)?,
    )?;
    let evidence = json!({"status":"auxiliary_sign_annotations_only_original_proof_not_rewritten",
        "source":args.fixture,"source_sha256":format!("{:x}",Sha256::digest(bytes)),
        "helper_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("seed_probe.rs"))),
        "original_split_count":original_count,"auxiliary_splits":args.split,
        "normalized_auxiliary_splits":normalized.split[original_count..].iter().map(|p|normalized.algebra.format(p)).collect::<Vec<_>>(),
        "domain_unchanged":true,"extra_seams_must_be_retained":true});
    std::fs::write(
        args.output.join("preparation.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&evidence)?);
    Ok(())
}
