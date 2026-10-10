//! Exact positive-factor and cubic-in-ratio diagnostic; no coverage claim.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::algebra::{Algebra, Poly};
#[derive(Parser)]
struct Args {
    #[arg(
        long,
        default_value = "artifacts/research/a446-filtered-jacobian-01/chart-v6.json"
    )]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long)]
    bernstein: bool,
    #[arg(long)]
    factor: bool,
    #[arg(long)]
    certificates: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
fn save(path: &PathBuf, v: &Value) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let t = path.with_extension("tmp");
    fs::write(&t, serde_json::to_vec_pretty(v)?)?;
    fs::rename(t, path)?;
    Ok(())
}
fn stats(a: &Algebra, p: &Poly) -> Value {
    json!({"polynomial":a.format(p),"terms":p.nterms(),"degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),"negative_coefficients":p.coefficients.iter().filter(|c|**c<Rational::zero()).count(),"positive_coefficients":p.coefficients.iter().filter(|c|**c>Rational::zero()).count()})
}
fn structure(a: &Algebra) -> Result<(Poly, Poly, Poly, Poly, Poly)> {
    let b = a.parse("rr+ww+tt+vv+zz")?;
    let aa =
        a.parse("vv*((rr+ww+tt+vv+zz)^2-(5/18)*(ww+zz)*(rr+tt+vv+zz))-(5/18)*rr*zz*(ww+zz)")?;
    let bb = a.parse("vv*((rr+ww+tt+vv+zz)^2-(5/18)*ww*(rr+tt+vv+zz))-(5/18)*rr*ww*zz")?;
    let a4 = &a.parse("zz")? * &aa - a.parse("4*tt*(tt+vv+zz)*(rr+vv)*(ww+zz)")?;
    let b4 = &a.parse("zz")? * &bb - a.parse("4*tt*(tt+vv+zz)*rr*ww")?;
    let difference = &b4 - &a4;
    ensure!(
        !difference.is_zero()
            && difference
                .coefficients
                .iter()
                .all(|c| *c > Rational::zero()),
        "A4<B4"
    );
    Ok((b, aa, bb, a4, b4))
}
fn strip_positive(a: &Algebra, f: &Poly) -> Result<(Poly, Poly, Vec<Value>)> {
    let mut reduced = f.clone();
    let mut factor = f.one();
    let mut removed = Vec::new();
    for text in [
        "hh",
        "rr+ww+tt+vv+zz",
        "1+hh",
        "rr+vv",
        "ww+zz",
        "rr",
        "ww",
        "tt",
        "vv",
        "zz",
    ] {
        let p = a.parse(text)?;
        let mut power = 0;
        while power < 16 {
            let Some(q) = reduced.try_div(&p) else { break };
            ensure!(&q * &p == reduced, "positive factor quotient replay");
            reduced = q;
            factor = &factor * &p;
            power += 1;
        }
        if power > 0 {
            removed.push(json!({"factor":text,"power":power}));
        }
    }
    ensure!(&factor * &reduced == *f, "all positive factors replay");
    Ok((factor, reduced, removed))
}
fn positive(p: &Poly) -> bool {
    !p.is_zero() && p.coefficients.iter().all(|c| *c > Rational::zero())
}
// Sufficient coefficient certificate only. No failed search decides a sign.
fn subtract_premise(f: &Poly, premise: &Poly, reverse: bool) -> Result<(Poly, Poly, usize)> {
    use std::collections::BTreeMap;
    ensure!(
        f.variables() == premise.variables() && f.nterms() <= 16384 && premise.nterms() <= 64,
        "certificate bounds"
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
        let mut negative = coefficients
            .iter()
            .filter(|(_, c)| **c < Rational::zero())
            .collect::<Vec<_>>();
        if reverse {
            negative.reverse();
        }
        let mut selected = None;
        'outer: for (e, c) in negative {
            for (pc, pe) in premise
                .coefficients
                .iter()
                .zip(premise.exponents_iter())
                .filter(|(c, _)| **c < Rational::zero())
            {
                if e.iter().zip(pe).any(|(a, b)| a < b) {
                    continue;
                }
                let shift = e.iter().zip(pe).map(|(a, b)| a - b).collect::<Vec<_>>();
                let weight = c / pc;
                let fits = premise
                    .coefficients
                    .iter()
                    .zip(premise.exponents_iter())
                    .filter(|(c, _)| **c > Rational::zero())
                    .all(|(c, e)| {
                        let target = e.iter().zip(&shift).map(|(a, b)| a + b).collect::<Vec<_>>();
                        coefficients
                            .get(&target)
                            .is_some_and(|available| *available >= &weight * c)
                    });
                if fits {
                    selected = Some((shift, weight));
                    break 'outer;
                }
            }
        }
        let Some((shift, weight)) = selected else {
            break;
        };
        let mut term = f.zero();
        term.append_monomial(weight, &shift);
        residual = &residual - &(&term * premise);
        multiplier = &multiplier + &term;
        steps += 1;
        ensure!(
            steps <= 2048 && residual.nterms() <= 16384,
            "certificate work bound"
        );
    }
    ensure!(
        &multiplier * premise + residual.clone() == *f,
        "conditional identity reconstruction"
    );
    ensure!(
        multiplier.is_zero() || positive(&multiplier),
        "nonnegative multiplier"
    );
    Ok((multiplier, residual, steps))
}
fn certificate_portfolio(a: &Algebra, f: &Poly, b: &Poly, premises: &[Poly]) -> Result<Value> {
    let mut attempts = Vec::new();
    for power in 0..=2 {
        for reverse in [false, true] {
            for order in 0..premises.len() {
                let scale = b.pow(power);
                let mut residual = &scale * f;
                let mut multipliers = vec![f.zero(); premises.len()];
                let mut steps = 0;
                for k in 0..premises.len() {
                    let index = (order + k) % premises.len();
                    let (m, r, n) = subtract_premise(&residual, &premises[index], reverse)?;
                    multipliers[index] = m;
                    residual = r;
                    steps += n;
                }
                let rebuilt = premises
                    .iter()
                    .zip(&multipliers)
                    .fold(residual.clone(), |acc, (p, m)| acc + p * m);
                ensure!(rebuilt == &scale * f, "portfolio identity");
                let success = positive(&residual);
                attempts.push(json!({"scale":a.format(&scale),"multipliers":multipliers.iter().map(|p|a.format(p)).collect::<Vec<_>>(),"residual":stats(a,&residual),"steps":steps,"strict_certificate":success,"identity_checked":true}));
                if success {
                    return Ok(json!({"strict_certificate":true,"attempts":attempts}));
                }
            }
        }
    }
    Ok(json!({"strict_certificate":false,"attempts":attempts}))
}
fn original_jacobian_point(point: &[Rational], reduced_value: &Rational) -> Result<Value> {
    let a = Algebra::new(&["ss", "uu", "pp", "qq", "rr", "ww", "xx", "yy"].map(String::from))?;
    let ap = a.parse("pp+qq+xx")?;
    let bp = a.parse("rr+ww+yy")?;
    let l = &ap + &bp;
    let t = &bp * &a.parse("pp*qq")? + &ap * &a.parse("rr*ww")?;
    let e = &l * &ap * &bp - a.parse("(5/18)*((rr+ww+yy)*xx*qq+(pp+qq+xx)*yy*ww)")?;
    let a0 = l.pow(2) - a.parse("(5/18)*(xx+yy)*(qq+ww)")?;
    let aa = &a0 - &a.parse("ss*xx*yy+uu*(pp+rr)*(qq+ww)")?;
    let bb = &e - &(&a.parse("uu")? * &t);
    let jac = &aa.derivative(6) * &bb.derivative(7) - &aa.derivative(7) * &bb.derivative(6);
    let [h, r, w, zx, v, z]: [Rational; 6] = point
        .to_vec()
        .try_into()
        .map_err(|_| anyhow::anyhow!("six chart coordinates"))?;
    let x = &h * &zx;
    let y = &zx + &v + z.clone();
    let p = &h * &(&r + &v);
    let q = &h * &(&w + &z);
    let mut values = vec![
        Rational::zero(),
        Rational::zero(),
        p,
        q,
        r.clone(),
        w.clone(),
        x.clone(),
        y.clone(),
    ];
    let tv = t.replace_all(&values);
    let ev = e.replace_all(&values);
    let a0v = a0.replace_all(&values);
    let xy = a.parse("(pp+rr)*(qq+ww)")?.replace_all(&values);
    let u = &ev / &tv;
    let s = (a0v - &u * &xy) / (&x * &y);
    values[0] = s.clone();
    values[1] = u.clone();
    ensure!(
        aa.replace_all(&values).is_zero() && bb.replace_all(&values).is_zero(),
        "original A=B=0 at reconstructed kinematics"
    );
    let actual = jac.replace_all(&values);
    let bv = &r + &w + y.clone();
    let predicted = -reduced_value.clone() * h.pow(4) * bv.pow(2) / (&x * &y * tv.pow(2));
    ensure!(
        actual == predicted,
        "independent original Jacobian agrees with reduced cubic"
    );
    let c0 = a
        .parse("(pp+rr+qq+ww)^2-uu*(pp+rr)*(qq+ww)")?
        .replace_all(&values);
    ensure!(
        s > Rational::from(4) && u > Rational::from(4) && c0 < Rational::zero(),
        "strict original complement"
    );
    Ok(
        json!({"chart":point.iter().map(ToString::to_string).collect::<Vec<_>>(),"original_variables":a.names,"original_values":values.iter().map(ToString::to_string).collect::<Vec<_>>(),"s":s.to_string(),"u":u.to_string(),"C0":c0.to_string(),"jacobian":actual.to_string(),"reduced_negative_jacobian":reduced_value.to_string(),"A_and_B_zero":true,"independent_jacobian_checked":true}),
    )
}
fn critical_counterexample(reduced: &Poly, point: &[Rational]) -> Result<Value> {
    let a = Algebra::new(&["hh", "rr", "ww", "tt", "vv", "zz"].map(String::from))?;
    let (_, _, _, a4, b4) = structure(&a)?;
    ensure!(
        a4.replace_all(point) > Rational::zero() && b4.replace_all(point) > Rational::zero(),
        "s>4 for every positive h"
    );
    let un = a.parse("(hh+1)*(rr+ww+tt+vv+zz)^2-(5/18)*(hh*tt*(ww+zz)+(tt+vv+zz)*ww)")?;
    let ud = a.parse("hh*(rr+vv)*(ww+zz)+rr*ww")?;
    let uc = &un - &ud.clone().mul_coeff(Rational::from(4));
    let c0 = a.parse("(hh*(rr+vv+ww+zz)+rr+ww)^2")? * &ud
        - &un * &a.parse("(hh*(rr+vv)+rr)*(hh*(ww+zz)+ww)")?;
    let mut univariate = reduced.clone();
    for (i, value) in point.iter().enumerate().skip(1) {
        univariate = univariate.replace(i, value);
    }
    let fiber = univariate.to_univariate_from_univariate(0);
    ensure!(
        fiber
            .coefficients()
            .last()
            .is_some_and(|c| *c < Rational::zero()),
        "negative cubic leading coefficient"
    );
    let mut low = point.to_vec();
    let mut high = point.to_vec();
    let mut found_low = false;
    let mut found_high = false;
    for _ in 0..128 {
        if reduced.replace_all(&low) > Rational::zero() {
            found_low = true;
            break;
        }
        low[0] = &low[0] / &Rational::from(2);
    }
    for _ in 0..128 {
        if reduced.replace_all(&high) < Rational::zero() {
            found_high = true;
            break;
        }
        high[0] = &high[0] * &Rational::from(2);
    }
    ensure!(
        found_low && found_high && low[0] < high[0],
        "opposite exact Jacobian signs"
    );
    let count = symgcad::roots::roots_in_open_interval(&fiber, &low[0], &high[0]);
    ensure!(
        count > 0,
        "exact positive critical root between opposite signs"
    );
    let mut guards = Vec::new();
    for (name, mut guard) in [("u_minus_4", uc), ("minus_C0", -c0)] {
        for (i, value) in point.iter().enumerate().skip(1) {
            guard = guard.replace(i, value);
        }
        let gp = guard.to_univariate_from_univariate(0);
        let n = symgcad::roots::roots_in_open_interval(&gp, &low[0], &high[0]);
        ensure!(
            n == 0
                && gp.evaluate(&low[0]) > Rational::zero()
                && gp.evaluate(&high[0]) > Rational::zero(),
            "strict kinematic guard on entire critical interval"
        );
        guards.push(json!({"name":name,"positive_numerator":a.format(&guard),"positive_denominator":a.format(&ud),"roots_in_critical_interval":n,"uniform_positive_coefficients":positive(&guard)}));
    }
    Ok(
        json!({"polynomial_in_h":univariate.to_string(),"coefficient_values":fiber.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>(),"critical_roots_in_interval":count,"strict_interval_guards":guards,"negative_jacobian_endpoint":original_jacobian_point(&low,&reduced.replace_all(&low))?,"positive_jacobian_endpoint":original_jacobian_point(&high,&reduced.replace_all(&high))?,"argument":"A4>0 and B4>0 keep s>4 for every h>0; exact Sturm checks retain u>4,C0<0 throughout the bracket. Opposite independently checked original Jacobian signs prove a positive critical point."}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 300.0 && args.seconds.is_finite(),
        "wall cap"
    );
    ensure!(!args.output.exists(), "fresh output");
    if !args.worker {
        let mut c = Command::new("taskset");
        c.args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .arg("--worker")
            .arg("--input")
            .arg(&args.input)
            .arg("--output")
            .arg(&args.output)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        if args.bernstein {
            c.arg("--bernstein");
        }
        if args.factor {
            c.arg("--factor");
        }
        if args.certificates {
            c.arg("--certificates");
        }
        let p = symgcad::benchmark::run_bounded(
            &mut c,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 8192,
                workers: 1,
                max_cells: None,
            },
        )?;
        save(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(p)?,
        )?;
        return Ok(());
    }
    let started = Instant::now();
    let bytes = fs::read(&args.input)?;
    ensure!(bytes.len() <= 32 * 1024 * 1024, "input cap");
    let source: Value = serde_json::from_slice(&bytes)?;
    let chart = &source["positive_cone_chart"];
    let a = Algebra::new(&["hh", "rr", "ww", "tt", "vv", "zz"].map(String::from))?;
    let parse = |v: &Value| a.parse(v.as_str().context("polynomial expression")?);
    let f = -parse(&chart["jacobian"])?;
    let v = parse(&chart["V"])?;
    let g = parse(&chart["s_above_4_numerator"])?;
    let (b, aa, bb, a4, b4) = structure(&a)?;
    ensure!(
        v == a.parse("hh")? * &(&a.parse("hh")? * &aa + bb.clone()),
        "V linear after h removal"
    );
    ensure!(
        g == a.parse("hh^2")? * &b * &(&a.parse("hh")? * &a4 + b4.clone()),
        "s>4 linear numerator"
    );
    let (positive_factor, reduced, removed) = strip_positive(&a, &f)?;
    ensure!(reduced.degree(0) <= 3, "cubic ratio degree");
    let mut coefficients = vec![reduced.zero(); 4];
    for (c, power) in reduced.to_univariate_polynomial_list(0) {
        coefficients[power as usize] = c;
    }
    let mut report = json!({"research_only":true,"coverage_claim":false,"input_sha256":format!("{:x}",Sha256::digest(&bytes)),"variables":a.names,"identity_checked":true,"positive_factors_removed":removed,"positive_factor":a.format(&positive_factor),"reduced":stats(&a,&reduced),"V_over_h_coefficients":[stats(&a,&bb),stats(&a,&aa)],"s_above_4_coefficients":[stats(&a,&b4),stats(&a,&a4)],"cubic_coefficients":coefficients.iter().map(|p|stats(&a,p)).collect::<Vec<_>>(),"elapsed_seconds":started.elapsed().as_secs_f64(),"stage":"structure_complete"});
    save(&args.output, &report)?;
    if args.certificates {
        let mut attempts = Vec::new();
        for (i, c) in coefficients.iter().enumerate() {
            attempts.push(json!({"coefficient":i,"premises":["B4>0"],"result":certificate_portfolio(&a,c,&b,&[b4.clone()])?}));
            attempts.push(json!({"coefficient":i,"premises":["A4>=0","B4>0"],"result":certificate_portfolio(&a,c,&b,&[a4.clone(),b4.clone()])?}));
            report["coefficient_certificates"] = json!(attempts);
            save(&args.output, &report)?;
        }
    }
    if args.bernstein {
        let minus_a = -a4.clone();
        let a2 = minus_a.pow(2);
        let a3 = minus_a.pow(3);
        let b2 = b4.pow(2);
        let b3 = b4.pow(3);
        let terms = [
            &a3 * &coefficients[0],
            &a2 * &b4 * &coefficients[1],
            &minus_a * &b2 * &coefficients[2],
            &b3 * &coefficients[3],
        ];
        let transformed = [
            terms[0].clone(),
            terms[0].clone().mul_coeff(Rational::from(3)) + terms[1].clone(),
            terms[0].clone().mul_coeff(Rational::from(3))
                + terms[1].clone().mul_coeff(Rational::from(2))
                + terms[2].clone(),
            terms[0].clone() + terms[1].clone() + terms[2].clone() + terms[3].clone(),
        ];
        // At h=B4*z/((-A4)*(1+z)), multiplying by (-A4)^3(1+z)^3 gives these coefficients.
        let z = a.parse("hh")?;
        let one_plus = a.parse("1+hh")?;
        let numerator = &b4 * &z;
        let denominator = &minus_a * &one_plus;
        let mut direct = reduced.zero();
        let mut expanded = reduced.zero();
        for i in 0..4 {
            ensure!(
                coefficients[i].degree(0) == 0 && transformed[i].degree(0) == 0,
                "base-only transformed coefficients"
            );
            direct = direct + &coefficients[i] * &numerator.pow(i) * &denominator.pow(3 - i);
            expanded = expanded + &transformed[i] * &z.pow(i);
        }
        ensure!(direct == expanded, "full exact Mobius reconstruction");
        let obligations = [
            coefficients[0].clone(),
            transformed[1]
                .try_div(&a2)
                .context("known positive L^2 factor")?,
            transformed[2]
                .try_div(&minus_a)
                .context("known positive L factor")?,
            transformed[3].clone(),
        ];
        ensure!(
            &obligations[1] * &a2 == transformed[1] && &obligations[2] * &minus_a == transformed[2],
            "positive factors restored"
        );
        report["bounded_fiber_transform"] = json!({"premises":["A4<0","B4>0","z>0"],"h":"B4*z/((-A4)*(1+z))","full_identity_checked":true,"coefficients":transformed.iter().map(|p|stats(&a,p)).collect::<Vec<_>>(),"factored_obligations":obligations.iter().map(|p|stats(&a,p)).collect::<Vec<_>>()});
        let mut counterexamples = serde_json::Map::new();
        let grid = [(1, 100), (1, 10), (1, 3), (1, 1), (3, 1), (10, 1), (100, 1)];
        let mut seed = 0x734bea5191u64;
        for _ in 0..2048 {
            let mut point = vec![Rational::one()];
            for _ in 1..6 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let (n, d) = grid[(seed as usize) % grid.len()];
                point.push(Rational::from((n, d)));
            }
            let av = a4.replace_all(&point);
            let bv = b4.replace_all(&point);
            if bv <= Rational::zero() {
                continue;
            }
            let targets = if av < Rational::zero() {
                &obligations[..]
            } else {
                &coefficients[..]
            };
            for (i, p) in targets.iter().enumerate() {
                let key = format!(
                    "{}_{}",
                    if av < Rational::zero() {
                        "bounded"
                    } else {
                        "unbounded"
                    },
                    i
                );
                if counterexamples.contains_key(&key) {
                    continue;
                }
                let value = p.replace_all(&point);
                if value < Rational::zero() {
                    counterexamples.insert(key,json!({"point":point.iter().map(ToString::to_string).collect::<Vec<_>>(),"A4":av.to_string(),"B4":bv.to_string(),"value":value.to_string()}));
                }
            }
        }
        report["conditional_coefficient_counterexamples"] = json!(counterexamples);
        if let Some(witness) = counterexamples.get("unbounded_3") {
            let point = witness["point"]
                .as_array()
                .context("counterexample coordinates")?
                .iter()
                .map(|x| symgcad::algebra::rational(x.as_str().context("rational coordinate")?))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                a4.replace_all(&point) > Rational::zero()
                    && b4.replace_all(&point) > Rational::zero(),
                "unbounded strict kinematic region"
            );
            report["critical_counterexample"] = critical_counterexample(&reduced, &point)?;
        }
        report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        report["stage"] = json!("bernstein_complete");
        save(&args.output, &report)?;
        if args.factor {
            report["stage"] = json!("threshold_factor_running");
            save(&args.output, &report)?;
            let factors = transformed[3].factor();
            let product = factors
                .iter()
                .fold(transformed[3].one(), |acc, (p, e)| &acc * &p.pow(*e));
            ensure!(
                product == transformed[3],
                "full signed factorization reconstructed"
            );
            report["threshold_factors"] = json!(
                factors
                    .iter()
                    .map(|(p, e)| json!({"factor":stats(&a,p),"multiplicity":e}))
                    .collect::<Vec<_>>()
            );
            report["threshold_factor_product_checked"] = json!(true);
        }
    }
    report["stage"] = json!("complete");
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    save(&args.output, &report)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_parameter_difference_and_factor_reconstruction() -> Result<()> {
        let a = Algebra::new(&["hh", "rr", "ww", "tt", "vv", "zz"].map(String::from))?;
        structure(&a)?;
        let f = a.parse("hh^4*(1+hh)*(rr+ww+tt+vv+zz)^2*(vv^2-2*vv+3)")?;
        let (_, r, _) = strip_positive(&a, &f)?;
        ensure!(
            r == a.parse("vv^2-2*vv+3")?,
            "positive factors removed exactly"
        );
        Ok(())
    }
    #[test]
    fn exact_mobius_cubic_identity() -> Result<()> {
        let a = Algebra::new(&["z".into()])?;
        let z = a.parse("z")?;
        let one_plus = a.parse("1+z")?;
        let aa = Rational::from(2);
        let bb = Rational::from(3);
        let coeff = [1, 2, 3, 4].map(Rational::from);
        let lhs = one_plus.pow(3).mul_coeff(&aa * &aa * &aa * &coeff[0])
            + (&z * &one_plus.pow(2)).mul_coeff(&aa * &aa * &bb * &coeff[1])
            + (&z.pow(2) * &one_plus).mul_coeff(&aa * &bb * &bb * &coeff[2])
            + z.pow(3).mul_coeff(&bb * &bb * &bb * &coeff[3]);
        ensure!(
            lhs == a.parse("8+48*z+126*z^2+194*z^3")?,
            "exact scaled cubic substitution"
        );
        Ok(())
    }
    #[test]
    fn conditional_coefficient_certificate_replays_and_failure_is_inconclusive() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let premise = a.parse("y-x")?;
        let target = a.parse("y-x+x*y")?;
        let (m, r, _) = subtract_premise(&target, &premise, false)?;
        ensure!(
            m == a.parse("1")? && r == a.parse("x*y")? && positive(&r),
            "exact sufficient certificate"
        );
        let (_, r, _) = subtract_premise(&a.parse("-1")?, &premise, false)?;
        ensure!(!positive(&r), "negative constant not proved positive");
        Ok(())
    }
    #[test]
    fn original_positive_common_roots_can_have_a_zero_jacobian_in_the_strict_complement()
    -> Result<()> {
        let a = Algebra::new(&["hh", "rr", "ww", "tt", "vv", "zz"].map(String::from))?;
        let cubic=a.parse("246482922013046330125393/253125000000+103136217549968125163498071/63281250000*hh+317543526023029208134644139/253125000000*hh^2-50026512848510097823/10546875000*hh^3")?;
        let point = ["1", "1/100", "10", "1/100", "3", "100"]
            .map(symgcad::algebra::rational)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let evidence = critical_counterexample(&cubic, &point)?;
        ensure!(
            evidence["critical_roots_in_interval"] == 1,
            "exact unique positive critical point"
        );
        for h in [2, 4, 8] {
            let mut q = point.clone();
            q[0] = Rational::from(h);
            original_jacobian_point(&q, &cubic.replace_all(&q))?;
        }
        Ok(())
    }
}
