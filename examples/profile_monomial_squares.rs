//! Research-only monomial-square proposals with independent strict residual replay.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    domain::{
        CertificateOptions, Domain, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

const MAX_TERMS: usize = 128;
const MAX_CANDIDATES: usize = 32;
#[derive(Parser)]
struct Args {
    scan: PathBuf,
    #[arg(long)]
    scan_sha256: String,
    #[arg(long)]
    input_audit: PathBuf,
    #[arg(long)]
    audit_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 12)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Proof {
    sign: i8,
    beta: Vec<u32>,
    gamma: Vec<u32>,
    weight: String,
    ratio: String,
    residual: String,
    residual_certificate: DomainCertificate,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(path: &Path, cap: usize) -> Result<Vec<u8>> {
    ensure!(path.is_file(), "regular input required");
    let mut b = Vec::new();
    fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut b)?;
    ensure!(b.len() <= cap, "input byte cap");
    Ok(b)
}
fn height(r: &Rational) -> u64 {
    r.numerator_ref()
        .significant_bits()
        .saturating_add(r.denominator_ref().significant_bits())
}
fn bounded(p: &Poly) -> bool {
    p.nterms() <= MAX_TERMS + 3
        && p.nvars() <= 16
        && (0..p.nvars()).all(|v| p.degree(v) <= 32)
        && p.coefficients.iter().all(|c| height(c) <= 4096)
}
// The audited factor outputs are primitive integer sparse expressions. Reject
// arbitrary expression syntax, powers and duplicate monomials before conversion.
fn parse_sparse(a: &Algebra, text: &str) -> Result<Poly> {
    parse_sparse_limits(a, text, MAX_TERMS, 512)
}
fn parse_sparse_limits(a: &Algebra, text: &str, max_terms: usize, max_height: u64) -> Result<Poly> {
    ensure!(text.len() <= 128 * 1024, "polynomial bytes");
    let mut p = a.parse("0")?;
    if text == "0" {
        return Ok(p);
    }
    let mut seen = HashSet::new();
    for (i, term) in text.split('+').enumerate() {
        ensure!(i < max_terms, "polynomial term cap");
        let (c, tail) = term
            .strip_prefix('(')
            .context("coefficient prefix")?
            .split_once(')')
            .context("coefficient suffix")?;
        let unsigned = c.strip_prefix('-').unwrap_or(c);
        let (num, den) = unsigned.split_once('/').unwrap_or((unsigned, "1"));
        let max_digits = (max_height * 302 / 1000 + 2) as usize;
        ensure!(
            [num, den].iter().all(|digits| !digits.is_empty()
                && digits.len() <= max_digits
                && digits.bytes().all(|b| b.is_ascii_digit())),
            "rational coefficient grammar"
        );
        let c = rational(c)?;
        ensure!(
            !c.is_zero() && height(&c) <= max_height,
            "coefficient height"
        );
        let mut e = vec![0; a.names.len()];
        if !tail.is_empty() {
            for factor in tail
                .strip_prefix('*')
                .context("monomial prefix")?
                .split('*')
            {
                let (name, power) = factor.split_once('^').unwrap_or((factor, "1"));
                let v = a
                    .names
                    .iter()
                    .position(|n| n == name)
                    .context("unknown variable")?;
                ensure!(
                    !power.is_empty()
                        && power.len() <= 2
                        && power.bytes().all(|b| b.is_ascii_digit()),
                    "literal power"
                );
                let power: u32 = power.parse()?;
                ensure!(
                    (1..=32).contains(&power) && e[v] == 0,
                    "power/repetition cap"
                );
                e[v] = power;
            }
        }
        ensure!(seen.insert(e.clone()), "duplicate monomial");
        p.append_monomial(c, &e);
    }
    Ok(p)
}
fn parse_constraint(a: &Algebra, text: &str) -> Result<Poly> {
    ensure!(
        !text.is_empty()
            && text.len() <= 128
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_+-() ".contains(&b)),
        "only bounded affine constraint syntax"
    );
    ensure!(
        text.split(|c: char| !c.is_ascii_digit())
            .all(|s| s.len() <= 16),
        "constraint integer height"
    );
    let p = a.parse(text)?;
    ensure!(
        p.nterms() <= 2
            && (0..p.nvars()).filter(|&v| p.degree(v) > 0).count() <= 1
            && (0..p.nvars()).all(|v| p.degree(v) <= 1),
        "coordinate-affine constraint required"
    );
    ensure!(
        !p.is_constant() || p.get_constant() > Rational::zero(),
        "constant constraint must be positive"
    );
    Ok(p)
}
fn monomial(p: &Poly, e: &[u32]) -> Poly {
    let mut m = p.zero();
    m.append_monomial(Rational::one(), e);
    m
}
fn square(
    p: &Poly,
    beta: &[u32],
    gamma: &[u32],
    weight: &Rational,
    ratio: &Rational,
) -> Result<Poly> {
    ensure!(
        beta.len() == p.nvars() && gamma.len() == p.nvars() && beta != gamma,
        "square exponent map"
    );
    ensure!(
        beta.iter().chain(gamma).all(|&x| x <= 16),
        "square degree cap"
    );
    ensure!(
        *weight > Rational::zero()
            && *ratio > Rational::zero()
            && height(weight) <= 2048
            && height(ratio) <= 2048
            && height(weight)
                .saturating_add(2 * height(ratio))
                .saturating_add(4)
                <= 4096,
        "positive bounded square weights"
    );
    let g = monomial(p, beta) - monomial(p, gamma).mul_coeff(ratio.clone());
    let result = g.pow(2).mul_coeff(weight.clone());
    ensure!(bounded(&result), "square product height/term cap");
    Ok(result)
}
fn replay(a: &Algebra, p: &Poly, constraints: &[Poly], proof: &Proof) -> Result<()> {
    ensure!(
        bounded(p) && (proof.sign == 1 || proof.sign == -1),
        "invalid target/sign"
    );
    ensure!(
        proof.weight.len() <= 1300
            && proof.ratio.len() <= 1300
            && proof.residual.len() <= 256 * 1024,
        "proof text caps"
    );
    let w = rational(&proof.weight)?;
    let r = rational(&proof.ratio)?;
    let sq = square(p, &proof.beta, &proof.gamma, &w, &r)?;
    let residual = parse_sparse_limits(a, &proof.residual, MAX_TERMS + 3, 4096)?;
    ensure!(bounded(&residual), "residual bound");
    ensure!(
        &sq + &residual == p.clone().mul_coeff(Rational::from(proof.sign as i64)),
        "signed square+residual identity"
    );
    ensure!(
        proof.residual_certificate.sign == 1,
        "residual must be strictly positive"
    );
    ensure!(
        proof.residual_certificate.polynomial == a.format(&residual),
        "residual leaf polynomial binding"
    );
    verify_certificate(a, &residual, constraints, &[], &proof.residual_certificate)?;
    Ok(())
}
#[derive(Default, Serialize)]
struct Counts {
    negative_terms: usize,
    diagonal_pairs: usize,
    admissible_pairs: usize,
    candidate_attempts: usize,
    candidate_limit_reached: bool,
    coefficient_budget_declines: usize,
    residual_declines: usize,
    elapsed_budget_reached: bool,
}
fn discover(
    a: &Algebra,
    p: &Poly,
    domain: &Domain,
    constraints: &[Poly],
    deadline: Option<(Instant, f64)>,
) -> Result<(Option<Proof>, Counts)> {
    let mut counts = Counts::default();
    for sign in [1i8, -1] {
        let target = p.clone().mul_coeff(Rational::from(sign as i64));
        let terms: Vec<_> = target
            .coefficients
            .iter()
            .zip(target.exponents_iter())
            .map(|(c, e)| (c.clone(), e.to_vec()))
            .collect();
        let diagonals: BTreeMap<Vec<u32>, Rational> = terms
            .iter()
            .filter(|(c, e)| *c > Rational::zero() && e.iter().all(|x| x % 2 == 0))
            .map(|(c, e)| (e.iter().map(|x| x / 2).collect(), c.clone()))
            .collect();
        for (cross, alpha) in terms.iter().filter(|(c, _)| c.is_negative()) {
            counts.negative_terms += 1;
            for (beta, left) in &diagonals {
                if beta.iter().zip(alpha).any(|(b, a)| b > a) {
                    continue;
                }
                let gamma: Vec<_> = alpha.iter().zip(beta).map(|(a, b)| a - b).collect();
                if beta >= &gamma {
                    continue;
                }
                let Some(right) = diagonals.get(&gamma) else {
                    continue;
                };
                counts.diagonal_pairs += 1;
                let k = -cross.clone();
                let lower = k.clone() / (Rational::from(2) * left.clone());
                let upper = Rational::from(2) * right.clone() / k.clone();
                if lower > upper {
                    continue;
                }
                counts.admissible_pairs += 1;
                let mut ratios = vec![
                    lower.clone(),
                    upper.clone(),
                    (&lower + &upper) / Rational::from(2),
                ];
                if lower <= Rational::one() && upper >= Rational::one() {
                    ratios.insert(0, Rational::one());
                }
                let mut seen = Vec::new();
                for r in ratios {
                    if seen.contains(&r) {
                        continue;
                    }
                    seen.push(r.clone());
                    if counts.candidate_attempts >= MAX_CANDIDATES {
                        counts.candidate_limit_reached = true;
                        return Ok((None, counts));
                    }
                    if deadline
                        .is_some_and(|(start, seconds)| start.elapsed().as_secs_f64() >= seconds)
                    {
                        counts.elapsed_budget_reached = true;
                        return Ok((None, counts));
                    }
                    counts.candidate_attempts += 1;
                    let w = k.clone() / (Rational::from(2) * r.clone());
                    if height(&w) > 2048
                        || height(&r) > 2048
                        || height(&w).saturating_add(2 * height(&r)).saturating_add(4) > 4096
                    {
                        counts.coefficient_budget_declines += 1;
                        continue;
                    }
                    let sq = square(p, beta, &gamma, &w, &r)?;
                    ensure!(
                        sq.coefficient(alpha).is_some_and(|c| c == *cross),
                        "square does not match cross coefficient"
                    );
                    let residual = &target - &sq;
                    if !bounded(&residual) {
                        counts.coefficient_budget_declines += 1;
                        continue;
                    }
                    if let Some(cert) =
                        certify_nonzero(a, &residual, domain, &CertificateOptions::default())?
                    {
                        if cert.sign == 1 {
                            let proof = Proof {
                                sign,
                                beta: beta.clone(),
                                gamma: gamma.clone(),
                                weight: w.to_string(),
                                ratio: r.to_string(),
                                residual: a.format(&residual),
                                residual_certificate: cert,
                            };
                            replay(a, p, constraints, &proof)?;
                            return Ok((Some(proof), counts));
                        }
                    }
                    counts.residual_declines += 1;
                }
            }
        }
    }
    Ok((None, counts))
}
fn checked_source(
    row: &Value,
    names: &[String],
    cache: &mut BTreeMap<PathBuf, Value>,
    bytes_read: &mut usize,
) -> Result<Value> {
    let source = row["sources"]
        .as_array()
        .and_then(|s| s.first())
        .context("missing source provenance")?;
    let path = PathBuf::from(source["path"].as_str().context("source path")?);
    if !cache.contains_key(&path) {
        ensure!(cache.len() < 4096, "source count cap");
        let b = read(&path, 32 * 1024 * 1024)?;
        *bytes_read = bytes_read
            .checked_add(b.len())
            .context("source size overflow")?;
        ensure!(*bytes_read <= 128 * 1024 * 1024, "aggregate source cap");
        let record: Value = serde_json::from_slice(&b)?;
        ensure!(source["sha256"] == hash(&b), "source file hash mismatch");
        let output: Vec<String> = serde_json::from_value(record["output"].clone())?;
        let multiplicities: Option<Vec<u64>> =
            serde_json::from_value(record["multiplicities"].clone())?;
        ensure!(
            record["output_hash"] == hash(&serde_json::to_vec(&(&output, &multiplicities))?),
            "source output checksum mismatch"
        );
        cache.insert(path.clone(), json!({"record":record,"sha256":hash(&b)}));
    }
    let saved = &cache[&path];
    let record = &saved["record"];
    ensure!(
        saved["sha256"] == source["sha256"] && record["variable_names"] == json!(names),
        "source map/hash mismatch"
    );
    ensure!(
        record["output"]
            .as_array()
            .context("source output")?
            .contains(&row["polynomial"]),
        "candidate absent from source"
    );
    Ok(source.clone())
}
fn audit_parts(audit: &Value) -> Result<(&Value, &Value)> {
    let fingerprint = audit["domain_fingerprint"]
        .as_array()
        .context("audited domain fingerprint")?;
    let offset = if fingerprint.first().is_some_and(Value::is_string) {
        ensure!(
            fingerprint[0] == "local-generic-mc-v1",
            "unsupported tagged domain fingerprint"
        );
        1
    } else {
        0
    };
    ensure!(fingerprint.len() >= offset + 2, "short domain fingerprint");
    Ok((&fingerprint[offset], &fingerprint[offset + 1]))
}
fn worker(args: &Args) -> Result<Value> {
    let started = Instant::now();
    let scan_bytes = read(&args.scan, 16 * 1024 * 1024)?;
    let audit_bytes = read(&args.input_audit, 2 * 1024 * 1024)?;
    ensure!(
        hash(&scan_bytes) == args.scan_sha256 && hash(&audit_bytes) == args.audit_sha256,
        "scan/audit checksum mismatch"
    );
    let scan: Value = serde_json::from_slice(&scan_bytes)?;
    let audit: Value = serde_json::from_slice(&audit_bytes)?;
    ensure!(
        scan["kind"] == "research_recursive_quadratic_nonvanishing"
            && scan["completed_scan"] == true,
        "complete source scan required"
    );
    let names: Vec<String> = serde_json::from_value(scan["variables"].clone())?;
    ensure!(
        !names.is_empty()
            && names.len() <= 16
            && names.iter().all(|s| !s.is_empty()
                && s.len() <= 32
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            && names.iter().collect::<HashSet<_>>().len() == names.len(),
        "variable map cap"
    );
    let (audited_names, audited_constraints) = audit_parts(&audit)?;
    ensure!(
        *audited_names == scan["variables"] && audit["constraint_arguments"] == scan["constraints"],
        "audited map/constraints mismatch"
    );
    let a = Algebra::new(&names)?;
    let constraint_strings: Vec<String> = serde_json::from_value(scan["constraints"].clone())?;
    ensure!(constraint_strings.len() <= 64, "constraint count cap");
    let constraints = constraint_strings
        .iter()
        .map(|s| parse_constraint(&a, s))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        *audited_constraints
            == json!(
                constraints
                    .iter()
                    .map(|p| format!("({})>0", a.format(p)))
                    .collect::<Vec<_>>()
            ),
        "audited normalized constraints mismatch"
    );
    let domain = derive_domain(&a, &constraints, &[])?;
    ensure!(
        !domain.empty && json!(domain) == scan["domain"],
        "exact original domain mismatch"
    );
    let rows = scan["rows"].as_array().context("source rows")?;
    ensure!(rows.len() <= 4096, "row cap");
    let mut results = Vec::new();
    let mut cache = BTreeMap::new();
    let mut bytes_read = 0;
    let mut attempts = 0;
    let mut successes = 0;
    let mut baseline_now = 0;
    let mut complete = true;
    for row in rows {
        if !row["additional_certificate"].is_null() {
            continue;
        }
        if started.elapsed().as_secs_f64() >= args.seconds * 0.85 {
            complete = false;
            break;
        }
        let source = checked_source(row, &names, &mut cache, &mut bytes_read)?;
        let expression = row["polynomial"].as_str().context("polynomial")?;
        let p = parse_sparse(&a, expression)?;
        if let Some(cert) = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())? {
            verify_certificate(&a, &p, &constraints, &[], &cert)?;
            baseline_now += 1;
            results.push(json!({"polynomial":expression,"source":source,"already_certified":cert}));
            continue;
        }
        let (proof, counts) = discover(
            &a,
            &p,
            &domain,
            &constraints,
            Some((started, args.seconds * 0.85)),
        )?;
        attempts += counts.candidate_attempts;
        successes += usize::from(proof.is_some());
        let stop = counts.elapsed_budget_reached;
        results.push(json!({"polynomial":expression,"source":source,"counts":counts,"additional_certificate":proof}));
        if stop {
            complete = false;
            break;
        }
    }
    Ok(
        json!({"schema_version":1,"kind":"research_monomial_square_strict_residual","source_scan_sha256":args.scan_sha256,"input_audit_sha256":args.audit_sha256,"variables":names,"constraints":constraint_strings,"domain":domain,"input_declines":rows.len(),"processed":results.len(),"current_baseline_successes":baseline_now,"candidate_attempts":attempts,"additional_certificates":successes,"completed_scan":complete,"records_checked":cache.len(),"source_bytes_read":bytes_read,"rows":results,"limits":{"candidates_per_polynomial":MAX_CANDIDATES,"input_terms":MAX_TERMS,"input_coefficient_height":512,"residual_coefficient_height":4096},"wall_seconds":started.elapsed().as_secs_f64(),"coverage_claimed":false,"production_changes":false}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "deadline (0,60]"
    );
    if args.worker {
        println!("{}", worker(&args)?);
        return Ok(());
    }
    ensure!(!args.output.exists(), "output must be fresh");
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new("taskset");
    cmd.arg("-c")
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
    let mut process = run_bounded(&mut cmd, &limits)?;
    let result = process
        .stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .next_back()
        .and_then(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["kind"] == "research_monomial_square_strict_residual");
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.is_some();
    if result.is_some() {
        process.stdout.clear();
    }
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::OpenOptions::new().write(true).create_new(true).open(&args.output)?.write_all(&serde_json::to_vec_pretty(&json!({"schema_version":1,"binary_sha256":hash(&fs::read(exe)?),"limits":limits,"cpu":args.cpu,"process":process,"result":result}))?)?;
    ensure!(success, "bounded research worker incomplete");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_audited_domain_layouts_preserve_exact_bindings() {
        for fingerprint in [
            json!(["local-generic-mc-v1", ["x"], ["((1)*x)>0"]]),
            json!([["x"], ["((1)*x)>0"], {}]),
        ] {
            let audit = json!({"domain_fingerprint": fingerprint});
            let (names, constraints) = audit_parts(&audit).unwrap();
            assert_eq!(names, &json!(["x"]));
            assert_eq!(constraints, &json!(["((1)*x)>0"]));
        }
        assert!(audit_parts(&json!({"domain_fingerprint": ["unknown", [], []]})).is_err());
        assert!(audit_parts(&json!({"domain_fingerprint": []})).is_err());
    }
    #[test]
    fn weighted_monomial_square_has_independent_strict_replay() {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
        let constraints = vec![];
        let domain = derive_domain(&a, &constraints, &[]).unwrap();
        let p = a.parse("(2*x*y-3*z^2)^2+1").unwrap();
        assert!(
            certify_nonzero(&a, &p, &domain, &CertificateOptions::default())
                .unwrap()
                .is_none()
        );
        let (proof, counts) = discover(&a, &p, &domain, &constraints, None).unwrap();
        let proof = proof.unwrap();
        assert!(counts.candidate_attempts <= 32);
        replay(&a, &p, &constraints, &proof).unwrap();
        let mut bad = proof.clone();
        bad.weight = "-1".into();
        assert!(replay(&a, &p, &constraints, &bad).is_err());
        let mut bad = proof.clone();
        bad.ratio = "1".into();
        assert!(replay(&a, &p, &constraints, &bad).is_err());
        let mut bad = proof.clone();
        bad.residual = "0".into();
        assert!(replay(&a, &p, &constraints, &bad).is_err());
        let mut bad = proof;
        bad.residual_certificate.sign = -1;
        assert!(replay(&a, &p, &constraints, &bad).is_err());
        let negative = p.mul_coeff(Rational::from(-1));
        let (proof, _) = discover(&a, &negative, &domain, &constraints, None).unwrap();
        assert_eq!(proof.unwrap().sign, -1);
    }
    #[test]
    fn common_interior_zeros_never_count_as_nonvanishing() {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into(), "w".into()]).unwrap();
        let domain = derive_domain(&a, &[], &[]).unwrap();
        for text in ["(x*y-z^2)^2", "(x*y-z^2)^2+w^2"] {
            assert!(
                discover(&a, &a.parse(text).unwrap(), &domain, &[], None)
                    .unwrap()
                    .0
                    .is_none()
            );
        }
    }
    #[test]
    fn raw_input_caps_and_original_domain_binding() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        assert_eq!(
            parse_sparse(&a, "(2)+(-3)*x*y").unwrap(),
            a.parse("2-3*x*y").unwrap()
        );
        for bad in [
            "(2^1000000)",
            "(1)*x^999999999",
            "(1)*x+(1)*x",
            "(1)*x*x",
            "(0)",
        ] {
            assert!(parse_sparse(&a, bad).is_err());
        }
        for bad in ["x^1000000", "-1", "x*y", "x+y"] {
            assert!(parse_constraint(&a, bad).is_err());
        }
        let p = a.parse("(x*y-1)^2+x").unwrap();
        let constraints = vec![a.parse("x").unwrap()];
        let domain = derive_domain(&a, &constraints, &[]).unwrap();
        let proof = discover(&a, &p, &domain, &constraints, None)
            .unwrap()
            .0
            .unwrap();
        assert!(replay(&a, &p, &[], &proof).is_err());
    }
}
