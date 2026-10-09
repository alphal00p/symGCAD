//! Research-only two-coordinate homogeneous simplex chart.
//! A verified reduced CAD is not yet an original-coordinate CAD certificate.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, process::Command};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    domain::homogeneous_degree,
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    fixture: PathBuf,
    #[arg(long)]
    a: String,
    #[arg(long)]
    b: String,
    #[arg(long, value_delimiter = ',', required = true)]
    block: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    /// Reduced lifting order; b denotes r and must precede the other block axes.
    #[arg(long, value_delimiter = ',')]
    order: Vec<String>,
    /// Omit to construct and check the exact chart without starting a solver.
    #[arg(long)]
    executable: Option<PathBuf>,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    #[arg(long, default_value_t = 60.0)]
    verification_secs: f64,
    #[arg(long, default_value_t = 4096)]
    memory_mib: u64,
    #[arg(long, default_value = "local")]
    engine: String,
    #[arg(long)]
    adjacent_pairs: bool,
    #[arg(long, default_value_t = 1_000_000)]
    max_refinements: usize,
    #[arg(long, default_value_t = 10_000_000)]
    max_nodes: usize,
}

fn positive_coordinate(p: &Poly, axis: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter().next().is_some_and(|e| {
            e.iter()
                .enumerate()
                .all(|(i, exponent)| *exponent == u32::from(i == axis))
        })
}

fn prepare(
    source: &Problem,
    a_name: &str,
    b_name: &str,
    block_names: &[String],
    order: &[String],
) -> Result<(Problem, Value)> {
    source.validate()?;
    ensure!(a_name != b_name, "simplex coordinates must differ");
    let block_set: BTreeSet<_> = block_names.iter().collect();
    ensure!(block_set.len() == block_names.len(), "duplicate block axes");
    ensure!(
        block_names.iter().any(|v| v == a_name) && block_names.iter().any(|v| v == b_name),
        "both simplex coordinates must belong to the declared block"
    );
    let mut groups = if source.variable_groups.is_empty() {
        vec![source.variables.clone()]
    } else {
        source.variable_groups.clone()
    };
    ensure!(
        groups
            .last()
            .context("empty variable map")?
            .iter()
            .collect::<BTreeSet<_>>()
            == block_set,
        "research chart requires the block to be exactly the final variable group"
    );
    groups.pop();
    let original = normalize(source, &source.variables)?;
    let index = |name: &str| -> Result<usize> {
        source
            .variables
            .iter()
            .position(|v| v == name)
            .context("unknown block axis")
    };
    let a = index(a_name)?;
    let b = index(b_name)?;
    let block = block_names
        .iter()
        .map(|v| index(v))
        .collect::<Result<Vec<_>>>()?;
    for &axis in &block {
        ensure!(
            original
                .positives
                .iter()
                .any(|p| positive_coordinate(p, axis)),
            "research chart requires an original positive coordinate constraint for {}",
            source.variables[axis]
        );
    }
    let remainder = block_names
        .iter()
        .filter(|v| v.as_str() != a_name && v.as_str() != b_name)
        .cloned()
        .collect::<Vec<_>>();
    groups.push(vec![b_name.to_owned()]);
    if !remainder.is_empty() {
        groups.push(remainder);
    }
    let mut reduced = source.clone();
    reduced.name = Some(format!(
        "{}/simplex_{}_{}_research",
        source.name.as_deref().unwrap_or("unnamed"),
        a_name,
        b_name
    ));
    reduced.variables.retain(|v| v != a_name);
    reduced.variable_groups = groups;
    reduced.parameters.clear();
    reduced.constraints.clear();
    reduced.split.clear();
    reduced.solver.homogeneous_anchor = None;
    reduced.solver.homogeneous_block.clear();
    reduced.solver.ordering = "given".into();
    reduced.solver.order = if order.is_empty() {
        reduced.variable_groups.iter().flatten().cloned().collect()
    } else {
        order.to_vec()
    };
    let mut original_order = reduced.solver.order.clone();
    let slot = original_order
        .iter()
        .position(|v| v == b_name)
        .context("reduced order omits b")?;
    original_order.insert(slot, a_name.to_owned());
    ensure!(
        source.solver.order.is_empty() || source.solver.order == original_order,
        "chart order conflicts with the original explicit fixed order"
    );
    let reduced_algebra = Algebra::new(&reduced.variables)?;
    let forward = original.algebra.parse(&format!("1-{b_name}"))?;
    let scale = original.algebra.parse(&format!("{a_name}+{b_name}"))?;
    let mut identities = Vec::new();
    for (i, p) in original.positives.iter().chain(&original.split).enumerate() {
        let degree = homogeneous_degree(p, &block).context("nonhomogeneous input polynomial")?;
        ensure!(degree <= 16, "research expansion degree exceeds 16");
        let q = p.replace_with_poly(a, &forward);
        ensure!(q.degree(a) == 0, "anchor survived substitution");
        let expression = original.algebra.format(&q);
        let parsed = reduced_algebra.parse(&expression)?;
        let mut restored = p.zero();
        for (coefficient, exponents) in q.coefficients.iter().zip(q.exponents_iter()) {
            let term_degree = block.iter().map(|&j| exponents[j]).sum::<u32>();
            ensure!(term_degree <= degree, "negative rehomogenization exponent");
            let mut monomial = p.zero();
            monomial.append_monomial(coefficient.clone(), exponents);
            restored = &restored + &(&monomial * &scale.pow((degree - term_degree) as usize));
        }
        ensure!(
            restored == *p,
            "exact polynomial rehomogenization identity failed"
        );
        if i < original.positives.len() {
            reduced.constraints.push(format!("({expression}) > 0"));
        } else {
            reduced.split.push(expression.clone());
        }
        let derivative = q.derivative(b);
        identities.push(json!({
            "input_index":i,"kind":if i<original.positives.len(){"constraint"}else{"split"},
            "original":original.algebra.format(p),"transformed":expression,
            "homogeneous_degree":degree,"original_terms":p.nterms(),"transformed_terms":q.nterms(),
            "simplex_degree":q.degree(b),"simplex_derivative":original.algebra.format(&derivative),
            "simplex_derivative_all_coefficients_nonnegative":derivative.coefficients.iter().all(|c|c>=&Rational::zero()),
            "per_axis_degrees":reduced.variables.iter().enumerate().map(|(j,n)|json!({"axis":n,"degree":parsed.degree(j)})).collect::<Vec<_>>(),
            "exact_rehomogenization_identity":true,"exact_difference":"0"
        }));
    }
    reduced.validate()?;
    let evidence = json!({
        "kind":"simplex_input_identity_only","original_completion":false,
        "a":a_name,"b_symbol_means_r":b_name,"block":block_names,
        "original_reconstruction_order":original_order,"reduced_order":reduced.solver.order,
        "forward":"a=scale; b=scale*r/(1-r); other_block=scale*z/(1-r)",
        "inverse":"scale=a; r=b/(a+b); z=other_block/(a+b)",
        "strict_chart_domain":"scale>0, 0<r<1, all z>0",
        "identity":"(a+b)^D*q(b/(a+b),other_block/(a+b))=p",
        "root_reconstruction_pending":"r boundaries need ordinals restricted to (0,1); a boundary at r=1 maps to positive infinity; other axes scale by the positive a+b",
        "identities":identities
    });
    Ok((reduced, evidence))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.verification_secs.is_finite()
            && args.verification_secs > 0.0
            && args.verification_secs <= 3600.0,
        "verification seconds must lie in (0,3600]"
    );
    let source_bytes = fs::read(&args.fixture)?;
    let source = Problem::from_toml(std::str::from_utf8(&source_bytes)?)?;
    let (mut reduced, mut evidence) = prepare(&source, &args.a, &args.b, &args.block, &args.order)?;
    fs::create_dir_all(&args.output)?;
    let output = fs::canonicalize(&args.output)?;
    reduced.solver.engine = args.engine;
    reduced.solver.positivity = true;
    reduced.solver.merge = "none".into();
    reduced.solver.local_batch_siblings = false;
    reduced.solver.local_pair_cancellations = 1;
    reduced.solver.local_adjacent_pairs = args.adjacent_pairs;
    reduced.solver.local_max_refinements = Some(args.max_refinements);
    reduced.solver.local_max_nodes = Some(args.max_nodes);
    reduced.solver.checkpoint_dir = Some(output.join("cache"));
    reduced.limits.workers = 1;
    reduced.limits.memory_mib = args.memory_mib;
    reduced.limits.wall_time_secs = args.seconds;
    reduced.validate()?;
    evidence["source"] = json!(args.fixture);
    evidence["source_sha256"] = json!(format!("{:x}", Sha256::digest(&source_bytes)));
    fs::write(output.join("source.toml"), source_bytes)?;
    fs::write(output.join("input.toml"), toml::to_string_pretty(&reduced)?)?;
    let input = output.join("input.json");
    fs::write(&input, serde_json::to_vec_pretty(&reduced)?)?;
    fs::write(
        output.join("identities.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    if let Some(executable) = args.executable {
        let executable = fs::canonicalize(executable)?;
        let result = output.join("result.json");
        let capture = output.join("capture");
        fs::create_dir_all(&capture)?;
        let mut command = Command::new(&executable);
        command
            .args(["_worker", "solve"])
            .arg(&input)
            .arg("--output")
            .arg(&result)
            .env("RAYON_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1")
            .env("SYMGCAD_PROGRESS_PATH", output.join("progress.json"))
            .env("SYMGCAD_CAPTURE_DIR", &capture);
        let process = run_bounded(&mut command, &reduced.limits)?;
        let summary: Option<Value> = serde_json::from_str(&process.stdout).ok();
        let complete = process.status == "completed"
            && summary
                .as_ref()
                .is_some_and(|v| v["status"] == "complete_generic");
        let verification = if complete {
            let mut command = Command::new(&executable);
            command
                .args(["_worker", "verify"])
                .arg(&result)
                .arg("--output")
                .arg(output.join("verification.json"))
                .env("RAYON_NUM_THREADS", "1")
                .env("OMP_NUM_THREADS", "1");
            let mut limits = reduced.limits.clone();
            limits.wall_time_secs = args.verification_secs;
            Some(run_bounded(&mut command, &limits)?)
        } else {
            None
        };
        let verified = verification
            .as_ref()
            .is_some_and(|v| v.status == "completed")
            && fs::read(output.join("verification.json"))
                .ok()
                .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
                .is_some_and(|v| v["verified"] == true);
        let record = json!({"executable":executable,"worker_sha256":format!("{:x}",Sha256::digest(fs::read(&executable)?)),
            "process":process,"summary":summary,"verification":verification,"verified_transformed_only":verified,"original_completion":false});
        fs::write(
            output.join("record.json"),
            serde_json::to_vec_pretty(&record)?,
        )?;
        println!(
            "{}",
            serde_json::to_string(
                &json!({"output":output,"transformed_status":record["summary"]["status"],"process_status":record["process"]["status"],"verified_transformed_only":verified,"original_completion":false})
            )?
        );
    } else {
        println!(
            "{}",
            serde_json::to_string(
                &json!({"output":output,"identities_checked":evidence["identities"].as_array().unwrap().len(),"original_completion":false})
            )?
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Problem {
        Problem {
            variables: vec!["s".into(), "a".into(), "b".into(), "z".into()],
            variable_groups: vec![vec!["s".into()], vec!["a".into(), "b".into(), "z".into()]],
            constraints: vec!["s-4>0".into(), "a>0".into(), "2*b>0".into(), "z>0".into()],
            split: vec!["(a+b)^2*z-s*a*b*z".into(), "0".into(), "3".into()],
            ..Default::default()
        }
    }
    fn block() -> Vec<String> {
        vec!["a".into(), "b".into(), "z".into()]
    }

    #[test]
    fn exact_chart_preserves_domain_and_checks_every_polynomial() -> Result<()> {
        let (reduced, evidence) = prepare(&source(), "a", "b", &block(), &[])?;
        assert_eq!(reduced.solver.order, vec!["s", "b", "z"]);
        assert_eq!(reduced.variables, vec!["s", "b", "z"]);
        assert_eq!(
            evidence["original_reconstruction_order"],
            json!(["s", "a", "b", "z"])
        );
        assert_eq!(evidence["identities"].as_array().unwrap().len(), 7);
        let normalized = normalize(&reduced, &reduced.variables)?;
        assert!(
            normalized
                .positives
                .iter()
                .any(|p| *p == normalized.algebra.parse("1-b").unwrap())
        );
        assert_eq!(
            normalized.split[0],
            normalized.algebra.parse("z-s*b*z+s*b^2*z")?
        );
        assert!(normalized.split[1].is_zero());
        Ok(())
    }

    #[test]
    fn detects_affine_simplex_cancellation() -> Result<()> {
        let mut problem = source();
        problem.split = vec!["(a+b)^2*z+3*b*z^2".into()];
        let (_, evidence) = prepare(&problem, "a", "b", &block(), &[])?;
        assert_eq!(evidence["identities"][4]["simplex_degree"], 1);
        assert_eq!(
            evidence["identities"][4]["simplex_derivative_all_coefficients_nonnegative"],
            true
        );
        Ok(())
    }

    #[test]
    fn rejects_nonhomogeneous_or_unproved_positive_domain() {
        let mut problem = source();
        problem.split = vec!["a+b+1".into()];
        assert!(prepare(&problem, "a", "b", &block(), &[]).is_err());
        problem = source();
        problem.constraints[1] = "a+1>0".into();
        assert!(prepare(&problem, "a", "b", &block(), &[]).is_err());
        problem = source();
        problem.constraints[3] = "z^2>0".into();
        assert!(prepare(&problem, "a", "b", &block(), &[]).is_err());
    }

    #[test]
    fn rejects_incompatible_groups_and_fixed_orders() {
        let mut problem = source();
        problem.variable_groups = vec![vec!["s".into(), "a".into(), "b".into(), "z".into()]];
        assert!(prepare(&problem, "a", "b", &block(), &[]).is_err());
        problem = source();
        problem.solver.order = vec!["s".into(), "z".into(), "a".into(), "b".into()];
        assert!(prepare(&problem, "a", "b", &block(), &[]).is_err());
        let invalid = vec!["s".into(), "z".into(), "b".into()];
        assert!(prepare(&source(), "a", "b", &block(), &invalid).is_err());
        assert!(prepare(&source(), "a", "a", &block(), &[]).is_err());
    }
}
