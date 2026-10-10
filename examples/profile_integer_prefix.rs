//! Research-only exact multivariate evaluation with one final rational reduction.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, hint::black_box, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::algebra::{Algebra, Poly, rational};

fn validate(p: &Poly, prefix: &[Rational]) -> Result<()> {
    ensure!(prefix.len() <= p.nvars(), "prefix exceeds variable map");
    ensure!(
        (prefix.len()..p.nvars()).all(|i| p.degree(i) == 0),
        "unassigned variable"
    );
    Ok(())
}

/// Compute N/D with D=product(d_i^degree_i)>0. Every integer term is
/// c_e*product(n_i^e_i*d_i^(degree_i-e_i)), so the final Rational is exact.
/// All integer arithmetic and rational normalization are supplied by Symbolica.
fn scaled(p: &Poly, prefix: &[Rational]) -> Option<Rational> {
    if p.is_constant() {
        return Some(p.get_constant());
    }
    if p.nterms() > 4096 || p.coefficients.iter().any(|c| !c.is_integer()) {
        return None;
    }
    let active = (0..prefix.len())
        .filter_map(|i| {
            let d = p.degree(i) as usize;
            (d > 0).then_some((i, d))
        })
        .collect::<Vec<_>>();
    if active.len() > 16 || active.iter().any(|(_, d)| *d > 64) {
        return None;
    }
    let mut bits = 0u128;
    let mut storage = 0u128;
    for &(i, d) in &active {
        let b = u128::from(prefix[i].numerator_ref().significant_bits())
            + u128::from(prefix[i].denominator_ref().significant_bits())
            + 2;
        bits += b * d as u128;
        storage += 3 * (d + 1) as u128 * (64 + b * d as u128 / 8);
    }
    if bits > 16384 || storage > 1024 * 1024 {
        return None;
    }
    let mut denominator = Integer::one();
    let mut powers = Vec::with_capacity(active.len());
    for &(i, d) in &active {
        let mut n = vec![Integer::one(); d + 1];
        let mut den = vec![Integer::one(); d + 1];
        for e in 1..=d {
            n[e] = Z.mul(&n[e - 1], prefix[i].numerator_ref());
            den[e] = Z.mul(&den[e - 1], prefix[i].denominator_ref());
        }
        Z.mul_assign(&mut denominator, &den[d]);
        powers.push(
            (0..=d)
                .map(|e| Z.mul(&n[e], &den[d - e]))
                .collect::<Vec<_>>(),
        );
    }
    let mut numerator = Integer::zero();
    for term in p {
        let mut value = term.coefficient.numerator_ref().clone();
        for (k, &(i, _)) in active.iter().enumerate() {
            let power = &powers[k][term.exponents[i] as usize];
            if !power.is_one() {
                Z.mul_assign(&mut value, power);
            }
            if value.is_zero() {
                break;
            }
        }
        Z.add_assign(&mut numerator, &value);
    }
    Some(Rational::from((numerator, denominator)))
}
fn evaluate(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
    validate(p, prefix)?;
    Ok(scaled(p, prefix).unwrap_or_else(|| p.replace_all(prefix)))
}

#[derive(Deserialize)]
struct Batch {
    variables: Vec<String>,
    prefix: Vec<String>,
    guards: Vec<String>,
    #[serde(default)]
    select_supported: bool,
}
#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 200)]
    iterations: usize,
    #[arg(long, default_value_t = 5)]
    repeats: usize,
    #[arg(long, default_value_t = 64)]
    max_guards: usize,
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        (1..=10000).contains(&args.iterations) && (1..=9).contains(&args.repeats),
        "timing limits"
    );
    ensure!((1..=256).contains(&args.max_guards), "guard limit");
    let bytes = fs::read(&args.input)?;
    let input: Batch = serde_json::from_slice(&bytes)?;
    let algebra = Algebra::new(&input.variables)?;
    let prefix = input
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let mut guards = input
        .guards
        .iter()
        .map(|s| algebra.parse(s))
        .collect::<Result<Vec<_>>>()?;
    let input_count = guards.len();
    if input.select_supported {
        guards.retain(|p| validate(p, &prefix).is_ok());
    }
    let supported_count = guards.len();
    guards.sort_by_key(|p| std::cmp::Reverse(p.nterms()));
    guards.truncate(args.max_guards);
    ensure!(!guards.is_empty(), "no guards");
    let mut fallback = 0;
    let mut values = Vec::new();
    for p in &guards {
        validate(p, &prefix)?;
        fallback += usize::from(scaled(p, &prefix).is_none());
        let value = evaluate(p, &prefix)?;
        ensure!(
            value == p.replace_all(&prefix),
            "integer prefix value mismatch"
        );
        values.push(value.to_string());
    }
    let mut timings = Vec::new();
    for repeat in 0..args.repeats {
        let mut seconds = [0.0; 2];
        for mode in [repeat % 2, 1 - repeat % 2] {
            let start = Instant::now();
            for _ in 0..args.iterations {
                for p in &guards {
                    let p = black_box(p);
                    let prefix = black_box(prefix.as_slice());
                    if mode == 0 {
                        validate(p, prefix)?;
                        black_box(p.replace_all(prefix));
                    } else {
                        black_box(evaluate(p, prefix)?);
                    }
                }
            }
            seconds[mode] = start.elapsed().as_secs_f64();
        }
        timings.push(json!({"repeat":repeat,"first":if repeat%2==0 {"rational"} else {"integer"},"rational":seconds[0],"integer":seconds[1]}));
    }
    fs::write(
        args.output,
        serde_json::to_vec_pretty(
            &json!({"kind":"research_exact_integer_prefix_microbenchmark_no_completion_claim",
        "input_sha256":format!("{:x}",Sha256::digest(&bytes)),"variables":input.variables,"prefix":input.prefix,
        "input_guards":input_count,"supported_guards":supported_count,"selected_guards":guards.len(),"fallbacks":fallback,
        "selected_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&guards.iter().map(|p|algebra.format(p)).collect::<Vec<_>>())?)),
        "values_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&values)?)),"exact_values_equal":true,
        "terms":guards.iter().map(Poly::nterms).sum::<usize>(),"iterations":args.iterations,"timings":timings}),
        )?,
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_values_for_zero_negative_rationals_inactive_variables_and_cancellation() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into(), "later".into()]).unwrap();
        for text in ["0", "7", "x^64+y^2", "x^2+2*x*y+y^2", "3*x^4*y^3-7*x*y+9"] {
            let p = a.parse(text).unwrap();
            for point in [
                ["0", "3", "-2/3"],
                ["-3/2", "0", "3/2"],
                ["999999999999999999999/17", "9", "-1234567/19"],
            ] {
                let point = point.map(|s| rational(s).unwrap());
                assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
            }
        }
    }
    #[test]
    fn rational_and_degree_caps_fall_back_without_accepting_unassigned_support() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let point = [rational("-2/3").unwrap()];
        for text in ["x/7+1/3", "x^65-2*x"] {
            let p = a.parse(text).unwrap();
            assert!(scaled(&p, &point).is_none());
            assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
        }
        assert!(evaluate(&a.parse("x+y").unwrap(), &point).is_err());
        let huge = Rational::from((Integer::one() << 17000usize, Integer::from(3)));
        let p = a.parse("x^2+1").unwrap();
        assert!(scaled(&p, &[huge.clone()]).is_none());
        assert_eq!(
            evaluate(&p, &[huge.clone()]).unwrap(),
            p.replace_all(&[huge])
        );
    }
    #[test]
    fn deterministic_integer_polynomial_family_matches_rational_evaluation() {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
        for seed in 0..100 {
            let terms = (0..12)
                .map(|k| {
                    format!(
                        "({})*x^{}*y^{}*z^{}",
                        (seed * 17 + k * 13) % 43 - 21,
                        (seed + k) % 5,
                        (seed + 2 * k) % 4,
                        (2 * seed + k) % 3
                    )
                })
                .collect::<Vec<_>>()
                .join("+");
            let p = a.parse(&terms).unwrap();
            let point = [
                Rational::from((seed - 50, 7)),
                Rational::from((-seed, 11)),
                Rational::from((seed + 1, 13)),
            ];
            assert_eq!(evaluate(&p, &point).unwrap(), p.replace_all(&point));
        }
    }
}
