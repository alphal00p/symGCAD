//! Research-only bounded native SQF and full signed-product replay.
//! No irreducibility, coprimality, root, projection or CAD authority is exported.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

const MAX_FILE: usize = 16 * 1024 * 1024;
const MAX_EXPRESSION: usize = 4 * 1024 * 1024;
const MAX_INVENTORY_BITS: u64 = 2_000_000;
const MAX_PRODUCT_PAIRS: usize = 64_000_000;
const POLICY: &str = "multivariate-sqf-product-v1";

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    /// Must not exist. No solver cache is read or written.
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value_t = 1)]
    cpu: usize,
    #[arg(long, default_value_t = 3600.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    ensure!(file.metadata()?.is_file(), "regular input required");
    let mut bytes = Vec::new();
    (&mut file).take(cap as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= cap, "input byte cap");
    Ok(bytes)
}
fn stream_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    ensure!(file.metadata()?.is_file(), "regular binary required");
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
fn write_json(path: &Path, v: &Value, replace: bool) -> Result<String> {
    let bytes = serde_json::to_vec_pretty(v)?;
    ensure!(bytes.len() <= MAX_FILE, "output byte cap");
    let temp = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    if replace {
        fs::rename(&temp, path)?;
    } else {
        fs::hard_link(&temp, path)?;
        fs::remove_file(&temp)?;
    }
    Ok(hash(&bytes))
}
fn names_ok(names: &[String]) -> bool {
    let mut seen = HashSet::new();
    (1..=64).contains(&names.len())
        && names.iter().all(|n| {
            !n.is_empty()
                && n.len() <= 32
                && n.as_bytes()[0].is_ascii_alphabetic()
                && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && seen.insert(n)
        })
}
fn scalar(text: &str) -> Result<Rational> {
    let mut parts = text.split('/');
    let numerator = parts.next().context("missing numerator")?;
    let denominator = parts.next();
    ensure!(parts.next().is_none(), "rational grammar");
    let digits = numerator.strip_prefix('-').unwrap_or(numerator);
    ensure!(
        !digits.is_empty() && digits.len() <= 1234 && digits.bytes().all(|b| b.is_ascii_digit()),
        "numerator grammar/height"
    );
    if let Some(d) = denominator {
        ensure!(
            !d.is_empty() && d.len() <= 1234 && d.bytes().all(|b| b.is_ascii_digit()),
            "denominator grammar/height"
        );
    }
    let q = rational(text)?;
    ensure!(
        !q.is_zero()
            && q.numerator_ref().significant_bits() <= 4096
            && q.denominator_ref().significant_bits() <= 4096,
        "rational height/zero"
    );
    Ok(q)
}
// No arbitrary Atom grammar: literal rational coefficients and bounded monomials.
fn parse(a: &Algebra, text: &str, terms: usize) -> Result<Poly> {
    ensure!(
        !text.is_empty() && text.len() <= MAX_EXPRESSION,
        "expression byte cap"
    );
    let mut p = a.parse("0")?;
    let mut seen = HashSet::new();
    let mut denominator_bits = 0u64;
    let mut storage = 0u64;
    let mut active = HashSet::new();
    for (i, term) in text.split('+').enumerate() {
        ensure!(i < terms, "term cap");
        let (c, tail) = term
            .strip_prefix('(')
            .context("coefficient prefix")?
            .split_once(')')
            .context("coefficient suffix")?;
        let c = scalar(c)?;
        storage += c.numerator_ref().significant_bits() as u64
            + c.denominator_ref().significant_bits() as u64;
        if !c.is_integer() {
            denominator_bits += c.denominator_ref().significant_bits() as u64;
        }
        ensure!(
            storage <= MAX_INVENTORY_BITS && denominator_bits <= 4096,
            "coefficient storage/common denominator bound"
        );
        let mut e = vec![0u32; a.names.len()];
        if !tail.is_empty() {
            for monomial in tail
                .strip_prefix('*')
                .context("monomial prefix")?
                .split('*')
            {
                let (name, power) = monomial.split_once('^').unwrap_or((monomial, "1"));
                let axis = a
                    .names
                    .iter()
                    .position(|n| n == name)
                    .context("unknown variable")?;
                ensure!(
                    !power.is_empty()
                        && power.len() <= 2
                        && power.bytes().all(|b| b.is_ascii_digit()),
                    "exponent grammar"
                );
                let exponent: u32 = power.parse()?;
                ensure!(
                    (1..=32).contains(&exponent) && e[axis] == 0,
                    "axis degree/repeated variable"
                );
                e[axis] = exponent;
                active.insert(axis);
            }
        }
        ensure!(
            active.len() <= 3 && e.iter().sum::<u32>() <= 64 && seen.insert(e.clone()),
            "support/total degree/duplicate term"
        );
        p.append_monomial(c, &e);
    }
    Ok(p)
}
fn bits(p: &Poly) -> u64 {
    p.coefficients
        .iter()
        .map(|c| c.numerator_ref().significant_bits() as u64)
        .max()
        .unwrap_or(0)
}
fn storage(p: &Poly) -> u64 {
    p.coefficients
        .iter()
        .map(|c| {
            c.numerator_ref().significant_bits() as u64
                + c.denominator_ref().significant_bits() as u64
        })
        .sum()
}
fn summary(a: &Algebra, p: &Poly) -> Value {
    json!({"terms":p.nterms(),"degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),
        "total_degree":p.exponents_iter().map(|e|e.iter().sum::<u32>()).max().unwrap_or(0),
        "max_numerator_bits":bits(p),"max_denominator_bits":p.coefficients.iter()
            .map(|c|c.denominator_ref().significant_bits()).max().unwrap_or(0),
        "coefficient_storage_bits":storage(p),"expression_sha256":hash(a.format(p).as_bytes())})
}
fn ceil_log(n: usize) -> u64 {
    if n <= 1 {
        0
    } else {
        usize::BITS as u64 - (n - 1).leading_zeros() as u64
    }
}
fn inventory(p: &Poly, factors: &[(Poly, usize)]) -> Result<Value> {
    ensure!(!factors.is_empty() && factors.len() <= 64, "block count");
    let mut degrees = vec![0u64; p.nvars()];
    let mut terms = 0usize;
    let mut cost = 0u64;
    let mut norm_bits = 0u64;
    let mut denominator_bits = 0u64;
    let mut multiplicities = 0usize;
    let mut total_degree = 0u64;
    for (f, m) in factors {
        ensure!(
            !f.is_zero()
                && (1..=64).contains(m)
                && f.nvars() == p.nvars()
                && f.get_vars_ref() == p.get_vars_ref(),
            "block shape/map/multiplicity"
        );
        let nonconstant = (0..p.nvars()).any(|i| f.degree(i) != 0);
        ensure!(
            !nonconstant || f.coefficients.iter().all(|c| c.is_integer()),
            "native nonconstant blocks must be integral"
        );
        ensure!(
            bits(f) <= 4096
                && f.coefficients
                    .iter()
                    .all(|c| c.denominator_ref().significant_bits() <= 4096),
            "block coefficient height"
        );
        terms = terms.checked_add(f.nterms()).context("term overflow")?;
        cost = cost.checked_add(storage(f)).context("storage overflow")?;
        norm_bits += (bits(f) + ceil_log(f.nterms())) * *m as u64;
        denominator_bits += f
            .coefficients
            .iter()
            .map(|c| {
                if c.is_integer() {
                    0
                } else {
                    c.denominator_ref().significant_bits() as u64
                }
            })
            .sum::<u64>()
            * *m as u64;
        multiplicities += *m;
        total_degree += f
            .exponents_iter()
            .map(|e| e.iter().sum::<u32>())
            .max()
            .unwrap_or(0) as u64
            * *m as u64;
        for (axis, sum) in degrees.iter_mut().enumerate() {
            *sum += u64::from(f.degree(axis)) * *m as u64;
        }
    }
    ensure!(
        terms <= 4096
            && cost <= MAX_INVENTORY_BITS
            && norm_bits <= 16384
            && denominator_bits <= 4096
            && multiplicities <= 192,
        "block/product preflight"
    );
    ensure!(
        degrees
            .iter()
            .enumerate()
            .all(|(i, d)| *d == u64::from(p.degree(i))),
        "weighted degree identity"
    );
    ensure!(
        total_degree
            == p.exponents_iter()
                .map(|e| e.iter().sum::<u32>())
                .max()
                .unwrap_or(0) as u64,
        "weighted total degree identity"
    );
    Ok(
        json!({"blocks":factors.len(),"total_block_terms":terms,"block_storage_bits":cost,
        "product_numerator_norm_bit_bound":norm_bits,"product_denominator_bit_bound":denominator_bits,
        "weighted_degrees":degrees,"weighted_total_degree":total_degree,"multiplicity_sum":multiplicities}),
    )
}
fn reconstruct(p: &Poly, factors: &[(Poly, usize)]) -> Result<usize> {
    inventory(p, factors)?;
    // Native nonconstant factors are integral; apply rational scalar factors last.
    let mut product = p.one();
    let mut work = 0usize;
    for constant in [false, true] {
        for (f, m) in factors {
            if (0..p.nvars()).all(|i| f.degree(i) == 0) != constant {
                continue;
            }
            for _ in 0..*m {
                let mut dense = 1usize;
                for axis in 0..p.nvars() {
                    let d = product.degree(axis) + f.degree(axis);
                    ensure!(d <= p.degree(axis), "intermediate degree");
                    dense = dense
                        .checked_mul(d as usize + 1)
                        .context("dense overflow")?;
                }
                ensure!(dense <= 40_000, "intermediate dense support cap");
                let pairs = product
                    .nterms()
                    .checked_mul(f.nterms())
                    .context("product pair overflow")?;
                work = work.checked_add(pairs).context("work overflow")?;
                ensure!(
                    pairs <= 8_000_000 && work <= MAX_PRODUCT_PAIRS,
                    "reconstruction product-work cap"
                );
                ensure!(
                    bits(&product) + bits(f) + ceil_log(product.nterms().min(f.nterms())) <= 16384,
                    "intermediate coefficient bound"
                );
                product = &product * f;
                ensure!(
                    product.nterms() <= 40_000 && bits(&product) <= 16384,
                    "intermediate output cap"
                );
            }
        }
    }
    ensure!(product == *p, "full signed product mismatch");
    Ok(work)
}
fn load(args: &Args) -> Result<(Algebra, Poly, Value)> {
    let bytes = read(&args.capture, MAX_EXPRESSION)?;
    ensure!(hash(&bytes) == args.capture_sha256, "capture SHA mismatch");
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(
        capture.schema_version == 1
            && capture.coefficient_domain == "Q"
            && capture.operation == AlgebraOperation::Factor
            && capture.variable.is_none()
            && !capture.operands_omitted_for_size
            && names_ok(&capture.variable_names),
        "capture envelope"
    );
    let operands = capture.operands.context("missing operands")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(&operands)?) == capture.input_hash,
        "ordered operand-array digest"
    );
    let a = Algebra::new(&capture.variable_names)?;
    let p = parse(&a, &operands[0], 2048)?;
    ensure!((0..p.nvars()).any(|i| p.degree(i) > 0), "constant source");
    let binding = json!({"capture_sha256":args.capture_sha256,"captured_backend":capture.backend_version,
        "input_hash":capture.input_hash,"source_key":capture.key,"variable_names":capture.variable_names,
        "source":summary(&a,&p),"captured_expression_sha256":hash(operands[0].as_bytes())});
    Ok((a, p, binding))
}
fn worker(args: &Args) -> Result<()> {
    let start = Instant::now();
    let (a, p, binding) = load(args)?;
    let mut report = json!({"schema_version":1,"policy":POLICY,"binding":binding,
        "phase":"native_square_free","parse_seconds":start.elapsed().as_secs_f64(),
        "exact_signed_product_verified":false,"square_free_independently_verified":false,
        "no_projection_root_or_cad_claim":true});
    let progress = args.output_dir.join("progress.json");
    write_json(&progress, &report, true)?;
    let timer = Instant::now();
    let native = p.square_free_factorization();
    report["native_square_free_seconds"] = json!(timer.elapsed().as_secs_f64());
    report["phase"] = json!("proposal_preflight");
    write_json(&progress, &report, true)?;
    report["inventory"] = inventory(&p, &native)?;
    let timer = Instant::now();
    let mut records = Vec::new();
    let mut replay = Vec::new();
    let mut bytes = 0usize;
    for (f, m) in native {
        let expression = a.format(&f);
        bytes = bytes
            .checked_add(expression.len())
            .context("proposal size overflow")?;
        ensure!(bytes <= MAX_EXPRESSION, "proposal expression inventory");
        let decoded = parse(&a, &expression, 4096)?;
        ensure!(decoded == f, "proposal sparse roundtrip");
        records.push(json!({"multiplicity":m,"polynomial":summary(&a,&f),
            "expression_sha256":hash(expression.as_bytes()),"expression":expression}));
        replay.push((decoded, m));
    }
    let proposal = json!({"schema_version":1,"policy":POLICY,"binding":report["binding"],
        "native_square_free_seconds":report["native_square_free_seconds"],"inventory":report["inventory"],
        "exact_signed_product_verified":false,"square_free_independently_verified":false,
        "factors":records,"note":"Unverified native proposal. Preserve constants and every multiplicity; a consumer must replay the full signed product."});
    let proposal_hash = write_json(&args.output_dir.join("proposal.json"), &proposal, false)?;
    report["proposal_sha256"] = json!(proposal_hash);
    report["proposal_seconds"] = json!(timer.elapsed().as_secs_f64());
    report["phase"] = json!("signed_product_reconstruction");
    write_json(&progress, &report, true)?;
    let timer = Instant::now();
    let work = reconstruct(&p, &replay)?;
    report["reconstruction_seconds"] = json!(timer.elapsed().as_secs_f64());
    report["reconstruction_product_pairs"] = json!(work);
    report["exact_signed_product_verified"] = json!(true);
    report["phase"] = json!("complete");
    report["total_worker_seconds_before_final_write"] = json!(start.elapsed().as_secs_f64());
    let digest = write_json(&args.output_dir.join("worker.json"), &report, false)?;
    write_json(&progress, &report, true)?;
    println!(
        "{}",
        json!({"phase":"complete","worker_report_sha256":digest})
    );
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 3600.0,
        "deadline must be in (0,3600]"
    );
    ensure!(
        args.capture_sha256.len() == 64
            && args.capture_sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "SHA-256 syntax"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(!args.output_dir.exists(), "fresh output directory required");
    let before = hash(&read(&args.capture, MAX_EXPRESSION)?);
    ensure!(before == args.capture_sha256, "source pin mismatch");
    fs::create_dir_all(&args.output_dir)?;
    let executable = std::env::current_exe()?;
    let binary_before = stream_hash(&executable)?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
        .arg(&args.capture)
        .arg("--capture-sha256")
        .arg(&args.capture_sha256)
        .arg("--output-dir")
        .arg(&args.output_dir)
        .arg("--seconds")
        .arg(args.seconds.to_string())
        .arg("--worker");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let process = run_bounded(&mut command, &limits)?;
    // Preserve the process outcome even if later evidence/hash inspection fails.
    write_json(
        &args.output_dir.join("process.json"),
        &json!({"limits":limits,"cpu":args.cpu,
        "binary_sha256":binary_before,"capture_sha256":before,"process":process}),
        false,
    )?;
    let message: Option<Value> = process
        .stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| serde_json::from_str(l).ok());
    let good = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && message.as_ref().is_some_and(|v| v["phase"] == "complete");
    let source_unchanged = hash(&read(&args.capture, MAX_EXPRESSION)?) == before;
    let binary_unchanged = stream_hash(&executable)? == binary_before;
    let checked_report = if good {
        let b = read(&args.output_dir.join("worker.json"), MAX_FILE)?;
        ensure!(
            message.as_ref().unwrap()["worker_report_sha256"] == hash(&b),
            "worker report binding"
        );
        let r: Value = serde_json::from_slice(&b)?;
        ensure!(
            r["phase"] == "complete" && r["exact_signed_product_verified"] == true,
            "signed product did not complete"
        );
        let proposal = read(&args.output_dir.join("proposal.json"), MAX_FILE)?;
        ensure!(r["proposal_sha256"] == hash(&proposal), "proposal binding");
        Some(r)
    } else {
        None
    };
    write_json(
        &args.output_dir.join("report.json"),
        &json!({"schema_version":1,"policy":POLICY,
        "source_unchanged":source_unchanged,"binary_unchanged":binary_unchanged,
        "completed_signed_product":good && source_unchanged && binary_unchanged,"worker":checked_report,
        "note":"Algebra diagnostic only. Product equality preserves all zero sets and multiplicities; no independent SQF/coprime/irreducible, root or CAD proof."}),
        false,
    )?;
    ensure!(
        good && source_unchanged && binary_unchanged,
        "bounded research stage incomplete; all available evidence retained"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn algebra() -> Algebra {
        Algebra::new(&["x".into(), "y".into(), "z".into(), "inactive".into()]).unwrap()
    }
    #[test]
    fn full_multivariate_signed_product_retains_content_and_multiplicity() {
        let a = algebra();
        let p = a.parse("-6/5*x^3*(y+1)^2*(z+x*y)^2*(z-1)").unwrap();
        let native = p.square_free_factorization();
        assert!(native.iter().any(|(_, m)| *m == 2));
        reconstruct(&p, &native).unwrap();
        for (f, _) in &native {
            assert_eq!(parse(&a, &a.format(f), 4096).unwrap(), *f);
        }
        let mut bad = native.clone();
        bad[0].1 += 1;
        assert!(reconstruct(&p, &bad).is_err());
        // A direct source-sign mutation cannot pass the signed comparison.
        assert!(reconstruct(&(-p.clone()), &native).is_err());
    }
    #[test]
    fn sparse_input_caps_precede_expression_expansion() {
        let a = algebra();
        assert_eq!(
            parse(&a, "(-2/3)*x^2*y+(1)*z", 2048).unwrap(),
            a.parse("-2/3*x^2*y+z").unwrap()
        );
        for bad in [
            "(2^999999)",
            "(1)*x^33",
            "(1)*x^1*x^2",
            "(1)*x+(2)*x",
            "(1)*x*y*z*inactive",
            "(1/0)*x",
            "(0)*x",
            "(1)*x^32*y^32*z",
        ] {
            assert!(parse(&a, bad, 2048).is_err(), "{bad}");
        }
        assert!(!names_ok(&["x".into(), "x".into()]));
        assert!(!names_ok(&["x+y".into()]));
    }
    #[test]
    fn product_equality_does_not_assert_square_freeness_or_coprimality() {
        let a = algebra();
        let p = a.parse("(z-x)^2*(z+y)").unwrap();
        let blocks = vec![
            (a.parse("(z-x)^2").unwrap(), 1),
            (a.parse("z+y").unwrap(), 1),
        ];
        reconstruct(&p, &blocks).unwrap();
        let overlap = vec![
            (a.parse("z-x").unwrap(), 1),
            (a.parse("(z-x)*(z+y)").unwrap(), 1),
        ];
        reconstruct(&p, &overlap).unwrap();
        // These valid product proposals are deliberately not projection bases.
        assert!(inventory(&p, &[(p.clone(), 0)]).is_err());
        assert!(inventory(&p, &[(a.parse("z^33").unwrap(), 1)]).is_err());
    }
    #[test]
    fn checkpoint_is_unverified_immutable_and_bound_to_source_bytes() {
        let directory =
            std::env::temp_dir().join(format!("multivariate-sqf-fixture-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let proposal = directory.join("proposal.json");
        let record = json!({"exact_signed_product_verified":false,"factors":[]});
        let digest = write_json(&proposal, &record, false).unwrap();
        assert_eq!(digest, hash(&read(&proposal, MAX_FILE).unwrap()));
        assert!(
            write_json(
                &proposal,
                &json!({"exact_signed_product_verified":true}),
                false
            )
            .is_err()
        );
        assert_eq!(digest, hash(&read(&proposal, MAX_FILE).unwrap()));
        let capture = directory.join("capture.json");
        let operands = vec!["(-2)*x+(1)*z".to_string()];
        let value = json!({"schema_version":1,"backend_version":"fixture","coefficient_domain":"Q",
            "variable_names":["x","y","z","inactive"],"operation":"factor","variable":null,
            "key":"fixture","input_hash":hash(&serde_json::to_vec(&operands).unwrap()),"operands":operands,
            "operands_omitted_for_size":false,"complete":false,"elapsed_seconds":null,"output_hash":null});
        let digest = write_json(&capture, &value, false).unwrap();
        let args = Args {
            capture: capture.clone(),
            capture_sha256: digest,
            output_dir: directory.join("output"),
            cpu: 1,
            seconds: 1.0,
            worker: true,
        };
        let (a, p, _) = load(&args).unwrap();
        assert_eq!(a.names.len(), 4);
        assert_eq!(p.degree(3), 0);
        fs::write(&capture, b"{}").unwrap();
        assert!(load(&args).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
