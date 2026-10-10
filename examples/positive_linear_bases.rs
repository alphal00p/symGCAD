//! Bounded research screen of the base needed for one positive linear fiber.
//! No transformed result is credited as an original-problem decomposition.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{Limits, Problem, algebra::Poly, problem::normalize};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    guide: PathBuf,
    #[arg(long)]
    solver: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Test only the first peeled axis, with the guide's structural order.
    #[arg(long)]
    structural_order: bool,
    /// Check the hereditary endpoint identities without launching solve trials.
    #[arg(long)]
    algebra_only: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Deserialize)]
struct Summary {
    status: String,
    statistics: Value,
    diagnostic: Option<String>,
}
fn sign(p: &Poly) -> i8 {
    if !p.is_zero() && p.coefficients.iter().all(|c| c > &Rational::zero()) {
        1
    } else if !p.is_zero() && p.coefficients.iter().all(|c| c < &Rational::zero()) {
        -1
    } else {
        0
    }
}
fn coordinate(p: &Poly) -> Option<usize> {
    if p.nterms() != 1 || p.coefficients[0] <= Rational::zero() {
        return None;
    }
    let e = p.exponents_iter().next()?;
    let mut support = e.iter().enumerate().filter(|(_, d)| **d != 0);
    let (axis, &degree) = support.next()?;
    (degree == 1 && support.next().is_none()).then_some(axis)
}
fn nonnegative_comparison(residual: &Poly) -> Result<bool> {
    if residual.is_zero() {
        return Ok(true);
    }
    let factors = residual.factor();
    let rebuilt = factors
        .iter()
        .fold(residual.one(), |v, (q, m)| &v * &q.pow(*m));
    ensure!(rebuilt == *residual, "comparison reconstruction failed");
    Ok(factors
        .iter()
        .filter(|(_, m)| m % 2 == 1)
        .fold(1_i8, |s, (q, _)| s * sign(q))
        == 1)
}
fn endpoint_chain(
    algebra: &symgcad::algebra::Algebra,
    target: &Poly,
    guide: &Poly,
    order: &[String],
) -> Result<Value> {
    let mut target = target.clone();
    let mut guide = guide.clone();
    let mut steps = Vec::new();
    for axis_name in order {
        if sign(&target) == 1 {
            break;
        }
        let axis = algebra
            .names
            .iter()
            .position(|name| name == axis_name)
            .ok_or_else(|| anyhow::anyhow!("unknown endpoint axis"))?;
        ensure!(target.degree(axis) == 1 && guide.degree(axis) == 1);
        let a = target.derivative(axis);
        let b = target.replace(axis, &Rational::zero());
        let c = guide.derivative(axis);
        let d = guide.replace(axis, &Rational::zero());
        let residual = &b * &c - &a * &d;
        ensure!(sign(&c) == 1 && sign(&d) == 1);
        ensure!(nonnegative_comparison(&residual)?);
        let factors = residual.factor();
        let strict = !residual.is_zero() && factors.iter().all(|(p, _)| sign(p) != 0);
        steps.push(json!({
            "axis":axis_name,"target":algebra.format(&target),
            "a":algebra.format(&a),"b":algebra.format(&b),
            "c":algebra.format(&c),"d":algebra.format(&d),
            "comparison":algebra.format(&residual),
            "comparison_strict":strict,
            "comparison_factors":factors.iter().map(|(p,m)|json!({"polynomial":algebra.format(p),"multiplicity":m,"coefficient_sign":sign(p)})).collect::<Vec<_>>(),
        }));
        target = b;
        guide = d;
    }
    Ok(
        json!({"steps":steps,"terminal":algebra.format(&target),"terminal_terms":target.nterms(),"terminal_coefficient_sign":sign(&target),"terminal_guide":algebra.format(&guide)}),
    )
}
fn worker(solver: &Path, operation: &str, input: &Path, output: &Path) -> Command {
    let mut command = Command::new(solver);
    command
        .arg("_worker")
        .arg(operation)
        .arg(input)
        .arg("--output")
        .arg(output)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    command
}
fn write(path: &Path, value: &Value) -> Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "report path must be fresh");
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    if !args.worker {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg(&args.input)
            .arg("--guide")
            .arg(&args.guide)
            .arg("--solver")
            .arg(&args.solver)
            .arg("--output")
            .arg(&args.output)
            .arg("--worker");
        if args.structural_order {
            command.arg("--structural-order");
        }
        if args.algebra_only {
            command.arg("--algebra-only");
        }
        let process = symgcad::benchmark::run_bounded(
            &mut command,
            &Limits {
                wall_time_secs: 180.,
                memory_mib: 4096,
                workers: 1,
                max_cells: None,
            },
        )?;
        fs::write(
            args.output.with_extension("process.json"),
            serde_json::to_vec_pretty(&process)?,
        )?;
        ensure!(
            process.status == "completed" && process.exit_code == Some(0),
            "research worker stopped: {}",
            process.status
        );
        return Ok(());
    }
    let started = Instant::now();
    let source = fs::read(&args.input)?;
    let problem = Problem::from_toml(std::str::from_utf8(&source)?)?;
    ensure!(
        problem.variables.len() <= 8 && problem.split.is_empty(),
        "bounded unsplit input required"
    );
    let n = normalize(&problem, &problem.variables)?;
    let target = n
        .positives
        .iter()
        .enumerate()
        .filter(|(_, p)| coordinate(p).is_none())
        .collect::<Vec<_>>();
    ensure!(target.len() == 1, "one positive target required");
    for axis in 0..problem.variables.len() {
        ensure!(
            n.positives.iter().any(|p| coordinate(p) == Some(axis)),
            "positive-coordinate premise missing"
        );
    }
    let (target_index, p) = target[0];
    ensure!(p.nterms() <= 256, "target term cap exceeded");
    let guide_json: Value = serde_json::from_slice(&fs::read(&args.guide)?)?;
    let structural_order: Vec<String> =
        serde_json::from_value(guide_json["lifting_order"].clone())?;
    let guide = n.algebra.parse(
        guide_json["guide"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("guide missing"))?,
    )?;
    let case_dir = args.output.with_extension("cases");
    ensure!(!case_dir.exists(), "case directory must be fresh");
    fs::create_dir(&case_dir)?;
    let mut report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(&source)),"solver":args.solver,"solver_sha256":format!("{:x}",Sha256::digest(fs::read(&args.solver)?)),"original_completion":false,"scope":"Each case solves only B>0 with split A in the reduced coordinates. No original positive fiber transport is implemented.","runs":[]});
    let peel_order: Vec<String> = serde_json::from_value(guide_json["peel_order"].clone())?;
    report["endpoint_chain"] = endpoint_chain(&n.algebra, p, &guide, &peel_order)?;
    write(&args.output, &report)?;
    if args.algebra_only {
        return Ok(());
    }
    for axis in 0..p.nvars() {
        if args.structural_order && structural_order.last() != Some(&n.algebra.names[axis]) {
            continue;
        }
        if p.degree(axis) != 1 || guide.degree(axis) != 1 {
            continue;
        }
        let a = p.derivative(axis);
        let b = p.replace(axis, &Rational::zero());
        let c = guide.derivative(axis);
        let d = guide.replace(axis, &Rational::zero());
        ensure!(
            sign(&c) == 1 && sign(&d) == 1,
            "strict guide coefficient proof failed"
        );
        let residual = &b * &c - &a * &d;
        ensure!(
            nonnegative_comparison(&residual)?,
            "comparison nonnegativity proof failed"
        );
        for engine in ["optimized", "local"] {
            ensure!(
                started.elapsed().as_secs_f64() < 165.,
                "research campaign time exhausted"
            );
            let label = format!("{}-{engine}", n.algebra.names[axis]);
            let path = case_dir.join(&label);
            fs::create_dir(&path)?;
            let mut base = problem.clone();
            base.name = Some(format!(
                "research_positive_linear_base/{}/{label}",
                problem.name.as_deref().unwrap_or("input")
            ));
            base.variables.retain(|v| v != &n.algebra.names[axis]);
            base.variable_groups = base
                .variable_groups
                .into_iter()
                .map(|g| {
                    g.into_iter()
                        .filter(|v| v != &n.algebra.names[axis])
                        .collect::<Vec<_>>()
                })
                .filter(|g| !g.is_empty())
                .collect();
            base.parameters.clear();
            base.constraints = n
                .positives
                .iter()
                .enumerate()
                .filter(|(i, q)| *i != target_index && coordinate(q) != Some(axis))
                .map(|(_, q)| format!("({})>0", n.algebra.format(q)))
                .collect();
            base.constraints
                .push(format!("({})>0", n.algebra.format(&b)));
            base.split = vec![n.algebra.format(&a)];
            base.solver.engine = engine.into();
            base.solver.ordering = "brown".into();
            base.solver.order.clear();
            if args.structural_order {
                base.solver.ordering = "given".into();
                base.solver.order = structural_order
                    .iter()
                    .filter(|v| *v != &n.algebra.names[axis])
                    .cloned()
                    .collect();
            }
            base.solver.positivity = true;
            base.solver.merge = "none".into();
            base.solver.homogeneous_anchor = None;
            base.solver.homogeneous_block.clear();
            base.solver.negative_linear_guide = None;
            base.solver.checkpoint_dir = None;
            base.limits = Limits {
                wall_time_secs: if args.structural_order { 30. } else { 5. },
                memory_mib: 4096,
                workers: 1,
                max_cells: Some(if args.structural_order { 10_000 } else { 1000 }),
            };
            base.validate()?;
            let input = path.join("input.json");
            let result = path.join("result.json");
            fs::write(&input, serde_json::to_vec_pretty(&base)?)?;
            fs::write(path.join("input.toml"), toml::to_string_pretty(&base)?)?;
            let solve = symgcad::benchmark::run_bounded(
                &mut worker(&args.solver, "solve", &input, &result),
                &base.limits,
            )?;
            let mut row = json!({"axis":n.algebra.names[axis],"engine":engine,"leading_terms":a.nterms(),"endpoint_terms":b.nterms(),"comparison_identity_verified":true,"input":input,"solve":solve,"verified":false});
            if solve.status == "completed" && solve.exit_code == Some(0) {
                let summary: Summary =
                    serde_json::from_reader(BufReader::new(fs::File::open(&result)?))?;
                row["status"] = json!(summary.status);
                row["statistics"] = summary.statistics;
                row["diagnostic"] = json!(summary.diagnostic);
                if summary.status == "complete_generic" {
                    let verification_file = path.join("verification.json");
                    let verification = symgcad::benchmark::run_bounded(
                        &mut worker(&args.solver, "verify", &result, &verification_file),
                        &base.limits,
                    )?;
                    let verified = verification.status == "completed"
                        && verification.exit_code == Some(0)
                        && serde_json::from_slice::<Value>(&fs::read(&verification_file)?)?["verified"]
                            == true;
                    row["verification"] = json!(verification);
                    row["verified"] = json!(verified);
                }
            }
            report["runs"].as_array_mut().unwrap().push(row);
            report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
            write(&args.output, &report)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::algebra::Algebra;

    #[test]
    fn exact_factor_comparison_allows_mixed_sign_square() -> Result<()> {
        let algebra = Algebra::new(&["x".into(), "y".into()])?;
        assert!(nonnegative_comparison(&algebra.parse("10*(x-y)^2*(x+y)")?)?);
        assert!(nonnegative_comparison(&algebra.parse("0")?)?);
        Ok(())
    }

    #[test]
    fn exact_factor_comparison_rejects_negative_or_mixed_odd_factor() -> Result<()> {
        let algebra = Algebra::new(&["x".into(), "y".into()])?;
        assert!(!nonnegative_comparison(&algebra.parse("-10*(x-y)^2")?)?);
        assert!(!nonnegative_comparison(&algebra.parse("(x-y)*(x+y)^2")?)?);
        Ok(())
    }

    #[test]
    fn endpoint_rule_accepts_nonnegative_comparison_without_false_strict_claim() -> Result<()> {
        let algebra = Algebra::new(&["x".into(), "y".into(), "z".into()])?;
        let q = algebra.parse("z+1+(x-y)^2")?;
        let u = algebra.parse("z+1")?;
        let proof = endpoint_chain(&algebra, &q, &u, &["z".into()])?;
        assert_eq!(proof["steps"][0]["comparison_strict"], false);
        assert_eq!(proof["steps"][0]["axis"], "z");
        Ok(())
    }
}
