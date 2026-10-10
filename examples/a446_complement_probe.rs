//! Bounded exact research on the unresolved A446/Ax446 kinematic complement.
use anyhow::{Context, Result, ensure};
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
    algebra::{Algebra, Poly, rational},
    projection::{discriminant, exact_resultant},
    roots::{isolate_union, refine_once, roots_in_open_interval},
};
const INPUT_HASH: &str = "9adaac7df7cde50989684002f8c5a93e225edfc95885c91f8f48f1cb426506b8";
#[derive(Parser)]
struct Args {
    #[arg(
        long,
        default_value = "artifacts/research/a446-kinematic-partition-01/report.json"
    )]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long)]
    discriminant: bool,
    #[arg(long)]
    slice_discriminant: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
fn save(path: &Path, value: &Value) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let temp = path.with_extension("partial.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn positive(p: &Poly) -> bool {
    !p.is_zero() && p.coefficients.iter().all(|c| *c > Rational::zero())
}
fn counter_family() -> Result<Value> {
    let a = Algebra::new(&["s", "u", "v", "w", "sigma"].map(String::from))?;
    let p = |s: &str| a.parse(s);
    let n = p("(s+4)*(v+4)-20/9")?;
    let d = p("v+31/9")?;
    let wnum = &p("v^2")? * &d;
    let unum = &p("v^2+(67/9)*v+16")? * &n - &p("s")? * &wnum;
    let uden = &p("4")? * &n;
    let bv = p("w*(v+31/9)+2*v^2+(103/9)*v+16-u*(v+4)")?;
    let cleared_b = &wnum * &d * &p("4")? + &p("2*v^2+(103/9)*v+16")? * &uden - &unum * &p("v+4")?;
    ensure!(cleared_b.is_zero(), "B common-root parameter identity");
    let av = p("v^2+(67/9)*v+16-4*u-s*w")?;
    let cleared_a = &p("v^2+(67/9)*v+16")? * &n - unum.clone() - &p("s")? * &wnum;
    ensure!(cleared_a.is_zero(), "A common-root parameter identity");
    let positive_u = &unum - &(&p("16")? * &n);
    ensure!(
        positive_u == &p("v")? * &(&p("v*(4*v+16+(5/9)*(s-4))")? + &(&p("67/9")? * &n)),
        "u-4 identity"
    );
    ensure!(
        positive(&positive_u.replace_with_poly(0, &p("sigma+4")?)),
        "u>4 coefficients"
    );
    let separation = &p("v^2")? * &n - &p("4")? * &wnum;
    ensure!(
        separation == p("s*v^2*(v+4)")?,
        "two positive roots identity"
    );
    let du = &unum.derivative(2) * &uden - &unum * &uden.derivative(2);
    ensure!(
        positive(&du.replace_with_poly(0, &p("sigma+4")?)),
        "u(v) is strictly increasing"
    );
    Ok(
        json!({"positive_parameter_domain":"s>4,v>0","x1_x3_x5_x7":"1","sum_x2_x4":"v","product_x2_x4_numerator":a.format(&wnum),"product_denominator":a.format(&n),"u_numerator":a.format(&unum),"u_denominator":a.format(&uden),"A_symmetric":a.format(&av),"B_symmetric":a.format(&bv),"u_minus4_cleared_positive":a.format(&positive_u),"sum_product_discriminant_cleared":a.format(&separation),"u_derivative_numerator_positive":a.format(&du),"claim":"For every s>4,u>4 there are two distinct positive x4 values with positive common x2 roots of A=B=0; u(v) rises continuously from4 toinfinity."}),
    )
}
fn variations(p: &UnivariatePolynomial<Q>) -> usize {
    let mut last = 0;
    let mut count = 0;
    for c in p.coefficients() {
        if c.is_zero() {
            continue;
        }
        let sign = if c.is_negative() { -1 } else { 1 };
        if last != 0 && last != sign {
            count += 1;
        }
        last = sign;
    }
    count
}
fn classify_positive_common_roots(
    r: &UnivariatePolynomial<Q>,
    j: &UnivariatePolynomial<Q>,
    k: &UnivariatePolynomial<Q>,
) -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    let gj = r.gcd(j);
    let gk = r.gcd(k);
    for mut root in isolate_union(vec![(0, r.clone())])? {
        for attempt in 0..4096 {
            let (lo, hi) = &root.interval;
            if hi <= &Rational::zero() {
                break;
            }
            if lo <= &Rational::zero() {
                refine_once(&mut root);
                continue;
            }
            let contains_root = |p: &UnivariatePolynomial<Q>| {
                if lo == hi {
                    p.evaluate(lo).is_zero()
                } else {
                    roots_in_open_interval(p, lo, hi) > 0
                }
            };
            if contains_root(&gj) || contains_root(&gk) {
                rows.push(json!({"x4_interval":[lo.to_string(),hi.to_string()],"classification":"degenerate_J_or_K","J_zero":contains_root(&gj),"K_zero":contains_root(&gk)}));
                break;
            }
            if !j.evaluate(lo).is_zero()
                && !j.evaluate(hi).is_zero()
                && !k.evaluate(lo).is_zero()
                && !k.evaluate(hi).is_zero()
                && !contains_root(j)
                && !contains_root(k)
            {
                let js = if j.evaluate(lo).is_negative() { -1 } else { 1 };
                let ks = if k.evaluate(lo).is_negative() { -1 } else { 1 };
                rows.push(json!({"x4_interval":[lo.to_string(),hi.to_string()],"classification":"unique_common_x2","J_sign":js,"K_sign":ks,"common_x2_sign":-js*ks}));
                break;
            }
            ensure!(attempt < 4095, "common-root sign refinement cap");
            refine_once(&mut root);
        }
    }
    Ok(rows)
}
fn numeric_fibers(a: &Algebra, r: &Poly, c0: &Poly, j: &Poly, k: &Poly) -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    for s in ["5", "10", "100", "103/21"] {
        for coordinates in [
            ["1", "1", "1", "1"],
            ["1", "2", "3", "4"],
            ["1", "1", "1/10", "1/10"],
            ["1/4", "2", "1", "3"],
            ["1/1000", "5", "1", "1"],
            ["1/1000", "10", "1", "1"],
            ["1/1000", "100", "1", "1"],
        ] {
            let x = &rational(coordinates[0])? + &rational(coordinates[2])?;
            let y = &rational(coordinates[1])? + &rational(coordinates[3])?;
            let threshold = (&x + &y).pow(2) / (&x * &y);
            let mut us = ["5", "12", "30", "100", "104", "395/63"]
                .into_iter()
                .map(rational)
                .collect::<Result<Vec<_>>>()?;
            for offset in ["1/10000", "1/100", "1", "10"] {
                us.push(&threshold + &rational(offset)?);
            }
            us.push(&rational("395/63")? + &rational("1/10000")?);
            us.push(&rational("395/63")? - &rational("1/10000")?);
            for u in us {
                let u_string = u.to_string();
                let substitutions = [
                    (0, s),
                    (1, u_string.as_str()),
                    (2, coordinates[0]),
                    (4, coordinates[1]),
                    (6, coordinates[2]),
                    (7, coordinates[3]),
                ];
                let mut f = r.clone();
                let mut c = c0.clone();
                let mut jf = j.clone();
                let mut kf = k.clone();
                for (axis, value) in substitutions {
                    let q = rational(value)?;
                    f = f.replace(axis, &q);
                    c = c.replace(axis, &q);
                    jf = jf.replace(axis, &q);
                    kf = kf.replace(axis, &q);
                }
                if c.get_constant() >= Rational::zero() {
                    continue;
                }
                let fiber = f.to_univariate_from_univariate(5);
                let radius = fiber
                    .coefficients()
                    .iter()
                    .take(fiber.degree())
                    .map(|c| c.abs() / fiber.lcoeff().abs())
                    .max()
                    .unwrap_or_default()
                    + Rational::from(2);
                let common = classify_positive_common_roots(
                    &fiber,
                    &jf.to_univariate_from_univariate(5),
                    &kf.to_univariate_from_univariate(5),
                )?;
                rows.push(json!({"s":s,"u":u_string,"x1_x3_x5_x7":coordinates,"C0":c.get_constant().to_string(),"fiber":a.format(&f),"degree":fiber.degree(),"positive_roots":roots_in_open_interval(&fiber,&Rational::zero(),&radius),"negative_roots":roots_in_open_interval(&fiber,&(-radius),&Rational::zero()),"positive_variations":variations(&fiber),"endpoint":fiber.evaluate(&Rational::zero()).to_string(),"common_x2_classification":common}));
            }
        }
    }
    Ok(rows)
}
fn positive_collision(a: &Algebra, slope: &Poly, endpoint: &Poly, r: &Poly) -> Result<Value> {
    let mut q = slope.clone();
    let mut b = endpoint.clone();
    let mut fiber = r.clone();
    for (axis, value) in [
        (0, "103/21"),
        (1, "395/63"),
        (2, "1"),
        (4, "1"),
        (6, "1"),
        (7, "1"),
    ] {
        let value = rational(value)?;
        q = q.replace(axis, &value);
        b = b.replace(axis, &value);
        fiber = fiber.replace(axis, &value);
    }
    q = q.replace(5, &Rational::one());
    b = b.replace(5, &Rational::one());
    ensure!(
        q == a.parse("x2^2+(286/63)*x2-40/63")? && b == q.clone().mul_coeff(Rational::from(3)),
        "positive collision identity"
    );
    ensure!(
        fiber.replace(5, &Rational::one()).is_zero()
            && fiber.derivative(5).replace(5, &Rational::one()).is_zero(),
        "positive resultant root is repeated"
    );
    ensure!(
        !fiber
            .derivative(5)
            .derivative(5)
            .replace(5, &Rational::one())
            .is_zero(),
        "root has multiplicity exactly two"
    );
    let factors = symgcad::algebra::factors_checked(&[fiber.clone()])?;
    Ok(
        json!({"s":"103/21","u":"395/63","x1_x3_x5_x7_x4":"1","A":a.format(&q),"B":"3*A","R":a.format(&fiber),"R_canonical_factors":factors.iter().map(|p|a.format(p)).collect::<Vec<_>>(),"positive_R_root":"1","multiplicity":2,"A_positive_roots":1,"A_negative_roots":1,"C0":"-572/63","strict_original_domain":true}),
    )
}
fn discriminant_slice(a: &Algebra, r: &Poly, c0: &Poly) -> Result<Value> {
    let started = Instant::now();
    let mut slice = r.clone();
    let mut domain = c0.clone();
    for (axis, value) in [(0, "5"), (1, "104"), (2, "1/1000"), (6, "1"), (7, "1")] {
        let q = rational(value)?;
        slice = slice.replace(axis, &q);
        domain = domain.replace(axis, &q);
    }
    let at_zero = slice.replace(5, &Rational::zero());
    ensure!(
        at_zero.degree(4) == 2 && at_zero.univariate_lcoeff(4).get_constant() > Rational::zero(),
        "endpoint convexity"
    );
    ensure!(
        domain.degree(4) == 2 && domain.univariate_lcoeff(4).get_constant() > Rational::zero(),
        "C0 convexity"
    );
    let mut counts = Vec::new();
    for value in [5, 10] {
        let q = Rational::from(value);
        ensure!(
            at_zero.replace(4, &q).get_constant() < Rational::zero()
                && domain.replace(4, &q).get_constant() < Rational::zero(),
            "strict path bounds"
        );
        let fiber = slice.replace(4, &q).to_univariate_from_univariate(5);
        let radius = fiber
            .coefficients()
            .iter()
            .take(fiber.degree())
            .map(|c| c.abs() / fiber.lcoeff().abs())
            .max()
            .unwrap_or_default()
            + Rational::from(2);
        counts.push(roots_in_open_interval(&fiber, &Rational::zero(), &radius));
    }
    ensure!(counts == [3, 1], "positive root-count change");
    let disc = discriminant(&slice, 5)?;
    let disc_seconds = started.elapsed().as_secs_f64();
    let factors = symgcad::algebra::factors_checked(&[disc.clone()])?;
    let rows=factors.iter().map(|p|json!({"polynomial":a.format(p),"degree":p.degree(4),"terms":p.nterms(),"roots_between5and10":roots_in_open_interval(&p.to_univariate_from_univariate(4),&Rational::from(5),&Rational::from(10))})).collect::<Vec<_>>();
    Ok(
        json!({"s":"5","u":"104","x1":"1/1000","x5_x7":"1","remaining_variables":["x3","x4"],"resultant_slice":a.format(&slice),"endpoint_path":a.format(&at_zero),"C0_path":a.format(&domain),"positive_root_counts_at_x3_5_10":counts,"discriminant":a.format(&disc),"discriminant_degree_x3":disc.degree(4),"discriminant_terms":disc.nterms(),"discriminant_seconds":disc_seconds,"total_seconds":started.elapsed().as_secs_f64(),"factors":rows}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 120.0 && args.seconds.is_finite(),
        "invalid budget"
    );
    ensure!(!args.output.exists(), "fresh output required");
    if !args.worker {
        let mut command = Command::new("taskset");
        command
            .args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .arg("--worker")
            .arg("--input")
            .arg(&args.input)
            .arg("--output")
            .arg(&args.output)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        if args.discriminant {
            command.arg("--discriminant");
        }
        if args.slice_discriminant {
            command.arg("--slice-discriminant");
        }
        let outcome = symgcad::benchmark::run_bounded(
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
            &serde_json::to_value(outcome)?,
        )?;
        return Ok(());
    }
    let bytes = fs::read(&args.input)?;
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == INPUT_HASH,
        "input research report checksum"
    );
    let input: Value = serde_json::from_slice(&bytes)?;
    let names: Vec<String> = serde_json::from_value(input["variables"].clone())?;
    let a = Algebra::new(&names)?;
    let from = |key: &str| {
        a.parse(
            input[key]
                .as_str()
                .with_context(|| format!("missing {key}"))?,
        )
    };
    let slope = from("A")?;
    let endpoint = from("B")?;
    let r = from("resultant_A_B")?;
    ensure!(
        exact_resultant(&slope, &endpoint, 3)? == r,
        "input resultant identity"
    );
    let mut symmetric_a = slope.clone();
    let mut symmetric_b = endpoint.clone();
    for axis in [2, 4, 6, 7] {
        symmetric_a = symmetric_a.replace(axis, &Rational::one());
        symmetric_b = symmetric_b.replace(axis, &Rational::one());
    }
    ensure!(
        symmetric_a == a.parse("(x2+x4)^2+(67/9)*(x2+x4)+16-4*u-s*x2*x4")?,
        "original symmetric A identity"
    );
    ensure!(
        symmetric_b == a.parse("x2*x4*(x2+x4+31/9)+2*(x2+x4)^2+(103/9)*(x2+x4)+16-u*(x2+x4+4)")?,
        "original symmetric B identity"
    );
    let started = Instant::now();
    let family = counter_family()?;
    let collision = positive_collision(&a, &slope, &endpoint, &r)?;
    let rows = numeric_fibers(
        &a,
        &r,
        &from("C0")?,
        &from("B_minus_bA_linear")?,
        &from("B_minus_bA_constant")?,
    )?;
    let endpoint_r = r.replace(5, &Rational::zero());
    let leading = r.univariate_lcoeff(5);
    let basic_seconds = started.elapsed().as_secs_f64();
    let mut report = json!({"schema_version":1,"research_only":true,"coverage_claim":false,"input_sha256":INPUT_HASH,"variables":names,"resultant_terms":r.nterms(),"resultant_degrees":(0..a.names.len()).map(|i|r.degree(i)).collect::<Vec<_>>(),"endpoint":a.format(&endpoint_r),"leading":a.format(&leading),"endpoint_terms":endpoint_r.nterms(),"leading_terms":leading.nterms(),"common_positive_root_family":family,"positive_discriminant_seam":collision,"numeric_fibers":rows,"basic_seconds":basic_seconds,"stage":"basic_complete"});
    save(&args.output, &report)?;
    let factor_start = Instant::now();
    let small_factors =
        symgcad::algebra::factors_checked(&[endpoint_r, leading, from("B_discriminant_x2")?])?;
    report["small_projection_factors"] = json!(
        small_factors
            .iter()
            .map(|p| a.format(p))
            .collect::<Vec<_>>()
    );
    report["small_factor_seconds"] = json!(factor_start.elapsed().as_secs_f64());
    report["stage"] = json!("small_factors_complete");
    save(&args.output, &report)?;
    if args.slice_discriminant {
        report["stage"] = json!("slice_discriminant_running");
        save(&args.output, &report)?;
        report["discriminant_slice"] = discriminant_slice(&a, &r, &from("C0")?)?;
        report["stage"] = json!("slice_complete");
        save(&args.output, &report)?;
    }
    if args.discriminant {
        let started = Instant::now();
        report["stage"] = json!("discriminant_running");
        save(&args.output, &report)?;
        let disc = discriminant(&r, 5)?;
        report["discriminant_seconds"] = json!(started.elapsed().as_secs_f64());
        report["discriminant"] = json!(a.format(&disc));
        report["discriminant_terms"] = json!(disc.nterms());
        report["stage"] = json!("discriminant_factor_running");
        save(&args.output, &report)?;
        let started = Instant::now();
        let factors = symgcad::algebra::factors_checked(&[disc])?;
        report["discriminant_factors"] =
            json!(factors.iter().map(|p| a.format(p)).collect::<Vec<_>>());
        report["discriminant_factor_seconds"] = json!(started.elapsed().as_secs_f64());
        report["stage"] = json!("complete");
        save(&args.output, &report)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_root_sign_checks_both_coordinates() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        let p = |s: &str| Ok::<_, anyhow::Error>(a.parse(s)?.to_univariate_from_univariate(0));
        let rows = classify_positive_common_roots(&p("(x-1)*(x-2)")?, &p("1")?, &p("2*x-3")?)?;
        ensure!(
            rows.len() == 2 && rows[0]["common_x2_sign"] == 1 && rows[1]["common_x2_sign"] == -1,
            "common-coordinate signs"
        );
        let degenerate = classify_positive_common_roots(&p("x-1")?, &p("x-1")?, &p("x-1")?)?;
        ensure!(
            degenerate[0]["classification"] == "degenerate_J_or_K",
            "coincident quadratics need separate treatment"
        );
        Ok(())
    }
    #[test]
    fn common_root_family_is_exact_and_strict() -> Result<()> {
        let r = counter_family()?;
        ensure!(r["claim"].as_str().is_some(), "claim");
        Ok(())
    }
    #[test]
    fn sign_variations_ignore_internal_zero() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        ensure!(
            variations(&a.parse("x^6-3*x^3+1")?.to_univariate_from_univariate(0)) == 2,
            "variations"
        );
        Ok(())
    }
}
