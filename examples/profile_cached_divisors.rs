//! Research-only exact reuse of proposed factors from an operation directory.
//! Run under an external timeout and memory limit; no CAD/cache output is written.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, canonical},
    optimized_projection::{AlgebraOperation, OperationCapture},
};

const FILE_BYTES: usize = 4 * 1024 * 1024;
const TOTAL_BYTES: usize = 256 * 1024 * 1024;
const CANDIDATES: usize = 4096;

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    cache: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Deserialize)]
struct Proposals {
    operation: String,
    variable_names: Vec<String>,
    output: Vec<String>,
}

fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "regular file required"
    );
    ensure!(
        fs::metadata(path)?.len() <= FILE_BYTES as u64,
        "file byte cap"
    );
    let mut b = Vec::new();
    fs::File::open(path)?
        .take(FILE_BYTES as u64 + 1)
        .read_to_end(&mut b)?;
    ensure!(b.len() <= FILE_BYTES, "file grew beyond cap");
    Ok(b)
}
fn write(path: &Path, report: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(report)?;
    ensure!(bytes.len() <= 16 * FILE_BYTES, "report byte cap");
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn bounded(p: &Poly, terms: usize) -> bool {
    p.nterms() <= terms
        && p.nvars() <= 32
        && (0..p.nvars()).all(|i| p.degree(i) <= 64)
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= 8192
                && c.denominator_ref().significant_bits() <= 8192
        })
}

/// A nonzero specialization of an actual divisor divides the specialized target.
/// A zero proposed image gives no information and must not reject the proposal.
fn fiber_rejects(source: &Poly, candidate: &Poly, axis: usize, shift: i64) -> bool {
    let mut p = source.clone();
    let mut q = candidate.clone();
    for i in 0..source.nvars() {
        if i != axis {
            let value = Rational::from(1 + shift * (i as i64 + 1));
            p = p.replace(i, &value);
            q = q.replace(i, &value);
        }
    }
    !q.is_zero() && p.try_div(&q).is_none()
}

fn divide(source: &Poly, candidate: &Poly) -> Result<Option<Poly>> {
    ensure!(
        source.variables() == candidate.variables(),
        "full map mismatch"
    );
    ensure!(!candidate.is_constant(), "nonconstant proposal required");
    let Some(quotient) = source.try_div(candidate) else {
        return Ok(None);
    };
    ensure!(
        &quotient * candidate == *source,
        "exact division reconstruction failed"
    );
    Ok(Some(quotient))
}

fn run(args: &Args) -> Result<()> {
    let start = Instant::now();
    let bytes = read(&args.capture)?;
    ensure!(hash(&bytes) == args.capture_sha256, "capture hash mismatch");
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_deref()
        .context("capture operands omitted")?;
    ensure!(
        capture.schema_version == 1
            && capture.coefficient_domain == "Q"
            && capture.operation == AlgebraOperation::Factor
            && !capture.operands_omitted_for_size
            && operands.len() == 1,
        "one exact rational factor operand required"
    );
    ensure!(
        hash(&serde_json::to_vec(operands)?) == capture.input_hash,
        "operand hash mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let source = algebra.parse(&operands[0])?;
    ensure!(
        !source.is_constant() && bounded(&source, 8192),
        "source admission"
    );
    let axis = (0..source.nvars())
        .filter(|&i| source.degree(i) > 0)
        .min_by_key(|&i| source.degree(i))
        .context("active axis")?;
    let mut report = json!({"kind":"exact_cached_divisor_diagnostic", "status":"running",
        "capture_sha256":args.capture_sha256, "source_input_hash":capture.input_hash,
        "variable_names":algebra.names, "retained_fiber_axis":axis,
        "source_terms":source.nterms(), "source_degrees":(0..source.nvars()).map(|i|source.degree(i)).collect::<Vec<_>>(),
        "source_hash":hash(algebra.format(&source).as_bytes()), "factors":[],
        "scanned_files":0,"read_bytes":0,"proposals":0,"tested":0,"fiber_rejections":0,
        "inventory_rejections":0,"inventory_truncated":false,
        "division_rejections":0,"declined":0,"phase":"inventory","complete_generic":false,
        "cache_trust":"strings are proposals only; exact division and signed reconstruction decide acceptance"});
    write(&args.output, &report)?;
    let mut paths = fs::read_dir(&args.cache)?
        .map(|e| e.map(|x| x.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(paths.len() <= 25_000, "directory entry cap");
    paths.sort();
    let mut proposals = BTreeMap::<String, (PathBuf, String)>::new();
    let mut total = 0;
    for path in paths {
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = read(&path) else {
            continue;
        };
        total += bytes.len();
        ensure!(total <= TOTAL_BYTES, "total byte cap");
        report["scanned_files"] = json!(report["scanned_files"].as_u64().unwrap() + 1);
        let saved: Proposals = serde_json::from_slice(&bytes)?;
        if saved.operation != "factor" || saved.variable_names != algebra.names {
            continue;
        }
        for expression in saved.output {
            if expression.len() > 16_384 {
                continue;
            }
            let p = algebra.parse(&expression)?;
            if p.is_constant()
                || !bounded(&p, 1024)
                || (0..source.nvars()).any(|i| p.degree(i) > source.degree(i))
            {
                report["inventory_rejections"] =
                    json!(report["inventory_rejections"].as_u64().unwrap() + 1);
                continue;
            }
            proposals
                .entry(expression)
                .or_insert_with(|| (path.clone(), hash(&bytes)));
        }
        while proposals.len() > CANDIDATES {
            let worst = proposals
                .keys()
                .max_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)))
                .unwrap()
                .clone();
            proposals.remove(&worst);
            report["inventory_truncated"] = json!(true);
        }
        if report["scanned_files"].as_u64().unwrap() % 256 == 0 {
            report["read_bytes"] = json!(total);
            report["proposals"] = json!(proposals.len());
            write(&args.output, &report)?;
        }
    }
    report["read_bytes"] = json!(total);
    report["proposals"] = json!(proposals.len());
    let mut proposals = proposals.into_iter().collect::<Vec<_>>();
    proposals.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then(a.0.cmp(&b.0)));
    let mut residual = source.clone();
    let mut product = source.one();
    for (expression, (path, file_hash)) in proposals {
        report["phase"] = json!("testing_proposal");
        report["pending"] = json!({"file":path,"file_sha256":file_hash,"expression_sha256":hash(expression.as_bytes())});
        write(&args.output, &report)?;
        let candidate = canonical(&algebra.parse(&expression)?);
        if candidate.is_constant()
            || !bounded(&candidate, 1024)
            || (0..source.nvars()).any(|i| candidate.degree(i) > residual.degree(i))
            || candidate == canonical(&residual)
        {
            report["declined"] = json!(report["declined"].as_u64().unwrap() + 1);
            continue;
        }
        report["tested"] = json!(report["tested"].as_u64().unwrap() + 1);
        if [0, 1, 2]
            .iter()
            .any(|&shift| fiber_rejects(&residual, &candidate, axis, shift))
        {
            report["fiber_rejections"] = json!(report["fiber_rejections"].as_u64().unwrap() + 1);
            continue;
        }
        let Some(quotient) = divide(&residual, &candidate)? else {
            report["division_rejections"] =
                json!(report["division_rejections"].as_u64().unwrap() + 1);
            continue;
        };
        ensure!(bounded(&quotient, 8192), "quotient resource cap");
        product = &product * &candidate;
        ensure!(
            &product * &quotient == source,
            "complete signed product failed"
        );
        residual = quotient;
        report["factors"]
            .as_array_mut()
            .unwrap()
            .push(json!({"polynomial":algebra.format(&candidate),
            "proposal_file":path,"proposal_file_sha256":file_hash,"exact_division_verified":true}));
        report["residual"] = json!(algebra.format(&residual));
        report["residual_terms"] = json!(residual.nterms());
        report["signed_product_verified"] = json!(true);
        write(&args.output, &report)?;
    }
    report["status"] = json!("diagnostic_completed");
    report["phase"] = json!("finished");
    report["pending"] = Value::Null;
    report["elapsed_seconds"] = json!(start.elapsed().as_secs_f64());
    report["residual_terms"] = json!(residual.nterms());
    report["residual_degrees"] = json!(
        (0..source.nvars())
            .map(|i| residual.degree(i))
            .collect::<Vec<_>>()
    );
    ensure!(
        hash(&read(&args.capture)?) == args.capture_sha256,
        "source changed"
    );
    write(&args.output, &report)?;
    println!(
        "{}",
        json!({"report":args.output,"status":report["status"],"accepted":report["factors"].as_array().unwrap().len(),"elapsed_seconds":report["elapsed_seconds"]})
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "fresh report path required");
    let result = run(&args);
    if let Err(error) = &result {
        if let Ok(bytes) = fs::read(&args.output) {
            if let Ok(mut report) = serde_json::from_slice::<Value>(&bytes) {
                report["status"] = json!("failed_or_declined");
                report["error"] = json!(format!("{error:#}"));
                let _ = write(&args.output, &report);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_reuse_preserves_units_and_repeated_factors() -> Result<()> {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into()])?;
        let q = a.parse("x+y")?;
        let p = a.parse("-3/7*(x+y)^2*(x^2+y+1)")?;
        let r = divide(&p, &q)?.unwrap();
        assert!(!fiber_rejects(&p, &q, 0, 0));
        let r = divide(&r, &q)?.unwrap();
        assert_eq!(r, a.parse("-3/7*(x^2+y+1)")?);
        assert!(divide(&r, &q)?.is_none());
        Ok(())
    }
    #[test]
    fn zero_fibers_are_inconclusive_and_false_fiber_hits_need_exact_division() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let q = a.parse("(y-1)*(x+1)")?;
        let p = a.parse("(y-1)*(x+1)*(x+y)")?;
        assert!(!fiber_rejects(&p, &q, 0, 0));
        assert!(divide(&p, &q)?.is_some());
        let p = a.parse("x+y")?;
        let q = a.parse("x+1")?;
        assert!(!fiber_rejects(&p, &q, 0, 0));
        assert!(fiber_rejects(&p, &q, 0, 1));
        assert!(divide(&p, &q)?.is_none());
        Ok(())
    }
}
