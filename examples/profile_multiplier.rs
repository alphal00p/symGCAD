//! Explore bounded positive multipliers on a captured projection basis.
//! This diagnoses sufficient coefficient certificates, never changes a solve.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
use symgcad::algebra::{Algebra, Poly};

#[derive(Parser)]
struct Args {
    checkpoint: PathBuf,
    #[arg(long, default_value_t = 4)]
    order: usize,
    #[arg(long, default_value_t = 20_000)]
    max_terms: usize,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Deserialize)]
struct Basis {
    variable_names: Vec<String>,
    output: Vec<String>,
    output_hash: String,
}

fn coefficient_sign(p: &Poly) -> Option<i8> {
    if p.is_zero() {
        return None;
    }
    let negative = p.coefficients[0].is_negative();
    p.coefficients
        .iter()
        .all(|c| c.is_negative() == negative)
        .then_some(if negative { -1 } else { 1 })
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.order <= 4 && args.max_terms <= 100_000,
        "bounded diagnostic limits exceeded"
    );
    let bytes = std::fs::read(&args.checkpoint)?;
    let basis: Basis = serde_json::from_slice(&bytes)?;
    ensure!(
        basis.output_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(&basis.output)?)),
        "checkpoint output hash mismatch"
    );
    let algebra = Algebra::new(&basis.variable_names)?;
    let mut already_signed = 0;
    let mut qualified = Vec::new();
    let mut mixed = 0;
    let mut multiplier_seconds = 0.0;
    for (index, text) in basis.output.iter().enumerate() {
        let polynomial = algebra.parse(text)?;
        ensure!(
            (0..polynomial.nvars()).all(|v| u64::from(polynomial.degree(v))
                <= symgcad::algebra::MAX_DEGREE.saturating_sub(args.order as u64)),
            "positive multiplier could exceed the supported exponent range"
        );
        if coefficient_sign(&polynomial).is_some() {
            already_signed += 1;
            continue;
        }
        mixed += 1;
        let start = Instant::now();
        let support = (0..polynomial.nvars())
            .filter(|&v| polynomial.degree(v) > 0)
            .collect::<Vec<_>>();
        let mut multiplier = polynomial.one();
        for variable in &support {
            multiplier = &multiplier + &algebra.parse(&basis.variable_names[*variable])?;
        }
        let mut product = polynomial.clone();
        for order in 1..=args.order {
            if product.nterms().saturating_mul(multiplier.nterms()) > args.max_terms {
                break;
            }
            product = &product * &multiplier;
            if let Some(sign) = coefficient_sign(&product) {
                // The explicitly recorded multiplier is strictly positive on
                // the positive orthant. A same-sign product proves p nonzero.
                ensure!(
                    &polynomial * &multiplier.pow(order) == product,
                    "multiplier reconstruction differs"
                );
                qualified.push(json!({"index":index,"polynomial":text,"order":order,"sign":sign,
                    "support":support,"input_terms":polynomial.nterms(),"output_terms":product.nterms()}));
                break;
            }
        }
        multiplier_seconds += start.elapsed().as_secs_f64();
    }
    let report = json!({"kind":"exploratory_positive_orthant_multiplier_diagnostic",
        "checkpoint":args.checkpoint,"checkpoint_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "assumptions":basis.variable_names.iter().map(|v|format!("{v}>0")).collect::<Vec<_>>(),
        "order_limit":args.order,"term_limit":args.max_terms,"basis_count":basis.output.len(),
        "already_same_sign":already_signed,"mixed_sign":mixed,"qualified_count":qualified.len(),
        "multiplier_seconds":multiplier_seconds,"qualified":qualified});
    std::fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{} polynomials: {already_signed} already signed; {} of {mixed} mixed cases certified in {multiplier_seconds:.6}s",
        basis.output.len(),
        qualified.len()
    );
    Ok(())
}
