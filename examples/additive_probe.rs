//! Prepare an exactly invertible triangular chart for bounded experiments.
//! A transformed CAD is not an original-coordinate coverage certificate.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use symgcad::{Problem, algebra::Algebra, problem::normalize};

#[derive(Parser)]
struct Args {
    fixture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Original-coordinate lifting order, checked for triangular dependence.
    #[arg(long, value_delimiter = ',')]
    order: Vec<String>,
    /// Retain only transformed original constraints for the domain ablation.
    #[arg(long)]
    no_derived_bounds: bool,
    /// Explicitly partition the u<0/u>0 coefficient-sign chambers.
    #[arg(long)]
    split_shear: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.fixture)?;
    let mut problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    problem.solver.order = if args.order.is_empty() {
        ["s", "t", "x2", "x3", "x4", "x7", "x1", "x5", "x8"]
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        args.order
    };
    problem.solver.ordering = "given".into();
    problem.validate()?;
    let original_order = problem.solver.order.clone();
    let mut names = problem.variables.clone();
    for name in ["u", "a", "b"] {
        ensure!(!names.iter().any(|n| n == name), "chart name already used");
        names.push(name.into());
    }
    let algebra = Algebra::new(&names)?;
    let mappings = [
        ("t", "u", "u-s-13/9", "s+t+13/9", vec!["s"]),
        ("x1", "a", "a-x2-x3", "x1+x2+x3", vec!["x2", "x3"]),
        ("x5", "b", "b-x4-x7", "x4+x5+x7", vec!["x4", "x7"]),
    ];
    let mut transforms = Vec::new();
    for (old, new, forward, inverse, earlier) in &mappings {
        let old_index = names.iter().position(|n| n == old).unwrap();
        let new_index = names.iter().position(|n| n == new).unwrap();
        let order_index = original_order.iter().position(|n| n == old).unwrap();
        ensure!(
            earlier
                .iter()
                .all(|n| original_order.iter().position(|v| v == n).unwrap() < order_index),
            "chart is not triangular in the requested original order"
        );
        transforms.push((
            old_index,
            new_index,
            algebra.parse(forward)?,
            algebra.parse(inverse)?,
        ));
    }
    let normalized = normalize(&problem, &problem.variables)?;
    let rename = |names: &mut Vec<String>| {
        for name in names {
            for (old, new, _, _, _) in &mappings {
                if name == old {
                    *name = (*new).into();
                    break;
                }
            }
        }
    };
    let mut transformed = problem.clone();
    transformed.name = Some(format!(
        "{}/additive_chart_experiment",
        problem.name.as_deref().unwrap_or("unnamed")
    ));
    rename(&mut transformed.variables);
    rename(&mut transformed.solver.order);
    for group in &mut transformed.variable_groups {
        rename(group);
    }
    transformed.parameters.clear();
    transformed.constraints.clear();
    transformed.split.clear();
    let mut identities = Vec::new();
    for (index, polynomial) in normalized
        .positives
        .iter()
        .chain(&normalized.split)
        .enumerate()
    {
        let original = algebra.parse(&normalized.algebra.format(polynomial))?;
        let mut target = original.clone();
        for (old, _, forward, _) in &transforms {
            target = target.replace_with_poly(*old, forward);
            ensure!(
                target.degree(*old) == 0,
                "original coordinate survived substitution"
            );
        }
        let mut restored = target.clone();
        for (_, new, _, inverse) in &transforms {
            restored = restored.replace_with_poly(*new, inverse);
        }
        ensure!(restored == original, "exact chart inverse identity failed");
        let expression = algebra.format(&target);
        if index < normalized.positives.len() {
            transformed.constraints.push(format!("({expression}) > 0"));
        } else {
            transformed.split.push(expression.clone());
        }
        identities.push(json!({"index":index,"original":algebra.format(&original),"transformed":expression,
            "original_terms":original.nterms(),"transformed_terms":target.nterms(),
            "degrees":transformed.solver.order.iter().map(|n| json!([n,target.degree(names.iter().position(|v|v==n).unwrap())])).collect::<Vec<_>>(),
            "exact_inverse_identity":true}));
    }
    // Make positivity that is already implied by the source orthant explicit.
    // The local positive-ray theorem deliberately requires a literal bound;
    // do not ask it to infer positivity from a shifted coordinate inequality.
    let mut derived_constraints = Vec::new();
    for (coordinate, sources) in [("a", ["x1", "x2", "x3"]), ("b", ["x4", "x5", "x7"])]
        .into_iter()
        .filter(|_| !args.no_derived_bounds)
    {
        let mut source_indices = Vec::new();
        let mut sum = algebra.parse("0")?;
        for source in sources {
            let target = algebra.parse(source)?;
            let mut found = None;
            for (index, polynomial) in normalized.positives.iter().enumerate() {
                if algebra.parse(&normalized.algebra.format(polynomial))? == target {
                    found = Some(index);
                    break;
                }
            }
            source_indices
                .push(found.ok_or_else(|| anyhow::anyhow!("missing original {source}>0 bound"))?);
            sum = sum + target;
        }
        for (old, _, forward, _) in &transforms {
            sum = sum.replace_with_poly(*old, forward);
        }
        ensure!(
            sum == algebra.parse(coordinate)?,
            "derived positivity identity failed"
        );
        transformed.constraints.push(format!("{coordinate} > 0"));
        derived_constraints.push(json!({"polynomial":coordinate,"method":"positive_sum",
            "original_constraint_indices":source_indices,"weights":["1","1","1"],"exact_identity":true}));
    }
    if args.split_shear {
        transformed.split.push("u".into());
    }
    transformed.validate()?;
    std::fs::create_dir_all(&args.output)?;
    std::fs::write(
        args.output.join("transformed.toml"),
        toml::to_string_pretty(&transformed)?,
    )?;
    let evidence = json!({"status":"input_identity_only_no_original_coordinate_cell_certificate",
        "source":args.fixture,"source_sha256":format!("{:x}",Sha256::digest(bytes)),
        "helper_source_sha256":format!("{:x}",Sha256::digest(include_bytes!("additive_probe.rs"))),
        "original_order":original_order,"transformed_order":transformed.solver.order,
        "forward":["t=u-s-13/9","x1=a-x2-x3","x5=b-x4-x7"],
        "inverse":["u=s+t+13/9","a=x1+x2+x3","b=x4+x5+x7"],
        "axis_slopes":["1","1","1"],"identities":identities,"derived_constraints":derived_constraints,
        "auxiliary_splits":if args.split_shear {vec!["u"]} else {vec![]}});
    std::fs::write(
        args.output.join("identities.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"output":args.output,"transformed_order":transformed.solver.order,
        "split_terms":evidence["identities"].as_array().unwrap().last().map(|i|json!([i["original_terms"],i["transformed_terms"]]))})
        )?
    );
    Ok(())
}
