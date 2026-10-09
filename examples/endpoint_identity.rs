//! Exact comparison of endpoint problems; this is not a solve certificate.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use symbolica::prelude::*;
use symgcad::{Problem, problem::normalize};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    left: PathBuf,
    #[arg(long)]
    right: PathBuf,
    #[arg(long, value_delimiter = ',')]
    zero: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    one: Vec<String>,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let left_bytes = fs::read(&args.left)?;
    let right_bytes = fs::read(&args.right)?;
    let left = Problem::from_toml(std::str::from_utf8(&left_bytes)?)?;
    let right = Problem::from_toml(std::str::from_utf8(&right_bytes)?)?;
    ensure!(
        left.variables == right.variables,
        "different coordinate maps"
    );
    let a = normalize(&left, &left.variables)?;
    let b = normalize(&right, &left.variables)?;
    ensure!(
        a.split.len() == 1 && b.split.len() == 1,
        "expected one split on each side"
    );
    let rhs_constraints = b
        .positives
        .iter()
        .map(|p| a.algebra.parse(&b.algebra.format(p)))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        a.positives == rhs_constraints,
        "original strict domains differ"
    );
    let mut lhs = a.split[0].clone();
    let mut rhs = a.algebra.parse(&b.algebra.format(&b.split[0]))?;
    for (names, value) in [(&args.zero, Rational::zero()), (&args.one, Rational::one())] {
        for name in names {
            let variable = left
                .variables
                .iter()
                .position(|v| v == name)
                .context("unknown substitution coordinate")?;
            lhs = lhs.replace(variable, &value);
            rhs = rhs.replace(variable, &value);
        }
    }
    let difference = &lhs - &rhs;
    ensure!(
        difference.is_zero(),
        "normalized endpoint polynomials differ"
    );
    let report = json!({
        "kind":"exact_endpoint_identity_not_completion",
        "left":args.left,"right":args.right,"variables":left.variables,
        "left_sha256":format!("{:x}",Sha256::digest(left_bytes)),
        "right_sha256":format!("{:x}",Sha256::digest(right_bytes)),
        "zero":args.zero,"one":args.one,"original_constraints_equal":true,
        "left_normalized_split":a.algebra.format(&a.split[0]),
        "right_normalized_split":b.algebra.format(&b.split[0]),
        "left_endpoint":a.algebra.format(&lhs),
        "right_endpoint":a.algebra.format(&rhs),
        "exact_difference":a.algebra.format(&difference),
        "equal":true,
        "note":"Each original problem still requires its own full-domain certified solve. This identity does not assert monotone applicability."
    });
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
