//! Research only: finite moving-scale Descartes covers, never solver evidence.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, rational},
    benchmark::run_bounded,
    optimized_projection::OperationCapture,
    roots::roots_in_open_interval,
    solver::sign,
};

const MAX_TERMS: usize = 65536;
const MAX_PRODUCTS: usize = 1_000_000;
const MAX_BITS: u64 = 512;

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    /// Repeat for several earlier-coordinate prefixes of the same polynomial.
    #[arg(long, allow_hyphen_values = true, required = true)]
    prefix: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 60.)]
    seconds: f64,
    #[arg(long, default_value_t = 3)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

fn variations(v: &[Rational]) -> usize {
    let s = v.iter().map(sign).filter(|s| *s != 0).collect::<Vec<_>>();
    s.windows(2).filter(|w| w[0] != w[1]).count()
}
fn choose(n: usize, k: usize) -> u64 {
    let mut r = 1;
    for i in 1..=k.min(n - k) {
        r = r * (n + 1 - i) as u64 / i as u64;
    }
    r
}
fn weights(d: usize, a: &Rational, b: Option<&Rational>) -> Vec<Vec<Rational>> {
    (0..=d)
        .map(|k| {
            (0..=d)
                .map(|j| {
                    if let Some(b) = b {
                        (k.saturating_sub(d - j)..=j.min(k)).fold(Rational::zero(), |s, l| {
                            s + a.pow((j - l) as u64)
                                * b.pow(l as u64)
                                * Rational::from(choose(j, l) * choose(d - j, k - l))
                        })
                    } else if j >= k {
                        a.pow((j - k) as u64) * Rational::from(choose(j, k))
                    } else {
                        Rational::zero()
                    }
                })
                .collect()
        })
        .collect()
}
fn families() -> Vec<Vec<Rational>> {
    [
        vec!["1/4", "1/2", "1", "2", "4", "8"],
        vec!["3/8", "3/4", "3/2", "3", "6", "12"],
        vec!["1/4", "1/2", "3/4", "1", "3/2", "2", "3", "4", "6", "8"],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(|s| rational(s).unwrap()).collect())
    .collect()
}
fn checked(p: Poly) -> Result<Poly> {
    ensure!(p.nterms() <= MAX_TERMS, "polynomial term cap");
    ensure!(
        p.coefficients
            .iter()
            .all(
                |c| c.numerator_ref().significant_bits() + c.denominator_ref().significant_bits()
                    <= MAX_BITS
            ),
        "coefficient height cap"
    );
    Ok(p)
}
fn product(a: &Poly, b: &Poly) -> Result<Poly> {
    ensure!(
        a.nterms().saturating_mul(b.nterms()) <= MAX_PRODUCTS,
        "product-work cap"
    );
    checked(a * b)
}
fn powers(p: &Poly, d: usize) -> Result<Vec<Poly>> {
    let mut v = vec![p.one()];
    for i in 1..=d {
        v.push(product(&v[i - 1], p)?);
    }
    Ok(v)
}
fn positive(p: &Poly, prefix: &[Rational]) -> Option<Poly> {
    let v = p.replace_all(prefix);
    if v.is_zero() {
        None
    } else if v.is_negative() {
        Some(-p.clone())
    } else {
        Some(p.clone())
    }
}
fn coefficients(p: &Poly, variable: usize) -> Vec<Poly> {
    let mut a = vec![p.zero(); p.degree(variable) as usize + 1];
    for (c, i) in p.to_univariate_polynomial_list(variable) {
        a[i as usize] = c;
    }
    a
}
fn numeric_cover(
    f: &UnivariatePolynomial<Q>,
    scale: &Rational,
    knots: &[Rational],
) -> Result<Option<Value>> {
    ensure!(*scale > Rational::zero(), "positive scale required");
    let d = f.degree();
    ensure!(
        d <= 32 && (knots.len() + 1) * (d + 1) <= 256,
        "degree/slot cap"
    );
    let numeric = f
        .coefficients()
        .iter()
        .enumerate()
        .map(|(i, a)| a * &scale.pow(i as u64))
        .collect::<Vec<_>>();
    let leading = f.lcoeff().abs();
    ensure!(!leading.is_zero(), "zero leading coefficient");
    let bound = f
        .coefficients()
        .iter()
        .take(d)
        .map(|c| c.abs() / &leading)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let mut intervals = Vec::new();
    let mut lower = Rational::zero();
    let mut total = 0;
    for upper in knots.iter().map(Some).chain([None]) {
        let rows = weights(d, &lower, upper);
        let values = rows
            .iter()
            .map(|row| {
                row.iter()
                    .zip(&numeric)
                    .fold(Rational::zero(), |s, (w, c)| s + w * c)
            })
            .collect::<Vec<_>>();
        let count = variations(&values);
        if count > 1
            || f.evaluate(&(&lower * scale)).is_zero()
            || upper.is_some_and(|u| f.evaluate(&(u * scale)).is_zero())
        {
            return Ok(None);
        }
        let lo = &lower * scale;
        let hi = upper
            .map(|u| u * scale)
            .unwrap_or_else(|| bound.clone().max(&lo + &Rational::one()));
        ensure!(
            roots_in_open_interval(f, &lo, &hi) == count,
            "interval count/ordinal mismatch"
        );
        total += count;
        intervals.push(
            json!({"lower":lower.to_string(),"upper":upper.map(ToString::to_string),
            "variations":count,"coefficient_signs":values.iter().map(sign).collect::<Vec<_>>()}),
        );
        if let Some(u) = upper {
            lower = u.clone();
        }
    }
    ensure!(
        total == roots_in_open_interval(f, &Rational::zero(), &bound),
        "cover count mismatch"
    );
    Ok(Some(
        json!({"intervals":intervals,"positive_roots":total,"original_positive_ordinals_preserved":true}),
    ))
}

fn symbolic_cover(
    algebra: &Algebra,
    p: &Poly,
    axis: usize,
    numerator: &Poly,
    denominator: &Poly,
    prefix: &[Rational],
    knots: &[Rational],
) -> Result<Value> {
    let started = Instant::now();
    let a = coefficients(p, axis);
    let d = a.len() - 1;
    ensure!(
        numerator.degree(axis) == 0 && denominator.degree(axis) == 0,
        "scale axis support"
    );
    ensure!(
        numerator.replace_all(prefix) > Rational::zero()
            && denominator.replace_all(prefix) > Rational::zero(),
        "scale sign premises"
    );
    let np = powers(numerator, d)?;
    let dp = powers(denominator, d)?;
    let scaled = (0..=d)
        .map(|i| product(&product(&a[i], &np[i])?, &dp[d - i]))
        .collect::<Result<Vec<_>>>()?;
    let z = algebra.parse(&algebra.names[axis])?;
    let mut registry = HashSet::new();
    let mut guards = Vec::new();
    let mut total_terms = 0;
    let mut max_degrees = vec![0u32; algebra.names.len()];
    let mut add_guard = |guard: &Poly| -> Result<()> {
        if guard.is_zero() {
            return Ok(());
        }
        ensure!(
            !guard.replace_all(prefix).is_zero(),
            "nonzero coefficient vanishes at prefix"
        );
        let guard = canonical(guard);
        if registry.insert(guard.clone()) {
            total_terms += guard.nterms();
            ensure!(
                guards.len() < 256 && total_terms <= MAX_TERMS,
                "guard inventory cap"
            );
            for (i, v) in max_degrees.iter_mut().enumerate() {
                *v = (*v).max(guard.degree(i));
            }
            guards.push(json!({"terms":guard.nterms(),"expression":algebra.format(&guard)}));
        }
        Ok(())
    };
    for guard in [numerator, denominator, &a[0], &a[d]] {
        add_guard(guard)?;
    }
    let mut lower = Rational::zero();
    let mut covers = Vec::new();
    for upper in knots.iter().map(Some).chain([None]) {
        let matrix = weights(d, &lower, upper);
        let transformed = matrix
            .iter()
            .map(|row| {
                checked(
                    row.iter()
                        .zip(&scaled)
                        .fold(p.zero(), |s, (w, a)| s + a.clone().mul_coeff(w.clone())),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let affine = p.one().mul_coeff(lower.clone())
            + z.clone()
                .mul_coeff(upper.cloned().unwrap_or_else(Rational::one));
        let af = powers(&affine, d)?;
        let denominator_affine = if upper.is_some() {
            &p.one() + &z
        } else {
            p.one()
        };
        let da = powers(&denominator_affine, d)?;
        let direct = (0..=d).try_fold(p.zero(), |s, i| {
            checked(s + product(&product(&scaled[i], &af[i])?, &da[d - i])?)
        })?;
        let reconstructed = transformed
            .iter()
            .enumerate()
            .try_fold(p.zero(), |s, (i, c)| checked(s + c * &z.pow(i)))?;
        ensure!(
            direct == reconstructed,
            "exact cleared Mobius/tail identity mismatch"
        );
        let values = transformed
            .iter()
            .map(|c| c.replace_all(prefix))
            .collect::<Vec<_>>();
        ensure!(
            variations(&values) <= 1,
            "symbolic coefficient count mismatch"
        );
        for c in &transformed {
            add_guard(c)?;
        }
        covers.push(json!({"lower":lower.to_string(),"upper":upper.map(ToString::to_string),"coefficient_signs":values.iter().map(sign).collect::<Vec<_>>(),"identity_checked":true}));
        if let Some(u) = upper {
            lower = u.clone();
        }
    }
    Ok(
        json!({"guard_count":guards.len(),"guard_terms":total_terms,"max_guard_degrees":max_degrees,
        "guards":guards,"intervals":covers,"seconds":started.elapsed().as_secs_f64(),
        "numerator":algebra.format(numerator),"denominator":algebra.format(denominator),
        "scope":"Exact local certificate proposal only; all strict sign premises still need ancestor-cell proof."}),
    )
}

fn work(args: &Args) -> Result<Value> {
    let bytes = fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let expressions = capture.operands.as_ref().context("missing operands")?;
    ensure!(expressions.len() == 1, "one operand required");
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(expressions)?)),
        "capture checksum"
    );
    let axis = capture.variable.context("missing axis")?;
    let algebra = Algebra::new(&capture.variable_names)?;
    let p = algebra.parse(&expressions[0])?;
    ensure!(p.nterms() <= 4096 && p.degree(axis) <= 32, "input caps");
    ensure!(
        p.exponents_iter()
            .all(|e| e[axis + 1..].iter().all(|x| *x == 0)),
        "later-axis input support"
    );
    let a = coefficients(&p, axis);
    let d = a.len() - 1;
    let mut recipes = vec![("constant_one".to_owned(), p.one(), p.one())];
    for i in 0..axis {
        if p.degree(i) > 0 {
            recipes.push((
                format!("coordinate_{}", algebra.names[i]),
                algebra.parse(&algebra.names[i])?,
                p.one(),
            ));
        }
    }
    for i in (0..d).rev().take(2) {
        if a[i].nterms() <= 8 && a[d].nterms() <= 8 {
            recipes.push((
                format!("coefficient_{i}_over_{d}"),
                a[i].clone(),
                a[d].clone(),
            ));
        }
    }
    let mut prefixes = Vec::new();
    for text in &args.prefix {
        let prefix = text.split(',').map(rational).collect::<Result<Vec<_>>>()?;
        ensure!(prefix.len() == axis, "prefix dimension mismatch");
        let fiber = algebra.specialize_univariate(&p, axis, &prefix)?;
        ensure!(
            fiber.degree() == d && !fiber.evaluate(&Rational::zero()).is_zero(),
            "degree/endpoint seam"
        );
        let mut rows = Vec::new();
        for (name, numerator, denominator) in &recipes {
            let Some(num) = positive(numerator, &prefix) else {
                rows.push(json!({"recipe":name,"decline":"numerator seam"}));
                continue;
            };
            let Some(den) = positive(denominator, &prefix) else {
                rows.push(json!({"recipe":name,"decline":"denominator seam"}));
                continue;
            };
            let scale = num.replace_all(&prefix) / den.replace_all(&prefix);
            let mut chosen = None;
            for (family, knots) in families().iter().enumerate() {
                if let Some(numeric) = numeric_cover(&fiber, &scale, knots)? {
                    chosen = Some((family, knots.clone(), numeric));
                    break;
                }
            }
            let Some((family, knots, numeric)) = chosen else {
                rows.push(
                    json!({"recipe":name,"scale":scale.to_string(),"numeric_accepted":false}),
                );
                continue;
            };
            // Only the constant control and coefficient scale warrant symbolic
            // expansion here. Coordinate rows are inexpensive numeric evidence.
            let symbolic = if name == "constant_one" || name.starts_with("coefficient_") {
                match symbolic_cover(&algebra, &p, axis, &num, &den, &prefix, &knots) {
                    Ok(proof) => json!({"accepted":true,"proof":proof}),
                    Err(error) => json!({"accepted":false,"decline":error.to_string()}),
                }
            } else {
                Value::Null
            };
            rows.push(json!({"recipe":name,"scale":scale.to_string(),"numeric_accepted":true,"family":family,"numeric":numeric,"symbolic":symbolic}));
        }
        prefixes.push(json!({"prefix":text,"rows":rows}));
    }
    Ok(
        json!({"capture_sha256":format!("{:x}",Sha256::digest(bytes)),"prefixes":prefixes,
        "fixed_knots":families().iter().map(|v|v.iter().map(ToString::to_string).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "original_completion":false}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 60.,
        "deadline cap"
    );
    if args.worker {
        println!("{}", work(&args)?);
        return Ok(());
    }
    ensure!(!args.output.exists(), "fresh output required");
    if let Some(p) = args.output.parent() {
        fs::create_dir_all(p)?;
    }
    let exe = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .args(std::env::args_os().skip(1))
        .arg("--worker");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 8192,
        workers: 1,
        max_cells: None,
    };
    let mut process = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<Value>(&process.stdout).ok();
    if result.is_some() {
        process.stdout.clear();
    }
    fs::write(
        &args.output,
        serde_json::to_vec_pretty(&json!({"process":process,"result":result,
        "binary_sha256":format!("{:x}",Sha256::digest(fs::read(exe)?)),"limits":limits,"cpu":args.cpu}))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_cover_checks_exact_identities_and_original_positive_ordinals() -> Result<()> {
        let algebra = Algebra::new(&["s".into(), "z".into()])?;
        let p = algebra.parse("(z-s)*(z-2*s)*(z^2+s^2)")?;
        let numerator = algebra.parse("s")?;
        let knots = vec![rational("3/2")?, rational("3")?];
        for s in [1, 2, 100] {
            let prefix = [Rational::from(s)];
            let fiber = algebra.specialize_univariate(&p, 1, &prefix)?;
            assert_eq!(
                numeric_cover(&fiber, &prefix[0], &knots)?.unwrap()["positive_roots"],
                2
            );
            let proof = symbolic_cover(&algebra, &p, 1, &numerator, &p.one(), &prefix, &knots)?;
            assert!(
                proof["intervals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v["identity_checked"] == true)
            );
        }
        Ok(())
    }
    #[test]
    fn signs_seams_and_nonzero_sample_guards_cannot_be_skipped() -> Result<()> {
        let algebra = Algebra::new(&["s".into(), "z".into()])?;
        let s = algebra.parse("s")?;
        assert!(positive(&s, &[Rational::zero()]).is_none());
        assert_eq!(positive(&s, &[Rational::from(-2)]).unwrap(), -s.clone());
        let p = algebra.parse("(z-1)*(z-2)*(z^2+1)")?;
        assert!(
            symbolic_cover(
                &algebra,
                &p,
                1,
                &s,
                &p.one(),
                &[Rational::from(-1)],
                &[rational("3/2")?]
            )
            .is_err()
        );
        let f = algebra.specialize_univariate(&p, 1, &[Rational::one()])?;
        assert!(numeric_cover(&f, &Rational::one(), &[Rational::one()])?.is_none());
        Ok(())
    }
}
