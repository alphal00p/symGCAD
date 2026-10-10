//! Exact research on positive common-root critical points of the A446 ladder.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    roots::{FiberRoot, isolate_union, refine_once, roots_in_open_interval},
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
fn sign(c: &Rational) -> i8 {
    if c.is_zero() {
        0
    } else if c.is_negative() {
        -1
    } else {
        1
    }
}
fn signs_at_root(root: &mut FiberRoot, guards: &[UnivariatePolynomial<Q>]) -> Result<Vec<i8>> {
    let gcds = guards.iter().map(|p| root.fiber.gcd(p)).collect::<Vec<_>>();
    for _ in 0..4096 {
        let (lo, hi) = &root.interval;
        let mut signs = Vec::new();
        let mut ready = true;
        for (p, gcd) in guards.iter().zip(&gcds) {
            if lo == hi {
                signs.push(sign(&p.evaluate(lo)));
                continue;
            }
            if roots_in_open_interval(gcd, lo, hi) > 0 {
                signs.push(0);
                continue;
            }
            let left = p.evaluate(lo);
            let right = p.evaluate(hi);
            if left.is_zero() || right.is_zero() || roots_in_open_interval(p, lo, hi) > 0 {
                ready = false;
                break;
            }
            ensure!(sign(&left) == sign(&right), "guard interval sign");
            signs.push(sign(&left));
        }
        if ready {
            return Ok(signs);
        }
        refine_once(root);
    }
    anyhow::bail!("critical guard refinement budget")
}
struct Structure {
    algebra: Algebra,
    numerator: Poly,
    guards: Vec<Poly>,
    s_num: Poly,
    s_den: Poly,
    u_num: Poly,
    u_den: Poly,
    original_a: Poly,
    original_b: Poly,
    factored_s: Poly,
    chart_u_numerator: Poly,
    chart_u_denominator: Poly,
}
fn structure() -> Result<Structure> {
    let algebra =
        Algebra::new(&["s", "u", "x1", "x3", "x5", "x7", "x2", "x4", "h", "b"].map(String::from))?;
    let p = |s: &str| algebra.parse(s);
    let a0 = p("(x1+x3+x5+x7+x2+x4)^2-(5/18)*(x2+x4)*(x3+x7)")?;
    let t = p("(x4+x5+x7)*x1*x3+(x2+x1+x3)*x5*x7")?;
    let e =
        p("(x1+x3+x5+x7+x2+x4)*(x1+x3+x2)*(x5+x7+x4)-(5/18)*((x4+x5+x7)*x2*x3+(x2+x1+x3)*x4*x7)")?;
    let xy = p("(x1+x5)*(x3+x7)")?;
    let w = p("x2*x4")?;
    let sn = &a0 * &t - &xy * &e;
    let sd = &w * &t;
    let factored_s = p(
        "(x3*(x4+x5+x7)-x7*(x2+x1+x3))*((x1+x3+x5+x7+x2+x4)*(x1*(x4+x5+x7)-x5*(x2+x1+x3))+(5/18)*(x3+x7)*(x5*x2-x1*x4))",
    )?;
    ensure!(factored_s == sn, "factored s numerator identity");
    let chart = |poly: &Poly| -> Result<Poly> {
        Ok(poly
            .replace_with_poly(6, &p("h*b-x1-x3")?)
            .replace_with_poly(7, &p("b-x5-x7")?))
    };
    let chart_un = p("h*b*((h+1)*b-(5/18)*(x3+x7))+(5/18)*((x1+x3)*x3+h*(x5+x7)*x7)")?;
    let chart_ud = p("x1*x3+h*x5*x7")?;
    ensure!(
        chart(&e)? == &p("b")? * &chart_un && chart(&t)? == &p("b")? * &chart_ud,
        "positive ratio chart u identity"
    );
    ensure!(
        chart_un.derivative(9) == p("h*(2*(h+1)*b-(5/18)*(x3+x7))")?,
        "strict scale derivative identity"
    );
    let nx = &e.derivative(6) * &t - &e * &t.derivative(6);
    let ny = &e.derivative(7) * &t - &e * &t.derivative(7);
    let nx_positive = p(
        "(x4+x5+x7)*((x4+x5+x7)*x1*x3*(2*(x2+x1+x3)+(x4+x5+x7)-(5/18)*(x3+x7))+x5*x7*((x2+x1+x3)^2-(5/18)*x3^2)+(5/18)*x1*x7^2*x3)",
    )?;
    let ny_positive = p(
        "(x2+x1+x3)*((x2+x1+x3)*x5*x7*((x2+x1+x3)+2*(x4+x5+x7)-(5/18)*(x3+x7))+x1*x3*((x4+x5+x7)^2-(5/18)*x7^2)+(5/18)*x5*x7*x3^2)",
    )?;
    ensure!(
        nx == nx_positive && ny == ny_positive,
        "positive coordinate derivative identities"
    );
    ensure!(
        nx.coefficients
            .iter()
            .chain(&ny.coefficients)
            .all(|c| *c > Rational::zero()),
        "u increases in each positive coordinate"
    );
    let numerator = &(&p("x2")? * &a0.derivative(6) * &t - sn.clone()) * &p("x4")? * &ny
        - &(&p("x4")? * &a0.derivative(7) * &t - sn.clone()) * &p("x2")? * &nx;
    let aa = a0.clone() - &p("s")? * &w - &p("u")? * &xy;
    let bb = e.clone() - &p("u")? * &t;
    let jac = &aa.derivative(6) * &bb.derivative(7) - &aa.derivative(7) * &bb.derivative(6);
    ensure!(
        jac.degree(0) <= 1 && jac.degree(1) <= 1,
        "bilinear kinematic Jacobian"
    );
    let mut reconstructed = numerator.zero();
    for (cp, i) in jac.to_univariate_polynomial_list(0) {
        for (cq, j) in cp.to_univariate_polynomial_list(1) {
            let term = &cq
                * &sn.pow(i as usize)
                * &sd.pow((1 - i) as usize)
                * &e.pow(j as usize)
                * &t.pow((1 - j) as usize);
            reconstructed = &reconstructed + &term;
        }
    }
    ensure!(
        reconstructed == numerator,
        "cleared exact Jacobian identity"
    );
    ensure!(
        t.coefficients.iter().all(|c| *c > Rational::zero()),
        "strict denominator positivity"
    );
    let guards = vec![
        &sn - &sd.clone().mul_coeff(Rational::from(4)),
        &e - &t.clone().mul_coeff(Rational::from(4)),
        &xy * &e - &p("(x1+x3+x5+x7)^2")? * &t,
        sn.clone(),
    ];
    Ok(Structure {
        algebra,
        numerator,
        guards,
        s_num: sn,
        s_den: sd,
        u_num: e,
        u_den: t,
        original_a: aa,
        original_b: bb,
        factored_s,
        chart_u_numerator: chart_un,
        chart_u_denominator: chart_ud,
    })
}
fn save(path: &PathBuf, v: &Value) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 120.0 && args.seconds.is_finite(),
        "budget"
    );
    ensure!(!args.output.exists(), "fresh output");
    if !args.worker {
        let mut cmd = Command::new("taskset");
        cmd.args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .arg("--worker")
            .arg("--output")
            .arg(&args.output)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        let outcome = symgcad::benchmark::run_bounded(
            &mut cmd,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        save(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(outcome)?,
        )?;
        return Ok(());
    }
    let started = Instant::now();
    let s = structure()?;
    let a = &s.algebra;
    let original_bytes = fs::read("artifacts/research/a446-kinematic-partition-01/report.json")?;
    let original_hash = format!("{:x}", Sha256::digest(&original_bytes));
    ensure!(
        original_hash == "9adaac7df7cde50989684002f8c5a93e225edfc95885c91f8f48f1cb426506b8",
        "original identity report checksum"
    );
    let original: Value = serde_json::from_slice(&original_bytes)?;
    ensure!(
        a.parse(original["A"].as_str().unwrap())? == s.original_a
            && a.parse(original["B"].as_str().unwrap())? == s.original_b,
        "original A/B identity"
    );
    let mut report = json!({"research_only":true,"coverage_claim":false,"original_identity_report_hash":original_hash,"variables":a.names,"jacobian_numerator":a.format(&s.numerator),"terms":s.numerator.nterms(),"degrees":(0..a.names.len()).map(|i|s.numerator.degree(i)).collect::<Vec<_>>(),"s_numerator":a.format(&s.s_num),"factored_s_numerator":a.format(&s.factored_s),"s_denominator":a.format(&s.s_den),"u_numerator":a.format(&s.u_num),"u_denominator":a.format(&s.u_den),"chart_u_numerator":a.format(&s.chart_u_numerator),"chart_u_denominator":a.format(&s.chart_u_denominator),"chart_conditions":"h>0,b>max(x5+x7,(x1+x3)/h); then du/db>0","strict_guards":s.guards.iter().map(|p|a.format(p)).collect::<Vec<_>>(),"stage":"identities_checked","rows":[]});
    save(&args.output, &report)?;
    let mut rows = Vec::new();
    let mut fibers = 0usize;
    let mut identically_zero = 0usize;
    let coordinates = [
        ["1", "1", "1", "1"],
        ["1/1000", "5", "1", "1"],
        ["1/1000", "10", "1", "1"],
        ["1/1000", "100", "1", "1"],
        ["1", "2", "3", "4"],
        ["1/10", "10", "1/10", "10"],
        ["10", "1/10", "10", "1/10"],
        ["1", "1", "1/10", "1/10"],
        ["1/100", "100", "1", "10"],
        ["100", "1/100", "10", "1"],
        ["1/100", "1", "100", "1"],
        ["1", "100", "1", "1/100"],
    ];
    for values in coordinates {
        for x in ["1/1000", "1/100", "1/10", "1", "10", "100", "1000"] {
            ensure!(
                started.elapsed().as_secs_f64() < args.seconds * 0.9,
                "soft research budget"
            );
            let mut n = s.numerator.clone();
            let mut guards = s.guards.clone();
            for (axis, value) in [
                (2, values[0]),
                (3, values[1]),
                (4, values[2]),
                (5, values[3]),
                (6, x),
            ] {
                let q = rational(value)?;
                n = n.replace(axis, &q);
                for g in &mut guards {
                    *g = g.replace(axis, &q);
                }
            }
            fibers += 1;
            if n.is_zero() {
                identically_zero += 1;
                continue;
            }
            let np = n.to_univariate_from_univariate(7);
            let gp = guards
                .iter()
                .map(|p| p.to_univariate_from_univariate(7))
                .collect::<Vec<_>>();
            for mut root in isolate_union(vec![(0, np.clone())])? {
                while root.interval.0 <= Rational::zero() && root.interval.1 > Rational::zero() {
                    refine_once(&mut root);
                }
                if root.interval.1 <= Rational::zero() {
                    continue;
                }
                let signs = signs_at_root(&mut root, &gp)?;
                rows.push(json!({"x1_x3_x5_x7":values,"x2":x,"x4_interval":[root.interval.0.to_string(),root.interval.1.to_string()],"jacobian_fiber":a.format(&n),"guard_signs_s4_u4_minusC0_s":signs,"strict_positive_critical_point":signs[..3]==[1,1,1]}));
            }
        }
        report["rows"] = json!(rows);
        report["fibers"] = json!(fibers);
        report["identically_zero_fibers"] = json!(identically_zero);
        report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        save(&args.output, &report)?;
    }
    report["stage"] = json!("finite_scan_complete");
    save(&args.output, &report)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_kinematic_elimination() -> Result<()> {
        let s = structure()?;
        ensure!(!s.numerator.is_zero(), "nontrivial Jacobian");
        Ok(())
    }
    #[test]
    fn guard_signs_include_zero() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        let p = |t: &str| Ok::<_, anyhow::Error>(a.parse(t)?.to_univariate_from_univariate(0));
        let mut roots = isolate_union(vec![(0, p("x^2-2")?)])?;
        let mut root = roots.pop().unwrap();
        assert_eq!(
            signs_at_root(&mut root, &[p("x-1")?, p("x-2")?, p("x^2-2")?])?,
            vec![1, -1, 0]
        );
        Ok(())
    }
}
