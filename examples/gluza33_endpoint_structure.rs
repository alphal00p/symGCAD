//! Exact research identities for the massless-coordinate endpoint of Gluza33.
//! No coordinate transformation or original decomposition is claimed.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{Limits, Problem, algebra::Poly, problem::normalize};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Optional original massive coordinate whose endpoint discriminant is factored.
    #[arg(long)]
    axis: Option<String>,
    #[arg(long, hide = true)]
    worker: bool,
}

fn coordinate(p: &Poly) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter()
            .next()
            .is_some_and(|e| e.iter().map(|&x| u64::from(x)).sum::<u64>() == 1)
}

fn identities(problem: &Problem) -> Result<Value> {
    let n = normalize(problem, &problem.variables)?;
    let a = &n.algebra;
    ensure!(problem.split.is_empty());
    let targets: Vec<_> = n.positives.iter().filter(|p| !coordinate(p)).collect();
    ensure!(targets.len() == 1, "one strict positive target required");
    for name in ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"] {
        let axis = a
            .names
            .iter()
            .position(|v| v == name)
            .context("missing original coordinate")?;
        ensure!(
            n.positives
                .iter()
                .any(|p| coordinate(p) && p.degree(axis) == 1)
        );
    }
    let i3 = a.names.iter().position(|v| v == "x3").unwrap();
    let i5 = a.names.iter().position(|v| v == "x5").unwrap();
    let endpoint = targets[0]
        .replace(i3, &Rational::zero())
        .replace(i5, &Rational::zero());
    let big_a = a.parse("x0+x6")?;
    let big_d = a.parse("x4+x7")?;
    let s = &big_a + &big_d;
    let b = a.parse("x1+x2")?;
    let k = a.parse("x6*x4-x0*x7")?;
    let p = &(&big_a * &big_d) * &s - &a.parse("10")? * &a.parse("x0*x6*(x4+x7)+x4*x7*(x0+x6)")?;
    let r = a.parse("(x0+x4)^2-8*(x0+x4)*(x6+x7)+(x6+x7)^2")?;
    ensure!(
        &s * &p - &a.parse("10")? * &k.pow(2) == &(&big_a * &big_d) * &r,
        "compact determinant identity failed"
    );
    let reconstructed =
        &(&big_a * &big_d) * &b.pow(2) + &p * &b + &a.parse("x1*x2")? * &(&(&s * &b) + &r);
    ensure!(
        reconstructed == endpoint,
        "compact endpoint identity failed"
    );
    let alpha = a.parse("x1")?;
    let x2 = a.parse("x2")?;
    let ad = &big_a * &big_d;
    let leading = &ad + &(&s * &alpha);
    let middle =
        &(&(&ad * &alpha).mul_coeff(Rational::from(2)) + &p) + &(&s * &alpha.pow(2)) + &r * &alpha;
    let constant = &alpha * &(&(&ad * &alpha) + &p);
    ensure!(
        &(&leading * &x2.pow(2) + &middle * &x2) + &constant == endpoint,
        "original x2 quadratic identity failed"
    );
    let boundary_factor = &ad.pow(2) - &(&a.parse("10")? * &k.pow(2));
    ensure!(
        &ad * &middle - &alpha * &boundary_factor == &leading * &(&p + &(&ad * &alpha)),
        "zero-endpoint linear-coefficient identity failed"
    );
    let inner_axis = a.names.iter().position(|v| v == "x2").unwrap();
    let outer_axis = a.names.iter().position(|v| v == "x1").unwrap();
    let inner_discriminant = symgcad::projection::discriminant(&endpoint, inner_axis)?;
    let q_alpha = &(&(&s * &alpha.pow(2)) + &(&r * &alpha)) + &p;
    ensure!(
        inner_discriminant == &q_alpha.pow(2) - &(&a.parse("40")? * &(&k.pow(2) * &alpha.pow(2))),
        "nested first discriminant identity failed"
    );
    let outer_discriminant = symgcad::projection::discriminant(&inner_discriminant, outer_axis)?;
    let final_factor =
        (&r - &ad.mul_coeff(Rational::from(4))).pow(2) - &a.parse("160")? * &k.pow(2);
    let expected_outer = &(&(&(&(&a.parse("25600")? * &k.pow(4)) * &s.pow(2)) * &p.pow(2))
        * &r.pow(2))
        * &final_factor;
    ensure!(
        outer_discriminant == expected_outer,
        "nested quartic discriminant factor identity failed"
    );
    let mut extended = a.names.clone();
    extended.extend(["rho".into(), "tau".into()]);
    let formal = symgcad::algebra::Algebra::new(&extended)?;
    let rho = formal.parse("rho*(1-rho)")?;
    let tau = formal.parse("tau")?;
    let aa = formal.parse(&a.format(&big_a))?;
    let dd = formal.parse(&a.format(&big_d))?;
    let ss = formal.parse(&a.format(&s))?;
    let rr = formal.parse(&a.format(&r))?;
    let pp = formal.parse(&a.format(&p))?;
    let kk = formal.parse(&a.format(&k))?;
    let q = &(&(&rho * &ss) * &tau.pow(2) + &(&aa * &dd + &rho * &rr) * &tau) + &pp;
    let i1 = a.names.iter().position(|v| v == "x1").unwrap();
    let i2 = a.names.iter().position(|v| v == "x2").unwrap();
    let substituted = formal
        .parse(&a.format(&endpoint))?
        .replace_with_poly(i1, &formal.parse("rho*tau")?)
        .replace_with_poly(i2, &formal.parse("(1-rho)*tau")?);
    ensure!(substituted == &tau * &q, "exact ratio substitution failed");
    let disc = symgcad::projection::discriminant(&q, extended.len() - 1)?;
    let compact_disc =
        (&aa * &dd - &rho * &rr).pow(2) - &formal.parse("40")? * &(&rho * &kk.pow(2));
    ensure!(
        disc == compact_disc,
        "formal ratio discriminant identity failed"
    );
    Ok(
        json!({"endpoint":a.format(&endpoint),"endpoint_terms":endpoint.nterms(),
        "A":a.format(&big_a),"D":a.format(&big_d),"S":a.format(&s),"B":a.format(&b),
        "K":a.format(&k),"P":a.format(&p),"R":a.format(&r),
        "compact_identity":"d=A*D*B^2+P*B+x1*x2*(S*B+R)",
        "determinant_identity":"S*P-10*K^2=A*D*R",
        "ratio_identity":"x1=rho*B,x2=(1-rho)*B: d/B=rho*(1-rho)*S*B^2+(A*D+rho*(1-rho)*R)*B+P",
        "ratio_discriminant":"(A*D-rho*(1-rho)*R)^2-40*rho*(1-rho)*K^2",
        "original_quadratic":{"axis":"x2","alpha":"x1","leading":a.format(&leading),"middle":a.format(&middle),"constant":a.format(&constant),"leading_strict_positive_coefficients":leading.coefficients.iter().all(|c|c>&Rational::zero()),"endpoint_boundary_identity":"AD*M-alpha*((AD)^2-10K^2)=(AD+S*alpha)*(P+AD*alpha)","endpoint_boundary_factor":a.format(&boundary_factor)},
        "discriminant_ladder":{"first":"Disc_x2(d)=(S*x1^2+R*x1+P)^2-40*K^2*x1^2","first_terms":inner_discriminant.nterms(),"second":"Disc_x1(Disc_x2(d))=25600*K^4*S^2*P^2*R^2*((R-4AD)^2-160K^2)","second_terms":outer_discriminant.nterms(),"final_factor":a.format(&final_factor),"final_factor_terms":final_factor.nterms(),"exact_reconstruction":true},
        "exact_identities_verified":true,"ratio_domain":"B>0,0<rho<1",
        "original_coordinate_transport_implemented":false,"original_completion":false}),
    )
}

fn run(args: &Args) -> Result<()> {
    let source = fs::read(&args.input)?;
    let problem = Problem::from_toml(std::str::from_utf8(&source)?)?;
    let started = Instant::now();
    let mut result = identities(&problem)?;
    fs::write(
        args.output.join("identities.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    if let Some(name) = &args.axis {
        ensure!(["x0", "x1", "x2", "x4", "x6", "x7"].contains(&name.as_str()));
        let n = normalize(&problem, &problem.variables)?;
        let a = &n.algebra;
        let axis = a
            .names
            .iter()
            .position(|v| v == name)
            .context("unknown axis")?;
        let endpoint = a.parse(result["endpoint"].as_str().unwrap())?;
        ensure!(
            endpoint.degree(axis) == 2,
            "endpoint is not quadratic in axis"
        );
        let t = Instant::now();
        let disc = symgcad::projection::discriminant(&endpoint, axis)?;
        fs::write(
            args.output.join("discriminant.json"),
            serde_json::to_vec_pretty(
                &json!({"axis":name,"polynomial":a.format(&disc),"terms":disc.nterms()}),
            )?,
        )?;
        let factors = disc.factor();
        let rebuilt = factors
            .iter()
            .fold(disc.one(), |acc, (f, m)| &acc * &f.pow(*m));
        ensure!(rebuilt == disc, "discriminant factor reconstruction failed");
        result["discriminant"] = json!({"axis":name,"terms":disc.nterms(),"factors":factors.iter().map(|(f,m)|json!({"polynomial":a.format(f),"multiplicity":m,"terms":f.nterms(),"degrees":(0..a.names.len()).map(|i|f.degree(i)).collect::<Vec<_>>()})).collect::<Vec<_>>(),"factor_identity_verified":true,"elapsed_seconds":t.elapsed().as_secs_f64()});
    }
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(source)),"elapsed_seconds":started.elapsed().as_secs_f64(),"result":result}),
        )?,
    )?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        return run(&args);
    }
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&args.output).context("output must be fresh")?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(&args.input)
        .arg("--output")
        .arg(&args.output)
        .arg("--worker");
    if let Some(axis) = &args.axis {
        command.arg("--axis").arg(axis);
    }
    let outcome = symgcad::benchmark::run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 60.,
            memory_mib: 4096,
            workers: 1,
            max_cells: None,
        },
    )?;
    fs::write(
        args.output.join("process.json"),
        serde_json::to_vec_pretty(&outcome)?,
    )?;
    ensure!(
        outcome.status == "completed" && outcome.exit_code == Some(0),
        "endpoint research failed: {} {}",
        outcome.status,
        outcome.stderr
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_endpoint_and_ratio_discriminant_identities() -> Result<()> {
        let mut p = Problem::from_toml(include_str!("../tests/fixtures/gluza33_negative.toml"))?;
        p.constraints[0] = p.constraints[0].replacen("< 0", "> 0", 1);
        let result = identities(&p)?;
        assert_eq!(result["exact_identities_verified"], true);
        assert_eq!(result["original_completion"], false);
        Ok(())
    }
    #[test]
    fn changed_endpoint_or_missing_orthant_is_rejected() -> Result<()> {
        let mut p = Problem::from_toml(include_str!("../tests/fixtures/gluza33_negative.toml"))?;
        p.constraints[0] = p.constraints[0].replacen("< 0", "> 0", 1);
        p.constraints.push("x0+1>0".into());
        assert!(identities(&p).is_err());
        p.constraints.pop();
        p.constraints.remove(1);
        assert!(identities(&p).is_err());
        Ok(())
    }
}
