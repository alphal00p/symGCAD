//! Prepare a reproducible completion campaign with checked positive-scale
//! candidates. Every transformation is checked again by solve and verification.
use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use std::{fs, path::PathBuf};
use symgcad::{
    Problem,
    campaign::{CampaignConfig, Variant},
    domain::{derive_domain, homogeneous_anchor},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    /// Existing completion/ablation TOML, supplying the full frozen case list.
    input: PathBuf,
    /// New directory containing prepared inputs, manifest and campaign TOML.
    output: PathBuf,
    #[arg(long, default_value = "x2")]
    preferred_anchor: String,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(args.output.join("inputs"))?;
    let base = args.input.parent().context("campaign has no parent")?;
    let original: CampaignConfig = toml::from_str(&fs::read_to_string(&args.input)?)?;
    let mut prepared = original.clone();
    prepared.output_dir = fs::canonicalize(&args.output)?.join("results");
    prepared.equivalence_manifests = original
        .equivalence_manifests
        .iter()
        .map(|p| fs::canonicalize(base.join(p)))
        .collect::<std::io::Result<_>>()?;
    prepared.variants = vec![Variant::default()];
    prepared.solve_timeout_secs = args.seconds;
    prepared.verify_timeout_secs = args.seconds;
    prepared.cases.clear();
    let mut manifest = Vec::new();
    for (i, source) in original.cases.iter().enumerate() {
        let source = fs::canonicalize(base.join(source))?;
        let mut problem = Problem::from_toml(&fs::read_to_string(&source)?)?;
        problem.solver.engine = if problem.split.len() == 1 {
            "monotone"
        } else {
            "optimized"
        }
        .into();
        problem.solver.ordering = if problem.solver.order.is_empty() {
            "mods"
        } else {
            "given"
        }
        .into();
        problem.solver.positivity = true;
        problem.solver.input_positivity = true;
        problem.solver.homogeneous_block.clear();
        problem.solver.homogeneous_anchor = None;
        let normalized = normalize(&problem, &problem.variables)?;
        let domain = derive_domain(&normalized.algebra, &normalized.positives, &[])?;
        let block = domain
            .bounds
            .iter()
            .enumerate()
            .filter(|(_, b)| b.lower.as_deref() == Some("0") && b.upper.is_none())
            .map(|(v, _)| v)
            .collect::<Vec<_>>();
        let mut all = normalized.positives.clone();
        all.extend(normalized.split);
        let preferred = block
            .iter()
            .copied()
            .find(|&v| problem.variables[v] == args.preferred_anchor);
        let mut anchors = block.clone();
        anchors.sort_by_key(|&v| (preferred != Some(v), v));
        for anchor in anchors {
            // Kinematic groups remain outside the scale block; this ensures
            // the inverse map can be triangular in the declared group order.
            let legal_group = problem.variable_groups.is_empty()
                || problem
                    .variable_groups
                    .iter()
                    .any(|g| block.iter().all(|&v| g.contains(&problem.variables[v])));
            if !legal_group {
                break;
            }
            if !problem.solver.order.is_empty()
                && problem
                    .solver
                    .order
                    .iter()
                    .find(|v| block.iter().any(|&i| problem.variables[i] == **v))
                    != Some(&problem.variables[anchor])
            {
                continue;
            }
            if homogeneous_anchor(&normalized.algebra, &all, &block, anchor, &domain)?.is_some() {
                problem.solver.homogeneous_block = block
                    .iter()
                    .map(|&v| problem.variables[v].clone())
                    .collect();
                problem.solver.homogeneous_anchor = Some(problem.variables[anchor].clone());
                break;
            }
        }
        problem.validate()?;
        let destination = fs::canonicalize(&args.output)?.join(format!("inputs/{i:04}.toml"));
        fs::write(&destination, toml::to_string_pretty(&problem)?)?;
        manifest.push(
            json!({"source":source,"prepared":destination,"name":problem.name,
            "engine":problem.solver.engine,"block":problem.solver.homogeneous_block,
            "anchor":problem.solver.homogeneous_anchor}),
        );
        prepared.cases.push(destination);
    }
    fs::write(
        args.output.join("campaign.toml"),
        toml::to_string_pretty(&prepared)?,
    )?;
    fs::write(
        args.output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!(
        "Prepared {} cases at {}",
        prepared.cases.len(),
        args.output.display()
    );
    Ok(())
}
