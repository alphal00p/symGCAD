//! Research-only bounded real-root isolation of a signed discriminant operand.
//! A completed root inventory is not a CAD result or a nonvanishing certificate.
use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    roots::{FiberRoot, isolate_union_observed},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Operand {
    Original,
    Residual,
    FactorBlock,
    FactorUnion,
}

#[derive(Parser)]
struct Args {
    candidate: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_enum, default_value = "residual")]
    operand: Operand,
    #[arg(long)]
    factors: Option<PathBuf>,
    #[arg(long, requires = "factors")]
    factor_index: Option<usize>,
    #[arg(long, default_value_t = 10)]
    cpu: usize,
    #[arg(long, default_value_t = 180.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Deserialize)]
struct Candidate {
    operation: String,
    input_hash: String,
    result: String,
    result_sha256: String,
    result_terms: usize,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, max: usize) -> Result<Vec<u8>> {
    ensure!(fs::metadata(path)?.is_file(), "regular file required");
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= max, "input byte cap");
    Ok(bytes)
}

// The recorded candidate uses this integer-univariate grammar. Reject powers,
// coefficients and term counts outside the research envelope before parsing.
fn parse(a: &Algebra, expression: &str) -> Result<Poly> {
    let mut rest = expression;
    let mut p = a.parse("0")?;
    let mut last = None;
    loop {
        let text = rest.strip_prefix('(').context("coefficient prefix")?;
        let (coefficient, text) = text.split_once(')').context("coefficient suffix")?;
        let digits = coefficient.strip_prefix('-').unwrap_or(coefficient);
        ensure!(
            !digits.is_empty()
                && digits.len() <= 6000
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "coefficient grammar/height cap"
        );
        let text = text
            .strip_prefix("*s^")
            .context("explicit s power required")?;
        let end = text.find('+').unwrap_or(text.len());
        let exponent = &text[..end];
        ensure!(
            !exponent.is_empty()
                && exponent.len() <= 4
                && exponent.bytes().all(|b| b.is_ascii_digit()),
            "exponent grammar"
        );
        let exponent: u32 = exponent.parse()?;
        ensure!(
            exponent <= 4096 && last.is_none_or(|old| exponent > old) && p.nterms() < 4096,
            "degree/order/term cap"
        );
        let coefficient = rational(coefficient)?;
        ensure!(!coefficient.is_zero(), "zero sparse coefficient");
        p.append_monomial(coefficient, &[exponent]);
        last = Some(exponent);
        if end == text.len() {
            break;
        }
        rest = &text[end + 1..];
    }
    Ok(p)
}

fn summary(a: &Algebra, p: &Poly) -> Value {
    json!({"degree":p.degree(0),"terms":p.nterms(),"sha256":hash(a.format(p).as_bytes()),
        "maximum_coefficient_bits":p.coefficients.iter().map(|c|c.numerator_ref().significant_bits()).max().unwrap_or(0)})
}

fn deflate(a: &Algebra, p: &Poly) -> Result<(Poly, u32, usize)> {
    let zero = p
        .exponents_iter()
        .map(|e| e[0])
        .min()
        .context("empty input")?;
    let mut residual = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        residual.append_monomial(c.clone(), &[e[0] - zero]);
    }
    ensure!(
        residual.clone().mul_monomial(&Rational::one(), &[zero]) == *p,
        "zero-power identity"
    );
    let threshold = a.parse("s-4")?;
    let linear = threshold.to_univariate_from_univariate(0);
    let mut fiber = residual.to_univariate_from_univariate(0);
    let mut four = 0;
    while fiber.degree() > 0 && fiber.evaluate(&Rational::from(4)).is_zero() {
        ensure!(four < 64, "threshold multiplicity cap");
        let quotient = fiber
            .try_div(&linear)
            .context("nonexact threshold division")?;
        ensure!(
            &quotient * &linear == fiber,
            "threshold division reconstruction"
        );
        fiber = quotient;
        four += 1;
    }
    residual = fiber.to_multivariate::<u32>();
    ensure!(
        (&residual * &threshold.pow(four)).mul_monomial(&Rational::one(), &[zero]) == *p,
        "threshold reconstruction"
    );
    Ok((residual, zero, four))
}

fn factor_inputs(
    a: &Algebra,
    p: &Poly,
    candidate_hash: &str,
    bytes: &[u8],
    selected: Option<usize>,
) -> Result<Vec<(usize, Poly)>> {
    let sidecar: Value = serde_json::from_slice(bytes)?;
    ensure!(
        sidecar["schema_version"] == 1
            && sidecar["variable"] == "s"
            && sidecar["candidate_sha256"] == candidate_hash,
        "factor sidecar identity"
    );
    let factors = sidecar["factors"].as_array().context("missing factors")?;
    ensure!(
        !factors.is_empty() && factors.len() <= 64,
        "factor count cap"
    );
    ensure!(
        selected.is_none_or(|i| i < factors.len()),
        "factor index out of range"
    );
    let mut parsed = Vec::new();
    let mut degree = 0u64;
    let mut coefficient_bits = 0u64;
    for f in factors {
        let expression = f["expression"].as_str().context("factor expression")?;
        ensure!(
            f["expression_sha256"] == hash(expression.as_bytes()),
            "factor hash mismatch"
        );
        let factor = parse(a, expression)?;
        let multiplicity = f["multiplicity"].as_u64().context("factor multiplicity")?;
        ensure!(multiplicity > 0 && multiplicity <= 4096, "multiplicity cap");
        degree = degree
            .checked_add(u64::from(factor.degree(0)) * multiplicity)
            .context("degree overflow")?;
        let height = factor
            .coefficients
            .iter()
            .map(|c| c.numerator_ref().significant_bits())
            .max()
            .unwrap_or(0);
        coefficient_bits = coefficient_bits
            .checked_add(
                (height + (factor.nterms().max(1).next_power_of_two().trailing_zeros() as u64))
                    * multiplicity,
            )
            .context("coefficient bound overflow")?;
        ensure!(
            degree <= u64::from(p.degree(0)) && coefficient_bits <= 65536,
            "factor product work cap"
        );
        parsed.push((factor, multiplicity as usize));
    }
    ensure!(
        degree == u64::from(p.degree(0)) && (degree + 1) * coefficient_bits <= 256_000_000,
        "factor product bounds"
    );
    let mut product = p.one();
    for (factor, multiplicity) in &parsed {
        product = &product * &factor.pow(*multiplicity);
    }
    ensure!(
        product == *p,
        "signed original factor reconstruction failed"
    );
    let inputs = parsed
        .into_iter()
        .enumerate()
        .filter(|(i, (f, _))| selected.is_none_or(|j| j == *i) && !f.is_constant())
        .map(|(i, (f, _))| (i, f))
        .collect::<Vec<_>>();
    ensure!(!inputs.is_empty(), "selected factor is constant");
    Ok(inputs)
}

fn publish(path: &Path, value: &Value) -> Result<()> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

fn intervals(roots: &[FiberRoot]) -> Result<Value> {
    ensure!(roots.len() <= 4096, "root count cap");
    for pair in roots.windows(2) {
        ensure!(
            pair[0].interval.1 < pair[1].interval.0,
            "root intervals not strictly ordered"
        );
    }
    let mut output = Vec::new();
    for (global_index, root) in roots.iter().enumerate() {
        ensure!(root.interval.0 <= root.interval.1, "reversed root interval");
        if root.interval.0 == root.interval.1 {
            ensure!(
                root.fiber.evaluate(&root.interval.0).is_zero(),
                "false exact rational root"
            );
        }
        output.push(json!({"global_distinct_index":global_index,"polynomial":root.polynomial,"source_index":root.index,
            "lower":root.interval.0.to_string(),"upper":root.interval.1.to_string(),"retained_fiber_degree":root.fiber.degree()}));
    }
    Ok(json!(output))
}

fn worker(args: &Args) -> Result<()> {
    let bytes = read(&args.candidate, 16 * 1024 * 1024)?;
    let candidate_hash = hash(&bytes);
    let candidate: Candidate = serde_json::from_slice(&bytes)?;
    ensure!(
        candidate.operation == "discriminant"
            && hash(candidate.result.as_bytes()) == candidate.result_sha256,
        "candidate hash/operation mismatch"
    );
    let algebra = Algebra::new(&["s".into()])?;
    let p = parse(&algebra, &candidate.result)?;
    ensure!(
        p.nterms() == candidate.result_terms,
        "candidate term count mismatch"
    );
    let started = Instant::now();
    let factor_mode = matches!(args.operand, Operand::FactorBlock | Operand::FactorUnion);
    ensure!(
        factor_mode == args.factors.is_some(),
        "factor sidecar required only for factor operands"
    );
    ensure!(
        (args.operand == Operand::FactorBlock) == args.factor_index.is_some(),
        "factor-block requires exactly one factor index"
    );
    let mut sidecar_hash = None;
    let (inputs, zero, four) = if factor_mode {
        let bytes = read(args.factors.as_ref().unwrap(), 17 * 1024 * 1024)?;
        sidecar_hash = Some(hash(&bytes));
        (
            factor_inputs(&algebra, &p, &candidate_hash, &bytes, args.factor_index)?,
            0,
            0,
        )
    } else if args.operand == Operand::Residual {
        let (input, zero, four) = deflate(&algebra, &p)?;
        (vec![(0, input)], zero, four)
    } else {
        (vec![(0, p.clone())], 0, 0)
    };
    let preparation_seconds = started.elapsed().as_secs_f64();
    let progress_path = args.output.with_extension("progress.json");
    let mut report = json!({"complete":false,"original_coverage_claimed":false,"candidate_sha256":hash(&bytes),
        "input_hash":candidate.input_hash,"discriminant_sha256":candidate.result_sha256,"operand":format!("{:?}",args.operand),
        "inputs":inputs.iter().map(|(id,p)|json!({"polynomial":id,"summary":summary(&algebra,p)})).collect::<Vec<_>>(),"factor_sidecar_sha256":sidecar_hash,
        "exact_signed_factor_reconstruction":factor_mode,"removed_zero_multiplicity":zero,"removed_four_multiplicity":four,
        "preparation_seconds":preparation_seconds,"phase":"prepared"});
    publish(&progress_path, &report)?;
    let root_started = Instant::now();
    let mut events = 0usize;
    let single_id = (inputs.len() == 1).then_some(inputs[0].0);
    let fibers = inputs
        .into_iter()
        .map(|(id, p)| (id, p.to_univariate_from_univariate(0)))
        .collect();
    let roots = isolate_union_observed(fibers, |phase, id, fiber, count| {
        events += 1;
        if events == 1 || events % 128 == 0 {
            report["phase"] = json!(phase);
            report["current_polynomial"] = json!(id);
            report["current_fiber_degree"] = json!(fiber.degree());
            report["observed_roots"] = json!(count);
            report["root_seconds"] = json!(root_started.elapsed().as_secs_f64());
            publish(&progress_path, &report)?;
        }
        Ok(())
    })?;
    let isolation_seconds = root_started.elapsed().as_secs_f64();
    if let Some(id) = single_id {
        ensure!(
            roots
                .iter()
                .enumerate()
                .all(|(i, r)| r.polynomial == id && r.index == i),
            "single-polynomial full-real ordinals changed"
        );
    }
    report["intervals"] = intervals(&roots)?;
    report["intervals_sha256"] = json!(hash(&serde_json::to_vec(&report["intervals"])?));
    report["distinct_real_roots"] = json!(roots.len());
    report["isolation_seconds"] = json!(isolation_seconds);
    report["observer_calls"] = json!(events);
    report["complete"] = json!(true);
    report["phase"] = json!("complete");
    report["limitations"] = json!([
        "Root inventory only for the selected operand, not original CAD coverage",
        "Native exact isolation includes square-free factorization and may trigger an additional square-free GCD in symGCAD",
        "Internal native phase times are not observed by the root-union callback"
    ]);
    publish(&args.output.with_extension("worker.json"), &report)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 180.0,
        "deadline cap"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists() && !args.output.with_extension("worker.json").exists(),
        "output must be fresh"
    );
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let exe = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let process = run_bounded(&mut command, &limits)?;
    let result = read(&args.output.with_extension("worker.json"), 32 * 1024 * 1024)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let completed = process.status == "completed"
        && process.exit_code == Some(0)
        && result.as_ref().is_some_and(|r| r["complete"] == true);
    publish(
        &args.output,
        &json!({"binary_sha256":hash(&fs::read(exe)?),"limits":limits,"cpu":args.cpu,"process":process,"result":result}),
    )?;
    ensure!(
        completed,
        "bounded isolation incomplete; progress and process retained"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::roots::{isolate_union, roots_in_open_interval};
    #[test]
    fn deflation_and_repeated_real_roots_preserve_exact_inventory() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let p = a.parse("-3*s^4*(s-4)^2*(s^2-2)^3*(s-6)").unwrap();
        let (residual, zero, four) = deflate(&a, &p).unwrap();
        assert_eq!((zero, four), (4, 2));
        assert_eq!(residual, a.parse("-3*(s^2-2)^3*(s-6)").unwrap());
        let roots = isolate_union(vec![(0, p.to_univariate_from_univariate(0))]).unwrap();
        assert_eq!(roots.len(), 5);
        assert!(intervals(&roots).is_ok());
        for root in &roots {
            if root.interval.0 != root.interval.1 {
                assert_eq!(
                    roots_in_open_interval(&root.fiber, &root.interval.0, &root.interval.1),
                    1
                );
            }
        }
        let partial = isolate_union(vec![(0, residual.to_univariate_from_univariate(0))]).unwrap();
        assert_eq!(partial.len(), 3);
    }
    #[test]
    fn shared_roots_have_exact_owner_indices_and_ordered_intervals() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let roots = isolate_union(vec![
            (
                0,
                a.parse("(s-1)^2*(s^2-2)")
                    .unwrap()
                    .to_univariate_from_univariate(0),
            ),
            (
                1,
                a.parse("(s-1)*(s+2)")
                    .unwrap()
                    .to_univariate_from_univariate(0),
            ),
        ])
        .unwrap();
        assert_eq!(
            roots
                .iter()
                .map(|r| (r.polynomial, r.index))
                .collect::<Vec<_>>(),
            vec![(1, 0), (0, 0), (0, 1), (0, 2)]
        );
        assert!(intervals(&roots).is_ok());
    }
    #[test]
    fn bounded_sparse_input_rejects_hidden_expansion_and_duplicates() {
        let a = Algebra::new(&["s".into()]).unwrap();
        assert_eq!(
            parse(&a, "(-2)*s^0+(3)*s^7").unwrap(),
            a.parse("-2+3*s^7").unwrap()
        );
        for bad in [
            "(1)*s^9999",
            "(1)*s^1+(2)*s^1",
            "(2^999999)*s^0",
            "(0)*s^4",
            "(1/2)*s^1",
        ] {
            assert!(parse(&a, bad).is_err());
        }
    }
    #[test]
    fn sidecar_blocks_require_full_signed_reconstruction_before_selection() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let p = a.parse("-2*(s-1)^2*(s+2)").unwrap();
        let factors = [
            ("(-2)*s^0", 1),
            ("(-1)*s^0+(1)*s^1", 2),
            ("(2)*s^0+(1)*s^1", 1),
        ];
        let mut sidecar = json!({"schema_version":1,"variable":"s","candidate_sha256":"bound","factors":factors.iter().map(|(expression,multiplicity)|json!({"expression":expression,"expression_sha256":hash(expression.as_bytes()),"multiplicity":multiplicity})).collect::<Vec<_>>()});
        let inputs = factor_inputs(
            &a,
            &p,
            "bound",
            &serde_json::to_vec(&sidecar).unwrap(),
            Some(1),
        )
        .unwrap();
        assert_eq!(inputs[0].1, a.parse("s-1").unwrap());
        sidecar["factors"][2]["multiplicity"] = json!(2);
        assert!(
            factor_inputs(
                &a,
                &p,
                "bound",
                &serde_json::to_vec(&sidecar).unwrap(),
                Some(1)
            )
            .is_err()
        );
    }
}
