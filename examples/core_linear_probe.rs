//! Research-only exact preparation for an unrestricted final linear coordinate.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use symbolica::prelude::*;
use symgcad::{Problem, algebra::Algebra, problem::normalize};
#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "x0")]
    variable: String,
    #[arg(long, default_value = "x5")]
    anchor: String,
    #[arg(long, default_value = "x1")]
    monotone_axis: String,
    #[arg(long, value_delimiter = ',')]
    order: Vec<String>,
}
fn prepare(source: &Problem, variable: &str) -> Result<(Problem, serde_json::Value)> {
    source.validate()?;
    ensure!(
        source.split.is_empty(),
        "requires an unsplit inequality problem"
    );
    let index = source
        .variables
        .iter()
        .position(|v| v == variable)
        .context("unknown free axis")?;
    let normal = normalize(source, &source.variables)?;
    let targets = normal
        .positives
        .iter()
        .enumerate()
        .filter(|(_, p)| p.degree(index) > 0)
        .collect::<Vec<_>>();
    ensure!(
        targets.len() == 1,
        "free axis must occur in exactly one strict constraint, with no coordinate bound"
    );
    let (target, p) = targets[0];
    ensure!(p.degree(index) == 1, "target must be linear in free axis");
    let a = p.derivative(index);
    let b = p.replace(index, &Rational::zero());
    ensure!(
        !a.is_zero(),
        "identically zero leading coefficient cannot be excluded"
    );
    let axis = normal.algebra.parse(variable)?;
    ensure!(&(&a * &axis) + &b == *p, "exact affine identity failed");
    let mut base = source.clone();
    base.name = Some(format!(
        "{}/free-linear-{variable}-coefficient",
        source.name.as_deref().unwrap_or("unnamed")
    ));
    base.variables.remove(index);
    for group in &mut base.variable_groups {
        group.retain(|v| v != variable);
    }
    base.variable_groups.retain(|g| !g.is_empty());
    base.parameters.clear();
    base.constraints = normal
        .positives
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != target)
        .map(|(_, p)| format!("({})>0", normal.algebra.format(p)))
        .collect();
    base.split = vec![normal.algebra.format(&a)];
    base.solver.order.retain(|v| v != variable);
    base.solver.homogeneous_anchor = None;
    base.solver.homogeneous_block.clear();
    base.validate()?;
    let evidence = json!({"source_constraint_index":target,"variable":variable,"linear_coefficient":normal.algebra.format(&a),"endpoint":normal.algebra.format(&b),"polynomial":normal.algebra.format(p),"exact_affine_identity":true,
        "exceptional_seam":normal.algebra.format(&a),"boundary":"-endpoint/linear_coefficient","positive_coefficient_fiber":"x > boundary","negative_coefficient_fiber":"x < boundary",
        "base_split_terms":a.nterms(),"endpoint_terms":b.nterms(),"no_endpoint_projection_needed":true,"original_completion":false});
    Ok((base, evidence))
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "use a fresh output directory");
    let bytes = fs::read(&args.input)?;
    let source = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    let (mut base, mut evidence) = prepare(&source, &args.variable)?;
    let group = base
        .variable_groups
        .last()
        .context("missing inner variable group")?
        .clone();
    ensure!(
        group.contains(&args.anchor) && group.contains(&args.monotone_axis),
        "anchor and monotone axis must be in final group"
    );
    let mut order = base
        .variable_groups
        .iter()
        .take(base.variable_groups.len() - 1)
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    order.push(args.anchor.clone());
    if args.order.is_empty() {
        order.extend(
            group
                .iter()
                .filter(|v| *v != &args.anchor && *v != &args.monotone_axis)
                .cloned(),
        );
    } else {
        order.extend(args.order);
    }
    order.push(args.monotone_axis.clone());
    let mut original_order = order.clone();
    original_order.push(args.variable.clone());
    let mut original_check = source.clone();
    original_check.solver.order = original_order.clone();
    original_check.validate()?;
    if !source.solver.order.is_empty() {
        ensure!(
            source.solver.order == original_order,
            "cannot override original explicit order"
        );
    }
    if source.solver.ordering == "given" && source.solver.order.is_empty() {
        ensure!(
            source.effective_order() == original_order,
            "cannot override original given order"
        );
    }
    base.solver.ordering = "given".into();
    base.solver.order = order;
    base.solver.engine = "monotone".into();
    base.solver.monotone_base_engine = "local".into();
    base.solver.homogeneous_anchor = Some(args.anchor);
    base.solver.homogeneous_block = group;
    base.solver.positivity = true;
    base.solver.local_adjacent_pairs = true;
    base.solver.local_batch_siblings = false;
    base.solver.local_max_refinements = Some(100000);
    base.solver.local_max_nodes = Some(1000000);
    base.limits.workers = 1;
    base.limits.memory_mib = 4096;
    base.limits.wall_time_secs = 60.0;
    base.validate()?;
    evidence["source"] = json!(args.input);
    evidence["source_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    evidence["original_order"] = json!(original_order);
    evidence["status"] = json!("exact_affine_identity_and_base_preparation_only");
    fs::create_dir_all(&args.output)?;
    fs::write(
        args.output.join("base.toml"),
        toml::to_string_pretty(&base)?,
    )?;
    fs::write(
        args.output.join("identity.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&evidence)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn problem(constraints: Vec<String>) -> Problem {
        Problem {
            variables: vec!["a".into(), "x".into()],
            constraints,
            ..Default::default()
        }
    }
    #[test]
    fn unrestricted_linear_axis_only_needs_its_coefficient() {
        let p = problem(vec!["a*x+1>0".into(), "a>0".into()]);
        let (base, e) = prepare(&p, "x").unwrap();
        assert_eq!(base.variables, vec!["a"]);
        let algebra = Algebra::new(&base.variables).unwrap();
        assert_eq!(base.split.len(), 1);
        assert_eq!(
            algebra.parse(&base.split[0]).unwrap(),
            algebra.parse("a").unwrap()
        );
        assert_eq!(
            algebra.parse(e["endpoint"].as_str().unwrap()).unwrap(),
            algebra.parse("1").unwrap()
        );
    }
    #[test]
    fn bounded_or_nonlinear_axis_is_rejected() {
        assert!(prepare(&problem(vec!["a*x+1>0".into(), "x>0".into()]), "x").is_err());
        assert!(prepare(&problem(vec!["a*x^2+1>0".into()]), "x").is_err());
    }
    #[test]
    fn exact_coefficient_identity_survives_rational_normalization() {
        let (base, e) = prepare(&problem(vec!["a*x/2-1/3>0".into()]), "x").unwrap();
        let a = Algebra::new(&["a".into(), "x".into()]).unwrap();
        assert_eq!(
            a.parse(e["linear_coefficient"].as_str().unwrap()).unwrap(),
            a.parse("3*a").unwrap()
        );
        assert_eq!(base.split.len(), 1);
        assert_eq!(a.parse(&base.split[0]).unwrap(), a.parse("3*a").unwrap());
        assert_eq!(
            a.parse(e["endpoint"].as_str().unwrap()).unwrap(),
            a.parse("-2").unwrap()
        );
    }
}
