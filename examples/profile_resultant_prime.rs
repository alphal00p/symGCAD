//! Research-only timing of one exact finite-field image of Res(p, p').
//! No CRT reconstruction, discriminant value, cache insertion or CAD claim.
#[allow(dead_code)]
#[path = "profile_operation.rs"]
mod capture_parser;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{
    domains::{
        EuclideanDomain,
        finite_field::{FiniteFieldCore, PrimeIteratorU64, ToFiniteField, Zp},
        integer::{Integer, IntegerRing, Z},
        rational::{Q, Rational},
    },
    poly::polynomial::MultivariatePolynomial,
    prelude::LicenseManager,
};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

type IPoly = MultivariatePolynomial<IntegerRing, u32>;
const MAX_TERMS: usize = 8192;
const MAX_COEFFICIENT_BITS: u64 = 4096;
const MAX_DENOMINATOR_BITS: u64 = 16384;
const MAX_CLEARED_BITS: u64 = 32768;
const MAX_BOUND_BITS: u64 = 1_000_000;
const MAX_IMAGE_TERMS: usize = 1_000_000;
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
const MAX_SKIPPED_PRIMES: usize = 16;

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    /// Must be a fresh directory; all partial diagnostics remain there.
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 300.0)]
    seconds: f64,
    #[arg(long, default_value_t = 4096)]
    memory_mib: u64,
    #[arg(long, hide = true)]
    worker: bool,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut h = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn create_json(path: &Path, value: &Value) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut f, value)?;
    f.flush()?;
    Ok(())
}
fn progress(directory: &Path, value: &Value) -> Result<()> {
    let temp = directory.join("progress.tmp");
    create_json(&temp, value)?;
    fs::rename(temp, directory.join("progress.json"))?;
    Ok(())
}

fn load(bytes: &[u8], expected: &str) -> Result<(OperationCapture, Algebra, Poly, usize)> {
    ensure!(bytes.len() <= 8 * 1024 * 1024, "capture byte cap");
    ensure!(
        expected.len() == 64 && hash(bytes) == expected,
        "capture hash mismatch"
    );
    let c: OperationCapture = serde_json::from_slice(bytes)?;
    ensure!(
        c.schema_version == 1 && c.coefficient_domain == "Q",
        "capture schema/domain"
    );
    ensure!(
        c.operation == AlgebraOperation::Discriminant,
        "discriminant capture required"
    );
    ensure!(
        !c.variable_names.is_empty()
            && c.variable_names.len() <= 128
            && c.variable_names.iter().all(|n| n.len() <= 1024),
        "variable map cap"
    );
    let x = c.variable.context("missing elimination variable")?;
    ensure!(
        x < c.variable_names.len(),
        "elimination variable outside full map"
    );
    let operands = c.operands.as_ref().context("capture operands absent")?;
    ensure!(
        !c.operands_omitted_for_size && operands.len() == 1,
        "capture operand count"
    );
    ensure!(
        hash(&serde_json::to_vec(operands)?) == c.input_hash,
        "operand checksum mismatch"
    );
    let algebra = Algebra::new(&c.variable_names)?;
    let p = capture_parser::plan_operand(&algebra, &operands[0])?;
    ensure!(
        (2..=64).contains(&p.degree(x)),
        "elimination degree must be 2..64"
    );
    ensure!(
        (0..p.nvars())
            .filter(|&i| i != x && p.degree(i) > 0)
            .count()
            <= 3,
        "at most three active remaining parameters"
    );
    ensure!(
        (0..p.nvars()).all(|i| p.degree(i) <= 4096),
        "parameter degree cap"
    );
    Ok((c, algebra, p, x))
}

struct Primitive {
    p: IPoly,
    scale: Rational,
}

/// p = scale * primitive with positive rational scale; signs stay in p.
/// Check the original coefficient bounds before the first LCM/product.
fn primitive(p: &Poly) -> Result<Primitive> {
    ensure!(
        !p.is_zero() && p.nterms() <= MAX_TERMS && p.nvars() <= 128,
        "primitive input shape"
    );
    ensure!(
        p.coefficients.iter().all(
            |c| c.numerator_ref().significant_bits() <= MAX_COEFFICIENT_BITS
                && c.denominator_ref().significant_bits() <= MAX_COEFFICIENT_BITS
        ),
        "primitive input coefficient cap"
    );
    let mut d = Integer::one();
    for c in &p.coefficients {
        let gcd = Z.gcd(&d, c.denominator_ref());
        d = Z.quot_rem(&d, &gcd).0 * c.denominator_ref();
        ensure!(
            d.significant_bits() <= MAX_DENOMINATOR_BITS,
            "denominator LCM cap"
        );
    }
    let integer = p.map_coeff(
        |c| c.numerator_ref() * &Z.quot_rem(&d, c.denominator_ref()).0,
        Z,
    );
    ensure!(
        integer
            .coefficients
            .iter()
            .all(|c| c.significant_bits() <= MAX_CLEARED_BITS),
        "cleared coefficient cap"
    );
    let content = integer
        .coefficients
        .iter()
        .fold(Integer::zero(), |g, c| Z.gcd(&g, c))
        .abs();
    ensure!(!content.is_zero(), "zero primitive content");
    let p_integer = integer.div_coeff(&content);
    let scale = Rational::from((content, d));
    ensure!(scale > 0, "nonpositive primitive scale");
    let replay = p_integer.map_coeff(|c| Rational::from(c.clone()) * &scale, Q);
    ensure!(
        replay == *p && p_integer.variables() == p.variables(),
        "exact primitive replay failed"
    );
    Ok(Primitive {
        p: p_integer,
        scale,
    })
}

fn bound(p: &IPoly, q: &IPoly, x: usize) -> Result<(Integer, Integer, Integer)> {
    let norm = |f: &IPoly| {
        f.coefficients
            .iter()
            .fold(Integer::zero(), |n, c| n + c.abs())
    };
    let pn = norm(p);
    let qn = norm(q);
    let estimate = pn
        .significant_bits()
        .checked_mul(u64::from(q.degree(x)))
        .and_then(|v| v.checked_add(qn.significant_bits().checked_mul(u64::from(p.degree(x)))?));
    ensure!(
        estimate.is_some_and(|b| b <= MAX_BOUND_BITS),
        "coefficient norm bound cap"
    );
    let b = pn.pow(u64::from(q.degree(x))) * qn.pow(u64::from(p.degree(x)));
    Ok((pn, qn, b))
}

fn reduced_pair(
    p: &IPoly,
    q: &IPoly,
    x: usize,
    prime: u32,
) -> Result<
    Option<(
        MultivariatePolynomial<Zp, u32>,
        MultivariatePolynomial<Zp, u32>,
    )>,
> {
    ensure!(
        prime > 2 && Integer::from(prime).is_prime(0),
        "prime must be proven word-sized prime"
    );
    ensure!(
        p.variables() == q.variables() && x < p.nvars(),
        "image input maps/axis"
    );
    let field = Zp::new(prime);
    let pm = p.map_coeff(|c| c.to_finite_field(&field), field.clone());
    let qm = q.map_coeff(|c| c.to_finite_field(&field), field.clone());
    if pm.is_zero() || qm.is_zero() || pm.degree(x) != p.degree(x) || qm.degree(x) != q.degree(x) {
        return Ok(None);
    }
    Ok(Some((pm, qm)))
}

fn image(
    p: &IPoly,
    pm: &MultivariatePolynomial<Zp, u32>,
    qm: &MultivariatePolynomial<Zp, u32>,
    x: usize,
    prime: u32,
) -> Result<IPoly> {
    let field = Zp::new(prime);
    let finite = pm.to_univariate(x).resultant(&qm.to_univariate(x));
    let converted = finite.map_coeff(|c| field.to_symmetric_integer(c), Z);
    let r = if converted.is_constant() {
        p.constant(converted.get_constant())
    } else {
        converted
    };
    ensure!(
        r.variables() == p.variables() && r.degree(x) == 0,
        "prime image map/support mismatch"
    );
    ensure!(r.nterms() <= MAX_IMAGE_TERMS, "prime image term cap");
    for i in 0..r.nvars() {
        if i != x {
            let limit = u64::from(qm.degree(x)) * u64::from(pm.degree(i))
                + u64::from(pm.degree(x)) * u64::from(qm.degree(i));
            ensure!(
                u64::from(r.degree(i)) <= limit,
                "prime image parameter degree bound"
            );
        }
    }
    ensure!(
        r.coefficients
            .iter()
            .all(|c| c.abs() <= Integer::from(prime / 2)),
        "uncentered image coefficient"
    );
    Ok(r)
}

fn image_text(algebra: &Algebra, p: &IPoly) -> Result<String> {
    ensure!(
        p.variables() == &algebra.variables && p.nterms() <= MAX_IMAGE_TERMS,
        "image export shape"
    );
    if p.is_zero() {
        return Ok("0".into());
    }
    let mut out = String::new();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut term = format!("({c})");
        for (name, &power) in algebra.names.iter().zip(e) {
            if power > 0 {
                term.push('*');
                term.push_str(name);
                if power > 1 {
                    term.push('^');
                    term.push_str(&power.to_string());
                }
            }
        }
        ensure!(
            out.len()
                .checked_add(term.len() + usize::from(!out.is_empty()))
                .is_some_and(|b| b <= MAX_IMAGE_BYTES),
            "image export byte cap"
        );
        if !out.is_empty() {
            out.push('+');
        }
        out.push_str(&term);
    }
    Ok(out)
}

fn worker(args: &Args) -> Result<()> {
    let start = Instant::now();
    progress(
        &args.output,
        &json!({"phase":"input_preflight","full_resultant_computed":false}),
    )?;
    let bytes = capture_parser::bounded_capture_bytes(&args.capture)?;
    let (capture, algebra, p, x) = load(&bytes, &args.capture_sha256)?;
    let derivative = p.derivative(x);
    ensure!(
        p.nterms().saturating_add(derivative.nterms()) <= MAX_TERMS,
        "combined input term cap"
    );
    progress(
        &args.output,
        &json!({"phase":"primitive_associates","capture_sha256":args.capture_sha256,"input_hash":capture.input_hash,"variables":algebra.names,"variable":x}),
    )?;
    let left = primitive(&p)?;
    let right = primitive(&derivative)?;
    let (pn, qn, b) = bound(&left.p, &right.p, x)?;
    let preparation_seconds = start.elapsed().as_secs_f64();
    let mut metadata = json!({"schema_version":1,"scope":"one exact finite-field image only",
        "capture_sha256":args.capture_sha256,"input_hash":capture.input_hash,
        "backend":LicenseManager::get_version(),"variables":algebra.names,"variable":x,
        "input_terms":[p.nterms(),derivative.nterms()],"degrees":[p.degree(x),derivative.degree(x)],
        "primitive_terms":[left.p.nterms(),right.p.nterms()],
        "primitive_identity_replayed":true,"positive_rational_scales":[left.scale.to_string(),right.scale.to_string()],
        "primitive_norms":[pn.to_string(),qn.to_string()],"resultant_coefficient_bound_bits":b.significant_bits(),
        "resultant_coefficient_bound_sha256":hash(b.to_string().as_bytes()),
        "deterministic_crt_condition":"future CRT would require product of distinct accepted primes > 2*bound",
        "preparation_seconds":preparation_seconds,"full_resultant_computed":false,"discriminant_computed":false,"original_coverage_claimed":false,
        "helper_source_sha256":hash(include_bytes!("profile_resultant_prime.rs")),"parser_source_sha256":hash(include_bytes!("profile_operation.rs"))});
    let mut primes = PrimeIteratorU64::new(1 << 30);
    let mut skipped = Vec::new();
    for _ in 0..=MAX_SKIPPED_PRIMES {
        let prime = u32::try_from(primes.next().context("prime iterator exhausted")?)?;
        metadata["prime"] = json!(prime);
        metadata["skipped_degree_drop_primes"] = json!(skipped);
        metadata["phase"] = json!("modularization");
        progress(&args.output, &metadata)?;
        let mapped = Instant::now();
        let Some((pm, qm)) = reduced_pair(&left.p, &right.p, x, prime)? else {
            skipped.push(prime);
            continue;
        };
        metadata["mapping_seconds"] = json!(mapped.elapsed().as_secs_f64());
        metadata["modular_degrees"] = json!([pm.degree(x), qm.degree(x)]);
        metadata["modular_terms"] = json!([pm.nterms(), qm.nterms()]);
        metadata["phase"] = json!("finite_field_resultant");
        progress(&args.output, &metadata)?;
        let native = Instant::now();
        let r = image(&left.p, &pm, &qm, x, prime)?;
        metadata["image_seconds"] = json!(native.elapsed().as_secs_f64());
        metadata["phase"] = json!("image_export");
        progress(&args.output, &metadata)?;
        let text = image_text(&algebra, &r)?;
        let image_record = json!({"schema_version":1,"kind":"primitive_resultant_mod_prime","capture_sha256":args.capture_sha256,
            "input_hash":capture.input_hash,"prime":prime,"variables":algebra.names,"variable":x,
            "expression":text,"expression_sha256":hash(text.as_bytes()),"terms":r.nterms(),
            "degrees":(0..r.nvars()).map(|i|r.degree(i)).collect::<Vec<_>>()});
        create_json(&args.output.join("image.json"), &image_record)?;
        ensure!(
            hash(&capture_parser::bounded_capture_bytes(&args.capture)?) == args.capture_sha256,
            "capture changed during image calculation"
        );
        metadata["phase"] = json!("complete_one_image");
        metadata["image_terms"] = json!(r.nterms());
        metadata["image_file_sha256"] = json!(file_hash(&args.output.join("image.json"))?);
        metadata["image_expression_sha256"] = json!(hash(text.as_bytes()));
        metadata["worker_seconds"] = json!(start.elapsed().as_secs_f64());
        metadata["capture_unchanged"] = json!(true);
        metadata["one_modular_image_complete"] = json!(true);
        create_json(&args.output.join("worker.json"), &metadata)?;
        progress(&args.output, &metadata)?;
        return Ok(());
    }
    anyhow::bail!("bounded degree-drop prime stock exhausted")
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 300.0,
        "seconds must be in (0,300]"
    );
    ensure!(
        (1..=4096).contains(&args.memory_mib),
        "memory must be in1..4096MiB"
    );
    if args.worker {
        return worker(&args);
    }
    let before = capture_parser::bounded_capture_bytes(&args.capture)?;
    ensure!(
        hash(&before) == args.capture_sha256,
        "capture hash mismatch before worker"
    );
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&args.output).context("output must be a fresh directory")?;
    let binary = std::env::current_exe()?;
    let binary_hash = file_hash(&binary)?;
    let mut command = Command::new(&binary);
    command
        .arg(&args.capture)
        .arg("--capture-sha256")
        .arg(&args.capture_sha256)
        .arg("--output")
        .arg(&args.output)
        .arg("--seconds")
        .arg(args.seconds.to_string())
        .arg("--memory-mib")
        .arg(args.memory_mib.to_string())
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    create_json(
        &args.output.join("launch.json"),
        &json!({"schema_version":1,"capture_sha256":args.capture_sha256,
        "binary_sha256":binary_hash,"seconds":args.seconds,"memory_mib":args.memory_mib,
        "arguments":command.get_args().map(|a|a.to_string_lossy().into_owned()).collect::<Vec<_>>()}),
    )?;
    let outcome = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: args.seconds,
            memory_mib: args.memory_mib,
            workers: 1,
            max_cells: None,
        },
    )?;
    // Preserve the completed or interrupted process record before any fallible
    // post-run rehash or worker-file read. The audit is a separate artifact.
    create_json(
        &args.output.join("process.json"),
        &json!({"schema_version":1,"process":outcome,"binary_sha256":binary_hash,
            "capture_sha256":args.capture_sha256,"post_run_audit_pending":true,
            "full_resultant_computed":false,"discriminant_computed":false,"original_coverage_claimed":false}),
    )?;
    let unchanged = hash(&capture_parser::bounded_capture_bytes(&args.capture)?)
        == args.capture_sha256
        && file_hash(&binary)? == binary_hash;
    let succeeded = outcome.status == "completed"
        && outcome.exit_code == Some(0)
        && !outcome.output_truncated
        && unchanged;
    let result = if succeeded {
        Some(serde_json::from_slice::<Value>(&fs::read(
            args.output.join("worker.json"),
        )?)?)
    } else {
        None
    };
    create_json(
        &args.output.join("audit.json"),
        &json!({"schema_version":1,"process":outcome,"input_binary_unchanged":unchanged,
        "binary_sha256":binary_hash,"capture_sha256":args.capture_sha256,"result":result,
        "full_resultant_computed":false,"discriminant_computed":false,"original_coverage_claimed":false}),
    )?;
    ensure!(
        succeeded,
        "one-prime worker incomplete or failed; inspect preserved diagnostics"
    );
    println!(
        "{}",
        json!({"status":"one_image_complete","output":args.output,"full_resultant_computed":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn a(names: &[&str]) -> Algebra {
        Algebra::new(&names.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }
    fn compare(algebra: &Algebra, text: &str, x: usize, prime: u32) -> Result<IPoly> {
        let p = algebra.parse(text)?;
        let q = p.derivative(x);
        let left = primitive(&p)?;
        let right = primitive(&q)?;
        let (pm, qm) =
            reduced_pair(&left.p, &right.p, x, prime)?.context("test prime drops degree")?;
        let got = image(&left.p, &pm, &qm, x, prime)?;
        let exact = p
            .to_univariate(x)
            .resultant_ducos_integer(&q.to_univariate(x));
        let exact = if exact.is_constant() {
            p.constant(exact.get_constant())
        } else {
            exact
        };
        let scale =
            left.scale.pow(u64::from(q.degree(x))) * right.scale.pow(u64::from(p.degree(x)));
        let primitive_exact = exact.div_coeff(&scale);
        ensure!(
            primitive_exact
                .coefficients
                .iter()
                .all(|c| c.denominator_ref().is_one()),
            "integer resultant expected"
        );
        let field = Zp::new(prime);
        let expected = primitive_exact.map_coeff(
            |c| field.to_symmetric_integer(&c.numerator_ref().to_finite_field(&field)),
            Z,
        );
        assert_eq!(got, expected);
        assert_eq!(got.variables(), &algebra.variables);
        let formatted = image_text(algebra, &got)?;
        assert_eq!(
            algebra.format(&got.map_coeff(|c| Rational::from(c.clone()), Q)),
            formatted
        );
        let (_, _, b) = bound(&left.p, &right.p, x)?;
        assert!(
            primitive_exact
                .coefficients
                .iter()
                .all(|c| c.numerator_ref().abs() <= b)
        );
        Ok(got)
    }
    #[test]
    fn signed_rational_scales_and_full_inactive_map() -> Result<()> {
        let algebra = a(&["unused", "s", "x", "after"]);
        for p in [
            "(-3/5)*(s*x^5+x^2+7/11)",
            "(2/7)*(x^6+s*x+1)",
            "(-5/3)*(s*x^2+2*x+1)",
        ] {
            compare(&algebra, p, 2, 1009)?;
        }
        Ok(())
    }
    #[test]
    fn degree_drops_skip_and_parameter_support_drops_are_valid() -> Result<()> {
        let algebra = a(&["s", "x"]);
        let p = algebra.parse("101*s*x^3+x^2+1")?;
        let q = p.derivative(1);
        let l = primitive(&p)?;
        let r = primitive(&q)?;
        assert!(reduced_pair(&l.p, &r.p, 1, 101)?.is_none());
        compare(&algebra, "101*s*x^3+x^2+1", 1, 103)?;
        compare(&algebra, "x^3+101*s*x+1", 1, 101)?;
        assert!(reduced_pair(&l.p, &r.p, 1, 15).is_err());
        Ok(())
    }
    #[test]
    fn repeated_factor_zero_keeps_full_map() -> Result<()> {
        let algebra = a(&["unused", "x", "s"]);
        assert!(compare(&algebra, "(-7/11)*(x^3+s)^2", 1, 101)?.is_zero());
        Ok(())
    }
    #[test]
    fn capture_binding_sparse_preflight_and_generic_caps() -> Result<()> {
        let algebra = a(&["s", "x"]);
        let p = algebra.parse("x^4+s*x+1")?;
        let operands = vec![algebra.format(&p)];
        let mut c = OperationCapture {
            schema_version: 1,
            backend_version: "test".into(),
            coefficient_domain: "Q".into(),
            variable_names: algebra.names.clone(),
            operation: AlgebraOperation::Discriminant,
            variable: Some(1),
            key: "test".into(),
            input_hash: hash(&serde_json::to_vec(&operands)?),
            operands: Some(operands),
            operands_omitted_for_size: false,
            complete: false,
            elapsed_seconds: None,
            output_hash: None,
        };
        let bytes = serde_json::to_vec(&c)?;
        assert!(load(&bytes, &hash(&bytes)).is_ok());
        assert!(load(&bytes, &"0".repeat(64)).is_err());
        c.input_hash = "0".repeat(64);
        let bytes = serde_json::to_vec(&c)?;
        assert!(load(&bytes, &hash(&bytes)).is_err());
        assert!(capture_parser::plan_operand(&algebra, "(1)*(x+s)^100").is_err());
        assert!(primitive(&algebra.parse("2^4097*x^3+1")?).is_err());
        let big = algebra.parse("(2^4095*x^3+1)/(2^4095-1)")?;
        assert!(primitive(&big).is_ok());
        Ok(())
    }
}
