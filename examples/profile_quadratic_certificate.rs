//! Research-only scan of completed projection factors for exact no-real-root proofs.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
};

#[derive(Parser)]
struct Args {
    checkpoints: PathBuf,
    #[arg(long)]
    constraint: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 180.0)]
    seconds: f64,
    #[arg(long, default_value_t = 2048)]
    candidates: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct Witness {
    polynomial: String,
    variable: usize,
    leading: DomainCertificate,
    discriminant: DomainCertificate,
}

fn leaf_options() -> CertificateOptions {
    CertificateOptions {
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    }
}

fn coefficient(p: &Poly, variable: usize, degree: u32) -> Poly {
    let mut out = p.zero();
    for (c, powers) in p.coefficients.iter().zip(p.exponents_iter()) {
        if powers[variable] == degree {
            let mut powers = powers.to_vec();
            powers[variable] = 0;
            out.append_monomial(c.clone(), &powers);
        }
    }
    out
}

fn parts(p: &Poly, variable: usize) -> Result<(Poly, Poly, Poly, Poly)> {
    ensure!(
        variable < p.nvars() && p.degree(variable) == 2,
        "not a quadratic axis"
    );
    ensure!(
        p.nterms() <= 128 && (0..p.nvars()).all(|v| p.degree(v) <= 32),
        "candidate exceeds polynomial cap"
    );
    let a = coefficient(p, variable, 2);
    let b = coefficient(p, variable, 1);
    let c = coefficient(p, variable, 0);
    ensure!(
        b.nterms()
            .saturating_mul(b.nterms())
            .saturating_add(a.nterms().saturating_mul(c.nterms()))
            <= 4096,
        "discriminant product estimate exceeds cap"
    );
    let d = &b * &b - (&a * &c).mul_coeff(Rational::from(4));
    ensure!(d.nterms() <= 4096, "discriminant exceeds term cap");
    Ok((a, b, c, d))
}

fn verify(algebra: &Algebra, constraints: &[Poly], p: &Poly, witness: &Witness) -> Result<()> {
    ensure!(
        algebra.parse(&witness.polynomial)? == *p,
        "different quadratic"
    );
    let (a, b, c, d) = parts(p, witness.variable)?;
    let z = algebra.parse(&algebra.names[witness.variable])?;
    ensure!(
        &(&a * &z) * &z + &b * &z + c == *p,
        "quadratic reconstruction failed"
    );
    ensure!(
        a.degree(witness.variable) == 0 && d.degree(witness.variable) == 0,
        "coefficient proof depends on its eliminated axis"
    );
    verify_certificate(algebra, &a, constraints, &[], &witness.leading)?;
    verify_certificate(algebra, &d, constraints, &[], &witness.discriminant)?;
    ensure!(
        witness.discriminant.sign == -1,
        "discriminant is not strictly negative"
    );
    Ok(())
}

fn discover(
    algebra: &Algebra,
    constraints: &[Poly],
    p: &Poly,
    variable: usize,
) -> Result<Option<Witness>> {
    let Ok((a, _, _, d)) = parts(p, variable) else {
        return Ok(None);
    };
    let domain = derive_domain(algebra, constraints, &[])?;
    let options = leaf_options();
    // These are ordinary existing proofs. This helper never calls itself
    // recursively, adds assumptions, or accepts a sampled sign assertion.
    let Some(leading) = certify_nonzero(algebra, &a, &domain, &options)? else {
        return Ok(None);
    };
    let Some(discriminant) = certify_nonzero(algebra, &d, &domain, &options)? else {
        return Ok(None);
    };
    if discriminant.sign != -1 {
        return Ok(None);
    }
    let witness = Witness {
        polynomial: algebra.format(p),
        variable,
        leading,
        discriminant,
    };
    verify(algebra, constraints, p, &witness)?;
    Ok(Some(witness))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 300.0,
        "time cap must be within300s"
    );
    ensure!(
        args.candidates > 0 && args.candidates <= 4096,
        "candidate cap must be within4096"
    );
    let started = Instant::now();
    let mut paths = std::fs::read_dir(&args.checkpoints)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut names: Option<Vec<String>> = None;
    let mut inventory: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in paths {
        let data: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let variables: Vec<String> = serde_json::from_value(data["variable_names"].clone())?;
        if let Some(old) = &names {
            ensure!(*old == variables, "checkpoint variable maps differ");
        } else {
            names = Some(variables);
        }
        for polynomial in data["output"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing checkpoint output"))?
        {
            inventory
                .entry(
                    polynomial
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("non-string polynomial"))?
                        .into(),
                )
                .or_default()
                .push(path.display().to_string());
        }
    }
    let algebra = Algebra::new(&names.ok_or_else(|| anyhow::anyhow!("no checkpoints"))?)?;
    let constraints = args
        .constraint
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&algebra, &constraints, &[])?;
    ensure!(!domain.empty, "empty original box");
    let mut candidates = inventory
        .iter()
        .map(|(s, paths)| Ok((algebra.parse(s)?, paths.clone())))
        .collect::<Result<Vec<_>>>()?;
    let total_inventory = candidates.len();
    candidates.retain(|(p, _)| {
        p.nterms() <= 128
            && (0..p.nvars()).filter(|&v| p.degree(v) > 0).count() > 1
            && (0..p.nvars()).any(|v| p.degree(v) == 2)
    });
    candidates.sort_by_key(|(p, _)| p.nterms());
    let eligible = candidates.len();
    let mut attempts = 0;
    let mut current_proofs = 0;
    let mut new_proofs = Vec::new();
    let mut existing_seconds = 0.0;
    let mut candidate_seconds = 0.0;
    for (p, paths) in candidates {
        if attempts >= args.candidates || started.elapsed().as_secs_f64() >= args.seconds {
            break;
        }
        let then = Instant::now();
        let current = certify_nonzero(&algebra, &p, &domain, &leaf_options())?;
        existing_seconds += then.elapsed().as_secs_f64();
        if current.is_some() {
            current_proofs += 1;
            continue;
        }
        for variable in 0..algebra.names.len() {
            if p.degree(variable) != 2 {
                continue;
            }
            if attempts >= args.candidates || started.elapsed().as_secs_f64() >= args.seconds {
                break;
            }
            attempts += 1;
            let then = Instant::now();
            let witness = discover(&algebra, &constraints, &p, variable)?;
            let elapsed = then.elapsed().as_secs_f64();
            candidate_seconds += elapsed;
            if let Some(witness) = witness {
                new_proofs.push(json!({"witness": witness, "checkpoints":paths, "seconds":elapsed, "verified":true}));
                break;
            }
        }
    }
    let report = json!({"kind":"research_only_quadratic_nonvanishing", "checkpoints":args.checkpoints,
        "constraints":args.constraint,"domain":domain,"unique_factor_inventory":total_inventory,
        "eligible_multivariate_factors":eligible,"candidate_axis_attempts":attempts,
        "existing_certificates":current_proofs,"additional_certificates":new_proofs,
        "existing_portfolio_seconds":existing_seconds,"candidate_seconds":candidate_seconds,
        "wall_seconds":started.elapsed().as_secs_f64(),"limits":{"seconds":args.seconds,"candidates":args.candidates,
        "max_terms":128,"max_input_axis_degree":32,"product_estimate":4096,"recursive_discovery":false},
        "coverage_claim":"none; independently certified nonvanishing candidates only, no projection obligations were deleted"});
    std::fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{}",
        json!({"additional_certificates":report["additional_certificates"].as_array().unwrap().len(),
        "eligible_multivariate_factors":eligible,"candidate_axis_attempts":attempts,"wall_seconds":report["wall_seconds"]})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Algebra, Vec<Poly>) {
        let a = Algebra::new(&["m".into(), "x".into(), "y".into()]).unwrap();
        let c = ["m", "4-m", "x", "y"]
            .iter()
            .map(|p| a.parse(p).unwrap())
            .collect();
        (a, c)
    }
    #[test]
    fn physical_endpoint_has_exact_independent_leading_and_negative_discriminant_proofs() {
        let (a, c) = setup();
        for expression in ["(x+y+1)^2-m*(x+y)", "-((x+y+1)^2-m*(x+y))"] {
            let p = a.parse(expression).unwrap();
            assert!(
                certify_nonzero(
                    &a,
                    &p,
                    &derive_domain(&a, &c, &[]).unwrap(),
                    &leaf_options()
                )
                .unwrap()
                .is_none()
            );
            let witness = discover(&a, &c, &p, 1).unwrap().unwrap();
            assert_eq!(
                a.parse(&witness.discriminant.polynomial).unwrap(),
                a.parse("m*(m-4)").unwrap()
            );
            verify(&a, &c, &p, &witness).unwrap();
        }
    }
    #[test]
    fn supplied_proofs_reject_changed_axis_polynomial_discriminant_sign_and_domain() {
        let (a, c) = setup();
        let p = a.parse("(x+y+1)^2-m*(x+y)").unwrap();
        let good = discover(&a, &c, &p, 1).unwrap().unwrap();
        let mut wrong = good.clone();
        wrong.variable = 0;
        assert!(verify(&a, &c, &p, &wrong).is_err());
        let mut wrong = good.clone();
        wrong.discriminant.sign = 1;
        assert!(verify(&a, &c, &p, &wrong).is_err());
        let mut wrong = good.clone();
        wrong.leading.polynomial = "2".into();
        assert!(verify(&a, &c, &p, &wrong).is_err());
        assert!(verify(&a, &c, &a.parse("x^2-1").unwrap(), &good).is_err());
        assert!(verify(&a, &c[2..], &p, &good).is_err());
    }
    #[test]
    fn zeros_degree_drops_and_nonquadratics_decline() {
        let (a, c) = setup();
        for expression in ["(x-y)^2", "x^2-y", "m*x+y", "x^3+y"] {
            assert!(
                discover(&a, &c, &a.parse(expression).unwrap(), 1)
                    .unwrap()
                    .is_none()
            );
        }
    }
}
