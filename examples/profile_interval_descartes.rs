//! Research only: bounded rational interval Descartes guard discovery/costs.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, factors_checked, rational},
    benchmark::run_bounded,
    optimized_projection::OperationCapture,
    roots::{isolate_union, refine_once, sector_sample},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    prefix: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    factor_guards: bool,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 0)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}
fn sign(x: &Rational) -> i8 {
    if x.is_zero() {
        0
    } else if x.is_negative() {
        -1
    } else {
        1
    }
}
fn variations(values: &[Rational]) -> usize {
    values
        .iter()
        .map(sign)
        .filter(|&s| s != 0)
        .fold((0, 0), |(last, n), s| {
            (s, n + usize::from(last != 0 && last != s))
        })
        .1
}
fn choose(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let mut c = 1;
    for i in 1..=k.min(n - k) {
        c = c * (n + 1 - i) as u64 / i as u64;
    }
    c
}
fn powers(x: &Rational, d: usize) -> Vec<Rational> {
    let mut p = vec![Rational::one()];
    for i in 1..=d {
        p.push(&p[i - 1] * x);
    }
    p
}
/// Rows are transformed coefficients, columns are original coefficients.
/// Finite: (1+u)^d p((a+b*u)/(1+u)); tail: p(a+u).
fn weights(d: usize, a: &Rational, b: Option<&Rational>) -> Vec<Vec<Rational>> {
    let ap = powers(a, d);
    let bp = b.map(|b| powers(b, d));
    let mut w = vec![vec![Rational::zero(); d + 1]; d + 1];
    for (k, row) in w.iter_mut().enumerate() {
        for (j, entry) in row.iter_mut().enumerate() {
            if let Some(bp) = &bp {
                for l in k.saturating_sub(d - j)..=j.min(k) {
                    *entry +=
                        (&ap[j - l] * &bp[l]) * Rational::from(choose(j, l) * choose(d - j, k - l));
                }
            } else if j >= k {
                *entry = &ap[j - k] * &Rational::from(choose(j, k));
            }
        }
    }
    w
}
fn transformed(f: &UnivariatePolynomial<Q>, w: &[Vec<Rational>]) -> Vec<Rational> {
    w.iter()
        .map(|row| {
            row.iter()
                .zip(f.coefficients())
                .fold(Rational::zero(), |a, (w, c)| a + w * c)
        })
        .collect()
}
fn height(x: &Rational) -> u64 {
    x.numerator_ref().significant_bits() + x.denominator_ref().significant_bits()
}

#[derive(Clone)]
struct Interval {
    a: Rational,
    b: Option<Rational>,
    depth: usize,
}
struct Probe {
    report: Value,
    guards: Vec<Poly>,
}
fn probe(algebra: &Algebra, p: &Poly, variable: usize, prefix: &[Rational]) -> Result<Probe> {
    ensure!(
        p.degree(variable) <= 32 && p.nterms() <= 4096,
        "research degree/term cap"
    );
    let started = Instant::now();
    let f = algebra.specialize_univariate(p, variable, prefix)?;
    ensure!(
        !f.is_zero()
            && f.degree() == p.degree(variable) as usize
            && !f.evaluate(&Rational::zero()).is_zero(),
        "full degree/nonzero endpoint required"
    );
    let d = f.degree();
    let mut roots = isolate_union(vec![(0, f.clone())])?;
    for root in &mut roots {
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            refine_once(root);
        }
    }
    roots.retain(|r| r.interval.0 > Rational::zero());
    let isolation_seconds = started.elapsed().as_secs_f64();
    let mut boundaries = vec![Rational::zero()];
    boundaries.extend(
        roots
            .windows(2)
            .map(|pair| sector_sample(Some(&pair[0]), Some(&pair[1]))),
    );
    let mut pending = VecDeque::new();
    for (i, a) in boundaries.iter().enumerate() {
        pending.push_back(Interval {
            a: a.clone(),
            b: boundaries.get(i + 1).cloned(),
            depth: 0,
        });
    }
    let mut accepted = Vec::new();
    let mut attempted = 0;
    let mut decline = None;
    while let Some(interval) = pending.pop_front() {
        attempted += 1;
        let w = weights(d, &interval.a, interval.b.as_ref());
        let values = transformed(&f, &w);
        let v = variations(&values);
        if v <= 1 {
            accepted.push((interval, w, values, v));
            continue;
        }
        if accepted.len() + pending.len() + 2 > 64 || interval.depth >= 32 {
            decline = Some("interval/depth budget");
            break;
        }
        let mut cut = if let Some(b) = &interval.b {
            (&interval.a + b) / Rational::from(2)
        } else {
            let after_roots = roots
                .last()
                .map(|r| sector_sample(Some(r), None))
                .unwrap_or_else(Rational::one);
            if after_roots > interval.a {
                after_roots
            } else {
                &interval.a * &Rational::from(2) + Rational::one()
            }
        };
        // Never put a rational root at an unguarded interval endpoint.
        let mut shifts = 0;
        while f.evaluate(&cut).is_zero() && shifts < 32 {
            cut = (&interval.a + &cut) / Rational::from(2);
            shifts += 1;
        }
        if height(&cut) > 256 || f.evaluate(&cut).is_zero() {
            decline = Some("separator height/root budget");
            break;
        }
        pending.push_back(Interval {
            a: interval.a.clone(),
            b: Some(cut.clone()),
            depth: interval.depth + 1,
        });
        pending.push_back(Interval {
            a: cut,
            b: interval.b,
            depth: interval.depth + 1,
        });
    }
    accepted.sort_by(|a, b| a.0.a.cmp(&b.0.a));
    let count = accepted.iter().map(|x| x.3).sum::<usize>();
    let mut report = json!({"eligible":false,"degree":d,"terms":p.nterms(),"global_variations":variations(f.coefficients()),"positive_distinct_roots":roots.len(),"isolation_seconds":isolation_seconds,"numeric_discovery_seconds":started.elapsed().as_secs_f64(),"attempted_intervals":attempted,"accepted_intervals":accepted.len(),"sum_interval_variations":count,"declined":decline});
    if decline.is_some() || count != roots.len() {
        return Ok(Probe {
            report,
            guards: vec![],
        });
    }
    let construction = Instant::now();
    let zero = algebra.parse("0")?;
    let mut coefficients = vec![zero.clone(); d + 1];
    for (c, e) in p.to_univariate_polynomial_list(variable) {
        coefficients[e as usize] = c;
    }
    let mut guards = Vec::new();
    let mut registry = HashMap::new();
    let mut intervals = Vec::new();
    let mut zero_obstructions = 0;
    for (interval, w, values, v) in accepted {
        let mut indices = Vec::new();
        let mut signs = Vec::new();
        for (k, row) in w.iter().enumerate() {
            let q = row
                .iter()
                .zip(&coefficients)
                .filter(|(weight, _)| !weight.is_zero())
                .fold(zero.clone(), |acc, (weight, c)| {
                    acc + c.clone().mul_coeff(weight.clone())
                });
            ensure!(
                q.replace_all(prefix) == values[k],
                "symbolic interval coefficient differs from numeric transform"
            );
            if q.is_zero() {
                indices.push(None);
                signs.push(0);
                continue;
            }
            if values[k].is_zero() {
                zero_obstructions += 1;
            }
            signs.push(sign(&values[k]));
            let q = canonical(&q);
            let index = if let Some(&index) = registry.get(&q) {
                index
            } else {
                let index = guards.len();
                registry.insert(q.clone(), index);
                guards.push(q);
                index
            };
            indices.push(Some(index));
        }
        intervals.push(json!({"lower":interval.a.to_string(),"upper":interval.b.map(|x|x.to_string()),"variations":v,"coefficient_signs":signs,"guard_indices":indices,"subdivision_depth":interval.depth}));
    }
    report["intervals"] = json!(intervals);
    report["symbolic_guard_seconds"] = json!(construction.elapsed().as_secs_f64());
    report["sample_zero_guard_obstructions"] = json!(zero_obstructions);
    report["unique_guards"] = json!(guards.len());
    report["guards"]=json!(guards.iter().enumerate().map(|(index,g)|json!({"index":index,"terms":g.nterms(),"maximum_coefficient_bits":g.coefficients.iter().map(height).max().unwrap_or(0),"degrees":(0..g.nvars()).map(|i|g.degree(i)).collect::<Vec<_>>(),"expression":algebra.format(g)})).collect::<Vec<_>>());
    report["eligible"] = json!(zero_obstructions == 0);
    Ok(Probe { report, guards })
}
fn write_atomic(path: &std::path::Path, value: &Value) -> Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
fn work(args: &Args) -> Result<Value> {
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .context("capture omitted operands")?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture input checksum"
    );
    ensure!(operands.len() == 1, "single discriminant operand required");
    let algebra = Algebra::new(&capture.variable_names)?;
    let variable = capture.variable.context("variable missing")?;
    let p = algebra.parse(&operands[0])?;
    let prefix = args
        .prefix
        .iter()
        .map(|x| rational(x))
        .collect::<Result<Vec<_>>>()?;
    let Probe { mut report, guards } = probe(&algebra, &p, variable, &prefix)?;
    report["capture_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    report["input_hash"] = json!(capture.input_hash);
    report["variable"] = json!(variable);
    report["variables"] = json!(capture.variable_names);
    report["prefix"] = json!(args.prefix);
    report["factorization"] = json!([]);
    report["factorization_complete"] = json!(false);
    let progress = args.output.with_extension("progress.json");
    write_atomic(&progress, &report)?;
    if args.factor_guards && report["eligible"] == true {
        for (index, g) in guards.iter().enumerate() {
            report["active_guard"] = json!(index);
            write_atomic(&progress, &report)?;
            let started = Instant::now();
            let factors = factors_checked(&[g.clone()])?;
            report["factorization"].as_array_mut().unwrap().push(json!({"index":index,"seconds":started.elapsed().as_secs_f64(),"factors":factors.len(),"factor_terms":factors.iter().map(|f|f.nterms()).collect::<Vec<_>>()}));
            write_atomic(&progress, &report)?;
        }
        report["factorization_complete"] = json!(true);
        report["active_guard"] = Value::Null;
        write_atomic(&progress, &report)?;
    }
    Ok(report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 120.0,
        "deadline<=120s required"
    );
    if args.worker {
        println!("{}", work(&args)?);
        return Ok(());
    }
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut outcome = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<Value>(&outcome.stdout).ok();
    if result.is_some() {
        outcome.stdout.clear();
    }
    let report = json!({"process":outcome,"result":result,"limits":limits,"cpu":args.cpu,"binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?)),"note":"Research guard-cost diagnostic; no production certificate or original completion claimed."});
    write_atomic(&args.output, &report)?;
    println!(
        "{}",
        json!({"process":report["process"]["status"],"eligible":report["result"]["eligible"],"intervals":report["result"]["accepted_intervals"],"guards":report["result"]["unique_guards"],"factorization_complete":report["result"]["factorization_complete"]})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn attempt(expression: &str) -> Result<Probe> {
        let a = Algebra::new(&["s".into(), "x".into()])?;
        probe(&a, &a.parse(expression)?, 1, &[Rational::from(2)])
    }
    #[test]
    fn mobius_and_tail_weights_match_exact_cleared_evaluations() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        let p = a.parse("2*x^4-3*x^2+x-7")?;
        let f = a.specialize_univariate(&p, 0, &[])?;
        let lo = rational("2/3")?;
        let hi = rational("7/2")?;
        for upper in [None, Some(&hi)] {
            let c = transformed(&f, &weights(f.degree(), &lo, upper));
            for point in ["0", "1/3", "2", "-1/2"] {
                let u = rational(point)?;
                let lhs = c.iter().rev().fold(Rational::zero(), |a, c| a * &u + c);
                let rhs = if let Some(b) = upper {
                    f.evaluate(&((&lo + &(b * &u)) / (Rational::one() + &u)))
                        * powers(&(Rational::one() + &u), f.degree())[f.degree()].clone()
                } else {
                    f.evaluate(&(&lo + &u))
                };
                assert_eq!(lhs, rhs);
            }
        }
        Ok(())
    }
    #[test]
    fn unsaturated_global_variations_reduce_to_zero_one_intervals() -> Result<()> {
        let p = attempt("(x-1)*(x-2)*(x^2-x+1)")?;
        assert_eq!(p.report["global_variations"], 4);
        assert_eq!(p.report["positive_distinct_roots"], 2);
        assert_eq!(p.report["eligible"], true);
        assert_eq!(p.report["sum_interval_variations"], 2);
        let empty = attempt("x^2-x+1")?;
        assert_eq!(empty.report["eligible"], true);
        assert_eq!(empty.report["sum_interval_variations"], 0);
        Ok(())
    }
    #[test]
    fn repeated_positive_root_does_not_get_a_simple_root_certificate() -> Result<()> {
        assert_eq!(attempt("(x-1)^2*(x^2+1)")?.report["eligible"], false);
        Ok(())
    }
}
