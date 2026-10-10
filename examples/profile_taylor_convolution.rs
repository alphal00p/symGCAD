//! Research-only integer Taylor shifts by bounded exact convolution.
//! Every completed experiment compares all coefficients with native shift_var.
#[path = "support/taylor_recursive.rs"]
mod recursive;
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, rational},
    benchmark::{ProcessOutcome, run_bounded},
};

const MAX_DEGREE: usize = 4096;
const MAX_HEIGHT: u64 = 20_000;
const MAX_SHIFT_HEIGHT: u64 = 256;
const MAX_INPUT_BITS: u64 = 64_000_000;
const MAX_FACTORIAL_BITS: u64 = 65_536;
const MAX_PACKED_BITS: u64 = 256_000_000;
const MAX_LOGICAL_BITS: u64 = 2_000_000_000;
const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_METADATA: u64 = 1024 * 1024;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    input_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "1", allow_hyphen_values = true)]
    shift: String,
    /// Explicit research policy; does not increase any storage or work bound.
    #[arg(long, default_value_t = MAX_HEIGHT)]
    input_height_bits: u64,
    /// Compare the recursive method with native leaves of this maximum length.
    #[arg(long, conflicts_with = "plan_only")]
    recursive_leaf: Option<usize>,
    /// Compare direct triangular additions/subtractions for shifts +1 or -1.
    #[arg(long, conflicts_with_all = ["recursive_leaf", "plan_only"])]
    unit_add: bool,
    #[arg(long, default_value_t = 3)]
    cpu: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long)]
    plan_only: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Term {
    coefficient: String,
    exponents: Vec<usize>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u32,
    variables: Vec<String>,
    axis: usize,
    terms: Vec<Term>,
}
#[derive(Debug, Clone, Serialize)]
struct Plan {
    degree: usize,
    input_height_bits: u64,
    input_height_limit_bits: u64,
    factorial_height_bound: u64,
    digit_bits_bound: u64,
    encoded_output_bits_bound: u64,
    encoded_inputs_bits_bound: u64,
    /// Conservative logical buffers/scratch reservation, not measured process RAM.
    logical_retained_bits_bound: u64,
    output_text_bytes_bound: u64,
}
#[derive(Default, Serialize)]
struct Timing {
    factorial_and_weights_seconds: f64,
    packing_seconds: f64,
    multiplication_seconds: f64,
    decode_and_roundtrip_seconds: f64,
    exact_division_seconds: f64,
    total_seconds: f64,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= cap,
        "regular bounded input required"
    );
    let mut bytes = Vec::new();
    (&mut file).take(cap + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= cap, "input grew beyond cap");
    Ok(bytes)
}
fn file_hash(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn save(path: &Path, value: &Value, replace: bool, cap: u64) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() as u64 <= cap, "output byte cap");
    let tmp = path.with_extension("tmp");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    drop(f);
    if replace {
        fs::rename(tmp, path)?;
    } else {
        fs::hard_link(&tmp, path)?;
        fs::remove_file(tmp)?;
    }
    Ok(())
}
fn integer_literal(text: &str, bits: u64) -> Result<Integer> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let decimal_cap = bits.checked_mul(30103).context("decimal cap overflow")? / 100000 + 1;
    ensure!(
        !digits.is_empty()
            && digits.len() as u64 <= decimal_cap
            && digits.bytes().all(|b| b.is_ascii_digit()),
        "bounded integer literal required"
    );
    let r = rational(text)?;
    ensure!(
        r.denominator_ref().is_one() && r.numerator_ref().significant_bits() <= bits,
        "integer height cap"
    );
    Ok(r.numerator_ref().clone())
}
fn height_policy(bits: u64) -> Result<()> {
    ensure!(
        matches!(bits, 20_000 | 32_768 | 65_536),
        "input-height policy must be 20000, 32768, or 65536"
    );
    Ok(())
}
fn recursive_policy(leaf: Option<usize>) -> Result<()> {
    ensure!(
        leaf.is_none_or(|n| n.is_power_of_two() && (8..=512).contains(&n)),
        "recursive leaf must be a power of two from 8 through 512"
    );
    Ok(())
}
fn method(leaf: Option<usize>, unit_add: bool) -> &'static str {
    if unit_add {
        "triangular_unit_add_or_subtract"
    } else if leaf.is_some() {
        "recursive_cached_binomial_convolution"
    } else {
        "factorial_convolution"
    }
}
#[cfg(test)]
fn input(bytes: &[u8], expected: &str) -> Result<(Input, UnivariatePolynomial<Z>)> {
    input_with_height(bytes, expected, MAX_HEIGHT)
}
fn input_with_height(
    bytes: &[u8],
    expected: &str,
    height_limit: u64,
) -> Result<(Input, UnivariatePolynomial<Z>)> {
    height_policy(height_limit)?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES && expected.len() == 64 && hash(bytes) == expected,
        "input hash/byte binding"
    );
    let i: Input = serde_json::from_slice(bytes)?;
    ensure!(
        i.schema_version == 1
            && !i.variables.is_empty()
            && i.variables.len() <= 16
            && i.axis < i.variables.len()
            && i.terms.len() <= MAX_DEGREE + 1,
        "input schema/map/term cap"
    );
    let mut names = BTreeSet::new();
    for name in &i.variables {
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name.as_bytes()[0].is_ascii_alphabetic()
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && names.insert(name),
            "distinct bounded variable names"
        );
    }
    // Reject all malformed shapes/literals before constructing any big integer
    // or registering a Symbolica variable. JSON itself is byte bounded above.
    let mut previous = None;
    let mut decimal_bits = 0u64;
    let decimal_cap = height_limit * 30103 / 100000 + 1;
    for t in &i.terms {
        ensure!(
            t.exponents.len() == i.variables.len()
                && t.exponents.iter().enumerate().all(|(j, e)| if j == i.axis {
                    *e <= MAX_DEGREE
                } else {
                    *e == 0
                }),
            "full univariate support required"
        );
        let e = t.exponents[i.axis];
        ensure!(
            previous.is_none_or(|p| p < e),
            "strictly ordered unique exponents"
        );
        previous = Some(e);
        let d = t.coefficient.strip_prefix('-').unwrap_or(&t.coefficient);
        ensure!(
            !d.is_empty() && d.len() as u64 <= decimal_cap && d.bytes().all(|b| b.is_ascii_digit()),
            "coefficient literal preflight"
        );
        decimal_bits = decimal_bits
            .checked_add(
                (d.len() as u64)
                    .checked_mul(4)
                    .context("input bits overflow")?,
            )
            .context("input sum overflow")?;
        ensure!(decimal_bits <= MAX_INPUT_BITS, "input storage cap");
    }
    let mut coefficients = vec![Integer::zero(); previous.map_or(0, |e| e + 1)];
    for t in &i.terms {
        let c = integer_literal(&t.coefficient, height_limit)?;
        ensure!(!c.is_zero(), "sparse zero term");
        coefficients[t.exponents[i.axis]] = c;
    }
    let a = Algebra::new(&i.variables)?;
    let p = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(a.variables[i.axis].clone()),
    );
    Ok((i, p))
}
fn ceil_log2(n: u64) -> u64 {
    if n <= 1 {
        0
    } else {
        64 - (n - 1).leading_zeros() as u64
    }
}
#[cfg(test)]
fn plan(p: &UnivariatePolynomial<Z>, a: &Integer) -> Result<Plan> {
    plan_with_height(p, a, MAX_HEIGHT)
}
fn plan_with_height(p: &UnivariatePolynomial<Z>, a: &Integer, height_limit: u64) -> Result<Plan> {
    height_policy(height_limit)?;
    let n = if p.is_zero() { 0 } else { p.degree() };
    ensure!(
        n <= MAX_DEGREE && a.significant_bits() <= MAX_SHIFT_HEIGHT,
        "degree/shift cap"
    );
    let height = p
        .coefficients()
        .iter()
        .map(Integer::significant_bits)
        .max()
        .unwrap_or(0);
    ensure!(
        height <= height_limit
            && p.coefficients()
                .iter()
                .map(Integer::significant_bits)
                .sum::<u64>()
                <= MAX_INPUT_BITS,
        "input coefficient cap"
    );
    let factorial = (2..=n as u64).try_fold(1u64, |b, k| {
        b.checked_add(ceil_log2(k))
            .context("factorial bound overflow")
    })?;
    ensure!(factorial <= MAX_FACTORIAL_BITS, "factorial cap");
    let shift_power = if a.is_zero() || a.abs().is_one() {
        0
    } else {
        (n as u64)
            .checked_mul(a.significant_bits())
            .context("power bound overflow")?
    };
    let digit = height
        .checked_add(2 * factorial)
        .and_then(|v| v.checked_add(shift_power))
        .and_then(|v| v.checked_add(2 * ceil_log2(n as u64 + 1) + 2))
        .context("digit bound overflow")?;
    let output = (2 * n as u64 + 1)
        .checked_mul(digit)
        .context("output overflow")?;
    let inputs = (2 * n as u64 + 2)
        .checked_mul(digit)
        .context("input encoding overflow")?;
    // Reserve twelve output-sized buffers for vectors, packed values and
    // balanced recursion, plus explicit factorial/input scratch. Native GMP
    // transient allocation is additionally bounded by the external worker.
    let logical = output
        .checked_mul(12)
        .and_then(|v| v.checked_add(6 * (n as u64 + 1) * factorial))
        .and_then(|v| v.checked_add(2 * (n as u64 + 1) * height))
        .context("logical storage overflow")?;
    // |q_i| <= ||p||_1 (1+|a|)^n. One extra bit bounds 1+|a|.
    let q_height = height + ceil_log2(n as u64 + 1) + (n as u64) * (a.significant_bits() + 1) + 1;
    let text = (n as u64 + 1)
        .checked_mul(q_height * 30103 / 100000 + 512)
        .and_then(|v| v.checked_add(4096))
        .context("text bound overflow")?;
    ensure!(
        output <= MAX_PACKED_BITS
            && inputs <= MAX_PACKED_BITS
            && logical <= MAX_LOGICAL_BITS
            && text <= MAX_BYTES,
        "packed/logical/output text cap"
    );
    Ok(Plan {
        degree: n,
        input_height_bits: height,
        input_height_limit_bits: height_limit,
        factorial_height_bound: factorial,
        digit_bits_bound: digit,
        encoded_output_bits_bound: output,
        encoded_inputs_bits_bound: inputs,
        logical_retained_bits_bound: logical,
        output_text_bytes_bound: text,
    })
}
// Force arbitrary precision shifts, including for one, on pristine Symbolica.
fn shl(v: Integer, bits: usize) -> Integer {
    Integer::from(v.to_multi_prec() << bits)
}
fn pack(values: &[Integer], bits: usize) -> Integer {
    match values.len() {
        0 => Integer::zero(),
        1 => values[0].clone(),
        n => {
            let k = n / 2;
            pack(&values[..k], bits) + shl(pack(&values[k..], bits), k * bits)
        }
    }
}
fn unpack(value: Integer, values: &mut [Integer], bits: usize, bound: &Integer) -> Result<()> {
    if values.len() == 1 {
        ensure!(
            value.abs() <= *bound,
            "decoded coefficient exceeds norm bound"
        );
        values[0] = value;
        return Ok(());
    }
    let k = values.len() / 2;
    let radix = shl(Integer::one(), k * bits);
    let (mut high, mut low) = value.quot_rem(&radix);
    ensure!(
        low >= 0 && low < radix,
        "expected Euclidean positive-radix remainder"
    );
    if &low + &low >= radix {
        low -= &radix;
        high += Integer::one();
    }
    unpack(low, &mut values[..k], bits, bound)?;
    unpack(high, &mut values[k..], bits, bound)
}
fn quotient(value: &Integer, divisor: &Integer) -> Result<Integer> {
    let (q, r) = value.quot_rem(divisor);
    ensure!(r.is_zero(), "factorial quotient is not integral");
    Ok(q)
}
#[cfg(test)]
fn fast_shift(
    p: &UnivariatePolynomial<Z>,
    a: &Integer,
    event: impl FnMut(&str, &Value) -> Result<()>,
) -> Result<(UnivariatePolynomial<Z>, Value)> {
    fast_shift_with_height(p, a, MAX_HEIGHT, event)
}
fn fast_shift_with_height(
    p: &UnivariatePolynomial<Z>,
    a: &Integer,
    height_limit: u64,
    mut event: impl FnMut(&str, &Value) -> Result<()>,
) -> Result<(UnivariatePolynomial<Z>, Value)> {
    let started = Instant::now();
    let policy = plan_with_height(p, a, height_limit)?;
    let n = policy.degree;
    if p.is_zero() {
        return Ok((p.clone(), json!({"plan":policy,"zero_input":true})));
    }
    let mut timing = Timing::default();
    let step = Instant::now();
    event("factorial_and_weights", &json!({"plan":policy}))?;
    let mut factorials = Vec::with_capacity(n + 1);
    factorials.push(Integer::one());
    for j in 1..=n {
        factorials.push(&factorials[j - 1] * Integer::from(j as i64));
    }
    ensure!(
        factorials[n].significant_bits() <= policy.factorial_height_bound,
        "factorial bound failed"
    );
    let left = (0..=n)
        .rev()
        .map(|j| &p.coefficients()[j] * &factorials[j])
        .collect::<Vec<_>>();
    let mut right = Vec::with_capacity(n + 1);
    right.push(factorials[n].clone());
    for j in 1..=n {
        right.push(quotient(&(&right[j - 1] * a), &Integer::from(j as i64))?);
    }
    let norm = |v: &[Integer]| v.iter().fold(Integer::zero(), |s, c| s + c.abs());
    let bound = norm(&left) * norm(&right);
    let digit_bits = bound.significant_bits() as usize + 1;
    ensure!(
        digit_bits as u64 <= policy.digit_bits_bound,
        "exact norm exceeded planned bound"
    );
    timing.factorial_and_weights_seconds = step.elapsed().as_secs_f64();
    let step = Instant::now();
    event("balanced_packing", &json!({"digit_bits":digit_bits}))?;
    let l = pack(&left, digit_bits);
    let r = pack(&right, digit_bits);
    ensure!(
        l.significant_bits() + r.significant_bits() <= policy.encoded_inputs_bits_bound,
        "encoded input storage bound"
    );
    drop(left);
    drop(right);
    timing.packing_seconds = step.elapsed().as_secs_f64();
    let step = Instant::now();
    event(
        "integer_multiplication",
        &json!({"left_bits":l.significant_bits(),"right_bits":r.significant_bits()}),
    )?;
    let product = l * r;
    ensure!(
        product.significant_bits() <= policy.encoded_output_bits_bound,
        "encoded output bound"
    );
    timing.multiplication_seconds = step.elapsed().as_secs_f64();
    let step = Instant::now();
    event(
        "balanced_decode_and_roundtrip",
        &json!({"product_bits":product.significant_bits()}),
    )?;
    let mut convolution = vec![Integer::zero(); 2 * n + 1];
    unpack(product.clone(), &mut convolution, digit_bits, &bound)?;
    ensure!(
        pack(&convolution, digit_bits) == product,
        "complete convolution reencoding failed"
    );
    timing.decode_and_roundtrip_seconds = step.elapsed().as_secs_f64();
    let step = Instant::now();
    event("exact_factorial_divisions", &json!({}))?;
    let coefficients = (0..=n)
        .map(|i| quotient(&convolution[n - i], &(&factorials[n] * &factorials[i])))
        .collect::<Result<Vec<_>>>()?;
    let out = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(p.one().to_multivariate::<u32>().variables()[0].clone()),
    );
    ensure!(
        out.degree() == n && out.lcoeff() == p.lcoeff(),
        "shift changed leading coefficient/degree"
    );
    timing.exact_division_seconds = step.elapsed().as_secs_f64();
    timing.total_seconds = started.elapsed().as_secs_f64();
    Ok((
        out,
        json!({"plan":policy,"actual_digit_bits":digit_bits,"encoded_product_bits":product.significant_bits(),"timing":timing,"complete_signed_decode_roundtrip":true,"factorial_divisions_exact":true}),
    ))
}
fn polynomial_record(i: &Input, p: &UnivariatePolynomial<Z>) -> Input {
    let terms = p
        .coefficients()
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_zero())
        .map(|(e, c)| {
            let mut exponents = vec![0; i.variables.len()];
            exponents[i.axis] = e;
            Term {
                coefficient: c.to_string(),
                exponents,
            }
        })
        .collect();
    Input {
        schema_version: 1,
        variables: i.variables.clone(),
        axis: i.axis,
        terms,
    }
}
fn process_record(process: &ProcessOutcome) -> Value {
    json!({"status":process.status,"wall_time_secs":process.wall_time_secs,
        "user_cpu_secs":process.user_cpu_secs,"system_cpu_secs":process.system_cpu_secs,
        "peak_rss_kib":process.peak_rss_kib,"exit_code":process.exit_code,
        "output_truncated":process.output_truncated,
        "stdout_bytes":process.stdout.len(),"stdout_sha256":hash(process.stdout.as_bytes()),
        "stderr_bytes":process.stderr.len(),"stderr_sha256":hash(process.stderr.as_bytes())})
}
fn worker(args: &Args) -> Result<()> {
    let started = Instant::now();
    let bytes = read(&args.input, MAX_BYTES)?;
    let (i, p) = input_with_height(&bytes, &args.input_sha256, args.input_height_bits)?;
    let a = integer_literal(&args.shift, MAX_SHIFT_HEIGHT)?;
    let policy = plan_with_height(&p, &a, args.input_height_bits)?;
    let mut progress = |phase: &str, details: &Value| {
        ensure!(
            started.elapsed().as_secs_f64() < args.seconds,
            "worker deadline"
        );
        save(
            &args.output.join("progress.json"),
            &json!({"phase":phase,"details":details,"elapsed_seconds":started.elapsed().as_secs_f64(),"complete":false}),
            true,
            MAX_METADATA,
        )
    };
    progress("static_plan", &json!(policy))?;
    if args.plan_only {
        save(
            &args.output.join("worker.json"),
            &json!({"complete":true,"plan_only":true,"plan":policy,"algebra_computed":false,"input_sha256":args.input_sha256,"original_cad_coverage":false}),
            false,
            MAX_METADATA,
        )?;
        return Ok(());
    }
    // Keep the same conservative factorial envelope for both comparisons.
    // The recursive implementation additionally checks its own storage bounds.
    let (fast, stats) = if args.unit_add {
        progress("triangular_unit_shift_composite", &json!({}))?;
        recursive::unit_shift(&p, &a)?
    } else if let Some(leaf) = args.recursive_leaf {
        progress("recursive_shift_composite", &json!({"leaf":leaf}))?;
        recursive::recursive_shift(&p, &a, leaf)?
    } else {
        fast_shift_with_height(&p, &a, args.input_height_bits, &mut progress)?
    };
    progress("native_shift_var", &stats)?;
    let step = Instant::now();
    let native = p.shift_var(&a);
    let native_seconds = step.elapsed().as_secs_f64();
    ensure!(
        native == fast && native.coefficients() == fast.coefficients(),
        "native/full coefficient comparison failed"
    );
    progress("saving_checked_coefficients", &stats)?;
    let coefficients = serde_json::to_value(polynomial_record(&i, &fast))?;
    save(
        &args.output.join("shifted.json"),
        &coefficients,
        false,
        MAX_BYTES,
    )?;
    save(
        &args.output.join("worker.json"),
        &json!({"complete":true,"plan_only":false,"research_policy":"bounded-taylor-v2-explicit-input-height","method":method(args.recursive_leaf,args.unit_add),"recursive_leaf":args.recursive_leaf,"unit_add":args.unit_add,"shared_preflight":"conservative_factorial_envelope","plan":policy,"input_height_limit_bits":args.input_height_bits,"input_sha256":args.input_sha256,"variables":i.variables,"axis":i.axis,"shift":a.to_string(),"degree":policy.degree,"statistics":stats,"native_shift_seconds":native_seconds,"all_coefficients_equal":true,"shifted_sha256":file_hash(&args.output.join("shifted.json"))?,"pipeline_seconds":started.elapsed().as_secs_f64(),"original_cad_coverage":false,"root_isolation_computed":false,"production_modified":false}),
        false,
        MAX_METADATA,
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    height_policy(args.input_height_bits)?;
    recursive_policy(args.recursive_leaf)?;
    if args.unit_add {
        let shift = integer_literal(&args.shift, MAX_SHIFT_HEIGHT)?;
        ensure!(
            shift.is_one() || shift == Integer::from(-1),
            "unit shift requires +1 or -1"
        );
    }
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 60.,
        "deadline (0,60]"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(!args.output.exists(), "fresh output required");
    ensure!(
        hash(&read(&args.input, MAX_BYTES)?) == args.input_sha256,
        "input hash binding"
    );
    integer_literal(&args.shift, MAX_SHIFT_HEIGHT)?;
    fs::create_dir_all(&args.output)?;
    let exe = std::env::current_exe()?;
    let binary_hash = file_hash(&exe)?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 2048,
        workers: 1,
        max_cells: None,
    };
    let process = run_bounded(&mut command, &limits)?;
    let terminal = process_record(&process);
    save(
        &args.output.join("process.json"),
        &terminal,
        false,
        MAX_METADATA,
    )?;
    for (name, bytes) in [
        ("worker.stdout", process.stdout.as_bytes()),
        ("worker.stderr", process.stderr.as_bytes()),
    ] {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(args.output.join(name))?;
        file.write_all(bytes)?;
    }
    let unchanged = read(&args.input, MAX_BYTES).is_ok_and(|b| hash(&b) == args.input_sha256)
        && file_hash(&exe).is_ok_and(|h| h == binary_hash);
    let result = read(&args.output.join("worker.json"), MAX_METADATA)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let complete = unchanged
        && process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.as_ref().is_some_and(|r| {
            r["complete"] == true
                && (args.plan_only
                    || r["all_coefficients_equal"] == true
                        && file_hash(&args.output.join("shifted.json"))
                            .is_ok_and(|h| r["shifted_sha256"] == h))
        });
    save(
        &args.output.join("report.json"),
        &json!({"complete":complete,"process":terminal,"limits":limits,"cpu":args.cpu,"research_policy":"bounded-taylor-v2-explicit-input-height","method":method(args.recursive_leaf,args.unit_add),"recursive_leaf":args.recursive_leaf,"unit_add":args.unit_add,"input_height_limit_bits":args.input_height_bits,"binary_sha256":binary_hash,"input_sha256":args.input_sha256,"inputs_unchanged":unchanged,"result":result,"original_cad_coverage":false}),
        false,
        MAX_METADATA,
    )?;
    ensure!(
        complete,
        "bounded shift experiment incomplete; terminal evidence retained"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn poly(c: &[i64], variable: &str) -> UnivariatePolynomial<Z> {
        let a = Algebra::new(&[variable.into()]).unwrap();
        UnivariatePolynomial::from_coefficients(
            &Z,
            c.iter().map(|c| Integer::from(*c)).collect(),
            Arc::new(a.variables[0].clone()),
        )
    }
    #[test]
    fn full_coefficients_signed_shifts_zeros_and_endpoint_roots() {
        for c in [
            vec![],
            vec![0],
            vec![-7],
            vec![0, -1, 1],
            vec![-6, 11, -6, 1],
            vec![1, 0, 0, 0, 0, -9],
            vec![0, 0, 0, 3, -5, 2],
        ] {
            let p = poly(&c, "lambda_7");
            for shift in [-3, -1, 0, 1, 2, 7] {
                let a = Integer::from(shift);
                let (q, _) = fast_shift(&p, &a, |_, _| Ok(())).unwrap();
                assert_eq!(q, p.shift_var(&a));
            }
        }
    }
    #[test]
    fn centered_balanced_roundtrip_and_bad_quotients() {
        let v = [-31, 0, 31, -1, 7].map(Integer::from);
        let b = Integer::from(31);
        let x = pack(&v, 6);
        let mut out = vec![Integer::zero(); v.len()];
        unpack(x.clone(), &mut out, 6, &b).unwrap();
        assert_eq!(out, v);
        assert_eq!(pack(&out, 6), x);
        assert!(unpack(Integer::from(32), &mut [Integer::zero()], 6, &b).is_err());
        assert!(quotient(&Integer::from(7), &Integer::from(3)).is_err());
    }
    #[test]
    fn strict_parser_full_map_hash_and_caps() {
        let mut v = json!({"schema_version":1,"variables":["inactive","time"],"axis":1,"terms":[{"coefficient":"-3","exponents":[0,0]},{"coefficient":"2","exponents":[0,3]}]});
        let parse = |v: &Value| {
            let b = serde_json::to_vec(v).unwrap();
            input(&b, &hash(&b))
        };
        let (i, p) = parse(&v).unwrap();
        assert_eq!(i.axis, 1);
        assert_eq!(p, poly(&[-3, 0, 0, 2], "time"));
        v["terms"][0]["exponents"][0] = json!(1);
        assert!(parse(&v).is_err());
        v["terms"][0]["exponents"][0] = json!(0);
        v["terms"][0]["coefficient"] = json!("2/3");
        assert!(parse(&v).is_err());
        v["terms"][0]["coefficient"] = json!("-3");
        v["variables"][0] = json!("time");
        assert!(parse(&v).is_err());
        assert!(integer_literal("1e3", 20).is_err());
        assert!(integer_literal("256", 8).is_err());
        assert!(input(b"{}", &"0".repeat(64)).is_err());
        let too_high = UnivariatePolynomial::from_coefficients(
            &Z,
            vec![shl(Integer::one(), MAX_HEIGHT as usize)],
            Arc::new(Algebra::new(&["x".into()]).unwrap().variables[0].clone()),
        );
        assert!(plan(&too_high, &Integer::one()).is_err());
        let p = UnivariatePolynomial::from_coefficients(
            &Z,
            vec![Integer::one(); 4097],
            Arc::new(Algebra::new(&["x".into()]).unwrap().variables[0].clone()),
        );
        assert!(plan(&p, &shl(Integer::one(), 255)).is_err());
    }
    #[test]
    fn cancellation_and_rational_input_clearing() {
        let p = poly(&[3, -4, 0, 7], "r");
        assert!(fast_shift(&p, &Integer::one(), |_, _| anyhow::bail!("stopped")).is_err());
        // Positive denominator clearing commutes with an integer shift; this
        // checks signed rational coefficients without broadening worker input.
        let q = UnivariatePolynomial::from_coefficients(
            &Q,
            p.coefficients()
                .iter()
                .map(|c| Rational::from(c.clone()) / Rational::from(6))
                .collect(),
            Arc::new(p.one().to_multivariate::<u32>().variables()[0].clone()),
        );
        let (fast, _) = fast_shift(&p, &Integer::from(-2), |_, _| Ok(())).unwrap();
        let got = UnivariatePolynomial::from_coefficients(
            &Q,
            fast.coefficients()
                .iter()
                .map(|c| Rational::from(c.clone()) / Rational::from(6))
                .collect(),
            Arc::new(p.one().to_multivariate::<u32>().variables()[0].clone()),
        );
        assert_eq!(got, q.shift_var(&Rational::from(-2)));
    }
    #[test]
    fn general_dense_nontrivial_degree_all_coefficients() {
        let c = (0..129)
            .map(|i| ((i * 137) % 97) as i64 - 48)
            .collect::<Vec<_>>();
        let p = poly(&c, "alpha");
        for a in [-2, 1, 3] {
            let a = Integer::from(a);
            assert_eq!(
                fast_shift(&p, &a, |_, _| Ok(())).unwrap().0,
                p.shift_var(&a)
            );
        }
    }
    #[test]
    fn rational_shift_denominator_restoration_identity() {
        let a = Algebra::new(&["energy".into()]).unwrap();
        let variable = Arc::new(a.variables[0].clone());
        let p = UnivariatePolynomial::from_coefficients(
            &Q,
            ["-2/5", "0", "1/3", "-7/6"]
                .map(|c| rational(c).unwrap())
                .to_vec(),
            variable.clone(),
        );
        let e = Integer::from(30);
        let n = p.degree();
        for shift in ["-2/3", "3/2", "0"] {
            let shift = rational(shift).unwrap();
            let d = shift.denominator_ref();
            let clear = p
                .coefficients()
                .iter()
                .enumerate()
                .map(|(j, c)| {
                    let r = c * &Rational::from(&e * d.pow((n - j) as u64));
                    assert!(r.denominator_ref().is_one());
                    r.numerator_ref().clone()
                })
                .collect();
            let integer = UnivariatePolynomial::from_coefficients(&Z, clear, variable.clone());
            let (q, _) = fast_shift(&integer, shift.numerator_ref(), |_, _| Ok(())).unwrap();
            let restore = q
                .coefficients()
                .iter()
                .enumerate()
                .map(|(j, c)| {
                    Rational::from(c.clone()) / Rational::from(&e * d.pow((n - j) as u64))
                })
                .collect();
            assert_eq!(
                UnivariatePolynomial::from_coefficients(&Q, restore, variable.clone()),
                p.shift_var(&shift)
            );
        }
    }
    #[test]
    fn terminal_metadata_is_bounded_with_large_streams() {
        let p = ProcessOutcome {
            status: "timeout".into(),
            wall_time_secs: 60.,
            user_cpu_secs: 59.,
            system_cpu_secs: 1.,
            peak_rss_kib: 99,
            exit_code: None,
            stdout: "x".repeat(2 * 1024 * 1024),
            stderr: String::new(),
            output_truncated: false,
        };
        let record = process_record(&p);
        assert!(serde_json::to_vec(&record).unwrap().len() < 4096);
        assert_eq!(record["stdout_bytes"], 2 * 1024 * 1024);
        assert_eq!(record["stdout_sha256"], hash(p.stdout.as_bytes()));
    }
    #[test]
    fn explicit_height_policy_preserves_default_and_signed_identity() {
        let args = Args::try_parse_from([
            "helper",
            "input",
            "--input-sha256",
            "hash",
            "--output",
            "fresh",
        ])
        .unwrap();
        assert_eq!(args.input_height_bits, MAX_HEIGHT);
        assert_eq!(args.recursive_leaf, None);
        assert!(!args.unit_add);
        assert!(height_policy(20_001).is_err());
        assert!(height_policy(65_537).is_err());
        let high = shl(Integer::one(), 24_000) + Integer::from(7);
        let variable = Arc::new(Algebra::new(&["alpha".into()]).unwrap().variables[0].clone());
        let p = UnivariatePolynomial::from_coefficients(
            &Z,
            vec![
                -high.clone(),
                Integer::zero(),
                high.clone(),
                Integer::from(-3),
            ],
            variable,
        );
        assert!(plan(&p, &Integer::one()).is_err());
        for shift in [-2, 0, 1, 3] {
            let a = Integer::from(shift);
            let (q, stats) = fast_shift_with_height(&p, &a, 32_768, |_, _| Ok(())).unwrap();
            assert_eq!(q, p.shift_var(&a));
            assert_eq!(stats["plan"]["input_height_limit_bits"], 32_768);
        }
        let input_record = Input {
            schema_version: 1,
            variables: vec!["unused".into(), "alpha".into()],
            axis: 1,
            terms: vec![Term {
                coefficient: (-high).to_string(),
                exponents: vec![0, 0],
            }],
        };
        let bytes = serde_json::to_vec(&input_record).unwrap();
        assert!(input(&bytes, &hash(&bytes)).is_err());
        let (i, parsed) = input_with_height(&bytes, &hash(&bytes), 32_768).unwrap();
        assert_eq!(i.variables, input_record.variables);
        assert_eq!(i.axis, 1);
        assert_eq!(parsed.coefficients()[0], p.coefficients()[0]);
    }
    #[test]
    fn raised_height_keeps_exact_boundary_and_storage_caps() {
        let at = shl(Integer::one(), 65_535);
        assert_eq!(integer_literal(&at.to_string(), 65_536).unwrap(), at);
        let over = shl(Integer::one(), 65_536);
        assert!(integer_literal(&over.to_string(), 65_536).is_err());
        let lexical = json!({"schema_version":1,"variables":["x"],"axis":0,"terms":[{"coefficient":"9".repeat(19730),"exponents":[0]}]});
        let bytes = serde_json::to_vec(&lexical).unwrap();
        assert!(
            input_with_height(&bytes, &hash(&bytes), 65_536)
                .err()
                .unwrap()
                .to_string()
                .contains("literal preflight")
        );
        let p = UnivariatePolynomial::from_coefficients(
            &Z,
            vec![shl(Integer::one(), 20_000); MAX_DEGREE + 1],
            Arc::new(Algebra::new(&["x".into()]).unwrap().variables[0].clone()),
        );
        assert!(
            plan_with_height(&p, &Integer::one(), 32_768)
                .unwrap_err()
                .to_string()
                .contains("input coefficient cap")
        );
        let p = UnivariatePolynomial::from_coefficients(
            &Z,
            vec![Integer::one(); MAX_DEGREE + 1],
            Arc::new(Algebra::new(&["x".into()]).unwrap().variables[0].clone()),
        );
        assert!(plan_with_height(&p, &shl(Integer::one(), 255), 65_536).is_err());
    }
    #[test]
    fn recursive_cli_selection_and_envelope_are_explicit() {
        let argv = [
            "helper",
            "input",
            "--input-sha256",
            "hash",
            "--output",
            "fresh",
            "--recursive-leaf",
            "32",
            "--input-height-bits",
            "32768",
        ];
        let args = Args::try_parse_from(argv).unwrap();
        assert_eq!(args.recursive_leaf, Some(32));
        assert_eq!(args.input_height_bits, 32_768);
        assert_eq!(
            method(args.recursive_leaf, args.unit_add),
            "recursive_cached_binomial_convolution"
        );
        assert!(Args::try_parse_from(argv.into_iter().chain(["--plan-only"])).is_err());
        assert!(Args::try_parse_from(argv.into_iter().chain(["--unit-add"])).is_err());
        let unit = Args::try_parse_from([
            "helper",
            "input",
            "--input-sha256",
            "hash",
            "--output",
            "fresh",
            "--unit-add",
        ])
        .unwrap();
        assert_eq!(
            method(unit.recursive_leaf, unit.unit_add),
            "triangular_unit_add_or_subtract"
        );
        for leaf in [0, 7, 9, 1024] {
            assert!(recursive_policy(Some(leaf)).is_err());
        }
        for leaf in [32, 64, 128, 256] {
            recursive_policy(Some(leaf)).unwrap();
            let p = poly(
                &(0..259)
                    .map(|i| (i * 17 % 29) as i64 - 14)
                    .collect::<Vec<_>>(),
                "shift_axis",
            );
            let a = Integer::from(-3);
            plan_with_height(&p, &a, MAX_HEIGHT).unwrap();
            assert_eq!(
                recursive::recursive_shift(&p, &a, leaf).unwrap().0,
                p.shift_var(&a)
            );
        }
    }
}
