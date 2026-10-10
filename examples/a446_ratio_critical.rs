//! Research only: exact fixed-parent ratio coordinates for the A446 common-root arc.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::algebra::{Algebra, Poly};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 2)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
struct Geometry {
    algebra: Algebra,
    g: Poly,
    n: Poly,
    d: Poly,
    critical: Poly,
    jacobian_cleared: Poly,
}
fn geometry() -> Result<Geometry> {
    let algebra =
        Algebra::new(&["p", "q", "r", "w", "u", "h", "b", "x", "y", "s"].map(String::from))?;
    let parse = |s: &str| algebra.parse(s);
    let a0 = parse("(p+q+r+w+x+y)^2-(5/18)*(x+y)*(q+w)-u*(p+r)*(q+w)")?;
    let original_a = &a0 - &parse("s*x*y")?;
    let original_b = parse(
        "(p+q+r+w+x+y)*(p+q+x)*(r+w+y)-u*((r+w+y)*p*q+(p+q+x)*r*w)-(5/18)*((r+w+y)*x*q+(p+q+x)*y*w)",
    )?;
    let jac = &original_a.derivative(7) * &original_b.derivative(8)
        - &original_a.derivative(8) * &original_b.derivative(7);
    ensure!(jac.degree(9) <= 1, "Jacobian is affine in s");
    let cleared = &parse("x*y")? * &jac.replace(9, &Rational::zero()) + &a0 * &jac.derivative(9);
    let map = |f: &Poly| -> Result<Poly> {
        let f = f.replace_with_poly(7, &parse("h*b-p-q")?);
        Ok(f.replace_with_poly(8, &parse("b-r-w")?))
    };
    let g = parse("h*(1+h)*b^2-(5/18)*h*(q+w)*b+(5/18)*(p+q)*q-u*p*q+h*((5/18)*(r+w)*w-u*r*w)")?;
    ensure!(
        map(&original_b)? == &parse("b")? * &g,
        "exact original B=b*G"
    );
    let n = map(&a0)?;
    let d = parse("(h*b-p-q)*(b-r-w)")?;
    ensure!(
        d == map(&parse("x*y")?)?,
        "positive original fiber denominator"
    );
    ensure!(
        map(&original_a)? == &n - &(&parse("s")? * &d),
        "exact A=N-sD"
    );
    let nhd = &n.derivative(5) * &d - &n * &d.derivative(5);
    let nbd = &n.derivative(6) * &d - &n * &d.derivative(6);
    let critical = &nhd * &g.derivative(6) - &nbd * &g.derivative(5);
    let jacobian_cleared = map(&cleared)?;
    ensure!(
        &parse("b")? * &(&jacobian_cleared - &critical) == &nhd * &g,
        "exact original Jacobian identity modulo G"
    );
    let gb = g.derivative(6);
    ensure!(
        gb == parse("h*(2*(1+h)*b-(5/18)*(q+w))")?,
        "exact positive branch derivative"
    );
    ensure!(
        n.degree(6) == 2 && d.degree(6) == 2 && g.degree(6) == 2,
        "quadratic ratio equations"
    );
    Ok(Geometry {
        algebra,
        g,
        n,
        d,
        critical,
        jacobian_cleared,
    })
}
fn stats(a: &Algebra, p: &Poly) -> Value {
    json!({"polynomial":a.format(p),"terms":p.nterms(),"degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),"coefficient_bits":p.coefficients.iter().map(|c|c.numerator_ref().significant_bits().max(c.denominator_ref().significant_bits())).max().unwrap_or(0)})
}
fn save(path: &PathBuf, report: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(report)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}
fn pseudo_remainder(f: &Poly, g: &Poly, axis: usize) -> Result<(Poly, Poly, Poly)> {
    let lc = g.univariate_lcoeff(axis);
    let mut scale = f.one();
    let mut quotient = f.zero();
    let mut remainder = f.clone();
    let mut steps = 0;
    while !remainder.is_zero() && remainder.degree(axis) >= g.degree(axis) {
        let old = remainder.degree(axis);
        let mut exponents = vec![0; f.nvars()];
        exponents[axis] = old - g.degree(axis);
        let term = remainder
            .univariate_lcoeff(axis)
            .mul_monomial(&Rational::one(), &exponents);
        ensure!(
            lc.nterms().saturating_mul(remainder.nterms()) <= 1_000_000
                && term.nterms().saturating_mul(g.nterms()) <= 1_000_000,
            "pseudo-division product bound"
        );
        quotient = &lc * &quotient + term.clone();
        remainder = &lc * &remainder - &term * g;
        scale = &scale * &lc;
        ensure!(
            remainder.is_zero() || remainder.degree(axis) < old,
            "degree descent"
        );
        steps += 1;
        ensure!(
            steps <= 8 && remainder.nterms() <= 65536 && quotient.nterms() <= 65536,
            "pseudo-division bound"
        );
    }
    ensure!(
        &scale * f == &quotient * g + remainder.clone(),
        "exact pseudo-division reconstruction"
    );
    Ok((scale, quotient, remainder))
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "bounded60seconds"
    );
    ensure!(
        !args.output.exists() && !args.output.with_extension("process.json").exists(),
        "fresh output"
    );
    if !args.worker {
        let mut c = Command::new("taskset");
        c.args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .arg("--worker")
            .arg("--output")
            .arg(&args.output)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        let outcome = symgcad::benchmark::run_bounded(
            &mut c,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 8192,
                workers: 1,
                max_cells: None,
            },
        )?;
        return save(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(outcome)?,
        );
    }
    let started = Instant::now();
    let g = geometry()?;
    let a = &g.algebra;
    let mut report = json!({"research_only":true,"original_completion_claim":false,"variables":a.names,"identities_checked":true,"map":{"x":"h*b-p-q","y":"b-r-w","inverse_h":"(p+q+x)/(r+w+y)","inverse_b":"r+w+y"},"fixed_parent":["p","q","r","w","u"],"admissibility":["p,q,r,w,h>0","b>r+w","h*b>p+q"],"G":stats(a,&g.g),"N":stats(a,&g.n),"D":stats(a,&g.d),"critical":stats(a,&g.critical),"original_cleared_jacobian":stats(a,&g.jacobian_cleared),"stage":"identities_complete","elapsed_seconds":started.elapsed().as_secs_f64()});
    save(&args.output, &report)?;
    let (scale, quotient, remainder) = pseudo_remainder(&g.critical, &g.g, 6)?;
    report["pseudo_remainder"] = json!({"identity_checked":true,"scale":stats(a,&scale),"quotient":stats(a,&quotient),"remainder":stats(a,&remainder)});
    report["stage"] = json!("remainder_complete");
    save(&args.output, &report)?;
    ensure!(remainder.degree(6) <= 1, "linear remainder");
    let aa = g.g.univariate_lcoeff(6);
    let bb = g.g.derivative(6).replace(6, &Rational::zero());
    let cc = g.g.replace(6, &Rational::zero());
    let l = remainder.derivative(6);
    let m = remainder.replace(6, &Rational::zero());
    ensure!(
        l.nterms()
            .saturating_mul(m.nterms())
            .saturating_mul(bb.nterms())
            <= 5_000_000,
        "resultant product estimate"
    );
    let resultant = &aa * &m.pow(2) - &bb * &l * &m + &cc * &l.pow(2);
    ensure!(resultant.nterms() <= 131072, "resultant output bound");
    let direct = symgcad::projection::exact_resultant(&g.g, &remainder, 6)?;
    ensure!(
        resultant == direct,
        "independent exact quadratic-linear resultant"
    );
    report["critical_resultant"] = stats(a, &resultant);
    report["resultant_identity_checked"] = json!(true);
    // Removing only coordinate h factors keeps all non-coordinate factors explicit.
    let h = a.parse("h")?;
    let mut reduced = resultant.clone();
    let mut h_power = 0;
    while let Some(q) = reduced.try_div(&h) {
        ensure!(&q * &h == reduced, "positive h factor");
        reduced = q;
        h_power += 1;
        ensure!(h_power <= 64, "factor bound");
    }
    let one_h = a.parse("1+h")?;
    let mut one_h_power = 0;
    while let Some(q) = reduced.try_div(&one_h) {
        ensure!(&q * &one_h == reduced, "positive 1+h factor");
        reduced = q;
        one_h_power += 1;
        ensure!(one_h_power <= 64, "factor bound");
    }
    ensure!(
        &h.pow(h_power) * &one_h.pow(one_h_power) * &reduced == resultant,
        "stripped positive factors reconstructed"
    );
    report["positive_factors_removed"] = json!({"h":h_power,"1+h":one_h_power});
    report["reduced_critical_resultant"] = stats(a, &reduced);
    report["stage"] = json!("complete");
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    report["source_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(include_bytes!("a446_ratio_critical.rs"))
    ));
    save(&args.output, &report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_equations_and_jacobian_reconstruct() -> Result<()> {
        let g = geometry()?;
        ensure!(
            g.critical.degree(6) >= 2 && !g.critical.is_zero(),
            "nontrivial fixed-parent critical polynomial"
        );
        Ok(())
    }
    #[test]
    fn pseudo_division_preserves_the_ideal_identity() -> Result<()> {
        let a = Algebra::new(&["a".into(), "b".into()])?;
        let f = a.parse("b^4+3*b+1")?;
        let g = a.parse("a*b^2+b+2")?;
        let (s, q, r) = pseudo_remainder(&f, &g, 1)?;
        ensure!(r.degree(1) < 2 && &s * &f == &q * &g + r, "exact remainder");
        Ok(())
    }
}
