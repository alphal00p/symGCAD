//! Compare exact resultant/discriminant kernels on operand text files.
//!
//! Every kernel is deterministic and exact; the report records the canonical
//! output hash so different kernels can be compared. Run each trial under an
//! external `timeout`/`taskset`: native calls cannot be preempted.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
use symgcad::{
    algebra::{Algebra, Poly, canonical},
    projection, resultant_interpolation, resultant_segmentation,
};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Kernel {
    /// Deterministic integer Lazard-Ducos, no dispatch.
    Ducos,
    /// Bounded interpolation with the current production options.
    Interpolation,
    /// Bounded interpolation with the historical fixed 1024 degree cap.
    InterpolationLegacy,
    /// Binary segmentation (one or two surviving parameters).
    Segmentation,
    /// Production default dispatch (projection::discriminant / exact_resultant).
    Default,
    /// Production dispatch with binary segmentation enabled.
    Binary,
    /// Only report the static plans and degree bounds, no algebra.
    Plan,
}

#[derive(Parser)]
struct Args {
    /// Comma-separated variable map.
    #[arg(long, value_delimiter = ',')]
    vars: Vec<String>,
    /// Elimination variable name.
    #[arg(long)]
    var: String,
    /// One operand file: discriminant. Two operand files: resultant.
    #[arg(long, num_args = 1..=2)]
    operand: Vec<PathBuf>,
    #[arg(long, value_enum)]
    kernel: Kernel,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Override the segmentation degree-product admission (profiling only).
    #[arg(long)]
    segmentation_min_degree_product: Option<u64>,
}

fn hash(algebra: &Algebra, p: &Poly) -> String {
    format!(
        "{:x}",
        Sha256::digest(algebra.format(&canonical(p)).as_bytes())
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    let algebra = Algebra::new(&args.vars)?;
    let variable = algebra
        .names
        .iter()
        .position(|n| n == &args.var)
        .ok_or_else(|| anyhow::anyhow!("unknown elimination variable"))?;
    let operands = args
        .operand
        .iter()
        .map(|path| algebra.parse(std::fs::read_to_string(path)?.trim()))
        .collect::<Result<Vec<_>>>()?;
    ensure!(!operands.is_empty(), "missing operand");
    let discriminant = operands.len() == 1;
    let (left, right) = if discriminant {
        (operands[0].clone(), operands[0].derivative(variable))
    } else {
        (operands[0].clone(), operands[1].clone())
    };
    let parameters: Vec<_> = (0..algebra.names.len())
        .filter(|&v| v != variable && (left.degree(v) > 0 || right.degree(v) > 0))
        .collect();
    let mut bounds = json!(null);
    if parameters.len() == 1 {
        let parameter = parameters[0];
        bounds = json!({
            "sylvester": resultant_interpolation::degree_bound(&left, &right, variable, parameter)?,
            "assignment": resultant_interpolation::assignment_degree_bound(&left, &right, variable, parameter)?,
        });
    }
    let interpolation_plan = resultant_interpolation::plan(
        &left,
        &right,
        variable,
        &resultant_interpolation::Options::default(),
    )?;
    let mut segmentation_options = resultant_segmentation::Options {
        min_parameters: 1,
        ..Default::default()
    };
    if let Some(product) = args.segmentation_min_degree_product {
        segmentation_options.min_degree_product = product;
    }
    let segmentation_plan =
        resultant_segmentation::plan(&left, &right, variable, &segmentation_options)?;
    let started = Instant::now();
    let mut statistics = json!(null);
    let resultant = match args.kernel {
        Kernel::Plan => None,
        Kernel::Ducos => Some(projection::exact_resultant_ducos(&left, &right, variable)?),
        Kernel::Interpolation | Kernel::InterpolationLegacy => {
            let options = if matches!(args.kernel, Kernel::InterpolationLegacy) {
                resultant_interpolation::Options::legacy()
            } else {
                resultant_interpolation::Options::default()
            };
            let (r, s) =
                resultant_interpolation::try_resultant(&left, &right, variable, &options, || {
                    Ok(())
                })?
                .ok_or_else(|| anyhow::anyhow!("interpolation declined"))?;
            statistics = serde_json::to_value(s)?;
            Some(r)
        }
        Kernel::Segmentation => {
            let (r, s) = resultant_segmentation::try_resultant(
                &left,
                &right,
                variable,
                &segmentation_options,
                || Ok(()),
            )?
            .ok_or_else(|| anyhow::anyhow!("segmentation declined"))?;
            statistics = serde_json::to_value(s)?;
            Some(r)
        }
        Kernel::Default | Kernel::Binary => {
            let binary = matches!(args.kernel, Kernel::Binary);
            let output = if discriminant {
                if binary {
                    let (d, s) = projection::discriminant_with_binary_statistics(
                        &operands[0],
                        variable,
                    )?;
                    statistics = serde_json::to_value(s)?;
                    d
                } else {
                    projection::discriminant(&operands[0], variable)?
                }
            } else if binary {
                if let Some((r, s)) = resultant_segmentation::try_resultant(
                    &left,
                    &right,
                    variable,
                    &resultant_segmentation::Options::default(),
                    || Ok(()),
                )? {
                    statistics = serde_json::to_value(s)?;
                    r
                } else {
                    projection::exact_resultant(&left, &right, variable)?
                }
            } else {
                projection::exact_resultant(&left, &right, variable)?
            };
            let seconds = started.elapsed().as_secs_f64();
            let report = json!({
                "kernel": format!("{:?}", args.kernel), "operation": if discriminant {"discriminant"} else {"resultant"},
                "seconds": seconds, "output_terms": output.nterms(),
                "output_degrees": (0..output.nvars()).map(|v| output.degree(v)).collect::<Vec<_>>(),
                "canonical_output_sha256": hash(&algebra, &output),
                "statistics": statistics, "bounds": bounds,
                "interpolation_plan": interpolation_plan, "segmentation_plan": segmentation_plan,
                "input_terms": operands.iter().map(Poly::nterms).collect::<Vec<_>>(),
            });
            emit(&args, &report)?;
            return Ok(());
        }
    };
    let seconds = started.elapsed().as_secs_f64();
    let mut report = json!({
        "kernel": format!("{:?}", args.kernel),
        "operation": if discriminant {"discriminant"} else {"resultant"},
        "seconds": seconds, "statistics": statistics, "bounds": bounds,
        "interpolation_plan": interpolation_plan, "segmentation_plan": segmentation_plan,
        "input_terms": operands.iter().map(Poly::nterms).collect::<Vec<_>>(),
        "input_degrees": operands.iter().map(|p| (0..p.nvars()).map(|v| p.degree(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
    });
    if let Some(resultant) = resultant {
        // Report the canonical discriminant for discriminants, so hashes are
        // comparable with the production dispatch.
        let output = if discriminant {
            canonical(&resultant.quot_rem(&operands[0].univariate_lcoeff(variable), false).0)
        } else {
            canonical(&resultant)
        };
        report["output_terms"] = json!(output.nterms());
        report["output_degrees"] = json!(
            (0..output.nvars())
                .map(|v| output.degree(v))
                .collect::<Vec<_>>()
        );
        report["resultant_degrees"] = json!(
            (0..resultant.nvars())
                .map(|v| resultant.degree(v))
                .collect::<Vec<_>>()
        );
        report["canonical_output_sha256"] = json!(hash(&algebra, &output));
    }
    emit(&args, &report)
}

fn emit(args: &Args, report: &serde_json::Value) -> Result<()> {
    let text = serde_json::to_string_pretty(report)?;
    if let Some(path) = &args.output {
        std::fs::write(path, format!("{text}\n"))?;
    }
    println!("{text}");
    Ok(())
}
