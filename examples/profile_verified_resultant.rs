//! Research-only candidate generation followed by a deterministic identity proof.
//! Every successful verification uses more exact parameter samples than the
//! resultant degree bound. A partial check is explicitly not a certificate.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{domains::InternalOrdering, poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, factors_checked, rational},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
    projection::exact_resultant_ducos,
};

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum Strategy {
    Crt,
    Brown,
    Primitive,
    IntegerDucos,
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_enum, default_value = "crt")]
    strategy: Strategy,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    /// Reuse an untrusted candidate from the same captured input.
    #[arg(long)]
    candidate: Option<PathBuf>,
    /// Diagnostic only: stopping before D+1 successes never proves equality.
    #[arg(long)]
    check_limit: Option<usize>,
    /// Profile exact numeric images without generating a symbolic candidate.
    #[arg(long)]
    numeric_points: Option<usize>,
    /// Construct the resultant by D+1 exact images and Symbolica interpolation.
    #[arg(long)]
    interpolate: bool,
    /// Atomic exact numeric images, reusable across bounded invocations.
    #[arg(long)]
    images_dir: Option<PathBuf>,
    /// Factor a saved interpolation JSON, checking exact factor reconstruction.
    #[arg(long)]
    factor_interpolated: bool,
    /// Research ablation: remove inactive axes, then restore and reconstruct
    /// every exact factor in the original map before accepting the result.
    #[arg(long, requires = "factor_interpolated")]
    compact_factor_map: bool,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Serialize, Deserialize)]
struct Candidate {
    input_hash: String,
    variables: Vec<String>,
    eliminated_variable: usize,
    parameter: usize,
    strategy: String,
    generation_seconds: f64,
    expression: String,
    expression_sha256: String,
}

#[derive(Debug, Serialize)]
struct Verification {
    degree_bound: usize,
    required_points: usize,
    checked_points: usize,
    skipped_degree_drop_points: usize,
    verified: bool,
    seconds: f64,
    maximum_numeric_resultant_bits: u64,
}

#[derive(Serialize, Deserialize)]
struct NumericImage {
    schema_version: u32,
    backend: String,
    input_hash: String,
    variables: Vec<String>,
    variable: usize,
    parameter: usize,
    degrees: [u32; 2],
    point: i64,
    value: String,
    value_sha256: String,
    seconds: f64,
}

fn write_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn bound(p: &Poly, q: &Poly, variable: usize, parameter: usize) -> Result<usize> {
    ensure!(p.variables() == q.variables(), "different variable maps");
    ensure!(
        variable != parameter && variable < p.nvars() && parameter < p.nvars(),
        "invalid axes"
    );
    ensure!(
        (0..p.nvars())
            .all(|v| v == variable || v == parameter || (p.degree(v) == 0 && q.degree(v) == 0)),
        "bivariate inputs required"
    );
    ensure!(
        p.degree(variable) > 0 && q.degree(variable) > 0,
        "positive degrees required"
    );
    let rectangular = (p.degree(variable) as usize)
        .checked_mul(q.degree(parameter) as usize)
        .and_then(|d| {
            d.checked_add((q.degree(variable) as usize).checked_mul(p.degree(parameter) as usize)?)
        })
        .ok_or_else(|| anyhow::anyhow!("degree bound overflow"))?;
    // A Sylvester determinant monomial selects n coefficients of p and m
    // coefficients of q; their t exponents sum to m*n. If total degrees are
    // P,Q, its s degree is therefore at most n*P+m*Q-m*n. This is an upper
    // bound even when determinant terms cancel. Take the smaller valid bound.
    let total = |f: &Poly| {
        f.exponents_iter()
            .map(|e| e.iter().map(|&d| d as usize).sum::<usize>())
            .max()
            .unwrap_or(0)
    };
    let m = p.degree(variable) as usize;
    let n = q.degree(variable) as usize;
    let homogeneous = n * total(p) + m * total(q) - m * n;
    let degree = rectangular.min(homogeneous);
    ensure!(degree <= 10_000, "diagnostic degree bound exceeds 10000");
    Ok(degree)
}

fn verify_candidate(
    p: &Poly,
    q: &Poly,
    candidate: &Poly,
    variable: usize,
    parameter: usize,
    limit: Option<usize>,
    mut progress: impl FnMut(&Verification) -> Result<()>,
) -> Result<Verification> {
    let degree_bound = bound(p, q, variable, parameter)?;
    ensure!(
        candidate.variables() == p.variables(),
        "candidate variable map differs"
    );
    ensure!(
        (0..candidate.nvars()).all(|v| v == parameter || candidate.degree(v) == 0),
        "candidate has unsupported variables"
    );
    ensure!(
        candidate.degree(parameter) as usize <= degree_bound,
        "candidate degree exceeds the proved bound"
    );
    let started = Instant::now();
    let mut report = Verification {
        degree_bound,
        required_points: degree_bound + 1,
        checked_points: 0,
        skipped_degree_drop_points: 0,
        verified: false,
        seconds: 0.0,
        maximum_numeric_resultant_bits: 0,
    };
    let target = limit
        .unwrap_or(report.required_points)
        .min(report.required_points);
    let max_skips = p.degree(parameter) as usize + q.degree(parameter) as usize;
    for attempt in 0..target + max_skips + 1 {
        if report.checked_points == target {
            break;
        }
        // Distinct exact integers: 0,1,-1,2,-2,... . No generic/random premise.
        let k = if attempt % 2 == 1 {
            (attempt as i64 + 1) / 2
        } else {
            -(attempt as i64 / 2)
        };
        let value = Rational::from(k);
        let left = p.replace(parameter, &value);
        let right = q.replace(parameter, &value);
        if left.degree(variable) != p.degree(variable)
            || right.degree(variable) != q.degree(variable)
        {
            report.skipped_degree_drop_points += 1;
            continue;
        }
        let expected = exact_resultant_ducos(&left, &right, variable)?;
        ensure!(
            expected.is_constant(),
            "specialized resultant is not constant"
        );
        let actual = candidate.replace(parameter, &value);
        ensure!(actual == expected, "candidate mismatch at parameter {k}");
        let exact_value = expected.get_constant();
        report.maximum_numeric_resultant_bits = report.maximum_numeric_resultant_bits.max(
            exact_value
                .numerator_ref()
                .significant_bits()
                .max(exact_value.denominator_ref().significant_bits()),
        );
        report.checked_points += 1;
        report.seconds = started.elapsed().as_secs_f64();
        if report.checked_points == 1 || report.checked_points % 16 == 0 {
            progress(&report)?;
        }
    }
    ensure!(
        report.checked_points == target,
        "too many leading-coefficient zeros"
    );
    report.verified = report.checked_points == report.required_points;
    report.seconds = started.elapsed().as_secs_f64();
    progress(&report)?;
    Ok(report)
}

fn factor_in_compact_map(algebra: &Algebra, polynomial: &Poly) -> Result<Vec<Poly>> {
    let active: Vec<_> = algebra
        .names
        .iter()
        .enumerate()
        .filter(|(i, _)| polynomial.degree(*i) > 0)
        .map(|(_, name)| name.clone())
        .collect();
    let compact = Algebra::new(&active)?;
    let input = compact.parse(&algebra.format(polynomial))?;
    ensure!(
        algebra.parse(&compact.format(&input))? == *polynomial,
        "compact variable-map roundtrip changed the polynomial"
    );
    let mut reconstructed = polynomial.one();
    let mut factors = Vec::new();
    for (factor, multiplicity) in input.factor() {
        let restored = algebra.parse(&compact.format(&factor))?;
        ensure!(
            restored.variables() == polynomial.variables(),
            "restored factor has a different original variable map"
        );
        reconstructed = &reconstructed * &restored.pow(multiplicity);
        if !restored.is_constant() {
            factors.push(canonical(&restored));
        }
    }
    ensure!(
        reconstructed == *polynomial,
        "restored factor reconstruction failed"
    );
    factors.sort_by(|a, b| a.internal_cmp(b));
    factors.dedup();
    Ok(factors)
}

fn worker(args: &Args) -> Result<serde_json::Value> {
    let bytes = std::fs::read(&args.capture)?;
    if args.factor_interpolated {
        let saved: serde_json::Value = serde_json::from_slice(&bytes)?;
        let names: Vec<String> = serde_json::from_value(saved["identity"]["variables"].clone())?;
        let expression = saved["expression"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing expression"))?;
        ensure!(
            saved["expression_sha256"].as_str()
                == Some(&format!("{:x}", Sha256::digest(expression.as_bytes()))),
            "interpolated expression checksum differs"
        );
        let algebra = Algebra::new(&names)?;
        let p = canonical(&algebra.parse(expression)?);
        let start = Instant::now();
        let factors = if args.compact_factor_map {
            factor_in_compact_map(&algebra, &p)?
        } else {
            factors_checked(std::slice::from_ref(&p))?
        };
        let seconds = start.elapsed().as_secs_f64();
        let texts: Vec<_> = factors.iter().map(|f| algebra.format(f)).collect();
        let factors_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&texts)?));
        let output = args.output.with_extension("factors.json");
        write_atomic(
            &output,
            &json!({"variables":names,"source_expression_sha256":saved["expression_sha256"],"factors":texts,"factors_sha256":factors_sha256}),
        )?;
        return Ok(
            json!({"operation":"factor_checked","seconds":seconds,"input_terms":p.nterms(),
            "compact_factor_map":args.compact_factor_map,"original_variable_count":p.nvars(),
            "active_variable_count":(0..p.nvars()).filter(|&v|p.degree(v)>0).count(),
            "factor_count":factors.len(),"factor_degrees":factors.iter().map(|p|(0..p.nvars()).map(|v|p.degree(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "factors_sha256":factors_sha256,"factor_artifact":output}),
        );
    }
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(
        capture.operation == AlgebraOperation::Resultant,
        "resultant capture required"
    );
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing operands"))?;
    ensure!(operands.len() == 2, "two operands required");
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture checksum mismatch"
    );
    let variable = capture
        .variable
        .ok_or_else(|| anyhow::anyhow!("missing axis"))?;
    let algebra = Algebra::new(&capture.variable_names)?;
    ensure!(
        variable < algebra.names.len(),
        "invalid eliminated variable"
    );
    let p = algebra.parse(&operands[0])?;
    let q = algebra.parse(&operands[1])?;
    let parameters: Vec<_> = (0..algebra.names.len())
        .filter(|&v| v != variable && (p.degree(v) > 0 || q.degree(v) > 0))
        .collect();
    ensure!(
        parameters.len() == 1,
        "exactly one remaining parameter required"
    );
    let parameter = parameters[0];
    let degree_bound = bound(&p, &q, variable, parameter)?;
    let progress_path = args.output.with_extension("progress.json");
    let identity = json!({"input_hash":capture.input_hash,"variables":algebra.names,
        "variable":variable,"parameter":parameter,"degree_bound":degree_bound,
        "strategy":if args.interpolate {"ExactNewtonInterpolation".to_string()} else if args.numeric_points.is_some() {"ExactNumericImages".to_string()} else {format!("{:?}",args.strategy)},"backend":LicenseManager::get_version()});
    if args.numeric_points.is_some() || args.interpolate {
        let count = args.numeric_points.unwrap_or(degree_bound + 1);
        ensure!(
            count > 0 && count <= degree_bound + 1,
            "numeric sample count out of range"
        );
        ensure!(
            !args.interpolate || count == degree_bound + 1,
            "exact interpolation requires the full degree bound plus one"
        );
        let start = Instant::now();
        let mut points = Vec::new();
        let mut values = Vec::new();
        let mut samples = Vec::new();
        let images_dir = args
            .images_dir
            .clone()
            .unwrap_or_else(|| args.output.with_extension("images"));
        std::fs::create_dir_all(&images_dir)?;
        let backend = LicenseManager::get_version().to_string();
        let mut cache_hits = 0;
        let max_skips = p.degree(parameter) as usize + q.degree(parameter) as usize;
        for attempt in 0..count + max_skips + 1 {
            if points.len() == count {
                break;
            }
            let k = if attempt % 2 == 1 {
                (attempt as i64 + 1) / 2
            } else {
                -(attempt as i64 / 2)
            };
            let value = Rational::from(k);
            let left = p.replace(parameter, &value);
            let right = q.replace(parameter, &value);
            if left.degree(variable) != p.degree(variable)
                || right.degree(variable) != q.degree(variable)
            {
                continue;
            }
            write_atomic(
                &progress_path,
                &json!({"phase":"numeric_images","identity":identity,
                "next_parameter":k,"completed_points":points.len(),"cache_hits":cache_hits,"seconds":start.elapsed().as_secs_f64()}),
            )?;
            let started = Instant::now();
            let image_path = images_dir.join(format!("point-{k}.json"));
            let cached = std::fs::read(&image_path)
                .ok()
                .filter(|b| b.len() <= 1024 * 1024)
                .and_then(|b| serde_json::from_slice::<NumericImage>(&b).ok())
                .filter(|v| {
                    v.schema_version == 1
                        && v.backend == backend
                        && v.input_hash == capture.input_hash
                        && v.variables == algebra.names
                        && v.variable == variable
                        && v.parameter == parameter
                        && v.degrees == [p.degree(variable), q.degree(variable)]
                        && v.point == k
                        && v.value_sha256 == format!("{:x}", Sha256::digest(v.value.as_bytes()))
                })
                .and_then(|v| rational(&v.value).ok().map(|r| (r, v.seconds)));
            let was_cached = cached.is_some();
            let image = if let Some((r, _)) = &cached {
                cache_hits += 1;
                p.constant(r.clone())
            } else {
                exact_resultant_ducos(&left, &right, variable)?
            };
            ensure!(image.is_constant(), "numeric resultant was not constant");
            let constant = image.get_constant();
            // A zero resultant can have Symbolica's empty variable map. Newton
            // interpolation extends its first image in the original axis.
            let image = p.constant(constant.clone());
            let seconds = cached.map_or_else(|| started.elapsed().as_secs_f64(), |(_, s)| s);
            if !was_cached {
                let value = constant.to_string();
                write_atomic(
                    &image_path,
                    &NumericImage {
                        schema_version: 1,
                        backend: backend.clone(),
                        input_hash: capture.input_hash.clone(),
                        variables: algebra.names.clone(),
                        variable,
                        parameter,
                        degrees: [p.degree(variable), q.degree(variable)],
                        point: k,
                        value_sha256: format!("{:x}", Sha256::digest(value.as_bytes())),
                        value,
                        seconds,
                    },
                )?;
            }
            samples.push(json!({"parameter":k,"seconds":seconds,"cached":was_cached,
                "bits":constant.numerator_ref().significant_bits().max(constant.denominator_ref().significant_bits())}));
            points.push(value);
            values.push(image);
            write_atomic(
                &progress_path,
                &json!({"phase":"numeric_images","identity":identity,
                "completed_points":points.len(),"cache_hits":cache_hits,"seconds":start.elapsed().as_secs_f64()}),
            )?;
        }
        ensure!(points.len() == count, "insufficient full-degree images");
        let image_seconds = start.elapsed().as_secs_f64();
        let mut interpolation = None;
        if args.interpolate {
            write_atomic(
                &progress_path,
                &json!({"phase":"interpolation","identity":identity,
                "completed_points":count,"cache_hits":cache_hits,"image_seconds":image_seconds}),
            )?;
            let started = Instant::now();
            let result = Poly::newton_interpolation(&points, &values, parameter);
            ensure!(
                result.degree(parameter) as usize <= degree_bound,
                "interpolation degree exceeds bound"
            );
            let seconds = started.elapsed().as_secs_f64();
            let expression = algebra.format(&result);
            write_atomic(
                &args.output.with_extension("interpolated.json"),
                &json!({"identity":identity,"expression":expression,
                    "expression_sha256":format!("{:x}",Sha256::digest(expression.as_bytes()))}),
            )?;
            interpolation = Some(
                json!({"verified":true,"seconds":seconds,"terms":result.nterms(),
                "degree":result.degree(parameter),"algorithm":"Symbolica exact Newton interpolation"}),
            );
        }
        let result = json!({"identity":identity,"numeric_points":count,"image_seconds":image_seconds,
            "samples":samples,"images_dir":images_dir,"cache_hits":cache_hits,"interpolation":interpolation,"verified":args.interpolate});
        write_atomic(&progress_path, &json!({"phase":"finished","result":result}))?;
        return Ok(result);
    }
    write_atomic(
        &progress_path,
        &json!({"phase":"candidate_generation","identity":identity}),
    )?;
    let saved = if let Some(path) = &args.candidate {
        serde_json::from_slice::<Candidate>(&std::fs::read(path)?)?
    } else {
        let started = Instant::now();
        let a = p.to_univariate(variable);
        let b = q.to_univariate(variable);
        let polynomial = match args.strategy {
            Strategy::Crt => a.resultant_crt(&b),
            Strategy::Brown => a.resultant_brown(&b),
            Strategy::Primitive => a.resultant_primitive(&b),
            Strategy::IntegerDucos => exact_resultant_ducos(&p, &q, variable)?,
        };
        let generation_seconds = started.elapsed().as_secs_f64();
        let expression = algebra.format(&polynomial);
        ensure!(
            expression.len() <= 64 * 1024 * 1024,
            "candidate exceeds64MiB"
        );
        Candidate {
            input_hash: capture.input_hash.clone(),
            variables: algebra.names.clone(),
            eliminated_variable: variable,
            parameter,
            strategy: format!("{:?}", args.strategy),
            generation_seconds,
            expression_sha256: format!("{:x}", Sha256::digest(expression.as_bytes())),
            expression,
        }
    };
    ensure!(
        saved.input_hash == capture.input_hash
            && saved.variables == algebra.names
            && saved.eliminated_variable == variable
            && saved.parameter == parameter,
        "candidate input identity differs"
    );
    ensure!(
        saved.expression_sha256 == format!("{:x}", Sha256::digest(saved.expression.as_bytes())),
        "candidate checksum differs"
    );
    write_atomic(&args.output.with_extension("candidate.json"), &saved)?;
    let candidate = algebra.parse(&saved.expression)?;
    let candidate_info = json!({"generation_seconds":saved.generation_seconds,
        "generation_strategy":saved.strategy,
        "terms":candidate.nterms(),"degree":candidate.degree(parameter),
        "expression_sha256":saved.expression_sha256});
    write_atomic(
        &progress_path,
        &json!({"phase":"exact_validation","identity":identity,"candidate":candidate_info}),
    )?;
    let verification = verify_candidate(
        &p,
        &q,
        &candidate,
        variable,
        parameter,
        args.check_limit,
        |v| {
            write_atomic(
                &progress_path,
                &json!({"phase":"exact_validation","identity":identity,"candidate":candidate_info,"verification":v}),
            )
        },
    )?;
    let report = json!({"identity":identity,"candidate":candidate_info,"verification":verification,
        "proof_criterion":"Equality is proved only when verified=true: actual and candidate resultants have degree at most D in the sole remaining parameter, and their difference vanishes at D+1 distinct exact points preserving both input degrees."});
    write_atomic(&progress_path, &json!({"phase":"finished","result":report}))?;
    Ok(report)
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        println!("{}", serde_json::to_string(&worker(&args)?)?);
        return Ok(());
    }
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new(&executable);
    command
        .arg(&args.capture)
        .arg("--output")
        .arg(&args.output)
        .arg("--strategy")
        .arg(match args.strategy {
            Strategy::Crt => "crt",
            Strategy::Brown => "brown",
            Strategy::Primitive => "primitive",
            Strategy::IntegerDucos => "integer-ducos",
        })
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    if let Some(path) = &args.candidate {
        command.arg("--candidate").arg(path);
    }
    if let Some(limit) = args.check_limit {
        command.arg("--check-limit").arg(limit.to_string());
    }
    if let Some(points) = args.numeric_points {
        command.arg("--numeric-points").arg(points.to_string());
    }
    if args.interpolate {
        command.arg("--interpolate");
    }
    if let Some(path) = &args.images_dir {
        command.arg("--images-dir").arg(path);
    }
    if args.factor_interpolated {
        command.arg("--factor-interpolated");
    }
    if args.compact_factor_map {
        command.arg("--compact-factor-map");
    }
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut outcome = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<serde_json::Value>(&outcome.stdout).ok();
    if result.is_some() {
        outcome.stdout.clear();
    }
    let report = json!({"process":outcome,"result":result,"limits":limits,
        "binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?))});
    write_atomic(&args.output, &report)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_factor_map_restores_inactive_axes_multiplicities_and_scalar() {
        let a = Algebra::new(&["before".into(), "s".into(), "after".into()]).unwrap();
        for expression in ["-7/3", "-7*(s-1)^3*(s^2+1)^2/3", "s^5*(s+2)^2"] {
            let polynomial = a.parse(expression).unwrap();
            let compact = factor_in_compact_map(&a, &polynomial).unwrap();
            assert_eq!(
                compact,
                factors_checked(std::slice::from_ref(&polynomial)).unwrap()
            );
            assert!(
                compact
                    .iter()
                    .all(|p| p.variables() == polynomial.variables())
            );
        }
    }

    #[test]
    fn exact_identity_skips_degree_drops_and_rejects_wrong_candidates() {
        let a = Algebra::new(&["s".into(), "t".into(), "u".into()]).unwrap();
        let p = a.parse("s*t+1").unwrap();
        let q = a.parse("t^2+s").unwrap();
        let r = exact_resultant_ducos(&p, &q, 1).unwrap();
        let v = verify_candidate(&p, &q, &r, 1, 0, None, |_| Ok(())).unwrap();
        assert!(v.verified);
        assert_eq!(v.degree_bound, 3);
        assert_eq!(v.checked_points, 4);
        assert_eq!(v.skipped_degree_drop_points, 1);
        for wrong in [
            &r + &a.parse("1").unwrap(),
            &r + &a.parse("u").unwrap(),
            &r + &a.parse("s^4").unwrap(),
        ] {
            assert!(verify_candidate(&p, &q, &wrong, 1, 0, None, |_| Ok(())).is_err());
        }
        assert!(
            !verify_candidate(&p, &q, &r, 1, 0, Some(2), |_| Ok(()))
                .unwrap()
                .verified
        );
    }

    #[test]
    fn homogeneous_degree_bound_and_symbolica_interpolation_preserve_exact_scale() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        let p = a.parse("(s^3+s^2*t+s*t^2+t^3+1)/2").unwrap();
        let q = a.parse("(-s^2-t^2-1)/3").unwrap();
        assert_eq!(bound(&p, &q, 1, 0).unwrap(), 6);
        let expected = exact_resultant_ducos(&p, &q, 1).unwrap();
        assert!(expected.degree(0) <= 6);
        let points: Vec<_> = (0..7).map(Rational::from).collect();
        let images: Vec<_> = points
            .iter()
            .map(|s| exact_resultant_ducos(&p.replace(0, s), &q.replace(0, s), 1).unwrap())
            .collect();
        let interpolated = Poly::newton_interpolation(&points, &images, 0);
        assert_eq!(interpolated, expected);
        assert!(
            verify_candidate(&p, &q, &interpolated, 1, 0, None, |_| Ok(()))
                .unwrap()
                .verified
        );
    }

    #[test]
    fn first_zero_numeric_image_keeps_the_original_interpolation_axis() {
        let a = Algebra::new(&["s".into(), "t".into()]).unwrap();
        let p = a.parse("t^2+s").unwrap();
        let q = a.parse("t^3+s*t+1").unwrap();
        // Use q=t^3+s*t+s so both inputs share t at the first point s=0.
        let q = &q - &a.parse("1-s").unwrap();
        let degree = bound(&p, &q, 1, 0).unwrap();
        let points: Vec<_> = (0..=degree).map(|i| Rational::from(i as i64)).collect();
        let values: Vec<_> = points
            .iter()
            .map(|s| {
                let image = exact_resultant_ducos(&p.replace(0, s), &q.replace(0, s), 1).unwrap();
                p.constant(image.get_constant())
            })
            .collect();
        assert!(values[0].is_zero());
        assert_eq!(values[0].variables(), p.variables());
        let result = Poly::newton_interpolation(&points, &values, 0);
        assert_eq!(result, exact_resultant_ducos(&p, &q, 1).unwrap());
    }
}
