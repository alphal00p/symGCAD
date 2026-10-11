//! Research-only replay of the exact production interval rule and bounded multipliers.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    optimized_projection::OperationCapture,
    roots::{isolate_union, refine_once, roots_in_open_interval, sector_sample},
};
#[cfg(not(test))]
use symgcad::{algebra, domain, local_projection, roots, solver};

// The included producer also uses private configuration validation. Compile
// that source unchanged instead of duplicating its resource limits here.
#[cfg(not(test))]
#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;

// Compile the unmodified producer/checker implementation into this diagnostic.
// Its own production tests run in the library; the example tests below exercise
// only the additional multiplier screen. A local enum avoids coupling the
// copied module's interval types to the library's distinct Rust type identity.
#[cfg(not(test))]
#[path = "../src/local_projection/interval_descartes.rs"]
mod interval_descartes;
#[cfg(not(test))]
use symgcad::local_projection::LocalReport;
#[cfg(not(test))]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum PositiveRayCase {
    DescartesIntervals {
        intervals: Vec<interval_descartes::PositiveIntervalWitness>,
    },
}
#[cfg(not(test))]
fn prefix_value(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
    ensure!(
        p.exponents_iter()
            .all(|e| e[prefix.len()..].iter().all(|&x| x == 0)),
        "coefficient depends on an unassigned axis"
    );
    Ok(p.replace_all(prefix))
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    prefix: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 64)]
    max_power: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 3)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

fn variation(coefficients: &[Rational]) -> usize {
    coefficients
        .iter()
        .map(symgcad::solver::sign)
        .filter(|&s| s != 0)
        .fold((0, 0), |(last, count), s| {
            (s, count + usize::from(last != 0 && last != s))
        })
        .1
}

fn screen(f: &UnivariatePolynomial<Q>, max_power: usize) -> Result<Value> {
    screen_scaled(f, max_power, &Rational::one())
}
fn screen_scaled(f: &UnivariatePolynomial<Q>, max_power: usize, scale: &Rational) -> Result<Value> {
    ensure!(!f.is_zero() && max_power <= 64, "screen bounds");
    ensure!(
        *scale > Rational::zero(),
        "multiplier must be positive on the ray"
    );
    let leading = f.lcoeff().abs();
    let bound = f
        .coefficients()
        .iter()
        .take(f.degree())
        .map(|c| c.abs() / &leading)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let positive = roots_in_open_interval(f, &Rational::zero(), &bound);
    let multiplier = &f.one() + &f.monomial(scale.clone(), 1);
    let mut expanded = f.clone();
    let mut rows = Vec::new();
    for power in 0..=max_power {
        ensure!(expanded.degree() == f.degree() + power, "degree identity");
        ensure!(
            expanded.lcoeff() == f.lcoeff() * scale.pow(power as u64),
            "leading identity"
        );
        ensure!(
            expanded.evaluate(&Rational::zero()) == f.evaluate(&Rational::zero()),
            "endpoint identity"
        );
        rows.push(
            json!({"power":power,"variations":variation(expanded.coefficients()),
            "signs":expanded.coefficients().iter().map(symgcad::solver::sign).collect::<Vec<_>>()}),
        );
        expanded = &expanded * &multiplier;
    }
    let first = rows
        .iter()
        .find(|r| r["variations"].as_u64() == Some(positive as u64))
        .map(|r| r["power"].clone());
    Ok(
        json!({"scale":scale.to_string(),"positive_distinct_roots":positive,"first_saturated_power":first,
        "rows":rows,"strict_cauchy_bound":bound.to_string(),
        "note":"Saturation is numeric evidence only; a domain proof additionally needs exact symbolic coefficient and separator guards."}),
    )
}

fn reconstruct_multiplier(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    power: usize,
    scale: &Rational,
) -> Result<Value> {
    ensure!(
        power <= 64 && *scale > Rational::zero(),
        "multiplier reconstruction cap"
    );
    let started = Instant::now();
    let mut coefficients = vec![p.zero(); p.degree(variable) as usize + 1];
    for (coefficient, exponent) in p.to_univariate_polynomial_list(variable) {
        coefficients[exponent as usize] = coefficient;
    }
    for _ in 0..power {
        let mut next = vec![p.zero(); coefficients.len() + 1];
        for (i, coefficient) in coefficients.iter().enumerate() {
            next[i] = &next[i] + coefficient;
            next[i + 1] = &next[i + 1] + &coefficient.clone().mul_coeff(scale.clone());
        }
        ensure!(
            next.iter().map(Poly::nterms).sum::<usize>() <= 65536,
            "symbolic coefficient term cap"
        );
        coefficients = next;
    }
    let axis = algebra.parse(&algebra.names[variable])?;
    let one = algebra.parse("1")?;
    let expected = p * &(&one + &axis.clone().mul_coeff(scale.clone())).pow(power);
    let reconstructed = coefficients
        .iter()
        .enumerate()
        .fold(p.zero(), |sum, (i, c)| &sum + &(c * &axis.pow(i)));
    ensure!(
        expected == reconstructed,
        "symbolic multiplier identity failed"
    );
    let mut guards = Vec::new();
    let mut signs = Vec::new();
    for (degree, coefficient) in coefficients.iter().enumerate() {
        if coefficient.is_zero() {
            signs.push(0);
            continue;
        }
        let value = coefficient.replace_all(prefix);
        ensure!(
            !value.is_zero(),
            "nonzero symbolic coefficient vanishes at prefix"
        );
        signs.push(symgcad::solver::sign(&value));
        guards.push(json!({"degree":degree,"terms":coefficient.nterms(),
            "sign":symgcad::solver::sign(&value),"expression":algebra.format(coefficient)}));
    }
    let fiber = algebra.specialize_univariate(p, variable, prefix)?;
    ensure!(
        !fiber.evaluate(&Rational::zero()).is_zero(),
        "zero endpoint"
    );
    let mut roots = isolate_union(vec![(0, fiber.clone())])?;
    for root in &mut roots {
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            refine_once(root);
        }
    }
    roots.retain(|r| r.interval.0 > Rational::zero());
    let mut evaluations = Vec::new();
    let mut alternating = vec![symgcad::solver::sign(&fiber.evaluate(&Rational::zero()))];
    for pair in roots.windows(2) {
        let separator = sector_sample(Some(&pair[0]), Some(&pair[1]));
        let guard = p.replace(variable, &separator);
        let value = guard.replace_all(prefix);
        ensure!(!value.is_zero(), "separator is a root");
        alternating.push(symgcad::solver::sign(&value));
        evaluations.push(
            json!({"separator":separator.to_string(),"sign":symgcad::solver::sign(&value),
            "terms":guard.nterms(),"expression":algebra.format(&guard)}),
        );
    }
    alternating.push(symgcad::solver::sign(&fiber.lcoeff()));
    ensure!(
        alternating.iter().all(|&s| s != 0)
            && (roots.is_empty() || alternating.windows(2).all(|w| w[0] != w[1])),
        "separator signs do not alternate"
    );
    let nonzero: Vec<_> = signs.iter().copied().filter(|&s| s != 0).collect();
    ensure!(
        nonzero.windows(2).filter(|w| w[0] != w[1]).count() == roots.len(),
        "not saturated"
    );
    Ok(
        json!({"power":power,"scale":scale.to_string(),"identity_verified":true,"coefficient_guards":guards,
        "coefficient_guard_count":guards.len(),"coefficient_guard_terms":coefficients.iter().map(Poly::nterms).sum::<usize>(),
        "coefficient_signs":signs,"separator_guards":evaluations,"alternating_endpoint_separator_leading_signs":alternating,
        "positive_simple_roots":roots.len(),"seconds":started.elapsed().as_secs_f64(),
        "scope":"Local sign-guard proposal. Every coefficient/separator sign still requires propagation and full ancestor-cell verification; no original completion."}),
    )
}

#[cfg(not(test))]
fn replay(algebra: &Algebra, p: &Poly, variable: usize, prefix: &[Rational]) -> Result<Value> {
    let started = Instant::now();
    let mut stock = interval_descartes::WitnessStock::default();
    let attempt = stock.attempt(0, algebra, p, variable, prefix)?;
    let mut report = LocalReport::default();
    stock.report(&mut report);
    let Some((case, guards)) = attempt else {
        return Ok(
            json!({"fresh_stock_accepted":false,"seconds":started.elapsed().as_secs_f64(),
            "budget_fallbacks":report.descartes_interval_budget_fallbacks}),
        );
    };
    let verified = interval_descartes::verify_witness(
        p,
        variable,
        prefix,
        &case,
        config::DEFAULT_DESCARTES_INTERVAL_MAX_TERMS,
        config::DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS,
    )?;
    ensure!(
        guards == verified,
        "independent replay guard inventory differs"
    );
    let PositiveRayCase::DescartesIntervals { intervals } = &case;
    Ok(
        json!({"fresh_stock_accepted":true,"independent_replay":true,
        "positive_roots":interval_descartes::positive_count(intervals),"proof":case,
        "guards":guards.iter().map(|p|json!({"terms":p.nterms(),"expression":algebra.format(p)})).collect::<Vec<_>>(),
        "guard_count":guards.len(),"guard_terms":guards.iter().map(Poly::nterms).sum::<usize>(),
        "seconds":started.elapsed().as_secs_f64(),"templates":report.descartes_interval_templates,
        "budget_fallbacks":report.descartes_interval_budget_fallbacks,
        "history_note":"Fresh stock only. An interrupted worker's prior per-polynomial template stock was not exported."}),
    )
}
#[cfg(test)]
fn replay(_: &Algebra, _: &Poly, _: usize, _: &[Rational]) -> Result<Value> {
    anyhow::bail!("production adapter is only executed by the standalone diagnostic")
}

fn work(args: &Args) -> Result<Value> {
    let bytes = fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture.operands.as_ref().context("omitted operands")?;
    ensure!(operands.len() == 1, "single discriminant operand required");
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture checksum mismatch"
    );
    let variable = capture.variable.context("missing variable")?;
    ensure!(args.prefix.len() == variable, "prefix dimension mismatch");
    let algebra = Algebra::new(&capture.variable_names)?;
    let p = algebra.parse(&operands[0])?;
    ensure!(p.degree(variable) <= 32 && p.nterms() <= 4096, "input caps");
    let prefix = args
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let fiber = algebra.specialize_univariate(&p, variable, &prefix)?;
    let multipliers = screen(&fiber, args.max_power)?;
    let mut scaled_rows = Vec::new();
    let mut candidates = Vec::new();
    for exponent in -6i32..=6 {
        let two = Rational::from(2).pow(exponent.unsigned_abs() as u64);
        let scale = if exponent < 0 {
            Rational::one() / two
        } else {
            two
        };
        let row = screen_scaled(
            &fiber,
            24usize.min(32usize.saturating_sub(fiber.degree())),
            &scale,
        )?;
        if let Some(power) = row["first_saturated_power"].as_u64() {
            let height = scale.numerator_ref().significant_bits()
                + scale.denominator_ref().significant_bits();
            candidates.push((power as usize, height, scale));
        }
        scaled_rows.push(row);
    }
    candidates.sort();
    let reconstructed = if let Some((power, _, scale)) = candidates.first() {
        Some(reconstruct_multiplier(
            &algebra, &p, variable, &prefix, *power, scale,
        )?)
    } else {
        None
    };
    Ok(
        json!({"capture_sha256":format!("{:x}",Sha256::digest(bytes)),
        "input_hash":capture.input_hash,"prefix":args.prefix,"variables":capture.variable_names,
        "fresh_production_interval_rule":replay(&algebra,&p,variable,&prefix)?,
            "multiplier_screen":multipliers,"dyadic_multiplier_screen":scaled_rows,"multiplier_reconstruction":reconstructed,"original_completion":false}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 60.,
        "deadline cap"
    );
    ensure!(args.max_power <= 64, "power cap");
    if args.worker {
        println!("{}", work(&args)?);
        return Ok(());
    }
    ensure!(!args.output.exists(), "output must be fresh");
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
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
        "binary_sha256":format!("{:x}",Sha256::digest(fs::read(executable)?)),"limits":limits,"cpu":args.cpu}))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(text: &str) -> Result<Value> {
        let algebra = Algebra::new(&["x".into()])?;
        screen(
            &algebra.specialize_univariate(&algebra.parse(text)?, 0, &[])?,
            8,
        )
    }
    #[test]
    fn negative_multiplier_root_never_changes_positive_count() -> Result<()> {
        let report = run("(x-1)*(x-2)*(x^2-x+1)")?;
        assert_eq!(report["positive_distinct_roots"], 2);
        assert!(
            report["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["variations"].as_u64().unwrap() >= 2)
        );
        Ok(())
    }
    #[test]
    fn bounded_multiplier_can_certify_rootless_mixed_coefficients() -> Result<()> {
        let report = run("x^2-x+1")?;
        assert_eq!(report["positive_distinct_roots"], 0);
        assert_eq!(report["first_saturated_power"], 1);
        let algebra = Algebra::new(&["x".into()])?;
        let proof = reconstruct_multiplier(
            &algebra,
            &algebra.parse("x^2-x+1")?,
            0,
            &[],
            1,
            &Rational::one(),
        )?;
        assert_eq!(proof["identity_verified"], true);
        assert_eq!(proof["coefficient_guard_count"], 2);
        Ok(())
    }
}
