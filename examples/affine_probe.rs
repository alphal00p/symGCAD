//! Exact input preparation for a triangular affine shear experiment.
//! Output CADs are in transformed coordinates, not original proof artifacts.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use symgcad::{
    Problem, algebra::Algebra, optimized_projection::OperationCapture, problem::normalize,
};

#[derive(Parser)]
struct Args {
    fixture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Optional earlier exact operation whose term growth should be compared.
    #[arg(long)]
    capture: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let input = std::fs::read(&args.fixture)?;
    let problem = Problem::from_toml(std::str::from_utf8(&input)?)?;
    let original = normalize(&problem, &problem.variables)?;
    let s = problem
        .variables
        .iter()
        .position(|n| n == "s")
        .ok_or_else(|| anyhow::anyhow!("missing s"))?;
    let t = problem
        .variables
        .iter()
        .position(|n| n == "t")
        .ok_or_else(|| anyhow::anyhow!("missing t"))?;
    ensure!(
        s < t,
        "the positive-slope triangular shear requires s before t"
    );
    ensure!(
        !problem.variables.iter().any(|n| n == "u"),
        "u is already declared"
    );
    let mut names = problem.variables.clone();
    names.push("u".into());
    let extended = Algebra::new(&names)?;
    let forward = extended.parse("u-s-13/9")?;
    let inverse = extended.parse("s+t+13/9")?;
    let mut transformed = problem.clone();
    transformed.name = Some(format!(
        "{}/triangular_shear_experiment",
        problem.name.as_deref().unwrap_or("unnamed")
    ));
    transformed.variables[t] = "u".into();
    for group in &mut transformed.variable_groups {
        for name in group {
            if name == "t" {
                *name = "u".into();
            }
        }
    }
    // Ordering probes must preserve the triangular parameter order. Refine
    // its existing group, retaining all original inter-group restrictions.
    transformed.variable_groups = transformed
        .variable_groups
        .into_iter()
        .flat_map(|group| {
            if group.iter().any(|name| name == "s" || name == "u") {
                group.into_iter().map(|name| vec![name]).collect::<Vec<_>>()
            } else {
                vec![group]
            }
        })
        .collect();
    for name in &mut transformed.solver.order {
        if name == "t" {
            *name = "u".into();
        }
    }
    transformed.parameters.clear();
    transformed.constraints.clear();
    transformed.split.clear();
    let mut identities = Vec::new();
    for (i, polynomial) in original.positives.iter().chain(&original.split).enumerate() {
        let p = extended.parse(&original.algebra.format(polynomial))?;
        let q = p.replace_with_poly(t, &forward);
        ensure!(q.degree(t) == 0, "original t survived the substitution");
        ensure!(
            q.replace_with_poly(names.len() - 1, &inverse) == p,
            "exact inverse identity failed"
        );
        let expression = extended.format(&q);
        if i < original.positives.len() {
            transformed.constraints.push(format!("({expression}) > 0"));
        } else {
            transformed.split.push(expression.clone());
        }
        identities.push(json!({"input_index":i,"original":extended.format(&p),"transformed":expression,
            "original_terms":p.nterms(),"transformed_terms":q.nterms(),"exact_inverse_identity":true}));
    }
    transformed.validate()?;
    let mut capture_report = None;
    if let Some(path) = &args.capture {
        let bytes = std::fs::read(path)?;
        let capture: OperationCapture = serde_json::from_slice(&bytes)?;
        let old_t = capture
            .variable_names
            .iter()
            .position(|n| n == "t")
            .ok_or_else(|| anyhow::anyhow!("capture has no t"))?;
        let mut capture_names = capture.variable_names.clone();
        capture_names.push("u".into());
        let algebra = Algebra::new(&capture_names)?;
        let forward = algebra.parse("u-s-13/9")?;
        let inverse = algebra.parse("s+t+13/9")?;
        let mut operands = Vec::new();
        for expression in capture
            .operands
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("capture omitted operands"))?
        {
            let p = algebra.parse(expression)?;
            let q = p.replace_with_poly(old_t, &forward);
            ensure!(
                q.replace_with_poly(capture_names.len() - 1, &inverse) == p,
                "capture inverse identity failed"
            );
            operands.push(json!({"original_terms":p.nterms(),"transformed_terms":q.nterms(),
                "original_characters":expression.len(),"transformed_characters":algebra.format(&q).len(),
                "transformed":algebra.format(&q),"exact_inverse_identity":true}));
        }
        capture_report = Some(
            json!({"source":path,"source_sha256":format!("{:x}",Sha256::digest(bytes)),
            "operation":capture.operation,"operands":operands}),
        );
    }
    std::fs::create_dir_all(&args.output)?;
    std::fs::write(
        args.output.join("transformed.toml"),
        toml::to_string_pretty(&transformed)?,
    )?;
    let evidence = json!({"status":"input_identity_only_no_original_coordinate_cell_certificate",
        "source":args.fixture,"source_sha256":format!("{:x}",Sha256::digest(input)),
        "forward":"old_t = u-s-13/9","inverse":"u = s+old_t+13/9",
        "triangular_order_required":["s","t"],"axis_slope":"1","identities":identities,"capture":capture_report});
    std::fs::write(
        args.output.join("identities.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"output":args.output,"identities":evidence["identities"].as_array().unwrap().len(),
        "split_terms_before":original.split.iter().map(|p| p.nterms()).collect::<Vec<_>>(),
        "capture_terms":evidence["capture"]["operands"].as_array().map(|items| items.iter().map(|p|json!([p["original_terms"],p["transformed_terms"]])).collect::<Vec<_>>())})
        )?
    );
    Ok(())
}
