//! Bounded replay of the nonvanishing portfolio on a captured exact operand.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
use symgcad::{
    algebra::Algebra,
    domain::{CertificateOptions, certify_nonzero, derive_domain, verify_certificate},
    optimized_projection::OperationCapture,
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    /// Each expression is an original strict positivity constraint p > 0.
    #[arg(long)]
    constraint: Vec<String>,
    /// Original constraint indices omitted to certify a larger domain.
    #[arg(long)]
    exclude_constraint: Vec<usize>,
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("capture omitted operands"))?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture input checksum mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let constraints = args
        .constraint
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&algebra, &constraints, &args.exclude_constraint)?;
    let mut results = Vec::new();
    for operand in operands {
        let p = algebra.parse(operand)?;
        let started = Instant::now();
        let certificate = certify_nonzero(&algebra, &p, &domain, &CertificateOptions::default())?;
        let seconds = started.elapsed().as_secs_f64();
        if let Some(certificate) = &certificate {
            verify_certificate(
                &algebra,
                &p,
                &constraints,
                &args.exclude_constraint,
                certificate,
            )?;
        }
        results.push(json!({
            "terms": p.nterms(),
            "polynomial": algebra.format(&p),
            "seconds": seconds,
            "certified": certificate.is_some(),
            "method": certificate.as_ref().map(|c| &c.method),
            "sign": certificate.as_ref().map(|c| c.sign),
            "certificate": certificate,
            "replay_with_all_constraints": certificate.as_ref().map(|c|
                verify_certificate(&algebra, &p, &constraints, &[], c)
                    .map(|_| "accepted".to_string())
                    .unwrap_or_else(|error| format!("rejected: {error}"))),
        }));
    }
    let report = json!({"capture":args.capture,"capture_sha256":format!("{:x}", Sha256::digest(&bytes)),
        "constraints":args.constraint,"excluded_constraints":args.exclude_constraint,
        "domain":domain,"results":results});
    let report = serde_json::to_string_pretty(&report)? + "\n";
    if let Some(path) = args.output {
        std::fs::write(path, &report)?;
    }
    print!("{report}");
    Ok(())
}
