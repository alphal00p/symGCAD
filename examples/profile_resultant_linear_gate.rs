//! Research-only polynomial gate for a linear-ancestor discriminant reduction.
//! No discriminants, quotient-algebra determinants, or solver cache writes.
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
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, rational},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
    projection::exact_resultant_ducos,
};

#[derive(Parser)]
struct Args {
    ancestry: PathBuf,
    #[arg(long)]
    ancestry_sha256: String,
    #[arg(long)]
    capture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 12)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    /// After replaying the gate, extract exact Q[s] coefficient content of H.
    /// Retain both polynomial content and signed rational scalar; no factoring.
    #[arg(long)]
    h_content: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
pub(crate) fn read(p: &Path) -> Result<Vec<u8>> {
    ensure!(fs::metadata(p)?.is_file(), "regular input required");
    let mut b = Vec::new();
    fs::File::open(p)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut b)?;
    ensure!(b.len() <= 2 * 1024 * 1024, "input byte cap");
    Ok(b)
}
fn write_new(p: &Path, v: &Value) -> Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)?
        .write_all(&serde_json::to_vec_pretty(v)?)?;
    Ok(())
}
// Sparse integer grammar only, with bounds before any expression/power conversion.
pub(crate) fn parse(a: &Algebra, s: &str) -> Result<Poly> {
    ensure!(s.len() <= 1024 * 1024, "expression bytes");
    let mut p = a.parse("0")?;
    let mut seen = HashSet::new();
    for (i, t) in s.split('+').enumerate() {
        ensure!(i < 5000, "input term cap");
        let (c, tail) = t
            .strip_prefix('(')
            .context("coefficient prefix")?
            .split_once(')')
            .context("coefficient suffix")?;
        let digits = c.strip_prefix('-').unwrap_or(c);
        ensure!(
            !digits.is_empty() && digits.len() <= 620 && digits.bytes().all(|b| b.is_ascii_digit()),
            "integer grammar/height"
        );
        let c = rational(c)?;
        ensure!(
            !c.is_zero() && c.numerator_ref().significant_bits() <= 2048,
            "coefficient cap"
        );
        let mut e = vec![0u32; a.names.len()];
        if !tail.is_empty() {
            for m in tail
                .strip_prefix('*')
                .context("monomial prefix")?
                .split('*')
            {
                let (name, power) = m.split_once('^').unwrap_or((m, "1"));
                let v = a
                    .names
                    .iter()
                    .position(|n| n == name)
                    .context("unknown axis")?;
                ensure!(
                    !power.is_empty()
                        && power.len() <= 3
                        && power.bytes().all(|b| b.is_ascii_digit()),
                    "power grammar"
                );
                let power: u32 = power.parse()?;
                ensure!(
                    e[v] == 0 && (1..=128).contains(&power),
                    "power or repeated axis"
                );
                e[v] = power;
            }
        }
        ensure!(
            e.iter().sum::<u32>() <= 256 && seen.insert(e.clone()),
            "degree or duplicate term"
        );
        p.append_monomial(c, &e);
    }
    Ok(p)
}
pub(crate) fn coeff(p: &Poly, v: usize, k: u32) -> Poly {
    let mut q = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        if e[v] == k {
            let mut e = e.to_vec();
            e[v] = 0;
            q.append_monomial(c.clone(), &e);
        }
    }
    q
}
fn bits(p: &Poly) -> u64 {
    p.coefficients
        .iter()
        .map(|c| c.numerator_ref().significant_bits())
        .max()
        .unwrap_or(0) as u64
}
fn summary(a: &Algebra, p: &Poly) -> Value {
    json!({"terms":p.nterms(),"degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),
        "max_numerator_bits":bits(p),"expression_sha256":hash(a.format(p).as_bytes())})
}
fn ceil_log(n: usize) -> u64 {
    if n <= 1 {
        0
    } else {
        usize::BITS as u64 - (n - 1).leading_zeros() as u64
    }
}
#[derive(Default)]
struct Budget {
    product_pairs: usize,
}
impl Budget {
    fn post(&self, p: &Poly) -> Result<()> {
        ensure!(
            p.nterms() <= 30_000
                && bits(p) <= 4096
                && p.coefficients.iter().all(|c| c.is_integer()),
            "product output cap"
        );
        Ok(())
    }
    fn mul(&mut self, p: &Poly, q: &Poly) -> Result<Poly> {
        if p.is_zero() || q.is_zero() {
            return Ok(p.zero());
        }
        let mut dense = 1usize;
        for i in 0..p.nvars() {
            let d = p
                .degree(i)
                .checked_add(q.degree(i))
                .context("degree overflow")?;
            ensure!(d <= 128, "product degree cap");
            dense = dense
                .checked_mul(d as usize + 1)
                .context("dense support overflow")?;
        }
        ensure!(dense <= 30_000, "pre-product dense support cap");
        ensure!(
            bits(p) + bits(q) + ceil_log(p.nterms().min(q.nterms())) <= 4096,
            "pre-product coefficient cap"
        );
        let pairs = p
            .nterms()
            .checked_mul(q.nterms())
            .context("pair overflow")?;
        ensure!(pairs <= 4_000_000, "single product cap");
        self.product_pairs = self
            .product_pairs
            .checked_add(pairs)
            .context("work overflow")?;
        ensure!(self.product_pairs <= 30_000_000, "total product cap");
        let r = p * q;
        self.post(&r)?;
        Ok(r)
    }
    fn add(&self, p: &mut Poly, q: &Poly) -> Result<()> {
        ensure!(
            bits(p).max(bits(q)) + 1 <= 4096 && p.nterms() + q.nterms() <= 60_000,
            "sum preflight"
        );
        *p = &*p + q;
        self.post(p)
    }
    fn powers(&mut self, p: &Poly, n: usize) -> Result<Vec<Poly>> {
        let mut out = vec![p.one()];
        for i in 0..n {
            out.push(self.mul(&out[i], p)?)
        }
        Ok(out)
    }
}
fn resultant(budget: &Budget, p: &Poly, q: &Poly, z: usize) -> Result<Poly> {
    let d = p.degree(z) as usize;
    let e = q.degree(z) as usize;
    ensure!(
        (1..=8).contains(&d) && (1..=8).contains(&e) && p.nterms() + q.nterms() <= 2048,
        "resultant input caps"
    );
    let mut dense = 1usize;
    for i in 0..p.nvars() {
        if i != z {
            let degree = e * (p.degree(i) as usize) + d * (q.degree(i) as usize);
            ensure!(degree <= 128, "resultant degree bound");
            dense = dense
                .checked_mul(degree + 1)
                .context("resultant dense bound overflow")?;
        }
    }
    ensure!(dense <= 30_000, "resultant support bound");
    // Every determinant term uses e coefficients of p and d of q; N! terms.
    let bound = (e as u64) * (bits(p) + ceil_log(p.nterms()))
        + (d as u64) * (bits(q) + ceil_log(q.nterms()))
        + ((d + e) as u64) * ceil_log(d + e);
    ensure!(bound <= 4096, "resultant coefficient bound");
    let r = exact_resultant_ducos(p, q, z)?;
    budget.post(&r)?;
    Ok(r)
}
pub(crate) struct Gate {
    report: Value,
    pub(crate) h: Poly,
    delta: Option<Poly>,
    rho0: Option<Poly>,
    pub(crate) raw: Option<Poly>,
}
pub(crate) fn gate(
    a: &Algebra,
    p: &Poly,
    f: &Poly,
    g: &Poly,
    t: usize,
    z: usize,
    mut phase: impl FnMut(&str) -> Result<()>,
) -> Result<Gate> {
    let start = Instant::now();
    ensure!(t < p.nvars() && z < p.nvars() && t != z, "axis indices");
    ensure!(
        f.degree(t) == 1 && p.degree(z) == 0,
        "linear ancestor / eliminated axis"
    );
    let aa = coeff(f, t, 1);
    let b = coeff(f, t, 0);
    let m = g.degree(t) as usize;
    let d = f.degree(z);
    let e = g.degree(z);
    ensure!(
        matches!(d, 2 | 4) && (1..=6).contains(&m) && (1..=8).contains(&e),
        "small even-degree gate envelope"
    );
    ensure!(
        aa.degree(z) == d && b.degree(z) == d && coeff(g, t, m as u32).degree(z) == e,
        "actual ancestor degrees"
    );
    ensure!(
        (0..p.nvars())
            .filter(|i| f.degree(*i) > 0 || g.degree(*i) > 0)
            .count()
            <= 3,
        "three active axes"
    );
    ensure!(
        f.nterms() <= 128 && g.nterms() <= 1024 && p.nterms() <= 5000,
        "input support envelope"
    );
    let max_n = d
        .checked_mul(m as u32)
        .and_then(|x| x.checked_add(e))
        .context("N overflow")?;
    ensure!(max_n <= 32, "finite algebra degree cap");
    let axis = a.parse(&a.names[t])?;
    let mut budget = Budget::default();
    ensure!(&budget.mul(&aa, &axis)? + &b == *f, "linear reconstruction");
    phase("polynomial H and geometric quotient")?;
    let ap = budget.powers(&aa, m)?;
    let bp = budget.powers(&(-b.clone()), m)?;
    let at = budget.mul(&aa, &axis)?;
    let atp = budget.powers(&at, m)?;
    let mut h = p.zero();
    let mut quotient = p.zero();
    for j in 0..=m {
        let gj = coeff(g, t, j as u32);
        let term = budget.mul(&gj, &bp[j])?;
        let term = budget.mul(&term, &ap[m - j])?;
        budget.add(&mut h, &term)?;
        for k in 0..j {
            let term = budget.mul(&gj, &ap[m - j])?;
            let term = budget.mul(&term, &atp[j - 1 - k])?;
            let term = budget.mul(&term, &bp[k])?;
            budget.add(&mut quotient, &term)?;
        }
    }
    let lhs = budget.mul(&ap[m], g)?;
    let mut rhs = budget.mul(f, &quotient)?;
    budget.add(&mut rhs, &h)?;
    ensure!(lhs == rhs, "exact geometric quotient reconstruction");
    let n = h.degree(z);
    let mut report = json!({"input":summary(a,p),"f":summary(a,f),"g":summary(a,g),
        "a":summary(a,&aa),"b":summary(a,&b),"H":summary(a,&h),"quotient":summary(a,&quotient),
        "target_degree":max_n,"actual_H_degree":n,"exact_linear_identity":true,"exact_remainder_identity":true,
        "infinity_power":max_n.saturating_sub(n),"leading_f":a.format(&coeff(f,z,d)),"leading_a":a.format(&coeff(&aa,z,d)),
        "L":a.format(&coeff(&h,z,n)),"construction_seconds":start.elapsed().as_secs_f64(),
        "product_pairs":budget.product_pairs,"coverage_claimed":false,"discriminants_computed":false,"index_computed":false});
    if h.is_zero() || n != max_n || p.degree(t) != max_n {
        report["outcome"] = json!("declined: degree/infinity gate");
        return Ok(Gate {
            report,
            h,
            delta: None,
            rho0: None,
            raw: None,
        });
    }
    phase("delta resultant")?;
    let then = Instant::now();
    let delta = resultant(&budget, &aa, &b, z)?;
    budget.post(&delta)?;
    report["delta_seconds"] = json!(then.elapsed().as_secs_f64());
    report["delta"] = summary(a, &delta);
    phase("leading resultant")?;
    let then = Instant::now();
    let rho0 = resultant(&budget, &aa, &coeff(g, t, m as u32), z)?;
    budget.post(&rho0)?;
    report["rho0_seconds"] = json!(then.elapsed().as_secs_f64());
    report["rho0"] = summary(a, &rho0);
    if delta.is_zero() || rho0.is_zero() {
        report["outcome"] = json!("declined: noninvertible or leading-degenerate ancestor");
        return Ok(Gate {
            report,
            h,
            delta: Some(delta),
            rho0: Some(rho0),
            raw: None,
        });
    }
    phase("raw parent resultant and canonical scale")?;
    let then = Instant::now();
    let raw = resultant(&budget, f, g, z)?;
    budget.post(&raw)?;
    ensure!(
        canonical(&raw) == *p,
        "raw resultant does not match captured canonical polynomial"
    );
    let scale = &raw.coefficients[0] / &p.coefficients[0];
    ensure!(
        !scale.is_zero() && p.clone().mul_coeff(scale.clone()) == raw,
        "signed rational scale reconstruction"
    );
    ensure!(
        coeff(&raw, t, max_n) == rho0,
        "raw leading coefficient differs from rho0"
    );
    report["raw_resultant_seconds"] = json!(then.elapsed().as_secs_f64());
    report["raw_resultant"] = summary(a, &raw);
    report["kappa"] = json!(scale.to_string());
    report["rho"] = json!(a.format(&coeff(p, t, max_n)));
    report["outcome"] = json!("exact polynomial and cheap characteristic-polynomial gate passed");
    report["exact_raw_resultant_binding"] = json!(true);
    report["exact_leading_identity"] = json!(true);
    report["total_seconds"] = json!(start.elapsed().as_secs_f64());
    Ok(Gate {
        report,
        h,
        delta: Some(delta),
        rho0: Some(rho0),
        raw: Some(raw),
    })
}
fn progress(out: &Path, phase: &str) -> Result<()> {
    let temp = out.with_extension("progress.tmp");
    fs::write(&temp, serde_json::to_vec(&json!({"phase":phase}))?)?;
    fs::rename(temp, out.with_extension("progress.json"))?;
    Ok(())
}
fn payload(a: &Algebra, p: &Poly) -> Value {
    let s = a.format(p);
    json!({"expression":s,"expression_sha256":hash(s.as_bytes()),"summary":summary(a,p)})
}
pub(crate) fn content_screen(
    a: &Algebra,
    h: &Poly,
    z: usize,
    expected_degree: u32,
) -> Result<Value> {
    let start = Instant::now();
    ensure!(
        z < h.nvars()
            && !h.is_zero()
            && (2..=32).contains(&expected_degree)
            && h.degree(z) == expected_degree,
        "content stage degree/zero gate"
    );
    ensure!(
        h.nterms() <= 5000
            && h.degree(0) <= 128
            && (0..h.nvars()).all(|i| i == 0 || i == z || h.degree(i) == 0)
            && h.coefficients
                .iter()
                .all(|c| c.numerator_ref().significant_bits() <= 4096
                    && c.denominator_ref().significant_bits() <= 4096),
        "content input envelope"
    );
    let primitive = canonical(h);
    Budget::default().post(&primitive)?;
    let scalar = &h.coefficients[0] / &primitive.coefficients[0];
    ensure!(
        !scalar.is_zero() && primitive.clone().mul_coeff(scalar.clone()) == *h,
        "signed scalar reconstruction"
    );
    let scalar_seconds = start.elapsed().as_secs_f64();
    let then = Instant::now();
    let mut content = primitive.zero();
    let mut gcd_calls = 0usize;
    for k in 0..=expected_degree {
        let c = coeff(&primitive, z, k);
        if c.is_zero() {
            continue;
        }
        ensure!(
            c.degree(z) == 0 && c.nterms() <= 129 && bits(&c) <= 4096,
            "coefficient gcd envelope"
        );
        if content.is_zero() {
            content = canonical(&c);
        } else {
            content = canonical(&content.gcd(&c));
            gcd_calls += 1;
        }
        ensure!(
            !content.is_zero()
                && content.degree(z) == 0
                && content.degree(0) <= 128
                && content.nterms() <= 129
                && bits(&content) <= 4096,
            "gcd result envelope"
        );
        if content.is_constant() {
            break;
        }
    }
    let gcd_seconds = then.elapsed().as_secs_f64();
    let then = Instant::now();
    let (quotient, remainder) = primitive.quot_rem(&content, false);
    ensure!(
        remainder.is_zero() && quotient.degree(z) == expected_degree,
        "content exact quotient/degree preservation"
    );
    let residual = canonical(&quotient);
    let quotient_scalar = &quotient.coefficients[0] / &residual.coefficients[0];
    ensure!(
        residual.clone().mul_coeff(quotient_scalar.clone()) == quotient,
        "quotient signed scalar reconstruction"
    );
    let scalar = scalar * quotient_scalar;
    let mut budget = Budget::default();
    let reconstruction = budget.mul(&content, &residual)?.mul_coeff(scalar.clone());
    ensure!(reconstruction == *h, "full signed content reconstruction");
    Ok(json!({"kind":"exact_coefficient_content_not_root_omission",
        "degree":expected_degree,"H":summary(a,h),"integer_primitive_H":payload(a,&primitive),
        "content":payload(a,&content),"residual":payload(a,&residual),"signed_scalar":scalar.to_string(),
        "gcd_calls":gcd_calls,"scalar_seconds":scalar_seconds,"gcd_seconds":gcd_seconds,
        "quotient_reconstruction_seconds":then.elapsed().as_secs_f64(),"total_seconds":start.elapsed().as_secs_f64(),
        "exact_identity":"H = signed_scalar * content * residual","identity_verified":true,
        "discriminant_scale_power":2*expected_degree-2,
        "discriminant_identity":"Disc(H) = signed_scalar^(2N-2) * content^(2N-2) * Disc(residual)",
        "content_zero_loci_retained":true,"factorization_performed":false,"discriminants_computed":false}))
}
fn worker(args: &Args) -> Result<()> {
    let rb = read(&args.ancestry.join("report.json"))?;
    ensure!(hash(&rb) == args.ancestry_sha256, "ancestry report binding");
    let report: Value = serde_json::from_slice(&rb)?;
    let cb = read(&args.capture)?;
    ensure!(
        Some(hash(&cb).as_str()) == report["capture"]["sha256"].as_str(),
        "capture binding"
    );
    let capture: OperationCapture = serde_json::from_slice(&cb)?;
    ensure!(
        capture.schema_version == 1
            && capture.operation == AlgebraOperation::Discriminant
            && capture.variable == Some(1),
        "D_t capture"
    );
    let names = capture.variable_names;
    ensure!(
        (3..=16).contains(&names.len())
            && names[0] == "s"
            && names[1] == "t"
            && names[2] == "x0"
            && names.iter().all(|x| x.len() <= 32),
        "full map envelope"
    );
    let a = Algebra::new(&names)?;
    let operands = capture.operands.context("missing capture operands")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(&operands)?) == capture.input_hash,
        "capture operand binding"
    );
    let mut expressions = Vec::new();
    for label in ["p", "f", "g"] {
        let raw = read(&args.ancestry.join(format!("{label}.json")))?;
        ensure!(
            Some(hash(&raw).as_str())
                == report["polynomials"][label]["artifact"]["sha256"].as_str(),
            "sidecar file binding"
        );
        let value: Value = serde_json::from_slice(&raw)?;
        ensure!(
            value["schema_version"] == 1 && value["variable_names"] == json!(names),
            "sidecar map"
        );
        let expression = value["expression"]
            .as_str()
            .context("expression")?
            .to_owned();
        ensure!(
            Some(hash(expression.as_bytes()).as_str()) == value["expression_sha256"].as_str(),
            "expression hash"
        );
        expressions.push(expression);
    }
    ensure!(
        expressions[0] == operands[0]
            && hash(&serde_json::to_vec(&expressions[1..])?)
                == report["resultant_input_pair_digest"],
        "ordered pair binding"
    );
    let p = parse(&a, &expressions[0])?;
    let f = parse(&a, &expressions[1])?;
    let g = parse(&a, &expressions[2])?;
    ensure!(
        (0..names.len())
            .all(|i| i <= 2 || (p.degree(i) == 0 && f.degree(i) == 0 && g.degree(i) == 0)),
        "unexpected active axis"
    );
    ensure!(
        p.degree(1) == 32
            && f.degree(1) == 1
            && f.degree(2) == 4
            && coeff(&f, 1, 1).degree(2) == 4
            && coeff(&f, 1, 0).degree(2) == 4
            && g.degree(1) == 6
            && g.degree(2) == 8
            && coeff(&g, 1, 6).degree(2) == 8,
        "actual captured support gate"
    );
    let result = gate(&a, &p, &f, &g, 1, 2, |phase| progress(&args.output, phase))?;
    let content = if args.h_content {
        ensure!(
            result.raw.is_some(),
            "content screen requires successful gate"
        );
        progress(&args.output, "H signed scalar and coefficient gcd")?;
        Some(content_screen(&a, &result.h, 2, 32)?)
    } else {
        None
    };
    let bundle = json!({"schema_version":1,"kind":"research_linear_ancestor_gate","ancestry_sha256":args.ancestry_sha256,
        "capture_sha256":hash(&cb),"variables":names,"screen":result.report,
        "content_screen":content,
        "H":payload(&a,&result.h),"delta":result.delta.as_ref().map(|p|payload(&a,p)),
        "rho0":result.rho0.as_ref().map(|p|payload(&a,p)),"raw_resultant":result.raw.as_ref().map(|p|payload(&a,p))});
    let candidate = args.output.with_extension("candidate.json");
    write_new(&candidate, &bundle)?;
    println!(
        "{}",
        json!({"kind":"research_linear_ancestor_gate","candidate":candidate,"candidate_sha256":hash(&fs::read(&candidate)?),"screen":bundle["screen"],
            "content_summary":bundle["content_screen"].as_object().map(|v| json!({"content":v["content"]["summary"],
                "residual":v["residual"]["summary"],"signed_scalar":v["signed_scalar"],"gcd_calls":v["gcd_calls"],
                "total_seconds":v["total_seconds"],"identity_verified":v["identity_verified"]}))})
    );
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "seconds in(0,120]"
    );
    ensure!(
        args.ancestry_sha256.len() == 64
            && args.ancestry_sha256.bytes().all(|c| c.is_ascii_hexdigit()),
        "report digest"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists()
            && !args.output.with_extension("candidate.json").exists()
            && !args.output.with_extension("progress.json").exists(),
        "fresh output required"
    );
    if let Some(p) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p)?
    }
    let paths = [
        args.capture.clone(),
        args.ancestry.join("report.json"),
        args.ancestry.join("p.json"),
        args.ancestry.join("f.json"),
        args.ancestry.join("g.json"),
    ];
    let before = paths
        .iter()
        .map(|p| Ok(hash(&read(p)?)))
        .collect::<Result<Vec<_>>>()?;
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new("taskset");
    cmd.args(["-c", &args.cpu.to_string()])
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
        .filter(|v| v["kind"] == "research_linear_ancestor_gate");
    let success = process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.is_some();
    if result.is_some() {
        process.stdout.clear()
    }
    let after = paths
        .iter()
        .map(|p| Ok(hash(&read(p)?)))
        .collect::<Result<Vec<_>>>()?;
    write_new(
        &args.output,
        &json!({"schema_version":1,"inputs":paths,"input_hashes":before,"inputs_unchanged":before==after,"binary_sha256":hash(&fs::read(exe)?),"cpu":args.cpu,"limits":limits,"process":process,"result":result}),
    )?;
    ensure!(
        success && before == after,
        "incomplete bounded gate or changed source"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Algebra, Poly, Poly, Poly) {
        let a = Algebra::new(&["s".into(), "t".into(), "z".into()]).unwrap();
        let f = a.parse("(s+z^2)*t+z^2+1").unwrap();
        let g = a.parse("(z^2+2)*t^2+z+3").unwrap();
        let p = canonical(&exact_resultant_ducos(&f, &g, 2).unwrap());
        (a, p, f, g)
    }
    #[test]
    fn exact_fixture_and_trace_norm_gate() {
        let (a, p, f, g) = fixture();
        let r = gate(&a, &p, &f, &g, 1, 2, |_| Ok(())).unwrap();
        assert_eq!(r.report["exact_raw_resultant_binding"], true);
        assert_eq!(r.report["actual_H_degree"], 6);
        let aa = coeff(&f, 1, 1);
        let e = exact_resultant_ducos(&r.h, &aa, 2).unwrap();
        assert_eq!(
            e,
            &r.delta.as_ref().unwrap().pow(2) * r.rho0.as_ref().unwrap()
        );
        assert_eq!(
            exact_resultant_ducos(&r.h, &f, 2).unwrap(),
            &r.delta.unwrap().pow(2) * r.raw.as_ref().unwrap()
        );
    }
    #[test]
    fn degree_drop_is_an_explicit_decline() {
        let (a, _, f, _) = fixture();
        let g = a.parse("(z^2+2)*t^2-z^2-3").unwrap();
        let p = canonical(&exact_resultant_ducos(&f, &g, 2).unwrap());
        let r = gate(&a, &p, &f, &g, 1, 2, |_| Ok(())).unwrap();
        assert_eq!(r.report["outcome"], "declined: degree/infinity gate");
        assert!(r.report["infinity_power"].as_u64().unwrap() > 0);
        assert!(r.raw.is_none());
    }
    #[test]
    fn forged_polynomial_is_rejected() {
        let (a, p, f, g) = fixture();
        assert!(gate(&a, &(&p + &p.one()), &f, &g, 1, 2, |_| Ok(())).is_err());
    }
    #[test]
    fn common_factor_declines_and_rational_signed_scale_reconstructs() {
        let (a, _, _, g) = fixture();
        let f = a.parse("(z^2+1)*(t+1)").unwrap();
        let p = canonical(&exact_resultant_ducos(&f, &g, 2).unwrap());
        let r = gate(&a, &p, &f, &g, 1, 2, |_| Ok(())).unwrap();
        assert!(r.delta.as_ref().unwrap().is_zero());
        assert!(r.raw.is_none());
        let p = a.parse("-2*t^2+3*t+1").unwrap();
        let raw = p.clone().mul_coeff(rational("-7/3").unwrap());
        let scale = &raw.coefficients[0] / &p.coefficients[0];
        assert_eq!(scale, rational("-7/3").unwrap());
        assert_eq!(p.mul_coeff(scale), raw);
    }
    #[test]
    fn actual_degree_infinity_trap_and_zero_leading_norm_decline() {
        let (a, _, _, _) = fixture();
        let f = a.parse("(z^4+1)*t+z^4+2").unwrap();
        let g = a.parse("(z^8+1)*t^6-z^8").unwrap();
        // Both small norms are nonzero, but four roots lie at infinity in H.
        let r = gate(&a, &a.parse("t^32+1").unwrap(), &f, &g, 1, 2, |_| Ok(())).unwrap();
        assert_eq!(r.report["actual_H_degree"], 28);
        assert_eq!(r.report["infinity_power"], 4);
        assert!(r.raw.is_none());
        let aa = coeff(&f, 1, 1);
        assert_eq!(
            exact_resultant_ducos(&aa, &coeff(&f, 1, 0), 2).unwrap(),
            a.parse("1").unwrap()
        );
        assert_eq!(
            exact_resultant_ducos(&aa, &coeff(&g, 1, 6), 2).unwrap(),
            a.parse("16").unwrap()
        );
        let f = a.parse("(s+z^2)*t+z^2+1").unwrap();
        let g = a.parse("(s+z^2)*t^2+z+3").unwrap();
        let r = gate(&a, &a.parse("t^6+1").unwrap(), &f, &g, 1, 2, |_| Ok(())).unwrap();
        assert!(r.rho0.as_ref().unwrap().is_zero());
        assert!(r.raw.is_none());
    }
    #[test]
    fn sparse_parser_and_preproduct_bounds() {
        let (a, _, _, _) = fixture();
        let p = a.parse("-3*s^2*t+2*z").unwrap();
        assert_eq!(parse(&a, &a.format(&p)).unwrap(), p);
        for text in ["(2)^1000000", "(1)*s^129", "(1)*s+(2)*s", "(1)*s^1^2"] {
            assert!(parse(&a, text).is_err());
        }
        let mut b = Budget::default();
        assert!(
            b.mul(&a.parse("s^100").unwrap(), &a.parse("s^100").unwrap())
                .is_err()
        );
    }
    #[test]
    fn coefficient_content_retains_nonconstant_seams_and_signed_scale() {
        let (a, _, _, _) = fixture();
        let h = a.parse("-7/3*s*(s-4)^2*(z^2+s)").unwrap();
        let result = content_screen(&a, &h, 2, 2).unwrap();
        assert_eq!(result["signed_scalar"], "-7/3");
        assert_eq!(
            a.parse(result["content"]["expression"].as_str().unwrap())
                .unwrap(),
            a.parse("s*(s-4)^2").unwrap()
        );
        let q = a
            .parse(result["residual"]["expression"].as_str().unwrap())
            .unwrap();
        assert_eq!(q, a.parse("z^2+s").unwrap());
        let disc = |p: &Poly| {
            let r = exact_resultant_ducos(p, &p.derivative(2), 2).unwrap();
            let (d, rem) = r.quot_rem(&p.univariate_lcoeff(2), false);
            assert!(rem.is_zero());
            -d
        };
        let c = a.parse("-7/3*s*(s-4)^2").unwrap();
        assert_eq!(disc(&h), &c.pow(2) * &disc(&q));
        assert!(h.replace(0, &rational("4").unwrap()).is_zero());
        assert!(!q.replace(0, &rational("4").unwrap()).is_zero());
        assert_eq!(result["content_zero_loci_retained"], true);
    }
    #[test]
    fn coefficient_content_rejects_degree_drop_and_zero() {
        let (a, _, _, _) = fixture();
        for (text, n) in [("0", 2), ("s*z+1", 2), ("z^3+s", 2), ("z^2+t", 2)] {
            assert!(content_screen(&a, &a.parse(text).unwrap(), 2, n).is_err());
        }
    }
}
