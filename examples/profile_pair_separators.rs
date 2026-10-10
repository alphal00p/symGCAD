//! Research-only fixed rational separators for a pair of delineable positive fibers.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, factors_checked, rational},
    benchmark::run_bounded,
    optimized_projection::OperationCapture,
    roots::{FiberRoot, isolate_union, refine_once, sector_sample},
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

fn positive_roots(mut roots: Vec<FiberRoot>) -> Result<Vec<FiberRoot>> {
    for root in &mut roots {
        let mut steps = 0;
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            ensure!(
                root.interval.0 != root.interval.1,
                "zero root violates strict endpoint prerequisite"
            );
            ensure!(steps < 256, "positive separation refinement cap");
            refine_once(root);
            steps += 1;
        }
    }
    roots.retain(|r| r.interval.0 > Rational::zero());
    Ok(roots)
}

struct Probe {
    report: Value,
    guards: Vec<(String, Poly)>,
}

fn probe(
    algebra: &Algebra,
    polynomials: &[Poly],
    variable: usize,
    prefix: &[Rational],
) -> Result<Probe> {
    ensure!(polynomials.len() == 2, "two operands required");
    ensure!(
        polynomials
            .iter()
            .all(|p| p.degree(variable) <= 32 && p.nterms() <= 4096),
        "research degree/term cap"
    );
    let started = Instant::now();
    let fibers = polynomials
        .iter()
        .map(|p| algebra.specialize_univariate(p, variable, prefix))
        .collect::<Result<Vec<_>>>()?;
    let mut report = json!({"eligible":false,"prerequisite":"Independent individual positive-ray delineability/count proofs are required; this diagnostic does not construct them.",
        "operands":polynomials.iter().map(|p| json!({"terms":p.nterms(),"degrees":(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>(),"degree":p.degree(variable)})).collect::<Vec<_>>()});
    if fibers.iter().zip(polynomials).any(|(f, p)| {
        f.is_zero() || f.degree() != p.degree(variable) as usize || f.coefficients()[0].is_zero()
    }) {
        report["declined"] = json!("nullification, degree drop, or zero endpoint");
        return Ok(Probe {
            report,
            guards: vec![],
        });
    }
    let isolation_started = Instant::now();
    let mut ordinals = BTreeMap::new();
    let mut individual_counts = Vec::new();
    for (owner, fiber) in fibers.iter().enumerate() {
        let gcd = fiber.gcd(&fiber.derivative());
        if gcd.degree() > 0 && !positive_roots(isolate_union(vec![(owner, gcd)])?)?.is_empty() {
            report["declined"] = json!("repeated positive root");
            return Ok(Probe {
                report,
                guards: vec![],
            });
        }
        let roots = positive_roots(isolate_union(vec![(owner, fiber.clone())])?)?;
        individual_counts.push(roots.len());
        for (ordinal, root) in roots.iter().enumerate() {
            ordinals.insert((owner, root.index), ordinal);
        }
    }
    let roots = positive_roots(isolate_union(fibers.iter().cloned().enumerate().collect())?)?;
    report["individual_positive_counts"] = json!(individual_counts);
    report["distinct_positive_union_count"] = json!(roots.len());
    report["isolation_seconds"] = json!(isolation_started.elapsed().as_secs_f64());
    if roots.len() != individual_counts.iter().sum::<usize>() {
        report["declined"] = json!("shared positive root: use ordinary pair fallback/refinement");
        return Ok(Probe {
            report,
            guards: vec![],
        });
    }
    report["branch_order"] = json!(roots.iter().map(|r| json!({"owner":r.polynomial,"all_real_index":r.index,
        "positive_index":ordinals[&(r.polynomial,r.index)],"lower":r.interval.0.to_string(),"upper":r.interval.1.to_string()})).collect::<Vec<_>>());
    let construction_started = Instant::now();
    let mut edges = Vec::new();
    let mut guards = Vec::new();
    for (position, pair) in roots.windows(2).enumerate() {
        if pair[0].polynomial == pair[1].polynomial {
            continue;
        }
        let separator = sector_sample(Some(&pair[0]), Some(&pair[1]));
        ensure!(separator > Rational::zero(), "separator must be positive");
        ensure!(
            separator.numerator_ref().significant_bits()
                + separator.denominator_ref().significant_bits()
                <= 256,
            "separator height cap"
        );
        let mut values = Vec::new();
        for (owner, p) in polynomials.iter().enumerate() {
            let guard = p.replace(variable, &separator);
            let value = guard.replace_all(prefix);
            ensure!(
                !guard.is_zero() && !value.is_zero(),
                "separator evaluation vanishes"
            );
            ensure!(
                value == fibers[owner].evaluate(&separator),
                "evaluation specialization mismatch"
            );
            values.push(value.to_string());
            guards.push((format!("edge_{position}_owner_{owner}"), guard));
        }
        edges.push(json!({"position":position,"left_owner":pair[0].polynomial,"right_owner":pair[1].polynomial,
            "left_positive_index":ordinals[&(pair[0].polynomial,pair[0].index)],"right_positive_index":ordinals[&(pair[1].polynomial,pair[1].index)],
            "separator":separator.to_string(),"evaluation_values":values,
            "evaluation_signs":fibers.iter().map(|f|sign(&f.evaluate(&separator))).collect::<Vec<_>>() }));
    }
    report["edges"] = json!(edges);
    report["construction_seconds"] = json!(construction_started.elapsed().as_secs_f64());
    report["total_discovery_seconds"] = json!(started.elapsed().as_secs_f64());
    report["guards"] = json!(guards.iter().map(|(label,p)|json!({"label":label,"terms":p.nterms(),
        "degrees":(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>(),
        "maximum_coefficient_bits":p.coefficients.iter().map(|c|c.numerator_ref().significant_bits()+c.denominator_ref().significant_bits()).max().unwrap_or(0),
        "expression":algebra.format(p)})).collect::<Vec<_>>());
    report["eligible"] = json!(true);
    Ok(Probe { report, guards })
}

fn write_atomic(path: &std::path::Path, value: &Value) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(temporary, path)?;
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
        "capture checksum differs"
    );
    ensure!(operands.len() == 2, "two-operand capture required");
    let variable = capture.variable.context("missing variable")?;
    let algebra = Algebra::new(&capture.variable_names)?;
    let polynomials = operands
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let prefix = args
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let Probe { mut report, guards } = probe(&algebra, &polynomials, variable, &prefix)?;
    report["capture_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    report["operand_string_sha256"] = json!(
        operands
            .iter()
            .map(|p| format!("{:x}", Sha256::digest(p.as_bytes())))
            .collect::<Vec<_>>()
    );
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
            report["factorization"].as_array_mut().unwrap().push(json!({"label":label,"seconds":seconds,
                "factors":factors.len(),"factor_terms":factors.iter().map(|p|p.nterms()).collect::<Vec<_>>(),
                "factor_expressions":factors.iter().map(|p|algebra.format(p)).collect::<Vec<_>>() }));
            write_atomic(&progress, &report)?;
        }
        report["active_guard"] = Value::Null;
        report["factorization_complete"] = json!(true);
        write_atomic(&progress, &report)?;
    }
    Ok(report)
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "deadline cap120s"
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
        "note":"Research-only pair guard-cost diagnostic, conditional on separately proved individual positive-ray delineability; not a complete GCAD or production certificate."});
    write_atomic(&args.output, &report)?;
    println!("{report}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn case(p: &str, q: &str, sample: &str) -> Probe {
        let a = Algebra::new(&["s".into(), "x".into()]).unwrap();
        probe(
            &a,
            &[a.parse(p).unwrap(), a.parse(q).unwrap()],
            1,
            &[rational(sample).unwrap()],
        )
        .unwrap()
    }
    #[test]
    fn every_cross_owner_edge_is_present() {
        let p = case("(x-1)*(x-3)", "(x-2)*(x-4)", "0");
        assert_eq!(p.report["eligible"], true);
        assert_eq!(p.report["edges"].as_array().unwrap().len(), 3);
        assert_eq!(p.guards.len(), 6);
        assert_eq!(p.report["individual_positive_counts"], json!([2, 2]));
    }
    #[test]
    fn same_owner_edges_need_no_extra_guard() {
        let p = case("(x-1)*(x-2)", "(x-3)*(x-4)", "0");
        assert_eq!(p.report["edges"].as_array().unwrap().len(), 1);
        assert_eq!(p.guards.len(), 2);
    }
    #[test]
    fn shared_positive_roots_force_fallback() {
        let p = case("(x-1)*(x-2)", "(x-2)*(x-3)", "0");
        assert_eq!(p.report["eligible"], false);
        assert_eq!(p.report["distinct_positive_union_count"], 3);
    }
    #[test]
    fn negative_shared_roots_and_collisions_do_not_invalidate_positive_order() {
        for sample in ["17/8", "9/4", "5/2"] {
            let p = case("(x-1)*(x^2+3*x+s)", "(x-2)*(x^2+3*x+s)", sample);
            assert_eq!(p.report["eligible"], true);
            assert_eq!(p.report["individual_positive_counts"], json!([1, 1]));
            assert_eq!(p.report["edges"].as_array().unwrap().len(), 1);
        }
    }
    #[test]
    fn repeated_positive_degree_drop_and_zero_endpoint_decline() {
        for p in ["(x-1)^2", "s*x^2+x-1", "x*(x-1)"] {
            assert_eq!(case(p, "x-3", "0").report["eligible"], false);
        }
    }
    #[test]
    fn rootless_positive_member_needs_no_pair_guard() {
        let p = case("x^2+1", "(x-1)*(x-2)", "0");
        assert_eq!(p.report["eligible"], true);
        assert_eq!(p.report["edges"], json!([]));
    }
}
