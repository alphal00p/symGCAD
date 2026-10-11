//! Research-only full projection of a signed square-free product proposal.
//! No production cache, basis registry, or CAD completion authority is modified.
#[allow(dead_code)]
#[path = "profile_operation.rs"]
mod capture_parser;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, factors_checked},
    benchmark::run_bounded,
    optimized_projection::{AlgebraOperation, OperationCapture},
    projection::{discriminant, exact_resultant_ducos},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, default_value_t = 7)]
    cpu: usize,
    /// Factor every generated guard with signed product replay before continuing.
    #[arg(long)]
    factor_guards: bool,
    #[arg(long, default_value_t = 1)]
    levels: usize,
    /// Separate algebra-only experiment: shift active axis j by k*(j+1),
    /// factor once, invert the shifts, and replay the complete signed source.
    #[arg(long)]
    factor_translation: Option<i64>,
    #[arg(long, hide = true)]
    worker: bool,
}

fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(path: &Path, cap: u64) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "regular file required"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(cap + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= cap, "file byte cap");
    Ok(bytes)
}
fn write(path: &Path, v: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(v)?;
    ensure!(bytes.len() <= 32 * 1024 * 1024, "report byte cap");
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}
fn bounded(p: &Poly) -> Result<()> {
    ensure!(p.nvars() <= 64 && p.nterms() <= 20_000, "map/term cap");
    ensure!((0..p.nvars()).all(|i| p.degree(i) <= 256), "degree cap");
    ensure!(
        p.coefficients
            .iter()
            .all(|c| c.numerator_ref().significant_bits() <= 8192
                && c.denominator_ref().significant_bits() <= 8192),
        "coefficient height cap"
    );
    Ok(())
}
fn describe(a: &Algebra, p: &Poly) -> Value {
    let expression = a.format(p);
    json!({"expression_sha256":hash(expression.as_bytes()),"expression":expression,
        "terms":p.nterms(),"degrees":(0..p.nvars()).map(|i|p.degree(i)).collect::<Vec<_>>(),
        "zero":p.is_zero(),"constant":p.is_constant()})
}
fn translate(a: &Algebra, p: &Poly, shifts: &[(usize, i64)]) -> Result<Poly> {
    let mut q = p.clone();
    for &(axis, value) in shifts {
        ensure!(
            axis < a.names.len() && value.unsigned_abs() <= 64,
            "shift bound"
        );
        let replacement = &a.parse(&a.names[axis])? + &q.constant(Rational::from(value));
        q = q.replace_with_poly(axis, &replacement);
        bounded(&q)?;
    }
    Ok(q)
}
fn projection_guards(
    basis: &[Poly],
    axis: usize,
    mut save: impl FnMut(&str, &[usize], Option<&Poly>, f64) -> Result<()>,
) -> Result<Vec<Poly>> {
    let mut guards = Vec::new();
    for (i, p) in basis.iter().enumerate() {
        bounded(p)?;
        let degree = p.degree(axis);
        let kind = if degree == 0 {
            "content"
        } else {
            "leading_coefficient"
        };
        let start = Instant::now();
        let lc = if degree == 0 {
            p.clone()
        } else {
            p.univariate_lcoeff(axis)
        };
        save(kind, &[i], Some(&lc), start.elapsed().as_secs_f64())?;
        guards.push(lc);
        if degree >= 2 {
            save("discriminant", &[i], None, 0.0)?;
            let start = Instant::now();
            let d = discriminant(p, axis)?;
            save(
                "discriminant",
                &[i],
                Some(&d),
                start.elapsed().as_secs_f64(),
            )?;
            ensure!(
                !d.is_zero(),
                "zero discriminant requires rebasing, not omission"
            );
            guards.push(d);
        }
    }
    for i in 0..basis.len() {
        for j in i + 1..basis.len() {
            if basis[i].degree(axis) == 0 || basis[j].degree(axis) == 0 {
                continue;
            }
            save("resultant", &[i, j], None, 0.0)?;
            let start = Instant::now();
            let r = exact_resultant_ducos(&basis[i], &basis[j], axis)?;
            save(
                "resultant",
                &[i, j],
                Some(&r),
                start.elapsed().as_secs_f64(),
            )?;
            ensure!(
                !r.is_zero(),
                "zero pair resultant requires rebasing, not omission"
            );
            guards.push(r);
        }
    }
    Ok(guards)
}
fn worker(args: &Args) -> Result<()> {
    let bytes = read(&args.capture, 8 * 1024 * 1024)?;
    ensure!(hash(&bytes) == args.capture_sha256, "capture hash mismatch");
    let c: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(
        c.schema_version == 1
            && c.coefficient_domain == "Q"
            && c.operation == AlgebraOperation::Factor
            && !c.operands_omitted_for_size,
        "bounded factor capture required"
    );
    ensure!(
        c.variable_names.len() <= 64
            && !c.variable_names.is_empty()
            && c.variable_names.iter().all(|n| n.len() <= 32
                && !n.is_empty()
                && n.as_bytes()[0].is_ascii_alphabetic()
                && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')),
        "bounded variable names"
    );
    let operands = c.operands.as_ref().context("missing operand")?;
    ensure!(
        operands.len() == 1 && hash(&serde_json::to_vec(operands)?) == c.input_hash,
        "operand binding"
    );
    let a = Algebra::new(&c.variable_names)?;
    let p = capture_parser::plan_operand(&a, &operands[0])?;
    bounded(&p)?;
    ensure!(
        !p.is_zero()
            && p.nterms() <= 2048
            && (0..p.nvars()).all(|i| p.degree(i) <= 32)
            && (0..p.nvars()).filter(|&i| p.degree(i) > 0).count() <= 3,
        "initial source cap/zero"
    );
    let mut report = json!({"schema_version":1,"backend":LicenseManager::get_version(),
        "helper_source_sha256":hash(include_bytes!("profile_support_projection.rs")),
        "capture_sha256":hash(&bytes),"input_hash":c.input_hash,"variables":c.variable_names,
        "source":describe(&a,&p),"original_coverage_claimed":false,"cache_insertion":false,
        "phase":"square_free_proposal","levels":[]});
    let output = args.output.join("worker.json");
    write(&output, &report)?;
    if let Some(k) = args.factor_translation {
        ensure!((1..=8).contains(&k), "translation menu bound");
        let shifts = (0..p.nvars())
            .filter(|&i| p.degree(i) > 0)
            .enumerate()
            .map(|(j, i)| (i, k * (j as i64 + 1)))
            .collect::<Vec<_>>();
        let inverse = shifts.iter().map(|&(i, s)| (i, -s)).collect::<Vec<_>>();
        let start = Instant::now();
        let shifted = translate(&a, &p, &shifts)?;
        ensure!(
            translate(&a, &shifted, &inverse)? == p,
            "translation inverse failed"
        );
        report["translation"] = json!({"shifts":shifts,"seconds":start.elapsed().as_secs_f64(),
            "inverse_verified":true,"polynomial":describe(&a,&shifted)});
        report["phase"] = json!("translated_factorization");
        write(&output, &report)?;
        let start = Instant::now();
        let factors = shifted.factor();
        report["factor_seconds"] = json!(start.elapsed().as_secs_f64());
        let mut restored = p.one();
        let mut exported = Vec::new();
        for (q, m) in factors {
            ensure!(m <= 64, "multiplicity cap");
            let r = translate(&a, &q, &inverse)?;
            for _ in 0..m {
                ensure!(
                    restored.nterms().saturating_mul(r.nterms()) <= 8_000_000,
                    "product cap"
                );
                restored = &restored * &r;
                bounded(&restored)?;
            }
            exported.push(json!({"multiplicity":m,"polynomial":describe(&a,&r)}));
        }
        ensure!(restored == p, "restored signed factor product mismatch");
        report["phase"] = json!("complete");
        report["signed_product_verified"] = json!(true);
        report["restored_factors"] = json!(exported);
        report["scope"] = json!(
            "Internal algebra-only affine automorphism; no CAD coordinate transformation or completion"
        );
        return write(&output, &report);
    }
    let start = Instant::now();
    let raw = p.square_free_factorization();
    report["square_free_seconds"] = json!(start.elapsed().as_secs_f64());
    ensure!(raw.len() <= 64, "block count cap");
    let start = Instant::now();
    let mut reconstruction = p.one();
    let mut basis = Vec::new();
    let mut blocks = Vec::new();
    for (q, m) in raw {
        bounded(&q)?;
        ensure!(
            m <= 64 && reconstruction.nterms().saturating_mul(q.nterms()) <= 8_000_000,
            "reconstruction work cap"
        );
        for _ in 0..m {
            reconstruction = &reconstruction * &q;
            bounded(&reconstruction)?;
        }
        blocks.push(json!({"multiplicity":m,"polynomial":describe(&a,&q)}));
        if !q.is_constant() {
            basis.push(q);
        }
    }
    ensure!(
        reconstruction == p,
        "full signed source reconstruction failed"
    );
    report["reconstruction_seconds"] = json!(start.elapsed().as_secs_f64());
    report["blocks"] = json!(blocks);
    report["signed_product_verified"] = json!(true);
    write(&output, &report)?;
    for level in 0..args.levels {
        let Some(axis) = (0..p.nvars())
            .rev()
            .find(|&i| basis.iter().any(|p| p.degree(i) > 0))
        else {
            break;
        };
        ensure!(basis.len() <= 64, "basis count cap");
        report["levels"].as_array_mut().unwrap().push(json!({"axis":axis,
            "basis":basis.iter().map(|q|describe(&a,q)).collect::<Vec<_>>(),"guards":[],"factored_guards":[]}));
        let guards = projection_guards(&basis, axis, |kind, indices, result, seconds| {
            report["phase"] = json!(kind);
            report["pending"] = json!({"level":level,"kind":kind,"basis_indices":indices});
            if let Some(q) = result {
                bounded(q)?;
                report["levels"][level]["guards"].as_array_mut().unwrap().push(json!({
                    "kind":kind,"basis_indices":indices,"seconds":seconds,"polynomial":describe(&a,q)}));
            }
            write(&output, &report)
        })?;
        report["levels"][level]["full_formal_projection_completed"] = json!(true);
        write(&output, &report)?;
        if args.factor_guards {
            basis.clear();
            for (i, g) in guards.iter().enumerate() {
                report["phase"] = json!("factor_guard");
                report["pending"] = json!({"level":level,"guard":i});
                write(&output, &report)?;
                let start = Instant::now();
                let fs = factors_checked(std::slice::from_ref(g))?;
                let seconds = start.elapsed().as_secs_f64();
                ensure!(fs.len() <= 64, "factor count cap");
                for f in &fs {
                    bounded(f)?;
                }
                report["levels"][level]["factored_guards"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({
                    "guard":i,"seconds":seconds,"signed_product_checked":true,
                    "factors":fs.iter().map(|f|describe(&a,f)).collect::<Vec<_>>()}));
                basis.extend(fs);
                write(&output, &report)?;
            }
            basis.sort_by_key(|p| a.format(&canonical(p)));
            basis.dedup();
        } else {
            break;
        }
    }
    report["phase"] = json!("complete");
    report["pending"] = Value::Null;
    report["scope"] = json!(
        "Projection of this source's block subset only; active companion cuts and CAD verification are absent"
    );
    write(&output, &report)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite()
            && args.seconds > 0.0
            && args.seconds <= 600.0
            && (1..=3).contains(&args.levels)
            && (args.levels == 1 || args.factor_guards),
        "research limits"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(!args.output.exists(), "fresh output required");
    let before = read(&args.capture, 8 * 1024 * 1024)?;
    ensure!(hash(&before) == args.capture_sha256, "source pin mismatch");
    fs::create_dir_all(&args.output)?;
    let exe = std::env::current_exe()?;
    let binary = hash(&read(&exe, 1024 * 1024 * 1024)?);
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .arg(&args.capture)
        .arg("--capture-sha256")
        .arg(&args.capture_sha256)
        .arg("--output")
        .arg(&args.output)
        .arg("--seconds")
        .arg(args.seconds.to_string())
        .arg("--levels")
        .arg(args.levels.to_string())
        .arg("--worker");
    if args.factor_guards {
        command.arg("--factor-guards");
    }
    if let Some(k) = args.factor_translation {
        command.arg("--factor-translation").arg(k.to_string());
    }
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let process = run_bounded(&mut command, &limits)?;
    write(
        &args.output.join("process.json"),
        &json!({"process":process,"limits":limits,"cpu":args.cpu,
        "binary_sha256":binary,"capture_sha256":args.capture_sha256,"no_cad_claim":true}),
    )?;
    ensure!(
        hash(&read(&args.capture, 8 * 1024 * 1024)?) == args.capture_sha256,
        "input changed"
    );
    ensure!(
        hash(&read(&exe, 1024 * 1024 * 1024)?) == binary,
        "binary changed"
    );
    ensure!(
        process.status == "completed" && process.exit_code == Some(0) && !process.output_truncated,
        "incomplete gate; partial formal guards retained"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_signed_square_free_blocks_preserve_repeated_seam() {
        let a = Algebra::new(&["u".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("-3/7*(x-u)^2*(x^2+v)").unwrap();
        let factors = p.square_free_factorization();
        let product = factors
            .iter()
            .fold(p.one(), |acc, (f, m)| &acc * &f.pow(*m));
        assert_eq!(p, product);
        let basis = factors
            .into_iter()
            .filter(|(q, _)| !q.is_constant())
            .map(|(q, _)| q)
            .collect::<Vec<_>>();
        let mut records = Vec::new();
        projection_guards(&basis, 2, |kind, _, p, _| {
            if let Some(p) = p {
                records.push((kind.to_string(), p.clone()));
            }
            Ok(())
        })
        .unwrap();
        assert!(
            records
                .iter()
                .any(|(k, p)| k == "resultant" && !p.is_zero())
        );
        assert!(
            records
                .iter()
                .any(|(k, p)| k == "discriminant" && p == &a.parse("-4*v").unwrap())
        );
    }
    #[test]
    fn zero_collision_is_retained_then_rejected_and_content_stays() {
        let a = Algebra::new(&["u".into(), "x".into()]).unwrap();
        let p = a.parse("x-u").unwrap();
        let mut zero_seen = false;
        let r = projection_guards(&[p.clone(), p], 1, |kind, _, p, _| {
            zero_seen |= kind == "resultant" && p.is_some_and(Poly::is_zero);
            Ok(())
        });
        assert!(r.is_err() && zero_seen);
        let gs = projection_guards(
            &[a.parse("u").unwrap(), a.parse("u*x+1").unwrap()],
            1,
            |_, _, _, _| Ok(()),
        )
        .unwrap();
        assert_eq!(gs, vec![a.parse("u").unwrap(), a.parse("u").unwrap()]);
    }
    #[test]
    fn signed_translation_inverse_preserves_full_source_and_inactive_map() {
        let a = Algebra::new(&["u".into(), "v".into(), "x".into(), "unused".into()]).unwrap();
        let p = a.parse("-2/7*(x-u)^2*(u*v+1)").unwrap();
        let q = translate(&a, &p, &[(0, 2), (1, 4), (2, 6)]).unwrap();
        assert_eq!(translate(&a, &q, &[(0, -2), (1, -4), (2, -6)]).unwrap(), p);
        assert_eq!(q.degree(3), 0);
        assert!(translate(&a, &p, &[(3, 65)]).is_err());
    }
}
