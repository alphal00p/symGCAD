//! Bounded exact research on the filtered positive-common-root Jacobian.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{CertificateOptions, certify_nonzero, derive_domain, verify_certificate},
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long)]
    factor: bool,
    #[arg(long)]
    certify: bool,
    #[arg(long, default_value_t = 0)]
    samples: usize,
    #[arg(long)]
    positive_chart: bool,
    #[arg(long, default_value_t = 0)]
    multiplier_depth: usize,
    #[arg(long, hide = true)]
    worker: bool,
}
struct Identity {
    algebra: Algebra,
    m: Poly,
    nx: Poly,
    ny: Poly,
    q: Poly,
    v: Poly,
    t: Poly,
    derivative_f: Poly,
    original_jacobian: Poly,
    e: Poly,
}
fn identity() -> Result<Identity> {
    let algebra = Algebra::new(&["x1", "x3", "x5", "x7", "x2", "x4"].map(String::from))?;
    let p = |s: &str| algebra.parse(s);
    let t = p("(x4+x5+x7)*x1*x3+(x2+x1+x3)*x5*x7")?;
    let e =
        p("(x1+x3+x5+x7+x2+x4)*(x1+x3+x2)*(x5+x7+x4)-(5/18)*((x4+x5+x7)*x2*x3+(x2+x1+x3)*x4*x7)")?;
    let a0 = p("(x1+x3+x5+x7+x2+x4)^2-(5/18)*(x2+x4)*(x3+x7)")?;
    let q = p("x3*(x4+x5+x7)-x7*(x2+x1+x3)")?;
    let v = p("(x1+x3+x5+x7+x2+x4)*(x1*(x4+x5+x7)-x5*(x2+x1+x3))+(5/18)*(x3+x7)*(x5*x2-x1*x4)")?;
    let w = p("x2*x4")?;
    let s = &a0 * &t - &p("(x1+x5)*(x3+x7)")? * &e;
    ensure!(s == &q * &v, "factored s identity");
    let nx = &e.derivative(4) * &t - &e * &t.derivative(4);
    let ny = &e.derivative(5) * &t - &e * &t.derivative(5);
    ensure!(
        nx.coefficients
            .iter()
            .chain(&ny.coefficients)
            .all(|c| *c > Rational::zero()),
        "coordinate derivatives of u strictly positive"
    );
    let m = &p("x4")? * &(&p("x2")? * &v.derivative(4) - v.clone()) * &ny
        - &p("x2")? * &(&p("x4")? * &v.derivative(5) - v.clone()) * &nx;
    let dx = &q.derivative(4) * &t - &q * &t.derivative(4);
    let dy = &q.derivative(5) * &t - &q * &t.derivative(5);
    ensure!(
        dx == p("-x3*x7*(x1+x5)*(x4+x5+x7)")? && dy == p("x3*x7*(x1+x5)*(x2+x1+x3)")?,
        "ratio derivative identity"
    );
    let df = &dx * &ny - &dy * &nx;
    ensure!(
        df.coefficients.iter().all(|c| *c < Rational::zero()),
        "f decreases on fixed-u arc"
    );
    let jac = &(&p("x2")? * &a0.derivative(4) * &t - s.clone()) * &p("x4")? * &ny
        - &(&p("x4")? * &a0.derivative(5) * &t - s.clone()) * &p("x2")? * &nx;
    ensure!(
        &jac * &t == &w * &v * &df + &q * &m * &t,
        "product derivative and Jacobian identity"
    );
    Ok(Identity {
        algebra,
        m,
        nx,
        ny,
        q,
        v,
        t,
        derivative_f: df,
        original_jacobian: jac,
        e,
    })
}
fn save(path: &PathBuf, v: &Value) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec_pretty(v)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
/// A bounded sufficient certificate search, never a positivity decision procedure.
fn coefficient_multiplier(f: &Poly, v: &Poly) -> Result<(Poly, Poly, usize)> {
    use std::collections::BTreeMap;
    ensure!(
        f.variables() == v.variables() && f.nterms() <= 32768 && v.nterms() <= 512,
        "coefficient search bounds"
    );
    let mut residual = f.clone();
    let mut multiplier = f.zero();
    let mut steps = 0;
    loop {
        let coefficients = residual
            .coefficients
            .iter()
            .zip(residual.exponents_iter())
            .map(|(c, e)| (e.to_vec(), c.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut selected = None;
        'candidates: for (exponents, coefficient) in
            coefficients.iter().filter(|(_, c)| **c < Rational::zero())
        {
            for (vc, ve) in v
                .coefficients
                .iter()
                .zip(v.exponents_iter())
                .filter(|(c, _)| **c < Rational::zero())
            {
                if exponents.iter().zip(ve).any(|(a, b)| a < b) {
                    continue;
                }
                let shift = exponents
                    .iter()
                    .zip(ve)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>();
                let weight = coefficient / vc;
                let fits = v
                    .coefficients
                    .iter()
                    .zip(v.exponents_iter())
                    .filter(|(c, _)| **c > Rational::zero())
                    .all(|(c, e)| {
                        let target = e.iter().zip(&shift).map(|(a, b)| a + b).collect::<Vec<_>>();
                        coefficients
                            .get(&target)
                            .is_some_and(|available| *available >= &weight * c)
                    });
                if fits {
                    selected = Some((shift, weight));
                    break 'candidates;
                }
            }
        }
        let Some((shift, weight)) = selected else {
            break;
        };
        let mut term = f.zero();
        term.append_monomial(weight, &shift);
        residual = &residual - &(&term * v);
        multiplier = &multiplier + &term;
        steps += 1;
        ensure!(
            steps <= 8192 && residual.nterms() <= 32768,
            "coefficient search work cap"
        );
    }
    ensure!(
        f == &(&multiplier * v + residual.clone()),
        "exact multiplier reconstruction"
    );
    ensure!(
        multiplier
            .coefficients
            .iter()
            .all(|c| *c > Rational::zero()),
        "nonnegative multiplier"
    );
    Ok((multiplier, residual, steps))
}
fn pseudo_division(f: &Poly, v: &Poly, axis: usize) -> Result<(Poly, Poly, Poly)> {
    let lc = v.univariate_lcoeff(axis);
    ensure!(
        !lc.is_zero() && lc.coefficients.iter().all(|x| *x > Rational::zero()),
        "positive divisor LC"
    );
    let mut quotient = f.zero();
    let mut remainder = f.clone();
    let mut scale = f.one();
    let mut steps = 0;
    while !remainder.is_zero() && remainder.degree(axis) >= v.degree(axis) {
        let old_degree = remainder.degree(axis);
        let mut e = vec![0; f.nvars()];
        e[axis] = old_degree - v.degree(axis);
        let term = remainder
            .univariate_lcoeff(axis)
            .mul_monomial(&Rational::one(), &e);
        quotient = &lc * &quotient + term.clone();
        remainder = &lc * &remainder - (&term * v);
        scale = &scale * &lc;
        ensure!(
            remainder.is_zero() || remainder.degree(axis) < old_degree,
            "pseudo-division degree descent"
        );
        steps += 1;
        ensure!(
            steps <= 16 && remainder.nterms() <= 65536 && quotient.nterms() <= 65536,
            "pseudo-division cap"
        );
    }
    ensure!(
        &scale * f == &quotient * v + remainder.clone(),
        "pseudo-division identity"
    );
    Ok((scale, quotient, remainder))
}
fn chart_report(i: &Identity, depth: usize) -> Result<Value> {
    let started = Instant::now();
    let a = &i.algebra;
    let p = a.parse("x1*(x4+x5+x7)-x5*(x2+x1+x3)")?;
    let z = a.parse("(x1+x3+x2)*(x1+x3+x5+x7+x2+x4-(5/18)*(x3+x7))+(5/18)*x3*(x3+x7)")?;
    ensure!(
        a.parse("x1+x3+x2")? * &i.v == &z * &p - a.parse("(5/18)*x1*(x3+x7)")? * &i.q,
        "same-sign implication identity"
    );
    ensure!(
        z.coefficients.iter().all(|c| *c > Rational::zero()),
        "positive Z"
    );
    let swap = |p: &Poly| {
        let mut q = p.zero();
        for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
            q.append_monomial(c.clone(), &[e[2], e[3], e[0], e[1], e[5], e[4]]);
        }
        q
    };
    ensure!(
        swap(&i.original_jacobian) == -i.original_jacobian.clone()
            && swap(&i.q) == -i.q.clone()
            && swap(&i.v) == -i.v.clone()
            && swap(&i.t) == i.t,
        "negative-cone symmetry"
    );
    let names = a
        .names
        .iter()
        .cloned()
        .chain(["hh", "rr", "ww", "tt", "vv", "zz"].map(String::from))
        .collect::<Vec<_>>();
    let c = Algebra::new(&names)?;
    let substitutions = ["hh*(rr+vv)", "hh*(ww+zz)", "rr", "ww", "hh*tt", "tt+vv+zz"]
        .map(|s| c.parse(s))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let compose = |p: &Poly| -> Result<Poly> {
        let mut q = c.parse(&a.format(p))?;
        for (axis, v) in substitutions.iter().enumerate() {
            ensure!(q.nterms() <= 131072, "chart input cap");
            q = q.replace_with_poly(axis, v);
            ensure!(q.nterms() <= 131072, "chart output cap");
        }
        ensure!(
            (0..6).all(|axis| q.degree(axis) == 0),
            "old axes eliminated"
        );
        Ok(q)
    };
    ensure!(
        compose(&a.parse("x1+x3+x2")?)? == c.parse("hh*(rr+ww+tt+vv+zz)")?,
        "chart a=hb"
    );
    ensure!(
        compose(&p)? == c.parse("hh*(rr+ww+tt+vv+zz)*vv")?
            && compose(&i.q)? == c.parse("hh*(rr+ww+tt+vv+zz)*zz")?,
        "positive P/Q chart"
    );
    let j = compose(&i.original_jacobian)?;
    let v = compose(&i.v)?;
    let (multiplier, residual, steps) = coefficient_multiplier(&(-j.clone()), &v)?;
    let certificate_ok =
        !residual.is_zero() && residual.coefficients.iter().all(|c| *c > Rational::zero());
    let pos = j
        .coefficients
        .iter()
        .filter(|x| **x > Rational::zero())
        .count();
    let neg = j
        .coefficients
        .iter()
        .filter(|x| **x < Rational::zero())
        .count();
    let mut report = json!({"variables":names,"original_coordinates":substitutions.iter().map(|p|c.format(p)).collect::<Vec<_>>(),"same_sign_implication_checked":true,"negative_cone_symmetry_checked":true,"chart_identity_checked":true,"terms":j.nterms(),"positive_coefficients":pos,"negative_coefficients":neg,"strictly_negative_by_coefficients":pos==0&&neg>0,"jacobian":c.format(&j),"V":c.format(&v),"multiplier_search":{"identity_checked":true,"strict_certificate":certificate_ok,"steps":steps,"residual_negative_coefficients":residual.coefficients.iter().filter(|c|**c<Rational::zero()).count(),"multiplier":c.format(&multiplier),"residual":c.format(&residual)}});
    if depth > 0 {
        let (scale, quotient, remainder) = pseudo_division(&(-j.clone()), &v, 10)?;
        report["pseudo_division"] = json!({"axis":"vv","identity_checked":true,"scale":c.format(&scale),"quotient":c.format(&quotient),"remainder":c.format(&remainder),"quotient_negative_coefficients":quotient.coefficients.iter().filter(|x|**x<Rational::zero()).count(),"remainder_negative_coefficients":remainder.coefficients.iter().filter(|x|**x<Rational::zero()).count(),"quotient_terms":quotient.nterms(),"remainder_terms":remainder.nterms()});
        ensure!(depth <= 2, "multiplier degree cap");
        let g = compose(&(&i.q * &i.v - a.parse("4*x2*x4")? * &i.t))?;
        let mut attempts = Vec::new();
        for total in 0..=depth {
            for b_power in 0..=total {
                let h_power = total - b_power;
                let d = c.parse("1+hh")?.pow(h_power) * &c.parse("rr+ww+tt+vv+zz")?.pow(b_power);
                for first_g in [false, true] {
                    let f = &d * &(-j.clone());
                    if f.nterms() > 32768 {
                        continue;
                    }
                    let mut r = f.clone();
                    let mut cv = f.zero();
                    let mut cg = f.zero();
                    let mut steps = 0;
                    for _ in 0..8 {
                        let before = steps;
                        for use_g in [first_g, !first_g] {
                            let (m, rr, n) =
                                coefficient_multiplier(&r, if use_g { &g } else { &v })?;
                            if use_g {
                                cg = &cg + &m;
                            } else {
                                cv = &cv + &m;
                            }
                            steps += n;
                            r = rr;
                        }
                        if steps == before {
                            break;
                        }
                    }
                    ensure!(
                        f == &cv * &v + &cg * &g + r.clone(),
                        "two-premise multiplier identity"
                    );
                    let ok = !r.is_zero() && r.coefficients.iter().all(|x| *x > Rational::zero());
                    attempts.push(json!({"h_power":h_power,"b_power":b_power,"first_g":first_g,"strict_certificate":ok,"identity_checked":true,"steps":steps,"negative_residual_coefficients":r.coefficients.iter().filter(|x|**x<Rational::zero()).count(),"multiplier_V":c.format(&cv),"multiplier_s_above_4":c.format(&cg),"residual":c.format(&r)}));
                    if ok {
                        break;
                    }
                }
                if attempts
                    .last()
                    .is_some_and(|a| a["strict_certificate"] == true)
                {
                    break;
                }
            }
            if attempts
                .last()
                .is_some_and(|a| a["strict_certificate"] == true)
            {
                break;
            }
        }
        report["s_above_4_numerator"] = json!(c.format(&g));
        report["multiplier_portfolio"] = json!(attempts);
    }
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    Ok(report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 120.0 && args.seconds.is_finite(),
        "budget"
    );
    ensure!(!args.output.exists(), "fresh output");
    ensure!(
        args.samples <= 16384,
        "at most 16384 exact rational samples"
    );
    ensure!(
        args.multiplier_depth <= 2 && (args.multiplier_depth == 0 || args.positive_chart),
        "chart multiplier options"
    );
    if !args.worker {
        let mut command = Command::new("taskset");
        command
            .args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .arg("--worker")
            .arg("--output")
            .arg(&args.output)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        if args.factor {
            command.arg("--factor");
        }
        if args.certify {
            command.arg("--certify");
        }
        command.arg("--samples").arg(args.samples.to_string());
        if args.positive_chart {
            command.arg("--positive-chart");
        }
        command
            .arg("--multiplier-depth")
            .arg(args.multiplier_depth.to_string());
        let o = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        save(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(o)?,
        )?;
        return Ok(());
    }
    let started = Instant::now();
    let i = identity()?;
    let a = &i.algebra;
    let pos =
        i.m.coefficients
            .iter()
            .filter(|c| **c > Rational::zero())
            .count();
    let neg =
        i.m.coefficients
            .iter()
            .filter(|c| **c < Rational::zero())
            .count();
    let mut report = json!({"research_only":true,"coverage_claim":false,"variables":a.names,"identities_checked":true,"M":a.format(&i.m),"M_terms":i.m.nterms(),"M_positive_coefficients":pos,"M_negative_coefficients":neg,"M_strictly_negative_by_coefficients":pos==0&&neg>0,"N_x":a.format(&i.nx),"N_y":a.format(&i.ny),"Q":a.format(&i.q),"V":a.format(&i.v),"T":a.format(&i.t),"fixed_u_f_derivative_numerator":a.format(&i.derivative_f),"original_cleared_jacobian":a.format(&i.original_jacobian),"identity_seconds":started.elapsed().as_secs_f64(),"stage":"coefficient_screen_complete"});
    save(&args.output, &report)?;
    if args.positive_chart {
        report["positive_cone_chart"] = chart_report(&i, args.multiplier_depth)?;
        save(&args.output, &report)?;
    }
    if args.samples > 0 {
        let values = ["1/100", "1/10", "1/3", "1", "3", "10", "100"]
            .map(symgcad::algebra::rational)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let mut state = 0x41a446381u64;
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        let mut witnesses = std::collections::BTreeMap::<String, Value>::new();
        for _ in 0..args.samples {
            let point = (0..6)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    values[state as usize % values.len()].clone()
                })
                .collect::<Vec<_>>();
            let q = i.q.replace_all(&point);
            let v = i.v.replace_all(&point);
            let t = i.t.replace_all(&point);
            let den = &point[4] * &point[5] * &t;
            let s = &q * &v / &den;
            let u = i.e.replace_all(&point) / &t;
            let m = i.m.replace_all(&point);
            let j = i.original_jacobian.replace_all(&point);
            let sign = |r: &Rational| {
                if *r > Rational::zero() {
                    1
                } else if r.is_zero() {
                    0
                } else {
                    -1
                }
            };
            let region = if s > Rational::from(4) {
                "s>4"
            } else if s > Rational::zero() {
                "0<s<=4"
            } else if s.is_zero() {
                "s=0"
            } else {
                "s<0"
            };
            let key = format!("{region};Q={};M={};J={}", sign(&q), sign(&m), sign(&j));
            *counts.entry(key.clone()).or_default() += 1;
            witnesses.entry(key).or_insert_with(||json!({"point":point.iter().map(ToString::to_string).collect::<Vec<_>>(),"s":s.to_string(),"u":u.to_string(),"Q":q.to_string(),"V":v.to_string(),"M":m.to_string(),"jacobian_numerator":j.to_string()}));
        }
        report["rational_samples"] = json!({"count":args.samples,"counts":counts,"first_witness_per_sign_pattern":witnesses,"proof_of_universal_sign":false});
        save(&args.output, &report)?;
    }
    if args.certify {
        let constraints = a
            .names
            .iter()
            .map(|x| a.parse(x))
            .collect::<Result<Vec<_>>>()?;
        let domain = derive_domain(a, &constraints, &[])?;
        let certificate =
            certify_nonzero(a, &(-i.m.clone()), &domain, &CertificateOptions::default())?;
        if let Some(c) = &certificate {
            verify_certificate(a, &(-i.m.clone()), &constraints, &[], c)?;
        }
        report["minus_M_certificate"] = serde_json::to_value(certificate)?;
        report["certificate_elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        save(&args.output, &report)?;
    }
    if args.factor {
        report["stage"] = json!("factor_running");
        save(&args.output, &report)?;
        let factors = symgcad::algebra::factors_checked(&[i.m])?;
        report["factors"] = json!(factors.iter().map(|p| a.format(p)).collect::<Vec<_>>());
        report["factor_elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    }
    report["stage"] = json!("complete");
    save(&args.output, &report)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_product_and_derivative_identities() -> Result<()> {
        let i = identity()?;
        ensure!(!i.m.is_zero(), "nonzero derivative");
        Ok(())
    }
    #[test]
    fn positive_cone_chart_and_exchange_are_exact() -> Result<()> {
        let report = chart_report(&identity()?, 0)?;
        ensure!(report["chart_identity_checked"] == true, "exact chart");
        Ok(())
    }
    #[test]
    fn coefficient_multiplier_requires_a_complete_exact_certificate() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let v = a.parse("y-x")?;
        let (m, r, _) = coefficient_multiplier(&a.parse("y-x+x*y")?, &v)?;
        ensure!(
            m == a.parse("1")? && r == a.parse("x*y")?,
            "valid conditional certificate"
        );
        let (_, r, _) = coefficient_multiplier(&a.parse("-1")?, &v)?;
        ensure!(
            r.coefficients.iter().any(|c| *c < Rational::zero()),
            "failure retained"
        );
        Ok(())
    }
    #[test]
    fn exact_positive_scaled_pseudo_division() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let (scale, q, r) = pseudo_division(&a.parse("x^3+2*x*y+1")?, &a.parse("y*x-1")?, 0)?;
        ensure!(
            !scale.is_zero() && !q.is_zero() && r.degree(0) == 0,
            "proper remainder"
        );
        Ok(())
    }
    #[test]
    fn monotonicity_counterexample_is_inside_the_required_complement() -> Result<()> {
        let i = identity()?;
        let point = ["3", "100", "1/10", "10", "1", "1/3"]
            .map(symgcad::algebra::rational)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let s = i.q.replace_all(&point) * i.v.replace_all(&point)
            / (&point[4] * &point[5] * i.t.replace_all(&point));
        let u = i.e.replace_all(&point) / i.t.replace_all(&point);
        let xy = (&point[0] + &point[2]) * (&point[1] + &point[3]);
        let sum = &point[0] + &point[1] + &point[2] + &point[3];
        let c0 = &sum * &sum - &u * &xy;
        ensure!(
            s == symgcad::algebra::rational("9211/1260")?
                && u == symgcad::algebra::rational("3978697/103950")?,
            "exact kinematics"
        );
        ensure!(
            c0 == symgcad::algebra::rational("-983557/3780")?,
            "strict C0 complement"
        );
        ensure!(
            i.m.replace_all(&point) > Rational::zero()
                && i.original_jacobian.replace_all(&point) < Rational::zero(),
            "false monotonicity does not refute Jacobian sign"
        );
        Ok(())
    }
}
