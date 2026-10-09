//! Inspect exact monotone coefficient certificates without any projection.
use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::Poly,
    domain::{CertificateOptions, certify_nonzero, derive_domain},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    /// Original-coordinate strict restrictions for research-only piece probes.
    #[arg(long)]
    extra_constraint: Vec<String>,
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
