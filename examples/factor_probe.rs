//! Compare Symbolica's generic factor call with the certified linear precheck.
use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Problem,
    algebra::{canonical, factors_checked},
    problem::normalize,
};
#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    native_factor: bool,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let text = fs::read_to_string(&args.input)?;
    let mut p = Problem::from_toml(&text)?;
    if let Some(anchor) = p.solver.homogeneous_anchor.clone() {
        let n = normalize(&p, &p.variables)?;
        let index = p
            .variables
            .iter()
            .position(|v| v == &anchor)
            .context("anchor missing")?;
        p.constraints = n
            .positives
            .iter()
            .map(|q| {
                format!(
                    "({})>0",
                    n.algebra.format(&q.replace(index, &Rational::one()))
                )
            })
            .collect();
        p.split = n
            .split
            .iter()
            .map(|q| n.algebra.format(&q.replace(index, &Rational::one())))
            .collect();
        p.variables.retain(|v| v != &anchor);
        p.variable_groups
            .iter_mut()
            .for_each(|g| g.retain(|v| v != &anchor));
        p.variable_groups.retain(|g| !g.is_empty());
        p.solver.order.retain(|v| v != &anchor);
        p.solver.homogeneous_anchor = None;
        p.solver.homogeneous_block.clear();
        p.parameters.clear();
    }
    let order = if p.solver.order.is_empty() {
        p.variables.clone()
    } else {
        p.solver.order.clone()
    };
    let n = normalize(&p, &order)?;
    let q = n
        .positives
        .iter()
        .chain(&n.split)
        .max_by_key(|p| p.nterms())
        .context("no polynomial")?;
    let started = Instant::now();
    let mut factors = if args.native_factor {
        let raw = q.factor();
        let mut reconstructed = q.one();
        for (f, m) in &raw {
            reconstructed = &reconstructed * &f.pow(*m);
        }
        anyhow::ensure!(reconstructed == *q, "generic factor reconstruction failed");
        raw.into_iter()
            .filter(|(f, _)| !f.is_constant())
            .map(|(f, _)| canonical(&f))
            .collect::<Vec<_>>()
    } else {
        factors_checked(std::slice::from_ref(q))?
    };
    let elapsed = started.elapsed().as_secs_f64();
    factors.sort_by_key(|p| n.algebra.format(p));
    factors.dedup();
    let strings = factors
        .iter()
        .map(|p| n.algebra.format(p))
        .collect::<Vec<_>>();
    println!(
        "{}",
        json!({"input":args.input,"input_sha256":format!("{:x}",Sha256::digest(text.as_bytes())),"mode":if args.native_factor{"symbolica_generic"}else{"certified_precheck"},"variables":order,"terms":q.nterms(),"seconds":elapsed,"factor_count":strings.len(),"factors_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&strings)?))})
    );
    Ok(())
}
