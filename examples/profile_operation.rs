//! Replay captured algebra with existing Symbolica algorithms. Run each trial
//! under an external timeout and memory limit; this tool changes no backend.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    algebra::{Algebra, Poly, canonical},
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Strategy {
    Auto,
    IntegerDucos,
    RationalDucos,
    Brown,
    Primitive,
    Crt,
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_enum, default_value = "auto")]
    strategy: Strategy,
    #[arg(long)]
    output: Option<PathBuf>,
}

fn resultant(left: &Poly, right: &Poly, variable: usize, strategy: Strategy) -> Poly {
    let left = left.to_univariate(variable);
    let right = right.to_univariate(variable);
    match strategy {
        Strategy::Auto => left.resultant_auto(&right),
        Strategy::IntegerDucos => left.resultant_ducos_integer(&right),
        Strategy::RationalDucos => left.resultant(&right),
        Strategy::Brown => left.resultant_brown(&right),
        Strategy::Primitive => left.resultant_primitive(&right),
        Strategy::Crt => left.resultant_crt(&right),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("capture omitted oversized operands"))?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture input checksum mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let polynomials = operands
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let start = Instant::now();
    let output = match capture.operation {
        AlgebraOperation::Factor => {
            ensure!(
                polynomials.len() == 1,
                "factor capture must have one operand"
            );
            ensure!(
                matches!(args.strategy, Strategy::Auto),
                "factor profiling supports the existing auto strategy only"
            );
            polynomials[0]
                .factor()
                .into_iter()
                .map(|(p, _)| p)
                .filter(|p| !p.is_constant())
                .collect::<Vec<_>>()
        }
        AlgebraOperation::Resultant => {
            ensure!(
                polynomials.len() == 2,
                "resultant capture must have two operands"
            );
            let variable = capture
                .variable
                .ok_or_else(|| anyhow::anyhow!("missing elimination variable"))?;
            ensure!(
                variable < algebra.names.len(),
                "elimination variable out of range"
            );
            vec![resultant(
                &polynomials[0],
                &polynomials[1],
                variable,
                args.strategy,
            )]
        }
        AlgebraOperation::Discriminant => {
            ensure!(
                polynomials.len() == 1,
                "discriminant capture must have one operand"
            );
            let variable = capture
                .variable
                .ok_or_else(|| anyhow::anyhow!("missing elimination variable"))?;
            ensure!(
                variable < algebra.names.len(),
                "elimination variable out of range"
            );
            let p = &polynomials[0];
            let degree = p.degree(variable);
            if degree < 2 {
                vec![if degree == 0 { p.zero() } else { p.one() }]
            } else {
                let r = resultant(p, &p.derivative(variable), variable, args.strategy);
                let (mut disc, remainder) = r.quot_rem(&p.univariate_lcoeff(variable), false);
                ensure!(remainder.is_zero(), "discriminant division was not exact");
                if degree % 4 == 2 || degree % 4 == 3 {
                    disc = -disc;
                }
                vec![disc]
            }
        }
    };
    let algebra_seconds = start.elapsed().as_secs_f64();
    let mut canonical_output = output
        .iter()
        .map(|p| algebra.format(&canonical(p)))
        .collect::<Vec<_>>();
    canonical_output.sort();
    canonical_output.dedup();
    let report = json!({
        "schema_version":1,"capture":args.capture,"capture_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "operation":capture.operation,"strategy":format!("{:?}",args.strategy),
        "capture_backend":capture.backend_version,"backend":LicenseManager::get_version(),
        "variable":capture.variable,"variables":algebra.names,"input_hash":capture.input_hash,
        "input_terms":polynomials.iter().map(Poly::nterms).collect::<Vec<_>>(),
        "input_degrees":polynomials.iter().map(|p|(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "algebra_seconds":algebra_seconds,"output_terms":output.iter().map(Poly::nterms).sum::<usize>(),
        "canonical_output_hash":format!("{:x}",Sha256::digest(serde_json::to_vec(&canonical_output)?)),
        "maximum_coefficient_bits":output.iter().flat_map(|p|&p.coefficients).map(|c|c.numerator_ref().significant_bits().max(c.denominator_ref().significant_bits())).max().unwrap_or(0)
    });
    if let Some(path) = args.output {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = BufWriter::new(File::create(path)?);
        serde_json::to_writer_pretty(&mut out, &report)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }
    let mut out = BufWriter::new(std::io::stdout().lock());
    serde_json::to_writer_pretty(&mut out, &report)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
