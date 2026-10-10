//! Exact structural projection audit; never a decomposition/completion proof.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits, Problem,
    algebra::{Algebra, Poly, canonical, factors_checked},
    benchmark::run_bounded,
    optimized_projection::{OptimizedProjectionOptions, ProjectionCache},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
fn write(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn coefficients(p: &Poly, v: usize) -> Vec<Poly> {
    let mut c = vec![p.zero(); p.degree(v) as usize + 1];
    for (p, e) in p.to_univariate_polynomial_list(v) {
        c[e as usize] = p;
    }
    c
}
fn quadratic_discriminant(p: &Poly, v: usize) -> Result<Poly> {
    ensure!(p.degree(v) == 2, "expected quadratic");
    let c = coefficients(p, v);
    Ok(&c[1] * &c[1] - (&c[0] * &c[2]).mul_coeff(Rational::from(4)))
}
fn monic_pair_resultant(a: &Poly, b: &Poly, v: usize) -> Result<Poly> {
    ensure!(
        a.degree(v) == 2 && b.degree(v) == 2,
        "expected quadratic pair"
    );
    let ac = coefficients(a, v);
    ensure!(ac[2].is_one(), "first quadratic must be monic");
    let remainder = b - &(a * &b.univariate_lcoeff(v));
    ensure!(remainder.degree(v) <= 1, "linear remainder identity");
    let rc = coefficients(&remainder, v);
    let m = &rc[0];
    let l = rc.get(1).cloned().unwrap_or_else(|| a.zero());
    Ok(m * m - (&ac[1] * &l) * m + (&ac[0] * &l) * &l)
}
fn record(
    report: &mut Value,
    path: &Path,
    a: &Algebra,
    label: &str,
    p: &Poly,
    cache: &mut ProjectionCache<'_>,
) -> Result<()> {
    report["active_operation"] = json!(label);
    write(path, report)?;
    let start = Instant::now();
    let factors = cache.factor_basis(&[p.clone()])?;
    ensure!(
        factors == factors_checked(&[p.clone()])?,
        "checked factor reconstruction mismatch"
    );
    report["operations"].as_array_mut().unwrap().push(json!({"label":label,"polynomial":a.format(p),
        "terms":p.nterms(),"degrees":a.names.iter().enumerate().map(|(i,n)|json!([n,p.degree(i)])).collect::<Vec<_>>(),
        "square_free_factors":factors.iter().map(|p|json!({"polynomial":a.format(p),"terms":p.nterms()})).collect::<Vec<_>>(),
        "factor_and_independent_reconstruction_seconds":start.elapsed().as_secs_f64()}));
    write(path, report)
}
fn worker(args: &Args) -> Result<()> {
    let bytes = fs::read(&args.input)?;
    let problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    let normalized = normalize(&problem, &problem.variables)?;
    let algebra = &normalized.algebra;
    let index = |name: &str| problem.variables.iter().position(|v| v == name).unwrap();
    let f = -normalized.positives[0].clone();
    let c = f.replace(index("x4"), &Rational::zero());
    ensure!(c.degree(index("x5")) == 1, "endpoint must be linear in x5");
    let a = c.univariate_lcoeff(index("x5"));
    let b = c.replace(index("x5"), &Rational::zero());
    let expected_a = algebra.parse("(x0+x1+x3+x6)^2-s*x0*(x1+x3)-pp4*x6*(x1+x3)")?;
    let expected_b =
        algebra.parse("(x3+x6)*((x0+x1)^2-s*x0*x1)+(x0+x1)*((x3-x6)^2+(4-pp4)*x3*x6)")?;
    ensure!(
        a == expected_a && b == expected_b,
        "original endpoint identities differ"
    );
    let da = quadratic_discriminant(&a, index("x0"))?;
    ensure!(
        da == algebra.parse("(x1+x3)*(s*(s-4)*(x1+x3)-4*(s-pp4)*x6)")?,
        "D_A identity differs"
    );
    let db = quadratic_discriminant(&b, index("x0"))?;
    ensure!(
        db == algebra.parse(
            "((x3+x6)^2-pp4*x3*x6)^2-2*s*(x3+x6)*x1*((x3+x6)^2-pp4*x3*x6)+s*(s-4)*(x3+x6)^2*x1^2"
        )?,
        "D_B identity differs"
    );
    let start = Instant::now();
    let mut report = json!({"kind":"exact_physical_endpoint_projection_not_completion","input":args.input,
        "input_sha256":format!("{:x}",Sha256::digest(&bytes)),"variables":problem.variables,"original_constraints":problem.constraints,
        "verified_identities":{"A":algebra.format(&a),"B":algebra.format(&b),"D_A":algebra.format(&da),"D_B":algebra.format(&db)},
        "operations":[],"complete":false});
    write(&args.output, &report)?;
    let mut cache = ProjectionCache::new(
        algebra,
        &OptimizedProjectionOptions {
            checkpoint_dir: Some(args.output.with_extension("cache")),
            progress_path: Some(args.output.with_extension("progress.json")),
            capture_dir: Some(args.output.with_extension("capture")),
            ..Default::default()
        },
    );
    for (label, p) in [
        ("lc_x0_A", a.univariate_lcoeff(index("x0"))),
        ("lc_x0_B", b.univariate_lcoeff(index("x0"))),
        ("endpoint_x0_A", a.replace(index("x0"), &Rational::zero())),
        ("endpoint_x0_B", b.replace(index("x0"), &Rational::zero())),
        ("disc_x0_A", da.clone()),
        ("disc_x0_B", db.clone()),
    ] {
        record(&mut report, &args.output, algebra, label, &p, &mut cache)?;
    }
    report["active_operation"] = json!("resultant_x0_A_B");
    write(&args.output, &report)?;
    let r = cache.resultant(&a, &b, index("x0"))?;
    let exact_r = monic_pair_resultant(&a, &b, index("x0"))?;
    ensure!(
        canonical(&exact_r) == r,
        "exact monic remainder/resultant reconstruction differs"
    );
    let p = algebra.parse("(s*x3*x6-(x3+x6)^2)*x1-x3*((x3+x6)^2-pp4*x3*x6)")?;
    let q = algebra.parse(
        "-(s*(x3+x6)^2-s*pp4*x6*(x3+x6)+pp4^2*x6^2)*x1-(s*(x3+x6)-pp4*x6)*((x3+x6)^2-pp4*x3*x6)",
    )?;
    ensure!(
        &p * &q == exact_r,
        "compact resultant factors fail exact reconstruction"
    );
    let u = algebra.parse("s*(s-4)*(x1+x3)-4*(s-pp4)*x6")?;
    report["compact_resultant_factor_identity_verified"] = json!(true);
    report["compact_resultant_factors"] = json!([algebra.format(&p), algebra.format(&q)]);
    record(
        &mut report,
        &args.output,
        algebra,
        "resultant_x0_A_B",
        &exact_r,
        &mut cache,
    )?;
    if exact_r.degree(index("x1")) == 2 {
        let d = cache.discriminant(&exact_r, index("x1"))?;
        record(
            &mut report,
            &args.output,
            algebra,
            "disc_x1_resultant",
            &d,
            &mut cache,
        )?;
        record(
            &mut report,
            &args.output,
            algebra,
            "lc_x1_resultant",
            &exact_r.univariate_lcoeff(index("x1")),
            &mut cache,
        )?;
        record(
            &mut report,
            &args.output,
            algebra,
            "endpoint_x1_resultant",
            &exact_r.replace(index("x1"), &Rational::zero()),
            &mut cache,
        )?;
    }
    let ddb = cache.discriminant(&db, index("x1"))?;
    ensure!(
        quadratic_discriminant(&db, index("x1"))?
            == algebra.parse("16*s*(x3+x6)^2*((x3+x6)^2-pp4*x3*x6)^2")?,
        "nested B discriminant identity differs"
    );
    record(
        &mut report,
        &args.output,
        algebra,
        "disc_x1_disc_x0_B",
        &ddb,
        &mut cache,
    )?;
    let inputs = [("U", u), ("D_B", db), ("P", p), ("Q", q)];
    for (i, (name, p)) in inputs.iter().enumerate() {
        record(
            &mut report,
            &args.output,
            algebra,
            &format!("lc_x1_{name}"),
            &p.univariate_lcoeff(index("x1")),
            &mut cache,
        )?;
        record(
            &mut report,
            &args.output,
            algebra,
            &format!("endpoint_x1_{name}"),
            &p.replace(index("x1"), &Rational::zero()),
            &mut cache,
        )?;
        for (other, q) in &inputs[i + 1..] {
            let label = format!("resultant_x1_{name}_{other}");
            report["active_operation"] = json!(label);
            write(&args.output, &report)?;
            let pair = cache.resultant(p, q, index("x1"))?;
            record(
                &mut report,
                &args.output,
                algebra,
                &label,
                &pair,
                &mut cache,
            )?;
        }
    }
    report["complete"] = json!(true);
    report["active_operation"] = Value::Null;
    report["seconds"] = json!(start.elapsed().as_secs_f64());
    write(&args.output, &report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        return worker(&args);
    }
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--worker")
        .arg("--input")
        .arg(&args.input)
        .arg("--output")
        .arg(&args.output);
    let outcome = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: args.seconds,
            memory_mib: 4096,
            workers: 1,
            max_cells: None,
        },
    )?;
    write(
        &args.output.with_extension("process.json"),
        &serde_json::to_value(&outcome)?,
    )?;
    println!("{}", serde_json::to_string(&outcome)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monic_quadratic_remainder_identity_reconstructs_signed_resultant() -> Result<()> {
        let a = Algebra::new(&["s".into(), "x".into()])?;
        let p = a.parse("x^2+s*x+1")?;
        let q = a.parse("s*x^2-2*x+s+3")?;
        assert_eq!(
            monic_pair_resultant(&p, &q, 1)?,
            symgcad::projection::exact_resultant(&p, &q, 1)?
        );
        assert_eq!(
            quadratic_discriminant(&q, 1)?,
            symgcad::projection::discriminant(&q, 1)?
        );
        Ok(())
    }
}
