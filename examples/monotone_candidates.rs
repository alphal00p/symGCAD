//! Inspect exact monotone coefficient certificates without any projection.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::Poly,
    domain::{
        CertificateOptions, HomogeneousAnchor, certify_nonzero, derive_domain, homogeneous_anchor,
    },
    monotone::inspect_peels,
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    /// Original-coordinate strict restrictions for research-only piece probes.
    #[arg(long)]
    extra_constraint: Vec<String>,
    /// Use production discovery and exact replay, without solving the endpoint.
    #[arg(long)]
    inspect: bool,
    /// Inspect no scale fixing plus every anchor in the selected block.
    #[arg(long, requires = "inspect", conflicts_with = "anchor")]
    all_anchors: bool,
    #[arg(long, requires = "inspect")]
    anchor: Option<String>,
    /// Defaults to the configured scale block, last variable group, or all axes.
    #[arg(long, value_delimiter = ',', requires = "inspect")]
    block: Vec<String>,
    #[arg(long, requires = "inspect")]
    ordering: Option<String>,
    /// Check between inspections; use an external worker for a hard deadline.
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    /// Atomically updated after each completed inspection, including partial runs.
    #[arg(long, requires = "inspect")]
    output: Option<PathBuf>,
}

/// Transfer the same polynomial without using its requested sign to establish
/// coefficient positivity. This helper reports both orientations separately.
fn inspection_problem(p: &Problem) -> Result<(Problem, Option<usize>)> {
    ensure!(p.split.len() <= 1, "inspection needs at most one split");
    let mut out = p.clone();
    let target = if p.split.is_empty() {
        let n = normalize(p, &p.variables)?;
        let target = n
            .positives
            .iter()
            .position(|q| !coordinate_affine(q))
            .context("no eligible strict target or split")?;
        out.split = vec![n.algebra.format(&n.positives[target])];
        out.constraints.remove(target);
        Some(target)
    } else {
        None
    };
    out.solver.engine = "monotone".into();
    out.solver.monotone_negative_target = false;
    out.solver.homogeneous_anchor = None;
    out.solver.homogeneous_block.clear();
    out.validate()?;
    Ok((out, target))
}

fn fixed_order(p: &Problem) -> Vec<String> {
    if p.solver.order.is_empty() && !p.variable_groups.is_empty() {
        p.variable_groups.iter().flatten().cloned().collect()
    } else {
        p.effective_order()
    }
}

/// Same checked positive-scale identity as the production wrapper. No raw
/// substitution is accepted until all constraints and the target are proved
/// homogeneous and the original domain supplies a positive anchor bound.
fn anchored_problem(
    p: &Problem,
    anchor: &str,
    block: &[String],
) -> Result<(Problem, HomogeneousAnchor)> {
    let n = normalize(p, &p.variables)?;
    let index = p
        .variables
        .iter()
        .position(|v| v == anchor)
        .context("unknown anchor")?;
    let indices = block
        .iter()
        .map(|v| {
            p.variables
                .iter()
                .position(|n| n == v)
                .context("unknown block variable")
        })
        .collect::<Result<Vec<_>>>()?;
    if !p.solver.order.is_empty() || p.solver.ordering == "given" {
        let order = fixed_order(p);
        let position = order.iter().position(|v| v == anchor).unwrap();
        ensure!(
            block.iter().filter(|v| v.as_str() != anchor).all(|v| order
                .iter()
                .position(|n| n == v)
                .is_some_and(|i| i > position)),
            "given order places a ratio coordinate before its anchor"
        );
    }
    if !p.variable_groups.is_empty() {
        let anchor_group = p
            .variable_groups
            .iter()
            .position(|group| group.iter().any(|v| v == anchor))
            .context("anchor is absent from variable groups")?;
        ensure!(
            block.iter().all(|v| p
                .variable_groups
                .iter()
                .position(|group| group.contains(v))
                .is_some_and(|group| group >= anchor_group)),
            "variable groups place a ratio coordinate before its anchor"
        );
    }
    let mut all = n.positives.clone();
    all.extend(n.split.clone());
    let domain = derive_domain(&n.algebra, &n.positives, &[])?;
    let certificate = homogeneous_anchor(&n.algebra, &all, &indices, index, &domain)?
        .context("uncertified homogeneous scale reduction")?;
    let mut reduced = p.clone();
    reduced.variables.retain(|v| v != anchor);
    reduced.constraints = certificate.normalized_polynomials[..p.constraints.len()]
        .iter()
        .map(|q| format!("({q})>0"))
        .collect();
    reduced.split = certificate.normalized_polynomials[p.constraints.len()..].to_vec();
    reduced.parameters.clear();
    reduced.variable_groups = p
        .variable_groups
        .iter()
        .map(|group| {
            group
                .iter()
                .filter(|v| v.as_str() != anchor)
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|group| !group.is_empty())
        .collect();
    reduced.solver.order.retain(|v| v != anchor);
    reduced.validate()?;
    Ok((reduced, certificate))
}

fn write_report(path: Option<&PathBuf>, report: &serde_json::Value) -> Result<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temporary, serde_json::to_vec_pretty(report)?)?;
        fs::rename(temporary, path)?;
    }
    Ok(())
}

fn inspect(args: &Args, p: &Problem, bytes: &[u8]) -> Result<()> {
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 300.0,
        "inspection budget must be within (0,300] seconds"
    );
    let started = Instant::now();
    let (mut input, target) = inspection_problem(p)?;
    if let Some(ordering) = &args.ordering {
        input.solver.ordering = ordering.clone();
        // A heuristic override never discards an explicit original order.
        input.validate()?;
    }
    let block = if !args.block.is_empty() {
        args.block.clone()
    } else if !p.solver.homogeneous_block.is_empty() {
        p.solver.homogeneous_block.clone()
    } else {
        p.variable_groups
            .last()
            .cloned()
            .unwrap_or_else(|| p.variables.clone())
    };
    let anchors: Vec<Option<String>> = if args.all_anchors {
        std::iter::once(None)
            .chain(block.iter().cloned().map(Some))
            .collect()
    } else {
        vec![
            args.anchor
                .clone()
                .or_else(|| p.solver.homogeneous_anchor.clone()),
        ]
    };
    let mut report = json!({
        "kind":"monotone_peel_inspection_not_solve", "coverage_claim":false,
        "input":args.input,"input_sha256":format!("{:x}",Sha256::digest(bytes)),
        "target_constraint":target,"extra_constraints":args.extra_constraint,"ordering_override":args.ordering,
        "block":block,"requested_anchors":anchors,"completed":false,"rows":[],"seconds":0.0
    });
    write_report(args.output.as_ref(), &report)?;
    'anchors: for anchor in anchors {
        for orientation in [1, -1] {
            if started.elapsed().as_secs_f64() >= args.seconds {
                break 'anchors;
            }
            let row_started = Instant::now();
            let outcome = (|| -> Result<serde_json::Value> {
                let mut oriented = input.clone();
                if orientation == -1 {
                    oriented.split[0] = format!("-({})", oriented.split[0]);
                }
                let (prepared, homogeneity) = if let Some(anchor) = &anchor {
                    let (prepared, proof) = anchored_problem(&oriented, anchor, &block)?;
                    (prepared, Some(proof))
                } else {
                    (oriented, None)
                };
                let inspection = inspect_peels(&prepared)?;
                Ok(json!({"inspection":inspection,"homogeneity":homogeneity}))
            })();
            let mut row = json!({"anchor":anchor,"orientation":orientation,"seconds":row_started.elapsed().as_secs_f64()});
            match outcome {
                Ok(value) => {
                    row["inspection"] = value["inspection"].clone();
                    row["homogeneity"] = value["homogeneity"].clone();
                }
                Err(error) => row["error"] = json!(format!("{error:#}")),
            }
            report["rows"].as_array_mut().unwrap().push(row);
            report["seconds"] = json!(started.elapsed().as_secs_f64());
            write_report(args.output.as_ref(), &report)?;
        }
    }
    report["completed"] = json!(
        report["rows"].as_array().unwrap().len()
            == report["requested_anchors"].as_array().unwrap().len() * 2
    );
    report["seconds"] = json!(started.elapsed().as_secs_f64());
    write_report(args.output.as_ref(), &report)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn homogeneous() -> Problem {
        let mut p = Problem {
            variables: vec!["r".into(), "x".into(), "z".into()],
            constraints: vec!["r>0".into(), "x>0".into(), "z>0".into()],
            split: vec!["z^2+(x-r)*z+x^2-2*x*r".into()],
            ..Problem::default()
        };
        p.solver.ordering = "brown".into();
        p
    }

    #[test]
    fn transfer_removes_the_target_before_coefficient_certification() {
        let mut p = homogeneous();
        let q = p.split.pop().unwrap();
        p.constraints.push(format!("({q})<0"));
        let (prepared, target) = inspection_problem(&p).unwrap();
        assert_eq!(target, Some(3));
        assert_eq!(prepared.constraints.len(), 3);
        let n = normalize(&prepared, &prepared.variables).unwrap();
        let original = normalize(&p, &p.variables).unwrap();
        assert_eq!(n.split[0], original.positives[3]);
        assert_eq!(n.positives, original.positives[..3]);
        assert_eq!(p.constraints.len(), 4);
    }

    #[test]
    fn certified_anchor_preserves_conditional_discovery_and_rejects_invalid_scales() {
        let p = homogeneous();
        let (prepared, target) = inspection_problem(&p).unwrap();
        assert_eq!(target, None);
        let (reduced, evidence) = anchored_problem(&prepared, "r", &p.variables).unwrap();
        assert_eq!(reduced.variables, vec!["x", "z"]);
        assert_eq!(evidence.degrees, vec![1, 1, 1, 2]);
        let report = inspect_peels(&reduced).unwrap();
        assert!(report.steps[0].conditional_linear.is_some());
        let mut bad = p.clone();
        bad.split[0].push_str("+1");
        assert!(anchored_problem(&bad, "r", &p.variables).is_err());
        let mut bad = p.clone();
        bad.constraints.remove(0);
        assert!(anchored_problem(&bad, "r", &p.variables).is_err());
        let mut bad = p.clone();
        bad.solver.order = vec!["x".into(), "r".into(), "z".into()];
        assert!(anchored_problem(&bad, "r", &p.variables).is_err());
        let mut bad = p.clone();
        bad.variable_groups = vec![vec!["x".into()], vec!["r".into(), "z".into()]];
        assert!(anchored_problem(&bad, "r", &p.variables).is_err());
    }

    #[test]
    fn fixture_inspection_exposes_both_conditional_peels_without_solving() {
        let p = Problem::from_toml(include_str!("../tests/fixtures/a381_fixed.toml")).unwrap();
        let (prepared, _) = inspection_problem(&p).unwrap();
        let (reduced, _) = anchored_problem(&prepared, "x8", &p.variables).unwrap();
        let report = inspect_peels(&reduced).unwrap();
        assert_eq!(
            report
                .steps
                .iter()
                .map(|s| s.variable.as_str())
                .collect::<Vec<_>>(),
            vec!["x3", "x5"]
        );
        assert!(report.steps.iter().all(|s| s.conditional_linear.is_some()));
        assert_eq!(report.base.variables.len(), 4);
    }
}
fn coefficient(p: &Poly, variable: usize, degree: u32) -> Poly {
    let mut result = p.zero();
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        if m[variable] == degree {
            let mut exponent = m.to_vec();
            exponent[variable] = 0;
            result.append_monomial(c.clone(), &exponent);
        }
    }
    result
}
fn coordinate_affine(p: &Poly) -> bool {
    (0..p.nvars()).all(|v| p.degree(v) <= 1)
        && (0..p.nvars()).filter(|&v| p.degree(v) != 0).count() <= 1
}
fn positive_axis(p: &Poly, variable: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter().next().is_some_and(|m| {
            m.iter()
                .enumerate()
                .all(|(i, &e)| e == u32::from(i == variable))
        })
}
fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = fs::read(&args.input)?;
    let mut p = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    p.constraints.extend(args.extra_constraint.iter().cloned());
    if args.inspect {
        return inspect(&args, &p, &bytes);
    }
    let started = Instant::now();
    let mut n = normalize(&p, &p.variables)?;
    let target = n
        .positives
        .iter()
        .position(|p| !coordinate_affine(p))
        .context("no eligible target")?;
    let mut polynomial = n.positives.remove(target);
    if let Some(anchor) = &p.solver.homogeneous_anchor {
        let variable = p.variables.iter().position(|v| v == anchor).unwrap();
        polynomial = polynomial.replace(variable, &Rational::one());
        for constraint in &mut n.positives {
            *constraint = constraint.replace(variable, &Rational::one());
        }
    }
    let domain = derive_domain(&n.algebra, &n.positives, &[])?;
    let options = CertificateOptions::default();
    let mut rows = Vec::new();
    for orientation in [1, -1] {
        let oriented = if orientation == 1 {
            polynomial.clone()
        } else {
            -polynomial.clone()
        };
        for (variable, name) in p.variables.iter().enumerate() {
            let degree = oriented.degree(variable);
            if ![1, 2].contains(&degree) {
                continue;
            }
            let constraints: Vec<_> = n
                .positives
                .iter()
                .filter(|p| p.degree(variable) > 0)
                .collect();
            let admissible =
                !constraints.is_empty() && constraints.iter().all(|p| positive_axis(p, variable));
            if !admissible {
                continue;
            }
            let leading = coefficient(&oriented, variable, degree);
            let leading_sign = certify_nonzero(&n.algebra, &leading, &domain, &options)?
                .map(|certificate| certificate.sign);
            let linear = coefficient(&oriented, variable, 1);
            let linear_sign = if degree == 1 || linear.is_zero() {
                Some(0)
            } else {
                certify_nonzero(&n.algebra, &linear, &domain, &options)?
                    .map(|certificate| certificate.sign)
            };
            let candidate =
                leading_sign == Some(1) && (degree == 1 || matches!(linear_sign, Some(0 | 1)));
            let innermost_group = p.variable_groups.last().is_none_or(|g| g.contains(name));
            rows.push(json!({"variable":name,"degree":degree,"orientation":orientation,"leading_sign":leading_sign,"linear_sign":linear_sign,"certified_candidate":candidate,"innermost_group":innermost_group}));
        }
    }
    println!(
        "{}",
        json!({"input":args.input,"input_sha256":format!("{:x}",Sha256::digest(&bytes)),"target_constraint":target,"anchor":p.solver.homogeneous_anchor,"extra_constraints":args.extra_constraint,"rows":rows,"seconds":started.elapsed().as_secs_f64()})
    );
    Ok(())
}
