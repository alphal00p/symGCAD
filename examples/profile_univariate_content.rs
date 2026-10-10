//! Research-only content/SQF measurement. No irreducible factoring, roots or CAD claim.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, rational},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_TERMS: usize = 4096;
const MAX_DEGREE: u32 = 4096;
const MAX_DIGITS: usize = 13_546;
const MAX_HEIGHT: u64 = 45_000;
const MAX_BITS: u64 = 140_000_000;
const MAX_DENOMINATOR_BITS: u64 = 65_536;
const MAX_BLOCKS: usize = 64;
const MAX_PRODUCT_HEIGHT: u64 = 65_536;
const MAX_PRODUCT_WORK: u64 = 256_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Stage {
    Content,
    SquareFree,
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    candidate: PathBuf,
    #[arg(long)]
    expected_capture_sha256: String,
    #[arg(long)]
    expected_candidate_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 10)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, value_enum, default_value = "content")]
    stage: Stage,
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
fn read(path: &Path) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let meta = file.metadata()?;
    ensure!(
        meta.is_file() && meta.len() <= MAX_BYTES as u64,
        "regular input/byte cap"
    );
    let mut data = Vec::new();
    (&mut file)
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() <= MAX_BYTES, "input growth cap");
    Ok(data)
}
fn checked_map(names: &[String]) -> Result<()> {
    ensure!(
        !names.is_empty() && names.len() <= 64 && names[0] == "s",
        "full map/first axis"
    );
    let mut seen = std::collections::HashSet::new();
    for name in names {
        ensure!(
            !name.is_empty()
                && name.len() <= 128
                && name.as_bytes()[0].is_ascii_alphabetic()
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && seen.insert(name),
            "bounded unique alias required"
        );
    }
    Ok(())
}
// Complete structural/literal preflight precedes Algebra construction and all
// Symbolica number parsing. 10^1000 < 2^3322 makes this decimal bit bound safe.
fn integer_bound(text: &str, signed: bool) -> Result<u64> {
    let digits = if signed {
        text.strip_prefix('-').unwrap_or(text)
    } else {
        text
    };
    ensure!(
        !digits.is_empty()
            && digits.len() <= MAX_DIGITS
            && digits.bytes().all(|b| b.is_ascii_digit())
            && digits.as_bytes()[0] != b'0',
        "nonzero canonical integer literal/height"
    );
    let bound = (digits.len() as u64 * 3322).div_ceil(1000);
    ensure!(bound <= MAX_HEIGHT, "literal bit bound");
    Ok(bound)
}
fn preflight(expression: &str) -> Result<Vec<(&str, u32)>> {
    ensure!(
        !expression.is_empty() && expression.len() <= MAX_BYTES,
        "expression byte cap"
    );
    let mut terms = Vec::new();
    let mut bits = 0u64;
    let mut denominator_bits = 0u64;
    let mut last = None;
    for term in expression.split('+') {
        ensure!(terms.len() < MAX_TERMS, "term cap");
        let tail = term
            .strip_prefix('(')
            .ok_or_else(|| anyhow::anyhow!("coefficient opening"))?;
        let (coefficient, exponent) = tail
            .split_once(")*s^")
            .ok_or_else(|| anyhow::anyhow!("sparse grammar"))?;
        let (num, den) = coefficient.split_once('/').unwrap_or((coefficient, "1"));
        let nb = integer_bound(num, true)?;
        let db = if den == "1" {
            1
        } else {
            integer_bound(den, false)?
        };
        bits = bits
            .checked_add(nb + db)
            .ok_or_else(|| anyhow::anyhow!("bit overflow"))?;
        denominator_bits += db;
        ensure!(
            bits <= MAX_BITS && denominator_bits <= MAX_DENOMINATOR_BITS,
            "input coefficient/LCM cap"
        );
        ensure!(
            !exponent.is_empty()
                && exponent.len() <= 4
                && exponent.bytes().all(|b| b.is_ascii_digit()),
            "exponent grammar"
        );
        let exponent: u32 = exponent.parse()?;
        ensure!(
            exponent <= MAX_DEGREE && last.is_none_or(|x| exponent > x),
            "exponent/order cap"
        );
        last = Some(exponent);
        terms.push((coefficient, exponent));
    }
    Ok(terms)
}
fn parse(names: &[String], expression: &str) -> Result<(Algebra, Poly)> {
    checked_map(names)?;
    let terms = preflight(expression)?;
    let algebra = Algebra::new(names)?;
    let mut p = algebra.parse("0")?;
    let mut exponents = vec![0; names.len()];
    for (coefficient, exponent) in terms {
        let c = rational(coefficient)?;
        ensure!(
            !c.is_zero()
                && c.numerator_ref().significant_bits() <= MAX_HEIGHT
                && c.denominator_ref().significant_bits() <= MAX_HEIGHT,
            "exact coefficient cap"
        );
        exponents[0] = exponent;
        p.append_monomial(c, &exponents);
    }
    ensure!(
        !p.is_zero() && p.variables() == &algebra.variables,
        "nonzero/full exponent map"
    );
    Ok((algebra, p))
}
fn authenticate(
    capture_bytes: &[u8],
    candidate_bytes: &[u8],
    expected_capture: &str,
    expected_candidate: &str,
) -> Result<(OperationCapture, Candidate)> {
    ensure!(
        hash(capture_bytes) == expected_capture && hash(candidate_bytes) == expected_candidate,
        "authenticated file digest mismatch"
    );
    let capture: OperationCapture = serde_json::from_slice(capture_bytes)?;
    let candidate: Candidate = serde_json::from_slice(candidate_bytes)?;
    checked_map(&capture.variable_names)?;
    ensure!(
        capture.operation == AlgebraOperation::Discriminant
            && capture
                .variable
                .is_some_and(|x| x > 0 && x < capture.variable_names.len()),
        "discriminant operation/axis"
    );
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing operand"))?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == capture.input_hash,
        "captured operand binding"
    );
    ensure!(
        candidate.operation == "discriminant"
            && candidate.input_hash == capture.input_hash
            && hash(candidate.result.as_bytes()) == candidate.result_sha256,
        "candidate operation/expression binding"
    );
    Ok((capture, candidate))
}
fn reconstruction(raw: &Poly, primitive: &Poly, scale: &Rational) -> Result<()> {
    ensure!(
        !raw.is_zero() && !primitive.is_zero() && raw.variables() == primitive.variables(),
        "reconstruction nonzero/map"
    );
    ensure!(
        &raw.constant(scale.clone()) * primitive == *raw,
        "exact signed scalar reconstruction"
    );
    Ok(())
}
fn content(raw: &Poly) -> Result<(Poly, Rational)> {
    ensure!(!raw.is_zero(), "zero has no scalar-content stage");
    let primitive = canonical(raw);
    ensure!(
        primitive.variables() == raw.variables()
            && !primitive.lcoeff().is_negative()
            && primitive.coefficients.iter().all(|c| c.is_integer()),
        "canonical integer/full-map invariant"
    );
    let scale = raw.lcoeff() / primitive.lcoeff();
    Ok((primitive, scale))
}
// Bound native output before any factor powers/products. The integer primitive
// input must be reconstructed with every native scalar and multiplicity.
fn product_preflight(source: &Poly, factors: &mut [(Poly, usize)]) -> Result<()> {
    ensure!(
        !source.is_constant() && !factors.is_empty() && factors.len() <= MAX_BLOCKS,
        "nonconstant source/block count cap"
    );
    let mut degree = 0u64;
    let mut bits = 0u64;
    let mut height_bound = 0u64;
    for (p, multiplicity) in factors {
        ensure!(
            *multiplicity > 0 && *multiplicity <= MAX_DEGREE as usize && !p.is_zero(),
            "nonzero factor/multiplicity cap"
        );
        if p.is_constant() {
            *p = source.constant(p.get_constant());
        }
        ensure!(
            p.variables() == source.variables()
                && p.nterms() <= MAX_TERMS
                && p.exponents_iter()
                    .all(|e| e.iter().skip(1).all(|v| *v == 0)),
            "factor full map/support/term cap"
        );
        degree = degree
            .checked_add(u64::from(p.degree(0)) * *multiplicity as u64)
            .ok_or_else(|| anyhow::anyhow!("weighted degree overflow"))?;
        ensure!(degree <= u64::from(source.degree(0)), "weighted degree cap");
        let mut height = 0u64;
        for c in &p.coefficients {
            let h = c.numerator_ref().significant_bits();
            ensure!(
                c.is_integer() && h <= MAX_HEIGHT,
                "integer native coefficient height cap"
            );
            height = height.max(h);
            bits = bits
                .checked_add(h + 1)
                .ok_or_else(|| anyhow::anyhow!("block storage overflow"))?;
        }
        ensure!(bits <= MAX_BITS, "total retained block coefficient cap");
        let term_bound = u64::from(p.nterms().max(1).next_power_of_two().trailing_zeros());
        height_bound = height_bound
            .checked_add((height + term_bound) * *multiplicity as u64)
            .ok_or_else(|| anyhow::anyhow!("product height overflow"))?;
        ensure!(
            height_bound <= MAX_PRODUCT_HEIGHT
                && (u64::from(source.degree(0)) + 1) * height_bound <= MAX_PRODUCT_WORK,
            "reconstruction coefficient/work cap"
        );
    }
    ensure!(
        degree == u64::from(source.degree(0)),
        "complete weighted source degree"
    );
    Ok(())
}
fn reconstruct_factors(
    raw: &Poly,
    source: &Poly,
    scale: &Rational,
    factors: &mut [(Poly, usize)],
) -> Result<()> {
    product_preflight(source, factors)?;
    let mut product = source.one();
    for (p, multiplicity) in factors {
        product = &product * &p.pow(*multiplicity);
    }
    ensure!(
        product == *source,
        "complete canonical SQF product mismatch"
    );
    reconstruction(raw, &product, scale)?;
    Ok(())
}
fn summary(p: &Poly) -> Result<Value> {
    let mut numerator_bits = 0u64;
    let mut denominator_bits = 0u64;
    let mut height = 0u64;
    let mut denominator_height = 0u64;
    let mut text_bytes = 0usize;
    let mut digest = Sha256::new();
    for (i, (c, e)) in p.coefficients.iter().zip(p.exponents_iter()).enumerate() {
        ensure!(
            e.len() == p.nvars() && e.iter().skip(1).all(|v| *v == 0),
            "summary support/map"
        );
        let nb = c.numerator_ref().significant_bits();
        let db = c.denominator_ref().significant_bits();
        numerator_bits += nb;
        denominator_bits += db;
        height = height.max(nb);
        denominator_height = denominator_height.max(db);
        let term = format!("{}({c})*s^{}", if i == 0 { "" } else { "+" }, e[0]);
        text_bytes += term.len();
        ensure!(text_bytes <= MAX_BYTES, "summary text cap");
        digest.update(term.as_bytes());
    }
    let bits = numerator_bits + denominator_bits;
    ensure!(bits <= MAX_BITS, "summary total coefficient cap");
    let production_source_caps = !p.is_constant()
        && (2..=4096).contains(&p.degree(0))
        && p.nterms() <= 4096
        && p.nvars() <= 64
        && height <= 20_000
        && denominator_height <= 20_000
        && bits <= 64_000_000
        && denominator_bits <= 65_536
        && text_bytes <= 32 * 1024 * 1024;
    Ok(
        json!({"degree":p.degree(0), "terms":p.nterms(), "map_dimensions":p.nvars(),
        "minimum_exponent":p.exponents_iter().map(|e| e[0]).min(),
        "max_numerator_bits":height,"max_denominator_bits":denominator_height,
        "summed_numerator_bits":numerator_bits,"summed_denominator_bits":denominator_bits,
        "coefficient_cost_bits":bits,"sparse_expression_bytes":text_bytes,
        "sparse_expression_sha256":format!("{:x}",digest.finalize()),
        "fits_current_source_caps_only":production_source_caps,
        "basis_admission_certified":false}),
    )
}
fn progress(output: &Path, report: &Value) -> Result<()> {
    let temp = output.with_extension("progress.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(report)?)?;
    fs::rename(temp, output.with_extension("progress.json"))?;
    Ok(())
}
fn worker(args: &Args) -> Result<()> {
    let started = Instant::now();
    let capture_bytes = read(&args.capture)?;
    let candidate_bytes = read(&args.candidate)?;
    let (capture, candidate) = authenticate(
        &capture_bytes,
        &candidate_bytes,
        &args.expected_capture_sha256,
        &args.expected_candidate_sha256,
    )?;
    let parse_started = Instant::now();
    let (_algebra, raw) = parse(&capture.variable_names, &candidate.result)?;
    ensure!(
        raw.nterms() == candidate.result_terms,
        "candidate term count"
    );
    let mut report = json!({"schema_version":1,"phase":"parsed","stage":format!("{:?}",args.stage),
        "candidate_only":true,"discriminant_verified":false,"original_coverage":false,
        "capture_sha256":hash(&capture_bytes),"candidate_sha256":hash(&candidate_bytes),
        "candidate_expression_sha256":candidate.result_sha256,"input_hash":candidate.input_hash,
        "full_variable_map":capture.variable_names,"eliminated_axis":capture.variable,
        "parse_seconds":parse_started.elapsed().as_secs_f64()});
    progress(&args.output, &report)?;
    let step = Instant::now();
    let (primitive, scale) = content(&raw)?;
    report["canonicalization_seconds"] = json!(step.elapsed().as_secs_f64());
    report["phase"] = json!("reconstruction");
    progress(&args.output, &report)?;
    let step = Instant::now();
    reconstruction(&raw, &primitive, &scale)?;
    report["reconstruction_seconds"] = json!(step.elapsed().as_secs_f64());
    report["exact_signed_scalar_reconstruction"] = json!(true);
    let step = Instant::now();
    report["raw"] = summary(&raw)?;
    report["canonical"] = summary(&primitive)?;
    report["scale"] = json!({"sign":if scale.is_negative() {-1} else {1},
        "numerator_bits":scale.numerator_ref().significant_bits(),
        "denominator_bits":scale.denominator_ref().significant_bits(),
        "rational_text_sha256":hash(scale.to_string().as_bytes())});
    report["metadata_seconds"] = json!(step.elapsed().as_secs_f64());
    if args.stage == Stage::SquareFree {
        ensure!(!primitive.is_constant(), "SQF requires nonconstant input");
        ensure!(
            primitive
                .coefficients
                .iter()
                .all(|c| c.is_integer() && c.numerator_ref().significant_bits() <= MAX_HEIGHT),
            "canonical SQF input height cap"
        );
        // summary() already checks total coefficient storage before native work.
        report["phase"] = json!("native_square_free");
        progress(&args.output, &report)?;
        let step = Instant::now();
        let mut factors = primitive.square_free_factorization();
        report["native_square_free_seconds"] = json!(step.elapsed().as_secs_f64());
        report["phase"] = json!("factor_reconstruction");
        progress(&args.output, &report)?;
        let step = Instant::now();
        reconstruct_factors(&raw, &primitive, &scale, &mut factors)?;
        report["factor_reconstruction_seconds"] = json!(step.elapsed().as_secs_f64());
        report["exact_canonical_factor_product"] = json!(true);
        report["exact_signed_raw_factor_product"] = json!(true);
        report["factors"] = json!(
            factors
                .iter()
                .map(|(p, m)| Ok(json!({"multiplicity":m,"polynomial":summary(p)?})))
                .collect::<Result<Vec<_>>>()?
        );
    }
    report["total_worker_seconds"] = json!(started.elapsed().as_secs_f64());
    report["phase"] = json!("complete");
    report["caveat"] = json!(
        "Content and optionally native square-free decomposition of an authenticated candidate only. No irreducible factoring, roots, basis admission, candidate discriminant identity, or original CAD coverage. Current checker raw-source preflight must be assessed separately from cached canonical builder input."
    );
    progress(&args.output, &report)?;
    println!("{report}");
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite()
            && args.seconds > 0.0
            && args.seconds
                <= if args.stage == Stage::Content {
                    60.0
                } else {
                    300.0
                },
        "deadline content (0,60], square-free (0,300]"
    );
    for h in [
        &args.expected_capture_sha256,
        &args.expected_candidate_sha256,
    ] {
        ensure!(
            h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()),
            "expected SHA256 required"
        );
    }
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists()
            && !args.output.with_extension("progress.json").exists()
            && !args.output.with_extension("progress.tmp").exists(),
        "fresh output and sidecars required"
    );
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
        .arg(&args.capture)
        .arg(&args.candidate)
        .arg("--expected-capture-sha256")
        .arg(&args.expected_capture_sha256)
        .arg("--expected-candidate-sha256")
        .arg(&args.expected_candidate_sha256)
        .arg("--output")
        .arg(&args.output)
        .arg("--stage")
        .arg(args.stage.to_possible_value().unwrap().get_name())
        .arg("--seconds")
        .arg(args.seconds.to_string())
        .arg("--worker");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut process = run_bounded(&mut command, &limits)?;
    let result: Option<Value> = process
        .stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str(line).ok());
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.as_ref().is_some_and(|v| {
            v["phase"] == "complete"
                && v["exact_signed_scalar_reconstruction"] == true
                && (args.stage != Stage::SquareFree
                    || (v["exact_canonical_factor_product"] == true
                        && v["exact_signed_raw_factor_product"] == true))
        });
    if result.is_some() {
        process.stdout.clear();
    }
    let report = json!({"schema_version":1,"limits":limits,"cpu":args.cpu,
        "binary_sha256":hash(&fs::read(executable)?),"process":process,"result":result});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?;
    file.write_all(&serde_json::to_vec_pretty(&report)?)?;
    ensure!(
        success,
        "bounded content stage incomplete; report and progress preserved"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn names() -> Vec<String> {
        ["s", "t", "unused"].map(String::from).to_vec()
    }
    #[test]
    fn signed_rational_content_retains_repeated_coordinate_factors() {
        let (a, p) = parse(&names(), "(-6/5)*s^3+(12/5)*s^4+(-6/5)*s^5").unwrap();
        let (q, c) = content(&p).unwrap();
        assert_eq!(q, a.parse("s^3*(s-1)^2").unwrap());
        assert_eq!(c, rational("-6/5").unwrap());
        reconstruction(&p, &q, &c).unwrap();
        assert_eq!(summary(&q).unwrap()["minimum_exponent"], 3);
        assert!(reconstruction(&p, &q, &(-c)).is_err());
    }
    #[test]
    fn full_map_and_constant_content_are_preserved() {
        let (a, p) = parse(&names(), "(-9/7)*s^0").unwrap();
        let (q, c) = content(&p).unwrap();
        assert_eq!(q, a.parse("1").unwrap());
        assert_eq!(q.nvars(), 3);
        reconstruction(&p, &q, &c).unwrap();
        assert!(
            !summary(&q).unwrap()["fits_current_source_caps_only"]
                .as_bool()
                .unwrap()
        );
        assert!(content(&a.parse("0").unwrap()).is_err());
        let other = Algebra::new(&["s".into()]).unwrap().parse("1").unwrap();
        assert!(reconstruction(&p, &other, &c).is_err());
    }
    #[test]
    fn square_free_product_retains_signed_scale_and_all_multiplicities() {
        let a = Algebra::new(&names()).unwrap();
        let raw = a.parse("-18/7*s^5*(s-4)^8*(s^2+1)^2*(s^2-2)").unwrap();
        let (source, scale) = content(&raw).unwrap();
        let mut factors = source.square_free_factorization();
        reconstruct_factors(&raw, &source, &scale, &mut factors).unwrap();
        assert!(
            factors
                .iter()
                .any(|(p, m)| *m == 5 && *p == a.parse("s").unwrap())
        );
        assert!(
            factors
                .iter()
                .any(|(p, m)| *m == 8 && *p == a.parse("s-4").unwrap())
        );
        let mut wrong = factors.clone();
        let (p, _) = wrong.iter_mut().find(|(p, _)| !p.is_constant()).unwrap();
        *p = &*p + &p.one();
        assert!(reconstruct_factors(&raw, &source, &scale, &mut wrong).is_err());
        assert!(reconstruct_factors(&raw, &source, &(-scale.clone()), &mut factors).is_err());
        let mut missing = factors.clone();
        missing.pop();
        assert!(reconstruct_factors(&raw, &source, &scale, &mut missing).is_err());
    }
    #[test]
    fn native_output_caps_precede_reconstruction_powers() {
        let a = Algebra::new(&names()).unwrap();
        let source = a.parse("s^4096+1").unwrap();
        assert!(
            product_preflight(&source, &mut [(a.parse("1048576*s+1").unwrap(), 4096)]).is_err()
        );
        assert!(product_preflight(&source, &mut [(a.parse("s").unwrap(), 4097)]).is_err());
        assert!(product_preflight(&source, &mut [(a.parse("s").unwrap(), 0)]).is_err());
        assert!(
            product_preflight(
                &source,
                &mut vec![(a.parse("s").unwrap(), 1); MAX_BLOCKS + 1]
            )
            .is_err()
        );
        assert!(product_preflight(&source, &mut [(a.parse("t^4096").unwrap(), 1)]).is_err());
        let other = Algebra::new(&["s".into()])
            .unwrap()
            .parse("s^4096+1")
            .unwrap();
        assert!(product_preflight(&source, &mut [(other, 1)]).is_err());
        assert!(product_preflight(&source, &mut [(a.parse("s^4096/2+1").unwrap(), 1)]).is_err());
        assert!(
            product_preflight(&a.parse("1").unwrap(), &mut [(a.parse("1").unwrap(), 1)]).is_err()
        );
    }
    #[test]
    fn parser_rejects_expansion_and_caps_before_algebra() {
        for bad in [
            "(2^999)*s^1",
            "(1)*s^999999999",
            "(1)*t^1",
            "(1)*s^1+(1)*s^1",
            "(1)*s^2+(1)*s^1",
            "(0)*s^0",
            "(1/0)*s^1",
            "(1/-2)*s^1",
            "(+1)*s^1",
            "(1)*s^1+",
            "(01)*s^1",
            "(1/2/3)*s^1",
        ] {
            assert!(preflight(bad).is_err(), "{bad}");
        }
        assert!(preflight(&format!("({})*s^0", "9".repeat(MAX_DIGITS + 1))).is_err());
        let excessive = (0..=MAX_TERMS)
            .map(|i| format!("(1)*s^{i}"))
            .collect::<Vec<_>>()
            .join("+");
        assert!(preflight(&excessive).is_err());
        let oversized = "x".repeat(MAX_BYTES + 1);
        assert!(preflight(&oversized).is_err());
        assert!(checked_map(&["t".into(), "s".into()]).is_err());
        assert!(checked_map(&["s".into(), "s".into()]).is_err());
    }
    #[test]
    fn total_coefficient_and_denominator_envelopes_are_preflighted() {
        let big = "9".repeat(MAX_DIGITS);
        let terms = (0..3200)
            .map(|i| format!("({big})*s^{i}"))
            .collect::<Vec<_>>()
            .join("+");
        assert!(terms.len() < MAX_BYTES && preflight(&terms).is_err());
        let denominator = "9".repeat(10_000);
        assert!(preflight(&format!("(1/{denominator})*s^0+(1/{denominator})*s^1")).is_err());
    }
    #[test]
    fn candidate_and_operand_tampering_are_rejected() {
        // Minimal real capture schema taken through serde to exercise all bindings.
        let operands = vec!["t^2-s"];
        let input = hash(&serde_json::to_vec(&operands).unwrap());
        let capture = json!({"schema_version":1,"key":"test","input_hash":input,
            "backend_version":"test","coefficient_domain":"Q","operands_omitted_for_size":false,
            "operation":"discriminant","variable":1,"variable_names":names(),
            "operands":operands,"complete":false});
        let expression = "(4)*s^1";
        let candidate = json!({"operation":"discriminant","input_hash":input,"result":expression,
            "result_sha256":hash(expression.as_bytes()),"result_terms":1});
        let cb = serde_json::to_vec(&capture).unwrap();
        let pb = serde_json::to_vec(&candidate).unwrap();
        // The current capture schema may require additional provenance fields;
        // construct those explicitly below rather than ignoring deserialize errors.
        let valid = authenticate(&cb, &pb, &hash(&cb), &hash(&pb));
        assert!(valid.is_ok(), "{:?}", valid.err());
        let mut wrong = candidate.clone();
        wrong["result"] = json!("(-4)*s^1");
        let wb = serde_json::to_vec(&wrong).unwrap();
        assert!(authenticate(&cb, &wb, &hash(&cb), &hash(&wb)).is_err());
        let mut wrong = capture.clone();
        wrong["operands"] = json!(["t^2+s"]);
        let wb = serde_json::to_vec(&wrong).unwrap();
        assert!(authenticate(&wb, &pb, &hash(&wb), &hash(&pb)).is_err());
        assert!(authenticate(&cb, &pb, &hash(&cb), &"0".repeat(64)).is_err());
    }
}
