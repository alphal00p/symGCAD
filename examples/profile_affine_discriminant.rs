//! Research-only integer affine preconditioning with exact inverse replay.
//! No discriminant is computed; no scale, content, or boundary is omitted.
#[path = "profile_resultant_linear_gate.rs"]
#[allow(dead_code)]
mod gate_support;
use anyhow::{Context, Result, ensure};
use clap::Parser;
use gate_support::{coeff, content_screen, parse, read};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
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
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    #[arg(long)]
    shift_report: PathBuf,
    #[arg(long)]
    shift_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 12)]
    cpu: usize,
    #[arg(long, default_value_t = 60.)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
const OUTPUT_BYTES: usize = 16 * 1024 * 1024;
fn output_hash(mut reader: impl Read) -> Result<String> {
    let mut digest = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut total = 0usize;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total = total.checked_add(n).context("output size overflow")?;
        ensure!(total <= OUTPUT_BYTES, "output artifact byte cap");
        digest.update(&buf[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn write_new(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= OUTPUT_BYTES, "output serialization byte cap");
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&bytes)?;
    Ok(())
}
fn phase(path: &Path, text: &str) -> Result<()> {
    let temp = path.with_extension("progress.tmp");
    fs::write(&temp, serde_json::to_vec(&json!({"phase":text}))?)?;
    fs::rename(temp, path.with_extension("progress.json"))?;
    Ok(())
}
fn ceil_log(n: usize) -> u64 {
    if n <= 1 {
        0
    } else {
        u64::from(usize::BITS - (n - 1).leading_zeros())
    }
}
fn height(p: &Poly) -> u64 {
    p.coefficients
        .iter()
        .map(|c| {
            c.numerator_ref()
                .significant_bits()
                .max(c.denominator_ref().significant_bits())
        })
        .max()
        .unwrap_or(0)
}
fn shape(a: &Algebra, p: &Poly) -> Value {
    let text = a.format(p);
    json!({"expression":text,"expression_sha256":hash(text.as_bytes()),
        "terms":p.nterms(),"degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),
        "coefficient_bits":height(p)})
}
#[derive(Default)]
struct Work {
    pairs: usize,
}
impl Work {
    fn check(p: &Poly) -> Result<()> {
        ensure!(
            p.nvars() >= 2
                && p.nvars() <= 16
                && p.degree(0) <= 160
                && p.degree(1) <= 32
                && (2..p.nvars()).all(|i| p.degree(i) == 0)
                && p.nterms() <= 6000
                && height(p) <= 4096
                && p.coefficients.iter().all(|c| c.is_integer()),
            "intermediate envelope"
        );
        Ok(())
    }
    fn mul(&mut self, p: &Poly, q: &Poly) -> Result<Poly> {
        Self::check(p)?;
        Self::check(q)?;
        ensure!(p.variables() == q.variables(), "product maps");
        if p.is_zero() || q.is_zero() {
            return Ok(p.zero());
        }
        let ds = p
            .degree(0)
            .checked_add(q.degree(0))
            .context("s degree overflow")?;
        let dt = p
            .degree(1)
            .checked_add(q.degree(1))
            .context("t degree overflow")?;
        ensure!(
            ds <= 160 && dt <= 32 && (ds as usize + 1) * (dt as usize + 1) <= 6000,
            "pre-product dense support"
        );
        ensure!(
            height(p) + height(q) + ceil_log(p.nterms().min(q.nterms())) <= 4096,
            "pre-product height"
        );
        let pairs = p
            .nterms()
            .checked_mul(q.nterms())
            .context("product work overflow")?;
        ensure!(pairs <= 2_000_000, "single-product work");
        self.pairs = self
            .pairs
            .checked_add(pairs)
            .context("cumulative work overflow")?;
        ensure!(self.pairs <= 20_000_000, "cumulative product work");
        let r = p * q;
        Self::check(&r)?;
        Ok(r)
    }
    fn add(&self, p: &mut Poly, q: &Poly) -> Result<()> {
        ensure!(
            p.nterms() + q.nterms() <= 12000 && height(p).max(height(q)) + 1 <= 4096,
            "pre-sum envelope"
        );
        *p = &*p + q;
        Self::check(p)
    }
    fn scale(&self, p: &Poly, c: &Integer) -> Result<Poly> {
        ensure!(
            height(p) + c.significant_bits() <= 4096,
            "pre-scalar height"
        );
        let q = p.clone().mul_coeff(Rational::from(c.clone()));
        Self::check(&q)?;
        Ok(q)
    }
    fn powers(&mut self, p: &Poly, n: usize) -> Result<Vec<Poly>> {
        ensure!(n <= 32, "power count");
        let mut out = vec![p.one()];
        for i in 0..n {
            out.push(self.mul(&out[i], p)?);
        }
        Ok(out)
    }
}
fn transform(a: &Algebra, p: &Poly, offset: i64, denominator: u32) -> Result<(Poly, Value)> {
    Work::check(p)?;
    let n = p.degree(1) as usize;
    ensure!(
        (2..=32).contains(&n) && p.degree(0) <= 84 && p.nterms() <= 5000 && height(p) <= 1024,
        "original support envelope"
    );
    ensure!(
        offset.unsigned_abs() <= 64 && (1..=64).contains(&denominator),
        "affine constants"
    );
    // Coefficient L1 bound before constructing powers; 33 summands at most.
    let base_bits = 64 - (offset.unsigned_abs() + 1).leading_zeros() as u64;
    let den_bits = 32 - denominator.leading_zeros();
    let bound = height(p)
        + ceil_log(p.nterms())
        + (n as u64) * base_bits.max(u64::from(den_bits))
        + ceil_log(n + 1);
    ensure!(
        bound <= 4096 && (p.degree(0) as usize + n + 1) * (n + 1) <= 6000,
        "global forward height/support preflight"
    );
    let s = a.parse(&a.names[0])?;
    let t = a.parse(&a.names[1])?;
    let mut work = Work::default();
    let st = work.mul(&s, &t)?;
    let linear = &st + &p.one().mul_coeff(Rational::from(offset));
    let powers = work.powers(&linear, n)?;
    let mut star = p.zero();
    let q = Integer::from(denominator);
    for (i, power) in powers.iter().enumerate() {
        let term = work.mul(&coeff(p, 1, i as u32), power)?;
        let term = work.scale(&term, &q.pow((n - i) as u64))?;
        work.add(&mut star, &term)?;
    }
    ensure!(
        star.degree(1) == n as u32 && star.degree(0) <= p.degree(0) + n as u32,
        "forward degree preservation"
    );
    let forward_pairs = work.pairs;
    // Independent denominator-cleared inverse, valid polynomial identity at s=0 too.
    let inverse_linear = t.clone().mul_coeff(Rational::from(denominator))
        - p.one().mul_coeff(Rational::from(offset));
    let inverse_powers = work.powers(&inverse_linear, n)?;
    let s_powers = work.powers(&s, n)?;
    let mut recovered = p.zero();
    for j in 0..=n {
        let term = work.mul(&coeff(&star, 1, j as u32), &inverse_powers[j])?;
        let term = work.mul(&term, &s_powers[n - j])?;
        work.add(&mut recovered, &term)?;
    }
    let expected = work.mul(p, &s_powers[n])?;
    let expected = work.scale(&expected, &q.pow(n as u64))?;
    ensure!(
        recovered == expected,
        "exact inverse polynomial identity failed"
    );
    Ok((
        star,
        json!({"degree":n,"offset":offset,"denominator":denominator,
        "forward_coefficient_bound":bound,"forward_product_pairs":forward_pairs,
        "total_product_pairs":work.pairs,"inverse_identity_verified":true,
        "inverse_identity":"sum_j Pstar_j(s)*(q*t-offset)^j*s^(N-j) = q^N*s^N*p(s,t)",
        "inverse_evaluated_at_s_zero_by_polynomial_identity":true}),
    ))
}
fn bound_sources(args: &Args) -> Result<(OperationCapture, Value, Vec<PathBuf>)> {
    let cb = read(&args.capture)?;
    ensure!(hash(&cb) == args.capture_sha256, "capture file hash");
    let capture: OperationCapture = serde_json::from_slice(&cb)?;
    ensure!(
        capture.schema_version == 1
            && capture.coefficient_domain == "Q"
            && capture.operation == AlgebraOperation::Discriminant
            && capture.variable == Some(1),
        "original discriminant capture"
    );
    ensure!(
        (2..=16).contains(&capture.variable_names.len())
            && capture.variable_names[0] == "s"
            && capture.variable_names[1] == "t"
            && capture.variable_names.iter().all(|n| !n.is_empty()
                && n.len() <= 32
                && n.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')),
        "capture map envelope"
    );
    let sb = read(&args.shift_report)?;
    ensure!(hash(&sb) == args.shift_sha256, "shift report binding");
    let report: Value = serde_json::from_slice(&sb)?;
    ensure!(
        report["schema_version"] == 1
            && report["inputs_unchanged"] == true
            && report["process"]["status"] == "completed"
            && report["process"]["exit_code"] == 0
            && report["process"]["output_truncated"] == false,
        "shift worker not complete"
    );
    let image_path = args.shift_report.with_extension("image.json");
    let ib = read(&image_path)?;
    ensure!(
        report["result"]["image_sha256"] == hash(&ib),
        "shift image digest"
    );
    let image: Value = serde_json::from_slice(&ib)?;
    ensure!(
        image["kind"] == "exact_constant_shift_diagnostic"
            && image["shift"]["identity_verified"] == true
            && image["shift"]["p"] == "5"
            && image["shift"]["q"] == "18",
        "expected checked constant shift"
    );
    let paths = report["input_paths"]
        .as_array()
        .context("shift input paths")?;
    let hashes = report["input_sha256"]
        .as_array()
        .context("shift input hashes")?;
    ensure!(
        !paths.is_empty() && paths.len() <= 12 && paths.len() == hashes.len(),
        "shift provenance count"
    );
    let mut inputs = vec![args.capture.clone(), args.shift_report.clone(), image_path];
    let mut capture_bound = false;
    for (p, h) in paths.iter().zip(hashes) {
        let name = p.as_str().context("source path")?;
        ensure!(name.len() <= 4096, "path length");
        let path = PathBuf::from(name);
        ensure!(
            hash(&read(&path)?) == h.as_str().context("source digest")?,
            "shift source changed"
        );
        if fs::canonicalize(&path)? == fs::canonicalize(&args.capture)? {
            ensure!(
                h == &json!(args.capture_sha256),
                "capture ancestry mismatch"
            );
            capture_bound = true;
        }
        inputs.push(path);
    }
    ensure!(capture_bound, "capture absent from shift provenance");
    let operands = capture.operands.as_ref().context("captured operand")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == capture.input_hash,
        "capture operand digest"
    );
    Ok((capture, image, inputs))
}
fn worker(args: &Args) -> Result<()> {
    let start = Instant::now();
    let (capture, _, _) = bound_sources(args)?;
    let a = Algebra::new(&capture.variable_names)?;
    let p = parse(&a, &capture.operands.as_ref().unwrap()[0])?;
    ensure!(
        p.degree(1) == 32 && p.degree(0) == 84 && p.nterms() == 2502,
        "authentic operand shape"
    );
    phase(
        &args.output,
        "integer affine substitution and inverse replay",
    )?;
    let then = Instant::now();
    let (star, transformation) = transform(&a, &p, 5, 18)?;
    let transform_seconds = then.elapsed().as_secs_f64();
    phase(&args.output, "exact signed coefficient content of Pstar")?;
    let content = content_screen(&a, &star, 1, 32)?;
    let q = parse(
        &a,
        content["residual"]["expression"]
            .as_str()
            .context("Q expression")?,
    )?;
    let cp = parse(
        &a,
        content["content"]["expression"]
            .as_str()
            .context("content expression")?,
    )?;
    let scalar = rational(
        content["signed_scalar"]
            .as_str()
            .context("signed content scalar")?,
    )?;
    let d = cp.clone().mul_coeff(scalar.clone());
    ensure!(&d * &q == star, "final signed d*Q reconstruction");
    let qtext = a.format(&q);
    let operands = vec![qtext];
    let qcapture = OperationCapture {
        schema_version: 1,
        backend_version: capture.backend_version.clone(),
        coefficient_domain: "Q".into(),
        variable_names: capture.variable_names.clone(),
        operation: AlgebraOperation::Discriminant,
        variable: Some(1),
        key: hash(&serde_json::to_vec(
            &json!({"kind":"research_affine_Q_discriminant",
            "source_capture":args.capture_sha256,"Q":hash(operands[0].as_bytes())}),
        )?),
        input_hash: hash(&serde_json::to_vec(&operands)?),
        operands: Some(operands),
        operands_omitted_for_size: false,
        complete: false,
        elapsed_seconds: None,
        output_hash: None,
    };
    let qp = args.output.with_extension("Q.capture.json");
    write_new(&qp, &serde_json::to_value(&qcapture)?)?;
    let report = json!({"schema_version":1,"kind":"exact_affine_discriminant_preconditioning_gate",
        "capture_sha256":args.capture_sha256,"shift_report_sha256":args.shift_sha256,
        "variables":a.names,"old_t_slot_contains_u":true,"original":shape(&a,&p),
        "Pstar":shape(&a,&star),"d":shape(&a,&d),"Q":shape(&a,&q),
        "content_details":content,"transformation":transformation,"transform_seconds":transform_seconds,
        "discriminant_identity":"18^992*s^992*Disc_t(p) = d(s)^62*Disc_u(Q)",
        "forward_identity":"18^32*p(s,(5+s*u)/18) = Pstar(s,u) = d(s)*Q(s,u)",
        "all_content_zero_loci_retained":true,"discriminant_computed":false,
        "Q_capture":qp,"Q_capture_sha256":hash(&read(&qp)?),"total_seconds":start.elapsed().as_secs_f64(),
        "scope":"Integer polynomial identities only. No cancellation at s=0, no discriminant result, no original-coordinate chart or coverage claim."});
    let cp = args.output.with_extension("candidate.json");
    write_new(&cp, &report)?;
    println!(
        "{}",
        json!({"kind":"exact_affine_discriminant_preconditioning_gate","candidate":cp,
        "candidate_sha256":output_hash(fs::File::open(&cp)?)?,"Q_capture":qp,"Q_capture_sha256":report["Q_capture_sha256"],
        "transform_seconds":transform_seconds,"content_seconds":report["content_details"]["total_seconds"],
        "total_seconds":report["total_seconds"]})
    );
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 60.,
        "worker time cap"
    );
    for h in [&args.capture_sha256, &args.shift_sha256] {
        ensure!(
            h.len() == 64 && h.bytes().all(|c| c.is_ascii_hexdigit()),
            "digest syntax"
        );
    }
    if args.worker {
        return worker(&args);
    }
    for p in [
        &args.output,
        args.output.with_extension("candidate.json").as_path(),
        args.output.with_extension("Q.capture.json").as_path(),
        args.output.with_extension("progress.json").as_path(),
        args.output.with_extension("progress.tmp").as_path(),
    ] {
        ensure!(!p.exists(), "fresh outputs required");
    }
    if let Some(p) = args.output.parent() {
        fs::create_dir_all(p)?;
    }
    let (_, _, paths) = bound_sources(&args)?;
    let before = paths
        .iter()
        .map(|p| Ok(hash(&read(p)?)))
        .collect::<Result<Vec<_>>>()?;
    let exe = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .args(["-c", &args.cpu.to_string()])
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
    let mut process = run_bounded(&mut command, &limits)?;
    let result = process
        .stdout
        .lines()
        .last()
        .and_then(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["kind"] == "exact_affine_discriminant_preconditioning_gate");
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.is_some();
    if result.is_some() {
        process.stdout.clear();
    }
    let after = paths
        .iter()
        .map(|p| Ok(hash(&read(p)?)))
        .collect::<Result<Vec<_>>>()?;
    write_new(
        &args.output,
        &json!({"schema_version":1,"input_paths":paths,"input_sha256":before,
        "inputs_unchanged":before==after,"binary_sha256":hash(&fs::read(exe)?),"cpu":args.cpu,
        "limits":limits,"process":process,"result":result}),
    )?;
    ensure!(
        success && before == after,
        "incomplete gate or changed inputs"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::projection::exact_resultant_ducos;
    fn algebra() -> Algebra {
        Algebra::new(&["s".into(), "t".into()]).unwrap()
    }
    fn disc(p: &Poly) -> Poly {
        let n = p.degree(1);
        let r = exact_resultant_ducos(p, &p.derivative(1), 1).unwrap();
        let (mut d, rem) = r.quot_rem(&p.univariate_lcoeff(1), false);
        assert!(rem.is_zero());
        if n % 4 == 2 || n % 4 == 3 {
            d = -d;
        }
        d
    }
    fn identity(expression: &str, offset: i64, den: u32) {
        let a = algebra();
        let p = a.parse(expression).unwrap();
        let n = p.degree(1);
        let (star, replay) = transform(&a, &p, offset, den).unwrap();
        assert_eq!(replay["inverse_identity_verified"], true);
        let content = content_screen(&a, &star, 1, n).unwrap();
        let q = parse(&a, content["residual"]["expression"].as_str().unwrap()).unwrap();
        let d = parse(&a, content["content"]["expression"].as_str().unwrap())
            .unwrap()
            .mul_coeff(rational(content["signed_scalar"].as_str().unwrap()).unwrap());
        assert_eq!(&d * &q, star);
        let exponent = n * (n - 1);
        let lhs = (&a.parse("s").unwrap().pow(exponent as usize) * &disc(&p))
            .mul_coeff(Rational::from(Integer::from(den).pow(exponent as u64)));
        let rhs = &d.pow((2 * n - 2) as usize) * &disc(&q);
        assert_eq!(lhs, rhs);
    }
    #[test]
    fn signed_nonconstant_content_and_full_discriminant_scaling() {
        identity("-7*((18*t-5)^2-324*s^2)", 5, 18);
        identity("-3*s*(t^3+s*t+2)", -2, 3);
    }
    #[test]
    fn leading_specialization_loss_is_not_cancelled() {
        let a = algebra();
        let p = a.parse("t^2+t+1").unwrap();
        let (star, _) = transform(&a, &p, 5, 18).unwrap();
        assert_eq!(star.replace(0, &Rational::zero()).degree(1), 0);
        assert!(disc(&star).replace(0, &Rational::zero()).is_zero());
        assert!(!disc(&p).replace(0, &Rational::zero()).is_zero());
        identity("t^2+t+1", 5, 18);
    }
    #[test]
    fn malformed_and_oversized_support_declines_before_transform() {
        let a = algebra();
        assert!(transform(&a, &a.parse("s^85+t^2").unwrap(), 5, 18).is_err());
        assert!(transform(&a, &a.parse("t^33+s").unwrap(), 5, 18).is_err());
        assert!(transform(&a, &a.parse("t^2+s").unwrap(), 5, 0).is_err());
        assert!(parse(&a, "(1)*s^99999").is_err());
    }
    #[test]
    fn bounded_output_hash_accepts_large_report_without_weakening_input_cap() {
        let data = vec![b'x'; 2 * 1024 * 1024 + 100];
        assert_eq!(output_hash(data.as_slice()).unwrap(), hash(&data));
        assert!(output_hash(std::io::repeat(b'x').take(OUTPUT_BYTES as u64 + 1)).is_err());
    }
}
