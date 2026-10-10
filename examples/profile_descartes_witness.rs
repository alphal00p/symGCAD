//! Research-only saturated Descartes/sign-separator guard construction.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, factors_checked, rational},
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
    #[arg(long, default_value_t = 7)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

fn sign(value: &Rational) -> i8 {
    if value.is_zero() {
        0
    } else if value.is_negative() {
        -1
    } else {
        1
    }
}

fn write_atomic(path: &std::path::Path, value: &Value) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

struct Probe {
    report: Value,
    guards: Vec<(String, Poly)>,
}

fn witness(algebra: &Algebra, p: &Poly, variable: usize, prefix: &[Rational]) -> Result<Probe> {
    ensure!(
        p.degree(variable) <= 32 && p.nterms() <= 4096,
        "research arithmetic cap"
    );
    let started = Instant::now();
    let fiber = algebra.specialize_univariate(p, variable, prefix)?;
    let mut coefficients = p.to_univariate_polynomial_list(variable);
    coefficients.sort_by_key(|(_, degree)| *degree);
    let coefficient_signs: Vec<_> = coefficients
        .iter()
        .map(|(coefficient, _)| sign(&coefficient.replace_all(prefix)))
        .collect();
    let variations = coefficient_signs
        .iter()
        .copied()
        .filter(|&s| s != 0)
        .fold((0, 0), |(last, count), next| {
            (next, count + usize::from(last != 0 && last != next))
        })
        .1;
    let mut report = json!({"degree":p.degree(variable),"terms":p.nterms(),
        "degrees":(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>(),
        "coefficient_degrees":coefficients.iter().map(|(_,d)|d).collect::<Vec<_>>(),
        "coefficient_signs":coefficient_signs,"variations":variations,"eligible":false});
    if coefficient_signs.contains(&0)
        || fiber.is_zero()
        || fiber.coefficients()[0].is_zero()
        || !(2..=8).contains(&variations)
        || fiber.degree() != p.degree(variable) as usize
    {
        report["declined"] = json!("coefficient/endpoint/degree/variation prerequisite");
        return Ok(Probe {
            report,
            guards: vec![],
        });
    }
    let isolation_start = Instant::now();
    let mut roots = isolate_union(vec![(0, fiber.clone())])?;
    for root in &mut roots {
        let mut steps = 0;
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            ensure!(steps < 256, "positive-root separation refinement cap");
            refine_once(root);
            steps += 1;
        }
    }
    let positive: Vec<_> = roots
        .iter()
        .filter(|r| r.interval.0 > Rational::zero())
        .collect();
    report["isolation_seconds"] = json!(isolation_start.elapsed().as_secs_f64());
    report["all_real_roots"] = json!(roots.len());
    report["positive_roots"] = json!(positive.len());
    report["positive_intervals"] = json!(
        positive
            .iter()
            .enumerate()
            .map(|(ordinal, r)| json!({
        "all_real_index":r.index,"positive_index":ordinal,
        "lower":r.interval.0.to_string(),"upper":r.interval.1.to_string()}))
            .collect::<Vec<_>>()
    );
    if positive.len() != variations {
        report["declined"] = json!("positive distinct roots do not saturate Descartes bound");
        return Ok(Probe {
            report,
            guards: vec![],
        });
    }
    let separators: Vec<_> = positive
        .windows(2)
        .map(|pair| sector_sample(Some(pair[0]), Some(pair[1])))
        .collect();
    ensure!(
        separators.iter().all(|c| *c > Rational::zero())
            && separators.windows(2).all(|p| p[0] < p[1]),
        "separator ordering"
    );
    ensure!(
        separators
            .iter()
            .all(
                |c| c.numerator_ref().significant_bits() + c.denominator_ref().significant_bits()
                    <= 256
            ),
        "separator height cap"
    );
    let construction_start = Instant::now();
    let mut guards: Vec<_> = coefficients
        .into_iter()
        .map(|(p, d)| (format!("coefficient_{d}"), p))
        .collect();
    let evaluations: Vec<_> = separators.iter().map(|c| p.replace(variable, c)).collect();
    let mut signs = vec![sign(&fiber.coefficients()[0])];
    for (c, evaluation) in separators.iter().zip(&evaluations) {
        ensure!(!evaluation.is_zero(), "identically zero separator guard");
        let value = evaluation.replace_all(prefix);
        ensure!(
            value == fiber.evaluate(c),
            "symbolic specialization differs"
        );
        signs.push(sign(&value));
    }
    signs.push(sign(&fiber.lcoeff()));
    ensure!(
        signs.iter().all(|&s| s != 0) && signs.windows(2).all(|p| p[0] != p[1]),
        "separator signs do not alternate strictly"
    );
    guards.extend(
        evaluations
            .into_iter()
            .enumerate()
            .map(|(i, p)| (format!("separator_{i}"), p)),
    );
    report["construction_seconds"] = json!(construction_start.elapsed().as_secs_f64());
    report["total_discovery_seconds"] = json!(started.elapsed().as_secs_f64());
    report["separators"] = json!(
        separators
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    report["endpoint_separator_infinity_signs"] = json!(signs);
    report["guards"] = json!(
        guards
            .iter()
            .map(|(label, p)| json!({"label":label,"terms":p.nterms(),
        "degrees":(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>(),
        "maximum_coefficient_bits":p.coefficients.iter().map(|c|c.numerator_ref().significant_bits()
            +c.denominator_ref().significant_bits()).max().unwrap_or(0),
        "expression":algebra.format(p)}))
            .collect::<Vec<_>>()
    );
    report["eligible"] = json!(true);
    Ok(Probe { report, guards })
}

fn work(args: &Args) -> Result<Value> {
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .context("capture omits operands")?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "input checksum differs"
    );
    ensure!(operands.len() == 1, "profile requires one polynomial");
    let variable = capture.variable.context("missing variable")?;
    let algebra = Algebra::new(&capture.variable_names)?;
    let p = algebra.parse(&operands[0])?;
    let prefix = args
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let Probe { mut report, guards } = witness(&algebra, &p, variable, &prefix)?;
    report["capture_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    report["input_hash"] = json!(capture.input_hash);
    report["variables"] = json!(capture.variable_names);
    report["variable"] = json!(variable);
    report["prefix"] = json!(args.prefix);
    report["factorization"] = json!([]);
    report["factorization_complete"] = json!(false);
    let progress = args.output.with_extension("progress.json");
    write_atomic(&progress, &report)?;
    if args.factor_guards && report["eligible"] == true {
        for (label, guard) in guards {
            report["active_guard"] = json!(label);
            write_atomic(&progress, &report)?;
            let started = Instant::now();
            let factors = factors_checked(&[guard])?;
            let seconds = started.elapsed().as_secs_f64();
            report["factorization"].as_array_mut().unwrap().push(json!({"label":label,
                "seconds":seconds,"factors":factors.len(),"factor_terms":factors.iter().map(|p|p.nterms()).collect::<Vec<_>>(),
                "factor_expressions":factors.iter().map(|p|algebra.format(p)).collect::<Vec<_>>() }));
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
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "deadline exceeds120s"
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
    let report = json!({"process":outcome,"result":result,"limits":limits,"cpu":args.cpu,
        "binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?)),
        "note":"Research guard-cost diagnostic, not a complete GCAD or a production certificate."});
    write_atomic(&args.output, &report)?;
    println!("{report}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn probe(expression: &str, sample: &str) -> Probe {
        let a = Algebra::new(&["s".into(), "x".into()]).unwrap();
        witness(
            &a,
            &a.parse(expression).unwrap(),
            1,
            &[rational(sample).unwrap()],
        )
        .unwrap()
    }
    #[test]
    fn exact_four_root_saturation_has_three_strict_separators() {
        let p = probe("(x-1)*(x-2)*(x-3)*(x-4)", "0");
        assert_eq!(p.report["eligible"], true);
        assert_eq!(p.report["variations"], 4);
        assert_eq!(p.report["separators"].as_array().unwrap().len(), 3);
        assert_eq!(
            p.report["endpoint_separator_infinity_signs"],
            json!([1, -1, 1, -1, 1])
        );
    }
    #[test]
    fn repeated_positive_roots_do_not_saturate_distinct_count() {
        let p = probe("(x-1)^2*(x-2)^2", "0");
        assert_eq!(p.report["eligible"], false);
    }
    #[test]
    fn degree_drop_or_vanishing_coefficient_declines() {
        for expression in ["s*x^4-x^3+x^2-x+1", "x^4+s*x^3+x^2-x+1"] {
            assert_eq!(probe(expression, "0").report["eligible"], false);
        }
    }
    #[test]
    fn fewer_positive_roots_than_variations_declines() {
        assert_eq!(probe("x^4-x^3+x^2-x+1", "0").report["eligible"], false);
    }
    #[test]
    fn negative_collision_preserves_two_simple_positive_branches() {
        for (sample, real_count) in [("17/8", 4), ("9/4", 3), ("5/2", 2)] {
            let p = probe("(x-1)*(x-2)*(x^2+3*x+s)", sample);
            assert_eq!(p.report["eligible"], true);
            assert_eq!(p.report["variations"], 2);
            assert_eq!(p.report["positive_roots"], 2);
            assert_eq!(p.report["all_real_roots"], real_count);
            assert_eq!(
                p.report["endpoint_separator_infinity_signs"],
                json!([1, -1, 1])
            );
        }
    }
}
