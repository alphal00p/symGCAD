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

const DEFAULT_SOURCE_TERMS: usize = 2048;
const MAX_SOURCE_TERMS: usize = 8192;
const MAX_TRANSLATION_WORK: usize = 1_048_576;
const MAX_PRODUCT_WORK: usize = 8_000_000;
const SHIFT_MENU: [i64; 3] = [1, -1, 2];

fn validate_source_limit(limit: usize) -> Result<()> {
    ensure!(
        (1..=MAX_SOURCE_TERMS).contains(&limit),
        "source term budget must be between 1 and 8192"
    );
    Ok(())
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 7)]
    cpu: usize,
    /// Research source admission only; expansion/product bounds stay fixed.
    #[arg(long, default_value_t = DEFAULT_SOURCE_TERMS)]
    max_source_terms: usize,
    /// Inspect exact degree drops at the fixed affine menu; perform no factors.
    #[arg(long, conflicts_with_all = ["factor_translation", "factor_scale", "native_factor", "factor_guards", "square_free_only"])]
    admission_only: bool,
    /// Also compute exact full univariate fibers and gcd(f, f') at the fixed menu.
    #[arg(long, requires = "admission_only")]
    fiber_gcds: bool,
    /// Factor bounded degree-preserving square-free fibers; gate any source
    /// irreducibility certificate on an exact constant-content check.
    #[arg(long, requires_all = ["admission_only", "fiber_gcds"])]
    fiber_factors: bool,
    /// Replay signed square-free blocks, then stop before any projection.
    #[arg(long, conflicts_with_all = ["factor_translation", "factor_scale", "native_factor", "factor_guards", "admission_only"])]
    square_free_only: bool,
    /// Matched native control: factor the original source and replay its product.
    #[arg(long, conflicts_with_all = ["factor_translation", "factor_scale", "admission_only", "factor_guards"])]
    native_factor: bool,
    /// Factor every generated guard with signed product replay before continuing.
    #[arg(long)]
    factor_guards: bool,
    #[arg(long, default_value_t = 1)]
    levels: usize,
    /// Separate algebra-only experiment: shift active axis j by k*(j+1),
    /// factor once, invert the shifts, and replay the complete signed source.
    #[arg(long, allow_hyphen_values = true)]
    factor_translation: Option<i64>,
    /// Separate algebra-only experiment: one nonzero integer scale per active
    /// axis, in map order (e.g. 2,3,4). Invert over Q and replay signed products.
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true,
        conflicts_with_all = ["factor_translation", "native_factor", "factor_guards", "admission_only", "square_free_only"])]
    factor_scale: Vec<i64>,
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
        "coefficient_bits":p.coefficients.iter().map(|c|c.numerator_ref().significant_bits()
            .max(c.denominator_ref().significant_bits())).max().unwrap_or(0),
        "zero":p.is_zero(),"constant":p.is_constant()})
}
fn translate(a: &Algebra, p: &Poly, shifts: &[(usize, i64)]) -> Result<Poly> {
    ensure!(
        p.variables() == &a.variables,
        "translation full variable map mismatch"
    );
    bounded(p)?;
    let mut q = p.clone();
    let mut work = 0usize;
    for &(axis, value) in shifts {
        ensure!(
            axis < a.names.len() && value.unsigned_abs() <= 64,
            "shift bound"
        );
        let estimate = q
            .nterms()
            .checked_mul(q.degree(axis) as usize + 1)
            .context("translation work overflow")?;
        work = work
            .checked_add(estimate)
            .context("translation work overflow")?;
        ensure!(work <= MAX_TRANSLATION_WORK, "translation work cap");
        let replacement = &a.parse(&a.names[axis])? + &q.constant(Rational::from(value));
        q = q.replace_with_poly(axis, &replacement);
        bounded(&q)?;
    }
    Ok(q)
}

fn scale(a: &Algebra, p: &Poly, scales: &[(usize, i64)], inverse: bool) -> Result<Poly> {
    ensure!(
        p.variables() == &a.variables,
        "scaling full variable map mismatch"
    );
    bounded(p)?;
    ensure!(scales.len() <= 3, "scaling axis count cap");
    let mut seen = Vec::new();
    let mut powers = Vec::new();
    for &(axis, value) in scales {
        ensure!(
            axis < p.nvars() && !seen.contains(&axis),
            "scaling axis map"
        );
        ensure!(
            value != 0 && value.unsigned_abs() <= 64,
            "nonzero scaling bound"
        );
        seen.push(axis);
        let base = if inverse {
            Rational::from((1, value))
        } else {
            Rational::from(value)
        };
        powers.push((
            axis,
            (0..=p.degree(axis))
                .map(|e| base.pow(e as u64))
                .collect::<Vec<_>>(),
        ));
    }
    let mut q = p.clone();
    for (coefficient, exponents) in q.coefficients.iter_mut().zip(p.exponents_iter()) {
        for (axis, table) in &powers {
            *coefficient = &*coefficient * &table[exponents[*axis] as usize];
        }
    }
    bounded(&q)?;
    ensure!(
        q.nterms() == p.nterms() && q.exponents_iter().eq(p.exponents_iter()),
        "nonzero scaling changed monomial support"
    );
    Ok(q)
}

fn source_admission(a: &Algebra, p: &Poly, limit: usize) -> Result<()> {
    validate_source_limit(limit)?;
    ensure!(
        p.variables() == &a.variables,
        "source full variable map mismatch"
    );
    bounded(p)?;
    ensure!(
        !p.is_zero()
            && p.nterms() <= limit
            && (0..p.nvars()).all(|i| p.degree(i) <= 32)
            && (0..p.nvars()).filter(|&i| p.degree(i) > 0).count() <= 3,
        "initial source cap/zero"
    );
    Ok(())
}

fn specialize_fiber(a: &Algebra, p: &Poly, axis: usize, point: &[(usize, i64)]) -> Result<Poly> {
    ensure!(
        p.variables() == &a.variables,
        "fiber full variable map mismatch"
    );
    bounded(p)?;
    let active = (0..p.nvars())
        .filter(|&i| p.degree(i) > 0)
        .collect::<Vec<_>>();
    ensure!(
        active.contains(&axis) && active.len() <= 3 && p.degree(axis) <= 32,
        "fiber axis bound"
    );
    ensure!(
        point.len() == active.len()
            && active
                .iter()
                .all(|i| point.iter().filter(|(j, _)| i == j).count() == 1)
            && point.iter().all(|(_, x)| x.unsigned_abs() <= 129),
        "fiber evaluation map bound"
    );
    let mut specialized = p.clone();
    for &(other, coordinate) in point {
        if other != axis {
            specialized = specialized.replace(other, &Rational::from(coordinate));
            bounded(&specialized)?;
        }
    }
    ensure!(
        (0..p.nvars()).all(|i| i == axis || specialized.degree(i) == 0),
        "non-univariate fiber"
    );
    Ok(specialized)
}

fn fiber_inventory(a: &Algebra, p: &Poly, axis: usize, point: &[(usize, i64)]) -> Result<Value> {
    let specialized = specialize_fiber(a, p, axis, point)?;
    if specialized.is_zero() {
        return Ok(
            json!({"zero":true,"source_degree":p.degree(axis),"degree":null,
            "degree_preserved":false,"gcd_derivative_degree":null,"square_free":false,
            "coefficients_low_to_high":[]}),
        );
    }
    let fiber = specialized.to_univariate_from_univariate(axis);
    let gcd = fiber.gcd(&fiber.derivative());
    Ok(
        json!({"zero":false,"source_degree":p.degree(axis),"degree":fiber.degree(),
        "degree_preserved":fiber.degree() == p.degree(axis) as usize,
        "gcd_derivative_degree":gcd.degree(),"square_free":gcd.degree() == 0,
        "coefficients_low_to_high":fiber.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>(),
        "gcd_coefficients_low_to_high":gcd.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>()}),
    )
}

fn factor_fiber(
    a: &Algebra,
    p: &Poly,
    axis: usize,
    point: &[(usize, i64)],
    mut save: impl FnMut(&Value) -> Result<()>,
) -> Result<Value> {
    let inventory = fiber_inventory(a, p, axis, point)?;
    let mut report = json!({"axis":axis,"evaluation_point":point,"fiber":inventory,
        "source_irreducible_certified":false,"source_content_checked":false});
    if inventory["degree_preserved"] != true || inventory["square_free"] != true {
        report["admitted"] = json!(false);
        report["reason"] = json!("A nonzero degree-preserving square-free image is required");
        return Ok(report);
    }
    let image = specialize_fiber(a, p, axis, point)?;
    let start = Instant::now();
    let raw = image.factor();
    report["factor_seconds"] = json!(start.elapsed().as_secs_f64());
    let multiplicities = raw.iter().map(|(_, m)| *m).collect::<Vec<_>>();
    let start = Instant::now();
    let factors = restore_factors(a, &image, raw, &[])?;
    report["signed_reconstruction_seconds"] = json!(start.elapsed().as_secs_f64());
    let nonconstant = factors
        .iter()
        .zip(&multiplicities)
        .filter(|(q, _)| !q.is_constant())
        .collect::<Vec<_>>();
    let irreducible = nonconstant.len() == 1
        && *nonconstant[0].1 == 1
        && nonconstant[0].0.degree(axis) == p.degree(axis);
    report["admitted"] = json!(true);
    report["signed_product_verified"] = json!(true);
    report["image_irreducible"] = json!(irreducible);
    report["factor_degree_partition"] = json!(
        nonconstant
            .iter()
            .map(|(q, m)| json!({"degree":q.degree(axis),"multiplicity":m}))
            .collect::<Vec<_>>()
    );
    report["factors"] = json!(
        factors
            .iter()
            .zip(&multiplicities)
            .map(|(q, m)| json!({"multiplicity":m,"polynomial":describe(a,q)}))
            .collect::<Vec<_>>()
    );
    save(&report)?;
    if irreducible {
        let start = Instant::now();
        let content = p.univariate_content(axis);
        bounded(&content)?;
        ensure!(!content.is_zero(), "nonzero source has zero content");
        report["source_content_seconds"] = json!(start.elapsed().as_secs_f64());
        report["source_content_checked"] = json!(true);
        report["source_content"] = describe(a, &content);
        report["source_irreducible_certified"] = json!(content.is_constant());
        report["certificate_theorem"] = json!(
            "Constant univariate content and an irreducible degree-preserving specialization imply irreducibility over Q in the complete source ring"
        );
    }
    Ok(report)
}

fn factor_fiber_schedule(p: &Poly) -> Result<Vec<(usize, Option<i64>, Vec<(usize, i64)>)>> {
    let active = (0..p.nvars())
        .filter(|&i| p.degree(i) > 0)
        .collect::<Vec<_>>();
    ensure!(
        active.len() <= 3 && active.iter().all(|&i| p.degree(i) <= 32),
        "fiber schedule bounds"
    );
    let mut order = active.clone();
    order.sort_by_key(|&axis| (p.degree(axis), axis));
    let mut schedule = Vec::new();
    for axis in order {
        for k in [None, Some(1), Some(-1), Some(2)] {
            let point = active
                .iter()
                .enumerate()
                .map(|(j, &i)| (i, 1 + k.unwrap_or(0) * (j as i64 + 1)))
                .collect();
            schedule.push((axis, k, point));
        }
    }
    ensure!(schedule.len() <= 12, "fiber image count bound");
    Ok(schedule)
}

fn degree_drop_diagnostics(a: &Algebra, p: &Poly, fiber_gcds: bool) -> Result<Value> {
    ensure!(
        p.variables() == &a.variables,
        "diagnostic full variable map mismatch"
    );
    let active = (0..p.nvars())
        .filter(|&i| p.degree(i) > 0)
        .collect::<Vec<_>>();
    let leading = active
        .iter()
        .map(|&i| (i, p.univariate_lcoeff(i)))
        .collect::<Vec<_>>();
    let at = |point: &[(usize, i64)]| -> Result<(Vec<usize>, Vec<Value>)> {
        let mut drops = Vec::new();
        let mut values = Vec::new();
        for (axis, coefficient) in &leading {
            let mut value = coefficient.clone();
            for &(other, coordinate) in point {
                if other != *axis {
                    value = value.replace(other, &Rational::from(coordinate));
                }
            }
            if value.is_zero() {
                drops.push(*axis);
            }
            let mut record =
                json!({"axis":axis,"leading_value":a.format(&value),"degree_drop":value.is_zero()});
            if fiber_gcds {
                record["fiber"] = fiber_inventory(a, p, *axis, point)?;
            }
            values.push(record);
        }
        Ok((drops, values))
    };
    let ones = active.iter().map(|&i| (i, 1)).collect::<Vec<_>>();
    let (original, original_values) = at(&ones)?;
    let mut first = None;
    let mut first_gcd = None;
    let mut candidates = Vec::new();
    for k in SHIFT_MENU {
        let shifts = active
            .iter()
            .enumerate()
            .map(|(j, &i)| (i, k * (j as i64 + 1)))
            .collect::<Vec<_>>();
        let point = shifts.iter().map(|&(i, s)| (i, 1 + s)).collect::<Vec<_>>();
        let (drops, values) = at(&point)?;
        let improved = drops.len() < original.len();
        if improved && first.is_none() {
            first = Some(k);
        }
        let gcd_improved = original_values
            .iter()
            .zip(&values)
            .filter_map(|(old, new)| {
                if old["fiber"]["degree_preserved"] == true
                    && new["fiber"]["degree_preserved"] == true
                    && new["fiber"]["gcd_derivative_degree"].as_u64()?
                        < old["fiber"]["gcd_derivative_degree"].as_u64()?
                {
                    old["axis"].as_u64()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !gcd_improved.is_empty() && first_gcd.is_none() {
            first_gcd = Some(k);
        }
        candidates.push(json!({"k":k,"shifts":shifts,"evaluation_point":point,"degree_drop_axes":drops,"degree_drop_count":drops.len(),"leading_values":values,"improves":improved,"degree_preserving_gcd_improved_axes":gcd_improved}));
    }
    Ok(
        json!({"original_point":ones,"original_degree_drop_axes":original,"original_degree_drop_count":original.len(),"original_leading_values":original_values,"candidates":candidates,"first_improving_shift":first,"full_fiber_gcds":fiber_gcds,"first_degree_preserving_gcd_improvement":first_gcd,"scope":"Exact specialization heuristic only, not a factorization or CAD certificate; zero fibers count as degree drops."}),
    )
}

fn restore_factors(
    a: &Algebra,
    source: &Poly,
    factors: Vec<(Poly, usize)>,
    inverse: &[(usize, i64)],
) -> Result<Vec<Poly>> {
    restore_factors_by(a, source, factors, &mut 0, |q| translate(a, q, inverse))
}

fn restore_factors_by(
    a: &Algebra,
    source: &Poly,
    factors: Vec<(Poly, usize)>,
    work: &mut usize,
    mut transform: impl FnMut(&Poly) -> Result<Poly>,
) -> Result<Vec<Poly>> {
    let mut restored = source.one();
    let mut mapped = Vec::new();
    ensure!(factors.len() <= 64, "factor count cap");
    for (q, m) in factors {
        ensure!(
            q.variables() == &a.variables && m > 0 && m <= 64,
            "factor map/multiplicity cap"
        );
        let r = transform(&q)?;
        for _ in 0..m {
            *work = work
                .checked_add(
                    restored
                        .nterms()
                        .checked_mul(r.nterms())
                        .context("product work overflow")?,
                )
                .context("product work overflow")?;
            ensure!(*work <= MAX_PRODUCT_WORK, "product cap");
            restored = &restored * &r;
            bounded(&restored)?;
        }
        mapped.push(r);
    }
    ensure!(
        restored == *source,
        "restored signed factor product mismatch"
    );
    Ok(mapped)
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
    source_admission(&a, &p, args.max_source_terms)?;
    let diagnostic_start = Instant::now();
    let diagnostics = degree_drop_diagnostics(&a, &p, args.fiber_gcds)?;
    let diagnostic_seconds = diagnostic_start.elapsed().as_secs_f64();
    let mut report = json!({"schema_version":1,"backend":LicenseManager::get_version(),
        "helper_source_sha256":hash(include_bytes!("profile_support_projection.rs")),
        "capture_sha256":hash(&bytes),"input_hash":c.input_hash,"variables":c.variable_names,
        "source":describe(&a,&p),"original_coverage_claimed":false,"cache_insertion":false,
        "phase":"square_free_proposal","levels":[],"max_source_terms":args.max_source_terms,
        "degree_drop_diagnostics":diagnostics,"diagnostic_seconds":diagnostic_seconds});
    let output = args.output.join("worker.json");
    write(&output, &report)?;
    if args.fiber_factors {
        report["fiber_factor_inventories"] = json!([]);
        for (axis, k, point) in factor_fiber_schedule(&p)? {
            report["phase"] = json!("fiber_factor_inventory");
            report["pending"] = json!({"axis":axis,"k":k,"point":point});
            write(&output, &report)?;
            let mut inventory = factor_fiber(&a, &p, axis, &point, |inventory| {
                report["pending_fiber_inventory"] = inventory.clone();
                write(&output, &report)
            })?;
            inventory["k"] = json!(k);
            let certified = inventory["source_irreducible_certified"] == true;
            report["fiber_factor_inventories"]
                .as_array_mut()
                .unwrap()
                .push(inventory);
            report["pending_fiber_inventory"] = Value::Null;
            write(&output, &report)?;
            if certified {
                report["source_irreducible_certified"] = json!(true);
                report["stopped_after_certificate"] = json!(true);
                break;
            }
        }
        report["phase"] = json!("fiber_factor_inventories_complete");
        report["pending"] = Value::Null;
        report["scope"] = json!(
            "Exact source-factor diagnostics only; any accepted irreducibility claim includes a degree-preserving image and constant-content gate; no CAD completion or cache insertion"
        );
        return write(&output, &report);
    }
    if args.admission_only {
        report["phase"] = json!("admission_complete");
        return write(&output, &report);
    }
    if !args.factor_scale.is_empty() {
        let active = (0..p.nvars())
            .filter(|&i| p.degree(i) > 0)
            .collect::<Vec<_>>();
        ensure!(
            active.len() == args.factor_scale.len(),
            "one scale per active axis required"
        );
        let scales = active
            .into_iter()
            .zip(args.factor_scale.iter().copied())
            .collect::<Vec<_>>();
        let start = Instant::now();
        let scaled = scale(&a, &p, &scales, false)?;
        ensure!(
            scale(&a, &scaled, &scales, true)? == p,
            "scaling inverse failed"
        );
        report["scaling"] = json!({"scales":scales,"seconds":start.elapsed().as_secs_f64(),
            "inverse_verified":true,"support_preserved":true,"polynomial":describe(&a,&scaled)});
        report["phase"] = json!("scaled_factorization");
        write(&output, &report)?;
        let start = Instant::now();
        let factors = scaled.factor();
        report["factor_seconds"] = json!(start.elapsed().as_secs_f64());
        let multiplicities = factors.iter().map(|(_, m)| *m).collect::<Vec<_>>();
        let start = Instant::now();
        let mut replay_work = 0;
        restore_factors_by(&a, &scaled, factors.clone(), &mut replay_work, |q| {
            scale(&a, q, &[], false)
        })?;
        let restored = restore_factors_by(&a, &p, factors, &mut replay_work, |q| {
            scale(&a, q, &scales, true)
        })?;
        report["reconstruction_seconds"] = json!(start.elapsed().as_secs_f64());
        report["reconstruction_work"] = json!(replay_work);
        report["phase"] = json!("complete");
        report["signed_scaled_product_verified"] = json!(true);
        report["signed_product_verified"] = json!(true);
        report["restored_factors"] = json!(
            restored
                .iter()
                .zip(multiplicities)
                .map(|(r, m)| json!({"multiplicity":m,"polynomial":describe(&a,r)}))
                .collect::<Vec<_>>()
        );
        report["scope"] = json!(
            "Internal algebra-only diagonal Q-automorphism; no CAD coordinate transformation or completion"
        );
        return write(&output, &report);
    }
    if args.factor_translation.is_some() || args.native_factor {
        let shifts = if let Some(k) = args.factor_translation {
            ensure!(SHIFT_MENU.contains(&k), "translation menu bound");
            (0..p.nvars())
                .filter(|&i| p.degree(i) > 0)
                .enumerate()
                .map(|(j, i)| (i, k * (j as i64 + 1)))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        let inverse = shifts.iter().map(|&(i, s)| (i, -s)).collect::<Vec<_>>();
        let start = Instant::now();
        let shifted = translate(&a, &p, &shifts)?;
        ensure!(
            translate(&a, &shifted, &inverse)? == p,
            "translation inverse failed"
        );
        report["translation"] = json!({"shifts":shifts,"seconds":start.elapsed().as_secs_f64(),
            "inverse_verified":true,"polynomial":describe(&a,&shifted)});
        report["phase"] = json!(if args.native_factor {
            "native_factorization"
        } else {
            "translated_factorization"
        });
        write(&output, &report)?;
        let start = Instant::now();
        let factors = shifted.factor();
        report["factor_seconds"] = json!(start.elapsed().as_secs_f64());
        let multiplicities = factors.iter().map(|(_, m)| *m).collect::<Vec<_>>();
        let restored = restore_factors(&a, &p, factors, &inverse)?;
        let exported = restored
            .iter()
            .zip(multiplicities)
            .map(|(r, m)| json!({"multiplicity":m,"polynomial":describe(&a,r)}))
            .collect::<Vec<_>>();
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
    let multiplicities = raw.iter().map(|(_, m)| *m).collect::<Vec<_>>();
    let reconstructed = restore_factors(&a, &p, raw, &[])?;
    let mut basis = Vec::new();
    let mut blocks = Vec::new();
    for (q, m) in reconstructed.into_iter().zip(multiplicities) {
        blocks.push(json!({"multiplicity":m,"polynomial":describe(&a,&q)}));
        if !q.is_constant() {
            basis.push(q);
        }
    }
    report["reconstruction_seconds"] = json!(start.elapsed().as_secs_f64());
    report["blocks"] = json!(blocks);
    report["signed_product_verified"] = json!(true);
    write(&output, &report)?;
    if args.square_free_only {
        report["phase"] = json!("square_free_complete");
        report["scope"] =
            json!("Exact signed square-free block inventory only; no projection or CAD completion");
        return write(&output, &report);
    }
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
fn validate_args(args: &Args) -> Result<()> {
    validate_source_limit(args.max_source_terms)?;
    let affine_trial = args.admission_only
        || args.native_factor
        || args.factor_translation.is_some()
        || !args.factor_scale.is_empty()
        || args.square_free_only;
    let deadline_ceiling = if affine_trial { 60.0 } else { 600.0 };
    ensure!(
        args.seconds.is_finite()
            && args.seconds > 0.0
            && args.seconds <= deadline_ceiling
            && (1..=3).contains(&args.levels)
            && (args.levels == 1 || args.factor_guards),
        "research limits"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    validate_args(&args)?;
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
        .arg("--max-source-terms")
        .arg(args.max_source_terms.to_string())
        .arg("--worker");
    if args.admission_only {
        command.arg("--admission-only");
    }
    if args.fiber_gcds {
        command.arg("--fiber-gcds");
    }
    if args.fiber_factors {
        command.arg("--fiber-factors");
    }
    if args.square_free_only {
        command.arg("--square-free-only");
    }
    if args.native_factor {
        command.arg("--native-factor");
    }
    if args.factor_guards {
        command.arg("--factor-guards");
    }
    if let Some(k) = args.factor_translation {
        command.arg("--factor-translation").arg(k.to_string());
    }
    if !args.factor_scale.is_empty() {
        command.arg("--factor-scale").arg(
            args.factor_scale
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
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
    fn historical_projection_deadlines_remain_separate_from_affine_trial_limits() {
        let mut args = Args::try_parse_from([
            "probe",
            "p.json",
            "--capture-sha256",
            "abcd",
            "--output",
            "out",
            "--seconds",
            "120",
        ])
        .unwrap();
        validate_args(&args).unwrap();
        args.seconds = 600.0;
        validate_args(&args).unwrap();
        args.seconds = 601.0;
        assert!(validate_args(&args).is_err());
        args.seconds = 120.0;
        args.admission_only = true;
        assert!(validate_args(&args).is_err());
        args.admission_only = false;
        args.native_factor = true;
        assert!(validate_args(&args).is_err());
        args.native_factor = false;
        args.square_free_only = true;
        assert!(validate_args(&args).is_err());
        args.square_free_only = false;
        args.factor_translation = Some(1);
        assert!(validate_args(&args).is_err());
        args.seconds = 60.0;
        validate_args(&args).unwrap();
        for invalid in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            args.seconds = invalid;
            assert!(validate_args(&args).is_err());
        }
    }

    #[test]
    fn full_fiber_gcds_distinguish_accidental_repetition_and_zero_fibers() {
        let a = Algebra::new(&["u".into(), "unused".into(), "x".into()]).unwrap();
        let p = a.parse("x^2-2*x+u").unwrap();
        let report = degree_drop_diagnostics(&a, &p, true).unwrap();
        assert_eq!(report["original_degree_drop_count"], 0);
        assert!(report["first_improving_shift"].is_null());
        assert_eq!(report["first_degree_preserving_gcd_improvement"], 1);
        let x = &report["original_leading_values"][1]["fiber"];
        assert_eq!(x["degree_preserved"], true);
        assert_eq!(x["gcd_derivative_degree"], 1);
        assert_eq!(x["coefficients_low_to_high"], json!(["1", "-2", "1"]));
        for candidate in report["candidates"].as_array().unwrap() {
            assert_eq!(candidate["degree_preserving_gcd_improved_axes"], json!([2]));
            assert_eq!(
                candidate["leading_values"][1]["fiber"]["gcd_derivative_degree"],
                0
            );
            let shifts = candidate["shifts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v[0].as_u64().unwrap() as usize, v[1].as_i64().unwrap()))
                .collect::<Vec<_>>();
            let shifted = translate(&a, &p, &shifts).unwrap();
            // The candidate fibers are in original coordinates: undo the shift
            // along the surviving axis before comparing exact coefficients.
            let q = translate(&a, &shifted, &[(2, -shifts[1].1)]).unwrap();
            assert_eq!(
                fiber_inventory(&a, &q, 2, &[(0, 1), (2, 1)]).unwrap()["coefficients_low_to_high"],
                candidate["leading_values"][1]["fiber"]["coefficients_low_to_high"]
            );
        }
        let structural = a.parse("(x-u)^2").unwrap();
        let report = degree_drop_diagnostics(&a, &structural, true).unwrap();
        assert!(report["first_degree_preserving_gcd_improvement"].is_null());
        assert_eq!(
            report["original_leading_values"][1]["fiber"]["gcd_derivative_degree"],
            1
        );
        let ordinary = a.parse("x^2+u+1").unwrap();
        let fiber = fiber_inventory(&a, &ordinary, 2, &[(0, 1), (2, 1)]).unwrap();
        assert_eq!(fiber["square_free"], true);
        let zero = a.parse("(u-1)*(x^2+1)").unwrap();
        let fiber = fiber_inventory(&a, &zero, 2, &[(0, 1), (2, 1)]).unwrap();
        assert_eq!(fiber["zero"], true);
        assert!(fiber["degree"].is_null() && fiber["gcd_derivative_degree"].is_null());
        assert_eq!(fiber["degree_preserved"], false);
        assert!(fiber_inventory(&a, &p, 2, &[(2, 1)]).is_err());
        assert!(fiber_inventory(&a, &p, 2, &[(0, 1), (0, 1)]).is_err());
        let wrong = Algebra::new(&["x".into(), "unused".into(), "u".into()]).unwrap();
        assert!(fiber_inventory(&wrong, &p, 2, &[(0, 1), (2, 1)]).is_err());
    }

    #[test]
    fn square_free_stop_replays_signed_units_and_keeps_inactive_map() {
        let a = Algebra::new(&["u".into(), "unused".into(), "x".into()]).unwrap();
        let p = a.parse("-3/7*(x-u)^2*(x^2+u+1)").unwrap();
        let blocks = p.square_free_factorization();
        assert!(blocks.iter().any(|(_, m)| *m == 2));
        let restored = restore_factors(&a, &p, blocks, &[]).unwrap();
        assert!(
            restored
                .iter()
                .all(|q| q.variables() == &a.variables && q.degree(1) == 0)
        );
        let base = [
            "probe",
            "p.json",
            "--capture-sha256",
            "abcd",
            "--output",
            "out",
        ];
        let mut flags = base.to_vec();
        flags.push("--square-free-only");
        let args = Args::try_parse_from(&flags).unwrap();
        assert!(args.square_free_only);
        validate_args(&args).unwrap();
        flags.push("--native-factor");
        assert!(Args::try_parse_from(&flags).is_err());
        let mut flags = base.to_vec();
        flags.push("--fiber-gcds");
        assert!(Args::try_parse_from(&flags).is_err());
        flags.push("--admission-only");
        assert!(Args::try_parse_from(&flags).unwrap().fiber_gcds);
    }

    #[test]
    fn irreducible_fiber_requires_constant_content_and_preserved_degree() {
        let a = Algebra::new(&["y".into(), "unused".into(), "x".into()]).unwrap();
        let content_example = a.parse("(y+1)*(x^2+y)").unwrap();
        let report = factor_fiber(&a, &content_example, 2, &[(0, 1), (2, 1)], |_| Ok(())).unwrap();
        assert_eq!(report["image_irreducible"], true);
        assert_eq!(report["signed_product_verified"], true);
        assert_eq!(report["source_content_checked"], true);
        assert_eq!(report["source_content"]["constant"], false);
        assert_eq!(report["source_irreducible_certified"], false);
        let primitive = a.parse("-3/7*(x^2+y)").unwrap();
        let report = factor_fiber(&a, &primitive, 2, &[(0, 1), (2, 1)], |_| Ok(())).unwrap();
        assert_eq!(report["image_irreducible"], true);
        assert_eq!(report["source_content"]["constant"], true);
        assert_eq!(report["source_irreducible_certified"], true);
        let dropped = a.parse("y*x^2+x+1").unwrap();
        let report = factor_fiber(&a, &dropped, 2, &[(0, 0), (2, 1)], |_| Ok(())).unwrap();
        assert_eq!(report["admitted"], false);
        assert_eq!(report["source_content_checked"], false);
        assert_eq!(report["source_irreducible_certified"], false);
        let repeated = a.parse("(x-y)^2").unwrap();
        let report = factor_fiber(&a, &repeated, 2, &[(0, 1), (2, 1)], |_| Ok(())).unwrap();
        assert_eq!(report["admitted"], false);
        assert_eq!(report["source_content_checked"], false);
        let split = a.parse("(x+y)*(x-y)").unwrap();
        let report = factor_fiber(&a, &split, 2, &[(0, 1), (2, 1)], |_| Ok(())).unwrap();
        assert_eq!(report["image_irreducible"], false);
        assert_eq!(
            report["factor_degree_partition"],
            json!([{"degree":1,"multiplicity":1},{"degree":1,"multiplicity":1}])
        );
        assert_eq!(report["source_content_checked"], false);
        assert_eq!(report["source_irreducible_certified"], false);
    }

    #[test]
    fn fiber_factor_schedule_is_bounded_and_starts_with_lowest_degree() {
        let a = Algebra::new(&["u".into(), "unused".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("u^4+v^3+x^2").unwrap();
        let schedule = factor_fiber_schedule(&p).unwrap();
        assert_eq!(schedule.len(), 12);
        assert_eq!(schedule[0], (3, None, vec![(0, 1), (2, 1), (3, 1)]));
        assert_eq!(schedule[1], (3, Some(1), vec![(0, 2), (2, 3), (3, 4)]));
        assert_eq!(schedule[4].0, 2);
        assert_eq!(schedule[8].0, 0);
        assert!(factor_fiber_schedule(&a.parse("x^33").unwrap()).is_err());
        let mut flags = vec![
            "probe",
            "p.json",
            "--capture-sha256",
            "abcd",
            "--output",
            "out",
            "--fiber-factors",
        ];
        assert!(Args::try_parse_from(&flags).is_err());
        flags.push("--admission-only");
        assert!(Args::try_parse_from(&flags).is_err());
        flags.push("--fiber-gcds");
        assert!(Args::try_parse_from(&flags).unwrap().fiber_factors);
    }
    #[test]
    fn configurable_source_admission_is_bounded_and_keeps_the_full_map() {
        let a = Algebra::new(&["u".into(), "unused".into(), "v".into(), "x".into()]).unwrap();
        let mut support = a.parse("0").unwrap();
        for i in 0..=32 {
            for j in 0..=32 {
                support.append_monomial(Rational::one(), &[i, 0, j, 0]);
            }
        }
        let p = &support * &a.parse("x^2-x+1").unwrap() + a.parse("u").unwrap();
        assert_eq!(p.nterms(), 3267);
        assert!(source_admission(&a, &p, 2048).is_err());
        source_admission(&a, &p, 4096).unwrap();
        let diagnostics = degree_drop_diagnostics(&a, &p, false).unwrap();
        assert_eq!(diagnostics["original_degree_drop_count"], 0);
        assert!(diagnostics["first_improving_shift"].is_null());
        for bad in [0, 8193, usize::MAX] {
            assert!(source_admission(&a, &p, bad).is_err());
        }
        let wrong = Algebra::new(&["v".into(), "unused".into(), "u".into(), "x".into()]).unwrap();
        assert!(source_admission(&wrong, &p, 4096).is_err());
        assert!(translate(&wrong, &p, &[(0, 1)]).is_err());
        let args = Args::try_parse_from([
            "probe",
            "p.json",
            "--capture-sha256",
            "abcd",
            "--output",
            "out",
            "--admission-only",
        ])
        .unwrap();
        assert_eq!(args.max_source_terms, 2048);
        assert_eq!(args.seconds, 60.0);
        let args = Args::try_parse_from([
            "probe",
            "p.json",
            "--capture-sha256",
            "abcd",
            "--output",
            "out",
            "--max-source-terms",
            "4096",
            "--factor-translation",
            "-1",
            "--worker",
        ])
        .unwrap();
        assert_eq!(args.factor_translation, Some(-1));
        assert_eq!(args.max_source_terms, 4096);
    }

    #[test]
    fn fixed_menu_detects_exact_degree_losses_including_zero_fibers() {
        let a = Algebra::new(&["u".into(), "unused".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("-5/7*(u-1)*(x^2+v+1)^2*(x+u)").unwrap();
        let report = degree_drop_diagnostics(&a, &p, false).unwrap();
        assert!(report["original_degree_drop_count"].as_u64().unwrap() > 0);
        assert_eq!(report["first_improving_shift"], 1);
        for candidate in report["candidates"].as_array().unwrap() {
            let k = candidate["k"].as_i64().unwrap();
            assert!(SHIFT_MENU.contains(&k));
            let shifts = vec![(0, k), (2, 2 * k), (3, 3 * k)];
            let q = translate(&a, &p, &shifts).unwrap();
            for value in candidate["leading_values"].as_array().unwrap() {
                let axis = value["axis"].as_u64().unwrap() as usize;
                let mut lc = q.univariate_lcoeff(axis);
                for other in [0, 2, 3] {
                    if other != axis {
                        lc = lc.replace(other, &Rational::one());
                    }
                }
                assert_eq!(a.format(&lc), value["leading_value"]);
                assert_eq!(lc.is_zero(), value["degree_drop"]);
            }
            assert_eq!(q.variables(), &a.variables);
            assert_eq!(q.degree(1), 0);
        }
    }

    #[test]
    fn inverse_signed_factor_replay_rejects_corrupt_units_maps_and_multiplicities() {
        let a = Algebra::new(&["u".into(), "unused".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("-5/7*(u-1)*(x^2+v+1)^2*(x+u)").unwrap();
        for k in SHIFT_MENU {
            let shifts = vec![(0, k), (2, 2 * k), (3, 3 * k)];
            let inverse = shifts.iter().map(|&(i, s)| (i, -s)).collect::<Vec<_>>();
            let q = translate(&a, &p, &shifts).unwrap();
            assert_eq!(translate(&a, &q, &inverse).unwrap(), p);
            let factors = q.factor();
            let restored = restore_factors(&a, &p, factors.clone(), &inverse).unwrap();
            assert!(restored.iter().all(|f| f.variables() == &a.variables));
            let mut bad = factors.clone();
            bad[0].1 = 0;
            assert!(restore_factors(&a, &p, bad, &inverse).is_err());
            let mut bad = factors;
            bad[0].0 = bad[0].0.clone().mul_coeff(Rational::from(2));
            assert!(restore_factors(&a, &p, bad, &inverse).is_err());
        }
        let wrong = Algebra::new(&["v".into(), "unused".into(), "u".into(), "x".into()]).unwrap();
        assert!(restore_factors(&a, &p, vec![(wrong.parse("u+x").unwrap(), 1)], &[]).is_err());
        assert!(translate(&a, &p, &[(0, i64::MIN)]).is_err());
    }
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

    #[test]
    fn diagonal_scaling_preserves_support_full_map_and_regular_fibers() {
        let a = Algebra::new(&["u".into(), "unused".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("-5/7*(u-1)*(x^2+v+1)^2*(x+u)").unwrap();
        let scales = [(0, 2), (2, 3), (3, 4)];
        let q = scale(&a, &p, &scales, false).unwrap();
        assert_eq!(q.variables(), &a.variables);
        assert_eq!(q.degree(1), 0);
        assert!(q.exponents_iter().eq(p.exponents_iter()));
        assert_eq!(scale(&a, &q, &scales, true).unwrap(), p);
        let diagnostics = degree_drop_diagnostics(&a, &p, false).unwrap();
        assert!(diagnostics["original_degree_drop_count"].as_u64().unwrap() > 0);
        assert_eq!(
            degree_drop_diagnostics(&a, &q, false).unwrap()["original_degree_drop_count"],
            0
        );
    }

    #[test]
    fn scaled_signed_replay_checks_units_multiplicities_and_shared_budget() {
        let a = Algebra::new(&["u".into(), "unused".into(), "v".into(), "x".into()]).unwrap();
        let p = a.parse("-5/7*(u-1)*(x^2+v+1)^2*(x+u)").unwrap();
        let scales = [(0, -2), (2, 3), (3, 4)];
        let q = scale(&a, &p, &scales, false).unwrap();
        let factors = q.factor();
        let mut work = 0;
        restore_factors_by(&a, &q, factors.clone(), &mut work, |r| {
            scale(&a, r, &[], false)
        })
        .unwrap();
        let first_work = work;
        let restored = restore_factors_by(&a, &p, factors.clone(), &mut work, |r| {
            scale(&a, r, &scales, true)
        })
        .unwrap();
        assert!(work > first_work && restored.iter().all(|r| r.variables() == &a.variables));
        let mut bad = factors.clone();
        bad[0].0 = bad[0].0.clone().mul_coeff(Rational::from(2));
        assert!(restore_factors_by(&a, &p, bad, &mut 0, |r| scale(&a, r, &scales, true)).is_err());
        let mut bad = factors.clone();
        bad[0].1 = 0;
        assert!(restore_factors_by(&a, &p, bad, &mut 0, |r| scale(&a, r, &scales, true)).is_err());
        assert!(
            restore_factors_by(&a, &p, factors, &mut MAX_PRODUCT_WORK.clone(), |r| scale(
                &a, r, &scales, true
            ))
            .is_err()
        );
    }

    #[test]
    fn scaling_rejects_zero_duplicate_axes_bad_maps_and_coefficient_growth() {
        let a = Algebra::new(&["u".into(), "v".into()]).unwrap();
        let p = a.parse("u^3+v").unwrap();
        for scales in [
            vec![(0, 0)],
            vec![(0, 65)],
            vec![(0, i64::MIN)],
            vec![(0, 2), (0, 3)],
            vec![(2, 2)],
        ] {
            assert!(scale(&a, &p, &scales, false).is_err());
        }
        let wrong = Algebra::new(&["v".into(), "u".into()]).unwrap();
        assert!(scale(&wrong, &p, &[(0, 2)], false).is_err());
        let tall = p.mul_coeff(Rational::from(Integer::from(2).pow(8190)));
        assert!(scale(&a, &tall, &[(0, 64)], false).is_err());
    }

    #[test]
    fn scaling_cli_is_separate_and_keeps_the_short_algebra_deadline() {
        let base = [
            "probe",
            "p.json",
            "--capture-sha256",
            "abcd",
            "--output",
            "out",
            "--factor-scale",
            "2,3,4",
        ];
        let mut args = Args::try_parse_from(base).unwrap();
        assert_eq!(args.factor_scale, vec![2, 3, 4]);
        validate_args(&args).unwrap();
        args.seconds = 61.0;
        assert!(validate_args(&args).is_err());
        for option in [
            "--native-factor",
            "--admission-only",
            "--factor-guards",
            "--square-free-only",
        ] {
            let mut flags = base.to_vec();
            flags.push(option);
            assert!(Args::try_parse_from(flags).is_err());
        }
    }
}
