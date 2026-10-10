//! Replay captured algebra with existing Symbolica algorithms. Run each trial
//! under an external timeout and memory limit; this tool changes no backend.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    algebra::{Algebra, Poly, canonical, rational},
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Strategy {
    Auto,
    IntegerDucos,
    RationalDucos,
    Brown,
    Primitive,
    Crt,
    /// Production exact low-degree discriminant identities.
    Formula,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum FactorStart {
    Auto,
    Univariate,
    Bivariate,
    Disabled,
}

fn checked_native_factors(polynomial: &Poly) -> Result<Vec<Poly>> {
    let factors = polynomial.factor();
    let reconstructed = factors
        .iter()
        .fold(polynomial.one(), |p, (f, multiplicity)| {
            &p * &f.pow(*multiplicity)
        });
    ensure!(reconstructed == *polynomial, "factor reconstruction failed");
    Ok(factors
        .into_iter()
        .map(|(p, _)| p)
        .filter(|p| !p.is_constant())
        .collect())
}

fn collapse_sum(algebra: &Algebra, p: &Poly, names: &[String]) -> Result<(Poly, usize, Poly)> {
    ensure!(
        names.len() == 2 && names[0] != names[1],
        "sum collapse needs two distinct variables"
    );
    let i = algebra
        .names
        .iter()
        .position(|v| v == &names[0])
        .ok_or_else(|| anyhow::anyhow!("unknown first sum variable"))?;
    let j = algebra
        .names
        .iter()
        .position(|v| v == &names[1])
        .ok_or_else(|| anyhow::anyhow!("unknown second sum variable"))?;
    ensure!(
        p.degree(i) > 0 && p.derivative(i) == p.derivative(j),
        "polynomial is not invariant under the proposed sum substitution"
    );
    let first = algebra.parse(&names[0])?;
    let second = algebra.parse(&names[1])?;
    let q = p.replace(i, &Rational::zero());
    let inverse = &second + &first;
    ensure!(
        q.degree(i) == 0 && q.replace_with_poly(j, &inverse) == *p,
        "sum collapse identity failed"
    );
    Ok((q, j, inverse))
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_enum, default_value = "auto")]
    strategy: Strategy,
    /// Existing Symbolica integer-factorization start policy, in this process only.
    #[arg(long, value_enum, default_value = "auto")]
    factor_start: FactorStart,
    /// Exact algebra-only reduction when p depends on two variables through their sum.
    #[arg(long, value_delimiter = ',')]
    collapse_sum: Vec<String>,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Parse and report exact operand sizes without executing the operation.
    #[arg(long)]
    inspect_only: bool,
    /// Exact earlier-coordinate values for inexpensive coefficient-sign inspection.
    #[arg(
        long,
        value_delimiter = ',',
        allow_hyphen_values = true,
        requires = "inspect_only"
    )]
    prefix: Vec<String>,
    /// Certify positive-root counts and inspect (1+x)^k, k=0..16, at the prefix.
    #[arg(long, requires = "inspect_only")]
    positive_root_diagnostics: bool,
}

fn positive_root_diagnostics(
    algebra: &Algebra,
    polynomial: &Poly,
    variable: usize,
    prefix: &[Rational],
    fiber: &UnivariatePolynomial<Q>,
) -> Result<serde_json::Value> {
    let started = Instant::now();
    if fiber.is_zero() {
        return Ok(json!({"zero_fiber":true,"positive_roots_distinct":null,
            "certificate_candidate":false,"reason":"zero fiber has no finite root count"}));
    }
    ensure!(
        fiber.degree() <= 64,
        "positive-root diagnostic degree limit is 64"
    );
    let original_degree = polynomial.degree(variable) as usize;
    let degree_preserved = original_degree == fiber.degree();
    let endpoint = fiber.evaluate(&Rational::zero());
    let leading = fiber.coefficients().last().expect("nonzero fiber");
    let upper = fiber
        .coefficients()
        .iter()
        .take(fiber.degree())
        .map(|c| c.abs() / leading.abs())
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    // This strict Cauchy bound excludes both zero and the upper endpoint.
    ensure!(
        !fiber.evaluate(&upper).is_zero(),
        "Cauchy endpoint unexpectedly vanishes"
    );
    let sturm_count = symgcad::roots::roots_in_open_interval(fiber, &Rational::zero(), &upper);
    let isolated = symgcad::roots::isolate_union(vec![(0, fiber.clone())])?;
    let isolation_count = isolated
        .iter()
        .map(|root| {
            let (lo, hi) = &root.interval;
            if lo == hi {
                usize::from(lo > &Rational::zero())
            } else if lo >= &Rational::zero() {
                1
            } else if hi <= &Rational::zero() {
                0
            } else {
                symgcad::roots::roots_in_open_interval(&root.fiber, &Rational::zero(), hi)
            }
        })
        .sum::<usize>();
    ensure!(
        sturm_count == isolation_count,
        "Sturm and certified isolation disagree"
    );
    let linear = algebra.parse(&format!("1+{}", algebra.names[variable]))?;
    let univariate_linear = fiber.constant(Rational::one()) + fiber.monomial(Rational::one(), 1);
    let mut symbolic = polynomial.clone();
    let mut multiplied = fiber.clone();
    let mut rows = Vec::new();
    for power in 0..=16 {
        ensure!(
            multiplied.degree() == fiber.degree() + power,
            "multiplier changed expected degree"
        );
        ensure!(
            multiplied.evaluate(&Rational::zero()) == endpoint,
            "multiplier changed zero endpoint"
        );
        ensure!(
            multiplied.coefficients().last() == Some(leading),
            "multiplier changed leading coefficient"
        );
        let signs = multiplied
            .coefficients()
            .iter()
            .map(symgcad::solver::sign)
            .collect::<Vec<_>>();
        let nonzero = signs
            .iter()
            .copied()
            .filter(|&s| s != 0)
            .collect::<Vec<_>>();
        let variations = nonzero.windows(2).filter(|w| w[0] != w[1]).count();
        let vanished = symbolic
            .to_univariate_polynomial_list(variable)
            .iter()
            .filter_map(|(c, degree)| c.replace_all(prefix).is_zero().then_some(*degree))
            .collect::<Vec<_>>();
        let candidate = degree_preserved
            && !endpoint.is_zero()
            && vanished.is_empty()
            && variations <= 1
            && variations == sturm_count;
        rows.push(json!({"power":power,"degree":multiplied.degree(),
            "signs_low_to_high":signs,"positive_sign_variations":variations,
            "vanishing_nonzero_symbolic_coefficient_degrees":vanished,
            "degree_and_endpoint_checked":true,"certificate_candidate":candidate}));
        if power < 16 {
            symbolic = &symbolic * &linear;
            multiplied = &multiplied * &univariate_linear;
        }
    }
    Ok(
        json!({"zero_fiber":false,"original_degree":original_degree,"fiber_degree":fiber.degree(),
        "original_degree_preserved":degree_preserved,"zero_endpoint":endpoint.to_string(),
        "zero_endpoint_nonzero":!endpoint.is_zero(),"strict_cauchy_upper_bound":upper.to_string(),
        "cauchy_endpoint_nonzero_checked":true,"positive_roots_distinct_sturm":sturm_count,
        "positive_roots_distinct_isolation":isolation_count,
        "isolation_intervals":isolated.iter().map(|r|json!({"index":r.index,
            "lower":r.interval.0.to_string(),"upper":r.interval.1.to_string()})).collect::<Vec<_>>(),
        "first_candidate_power":rows.iter().find(|r|r["certificate_candidate"] == true).map(|r|r["power"].clone()),
        "multipliers":rows,"seconds":started.elapsed().as_secs_f64(),
        "note":"Exact fiber diagnostic only; no symbolic-domain certificate or original completion is claimed."}),
    )
}

fn resultant(left: &Poly, right: &Poly, variable: usize, strategy: Strategy) -> Poly {
    let left = left.to_univariate(variable);
    let right = right.to_univariate(variable);
    match strategy {
        Strategy::Auto => left.resultant_auto(&right),
        Strategy::IntegerDucos => left.resultant_ducos_integer(&right),
        Strategy::RationalDucos => left.resultant(&right),
        Strategy::Brown => left.resultant_brown(&right),
        Strategy::Primitive => left.resultant_primitive(&right),
        Strategy::Crt => left.resultant_crt(&right),
        Strategy::Formula => unreachable!("formula strategy is discriminant-only"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_profile_checks_scalar_and_repeated_factor_reconstruction() {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = algebra.parse("-3/7*(x+y)^2*(x-2)").unwrap();
        let factors = checked_native_factors(&p).unwrap();
        let mut actual = factors.iter().map(canonical).collect::<Vec<_>>();
        let mut expected = vec![
            canonical(&algebra.parse("x+y").unwrap()),
            canonical(&algebra.parse("x-2").unwrap()),
        ];
        actual.sort_by_key(|p| algebra.format(p));
        expected.sort_by_key(|p| algebra.format(p));
        assert_eq!(actual, expected);
    }

    #[test]
    fn sum_collapse_preserves_factorization_and_rejects_false_invariance() {
        let a = Algebra::new(&["s".into(), "t".into(), "x".into()]).unwrap();
        let p = a.parse("-3/7*(s+t+x)^2*(s+t-x^2)").unwrap();
        let (q, j, inverse) = collapse_sum(&a, &p, &["s".into(), "t".into()]).unwrap();
        assert_eq!(q, a.parse("-3/7*(t+x)^2*(t-x^2)").unwrap());
        let mapped = checked_native_factors(&q)
            .unwrap()
            .into_iter()
            .map(|f| canonical(&f.replace_with_poly(j, &inverse)))
            .collect::<Vec<_>>();
        assert!(mapped.contains(&canonical(&a.parse("s+t+x").unwrap())));
        assert!(mapped.contains(&canonical(&a.parse("s+t-x^2").unwrap())));
        assert!(collapse_sum(&a, &a.parse("s+2*t+x").unwrap(), &["s".into(), "t".into()]).is_err());
    }

    fn inspect(expression: &str, prefix: &[Rational]) -> serde_json::Value {
        let names = if prefix.is_empty() {
            vec!["x".into()]
        } else {
            vec!["a".into(), "x".into()]
        };
        let algebra = Algebra::new(&names).unwrap();
        let p = algebra.parse(expression).unwrap();
        let variable = prefix.len();
        let fiber = algebra.specialize_univariate(&p, variable, prefix).unwrap();
        positive_root_diagnostics(&algebra, &p, variable, prefix, &fiber).unwrap()
    }

    #[test]
    fn positive_root_counts_exclude_zero_and_detect_repeated_positive_roots() {
        let two = inspect("(x-1)*(x-2)", &[]);
        assert_eq!(two["positive_roots_distinct_sturm"], 2);
        assert_eq!(two["positive_roots_distinct_isolation"], 2);
        assert!(two["first_candidate_power"].is_null());
        let repeated = inspect("(x-1)^2", &[]);
        assert_eq!(repeated["positive_roots_distinct_sturm"], 1);
        assert!(repeated["first_candidate_power"].is_null());
        let endpoint = inspect("x*(x-1)", &[]);
        assert_eq!(endpoint["positive_roots_distinct_sturm"], 1);
        assert_eq!(endpoint["zero_endpoint_nonzero"], false);
        assert!(endpoint["first_candidate_power"].is_null());
    }

    #[test]
    fn multiplier_certificate_requires_nonvanishing_symbolic_coefficients() {
        let fixed = inspect("x^2-x+1", &[]);
        assert_eq!(fixed["positive_roots_distinct_sturm"], 0);
        assert_eq!(fixed["first_candidate_power"], 1);
        let parameter = inspect("a*x^2-x+1", &[Rational::one()]);
        assert_eq!(parameter["multipliers"][1]["positive_sign_variations"], 0);
        assert_eq!(parameter["multipliers"][1]["certificate_candidate"], false);
        assert_eq!(parameter["first_candidate_power"], 3);
    }

    #[test]
    fn degree_drop_and_zero_fiber_are_not_certificate_candidates() {
        let dropped = inspect("a*x^2+x+1", &[Rational::zero()]);
        assert_eq!(dropped["original_degree_preserved"], false);
        assert!(dropped["first_candidate_power"].is_null());
        let zero = inspect("a*(x^2+1)", &[Rational::zero()]);
        assert_eq!(zero["zero_fiber"], true);
        assert_eq!(zero["certificate_candidate"], false);
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("capture omitted oversized operands"))?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture input checksum mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let polynomials = operands
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    if args.inspect_only {
        let mut coefficient_signs = Vec::new();
        if !args.prefix.is_empty() || args.positive_root_diagnostics {
            let variable = capture
                .variable
                .ok_or_else(|| anyhow::anyhow!("prefix inspection requires a variable"))?;
            let prefix = args
                .prefix
                .iter()
                .map(|s| rational(s))
                .collect::<Result<Vec<_>>>()?;
            for polynomial in &polynomials {
                let fiber = algebra.specialize_univariate(polynomial, variable, &prefix)?;
                let signs: Vec<_> = fiber
                    .coefficients()
                    .iter()
                    .map(symgcad::solver::sign)
                    .collect();
                let nonzero: Vec<_> = signs.iter().copied().filter(|&s| s != 0).collect();
                let vanished_coefficient = polynomial
                    .to_univariate_polynomial_list(variable)
                    .iter()
                    .any(|(coefficient, _)| coefficient.replace_all(&prefix).is_zero());
                let diagnostics = if args.positive_root_diagnostics {
                    Some(positive_root_diagnostics(
                        &algebra, polynomial, variable, &prefix, &fiber,
                    )?)
                } else {
                    None
                };
                coefficient_signs.push(json!({"signs_low_to_high":signs,"positive_sign_variations":nonzero.windows(2).filter(|w|w[0]!=w[1]).count(),"nonzero_symbolic_coefficient_vanishes":vanished_coefficient,"positive_root_diagnostics":diagnostics}));
            }
        }
        let report = json!({
            "capture":args.capture,"capture_sha256":format!("{:x}",Sha256::digest(&bytes)),
            "operation":capture.operation,"variable":capture.variable,"variables":algebra.names,
            "input_hash":capture.input_hash,"input_terms":polynomials.iter().map(Poly::nterms).collect::<Vec<_>>(),
            "input_degrees":polynomials.iter().map(|p|(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "coefficient_bits":polynomials.iter().map(|p|p.coefficients.iter().map(|c|c.numerator_ref().significant_bits().max(c.denominator_ref().significant_bits())).max().unwrap_or(0)).collect::<Vec<_>>(),
            "prefix":args.prefix,"coefficient_signs":coefficient_signs,
        });
        let report = serde_json::to_string_pretty(&report)? + "\n";
        if let Some(path) = args.output {
            std::fs::write(path, &report)?;
        }
        print!("{report}");
        return Ok(());
    }
    ensure!(
        !matches!(args.strategy, Strategy::Formula)
            || capture.operation == AlgebraOperation::Discriminant,
        "formula strategy requires a discriminant capture"
    );
    let start = Instant::now();
    ensure!(
        args.factor_start == FactorStart::Auto || capture.operation == AlgebraOperation::Factor,
        "factor-start policy requires a factor capture"
    );
    ensure!(
        args.collapse_sum.is_empty() || capture.operation == AlgebraOperation::Factor,
        "sum collapse requires a factor capture"
    );
    let output = match capture.operation {
        AlgebraOperation::Factor => {
            ensure!(
                polynomials.len() == 1,
                "factor capture must have one operand"
            );
            ensure!(
                matches!(args.strategy, Strategy::Auto),
                "use --factor-start to select a factorization policy"
            );
            let (univariate, bivariate) = match args.factor_start {
                FactorStart::Auto => (true, true),
                FactorStart::Univariate => (true, false),
                FactorStart::Bivariate => (false, true),
                FactorStart::Disabled => (false, false),
            };
            symbolica::GLOBAL_SETTINGS
                .use_univariate_factorization
                .store(univariate, std::sync::atomic::Ordering::Relaxed);
            symbolica::GLOBAL_SETTINGS
                .use_bivariate_factorization
                .store(bivariate, std::sync::atomic::Ordering::Relaxed);
            if args.collapse_sum.is_empty() {
                checked_native_factors(&polynomials[0])?
            } else {
                let (q, j, inverse) = collapse_sum(&algebra, &polynomials[0], &args.collapse_sum)?;
                checked_native_factors(&q)?
                    .into_iter()
                    .map(|f| f.replace_with_poly(j, &inverse))
                    .collect()
            }
        }
        AlgebraOperation::Resultant => {
            ensure!(
                polynomials.len() == 2,
                "resultant capture must have two operands"
            );
            let variable = capture
                .variable
                .ok_or_else(|| anyhow::anyhow!("missing elimination variable"))?;
            ensure!(
                variable < algebra.names.len(),
                "elimination variable out of range"
            );
            vec![resultant(
                &polynomials[0],
                &polynomials[1],
                variable,
                args.strategy,
            )]
        }
        AlgebraOperation::Discriminant => {
            ensure!(
                polynomials.len() == 1,
                "discriminant capture must have one operand"
            );
            let variable = capture
                .variable
                .ok_or_else(|| anyhow::anyhow!("missing elimination variable"))?;
            ensure!(
                variable < algebra.names.len(),
                "elimination variable out of range"
            );
            let p = &polynomials[0];
            let degree = p.degree(variable);
            if matches!(args.strategy, Strategy::Formula) {
                vec![symgcad::projection::discriminant(p, variable)?]
            } else if degree < 2 {
                vec![if degree == 0 { p.zero() } else { p.one() }]
            } else {
                let r = resultant(p, &p.derivative(variable), variable, args.strategy);
                let (mut disc, remainder) = r.quot_rem(&p.univariate_lcoeff(variable), false);
                ensure!(remainder.is_zero(), "discriminant division was not exact");
                if degree % 4 == 2 || degree % 4 == 3 {
                    disc = -disc;
                }
                vec![disc]
            }
        }
    };
    let algebra_seconds = start.elapsed().as_secs_f64();
    let mut canonical_output = output
        .iter()
        .map(|p| algebra.format(&canonical(p)))
        .collect::<Vec<_>>();
    canonical_output.sort();
    canonical_output.dedup();
    let report = json!({
        "schema_version":1,"capture":args.capture,"capture_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "operation":capture.operation,"strategy":format!("{:?}",args.strategy),
        "factor_start":format!("{:?}",args.factor_start),
        "factorization_enabled":args.factor_start != FactorStart::Disabled,
        "collapse_sum":args.collapse_sum,
        "capture_backend":capture.backend_version,"backend":LicenseManager::get_version(),
        "variable":capture.variable,"variables":algebra.names,"input_hash":capture.input_hash,
        "input_terms":polynomials.iter().map(Poly::nterms).collect::<Vec<_>>(),
        "input_degrees":polynomials.iter().map(|p|(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "algebra_seconds":algebra_seconds,"output_terms":output.iter().map(Poly::nterms).sum::<usize>(),
        "canonical_output_hash":format!("{:x}",Sha256::digest(serde_json::to_vec(&canonical_output)?)),
        "maximum_coefficient_bits":output.iter().flat_map(|p|&p.coefficients).map(|c|c.numerator_ref().significant_bits().max(c.denominator_ref().significant_bits())).max().unwrap_or(0)
    });
    if let Some(path) = args.output {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = BufWriter::new(File::create(path)?);
        serde_json::to_writer_pretty(&mut out, &report)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }
    let mut out = BufWriter::new(std::io::stdout().lock());
    serde_json::to_writer_pretty(&mut out, &report)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
