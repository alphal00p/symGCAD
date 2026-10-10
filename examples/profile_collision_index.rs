//! Bounded exact integer images of a collision-index polynomial, not interpolation.
//! This helper never writes solver checkpoints or omits any zero locus.
#[path = "profile_resultant_linear_gate.rs"]
#[allow(dead_code)]
mod gate_support;
use anyhow::{Context, Result, ensure};
use clap::Parser;
use gate_support::{coeff, content_screen, gate, parse, read};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{prelude::*, tensors::matrix::Matrix};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Parser)]
struct Args {
    ancestry: PathBuf,
    #[arg(long)]
    ancestry_sha256: String,
    #[arg(long)]
    capture: PathBuf,
    #[arg(long)]
    content_report: PathBuf,
    #[arg(long)]
    content_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, allow_hyphen_values = true)]
    point: i64,
    #[arg(long, default_value_t = 12)]
    cpu: usize,
    #[arg(long, default_value_t = 60.)]
    seconds: f64,
    /// Diagnose a constant coordinate shift and exact s-content; no determinant.
    #[arg(long)]
    shift_only: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn write_new(path: &Path, value: &Value) -> Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(&serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn progress(path: &Path, phase: &str) -> Result<()> {
    let tmp = path.with_extension("progress.tmp");
    fs::write(&tmp, serde_json::to_vec(&json!({"phase":phase}))?)?;
    fs::rename(tmp, path.with_extension("progress.json"))?;
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
fn point_bound(p: &Poly, point: i64, z: usize) -> Result<u64> {
    ensure!(
        point.unsigned_abs() <= 64 && p.nvars() <= 16 && z > 0 && z < p.nvars(),
        "point/map bound"
    );
    ensure!(
        p.nterms() <= 5000
            && p.degree(0) <= 128
            && p.degree(z) <= 32
            && (0..p.nvars()).all(|i| i == 0 || i == z || p.degree(i) == 0)
            && p.coefficients.iter().all(|c| c.denominator_ref().is_one())
            && height(p) <= 4096,
        "specialization support/integer envelope"
    );
    let value_bits = 64 - point.unsigned_abs().leading_zeros() as u64;
    let bound = height(p) + u64::from(p.degree(0)) * value_bits + ceil_log(p.nterms());
    ensure!(bound <= 4096, "pre-specialization coefficient height");
    Ok(bound)
}
fn specialize(p: &Poly, point: i64, z: usize) -> Result<Poly> {
    let bound = point_bound(p, point, z)?;
    let q = p.replace(0, &Rational::from(point));
    ensure!(
        height(&q) <= bound && (0..q.nvars()).all(|i| i == z || q.degree(i) == 0),
        "specialization replay envelope"
    );
    Ok(q)
}
fn norm_bits(p: &Poly) -> Result<u64> {
    ensure!(
        p.coefficients.iter().all(|c| c.denominator_ref().is_one()),
        "integer coefficients required"
    );
    let norm = p
        .coefficients
        .iter()
        .fold(Integer::zero(), |n, c| n + c.numerator_ref().abs());
    Ok(norm.significant_bits())
}
fn checked_mul(p: &Poly, q: &Poly, z: usize) -> Result<Poly> {
    ensure!(
        p.variables() == q.variables()
            && p.degree(z) + q.degree(z) <= 124
            && norm_bits(p)? + norm_bits(q)? <= 4096,
        "pre-product power bound"
    );
    let r = p * q;
    ensure!(
        r.nterms() <= 125 && height(&r) <= 4096,
        "power result envelope"
    );
    Ok(r)
}
// This routine retains the declared degrees even if specialization lowers them.
// It computes a polynomial determinant, not a rational quotient at that point.
fn index_matrix(
    a: &Poly,
    b: &Poly,
    h: &Poly,
    z: usize,
    n: usize,
    d: usize,
) -> Result<(Matrix<IntegerRing>, Value)> {
    ensure!(
        (2..=32).contains(&n) && (1..=4).contains(&d),
        "declared degree envelope"
    );
    ensure!(
        a.variables() == b.variables()
            && a.variables() == h.variables()
            && z < h.nvars()
            && h.degree(z) as usize <= n
            && a.degree(z) as usize <= d
            && b.degree(z) as usize <= d
            && [a, b, h]
                .iter()
                .all(|p| (0..p.nvars()).all(|i| i == z || p.degree(i) == 0)),
        "numeric support/degree envelope"
    );
    let size = d
        .checked_mul(n - 1)
        .and_then(|x| x.checked_add(1))
        .context("matrix dimension overflow")?;
    let count = size.checked_mul(size).context("matrix entry overflow")?;
    ensure!(size <= 125 && count <= 15625, "matrix allocation cap");
    let remainder_columns = size.checked_sub(n).context("invalid rectangular degree")?;
    let an = norm_bits(a)?;
    let bn = norm_bits(b)?;
    let hn = norm_bits(h)?;
    let entry_bound = ((n - 1) as u64)
        .checked_mul(an.max(bn))
        .context("entry bound overflow")?
        .max(hn)
        .max(1);
    ensure!(
        entry_bound <= 4096,
        "entry height cap before powers/allocation"
    );
    let minor_bound = (size as u64)
        .checked_mul(entry_bound + ceil_log(size))
        .context("minor bound overflow")?;
    ensure!(minor_bound <= 600_000, "minor/determinant height envelope");
    // Two full matrices at the worst minor height; arithmetic temporaries and
    // allocator overhead still rely on the explicit 4GiB process bound.
    let aggregate_bits = 2u64
        .checked_mul(count as u64)
        .and_then(|x| x.checked_mul(minor_bound))
        .context("matrix bit overflow")?;
    ensure!(
        aggregate_bits <= 2 * 1024 * 1024 * 1024u64 * 8,
        "aggregate integer payload cap"
    );
    let mut ap = vec![a.one()];
    let mut bp = vec![b.one()];
    let neg_b = -b.clone();
    for j in 1..n {
        ap.push(checked_mul(&ap[j - 1], a, z)?);
        bp.push(checked_mul(&bp[j - 1], &neg_b, z)?);
    }
    let mut entries = vec![Integer::zero(); count];
    for j in 0..n {
        let w = checked_mul(&ap[n - 1 - j], &bp[j], z)?;
        for (c, e) in w.coefficients.iter().zip(w.exponents_iter()) {
            entries[e[z] as usize * size + j] = c.numerator();
        }
    }
    for k in 0..remainder_columns {
        for (c, e) in h.coefficients.iter().zip(h.exponents_iter()) {
            entries[(e[z] as usize + k) * size + n + k] = c.numerator();
        }
    }
    ensure!(
        entries.iter().all(|x| x.significant_bits() <= entry_bound),
        "matrix entries exceed preflight"
    );
    let actual_bits = entries
        .iter()
        .map(Integer::significant_bits)
        .max()
        .unwrap_or(0);
    let nonzero = entries.iter().filter(|x| !x.is_zero()).count();
    // Stable row-major, length-prefixed signed-decimal digest; no huge matrix JSON.
    let mut digest = Sha256::new();
    for entry in &entries {
        let s = entry.to_string();
        digest.update((s.len() as u64).to_le_bytes());
        digest.update(s.as_bytes());
    }
    let matrix = Matrix::from_linear(entries, size as u32, size as u32, Z)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((
        matrix,
        json!({"dimension":size,"entries":count,"nonzero_entries":nonzero,
        "declared_n":n,"declared_d":d,"H_shift_columns":remainder_columns,
        "actual_H_degree":h.degree(z),"actual_a_degree":a.degree(z),"actual_b_degree":b.degree(z),
        "specialized_degree_drop":h.is_zero()||h.degree(z) as usize!=n,
        "entry_bit_bound":entry_bound,"actual_max_entry_bits":actual_bits,"minor_bit_bound":minor_bound,
        "aggregate_integer_payload_bit_bound":aggregate_bits,
        "matrix_sha256":format!("{:x}",digest.finalize()),"matrix_hash_encoding":"row-major u64-LE byte length then signed decimal",
        "row_order":"ascending z^0 through z^(d*(N-1))","column_order":"a^(N-1-j)*(-b)^j, then H0*z^k"}),
    ))
}
fn load(args: &Args) -> Result<(Algebra, Poly, Poly, Poly, Value)> {
    let rb = read(&args.ancestry.join("report.json"))?;
    ensure!(hash(&rb) == args.ancestry_sha256, "ancestry report digest");
    let r: Value = serde_json::from_slice(&rb)?;
    let cb = read(&args.capture)?;
    ensure!(
        r["capture"]["sha256"] == hash(&cb),
        "capture ancestry binding"
    );
    let capture: OperationCapture = serde_json::from_slice(&cb)?;
    ensure!(
        capture.schema_version == 1
            && capture.operation == AlgebraOperation::Discriminant
            && capture.variable == Some(1),
        "original discriminant capture"
    );
    let names = &capture.variable_names;
    ensure!(
        (3..=16).contains(&names.len())
            && names[0] == "s"
            && names[1] == "t"
            && names[2] == "x0"
            && names.iter().all(|x| x.len() <= 32),
        "original full map"
    );
    let operands = capture.operands.context("capture operands")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(&operands)?) == capture.input_hash,
        "capture input digest"
    );
    let crb = read(&args.content_report)?;
    ensure!(hash(&crb) == args.content_sha256, "content report digest");
    let cr: Value = serde_json::from_slice(&crb)?;
    ensure!(
        cr["inputs_unchanged"] == true
            && cr["process"]["status"] == "completed"
            && cr["process"]["exit_code"] == 0
            && cr["process"]["output_truncated"] == false,
        "completed content evidence required"
    );
    let bundle_path = args.content_report.with_extension("candidate.json");
    let bb = read(&bundle_path)?;
    ensure!(
        hash(&bb) == cr["result"]["candidate_sha256"],
        "content candidate digest"
    );
    let bundle: Value = serde_json::from_slice(&bb)?;
    ensure!(
        bundle["schema_version"] == 1
            && bundle["variables"] == json!(names)
            && bundle["capture_sha256"] == hash(&cb)
            && bundle["ancestry_sha256"] == args.ancestry_sha256,
        "content input bindings"
    );
    let algebra = Algebra::new(names)?;
    let mut input = Vec::new();
    let mut expressions = Vec::new();
    for label in ["p", "f", "g"] {
        let b = read(&args.ancestry.join(format!("{label}.json")))?;
        ensure!(
            hash(&b) == r["polynomials"][label]["artifact"]["sha256"],
            "ancestor file hash"
        );
        let value: Value = serde_json::from_slice(&b)?;
        ensure!(
            value["schema_version"] == 1 && value["variable_names"] == json!(names),
            "ancestor variable map"
        );
        let expr = value["expression"]
            .as_str()
            .context("ancestor expression")?;
        ensure!(
            hash(expr.as_bytes()) == value["expression_sha256"],
            "ancestor expression hash"
        );
        input.push(parse(&algebra, expr)?);
        expressions.push(expr.to_owned());
    }
    ensure!(
        expressions[0] == operands[0]
            && hash(&serde_json::to_vec(&expressions[1..])?) == r["resultant_input_pair_digest"],
        "ordered raw parent binding"
    );
    ensure!(
        input
            .iter()
            .all(|p| (3..p.nvars()).all(|i| p.degree(i) == 0))
            && input[0].degree(1) == 32
            && input[1].degree(2) == 4
            && input[2].degree(2) == 8
            && input[2].degree(1) == 6,
        "authentic ancestry support envelope"
    );
    progress(
        &args.output,
        "replay exact ancestor gate and coefficient content",
    )?;
    let replay = gate(&algebra, &input[0], &input[1], &input[2], 1, 2, |p| {
        progress(&args.output, p)
    })?;
    ensure!(replay.raw.is_some(), "ancestor gate declined");
    let content = content_screen(&algebra, &replay.h, 2, 32)?;
    for key in [
        "H",
        "integer_primitive_H",
        "content",
        "residual",
        "signed_scalar",
        "degree",
    ] {
        ensure!(
            content[key] == bundle["content_screen"][key],
            "content reconstruction differs: {key}"
        );
    }
    let aa = coeff(&input[1], 1, 1);
    let b = coeff(&input[1], 1, 0);
    let h = parse(
        &algebra,
        content["residual"]["expression"]
            .as_str()
            .context("primitive H")?,
    )?;
    ensure!(
        aa.degree(2) == 4 && b.degree(2) == 4 && h.degree(2) == 32 && h.degree(0) == 20,
        "actual primitive degree envelope"
    );
    Ok((
        algebra,
        aa,
        b,
        h,
        json!({"candidate_sha256":hash(&bb),"H0_expression_sha256":content["residual"]["expression_sha256"],
        "signed_content_scalar":content["signed_scalar"],"content_expression":content["content"]["expression"],
        "full_discriminant_identity":"kappa^62*delta^372*Disc(p) = c^248*C0^2*Disc(H0)",
        "all_correction_factors_retained":true}),
    ))
}
fn constant_shift(algebra: &Algebra, a: &Poly, b: &Poly) -> Result<Value> {
    let zero = Rational::zero();
    let a0 = a.replace(0, &zero);
    let b0 = b.replace(0, &zero);
    if a0.is_zero() {
        return Ok(json!({"outcome":"declined: a(0,z) is zero"}));
    }
    let theta = if b0.is_zero() {
        Rational::zero()
    } else {
        -(&b0.coefficients[0] / &a0.coefficients[0])
    };
    ensure!(
        theta.numerator_ref().significant_bits() <= 256
            && theta.denominator_ref().significant_bits() <= 256,
        "constant-shift rational height cap"
    );
    if a0.clone().mul_coeff(theta.clone()) != -b0 {
        return Ok(json!({"outcome":"declined: no constant proportionality at s=0"}));
    }
    ensure!(
        height(a) <= 4096 - 256 && height(b) <= 4096 - 256,
        "translation pre-product cap"
    );
    let p = theta.numerator();
    let q = theta.denominator();
    let numerator = (-b.clone()).mul_coeff(Rational::from(q.clone()))
        - a.clone().mul_coeff(Rational::from(p.clone()));
    if numerator.is_zero() {
        return Ok(
            json!({"outcome":"declined: globally constant rational map","theta0":theta.to_string()}),
        );
    }
    let k = numerator
        .exponents_iter()
        .map(|e| e[0])
        .min()
        .context("nonzero shifted numerator")?;
    ensure!(k > 0 && k <= 128, "exact positive coordinate valuation");
    let mut residual = numerator.zero();
    for (c, e) in numerator
        .coefficients
        .iter()
        .zip(numerator.exponents_iter())
    {
        let mut e = e.to_vec();
        e[0] -= k;
        residual.append_monomial(c.clone(), &e);
    }
    let s = algebra.parse(&algebra.names[0])?;
    ensure!(
        &residual * &s.pow(usize::try_from(k)?) == numerator,
        "shifted numerator exact reconstruction"
    );
    let text = algebra.format(&residual);
    Ok(
        json!({"outcome":"exact constant shift and coordinate deflation","theta0":theta.to_string(),
        "p":p.to_string(),"q":q.to_string(),"s_power":k,"residual":text,"residual_sha256":hash(text.as_bytes()),
        "residual_degrees":(0..residual.nvars()).map(|i|residual.degree(i)).collect::<Vec<_>>(),
        "residual_terms":residual.nterms(),"residual_height":height(&residual),
        "identity":"q*(-b)-p*a = s^k*B0","identity_verified":true,
        "column_identity":"a^(N-1-j)*(-b-theta0*a)^j = sum_{i<=j} binom(j,i)*(-theta0)^(j-i)*a^(N-1-i)*(-b)^i",
        "index_identity":"q^496*C0 = s^(496*k)*C_shift; C_shift uses B0^j and unchanged H0 columns",
        "retained_s_power":496*k,"retained_q_power":496,
        "full_normalization":"c^248 and all delta/leading/collision factors remain required",
        "determinant_computed":false,"interpolation_claimed":false}),
    )
}
fn worker(args: &Args) -> Result<()> {
    let start = Instant::now();
    let (algebra, aa, b, h, bindings) = load(args)?;
    let replay_seconds = start.elapsed().as_secs_f64();
    if args.shift_only {
        progress(&args.output, "exact constant-shift/content diagnosis only")?;
        let then = Instant::now();
        let shift = constant_shift(&algebra, &aa, &b)?;
        let path = args.output.with_extension("image.json");
        let report = json!({"schema_version":1,"kind":"exact_constant_shift_diagnostic","bindings":bindings,
            "shift":shift,"replay_seconds":replay_seconds,"shift_seconds":then.elapsed().as_secs_f64(),
            "discriminant_verified":false,"determinant_computed":false});
        write_new(&path, &report)?;
        println!(
            "{}",
            json!({"kind":"exact_constant_shift_diagnostic","image":path,
            "image_sha256":hash(&read(&path)?),"outcome":shift["outcome"],"shift_seconds":report["shift_seconds"]})
        );
        return Ok(());
    }
    progress(
        &args.output,
        "bounded integer specialization and matrix construction",
    )?;
    let then = Instant::now();
    let av = specialize(&aa, args.point, 2)?;
    let bv = specialize(&b, args.point, 2)?;
    let hv = specialize(&h, args.point, 2)?;
    let (matrix, stats) = index_matrix(&av, &bv, &hv, 2, 32, 4)?;
    let construction_seconds = then.elapsed().as_secs_f64();
    let temporary = args.output.with_extension("progress.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec(&json!({"phase":"Symbolica integer Bareiss determinant",
        "matrix":stats,"point":args.point,"replay_seconds":replay_seconds,
        "construction_seconds":construction_seconds,"determinant_completed":false}))?,
    )?;
    fs::rename(temporary, args.output.with_extension("progress.json"))?;
    let then = Instant::now();
    let determinant = matrix.det().map_err(|e| anyhow::anyhow!("{e}"))?;
    let determinant_seconds = then.elapsed().as_secs_f64();
    ensure!(
        determinant.significant_bits() <= stats["minor_bit_bound"].as_u64().context("minor cap")?,
        "determinant height exceeds bound"
    );
    let text = determinant.to_string();
    let image = json!({"schema_version":1,"kind":"exact_integer_collision_index_image","point":args.point,
        "bindings":bindings,"matrix":stats,"index_value":text,"index_sha256":hash(text.as_bytes()),
        "index_bits":determinant.significant_bits(),"zero":determinant.is_zero(),"replay_seconds":replay_seconds,
        "construction_seconds":construction_seconds,"determinant_seconds":determinant_seconds,"total_seconds":start.elapsed().as_secs_f64(),
        "method":"Symbolica Matrix<Z>::det fraction-free Bareiss","full_interpolation":false,"discriminant_verified":false,
        "note":"Exact polynomial determinant image only; all normalization and collision factors remain. Special points do not justify cancelling factors in a specialized discriminant."});
    let path = args.output.with_extension("image.json");
    write_new(&path, &image)?;
    println!(
        "{}",
        json!({"kind":"exact_integer_collision_index_image","image":path,"image_sha256":hash(&read(&path)?),
        "point":args.point,"index_bits":determinant.significant_bits(),"zero":determinant.is_zero(),"matrix":stats,
        "replay_seconds":replay_seconds,"construction_seconds":construction_seconds,"determinant_seconds":determinant_seconds})
    );
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.point.unsigned_abs() <= 64
            && args.seconds.is_finite()
            && args.seconds > 0.
            && args.seconds <= 60.,
        "point/time limits"
    );
    for digest in [&args.ancestry_sha256, &args.content_sha256] {
        ensure!(
            digest.len() == 64 && digest.bytes().all(|c| c.is_ascii_hexdigit()),
            "digest syntax"
        );
    }
    if args.worker {
        return worker(&args);
    }
    for path in [
        &args.output,
        &args.output.with_extension("image.json"),
        &args.output.with_extension("progress.json"),
    ] {
        ensure!(!path.exists(), "fresh output required");
    }
    if let Some(p) = args.output.parent() {
        fs::create_dir_all(p)?;
    }
    let paths = vec![
        args.ancestry.join("report.json"),
        args.ancestry.join("p.json"),
        args.ancestry.join("f.json"),
        args.ancestry.join("g.json"),
        args.capture.clone(),
        args.content_report.clone(),
        args.content_report.with_extension("candidate.json"),
    ];
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
        .filter(|v| {
            v["kind"] == "exact_integer_collision_index_image"
                || v["kind"] == "exact_constant_shift_diagnostic"
        });
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
        &json!({"schema_version":1,"input_paths":paths,"input_sha256":before,"inputs_unchanged":before==after,
        "binary_sha256":hash(&fs::read(exe)?),"cpu":args.cpu,"point":args.point,"limits":limits,"process":process,"result":result}),
    )?;
    ensure!(
        success && before == after,
        "incomplete image or modified inputs"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::projection::exact_resultant_ducos;
    fn alg() -> Algebra {
        Algebra::new(&["s".into(), "t".into(), "z".into()]).unwrap()
    }
    fn disc(p: &Poly, v: usize) -> Poly {
        assert!(p.degree(v) >= 2);
        let r = exact_resultant_ducos(p, &p.derivative(v), v).unwrap();
        let (mut q, rem) = r.quot_rem(&p.univariate_lcoeff(v), false);
        assert!(rem.is_zero());
        if p.degree(v) % 4 == 2 || p.degree(v) % 4 == 3 {
            q = -q;
        }
        q
    }
    fn small_identity(hs: &str, aa: &str, bb: &str, d: usize, points: &[i64]) {
        let a = alg();
        let h = a.parse(hs).unwrap();
        let ap = a.parse(aa).unwrap();
        let bp = a.parse(bb).unwrap();
        let f = &(&ap * &a.parse("t").unwrap()) + &bp;
        assert_eq!(
            f.degree(2) as usize,
            d,
            "native resultant uses actual input degree"
        );
        let r = exact_resultant_ducos(&h, &f, 2).unwrap();
        let dr = disc(&r, 1);
        let dh = disc(&h, 2);
        for &point in points {
            let hv = specialize(&h, point, 2).unwrap();
            let av = specialize(&ap, point, 2).unwrap();
            let bv = specialize(&bp, point, 2).unwrap();
            let (m, _) = index_matrix(&av, &bv, &hv, 2, h.degree(2) as usize, d).unwrap();
            let c = m.det().unwrap();
            let lhs = dr.replace(0, &Rational::from(point));
            let rhs = dh
                .replace(0, &Rational::from(point))
                .mul_coeff(Rational::from(c.pow(2)));
            assert_eq!(lhs, rhs, "point {point}");
        }
    }
    #[test]
    fn exact_small_matrix_matches_independent_discriminants_and_signed_scaling() {
        small_identity("z^2-s", "z^2+1", "z", 2, &[2, 5, 10]);
        small_identity("-3*(z^2-1)", "-2", "-2*s*z", 1, &[-2, 0, 3]);
        small_identity("-3*(z^2-1)", "z^2+1", "s*z", 2, &[-2, 0, 3]);
        small_identity("2*z^3+z+s", "z^2+1", "z+2", 2, &[1, 5]);
    }
    #[test]
    fn collision_square_and_row_pivot_sign_are_retained() {
        let a = alg();
        let h = a.parse("z^2-1").unwrap();
        for s in [-2, 0, 3] {
            let b = a.parse("z").unwrap().mul_coeff(Rational::from(s));
            let (m, _) = index_matrix(&a.parse("1").unwrap(), &b, &h, 2, 2, 1).unwrap();
            assert_eq!(m.det().unwrap(), Integer::from(-s));
        }
        let (m, _) =
            index_matrix(&a.parse("z").unwrap(), &a.parse("1").unwrap(), &h, 2, 2, 1).unwrap();
        assert_eq!(m.det().unwrap(), Integer::one());
        small_identity("z^2-1", "1", "s*z", 1, &[-2, 0, 3]);
    }
    #[test]
    fn leading_degree_loss_uses_polynomial_specialization_without_cancellation() {
        small_identity("s*z^2+z+1", "z^2+1", "z", 2, &[0, 1, 4]);
        let a = alg();
        let h = a.parse("z+1").unwrap();
        let (m, stats) = index_matrix(
            &a.parse("z^2+1").unwrap(),
            &a.parse("z").unwrap(),
            &h,
            2,
            2,
            2,
        )
        .unwrap();
        assert_eq!(stats["specialized_degree_drop"], true);
        assert_eq!(m.det().unwrap().abs(), Integer::one());
    }
    #[test]
    fn matrix_and_specialization_caps_precede_large_allocation() {
        let a = alg();
        let h = a.parse("z^2-1").unwrap();
        let one = a.parse("1").unwrap();
        assert!(index_matrix(&one, &one, &h, 2, 33, 4).is_err());
        assert!(index_matrix(&one, &one, &h, 2, 2, 5).is_err());
        assert!(specialize(&a.parse("s+z").unwrap(), 65, 2).is_err());
        let tall = one.mul_coeff(Rational::from(Integer::from(2).pow(2000)));
        assert!(index_matrix(&tall, &tall, &h, 2, 32, 4).is_err());
    }
    #[test]
    fn rational_constant_shift_retains_column_scale_and_zero_parameter() {
        let algebra = alg();
        let a = algebra.parse("2").unwrap();
        let b = algebra.parse("-1-s*z").unwrap();
        let h = algebra.parse("z^2-1").unwrap();
        let proof = constant_shift(&algebra, &a, &b).unwrap();
        assert_eq!(proof["theta0"], "1/2");
        assert_eq!(proof["s_power"], 1);
        assert_eq!(proof["q"], "2");
        let residual = algebra.parse(proof["residual"].as_str().unwrap()).unwrap();
        assert_eq!(residual, algebra.parse("2*z").unwrap());
        for point in [0, 3, 5] {
            let bv = specialize(&b, point, 2).unwrap();
            let (original, _) = index_matrix(&a, &bv, &h, 2, 2, 1).unwrap();
            let (shifted, _) = index_matrix(&a, &(-residual.clone()), &h, 2, 2, 1).unwrap();
            assert_eq!(
                original.det().unwrap() * Integer::from(2),
                shifted.det().unwrap() * Integer::from(point)
            );
        }
    }
    #[test]
    fn constant_shift_declines_invalid_and_globally_constant_maps() {
        let a = alg();
        for (left, right, expected) in [
            ("s*z", "z+1", "declined: a(0,z) is zero"),
            (
                "z+1",
                "z^2+1",
                "declined: no constant proportionality at s=0",
            ),
            ("2", "-1", "declined: globally constant rational map"),
        ] {
            assert_eq!(
                constant_shift(&a, &a.parse(left).unwrap(), &a.parse(right).unwrap()).unwrap()["outcome"],
                expected
            );
        }
    }
}
