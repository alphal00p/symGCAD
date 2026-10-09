//! A bounded research ledger for strict subdomains; never a completion ledger.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symgcad::{Problem, benchmark::run_bounded};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    executable: PathBuf,
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 120.0)]
    campaign_secs: f64,
    #[arg(long, default_value_t = 20.0)]
    trial_secs: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Piece {
    input: PathBuf,
    extra_constraints: Vec<String>,
    label: String,
}
#[derive(Deserialize)]
struct Header {
    status: String,
}
fn hash(path: &Path) -> Result<String> {
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
fn command(binary: &Path, operation: &str, input: &Path, output: &Path) -> Command {
    let mut command = Command::new(binary);
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
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.campaign_secs > 0.0
            && args.campaign_secs <= 120.0
            && args.trial_secs > 0.0
            && args.trial_secs <= 20.0,
        "research piece campaign must fit 120 seconds and each trial 20 seconds"
    );
    ensure!(
        !args.output.exists(),
        "use a fresh research output directory"
    );
    let pieces: Vec<Piece> = serde_json::from_slice(&fs::read(&args.plan)?)?;
    fs::create_dir_all(&args.output)?;
    let output = args.output.canonicalize()?;
    let executable = output.join("symgcad");
    let executable_sha256 = hash(&args.executable)?;
    fs::copy(&args.executable, &executable)?;
    ensure!(hash(&executable)? == executable_sha256, "worker changed");
    let started = Instant::now();
    let mut records = Vec::<Value>::new();
    for (index, piece) in pieces.iter().enumerate() {
        let remaining = args.campaign_secs - started.elapsed().as_secs_f64();
        if remaining <= 0.05 {
            break;
        }
        ensure!(
            !piece.extra_constraints.is_empty(),
            "piece must have explicit strict restrictions"
        );
        let original_path = piece.input.canonicalize()?;
        let mut p = Problem::from_toml(&fs::read_to_string(&original_path)?)?;
        let original_name = p.name.clone();
        p.name = Some(format!(
            "{}/partial/{}",
            p.name.as_deref().unwrap_or("unnamed"),
            piece.label
        ));
        p.constraints
            .extend(piece.extra_constraints.iter().cloned());
        p.limits.workers = 1;
        p.limits.memory_mib = 8192;
        p.limits.wall_time_secs = args.trial_secs.min(remaining);
        p.validate()?;
        let directory = output.join(format!("{index:04}"));
        fs::create_dir(&directory)?;
        let input_file = directory.join("input.toml");
        fs::write(&input_file, toml::to_string_pretty(&p)?)?;
        let wire = directory.join("input.json");
        fs::write(&wire, serde_json::to_vec(&p)?)?;
        let result_file = directory.join("result.json");
        let verification_file = directory.join("verification.json");
        let mut worker = command(&executable, "solve", &wire, &result_file);
        worker.env("SYMGCAD_PROGRESS_PATH", directory.join("progress.json"));
        let solve = run_bounded(&mut worker, &p.limits)?;
        let header = fs::File::open(&result_file)
            .ok()
            .and_then(|f| serde_json::from_reader::<_, Header>(BufReader::new(f)).ok());
        let complete = solve.status == "completed"
            && solve.exit_code == Some(0)
            && header.is_some_and(|h| h.status == "complete_generic");
        let remaining = args.campaign_secs - started.elapsed().as_secs_f64();
        let verification = if complete && remaining > 0.05 {
            let mut limits = p.limits.clone();
            limits.wall_time_secs = args.trial_secs.min(remaining);
            Some(run_bounded(
                &mut command(&executable, "verify", &result_file, &verification_file),
                &limits,
            )?)
        } else {
            None
        };
        let check = fs::read(&verification_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        let verified = verification.as_ref().is_some_and(|run| {
            run.status == "completed"
                && run.exit_code == Some(0)
                && check.as_ref().is_some_and(|c| c["verified"] == true)
        });
        let record = json!({
            "label":piece.label,"partial_problem":true,"original_name":original_name,
            "original_input":original_path,"original_input_sha256":hash(&original_path)?,
            "extra_constraints":piece.extra_constraints,"input_file":input_file,
            "input_sha256":hash(&input_file)?,"solve":solve,"complete_partial":complete,
            "verification":verification,"verified_partial":verified,"verification_summary":check,
            "result_file":result_file,"result_sha256":result_file.exists().then(||hash(&result_file)).transpose()?,
            "verification_file":verification_file,"verification_sha256":verification_file.exists().then(||hash(&verification_file)).transpose()?
        });
        println!(
            "{}",
            json!({"index":index,"label":piece.label,"status":record["solve"]["status"],"verified_partial":verified,"elapsed":started.elapsed().as_secs_f64()})
        );
        records.push(record);
        fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version":1,"research_only":true,"original_completions_claimed":0,
                "plan":args.plan.canonicalize()?,"plan_sha256":hash(&args.plan)?,
                "executable":executable,"executable_sha256":executable_sha256,
                "harness_sha256":hash(&std::env::current_exe()?)?,
                "campaign_secs":args.campaign_secs,"elapsed_secs":started.elapsed().as_secs_f64(),
                "scheduled_pieces":pieces.len(),"attempts":records
            }))?,
        )?;
    }
    ensure!(!records.is_empty(), "no piece probe started");
    Ok(())
}
