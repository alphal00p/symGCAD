//! Independently verify one retained proof with explicit process-group limits.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symgcad::{Limits, benchmark::run_bounded};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    executable: PathBuf,
    #[arg(long)]
    proof: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 600.0)]
    seconds: f64,
    #[arg(long, default_value_t = 650.0)]
    campaign_seconds: f64,
    #[arg(long, default_value_t = 32768)]
    memory_mib: u64,
}

fn validate(seconds: f64, campaign: f64, memory: u64) -> Result<()> {
    ensure!(
        seconds.is_finite() && seconds > 0.0 && seconds <= 3600.0,
        "invalid verification limit"
    );
    ensure!(
        campaign.is_finite() && campaign >= seconds && campaign <= 3600.0,
        "invalid campaign limit"
    );
    ensure!((1..=65536).contains(&memory), "invalid memory limit");
    Ok(())
}

fn fresh_directory(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(path).context("verification output must be a fresh directory")
}

fn hash(path: &Path, started: Instant, budget: f64) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        ensure!(
            started.elapsed().as_secs_f64() < budget,
            "campaign deadline during hashing"
        );
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    validate(args.seconds, args.campaign_seconds, args.memory_mib)?;
    let started = Instant::now();
    let executable = args.executable.canonicalize()?;
    let proof = args.proof.canonicalize()?;
    ensure!(
        executable.is_file() && proof.is_file(),
        "worker and proof must be regular files"
    );
    fresh_directory(&args.output)?;
    let worker_hash = hash(&executable, started, args.campaign_seconds)?;
    let proof_hash = hash(&proof, started, args.campaign_seconds)?;
    let harness_hash = hash(&std::env::current_exe()?, started, args.campaign_seconds)?;
    let provenance = json!({"executable":executable,"executable_sha256":worker_hash,
        "proof":proof,"proof_sha256":proof_hash,"proof_bytes":fs::metadata(&proof)?.len(),
        "harness_sha256":harness_hash,"worker_seconds":args.seconds,
        "campaign_seconds":args.campaign_seconds,"memory_mib":args.memory_mib,"workers":1});
    write(&args.output.join("provenance.json"), &provenance)?;
    let remaining = args.campaign_seconds - started.elapsed().as_secs_f64();
    ensure!(remaining > 0.0, "campaign deadline before verification");
    let limits = Limits {
        wall_time_secs: args.seconds.min(remaining),
        memory_mib: args.memory_mib,
        workers: 1,
        max_cells: None,
    };
    let verification_path = args.output.join("verification.json");
    let mut command = Command::new(&executable);
    command
        .args(["_worker", "verify"])
        .arg(&proof)
        .arg("--output")
        .arg(&verification_path)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let process = run_bounded(&mut command, &limits)?;
    // Preserve the bounded worker outcome even if post-run hashing reaches the
    // independent campaign deadline. No proof is edited or overwritten.
    write(&args.output.join("process.json"), &process)?;
    let after_hash = hash(&proof, started, args.campaign_seconds);
    let unchanged = after_hash.as_ref().is_ok_and(|after| after == &proof_hash);
    let verification: Option<Value> = fs::metadata(&verification_path)
        .ok()
        .filter(|m| m.is_file() && m.len() <= 1024 * 1024)
        .and_then(|_| fs::read(&verification_path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let verified = unchanged
        && process.status == "completed"
        && process.exit_code == Some(0)
        && verification.as_ref().is_some_and(|v| v["verified"] == true);
    let report = json!({"schema_version":1,"provenance":provenance,"process":process,
        "verification":verification,"proof_unchanged":unchanged,
        "post_hash_error":after_hash.err().map(|e|e.to_string()),"verified":verified,
        "elapsed_seconds":started.elapsed().as_secs_f64()});
    write(&args.output.join("report.json"), &report)?;
    println!("verified={verified}, output={}", args.output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_reject_invalid_or_unbounded_campaigns() {
        for values in [
            (f64::NAN, 650.0, 32768),
            (0.0, 650.0, 32768),
            (600.0, 599.0, 32768),
            (600.0, 3601.0, 32768),
            (600.0, 650.0, 0),
            (600.0, 650.0, 65537),
        ] {
            assert!(validate(values.0, values.1, values.2).is_err());
        }
        assert!(validate(600.0, 650.0, 32768).is_ok());
    }
    #[test]
    fn existing_output_is_never_replaced() {
        let path =
            std::env::temp_dir().join(format!("symgcad-verify-fresh-{}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fresh_directory(&path).unwrap();
        fs::write(path.join("sentinel"), "keep").unwrap();
        assert!(fresh_directory(&path).is_err());
        assert_eq!(fs::read_to_string(path.join("sentinel")).unwrap(), "keep");
        fs::remove_dir_all(path).unwrap();
    }
}
