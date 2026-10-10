//! Research preparation only: exact triangular translations preserve cylinder
//! order, but a transformed solve is not an original-coordinate certificate.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use symgcad::{Problem, problem::normalize};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    axis: String,
    /// Original axis = new axis + shift(earlier coordinates).
    #[arg(long, allow_hyphen_values = true)]
    shift: String,
    #[arg(long, value_delimiter = ',')]
    order: Vec<String>,
}

fn prepare(
    source: &Problem,
    axis: &str,
    shift: &str,
    order: &[String],
) -> Result<(Problem, serde_json::Value)> {
    let mut trial = source.clone();
    if !source.solver.order.is_empty() {
        ensure!(
            source.solver.order == order,
            "cannot override a fixed order"
        );
    } else if source.solver.ordering == "given" {
        ensure!(
            source.effective_order() == order,
            "cannot override given order"
        );
    }
    trial.solver.order = order.to_vec();
    trial.solver.ordering = "given".into();
    trial.validate()?;
    let normal = normalize(&trial, order)?;
    let index = order
        .iter()
        .position(|name| name == axis)
        .context("unknown axis")?;
    let shift = normal.algebra.parse(shift)?;
    ensure!(
        (index..order.len()).all(|i| shift.degree(i) == 0),
        "shift must use only earlier coordinates"
    );
    let coordinate = normal.algebra.parse(axis)?;
    let forward = &coordinate + &shift;
    let inverse = &coordinate - &shift;
    let mut details = Vec::new();
    let mut translate = |kind, polynomial: &symgcad::algebra::Poly| -> Result<String> {
        let transformed = polynomial.replace_with_poly(index, &forward);
        ensure!(
            transformed.replace_with_poly(index, &inverse) == *polynomial,
            "exact translation reconstruction failed"
        );
        details.push(json!({"kind":kind,"original_terms":polynomial.nterms(),
            "transformed_terms":transformed.nterms(),"exact_inverse":true}));
        Ok(normal.algebra.format(&transformed))
    };
    trial.constraints = normal
        .positives
        .iter()
        .map(|p| Ok(format!("({})>0", translate("constraint", p)?)))
        .collect::<Result<_>>()?;
    trial.split = normal
        .split
        .iter()
        .map(|p| translate("split", p))
        .collect::<Result<_>>()?;
    trial.parameters.clear();
    trial.name = Some(format!(
        "{}/triangular-shift-{axis}",
        source.name.as_deref().unwrap_or("unnamed")
    ));
    trial.validate()?;
    Ok((
        trial,
        json!({"axis":axis,"order":order,"original_axis_expression":normal.algebra.format(&forward),
        "inverse_axis_expression":normal.algebra.format(&inverse),"polynomials":details,
        "original_completion":false,"requires_original_root_cell_and_exceptional_set_transport":true}),
    ))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "use a fresh output directory");
    let bytes = fs::read(&args.input)?;
    let source = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    let (trial, mut report) = prepare(&source, &args.axis, &args.shift, &args.order)?;
    let toml = toml::to_string_pretty(&trial)?;
    report["source"] = json!(args.input);
    report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
    report["transformed_sha256"] = json!(format!("{:x}", Sha256::digest(toml.as_bytes())));
    fs::create_dir_all(&args.output)?;
    fs::write(args.output.join("problem.toml"), toml)?;
    fs::write(
        args.output.join("preparation.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Problem {
        Problem::from_toml("variables=['s','t','x']\nconstraints=['s>4','t<2','x>0']\nsplit=['(s+t)^2*x+t']\nvariable_groups=[['s','t'],['x']]\n").unwrap()
    }
    #[test]
    fn exact_kinematic_shear_preserves_all_constraints_and_split() {
        let p = fixture();
        let (q, report) = prepare(&p, "t", "-s", &p.variables).unwrap();
        let n = normalize(&q, &q.variables).unwrap();
        assert_eq!(n.positives[1], n.algebra.parse("2-t+s").unwrap());
        assert_eq!(n.split[0], n.algebra.parse("t^2*x+t-s").unwrap());
        assert_eq!(report["original_completion"], false);
    }
    #[test]
    fn later_or_self_dependent_shift_is_rejected() {
        let p = fixture();
        for shift in ["x", "t", "s+x"] {
            assert!(prepare(&p, "t", shift, &p.variables).is_err());
        }
    }
    #[test]
    fn fixed_order_and_groups_are_obligations() {
        let mut p = fixture();
        p.solver.order = p.variables.clone();
        assert!(prepare(&p, "t", "-s", &["t".into(), "s".into(), "x".into()]).is_err());
        p.solver.order.clear();
        assert!(prepare(&p, "t", "-s", &["s".into(), "x".into(), "t".into()]).is_err());
    }
}
