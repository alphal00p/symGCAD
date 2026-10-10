//! Research adapter from a signed discriminant candidate to the existing
//! independently evaluated resultant grid. This program does not verify it.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use symgcad::{
    algebra::{Algebra, Poly},
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    candidate: PathBuf,
    #[arg(long)]
    output: PathBuf,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    ensure!(fs::metadata(path)?.is_file(), "regular file required");
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 64 * 1024 * 1024, "research input byte cap");
    Ok(bytes)
}
fn write(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer(&mut f, value)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn resultant_candidate(p: &Poly, d: &Poly, x: usize) -> Result<(Poly, Poly)> {
    ensure!(
        x < p.nvars() && p.variables() == d.variables(),
        "candidate map/axis mismatch"
    );
    let degree = u64::from(p.degree(x));
    ensure!(
        (2..=64).contains(&degree) && d.degree(x) == 0,
        "expected bounded degree>=2 and eliminated discriminant axis"
    );
    let derivative = p.derivative(x);
    let lc = p.univariate_lcoeff(x);
    let mut r = &lc * d;
    let negative = degree * (degree - 1) / 2 % 2 == 1;
    if negative {
        r = -r;
    }
    let (mut replay, remainder) = r.quot_rem(&lc, false);
    ensure!(
        remainder.is_zero(),
        "nonexact candidate leading-coefficient division"
    );
    if negative {
        replay = -replay;
    }
    ensure!(&replay == d, "signed discriminant reconstruction failed");
    Ok((derivative, r))
}
fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = read(&args.capture)?;
    let mut capture: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(
        capture.operation == AlgebraOperation::Discriminant,
        "discriminant capture required"
    );
    let operands = capture.operands.as_ref().context("missing operands")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == capture.input_hash,
        "original operand digest mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let x = capture.variable.context("missing eliminated axis")?;
    let p = algebra.parse(&operands[0])?;
    let candidate_bytes = read(&args.candidate)?;
    let candidate: serde_json::Value = serde_json::from_slice(&candidate_bytes)?;
    ensure!(
        candidate["input_hash"].as_str() == Some(&capture.input_hash)
            && candidate["operation"] == "discriminant",
        "candidate original operation identity mismatch"
    );
    let expression = candidate["result"]
        .as_str()
        .context("missing candidate expression")?;
    ensure!(
        candidate["result_sha256"].as_str() == Some(hash(expression.as_bytes()).as_str()),
        "candidate expression digest mismatch"
    );
    let d = algebra.parse(expression)?;
    let (derivative, r) = resultant_candidate(&p, &d, x)?;
    let original_key = capture.key.clone();
    let original_input_hash = capture.input_hash.clone();
    let pair = vec![operands[0].clone(), algebra.format(&derivative)];
    capture.input_hash = hash(&serde_json::to_vec(&pair)?);
    capture.key = hash(&serde_json::to_vec(&(
        "research-derived-discriminant-grid-v1",
        &original_key,
        &capture.input_hash,
    ))?);
    capture.operation = AlgebraOperation::Resultant;
    capture.operands = Some(pair);
    capture.complete = false;
    capture.output_hash = None;
    capture.elapsed_seconds = None;
    let result = algebra.format(&r);
    let result_hash = hash(result.as_bytes());
    fs::create_dir(&args.output).context("output directory must be fresh")?;
    write(&args.output.join("pair-capture.json"), &capture)?;
    write(
        &args.output.join("resultant-candidate.json"),
        &json!({"schema_version":1,"input_hash":capture.input_hash,
        "operation":"resultant","result":result,"result_sha256":result_hash}),
    )?;
    let report = json!({"schema_version":1,"verified":false,"purpose":"prepare independent resultant grid only",
        "source_capture_sha256":hash(&bytes),"source_input_hash":original_input_hash,"source_key":original_key,
        "source_candidate_sha256":hash(&candidate_bytes),"source_discriminant_sha256":hash(expression.as_bytes()),
        "derived_pair_input_hash":capture.input_hash,"derived_resultant_sha256":result_hash,
        "eliminated_axis":x,"input_degree":p.degree(x),"derivative_degree":derivative.degree(x),
        "input_terms":p.nterms(),"derivative_terms":derivative.nterms(),"discriminant_terms":d.nterms(),"resultant_terms":r.nterms(),
        "discriminant_degrees":(0..p.nvars()).map(|i|d.degree(i)).collect::<Vec<_>>(),
        "resultant_degrees":(0..p.nvars()).map(|i|r.degree(i)).collect::<Vec<_>>(),
        "identity":"R=(-1)^(n(n-1)/2) * lc_x(p) * D; exact quotient replay; no primitive normalization",
        "degree_drop_policy":"independent grid must skip points where either input loses eliminated degree"});
    write(&args.output.join("preparation.json"), &report)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolica::prelude::*;
    use symgcad::projection::exact_resultant_ducos;
    #[test]
    fn signed_rational_scale_and_leading_coefficient_are_retained() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        for (p, d) in [
            ("(t^2-s)/3", "4*s/9"),
            ("-2/3*(t^3-s*t+1)", "16/81*(4*s^3-27)"),
            ("t^4-s", "-256*s^3"),
        ] {
            let p = a.parse(p).unwrap();
            let d = a.parse(d).unwrap();
            let (q, r) = resultant_candidate(&p, &d, 1).unwrap();
            assert_eq!(r, exact_resultant_ducos(&p, &q, 1).unwrap());
        }
    }
    #[test]
    fn degree_drop_points_cannot_count_as_grid_identities() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        let p = a.parse("s*t^2+t+1").unwrap();
        let d = a.parse("1-4*s").unwrap();
        let (q, r) = resultant_candidate(&p, &d, 1).unwrap();
        let zero = Rational::zero();
        let p0 = p.replace(0, &zero);
        let q0 = q.replace(0, &zero);
        assert_ne!(p0.degree(1), p.degree(1));
        assert_ne!(q0.degree(1), q.degree(1));
        assert_ne!(
            r.replace(0, &zero),
            exact_resultant_ducos(&p0, &q0, 1).unwrap()
        );
        for value in [-2, -1, 1, 2].map(Rational::from) {
            assert_eq!(
                r.replace(0, &value),
                exact_resultant_ducos(&p.replace(0, &value), &q.replace(0, &value), 1).unwrap()
            );
        }
    }
}
