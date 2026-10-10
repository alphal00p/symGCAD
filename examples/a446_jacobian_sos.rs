//! Bounded exact SOS research on previously checked A446 Jacobian chart residuals.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::algebra::{Algebra, Poly, rational};

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
    #[arg(long, default_value_t = 300.0)]
    seconds: f64,
    #[arg(long, default_value_t = 48)]
    strategies: usize,
    #[arg(long, default_value_t = 0)]
    strategy_start: usize,
    #[arg(long, default_value_t = 128)]
    counterexample_samples: usize,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Square {
    weight: String,
    common: Vec<u32>,
    left: Vec<u32>,
    right: Vec<u32>,
    ratio: String,
}
struct Source {
    label: String,
    scale: Poly,
    cv: Poly,
    cg: Poly,
    residual: Poly,
}
struct Inputs {
    algebra: Algebra,
    f: Poly,
    v: Poly,
    g: Poly,
    sources: Vec<Source>,
}
fn nonnegative(p: &Poly) -> bool {
    p.coefficients.iter().all(|x| *x > Rational::zero())
}
fn strictly_positive(p: &Poly) -> bool {
    !p.is_zero() && nonnegative(p)
}
fn negative_count(p: &Poly) -> usize {
    p.coefficients
        .iter()
        .filter(|x| **x < Rational::zero())
        .count()
}
fn height(c: &Rational) -> u64 {
    c.numerator_ref().significant_bits() as u64 + c.denominator_ref().significant_bits() as u64
}
fn load(bytes: &[u8]) -> Result<Inputs> {
    ensure!(bytes.len() <= 32 * 1024 * 1024, "input size cap");
    let report: Value = serde_json::from_slice(bytes)?;
    let chart = &report["positive_cone_chart"];
    let names: Vec<String> = serde_json::from_value(chart["variables"].clone())?;
    ensure!(
        names
            == [
                "x1", "x3", "x5", "x7", "x2", "x4", "hh", "rr", "ww", "tt", "vv", "zz"
            ],
        "exact chart map"
    );
    let algebra = Algebra::new(&names)?;
    let parse =
        |v: &Value| -> Result<Poly> { algebra.parse(v.as_str().context("polynomial string")?) };
    let f = -parse(&chart["jacobian"])?;
    let v = parse(&chart["V"])?;
    let g = parse(&chart["s_above_4_numerator"])?;
    let substitutions =
        serde_json::from_value::<Vec<String>>(chart["original_coordinates"].clone())?
            .into_iter()
            .map(|s| algebra.parse(&s))
            .collect::<Result<Vec<_>>>()?;
    ensure!(substitutions.len() == 6, "six chart substitutions");
    let compose = |p: &Poly| -> Result<Poly> {
        let mut p = p.clone();
        for (axis, value) in substitutions.iter().enumerate() {
            ensure!(p.nterms() <= 131072, "composition cap");
            p = p.replace_with_poly(axis, value);
        }
        ensure!(p.nterms() <= 131072, "composition output cap");
        Ok(p)
    };
    ensure!(
        compose(&parse(&report["original_cleared_jacobian"])?)? == -f.clone(),
        "chart Jacobian replay"
    );
    ensure!(compose(&parse(&report["V"])?)? == v, "chart V replay");
    let q = compose(&parse(&report["Q"])?)?;
    let t = compose(&parse(&report["T"])?)?;
    ensure!(
        g == &q * &v - algebra.parse("4*hh*tt*(tt+vv+zz)")? * &t,
        "strict s>4 premise replay"
    );
    let zero = algebra.parse("0")?;
    let mut sources = vec![Source {
        label: "original_chart".into(),
        scale: f.one(),
        cv: zero.clone(),
        cg: zero.clone(),
        residual: f.clone(),
    }];
    let first = &chart["multiplier_search"];
    sources.push(Source {
        label: "unscaled_V_partial".into(),
        scale: f.one(),
        cv: parse(&first["multiplier"])?,
        cg: zero.clone(),
        residual: parse(&first["residual"])?,
    });
    for (index, p) in chart["multiplier_portfolio"]
        .as_array()
        .context("portfolio array")?
        .iter()
        .enumerate()
    {
        let hp = p["h_power"].as_u64().context("h power")? as usize;
        let bp = p["b_power"].as_u64().context("b power")? as usize;
        ensure!(hp + bp <= 2, "known positive scale degree cap");
        sources.push(Source {
            label: format!("portfolio_{index}"),
            scale: algebra.parse("1+hh")?.pow(hp) * &algebra.parse("rr+ww+tt+vv+zz")?.pow(bp),
            cv: parse(&p["multiplier_V"])?,
            cg: parse(&p["multiplier_s_above_4"])?,
            residual: parse(&p["residual"])?,
        });
    }
    for s in &sources {
        ensure!(
            strictly_positive(&s.scale) && nonnegative(&s.cv) && nonnegative(&s.cg),
            "positive certificate multipliers"
        );
        ensure!(
            &s.scale * &f == &s.cv * &v + &s.cg * &g + s.residual.clone(),
            "partial certificate replay"
        );
        ensure!(
            s.residual.nterms() <= 16384 && (0..6).all(|axis| s.residual.degree(axis) == 0),
            "residual support/size cap"
        );
        ensure!(
            [&s.cv, &s.cg, &s.scale]
                .into_iter()
                .all(|p| (0..6).all(|axis| p.degree(axis) == 0)),
            "multiplier chart support"
        );
    }
    Ok(Inputs {
        algebra,
        f,
        v,
        g,
        sources,
    })
}
fn monomial(template: &Poly, c: Rational, e: &[u32]) -> Poly {
    let mut p = template.zero();
    p.append_monomial(c, e);
    p
}
fn replay_square(template: &Poly, s: &Square) -> Result<Poly> {
    ensure!(
        [s.common.len(), s.left.len(), s.right.len()]
            .into_iter()
            .all(|n| n == template.nvars()),
        "square map"
    );
    ensure!(
        s.common
            .iter()
            .chain(&s.left)
            .chain(&s.right)
            .all(|e| *e <= 64),
        "square exponent cap"
    );
    let weight = rational(&s.weight)?;
    let ratio = rational(&s.ratio)?;
    ensure!(
        weight > Rational::zero() && ratio > Rational::zero(),
        "positive square weights"
    );
    let binomial =
        monomial(template, Rational::one(), &s.left) - monomial(template, ratio, &s.right);
    Ok(monomial(template, weight, &s.common) * &binomial.pow(2))
}
#[derive(Clone)]
struct Pair {
    u: Vec<u32>,
    v: Vec<u32>,
    a: Rational,
    b: Rational,
    slack: Rational,
    distance: u32,
}
fn extract(
    f: &Poly,
    strategy: usize,
    deadline: Option<(Instant, f64)>,
) -> Result<(Vec<Square>, Poly, bool)> {
    ensure!(
        f.nterms() <= 16384 && f.exponents_iter().all(|e| e.iter().all(|x| *x <= 64)),
        "SOS input bounds"
    );
    let mut coefficients = f
        .coefficients
        .iter()
        .zip(f.exponents_iter())
        .map(|(c, e)| (e.to_vec(), c.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut negatives = coefficients
        .iter()
        .filter(|(_, c)| **c < Rational::zero())
        .map(|(e, c)| (e.clone(), -c.clone()))
        .collect::<Vec<_>>();
    ensure!(negatives.len() <= 1024, "negative support cap");
    let order = strategy % 4;
    if order == 1 {
        negatives.reverse();
    }
    if order == 2 {
        negatives.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    }
    if order == 3 {
        negatives.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    }
    let ranking = (strategy / 4) % 3;
    let weight_mode = (strategy / 12) % 2;
    let partial = strategy >= 24;
    let mut squares = Vec::new();
    let mut stopped = false;
    'terms: for (e, mut c) in negatives {
        for _ in 0..if partial { 16 } else { 1 } {
            if deadline.is_some_and(|(start, seconds)| start.elapsed().as_secs_f64() >= seconds) {
                stopped = true;
                break 'terms;
            }
            if height(&c) > 4096 {
                break;
            }
            let c2 = &c * &c;
            let mut best: Option<Pair> = None;
            for (u, a) in coefficients.iter().filter(|(_, a)| **a > Rational::zero()) {
                if height(a) > 4096 {
                    continue;
                }
                if u.iter().zip(&e).any(|(u, e)| *u > 2 * e) {
                    continue;
                }
                let v = e.iter().zip(u).map(|(e, u)| 2 * e - u).collect::<Vec<_>>();
                if *u >= v {
                    continue;
                }
                let Some(b) = coefficients.get(&v).filter(|b| **b > Rational::zero()) else {
                    continue;
                };
                if height(b) > 4096 {
                    continue;
                }
                let four_ab = (a * b) * Rational::from(4);
                if four_ab < c2 && !partial {
                    continue;
                }
                let slack = &four_ab / &c2;
                let distance = u.iter().zip(&v).map(|(a, b)| a.abs_diff(*b)).sum();
                let candidate = Pair {
                    u: u.clone(),
                    v,
                    a: a.clone(),
                    b: b.clone(),
                    slack,
                    distance,
                };
                let better = best.as_ref().is_none_or(|old| match ranking {
                    0 => candidate.slack > old.slack,
                    1 => {
                        candidate.distance < old.distance
                            || (candidate.distance == old.distance && candidate.slack > old.slack)
                    }
                    _ => candidate.u > old.u,
                });
                if better {
                    best = Some(candidate);
                }
            }
            let Some(pair) = best else { break };
            let four_ab = (&pair.a * &pair.b) * Rational::from(4);
            let cancelled = if four_ab < c2 {
                let numerator = four_ab.numerator_ref() * four_ab.denominator_ref();
                let numerator = (numerator << 32usize).root(2);
                let denominator = four_ab.denominator_ref() << 16usize;
                Rational::from((numerator, denominator))
            } else {
                c.clone()
            };
            ensure!(
                cancelled > Rational::zero()
                    && cancelled <= c
                    && &cancelled * &cancelled <= four_ab,
                "exact partial AMGM bound"
            );
            let lower = (&cancelled * &cancelled) / (&pair.b * &Rational::from(4));
            let weight = if weight_mode == 0 || cancelled < c {
                pair.a.clone()
            } else {
                (&pair.a + &lower) / Rational::from(2)
            };
            ensure!(weight >= lower && weight <= pair.a, "AMGM interval");
            let ratio = &cancelled / &(&weight * &Rational::from(2));
            let used_b = &weight * &ratio * &ratio;
            ensure!(used_b <= pair.b, "second square coefficient budget");
            let common = pair
                .u
                .iter()
                .zip(&pair.v)
                .map(|(u, v)| (*u).min(*v))
                .collect::<Vec<_>>();
            let left = pair
                .u
                .iter()
                .zip(&common)
                .map(|(u, g)| (u - g) / 2)
                .collect::<Vec<_>>();
            let right = pair
                .v
                .iter()
                .zip(&common)
                .map(|(v, g)| (v - g) / 2)
                .collect::<Vec<_>>();
            ensure!(
                pair.u.iter().zip(&common).all(|(u, g)| (u - g) % 2 == 0)
                    && pair.v.iter().zip(&common).all(|(v, g)| (v - g) % 2 == 0),
                "even square exponents"
            );
            coefficients.insert(pair.u, &pair.a - &weight);
            coefficients.insert(pair.v, &pair.b - &used_b);
            coefficients.insert(e.clone(), &cancelled - &c);
            squares.push(Square {
                weight: weight.to_string(),
                common,
                left,
                right,
                ratio: ratio.to_string(),
            });
            c = c - cancelled;
            if c.is_zero() {
                break;
            }
        }
    }
    let mut residual = f.zero();
    for (e, c) in coefficients {
        if !c.is_zero() {
            residual.append_monomial(c, &e);
        }
    }
    let mut reconstructed = residual.clone();
    for s in &squares {
        reconstructed = &reconstructed + &replay_square(f, s)?;
    }
    ensure!(reconstructed == *f, "exact SOS reconstruction");
    Ok((squares, residual, stopped))
}
fn save(path: &PathBuf, value: &Value) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 3600.0 && args.seconds.is_finite(),
        "wall cap"
    );
    ensure!(
        args.strategies > args.strategy_start && args.strategies <= 48,
        "strategy cap"
    );
    ensure!(
        args.counterexample_samples <= 4096,
        "counterexample sample cap"
    );
    ensure!(!args.output.exists(), "fresh output");
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
            .arg(args.seconds.to_string())
            .arg("--strategies")
            .arg(args.strategies.to_string())
            .arg("--strategy-start")
            .arg(args.strategy_start.to_string())
            .arg("--counterexample-samples")
            .arg(args.counterexample_samples.to_string());
        let result = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 8192,
                workers: 1,
                max_cells: None,
            },
        )?;
        save(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(result)?,
        )?;
        return Ok(());
    }
    let started = Instant::now();
    let bytes = fs::read(&args.input)?;
    let input_hash = format!("{:x}", Sha256::digest(&bytes));
    let inputs = load(&bytes)?;
    let a = &inputs.algebra;
    let mut trials = Vec::new();
    let mut best_count = usize::MAX;
    let mut best = Value::Null;
    let mut success = false;
    let mut report = json!({"research_only":true,"coverage_claim":false,"input_sha256":input_hash,"variables":a.names,"input_identities_replayed":true,"status":"searching"});
    save(&args.output, &report)?;
    let mut counterexamples = BTreeMap::new();
    let values = ["1/100", "1/10", "1", "10", "100"]
        .map(rational)
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    for source in &inputs.sources {
        let mut state = 0x381446u64;
        for _ in 0..args.counterexample_samples {
            if started.elapsed().as_secs_f64() >= args.seconds * 0.90 {
                break;
            }
            let mut point = vec![Rational::one(); a.names.len()];
            for value in &mut point[6..] {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *value = values[state as usize % values.len()].clone();
            }
            let value = source.residual.replace_all(&point);
            if value < Rational::zero() {
                counterexamples.insert(source.label.clone(),json!({"point":point.iter().map(ToString::to_string).collect::<Vec<_>>(),"residual":value.to_string(),"V":inputs.v.replace_all(&point).to_string(),"s_above_4_numerator":inputs.g.replace_all(&point).to_string()}));
                break;
            }
        }
        report["residual_counterexamples"] = json!(counterexamples);
        save(&args.output, &report)?;
    }
    'search: for strategy in args.strategy_start..args.strategies {
        for source in &inputs.sources {
            if counterexamples.contains_key(&source.label) {
                continue;
            }
            if started.elapsed().as_secs_f64() >= args.seconds * 0.95 {
                break 'search;
            }
            let (squares, residual, stopped) = extract(
                &source.residual,
                strategy,
                Some((started, args.seconds * 0.95)),
            )?;
            let count = negative_count(&residual);
            let strict = strictly_positive(&residual);
            trials.push(json!({"source":source.label,"strategy":strategy,"squares":squares.len(),"negative_coefficients":count,"strict_certificate":strict,"stopped":stopped,"elapsed_seconds":started.elapsed().as_secs_f64()}));
            if count < best_count || strict {
                let mut reconstructed =
                    &source.cv * &inputs.v + &source.cg * &inputs.g + residual.clone();
                for square in &squares {
                    reconstructed = &reconstructed + &replay_square(&residual, square)?;
                }
                ensure!(
                    reconstructed == &source.scale * &inputs.f,
                    "full conditional SOS certificate"
                );
                best_count = count;
                best = json!({"source":source.label,"strategy":strategy,"scale":a.format(&source.scale),"multiplier_V":a.format(&source.cv),"multiplier_s_above_4":a.format(&source.cg),"squares":squares,"residual":a.format(&residual),"negative_coefficients":count,"strict_certificate":strict,"full_identity_checked":true});
            }
            report["trials"] = json!(trials);
            report["best"] = best.clone();
            report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
            save(&args.output, &report)?;
            if strict {
                success = true;
                break 'search;
            }
        }
    }
    report["status"] = json!(if success {
        "strict_certificate_found"
    } else {
        "bounded_search_inconclusive"
    });
    save(&args.output, &report)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_rational_and_odd_common_monomial_squares() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        for text in [
            "x^2-2*x*y+y^2+1",
            "2*x^2-3*x*y+2*y^2+1",
            "x^3*y-2*x^2*y^2+x*y^3+1",
            "(2/3)*x^2-x*y+(3/4)*y^2+1",
        ] {
            for strategy in 0..24 {
                let (s, r, _) = extract(&a.parse(text)?, strategy, None)?;
                ensure!(
                    !s.is_empty() && strictly_positive(&r),
                    "strict exact certificate"
                );
            }
        }
        Ok(())
    }
    #[test]
    fn infeasible_pair_and_square_zero_remain_unproved() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let (_, r, _) = extract(&a.parse("x^2-3*x*y+y^2+1")?, 0, None)?;
        ensure!(negative_count(&r) == 1, "inconclusive residual");
        let (_, r, _) = extract(&a.parse("x^2-2*x*y+y^2")?, 0, None)?;
        ensure!(!strictly_positive(&r), "nonnegative is not strict");
        Ok(())
    }
    #[test]
    fn tampered_square_and_out_of_support_rejected() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let p = a.parse("x^2-2*x*y+y^2+1")?;
        let (mut s, r, _) = extract(&p, 0, None)?;
        s[0].ratio = "2".into();
        ensure!(
            replay_square(&p, &s[0])? + r != p,
            "tampering changes identity"
        );
        s[0].weight = "-1".into();
        ensure!(
            replay_square(&p, &s[0]).is_err(),
            "negative weight rejected"
        );
        Ok(())
    }
    #[test]
    fn several_pairs_can_cancel_one_negative_coefficient_exactly() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let p = a.parse("x^4+y^4+x^3*y+x*y^3-3*x^2*y^2+1")?;
        let (_, r, _) = extract(&p, 0, None)?;
        ensure!(negative_count(&r) == 1, "no single pair suffices");
        let (s, r, _) = extract(&p, 24, None)?;
        ensure!(
            s.len() >= 2 && strictly_positive(&r),
            "combined exact AMGM squares"
        );
        Ok(())
    }
}
