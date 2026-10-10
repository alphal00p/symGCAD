//! Bounded research-only binary segmentation with up to three parameters.
//! Uses the unchanged public exact kernel with an explicit research envelope.
//! Sparse outputs are candidates, never completed operation-cache records or CAD proofs.
#[allow(dead_code)]
#[path = "profile_operation.rs"]
mod capture_parser;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{domains::integer::Integer, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
    resultant_segmentation::{self, Options, Plan},
};

const CAPTURE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_BITS: usize = 2_000_000_000;
const MAX_DIGITS: usize = 2_500_000;
const MAX_CANDIDATE_BYTES: u64 = 1024 * 1024 * 1024;
const SMALL_JSON_BYTES: u64 = 256 * 1024;
const HEADER_RESERVE: u64 = 64 * 1024;

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    /// Must be fresh. Partial outputs and terminal process records are preserved.
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 300.0)]
    seconds: f64,
    #[arg(long, default_value_t = 4096)]
    memory_mib: u64,
    /// Only perform parsing, exact static planning and storage preflights.
    #[arg(long)]
    plan_only: bool,
    #[arg(long, hide = true)]
    worker: bool,
}

fn options() -> Options {
    Options {
        min_parameters: 1,
        max_parameters: 3,
        max_digits: MAX_DIGITS,
        max_output_bits: MAX_BITS,
        max_total_input_bits: MAX_BITS,
        max_output_terms: MAX_DIGITS,
        ..Options::default()
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn open_regular(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    ensure!(file.metadata()?.is_file(), "expected regular file");
    Ok(file)
}
fn read_bounded(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let file = open_regular(path)?;
    ensure!(file.metadata()?.len() <= cap, "file byte cap");
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= cap, "file grew beyond cap");
    Ok(bytes)
}
fn file_hash(path: &Path, cap: u64) -> Result<String> {
    let mut f = open_regular(path)?;
    ensure!(f.metadata()?.len() <= cap, "hash file byte cap");
    let mut h = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total = total.checked_add(n as u64).context("hash byte overflow")?;
        ensure!(total <= cap, "hash file grew beyond cap");
        h.update(&buffer[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn create_json(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() as u64 <= SMALL_JSON_BYTES, "metadata byte cap");
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
fn progress(output: &Path, metadata: &Value) -> Result<()> {
    let temporary = output.join("progress.tmp");
    create_json(&temporary, metadata)?;
    fs::rename(temporary, output.join("progress.json"))?;
    Ok(())
}
fn authenticated(bytes: &[u8], expected: &str) -> Result<()> {
    ensure!(
        expected.len() == 64
            && expected.bytes().all(|b| b.is_ascii_hexdigit())
            && hash(bytes) == expected,
        "capture SHA256 mismatch"
    );
    Ok(())
}

struct Input {
    capture: OperationCapture,
    algebra: Algebra,
    p: Poly,
    q: Poly,
    x: usize,
    discriminant: bool,
}
fn load(bytes: &[u8], expected: &str) -> Result<Input> {
    ensure!(bytes.len() as u64 <= CAPTURE_BYTES, "capture byte cap");
    authenticated(bytes, expected)?;
    let c: OperationCapture = serde_json::from_slice(bytes)?;
    ensure!(
        c.schema_version == 1 && c.coefficient_domain == "Q",
        "capture schema/domain"
    );
    let discriminant = c.operation == AlgebraOperation::Discriminant;
    ensure!(
        discriminant || c.operation == AlgebraOperation::Resultant,
        "unsupported operation"
    );
    ensure!(
        !c.variable_names.is_empty() && c.variable_names.len() <= 128,
        "variable map size"
    );
    // Bound names before registering symbols in the backend.
    ensure!(
        c.variable_names.iter().all(|name| {
            let mut chars = name.bytes();
            name.len() <= 64
                && chars
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
        }),
        "bounded ASCII variable names required"
    );
    ensure!(
        c.variable_names
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            == c.variable_names.len(),
        "duplicate variable name"
    );
    let x = c.variable.context("missing elimination variable")?;
    ensure!(x < c.variable_names.len(), "elimination axis outside map");
    let operands = c.operands.as_ref().context("capture operands absent")?;
    ensure!(
        !c.operands_omitted_for_size && operands.len() == if discriminant { 1 } else { 2 },
        "operand count"
    );
    ensure!(
        hash(&serde_json::to_vec(operands)?) == c.input_hash,
        "ordered operand checksum mismatch"
    );
    let algebra = Algebra::new(&c.variable_names)?;
    let p = capture_parser::plan_operand(&algebra, &operands[0])?;
    let q = if discriminant {
        p.derivative(x)
    } else {
        capture_parser::plan_operand(&algebra, &operands[1])?
    };
    ensure!(
        (2..=64).contains(&p.degree(x)) && (1..=64).contains(&q.degree(x)),
        "elimination degree cap"
    );
    ensure!(
        p.nterms().saturating_add(q.nterms()) <= options().max_input_terms,
        "combined input term cap"
    );
    ensure!(
        (0..p.nvars()).all(|i| p.degree(i) <= 4096 && q.degree(i) <= 4096),
        "input degree cap"
    );
    ensure!(
        (0..p.nvars())
            .filter(|&i| i != x && (p.degree(i) != 0 || q.degree(i) != 0))
            .count()
            <= 3,
        "parameter count cap"
    );
    Ok(Input {
        capture: c,
        algebra,
        p,
        q,
        x,
        discriminant,
    })
}

fn denominator(p: &Poly, cap: u64) -> Result<Integer> {
    let mut d = Integer::one();
    for c in &p.coefficients {
        ensure!(
            c.numerator_ref()
                .significant_bits()
                .saturating_add(c.denominator_ref().significant_bits())
                <= 4096,
            "input coefficient cap before LCM"
        );
        d = d.lcm(c.denominator_ref());
        ensure!(d.significant_bits() <= cap, "denominator LCM cap");
    }
    Ok(d)
}
fn decimal_bound(bits: u64) -> Result<u64> {
    // 1/3 > log10(2); includes zero and allows an extra leading digit.
    bits.checked_div(3)
        .and_then(|n| n.checked_add(1))
        .context("decimal bound overflow")
}
fn exponent_bytes(degrees: &[usize]) -> Result<u64> {
    degrees.iter().try_fold(2u64, |sum, d| {
        sum.checked_add(d.to_string().len() as u64 + 1)
            .context("exponent byte bound overflow")
    })
}
fn worst_sparse_bytes(
    terms: usize,
    numerator_bits: u64,
    denominator_bits: u64,
    degrees: &[usize],
) -> Result<u64> {
    let per_term = decimal_bound(numerator_bits)?
        .checked_add(decimal_bound(denominator_bits)?)
        .and_then(|n| n.checked_add(48))
        .and_then(|n| n.checked_add(exponent_bytes(degrees).ok()?))
        .context("term byte bound overflow")?;
    (terms as u64)
        .checked_mul(per_term)
        .and_then(|n| n.checked_add(HEADER_RESERVE))
        .context("output byte bound overflow")
}
fn restoration_preflight(input: &Input, plan: &Plan) -> Result<Value> {
    let o = options();
    let dp = denominator(&input.p, o.max_denominator_bits)?;
    let dq = denominator(&input.q, o.max_denominator_bits)?;
    let db = |d: &Integer| if d.is_one() { 0 } else { d.significant_bits() };
    let denominator_bits = db(&dp)
        .checked_mul(u64::from(input.q.degree(input.x)))
        .and_then(|b| b.checked_add(db(&dq).checked_mul(u64::from(input.p.degree(input.x)))?))
        .context("restored denominator bound overflow")?;
    // Even integer Rational coefficients retain a denominator equal to one.
    let coefficient_bits = plan
        .coefficient_bound_bits
        .checked_add(denominator_bits.max(1))
        .context("restored coefficient bound overflow")?;
    let restored_bits = coefficient_bits
        .checked_mul(plan.digits as u64)
        .context("restored storage overflow")?;
    ensure!(
        restored_bits <= MAX_BITS as u64,
        "restored rational coefficient storage cap"
    );
    let bytes = worst_sparse_bytes(
        plan.digits,
        plan.coefficient_bound_bits,
        denominator_bits,
        &plan.degree_bounds,
    )?;
    ensure!(
        bytes <= MAX_CANDIDATE_BYTES,
        "pre-native sparse output byte cap"
    );
    Ok(
        json!({"positive_input_denominator_bits":[dp.significant_bits(),dq.significant_bits()],
        "resultant_denominator_bit_bound":denominator_bits,"resultant_restored_coefficient_bit_bound":restored_bits,
        "resultant_sparse_byte_bound":bytes,
        "discriminant_quotient_storage_checked_after_exact_division":input.discriminant}),
    )
}

/// The count is logical coefficient payload, not an estimate of native peak RAM.
fn output_cost(p: &Poly, map: &Poly, x: usize, plan: &Plan) -> Result<(u64, u64)> {
    ensure!(
        p.variables() == map.variables() && p.degree(x) == 0,
        "output map/eliminated support changed"
    );
    ensure!(p.nterms() <= MAX_DIGITS, "output term cap");
    ensure!(
        (0..p.nvars()).all(|i| p.degree(i) as usize <= plan.degree_bounds[i]),
        "output degree bound exceeded"
    );
    let mut bits = 0u64;
    let mut bytes = HEADER_RESERVE;
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        let nb = c.numerator_ref().significant_bits();
        let db = c.denominator_ref().significant_bits();
        bits = bits
            .checked_add(nb)
            .and_then(|v| v.checked_add(db))
            .context("retained bit overflow")?;
        ensure!(bits <= MAX_BITS as u64, "retained coefficient bit cap");
        bytes = bytes
            .checked_add(decimal_bound(nb)?)
            .and_then(|v| v.checked_add(decimal_bound(db).ok()?))
            .and_then(|v| v.checked_add(48))
            .context("candidate byte overflow")?;
        for exponent in e {
            bytes = bytes
                .checked_add(exponent.to_string().len() as u64 + 1)
                .context("candidate exponent byte overflow")?;
        }
        ensure!(bytes <= MAX_CANDIDATE_BYTES, "candidate sparse byte cap");
    }
    Ok((bits, bytes))
}

struct LimitedWriter<W> {
    inner: W,
    hash: Sha256,
    bytes: u64,
    cap: u64,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .checked_add(bytes.len() as u64)
            .is_none_or(|n| n > self.cap)
        {
            return Err(io::Error::other("stream byte cap"));
        }
        let n = self.inner.write(bytes)?;
        self.hash.update(&bytes[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
fn sparse<W: Write>(out: &mut W, p: &Poly) -> Result<()> {
    out.write_all(b"{\"terms\":[")?;
    for (i, (c, e)) in p.coefficients.iter().zip(p.exponents_iter()).enumerate() {
        if i != 0 {
            out.write_all(b",")?;
        }
        out.write_all(b"{\"coefficient\":")?;
        serde_json::to_writer(&mut *out, &c.to_string())?;
        out.write_all(b",\"exponents\":")?;
        serde_json::to_writer(&mut *out, e)?;
        out.write_all(b"}")?;
    }
    out.write_all(b"]}")?;
    Ok(())
}
fn sparse_hash(p: &Poly) -> Result<String> {
    let mut out = LimitedWriter {
        inner: io::sink(),
        hash: Sha256::new(),
        bytes: 0,
        cap: MAX_CANDIDATE_BYTES,
    };
    sparse(&mut out, p)?;
    Ok(format!("{:x}", out.hash.finalize()))
}
fn export(output: &Path, metadata: &Value, p: &Poly, estimated: u64) -> Result<(String, u64)> {
    let header = serde_json::to_vec(metadata)?;
    ensure!(
        header.len() as u64 + 64 <= HEADER_RESERVE && estimated <= MAX_CANDIDATE_BYTES,
        "candidate header/byte preflight"
    );
    let temporary = output.join("candidate.partial.json");
    let destination = output.join("candidate.json");
    ensure!(
        !destination.exists(),
        "candidate destination already exists"
    );
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut out = LimitedWriter {
        inner: io::BufWriter::new(file),
        hash: Sha256::new(),
        bytes: 0,
        cap: estimated,
    };
    out.write_all(b"{\"metadata\":")?;
    out.write_all(&header)?;
    out.write_all(b",\"polynomial\":")?;
    sparse(&mut out, p)?;
    out.write_all(b"}\n")?;
    out.flush()?;
    out.inner.get_ref().sync_all()?;
    let checksum = format!("{:x}", out.hash.finalize());
    fs::rename(temporary, destination)?;
    File::open(output)?.sync_all()?;
    Ok((checksum, out.bytes))
}
fn discriminant(p: &Poly, x: usize, r: Poly) -> Result<(Poly, Value)> {
    let r = if r.is_constant() {
        p.constant(r.get_constant())
    } else {
        r
    };
    ensure!(
        r.variables() == p.variables() && r.degree(x) == 0,
        "LC adapter input map/support"
    );
    let leading = p.univariate_lcoeff(x);
    ensure!(!leading.is_zero(), "zero symbolic leading coefficient");
    let resultant_hash = sparse_hash(&r)?;
    let leading_hash = sparse_hash(&leading)?;
    let (mut d, rem) = r.quot_rem(&leading, false);
    ensure!(rem.is_zero(), "nonexact signed discriminant LC division");
    let negative = matches!(p.degree(x) % 4, 2 | 3);
    if negative {
        d = -d;
    }
    if d.is_constant() {
        d = p.constant(d.get_constant());
    }
    ensure!(
        d.variables() == p.variables() && d.degree(x) == 0,
        "LC adapter output map/support"
    );
    Ok((
        d,
        json!({"relation":"D*LC(p)=(-1)^(m*(m-1)/2)*Res(p,p')",
        "checked_by":"exact quotient and zero remainder over Q[full variable map]",
        "negative_sign":negative,"degree":p.degree(x),"resultant_sparse_sha256":resultant_hash,
        "leading_coefficient_sparse_sha256":leading_hash,"remainder_zero":true,
        "specialized_leading_coefficient_zero_loci_retained":true}),
    ))
}

fn worker(args: &Args) -> Result<()> {
    let started = Instant::now();
    let mut metadata = json!({"schema_version":1,"kind":"research_binary_segmentation_candidate",
        "phase":"input_preflight","capture_sha256":args.capture_sha256,
        "independent_algebra_verification":false,"original_coverage_claimed":false,"cache_insertion":false,
        "helper_source_sha256":hash(include_bytes!("profile_multivariate_segmentation.rs")),
        "parser_source_sha256":hash(include_bytes!("profile_operation.rs")),
        "kernel_source_sha256":hash(include_bytes!("../src/resultant_segmentation.rs"))});
    progress(&args.output, &metadata)?;
    let bytes = read_bounded(&args.capture, CAPTURE_BYTES)?;
    let input = load(&bytes, &args.capture_sha256)?;
    metadata["variables"] = json!(input.algebra.names);
    metadata["variable"] = json!(input.x);
    metadata["operation"] = json!(input.capture.operation);
    metadata["input_hash"] = json!(input.capture.input_hash);
    metadata["input_terms"] = json!([input.p.nterms(), input.q.nterms()]);
    metadata["input_degrees"] = json!([input.p.degree(input.x), input.q.degree(input.x)]);
    metadata["backend"] = json!(LicenseManager::get_version());
    metadata["policy"] = json!({"parameters_max":3,"digits_max":MAX_DIGITS,"output_terms_max":MAX_DIGITS,
        "encoded_output_bits_max":MAX_BITS,"encoded_input_bits_max":MAX_BITS,
        "retained_rational_coefficient_bits_max":MAX_BITS,"candidate_bytes_max":MAX_CANDIDATE_BYTES,
        "other_limits":"unchanged public kernel Options defaults","production_policy_unchanged":true});
    metadata["phase"] = json!("static_kernel_and_restoration_preflight");
    progress(&args.output, &metadata)?;
    let Some(plan) = resultant_segmentation::plan(&input.p, &input.q, input.x, &options())? else {
        metadata["phase"] = json!("static_decline");
        metadata["status"] = json!("kernel_static_decline");
        metadata["algebra_complete"] = json!(false);
        create_json(&args.output.join("worker.json"), &metadata)?;
        progress(&args.output, &metadata)?;
        return Ok(());
    };
    metadata["plan"] = json!(plan);
    metadata["rational_storage_preflight"] = restoration_preflight(&input, &plan)?;
    metadata["preparation_seconds"] = json!(started.elapsed().as_secs_f64());
    if args.plan_only {
        metadata["phase"] = json!("plan_only_complete");
        metadata["status"] = json!("plan_only_complete");
        metadata["algebra_complete"] = json!(false);
        create_json(&args.output.join("worker.json"), &metadata)?;
        progress(&args.output, &metadata)?;
        return Ok(());
    }
    metadata["phase"] = json!("segmentation_encoding_native_decode");
    progress(&args.output, &metadata)?;
    let operation = Instant::now();
    let (r, statistics) =
        resultant_segmentation::try_resultant(&input.p, &input.q, input.x, &options(), || {
            ensure!(
                started.elapsed().as_secs_f64() < args.seconds,
                "worker deadline"
            );
            Ok(())
        })?
        .context("kernel declined after successful static planning")?;
    ensure!(
        serde_json::to_value(&statistics.plan)? == serde_json::to_value(&plan)?,
        "kernel plan changed"
    );
    metadata["segmentation_call_seconds"] = json!(operation.elapsed().as_secs_f64());
    metadata["statistics"] = json!(statistics);
    output_cost(&r, &input.p, input.x, &plan)?;
    metadata["phase"] = json!("signed_lc_adapter_and_output_checks");
    progress(&args.output, &metadata)?;
    let adapter = Instant::now();
    let result = if input.discriminant {
        let (d, relation) = discriminant(&input.p, input.x, r)?;
        metadata["discriminant_adapter"] = relation;
        d
    } else {
        r
    };
    let (bits, byte_bound) = output_cost(&result, &input.p, input.x, &plan)?;
    metadata["adapter_and_output_check_seconds"] = json!(adapter.elapsed().as_secs_f64());
    metadata["result_terms"] = json!(result.nterms());
    metadata["result_degrees"] = json!(
        (0..result.nvars())
            .map(|i| result.degree(i))
            .collect::<Vec<_>>()
    );
    metadata["result_logical_coefficient_bits"] = json!(bits);
    metadata["candidate_byte_bound"] = json!(byte_bound);
    metadata["deterministic_kernel_identity_checked"] = json!(true);
    metadata["phase"] = json!("bounded_sparse_candidate_export");
    progress(&args.output, &metadata)?;
    authenticated(
        &read_bounded(&args.capture, CAPTURE_BYTES)?,
        &args.capture_sha256,
    )?;
    let (candidate_hash, actual_bytes) = export(&args.output, &metadata, &result, byte_bound)?;
    authenticated(
        &read_bounded(&args.capture, CAPTURE_BYTES)?,
        &args.capture_sha256,
    )?;
    metadata["phase"] = json!("complete");
    metadata["status"] = json!("exact_kernel_candidate_complete");
    metadata["algebra_complete"] = json!(true);
    metadata["candidate_sha256"] = json!(candidate_hash);
    metadata["candidate_bytes"] = json!(actual_bytes);
    metadata["worker_seconds"] = json!(started.elapsed().as_secs_f64());
    create_json(&args.output.join("worker.json"), &metadata)?;
    progress(&args.output, &metadata)?;
    Ok(())
}

fn process_record(outcome: &symgcad::benchmark::ProcessOutcome) -> Value {
    json!({"status":outcome.status,"wall_time_secs":outcome.wall_time_secs,
        "user_cpu_secs":outcome.user_cpu_secs,"system_cpu_secs":outcome.system_cpu_secs,
        "peak_rss_kib":outcome.peak_rss_kib,"exit_code":outcome.exit_code,
        "output_truncated":outcome.output_truncated,
        "stdout_file":"worker.stdout","stdout_bytes":outcome.stdout.len(),"stdout_sha256":hash(outcome.stdout.as_bytes()),
        "stderr_file":"worker.stderr","stderr_bytes":outcome.stderr.len(),"stderr_sha256":hash(outcome.stderr.as_bytes())})
}
fn preserve_output(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 7200.0,
        "seconds must be in (0,7200]"
    );
    ensure!(
        (1..=51200).contains(&args.memory_mib),
        "memory must be in1..51200MiB"
    );
    if args.worker {
        return worker(&args);
    }
    // The parent only hashes bounded bytes; it never constructs huge polynomials.
    authenticated(
        &read_bounded(&args.capture, CAPTURE_BYTES)?,
        &args.capture_sha256,
    )?;
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&args.output).context("output must be a fresh directory")?;
    let binary = std::env::current_exe()?;
    let binary_hash = file_hash(&binary, MAX_CANDIDATE_BYTES)?;
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
    if args.plan_only {
        command.arg("--plan-only");
    }
    create_json(
        &args.output.join("launch.json"),
        &json!({"schema_version":1,
        "binary_sha256":binary_hash,"capture_sha256":args.capture_sha256,
        "seconds":args.seconds,"memory_mib":args.memory_mib,
        "arguments":command.get_args().map(|a|a.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        "original_coverage_claimed":false,"cache_insertion":false}),
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
    // Persist terminal status before any fallible audit, even on metadata failure.
    let terminal = process_record(&outcome);
    create_json(
        &args.output.join("process.json"),
        &json!({"schema_version":1,"process":terminal,
        "binary_sha256":binary_hash,"capture_sha256":args.capture_sha256,"post_run_audit_pending":true}),
    )?;
    preserve_output(
        &args.output.join("worker.stdout"),
        outcome.stdout.as_bytes(),
    )?;
    preserve_output(
        &args.output.join("worker.stderr"),
        outcome.stderr.as_bytes(),
    )?;
    let unchanged = hash(&read_bounded(&args.capture, CAPTURE_BYTES)?) == args.capture_sha256
        && file_hash(&binary, MAX_CANDIDATE_BYTES)? == binary_hash;
    let completed = outcome.status == "completed"
        && outcome.exit_code == Some(0)
        && !outcome.output_truncated
        && unchanged;
    let result = if completed {
        Some(serde_json::from_slice::<Value>(&read_bounded(
            &args.output.join("worker.json"),
            SMALL_JSON_BYTES,
        )?)?)
    } else {
        None
    };
    if let Some(result) = &result {
        ensure!(
            result["capture_sha256"] == args.capture_sha256,
            "worker capture binding"
        );
        if result["algebra_complete"] == true {
            ensure!(
                result["candidate_sha256"]
                    == file_hash(&args.output.join("candidate.json"), MAX_CANDIDATE_BYTES)?,
                "candidate hash mismatch"
            );
        }
    }
    create_json(
        &args.output.join("audit.json"),
        &json!({"schema_version":1,"process":terminal,
        "binary_sha256":binary_hash,"capture_sha256":args.capture_sha256,"input_binary_unchanged":unchanged,
        "worker":result,"independent_algebra_verification":false,"original_coverage_claimed":false,"cache_insertion":false}),
    )?;
    ensure!(
        completed,
        "worker incomplete or failed; inspect preserved process/progress"
    );
    println!(
        "{}",
        json!({"status":result.as_ref().and_then(|v|v.get("status")),"output":args.output,"original_coverage_claimed":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn algebra(names: &[&str]) -> Algebra {
        Algebra::new(&names.iter().map(|n| n.to_string()).collect::<Vec<_>>()).unwrap()
    }
    fn small_options() -> Options {
        Options {
            min_degree_product: 1,
            min_input_terms: 0,
            ..options()
        }
    }
    fn compare(p: &Poly, q: &Poly, x: usize) -> Result<(Poly, Plan)> {
        let (actual, statistics) =
            resultant_segmentation::try_resultant(p, q, x, &small_options(), || Ok(()))?
                .context("small exact kernel declined")?;
        assert_eq!(statistics.plan.parameters.len(), 3);
        let ordinary = p
            .to_univariate(x)
            .resultant_ducos_integer(&q.to_univariate(x));
        let ordinary = if ordinary.is_constant() {
            p.constant(ordinary.get_constant())
        } else {
            ordinary
        };
        assert_eq!(actual, ordinary);
        assert_eq!(actual.variables(), p.variables());
        output_cost(&actual, p, x, &statistics.plan)?;
        Ok((actual, statistics.plan))
    }
    #[test]
    fn genuine_three_parameter_pairs_signed_rational_and_permuted_full_maps() -> Result<()> {
        for (names, x) in [
            (["unused", "a", "x", "c", "b"], 2),
            (["x", "b", "unused", "a", "c"], 0),
        ] {
            let a = algebra(&names);
            let p = a.parse("(-2/3)*(a*x^3+b*x+c+1/7)")?;
            // The second scale is raised to deg(p)=3: its negative sign must survive.
            let q = a.parse("(-5/11)*(c*x^2+(a-b)*x+b)")?;
            let (r, plan) = compare(&p, &q, x)?;
            assert!(!r.is_zero());
            let mut serialized = Vec::new();
            sparse(&mut serialized, &r)?;
            let parsed: Value = serde_json::from_slice(&serialized)?;
            for term in parsed["terms"].as_array().unwrap() {
                assert_eq!(term["exponents"].as_array().unwrap().len(), 5);
                assert_eq!(term["exponents"][x], 0);
            }
            let (_, bytes) = output_cost(&r, &p, x, &plan)?;
            assert!(serialized.len() as u64 <= bytes);
        }
        Ok(())
    }
    #[test]
    fn signed_discriminants_keep_parameter_lc_degree_drop_and_repeated_loci() -> Result<()> {
        let a = algebra(&["unused", "a", "x", "b", "c"]);
        for n in [3u32, 4] {
            let p = a.parse(&format!("(-2/3)*((a-b)*x^{n}+c*x+b)"))?;
            let (r, plan) = compare(&p, &p.derivative(2), 2)?;
            let (d, relation) = discriminant(&p, 2, r)?;
            assert_eq!(relation["negative_sign"], n == 3);
            assert_eq!(d, symgcad::projection::discriminant(&p, 2)?);
            output_cost(&d, &p, 2, &plan)?;
            // Formal discriminant identity survives the LC degree drop.
            assert!(d.replace(1, &1.into()).replace(3, &1.into()).is_zero());
        }
        Ok(())
    }
    #[test]
    fn shared_factor_zero_retains_full_map_and_exact_lc_adapter() -> Result<()> {
        let a = algebra(&["a", "unused", "b", "x", "c"]);
        let p = a.parse("(-7/13)*((x+a)*(x+b+c))^2")?;
        let (r, plan) = compare(&p, &p.derivative(3), 3)?;
        assert!(r.is_zero());
        let (d, relation) = discriminant(&p, 3, r)?;
        assert!(d.is_zero());
        assert_eq!(d.variables(), p.variables());
        assert_eq!(relation["remainder_zero"], true);
        output_cost(&d, &p, 3, &plan)?;
        // A forged resultant is rejected instead of silently truncating a quotient.
        let p = a.parse("a*x^3+b*x+c")?;
        assert!(discriminant(&p, 3, p.one()).is_err());
        Ok(())
    }
    fn capture(a: &Algebra, p: &Poly, x: usize) -> Result<OperationCapture> {
        let operands = vec![a.format(p)];
        Ok(OperationCapture {
            schema_version: 1,
            backend_version: "test".into(),
            coefficient_domain: "Q".into(),
            variable_names: a.names.clone(),
            operation: AlgebraOperation::Discriminant,
            variable: Some(x),
            key: "research-only".into(),
            input_hash: hash(&serde_json::to_vec(&operands)?),
            operands: Some(operands),
            operands_omitted_for_size: false,
            complete: false,
            elapsed_seconds: None,
            output_hash: None,
        })
    }
    #[test]
    fn bounded_capture_authentication_and_sparse_parser_reject_tampering() -> Result<()> {
        let a = algebra(&["a", "b", "c", "x"]);
        let p = a.parse("a*x^3+b*x+c")?;
        let c = capture(&a, &p, 3)?;
        let bytes = serde_json::to_vec(&c)?;
        let input = load(&bytes, &hash(&bytes))?;
        assert_eq!(input.q, p.derivative(3));
        assert!(load(&bytes, &"0".repeat(64)).is_err());
        for mutation in 0..5 {
            let mut bad = c.clone();
            match mutation {
                0 => bad.input_hash = "0".repeat(64),
                1 => bad.variable_names[0] = "a".repeat(65),
                2 => bad.variable_names[0] = "b".into(),
                3 => bad.variable = Some(4),
                _ => {
                    bad.operands = Some(vec!["(1)*(a+x)^9999999".into()]);
                    bad.input_hash = hash(&serde_json::to_vec(bad.operands.as_ref().unwrap())?);
                }
            }
            let bytes = serde_json::to_vec(&bad)?;
            assert!(load(&bytes, &hash(&bytes)).is_err(), "mutation {mutation}");
        }
        assert!(capture_parser::plan_operand(&a, "(1/0)*x^3").is_err());
        assert!(capture_parser::plan_operand(&a, &format!("({})*x^3", "9".repeat(1235))).is_err());
        Ok(())
    }
    #[test]
    fn kernel_and_rational_retention_caps_decline_before_arithmetic() -> Result<()> {
        let a = algebra(&["a", "b", "c", "x"]);
        let p = a.parse("(a*x^3+b*x+c)/(2^1000+1)")?;
        let c = capture(&a, &p, 3)?;
        let bytes = serde_json::to_vec(&c)?;
        let input = load(&bytes, &hash(&bytes))?;
        let plan = resultant_segmentation::plan(&p, &input.q, 3, &small_options())?.unwrap();
        restoration_preflight(&input, &plan)?;
        let mut too_many = plan.clone();
        too_many.digits = MAX_DIGITS;
        assert!(restoration_preflight(&input, &too_many).is_err());
        for limit in 0..4 {
            let mut cap = small_options();
            match limit {
                0 => cap.max_parameters = 2,
                1 => cap.max_digits = 1,
                2 => cap.max_total_input_bits = 1,
                _ => cap.max_output_bits = 1,
            };
            assert!(resultant_segmentation::plan(&p, &input.q, 3, &cap)?.is_none());
        }
        assert!(worst_sparse_bytes(usize::MAX, u64::MAX, u64::MAX, &[4096; 128]).is_err());
        Ok(())
    }
    #[test]
    fn bounded_stream_preserves_signed_scale_full_exponents_and_byte_limit() -> Result<()> {
        let a = algebra(&["a", "b", "c", "x", "unused"]);
        let p = a.parse("-13/17*a^2+2/3*b-c")?;
        let mut actual = Vec::new();
        sparse(&mut actual, &p)?;
        let parsed: Value = serde_json::from_slice(&actual)?;
        let mut replay = p.zero();
        for term in parsed["terms"].as_array().unwrap() {
            let coefficient = a
                .parse(term["coefficient"].as_str().unwrap())?
                .get_constant();
            let exponents = serde_json::from_value::<Vec<u32>>(term["exponents"].clone())?;
            replay.append_monomial(coefficient, &exponents);
        }
        assert_eq!(replay, p);
        let mut bounded = LimitedWriter {
            inner: Vec::new(),
            hash: Sha256::new(),
            bytes: 0,
            cap: actual.len() as u64 - 1,
        };
        assert!(sparse(&mut bounded, &p).is_err());
        let mut good = LimitedWriter {
            inner: Vec::new(),
            hash: Sha256::new(),
            bytes: 0,
            cap: actual.len() as u64,
        };
        sparse(&mut good, &p)?;
        assert_eq!(good.inner, actual);
        assert_eq!(format!("{:x}", good.hash.finalize()), hash(&actual));
        Ok(())
    }
}
