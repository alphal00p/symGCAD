//! Exact real-root diagnostics for an interrupted projection operand.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, rational},
    optimized_projection::OperationCapture,
    roots::roots_in_open_interval,
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    prefix: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    /// Diagnose the one-step pseudo-remainder of two captured operands.
    #[arg(long)]
    pair_remainder: bool,
}
fn variations(values: impl IntoIterator<Item = Rational>) -> usize {
    let mut previous = 0;
    let mut count = 0;
    for value in values {
        if value.is_zero() {
            continue;
        }
        let sign = if value.is_negative() { -1 } else { 1 };
        if previous != 0 && previous != sign {
            count += 1;
        }
        previous = sign;
    }
    count
}
fn numeric_report(fiber: &UnivariatePolynomial<Q>) -> serde_json::Value {
    let leading = fiber.lcoeff().abs();
    let radius = fiber
        .coefficients()
        .iter()
        .take(fiber.degree())
        .map(|c| c.abs() / &leading)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let mut expanded = fiber.clone();
    let multiplier = &fiber.monomial(Rational::one(), 1) + &fiber.one();
    let mut polya = Vec::new();
    for order in 0..=8 {
        polya.push(json!({"order":order,"signs_low_to_high":expanded.coefficients().iter().map(|c| if c.is_zero() {0} else if c.is_negative() {-1} else {1}).collect::<Vec<_>>(),
            "variations":variations(expanded.coefficients().iter().cloned())}));
        expanded = &expanded * &multiplier;
    }
    json!({"degree":fiber.degree(),"coefficients_low_to_high":fiber.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>(),
        "positive_distinct_roots":roots_in_open_interval(fiber,&Rational::zero(),&radius),
        "negative_distinct_roots":roots_in_open_interval(fiber,&(-radius),&Rational::zero()),
        "positive_sign_variations":variations(fiber.coefficients().iter().cloned()),"polya":polya})
}
fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing operands"))?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture checksum mismatch"
    );
    let variable = capture
        .variable
        .ok_or_else(|| anyhow::anyhow!("missing variable"))?;
    let algebra = Algebra::new(&capture.variable_names)?;
    let prefix = args
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let mut polynomials = operands
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    // Continue only after specializing: this diagnostic never expands a
    // symbolic second or third pseudo-remainder and never computes a resultant.
    let mut numeric_prs = Vec::new();
    if args.pair_remainder {
        ensure!(polynomials.len() == 2, "pair remainder needs two operands");
        let mut high = algebra.specialize_univariate(&polynomials[0], variable, &prefix)?;
        let mut low = algebra.specialize_univariate(&polynomials[1], variable, &prefix)?;
        if high.degree() < low.degree() {
            std::mem::swap(&mut high, &mut low);
        }
        ensure!(high.degree() <= 16, "bounded numeric PRS needs degree <=16");
        let mut cancellations = 0;
        'prs: for round in 1..=8 {
            let divisor = low.clone();
            let mut remainder = high.clone();
            while remainder.degree() >= divisor.degree() {
                cancellations += 1;
                if cancellations > 16 {
                    break 'prs;
                }
                let shift = remainder.degree() - divisor.degree();
                remainder = remainder.clone().mul_coeff(&divisor.lcoeff())
                    - divisor.mul_exp(shift).mul_coeff(&remainder.lcoeff());
                if remainder.is_zero() || remainder.is_constant() {
                    break 'prs;
                }
                numeric_prs.push(json!({"round":round,"cancellation":cancellations,
                    "completed_remainder":remainder.degree()<divisor.degree(),"fiber":numeric_report(&remainder)}));
            }
            high = low;
            low = remainder;
        }
    }
    let pair = if args.pair_remainder {
        ensure!(polynomials.len() == 2, "pair remainder needs two operands");
        if polynomials[0].degree(variable) < polynomials[1].degree(variable) {
            polynomials.swap(0, 1);
        }
        let p = &polynomials[0];
        let q = &polynomials[1];
        let a = p.univariate_lcoeff(variable);
        let b = q.univariate_lcoeff(variable);
        let estimate = p
            .nterms()
            .saturating_mul(b.nterms())
            .saturating_add(q.nterms().saturating_mul(a.nterms()));
        ensure!(
            estimate <= 100_000,
            "profiling product exceeds bounded arithmetic cap"
        );
        let mut exponents = vec![0; p.nvars()];
        exponents[variable] = p.degree(variable) - q.degree(variable);
        let remainder = &b * p - &a * &q.clone().mul_monomial(&Rational::one(), &exponents);
        let report = json!({"input_terms":[p.nterms(),q.nterms()],"input_degrees":[p.degree(variable),q.degree(variable)],
            "monomial_product_estimate":estimate,"remainder_terms":remainder.nterms(),"remainder_degree":remainder.degree(variable)});
        polynomials = vec![remainder];
        Some(report)
    } else {
        None
    };
    let mut reports = Vec::new();
    for p in polynomials {
        let fiber = algebra.specialize_univariate(&p, variable, &prefix)?;
        ensure!(!fiber.is_zero(), "identically zero fiber");
        let leading = fiber.coefficients().last().unwrap().abs();
        let radius = fiber
            .coefficients()
            .iter()
            .take(fiber.degree())
            .map(|c| c.abs() / &leading)
            .max()
            .unwrap_or_else(Rational::zero)
            + Rational::from(2);
        let positive = roots_in_open_interval(&fiber, &Rational::zero(), &radius);
        let negative = roots_in_open_interval(&fiber, &(-radius), &Rational::zero());
        let zero = fiber.evaluate(&Rational::zero()).is_zero();
        let pos_variations = variations(fiber.coefficients().iter().cloned());
        let neg_variations = variations(
            fiber
                .coefficients()
                .iter()
                .enumerate()
                .map(|(i, c)| if i % 2 == 0 { c.clone() } else { -c.clone() }),
        );
        let mut polya = Vec::new();
        if args.pair_remainder {
            let multiplier = algebra.specialize_univariate(
                &algebra.parse(&format!("{}+1", algebra.names[variable]))?,
                variable,
                &prefix,
            )?;
            let mut expanded = fiber.clone();
            for order in 0..=8 {
                polya.push(json!({"order":order,"signs_low_to_high":expanded.coefficients().iter().map(|c| if c.is_zero() {0} else if c.is_negative() {-1} else {1}).collect::<Vec<_>>(),
                    "variations":variations(expanded.coefficients().iter().cloned())}));
                expanded = &expanded * &multiplier;
            }
        }
        reports.push(json!({"input_terms":p.nterms(),"degree":fiber.degree(),"coefficients_low_to_high":fiber.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>(),
            "positive_sign_variations":pos_variations,"negative_sign_variations":neg_variations,"positive_distinct_roots":positive,"negative_distinct_roots":negative,"zero_is_root":zero,"polya":polya}));
    }
    let report = json!({"capture":args.capture,"capture_sha256":format!("{:x}",Sha256::digest(bytes)),"variables":capture.variable_names,"variable":variable,"prefix":args.prefix,"pair_remainder":pair,"fibers":reports,"numeric_prs":numeric_prs});
    let text = serde_json::to_string_pretty(&report)? + "\n";
    std::fs::write(args.output, &text)?;
    print!("{text}");
    Ok(())
}
