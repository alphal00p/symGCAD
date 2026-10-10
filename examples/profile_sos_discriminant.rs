//! Bounded research diagnostic; no discriminant is omitted by production code.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    optimized_projection::OperationCapture,
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    prefix: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 64)]
    max_order: usize,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    #[arg(long, default_value_t = 2_000_000)]
    max_symbolic_products: usize,
    /// Solve all positive rational derivative weights exactly at each scale/order.
    #[arg(long)]
    continuous_weights: bool,
}

fn scaled(p: &UnivariatePolynomial<Q>, lambda: &Rational) -> UnivariatePolynomial<Q> {
    let mut power = Rational::one();
    let coefficients = p
        .coefficients()
        .iter()
        .map(|c| {
            let value = c * &power;
            power = &power * lambda;
            value
        })
        .collect();
    UnivariatePolynomial::from_coefficients(&Q, coefficients, p.get_vars())
}

fn sos(
    p: &UnivariatePolynomial<Q>,
    lambda: &Rational,
    weight: &Rational,
) -> UnivariatePolynomial<Q> {
    sos_weight(p, lambda, &(weight * weight))
}

fn sos_weight(
    p: &UnivariatePolynomial<Q>,
    lambda: &Rational,
    weight: &Rational,
) -> UnivariatePolynomial<Q> {
    let q = scaled(p, lambda);
    // Differentiate in the original z coordinate, then substitute z=lambda*u.
    let derivative = scaled(&p.derivative(), lambda);
    &q * &q + (&derivative * &derivative).mul_coeff(weight)
}

/// Intersect a_i + w*b_i >= 0 for every i with w > 0.
/// A singleton positive endpoint is allowed; the later symbolic test still
/// rejects sample-zero coefficients that are not identically zero polynomials.
fn feasible_weight(
    a: &[Rational],
    b: &[Rational],
) -> Option<(Rational, Rational, Option<Rational>)> {
    let zero = Rational::zero();
    let mut lower = zero.clone();
    let mut upper: Option<Rational> = None;
    for i in 0..a.len().max(b.len()) {
        let ai = a.get(i).unwrap_or(&zero);
        let bi = b.get(i).unwrap_or(&zero);
        if bi.is_zero() {
            if ai.is_negative() {
                return None;
            }
        } else {
            let boundary = -ai.clone() / bi;
            if bi.is_negative() {
                if upper.as_ref().is_none_or(|old| &boundary < old) {
                    upper = Some(boundary);
                }
            } else if boundary > lower {
                lower = boundary;
            }
        }
    }
    let chosen = if let Some(upper) = &upper {
        if upper <= &zero || &lower > upper {
            return None;
        }
        (&lower + upper) / Rational::from(2)
    } else {
        &lower + &Rational::one()
    };
    Some((chosen, lower, upper))
}

fn screen_weights(p: &UnivariatePolynomial<Q>, max_order: usize, seconds: f64) -> Result<Value> {
    let start = Instant::now();
    let grid = ["1", "1/4", "4", "1/16", "16", "1/64", "64", "1/256", "256"];
    let multiplier = p.one() + p.monomial(Rational::one(), 1);
    let mut rows = Vec::new();
    let mut best: Option<Value> = None;
    let mut finished = true;
    'portfolio: for lambda_text in grid {
        let lambda = rational(lambda_text)?;
        let q = scaled(p, &lambda);
        let derivative = scaled(&p.derivative(), &lambda);
        let mut a = &q * &q;
        let mut b = &derivative * &derivative;
        for order in 0..=max_order {
            if start.elapsed().as_secs_f64() >= seconds {
                finished = false;
                break 'portfolio;
            }
            let interval = feasible_weight(a.coefficients(), b.coefficients());
            let interval = interval.map(|(weight, lower, upper)| {
                let candidate = &a + &b.clone().mul_coeff(&weight);
                assert!(positive_coefficients(&candidate));
                let candidate = json!({"order":order,"lambda":lambda.to_string(),"weight":weight.to_string(),
                    "lower_inclusive":lower.to_string(),"upper_inclusive":upper.map(|v|v.to_string()),"weight_strictly_positive":true});
                if best.as_ref().is_none_or(|old| order < old["order"].as_u64().unwrap() as usize) {
                    best = Some(candidate.clone());
                }
                candidate
            });
            rows.push(
                json!({"lambda":lambda.to_string(),"order":order,"feasible_interval":interval}),
            );
            if order < max_order {
                a = &a * &multiplier;
                b = &b * &multiplier;
            }
        }
    }
    Ok(
        json!({"method":"exact_positive_weight_interval","complete_portfolio":finished,
        "trials":rows,"elapsed_seconds":start.elapsed().as_secs_f64(),"best":best}),
    )
}

fn positive_coefficients(p: &UnivariatePolynomial<Q>) -> bool {
    !p.is_zero() && p.coefficients().iter().all(|c| c >= &Rational::zero())
}

fn screen(p: &UnivariatePolynomial<Q>, max_order: usize, seconds: f64) -> Result<Value> {
    let start = Instant::now();
    let grid = ["1", "1/4", "4", "1/16", "16", "1/64", "64", "1/256", "256"];
    let multiplier = p.one() + p.monomial(Rational::one(), 1);
    let mut rows = Vec::new();
    let mut best: Option<(usize, String, String)> = None;
    let mut finished = true;
    'portfolio: for lambda_text in grid {
        let lambda = rational(lambda_text)?;
        for relative in grid {
            if start.elapsed().as_secs_f64() >= seconds {
                finished = false;
                break 'portfolio;
            }
            let weight = &lambda * &rational(relative)?;
            let mut polynomial = sos(p, &lambda, &weight);
            let mut success = None;
            let mut best_negative = usize::MAX;
            for order in 0..=max_order {
                best_negative = best_negative.min(
                    polynomial
                        .coefficients()
                        .iter()
                        .filter(|c| c.is_negative())
                        .count(),
                );
                if positive_coefficients(&polynomial) {
                    success = Some(order);
                    if best.as_ref().is_none_or(|(k, _, _)| order < *k) {
                        best = Some((order, lambda.to_string(), weight.to_string()));
                    }
                    break;
                }
                if order < max_order {
                    polynomial = &polynomial * &multiplier;
                }
            }
            rows.push(json!({"lambda":lambda.to_string(),"c":weight.to_string(),"success_order":success,"minimum_negative_coefficients":best_negative}));
        }
    }
    Ok(
        json!({"complete_portfolio":finished,"trials":rows,"elapsed_seconds":start.elapsed().as_secs_f64(),
        "best":best.map(|(k,l,c)|json!({"order":k,"lambda":l,"c":c}))}),
    )
}

fn reconstruct(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    best: &Value,
    budget: usize,
) -> Result<Value> {
    let start = Instant::now();
    let lambda = rational(best["lambda"].as_str().context("missing lambda")?)?;
    let weight = if let Some(weight) = best["weight"].as_str() {
        rational(weight)?
    } else {
        let c = rational(best["c"].as_str().context("missing weight")?)?;
        &c * &c
    };
    ensure!(
        weight > Rational::zero(),
        "weight must be strictly positive"
    );
    let order = best["order"].as_u64().context("missing order")? as usize;
    let axis = algebra.parse(&algebra.names[variable])?;
    let substitution = axis.clone().mul_coeff(lambda.clone());
    let q = p.replace_with_poly(variable, &substitution);
    let derivative = p
        .derivative(variable)
        .replace_with_poly(variable, &substitution);
    let mut work = q
        .nterms()
        .saturating_mul(q.nterms())
        .saturating_add(derivative.nterms().saturating_mul(derivative.nterms()));
    if work > budget {
        return Ok(
            json!({"expanded":false,"reason":"symbolic square product bound","estimated_products":work,"budget":budget}),
        );
    }
    let mut candidate = &q * &q + (&derivative * &derivative).mul_coeff(weight.clone());
    let initial_terms = candidate.nterms();
    let multiplier = &axis + &algebra.parse("1")?;
    for completed in 0..order {
        work = work.saturating_add(candidate.nterms().saturating_mul(2));
        if work > budget {
            return Ok(
                json!({"expanded":false,"reason":"symbolic multiplier product bound","completed_order":completed,"terms":candidate.nterms(),"estimated_products":work,"budget":budget}),
            );
        }
        candidate = &candidate * &multiplier;
    }
    let fiber = algebra.specialize_univariate(p, variable, prefix)?;
    let mut numeric = sos_weight(&fiber, &lambda, &weight);
    let numeric_multiplier = &fiber.one() + &fiber.monomial(Rational::one(), 1);
    for _ in 0..order {
        numeric = &numeric * &numeric_multiplier;
    }
    ensure!(
        algebra.specialize_univariate(&candidate, variable, prefix)? == numeric,
        "symbolic reconstruction differs from numeric identity"
    );
    let coefficients = candidate.to_univariate_polynomial_list(variable);
    let mut guards = Vec::new();
    let mut valid = true;
    for (coefficient, degree) in coefficients {
        ensure!(
            coefficient
                .exponents_iter()
                .all(|e| e[prefix.len()..].iter().all(|&v| v == 0)),
            "coefficient retains an unspecialized variable"
        );
        let value = coefficient.replace_all(prefix);
        valid &= value > Rational::zero();
        guards.push(json!({"degree":degree,"terms":coefficient.nterms(),"sample_value":value.to_string(),"polynomial":algebra.format(&coefficient)}));
    }
    Ok(
        json!({"expanded":true,"exact_specialization_identity":true,"sample_strict_guard_signs":valid,
        "initial_sos_terms":initial_terms,"multiplied_terms":candidate.nterms(),"guard_count":guards.len(),"guards":guards,
        "estimated_products":work,"elapsed_seconds":start.elapsed().as_secs_f64(),
        "note":"These are proposed lower-axis sign obligations, not a global nonvanishing certificate or CAD completion."}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(args.max_order <= 64, "order budget exceeds64");
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "seconds outside(0,120]"
    );
    let start = Instant::now();
    let bytes = fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .context("capture omitted operands")?;
    ensure!(operands.len() == 1, "expected a discriminant operand");
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture checksum mismatch"
    );
    let variable = capture.variable.context("capture omitted axis")?;
    ensure!(
        args.prefix.len() == variable,
        "prefix must assign every earlier axis"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let p = algebra.parse(&operands[0])?;
    let prefix = args
        .prefix
        .iter()
        .map(|v| rational(v))
        .collect::<Result<Vec<_>>>()?;
    let fiber = algebra.specialize_univariate(&p, variable, &prefix)?;
    ensure!(
        fiber.degree() == p.degree(variable) as usize && fiber.degree() <= 32,
        "degree drop or degree budget exceeded"
    );
    ensure!(
        !fiber.evaluate(&Rational::zero()).is_zero(),
        "endpoint must be nonzero"
    );
    let screened = if args.continuous_weights {
        screen_weights(&fiber, args.max_order, args.seconds)?
    } else {
        screen(&fiber, args.max_order, args.seconds)?
    };
    // Preserve the inexpensive screening evidence before any symbolic square.
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut report = json!({"kind":"sum_of_squares_discriminant_research_only","capture":args.capture,
        "capture_sha256":format!("{:x}",Sha256::digest(bytes)),"variables":capture.variable_names,"variable":variable,
        "prefix":args.prefix,"input_terms":p.nterms(),"fiber_degree":fiber.degree(),
        "fiber_coefficients":fiber.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>(),
        "formula":"(1+u)^k * (p(lambda*u)^2+w*p_z'(lambda*u)^2), w>0",
        "retained_obligations":["p(0) != 0","leading coefficient != 0","strict positive signs of every nonzero symbolic multiplied-S coefficient"],
        "screen":screened,"original_completion":false});
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    if !report["screen"]["best"].is_null() {
        report["symbolic"] = reconstruct(
            &algebra,
            &p,
            variable,
            &prefix,
            &report["screen"]["best"],
            args.max_symbolic_products,
        )?;
    }
    report["elapsed_seconds"] = json!(start.elapsed().as_secs_f64());
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"output":args.output,"best":report["screen"]["best"],"complete_portfolio":report["screen"]["complete_portfolio"],"symbolic_expanded":report["symbolic"]["expanded"],"elapsed_seconds":report["elapsed_seconds"],"original_completion":false})
        )?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fiber(expression: &str) -> UnivariatePolynomial<Q> {
        let a = Algebra::new(&["z".into()]).unwrap();
        a.specialize_univariate(&a.parse(expression).unwrap(), 0, &[])
            .unwrap()
    }
    #[test]
    fn positive_repeated_root_cannot_pass() {
        let p = fiber("(z-1)^2");
        let s = sos(&p, &Rational::from(2), &Rational::from(3));
        assert!(s.evaluate(&Rational::from((1, 2))).is_zero());
        assert!(!positive_coefficients(&s));
        assert!(screen(&p, 8, 5.0).unwrap()["best"].is_null());
    }
    #[test]
    fn negative_repeated_roots_are_allowed() {
        let p = fiber("(z+1)^2");
        assert!(positive_coefficients(&sos(
            &p,
            &Rational::one(),
            &Rational::one()
        )));
    }
    #[test]
    fn successful_symbolic_reconstruction_checks_the_scaled_derivative() {
        let a = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let p = a.parse("z^2+a").unwrap();
        let r = reconstruct(
            &a,
            &p,
            1,
            &[Rational::one()],
            &json!({"order":2,"lambda":"3/2","c":"2/3"}),
            10000,
        )
        .unwrap();
        assert_eq!(r["exact_specialization_identity"], true);
        assert_eq!(r["sample_strict_guard_signs"], true);
        assert_eq!(
            reconstruct(
                &a,
                &p,
                1,
                &[Rational::one()],
                &json!({"order":2,"lambda":"3/2","c":"2/3"}),
                1
            )
            .unwrap()["expanded"],
            false
        );
    }
    #[test]
    fn exact_weight_interval_intersects_both_bounds() {
        let (weight, lower, upper) = feasible_weight(
            &[Rational::from(-2), Rational::from(3)],
            &[Rational::one(), Rational::from(-1)],
        )
        .unwrap();
        assert_eq!(lower, Rational::from(2));
        assert_eq!(upper, Some(Rational::from(3)));
        assert_eq!(weight, Rational::from((5, 2)));
        assert!(feasible_weight(&[Rational::from(-1)], &[Rational::zero()]).is_none());
        assert!(
            feasible_weight(
                &[Rational::from(-3), Rational::from(2)],
                &[Rational::one(), Rational::from(-1)],
            )
            .is_none()
        );
    }
    #[test]
    fn singleton_weights_must_be_strictly_positive() {
        assert_eq!(
            feasible_weight(
                &[Rational::from(-1), Rational::one()],
                &[Rational::one(), Rational::from(-1)],
            )
            .unwrap()
            .0,
            Rational::one()
        );
        assert!(feasible_weight(&[Rational::zero()], &[Rational::from(-1)]).is_none());
        assert_eq!(feasible_weight(&[], &[]).unwrap().0, Rational::one());
    }
    #[test]
    fn rational_weight_needs_no_rational_square_root() {
        let a = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let p = a.parse("z^2+a").unwrap();
        let r = reconstruct(
            &a,
            &p,
            1,
            &[Rational::one()],
            &json!({"order":1,"lambda":"3/2","weight":"2/3"}),
            10000,
        )
        .unwrap();
        assert_eq!(r["exact_specialization_identity"], true);
        assert_eq!(r["sample_strict_guard_signs"], true);
        assert!(screen_weights(&fiber("(z-1)^2"), 8, 5.0).unwrap()["best"].is_null());
        assert!(!screen_weights(&fiber("(z+1)^2"), 0, 5.0).unwrap()["best"].is_null());
    }
}
