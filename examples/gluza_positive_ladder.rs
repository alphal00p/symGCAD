//! Exact two-fiber complement reduction; no original decomposition is asserted.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{Limits, Problem, algebra::Poly, problem::normalize};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    guide: PathBuf,
    #[arg(long)]
    rayleigh_report: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, hide = true)]
    worker: bool,
}

fn positive_coefficients(p: &Poly) -> bool {
    !p.is_zero() && p.coefficients.iter().all(|c| c > &Rational::zero())
}

fn coordinate(p: &Poly) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter()
            .next()
            .is_some_and(|e| e.iter().map(|&x| u64::from(x)).sum::<u64>() == 1)
}

struct Ladder {
    a: Poly,
    b: Poly,
    c: Poly,
    d: Poly,
    report: Value,
}
fn check(problem: &Problem, guide: &Value, rayleigh: &Value) -> Result<Ladder> {
    ensure!(problem.variables.len() == 7 && problem.split.is_empty());
    let original_order: Vec<String> = serde_json::from_value(guide["lifting_order"].clone())?;
    ensure!(
        problem.solver.order.is_empty() || problem.solver.order == original_order,
        "original fixed order conflicts with the proposed ladder"
    );
    let mut ordered = problem.clone();
    ordered.solver.order = original_order;
    ordered.validate()?;
    let n = normalize(problem, &problem.variables)?;
    let alg = &n.algebra;
    let axis = |name: &str| -> Result<usize> {
        alg.names
            .iter()
            .position(|x| x == name)
            .context("missing coordinate")
    };
    let i = axis("x0")?;
    let j = axis("x4")?;
    for k in 0..alg.names.len() {
        ensure!(
            n.positives
                .iter()
                .any(|p| coordinate(p) && p.degree(k) == 1),
            "original positive orthant premise missing"
        );
    }
    let targets = n
        .positives
        .iter()
        .filter(|p| !coordinate(p))
        .collect::<Vec<_>>();
    ensure!(targets.len() == 1, "expected one original target");
    let f = targets[0];
    ensure!(f.degree(i) == 1 && f.degree(j) == 1);
    let a = f.derivative(i).derivative(j);
    let b = f.derivative(i).replace(j, &Rational::zero());
    let c = f.derivative(j).replace(i, &Rational::zero());
    let d = f
        .replace(i, &Rational::zero())
        .replace(j, &Rational::zero());
    let x = alg.parse("x0")?;
    let y = alg.parse("x4")?;
    let first = &(&(&a * &x) * &y) + &(&b * &x);
    let second = &(&c * &y) + &d;
    ensure!(f == &(&first + &second), "bilinear reconstruction failed");
    ensure!(
        positive_coefficients(&d),
        "last endpoint is not strictly positive"
    );
    let u = alg.parse(
        guide["guide"]
            .as_str()
            .context("guide polynomial missing")?,
    )?;
    let c0 = u.derivative(i);
    let d0 = u.replace(i, &Rational::zero());
    let a0 = f.derivative(i);
    let b0 = f.replace(i, &Rational::zero());
    let w0 = &b0 * &c0 - &a0 * &d0;
    ensure!(
        positive_coefficients(&c0) && positive_coefficients(&d0) && positive_coefficients(&w0),
        "strict original guide comparison not proved"
    );
    let c4 = c0.derivative(j);
    let d4 = c0.replace(j, &Rational::zero());
    let w4 = &b * &c4 - &a * &d4;
    ensure!(
        positive_coefficients(&c4) && positive_coefficients(&d4) && positive_coefficients(&w4),
        "strict derivative guide comparison not proved"
    );
    let square = rayleigh["pairs"]
        .as_array()
        .context("pair report missing")?
        .iter()
        .find(|r| r["axes"] == json!(["x0", "x4"]))
        .context("pair square missing")?["square"]
        .as_str()
        .context("pair square is not text")?;
    let h = alg.parse(square)?;
    ensure!(
        &b * &c - &a * &d == h.pow(2),
        "proposed Rayleigh square failed exact replay"
    );
    let x1 = axis("x1")?;
    ensure!([&a, &b, &c].iter().all(|p| p.degree(x1) <= 1));
    let lower = alg.parse("1-x5-x6-x7")?;
    ensure!(
        a.derivative(x1) == lower,
        "negative derivative chain mismatch"
    );
    let r = alg.parse("x6+x7")?;
    let t = alg.parse("x5+x6+x7-1")?;
    let k = alg.parse("(1-x6-x7)*(1+x2)-x5*(x6+x7+x2)")?;
    ensure!(
        a == &k - &(&t * &alg.parse("x1")?),
        "low coefficient ladder mismatch"
    );
    let report = json!({"F":alg.format(f),"a":alg.format(&a),"b":alg.format(&b),"c":alg.format(&c),"d":alg.format(&d),"d_strict_coefficient_positive":true,"U":alg.format(&u),"C0":alg.format(&c0),"D0":alg.format(&d0),"W0_B0C0_minus_A0D0":alg.format(&w0),"C4":alg.format(&c4),"D4":alg.format(&d4),"W4_bC4_minus_aD4":alg.format(&w4),"both_guide_comparisons_strict_coefficient_positive":true,"H04":alg.format(&h),"bc_minus_ad_equals_H04_squared":true,"a_x1":alg.format(&lower),"r":alg.format(&r),"T":alg.format(&t),"K":alg.format(&k),"identities_verified":true,"case_table":[
        {"signs":[1,1,1],"x4":"(0,infinity)","x0":"(0,infinity)"},
        {"signs":[-1,-1,1],"x4":"(0,infinity)","x0":"(0,-(c*x4+d)/(a*x4+b))"},
        {"signs":[-1,-1,-1],"x4":"(0,-d/c)","x0":"(0,-(c*x4+d)/(a*x4+b))"},
        {"signs":[-1,1,1],"x4":"(0,-b/a)","x0":"(0,infinity)"},
        {"signs":[-1,1,1],"x4":"(-b/a,infinity)","x0":"(0,-(c*x4+d)/(a*x4+b))"},
        {"signs":[-1,1,-1],"x4":"(0,-b/a)","x0":"(0,infinity)"},
        {"signs":[-1,1,-1],"x4":"(-b/a,-d/c)","x0":"(0,-(c*x4+d)/(a*x4+b))"}],"generic_exceptions":["a","b","c","a*x4+b","c*x4+d","F"],"impossible_base_signs":"a>0 with b<0 or c<0","nonempty_width_proof":"a<0,b>0,c<0 implies rA<=rB from H04^2; equality at positive x4 contradicts W0>0, hence rA<rB"});
    Ok(Ladder { a, b, c, d, report })
}

fn generate(args: &Args) -> Result<()> {
    ensure!(!args.output.exists(), "output must be fresh");
    fs::create_dir_all(&args.output)?;
    let started = Instant::now();
    let source = fs::read(&args.input)?;
    let problem = Problem::from_toml(std::str::from_utf8(&source)?)?;
    let guide: Value = serde_json::from_slice(&fs::read(&args.guide)?)?;
    let rayleigh: Value = serde_json::from_slice(&fs::read(&args.rayleigh_report)?)?;
    let ladder = check(&problem, &guide, &rayleigh)?;
    let original_order: Vec<String> = serde_json::from_value(guide["lifting_order"].clone())?;
    let variables: Vec<String> = original_order
        .into_iter()
        .filter(|x| x != "x0" && x != "x4")
        .collect();
    let alg = normalize(&problem, &problem.variables)?.algebra;
    let mut base = Problem::default();
    base.name = Some("research Gluza30 positive two-fiber ladder base".into());
    base.variables = variables.clone();
    base.variable_groups = vec![variables.clone()];
    base.constraints = variables.iter().map(|x| format!("{x}>0")).collect();
    base.split = vec![
        alg.format(&ladder.a),
        alg.format(&ladder.b),
        alg.format(&ladder.c),
    ];
    base.solver.order = variables;
    base.solver.ordering = "given".into();
    base.solver.positivity = true;
    base.solver.local_adjacent_pairs = true;
    base.solver.local_max_refinements = Some(1_000_000);
    base.solver.local_max_nodes = Some(10_000_000);
    base.limits = Limits {
        wall_time_secs: 60.,
        memory_mib: 2048,
        workers: 1,
        max_cells: None,
    };
    for engine in ["optimized", "local"] {
        base.solver.engine = engine.into();
        base.validate()?;
        normalize(&base, &base.variables)?;
        fs::write(
            args.output.join(format!("base-{engine}.toml")),
            toml::to_string_pretty(&base)?,
        )?;
    }
    let report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(&source)),"guide_sha256":format!("{:x}",Sha256::digest(fs::read(&args.guide)?)),"rayleigh_proposal_sha256":format!("{:x}",Sha256::digest(fs::read(&args.rayleigh_report)?)),"identities":ladder.report,"base_variables":base.variables,"base_splits":base.split,"constant_endpoint_terms":ladder.d.nterms(),"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false,"scope":"Exact original identities and a proposed exhaustive two-fiber positive reconstruction. Original completion requires independently verified base and a production transport/replay certificate."});
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        return generate(&args);
    }
    ensure!(!args.output.exists(), "output must be fresh");
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(&args.input)
        .arg("--guide")
        .arg(&args.guide)
        .arg("--rayleigh-report")
        .arg(&args.rayleigh_report)
        .arg("--output")
        .arg(&args.output)
        .arg("--worker");
    let process = symgcad::benchmark::run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 30.,
            memory_mib: 2048,
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
        "preparation failed: {} {}",
        process.status,
        process.stderr
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_endpoints_and_strict_guides_are_replayed_against_original() -> Result<()> {
        let mut problem =
            Problem::from_toml(include_str!("../tests/fixtures/gluza30_negative.toml"))?;
        ensure!(problem.constraints[0].contains("< 0"));
        problem.constraints[0] = problem.constraints[0].replacen("< 0", "> 0", 1);
        let guide: Value =
            serde_json::from_str(include_str!("../reference/gluza30_rayleigh_guide.json"))?;
        let rayleigh = json!({"pairs":[{"axes":["x0","x4"],"square":"x6*x7+x5+x5*x6*x7+x2*x6*x7+x2*x5*x6+x1*x6*x7+x1*x5*x7-x1*x2+x1*x2*x7+x1*x2*x6+x1*x2*x5"}]});
        let ladder = check(&problem, &guide, &rayleigh)?;
        assert_eq!(ladder.d.nterms(), 8);
        let mut wrong = rayleigh.clone();
        for row in wrong["pairs"].as_array_mut().unwrap() {
            if row["axes"] == json!(["x0", "x4"]) {
                row["square"] = "1".into();
            }
        }
        assert!(check(&problem, &guide, &wrong).is_err());
        let mut wrong_problem = problem.clone();
        wrong_problem.solver.order = problem.variables.iter().rev().cloned().collect();
        assert!(check(&wrong_problem, &guide, &rayleigh).is_err());
        let mut wrong_groups = problem.clone();
        wrong_groups.variable_groups = vec![
            vec!["x0".into()],
            problem
                .variables
                .iter()
                .filter(|x| *x != "x0")
                .cloned()
                .collect(),
        ];
        assert!(check(&wrong_groups, &guide, &rayleigh).is_err());
        Ok(())
    }
    #[test]
    fn nonnegative_rayleigh_square_alone_does_not_prove_nonempty_middle_interval() {
        // -(x0+1)*(x4-1) has d>0 and bc-ad=0, but both x4
        // boundaries coincide. The strict W0 premise is therefore essential.
        let (a, b, c, d) = (-1i64, 1i64, -1i64, 1i64);
        assert_eq!(b * c - a * d, 0);
        assert_eq!(-b / a, -d / c);
        // Increasing d makes the certified ordering strict.
        assert_eq!(b * c - a * 2, 1);
        assert!(-b / a < -2 / c);
    }
}
