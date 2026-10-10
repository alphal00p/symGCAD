//! Bounded research-only modular rejection of proposed known univariate factors.
//! Zero remainders are NOT rational divisibility certificates or CAD evidence.
use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{
    domains::{
        InternalOrdering,
        finite_field::{PrimeIteratorU64, ToFiniteField, Zp},
    },
    prelude::*,
};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_FACTOR_BYTES: usize = 2 * 1024 * 1024;
const MAX_FACTORS: usize = 256;
const MAX_DEGREE: u32 = 4096;
const MAX_TERMS: usize = 4096;
const MAX_HEIGHT: u64 = 45_000;
const MAX_BITS: u64 = 140_000_000;
const MAX_FACTOR_BITS: u64 = 16_000_000;
const MAX_WORK: u64 = 100_000_000;
const LINEAR_SURVIVOR: &str = "f6eadec7cccbb756b7dc520bc6213aa344fb5edf0351bde9642c20c5c17f9e32";

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Selection {
    All,
    D24FourAndLinear,
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    candidate: PathBuf,
    inventory: PathBuf,
    sidecar: PathBuf,
    #[arg(long)]
    expected_capture_sha256: String,
    #[arg(long)]
    expected_candidate_sha256: String,
    #[arg(long)]
    expected_inventory_sha256: String,
    #[arg(long)]
    expected_sidecar_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 10)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, value_enum, default_value = "all")]
    selection: Selection,
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
#[derive(Deserialize)]
struct Inventory {
    schema_version: u32,
    factors: Vec<Proposal>,
}
#[derive(Deserialize)]
struct Proposal {
    expression: String,
    expression_sha256: String,
    degree: u32,
    terms: usize,
}
#[derive(Deserialize)]
struct Sidecar {
    schema_version: u32,
    candidate_sha256: String,
    variable: String,
    exact_signed_original_reconstruction: bool,
    factors: Vec<SidecarFactor>,
}
#[derive(Deserialize)]
struct SidecarFactor {
    multiplicity: usize,
    expression: String,
    expression_sha256: String,
    polynomial: PolynomialSummary,
}
#[derive(Deserialize)]
struct PolynomialSummary {
    degree: u32,
    terms: usize,
    sha256: String,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, expected: &str) -> Result<Vec<u8>> {
    ensure!(
        expected.len() == 64 && expected.bytes().all(|b| b.is_ascii_hexdigit()),
        "expected SHA256 required"
    );
    let mut f = fs::File::open(path)?;
    let meta = f.metadata()?;
    ensure!(
        meta.is_file() && meta.len() <= MAX_BYTES as u64,
        "regular bounded input required"
    );
    let mut bytes = Vec::new();
    (&mut f)
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_BYTES && hash(&bytes) == expected,
        "input growth/digest mismatch"
    );
    Ok(bytes)
}
fn checked_map(names: &[String]) -> Result<()> {
    ensure!(
        !names.is_empty() && names.len() <= 64 && names[0] == "s",
        "full map/first axis"
    );
    let mut seen = std::collections::HashSet::new();
    for n in names {
        ensure!(
            !n.is_empty()
                && n.len() <= 128
                && n.as_bytes()[0].is_ascii_alphabetic()
                && n.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                && seen.insert(n),
            "bounded unique aliases"
        );
    }
    Ok(())
}
fn integer_bits(text: &str, signed: bool) -> Result<u64> {
    let t = if signed {
        text.strip_prefix('-').unwrap_or(text)
    } else {
        text
    };
    ensure!(
        !t.is_empty()
            && t.len() <= 13_546
            && t.bytes().all(|b| b.is_ascii_digit())
            && t.as_bytes()[0] != b'0',
        "canonical nonzero integer literal cap"
    );
    let bits = (t.len() as u64 * 3322).div_ceil(1000);
    ensure!(bits <= MAX_HEIGHT, "coefficient height cap");
    Ok(bits)
}
// Both native canonical '(c)*s' spelling and explicit '(c)*s^0' sidecars.
// Reject general parser syntax before registering symbols or constructing numbers.
fn preflight(expression: &str) -> Result<Vec<(&str, u32)>> {
    ensure!(
        !expression.is_empty() && expression.len() <= MAX_BYTES,
        "expression bytes"
    );
    let mut terms = Vec::new();
    let mut last = None;
    let mut bits = 0u64;
    let mut denominator_bits = 0u64;
    for t in expression.split('+') {
        ensure!(terms.len() < MAX_TERMS, "term count cap");
        let t = t.strip_prefix('(').context("coefficient prefix")?;
        let (c, tail) = t.split_once(')').context("coefficient suffix")?;
        let (n, d) = c.split_once('/').unwrap_or((c, "1"));
        let nb = integer_bits(n, true)?;
        let db = integer_bits(d, false)?;
        bits = bits
            .checked_add(nb + db)
            .context("coefficient storage overflow")?;
        denominator_bits += db;
        ensure!(
            bits <= MAX_BITS && denominator_bits <= 65_536,
            "coefficient storage/denominator cap"
        );
        let e = if tail.is_empty() {
            0
        } else if tail == "*s" {
            1
        } else {
            let e = tail
                .strip_prefix("*s^")
                .context("univariate sparse grammar")?;
            ensure!(
                !e.is_empty() && e.len() <= 4 && e.bytes().all(|b| b.is_ascii_digit()),
                "exponent literal"
            );
            e.parse::<u32>()?
        };
        ensure!(
            e <= MAX_DEGREE && last.is_none_or(|x| e > x),
            "degree/strict sparse ordering"
        );
        terms.push((c, e));
        last = Some(e);
    }
    Ok(terms)
}
fn parse(algebra: &Algebra, expression: &str) -> Result<Poly> {
    let terms = preflight(expression)?;
    let mut p = algebra.parse("0")?;
    let mut powers = vec![0; algebra.variables.len()];
    for (c, e) in terms {
        let c = rational(c)?;
        ensure!(
            !c.is_zero()
                && c.numerator_ref().significant_bits() <= MAX_HEIGHT
                && c.denominator_ref().significant_bits() <= MAX_HEIGHT,
            "exact coefficient caps"
        );
        powers[0] = e;
        p.append_monomial(c, &powers);
    }
    ensure!(
        !p.is_zero() && p.variables() == &algebra.variables,
        "nonzero/full map"
    );
    Ok(p)
}
fn authenticated_candidate(
    capture: &[u8],
    candidate: &[u8],
) -> Result<(OperationCapture, Candidate)> {
    let capture: OperationCapture = serde_json::from_slice(capture)?;
    let candidate: Candidate = serde_json::from_slice(candidate)?;
    checked_map(&capture.variable_names)?;
    ensure!(
        capture.operation == AlgebraOperation::Discriminant
            && capture
                .variable
                .is_some_and(|x| x > 0 && x < capture.variable_names.len()),
        "capture discriminant axis"
    );
    let operands = capture
        .operands
        .as_ref()
        .context("captured operand omitted")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == capture.input_hash,
        "captured operand binding"
    );
    ensure!(
        candidate.operation == "discriminant"
            && candidate.input_hash == capture.input_hash
            && hash(candidate.result.as_bytes()) == candidate.result_sha256,
        "candidate signed expression binding"
    );
    preflight(&candidate.result)?;
    Ok((capture, candidate))
}
struct Factor {
    polynomial: Poly,
    expression_sha256: String,
    origins: Vec<Value>,
}
fn factors(a: &Algebra, inventory: &[u8], sidecar: &[u8]) -> Result<Vec<Factor>> {
    let inv: Inventory = serde_json::from_slice(inventory)?;
    let side: Sidecar = serde_json::from_slice(sidecar)?;
    ensure!(
        inv.schema_version == 1
            && inv.factors.len() <= MAX_FACTORS
            && side.schema_version == 1
            && side.factors.len() <= 64
            && side.variable == "s"
            && side.exact_signed_original_reconstruction
            && side.candidate_sha256.len() == 64,
        "proposal schema/size/source binding"
    );
    let mut prepared = Vec::new();
    let mut bytes = 0usize;
    for (i, r) in inv.factors.iter().enumerate() {
        ensure!(
            hash(r.expression.as_bytes()) == r.expression_sha256 && r.degree > 0,
            "inventory expression binding"
        );
        preflight(&r.expression)?;
        bytes += r.expression.len();
        prepared.push((
            &r.expression,
            r.expression_sha256.clone(),
            r.degree,
            r.terms,
            None,
            json!({"kind":"cache_inventory","index":i}),
        ));
    }
    for (i, r) in side.factors.iter().enumerate() {
        ensure!(
            r.multiplicity > 0
                && r.multiplicity <= 4096
                && hash(r.expression.as_bytes()) == r.expression_sha256,
            "sidecar factor binding"
        );
        preflight(&r.expression)?;
        if r.polynomial.degree == 0 {
            continue;
        }
        bytes += r.expression.len();
        prepared.push((&r.expression,r.expression_sha256.clone(),r.polynomial.degree,r.polynomial.terms,Some(&r.polynomial.sha256),json!({"kind":"checked_D24_sidecar","index":i,"source_candidate_sha256":side.candidate_sha256,"historical_multiplicity":r.multiplicity})));
    }
    ensure!(
        bytes <= MAX_FACTOR_BYTES && prepared.len() <= MAX_FACTORS,
        "aggregate factor text/count cap"
    );
    let mut unique: HashMap<Poly, Factor> = HashMap::new();
    let mut bits = 0u64;
    for (text, sha, degree, terms, formatted_hash, origin) in prepared {
        let p = parse(a, text)?;
        ensure!(
            p.degree(0) == degree && p.nterms() == terms,
            "factor shape mismatch"
        );
        if let Some(h) = formatted_hash {
            ensure!(
                hash(a.format(&p).as_bytes()) == *h,
                "sidecar native polynomial hash"
            );
        }
        if let Some(old) = unique.get_mut(&p) {
            old.origins.push(origin);
            continue;
        }
        bits += p
            .coefficients
            .iter()
            .map(|c| c.numerator_ref().significant_bits() + c.denominator_ref().significant_bits())
            .sum::<u64>();
        ensure!(bits <= MAX_FACTOR_BITS, "retained factor coefficient cap");
        unique.insert(
            p.clone(),
            Factor {
                polynomial: p,
                expression_sha256: sha,
                origins: vec![origin],
            },
        );
    }
    let mut result: Vec<_> = unique.into_values().collect();
    result.sort_by(|a, b| a.polynomial.internal_cmp(&b.polynomial));
    Ok(result)
}
fn selected(factor: &Factor, selection: Selection) -> bool {
    selection == Selection::All
        || factor.expression_sha256 == LINEAR_SURVIVOR
        || factor.origins.iter().any(|o| {
            o["kind"] == "checked_D24_sidecar"
                && o["index"]
                    .as_u64()
                    .is_some_and(|i| [0, 1, 3, 4].contains(&i))
        })
}
fn image(
    p: &Poly,
    field: &Zp,
) -> Option<symbolica::poly::polynomial::MultivariatePolynomial<Zp, u32>> {
    if p.coefficients
        .iter()
        .any(|c| field.is_zero(&c.denominator_ref().to_finite_field(field)))
    {
        return None;
    }
    let mapped = p.map_coeff(|c| c.to_finite_field(field), field.clone());
    if mapped.is_zero() || mapped.degree(0) != p.degree(0) {
        return None;
    }
    Some(mapped)
}
#[cfg(test)]
fn screen_pair(raw: &Poly, factor: &Poly, prime: u32) -> Result<Value> {
    ensure!(
        raw.variables() == factor.variables()
            && !factor.is_constant()
            && (1..raw.nvars()).all(|i| raw.degree(i) == 0 && factor.degree(i) == 0),
        "univariate/full-map pair"
    );
    let field = Zp::new(prime);
    let Some(p) = image(raw, &field) else {
        return Ok(json!({"prime":prime,"status":"inconclusive_candidate_bad_prime"}));
    };
    let Some(q) = image(factor, &field) else {
        return Ok(json!({"prime":prime,"status":"inconclusive_factor_bad_prime"}));
    };
    let remainder = p.quot_rem(&q, false).1;
    Ok(
        json!({"prime":prime,"status":if remainder.is_zero(){"modular_survivor"}else{"not_divisible"},"remainder_degree":if remainder.is_zero(){None}else{Some(remainder.degree(0))},"remainder_sha256":hash(format!("{remainder}").as_bytes())}),
    )
}
fn progress(path: &Path, value: &Value) -> Result<()> {
    let tmp = path.with_extension("progress.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path.with_extension("progress.json"))?;
    Ok(())
}
fn worker(args: &Args) -> Result<()> {
    let started = Instant::now();
    let cb = read(&args.capture, &args.expected_capture_sha256)?;
    let db = read(&args.candidate, &args.expected_candidate_sha256)?;
    let ib = read(&args.inventory, &args.expected_inventory_sha256)?;
    let sb = read(&args.sidecar, &args.expected_sidecar_sha256)?;
    let (capture, candidate) = authenticated_candidate(&cb, &db)?;
    let a = Algebra::new(&capture.variable_names)?;
    let raw = parse(&a, &candidate.result)?;
    ensure!(
        raw.nterms() == candidate.result_terms,
        "candidate term count"
    );
    let mut fs = factors(&a, &ib, &sb)?;
    let inventory_factor_count = fs.len();
    fs.retain(|f| selected(f, args.selection));
    ensure!(!fs.is_empty(), "empty selected factor inventory");
    if args.selection == Selection::D24FourAndLinear {
        ensure!(fs.len() <= 5, "narrow selection count cap");
    }
    let work = fs.iter().try_fold(0u64, |sum, f| {
        sum.checked_add(
            (u64::from(raw.degree(0)) + 1) * (u64::from(f.polynomial.degree(0)) + 1) * 3,
        )
        .context("work overflow")
    })?;
    ensure!(work <= MAX_WORK, "modular degree-product budget");
    let primes: Vec<u32> = PrimeIteratorU64::new(1 << 30)
        .take(3)
        .map(u32::try_from)
        .collect::<std::result::Result<_, _>>()?;
    let mut report = json!({"schema_version":1,"phase":"modular_screen","candidate_only":true,"discriminant_verified_by_this_helper":false,"exact_divisibility_proved":false,"original_coverage":false,"capture_sha256":hash(&cb),"candidate_sha256":hash(&db),"inventory_sha256":hash(&ib),"sidecar_sha256":hash(&sb),"input_hash":capture.input_hash,"full_variable_map":capture.variable_names,"candidate_degree":raw.degree(0),"candidate_terms":raw.nterms(),"factor_count":fs.len(),"inventory_factor_count":inventory_factor_count,"selection":format!("{:?}",args.selection),"remainder_strategy":"native_sparse_quot_rem","primes":primes,"degree_product_budget":work,"preparation_seconds":started.elapsed().as_secs_f64(),"checked":0,"factors":[]});
    progress(&args.output, &report)?;
    let step = Instant::now();
    let mut rows = Vec::new();
    // Cache only the three freshly reduced candidate images; no algebra operation cache.
    let fields: Vec<_> = primes.iter().map(|p| Zp::new(*p)).collect();
    let images: Vec<_> = fields.iter().map(|f| image(&raw, f)).collect();
    for (findex, f) in fs.iter().enumerate() {
        let factor_started = Instant::now();
        let mut trials = Vec::new();
        for ((prime, field), p) in primes.iter().zip(&fields).zip(&images) {
            let result = if let Some(p) = p {
                if let Some(q) = image(&f.polynomial, field) {
                    let r = p.quot_rem(&q, false).1;
                    json!({"prime":prime,"status":if r.is_zero(){"modular_survivor"}else{"not_divisible"},"remainder_degree":if r.is_zero(){None}else{Some(r.degree(0))},"remainder_sha256":hash(format!("{r}").as_bytes())})
                } else {
                    json!({"prime":prime,"status":"inconclusive_factor_bad_prime"})
                }
            } else {
                json!({"prime":prime,"status":"inconclusive_candidate_bad_prime"})
            };
            trials.push(result);
        }
        let rejected = trials.iter().any(|r| r["status"] == "not_divisible");
        let passed = trials
            .iter()
            .filter(|r| r["status"] == "modular_survivor")
            .count();
        rows.push(json!({"expression_sha256":f.expression_sha256,"native_polynomial_sha256":hash(a.format(&f.polynomial).as_bytes()),"degree":f.polynomial.degree(0),"terms":f.polynomial.nterms(),"origins":f.origins,"status":if rejected{"not_divisible"}else if passed==3{"survived_three_primes_requires_exact_division"}else{"inconclusive_bad_primes"},"valid_zero_images":passed,"trials":trials,"seconds":factor_started.elapsed().as_secs_f64()}));
        report["checked"] = json!(findex + 1);
        report["factors"] = json!(rows);
        progress(&args.output, &report)?;
    }
    report["modular_seconds"] = json!(step.elapsed().as_secs_f64());
    report["total_worker_seconds"] = json!(started.elapsed().as_secs_f64());
    report["phase"] = json!("complete");
    report["counts"] = json!({"not_divisible":rows.iter().filter(|r|r["status"]=="not_divisible").count(),"survivors":rows.iter().filter(|r|r["status"]=="survived_three_primes_requires_exact_division").count(),"inconclusive":rows.iter().filter(|r|r["status"]=="inconclusive_bad_primes").count()});
    report["caveat"] = json!(
        "Zero modular remainders are only screening candidates. No exact rational division, factorization, SQF, root isolation, operation injection or CAD coverage. All factor multiplicities and interior boundaries must be retained in any later exact decomposition."
    );
    progress(&args.output, &report)?;
    println!("{report}");
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "worker deadline (0,120]"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists()
            && !args.output.with_extension("progress.json").exists()
            && !args.output.with_extension("progress.tmp").exists(),
        "fresh output and sidecars"
    );
    if let Some(p) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p)?;
    }
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new("taskset");
    cmd.arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .args([
            &args.capture,
            &args.candidate,
            &args.inventory,
            &args.sidecar,
        ])
        .arg("--expected-capture-sha256")
        .arg(&args.expected_capture_sha256)
        .arg("--expected-candidate-sha256")
        .arg(&args.expected_candidate_sha256)
        .arg("--expected-inventory-sha256")
        .arg(&args.expected_inventory_sha256)
        .arg("--expected-sidecar-sha256")
        .arg(&args.expected_sidecar_sha256)
        .arg("--output")
        .arg(&args.output)
        .arg("--seconds")
        .arg(args.seconds.to_string())
        .arg("--selection")
        .arg(args.selection.to_possible_value().unwrap().get_name())
        .arg("--worker");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut process = run_bounded(&mut cmd, &limits)?;
    let result: Option<Value> = process
        .stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| serde_json::from_str(l).ok());
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.as_ref().is_some_and(|r| r["phase"] == "complete");
    if result.is_some() {
        process.stdout.clear()
    }
    let report = json!({"schema_version":1,"limits":limits,"cpu":args.cpu,"binary_sha256":hash(&fs::read(exe)?),"process":process,"result":result});
    let mut f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args.output)?;
    f.write_all(&serde_json::to_vec_pretty(&report)?)?;
    ensure!(
        success,
        "bounded modular screen incomplete; report/progress retained"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn algebra() -> Algebra {
        Algebra::new(&["s".into(), "t".into(), "unused".into()]).unwrap()
    }
    #[test]
    fn native_sparse_and_dense_remainders_agree_exactly() {
        let a = algebra();
        let field = Zp::new(101);
        for p in ["s^16+2*s^3+7", "-7/3*s^4*(s-4)^2*(s^2+1)"] {
            let p = image(&a.parse(p).unwrap(), &field).unwrap();
            for q in ["s", "s-4", "3*s+1", "s^2+1", "s^20+1"] {
                let q = image(&a.parse(q).unwrap(), &field).unwrap();
                let sparse = p.quot_rem(&q, false).1;
                let dense = p
                    .to_univariate_from_univariate(0)
                    .rem(&q.to_univariate_from_univariate(0));
                assert_eq!(sparse.to_univariate_from_univariate(0), dense);
            }
        }
    }
    #[test]
    fn narrow_selection_preserves_intact_inventory_and_explicit_controls() {
        let a = algebra();
        for i in 0..11 {
            let f = Factor {
                polynomial: a.parse("s-4").unwrap(),
                expression_sha256: "not-the-survivor".into(),
                origins: vec![json!({"kind":"checked_D24_sidecar","index":i})],
            };
            assert_eq!(
                selected(&f, Selection::D24FourAndLinear),
                [0, 1, 3, 4].contains(&i)
            );
            assert!(selected(&f, Selection::All));
        }
        let f = Factor {
            polynomial: a.parse("9*s-10").unwrap(),
            expression_sha256: LINEAR_SURVIVOR.into(),
            origins: vec![],
        };
        assert!(selected(&f, Selection::D24FourAndLinear));
    }
    #[test]
    fn exact_factor_survives_signed_rational_scale_and_repeats() {
        let a = algebra();
        let p = a.parse("-7/3*s^3*(s-4)^2*(s^2+1)").unwrap();
        for s in ["s", "s-4", "s^2+1"] {
            let r = screen_pair(&p, &a.parse(s).unwrap(), 101).unwrap();
            assert_eq!(r["status"], "modular_survivor");
        }
    }
    #[test]
    fn modular_rejection_and_false_positive_are_distinct() {
        let a = algebra();
        let p = a.parse("s+102").unwrap();
        let q = a.parse("s+1").unwrap();
        assert_eq!(
            screen_pair(&p, &q, 101).unwrap()["status"],
            "modular_survivor"
        );
        assert_eq!(screen_pair(&p, &q, 103).unwrap()["status"], "not_divisible");
    }
    #[test]
    fn denominator_and_degree_drop_primes_are_inconclusive() {
        let a = algebra();
        let q = a.parse("s+1").unwrap();
        for p in ["101*s+1", "(s+1)/101"] {
            assert!(
                screen_pair(&a.parse(p).unwrap(), &q, 101).unwrap()["status"]
                    .as_str()
                    .unwrap()
                    .starts_with("inconclusive")
            );
        }
        assert_eq!(
            screen_pair(&q, &a.parse("101*s+1").unwrap(), 101).unwrap()["status"],
            "inconclusive_factor_bad_prime"
        );
    }
    #[test]
    fn bounded_grammar_and_full_map() {
        let a = algebra();
        let p = parse(&a, "(-2/3)+(1)*s+(4)*s^3").unwrap();
        assert_eq!(p.variables(), &a.variables);
        assert_eq!(p, a.parse("-2/3+s+4*s^3").unwrap());
        for s in [
            "(1)*t^1",
            "(1)*s^4097",
            "(1)*s^2+(2)*s^1",
            "(0)*s^0",
            "(1)*s^(2)",
            "(1/0)*s^0",
        ] {
            assert!(preflight(s).is_err(), "{s}");
        }
        assert!(preflight(&format!("({})*s^0", "9".repeat(13547))).is_err());
        let b = Algebra::new(&["s".into()]).unwrap();
        assert!(screen_pair(&p, &b.parse("s+1").unwrap(), 101).is_err());
        assert!(screen_pair(&a.parse("s+t").unwrap(), &a.parse("s+1").unwrap(), 101).is_err());
    }
    #[test]
    fn inventory_and_sidecar_tampering_rejected() {
        let a = algebra();
        let e = "(-4)+(1)*s";
        let inv = json!({"schema_version":1,"factors":[{"expression":e,"expression_sha256":hash(e.as_bytes()),"degree":1,"terms":2}]});
        let se = "(-4)*s^0+(1)*s^1";
        let side = json!({"schema_version":1,"candidate_sha256":"0".repeat(64),"variable":"s","exact_signed_original_reconstruction":true,"factors":[{"multiplicity":2,"expression":se,"expression_sha256":hash(se.as_bytes()),"polynomial":{"degree":1,"terms":2,"sha256":hash(a.format(&a.parse("s-4").unwrap()).as_bytes())}}]});
        let ok = factors(
            &a,
            &serde_json::to_vec(&inv).unwrap(),
            &serde_json::to_vec(&side).unwrap(),
        )
        .unwrap();
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].origins.len(), 2);
        let mut bad = inv.clone();
        bad["factors"][0]["degree"] = json!(2);
        assert!(
            factors(
                &a,
                &serde_json::to_vec(&bad).unwrap(),
                &serde_json::to_vec(&side).unwrap()
            )
            .is_err()
        );
        bad["factors"][0]["expression"] = json!("(-3)+(1)*s");
        assert!(
            factors(
                &a,
                &serde_json::to_vec(&bad).unwrap(),
                &serde_json::to_vec(&side).unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn candidate_signed_binding_tamper_rejected() {
        let operands = vec!["s+t^2".to_string()];
        let cap = json!({"schema_version":1,"backend_version":"test","coefficient_domain":"Q","key":"test","operands_omitted_for_size":false,"elapsed_seconds":null,"output_hash":null,"operation":"discriminant","variable":1,"variable_names":["s","t"],"input_hash":hash(&serde_json::to_vec(&operands).unwrap()),"operands":operands,"complete":false});
        let expr = "(-4)*s^1";
        let cand = json!({"operation":"discriminant","input_hash":cap["input_hash"],"result":expr,"result_sha256":hash(expr.as_bytes()),"result_terms":1});
        assert!(
            authenticated_candidate(
                &serde_json::to_vec(&cap).unwrap(),
                &serde_json::to_vec(&cand).unwrap()
            )
            .is_ok()
        );
        let mut bad = cand;
        bad["result"] = json!("(4)*s^1");
        assert!(
            authenticated_candidate(
                &serde_json::to_vec(&cap).unwrap(),
                &serde_json::to_vec(&bad).unwrap()
            )
            .is_err()
        );
    }
}
