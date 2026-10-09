//! Compare exact substitution strategies on prefixes from a saved lifting trace.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use std::{fs::File, hint::black_box, io::BufReader, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    output::{LiftTrace, SolveResult},
};

#[derive(Parser)]
struct Args {
    result: PathBuf,
    #[arg(long, default_value_t = 2000)]
    stacks: usize,
    #[arg(long, default_value_t = 3)]
    repeats: usize,
    #[arg(long)]
    output: PathBuf,
}

fn collect(
    trace: &LiftTrace,
    prefix: &mut Vec<String>,
    count: &mut usize,
    stride: usize,
    limit: usize,
    samples: &mut Vec<Vec<String>>,
) {
    if let LiftTrace::Stack { sectors, .. } = trace {
        if *count % stride == 0 && samples.len() < limit {
            samples.push(prefix.clone());
        }
        *count += 1;
        for sector in sectors {
            prefix.push(sector.sample.clone());
            collect(&sector.child, prefix, count, stride, limit, samples);
            prefix.pop();
        }
    }
}

fn sequential(
    a: &Algebra,
    p: &Poly,
    axis: usize,
    prefix: &[Rational],
) -> Result<UnivariatePolynomial<Q>> {
    ensure!(
        axis < a.names.len() && prefix.len() == axis,
        "invalid axis/prefix"
    );
    ensure!(p.variables() == &a.variables, "different variable map");
    ensure!(
        (axis + 1..a.names.len()).all(|i| p.degree(i) == 0),
        "unassigned variable"
    );
    let mut p = p.clone();
    for (i, value) in prefix.iter().enumerate() {
        p = p.replace(i, value);
    }
    ensure!(p.degree(axis) <= u16::MAX as u32, "degree limit");
    Ok(p.to_univariate_from_univariate(axis))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.stacks > 0 && args.repeats > 0,
        "positive sample/repetition counts required"
    );
    let result: SolveResult = serde_json::from_reader(BufReader::new(File::open(&args.result)?))?;
    let a = Algebra::new(&result.order)?;
    let polynomials = result
        .polynomials
        .iter()
        .map(|p| a.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let stride = (result.statistics.visited_stacks / args.stacks).max(1);
    let mut samples = Vec::new();
    collect(
        result
            .trace
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing trace"))?,
        &mut Vec::new(),
        &mut 0,
        stride,
        args.stacks,
        &mut samples,
    );
    let samples = samples
        .iter()
        .map(|prefix| {
            prefix
                .iter()
                .map(|x| rational(x))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let mut calls = 0;
    for prefix in &samples {
        let batch = a.specialize_univariate_batch(
            result.projection_levels[prefix.len()]
                .iter()
                .map(|&id| &polynomials[id]),
            prefix.len(),
            prefix,
        )?;
        let mut position = 0;
        for &id in &result.projection_levels[prefix.len()] {
            let reference = sequential(&a, &polynomials[id], prefix.len(), prefix)?;
            ensure!(
                reference == a.specialize_univariate(&polynomials[id], prefix.len(), prefix)?
                    && reference == batch[position],
                "exact substitution mismatch"
            );
            calls += 1;
            position += 1;
        }
    }
    let mut measurements = Vec::new();
    for repeat in 0..args.repeats {
        // Rotate order to avoid systematically favoring a warmed strategy.
        for offset in 0..3 {
            let strategy = (repeat + offset) % 3;
            let started = Instant::now();
            for prefix in &samples {
                let ids = &result.projection_levels[prefix.len()];
                black_box(if strategy == 2 {
                    a.specialize_univariate_batch(
                        ids.iter().map(|&id| &polynomials[id]),
                        prefix.len(),
                        prefix,
                    )?
                } else {
                    ids.iter()
                        .map(|&id| {
                            if strategy == 1 {
                                a.specialize_univariate(&polynomials[id], prefix.len(), prefix)
                            } else {
                                sequential(&a, &polynomials[id], prefix.len(), prefix)
                            }
                        })
                        .collect::<Result<Vec<_>>>()?
                });
            }
            let strategy_name = [
                "sequential_replace",
                "replace_except",
                "replace_except_batch",
            ][strategy];
            measurements.push(json!({"repeat":repeat,"strategy":strategy_name,"seconds":started.elapsed().as_secs_f64()}));
        }
    }
    let report = json!({"result":args.result,"problem":result.problem.name,"sampled_stacks":samples.len(),
        "calls_per_repeat":calls,"exact_outputs_equal":true,"measurements":measurements});
    let text = serde_json::to_string_pretty(&report)? + "\n";
    std::fs::write(args.output, &text)?;
    print!("{text}");
    Ok(())
}
