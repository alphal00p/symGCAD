//! Bounded research generator of strict polynomial-combination hints.
//! The generated TOML preserves the original problem; every proposed identity
//! must still be independently checked by the production solver and verifier.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits, Problem,
    algebra::{Algebra, Poly, canonical, factors_checked},
    problem::normalize,
};

const MAX_HINTS: usize = 128;
const MAX_TERMS: usize = 4096;
const MAX_PRODUCT: usize = 65_536;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    guide: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Hint {
    schema_version: u32,
    polynomials: [String; 2],
    multipliers: [String; 2],
    square: String,
    multiplier: String,
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65_536];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        hash.update(&buffer[..length]);
    }
}

fn bounded(p: &Poly) -> bool {
    p.nterms() <= MAX_TERMS
        && p.nvars() <= 8
        && p.exponents_iter()
            .all(|e| e.iter().copied().map(u64::from).sum::<u64>() <= 32)
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= 4096
                && c.denominator_ref().significant_bits() <= 4096
        })
}

fn coefficient_sign(p: &Poly) -> i8 {
    if !p.is_zero() && p.coefficients.iter().all(|c| c > &Rational::zero()) {
        1
    } else if !p.is_zero() && p.coefficients.iter().all(|c| c < &Rational::zero()) {
        -1
    } else {
        0
    }
}

fn coordinate(p: &Poly) -> Option<usize> {
    if p.nterms() != 1 || p.coefficients[0] <= Rational::zero() {
        return None;
    }
    let e = p.exponents_iter().next()?;
    let mut support = e.iter().enumerate().filter(|(_, d)| **d != 0);
    let (axis, &degree) = support.next()?;
    (degree == 1 && support.next().is_none()).then_some(axis)
}

fn strict_square(residual: &Poly) -> Result<Option<(Rational, Poly)>> {
    if residual.is_zero() || !bounded(residual) {
        return Ok(None);
    }
    let factors = residual.factor();
    let mut rebuilt = residual.one();
    let mut square = residual.one();
    let mut multiplier = Rational::one();
    for (factor, count) in factors {
        rebuilt = &rebuilt * &factor.pow(count);
        if factor.is_constant() {
            multiplier *= factor.pow(count).get_constant();
        } else if count % 2 == 0 {
            square = &square * &factor.pow(count / 2);
        } else {
            return Ok(None);
        }
    }
    ensure!(
        rebuilt == *residual,
        "square factorization reconstruction failed"
    );
    if multiplier <= Rational::zero()
        || coefficient_sign(&square) == 0
        || !bounded(&square)
        || square.nterms().saturating_mul(square.nterms()) > MAX_PRODUCT
    {
        return Ok(None);
    }
    ensure!(
        &square.pow(2) * &residual.constant(multiplier.clone()) == *residual,
        "strict square identity failed"
    );
    Ok(Some((multiplier, square)))
}

fn canonical_pair_hint(
    algebra: &Algebra,
    a: &Poly,
    b: &Poly,
    c: &Poly,
    d: &Poly,
    p: &Poly,
    q: &Poly,
    multiplier: &Rational,
    square: &Poly,
) -> Result<Option<Hint>> {
    ensure!(*p == canonical(p) && *q == canonical(q));
    let (a_over_p, rem_a) = a.quot_rem(p, false);
    let (b_over_q, rem_b) = b.quot_rem(q, false);
    ensure!(
        rem_a.is_zero() && rem_b.is_zero(),
        "factor divisibility failed"
    );
    ensure!(&a_over_p * p == *a && &b_over_q * q == *b);
    if d.nterms().saturating_mul(a_over_p.nterms()) > MAX_PRODUCT
        || c.nterms().saturating_mul(b_over_q.nterms()) > MAX_PRODUCT
    {
        return Ok(None);
    }
    let l = -(&a_over_p * d);
    let m = &b_over_q * c;
    let pair_level = (0..p.nvars())
        .filter(|&i| p.degree(i) != 0)
        .max()
        .context("constant pair has no projection level")?;
    if (0..q.nvars()).filter(|&i| q.degree(i) != 0).max() != Some(pair_level) {
        return Ok(None);
    }
    if !bounded(&l)
        || !bounded(&m)
        || !bounded(p)
        || !bounded(q)
        || (pair_level + 1..p.nvars())
            .any(|i| l.degree(i) != 0 || m.degree(i) != 0 || square.degree(i) != 0)
        || p.nterms().saturating_mul(l.nterms()) > MAX_PRODUCT
        || q.nterms().saturating_mul(m.nterms()) > MAX_PRODUCT
    {
        return Ok(None);
    }
    ensure!(&(&l * p) + &(&m * q) == &square.pow(2) * &a.constant(multiplier.clone()));
    let mut polynomials = [algebra.format(p), algebra.format(q)];
    let mut multipliers = [algebra.format(&l), algebra.format(&m)];
    if polynomials[0] > polynomials[1] {
        polynomials.swap(0, 1);
        multipliers.swap(0, 1);
    }
    let hint = Hint {
        schema_version: 1,
        polynomials,
        multipliers,
        square: algebra.format(square),
        multiplier: multiplier.to_string(),
    };
    if hint
        .polynomials
        .iter()
        .chain(&hint.multipliers)
        .chain([&hint.square, &hint.multiplier])
        .any(|s| s.len() > 65_536)
    {
        return Ok(None);
    }
    Ok(Some(hint))
}

fn generate(args: &Args) -> Result<()> {
    let started = Instant::now();
    ensure!(!args.output.exists(), "output directory must be fresh");
    fs::create_dir_all(&args.output)?;
    let original = Problem::from_toml(&fs::read_to_string(&args.input)?)?;
    ensure!(original.variables.len() <= 8 && original.split.is_empty());
    let source_guide: Value = serde_json::from_slice(&fs::read(&args.guide)?)?;
    let order: Vec<String> = serde_json::from_value(source_guide["lifting_order"].clone())?;
    ensure!(
        original.solver.order.is_empty() || original.solver.order == order,
        "guide order conflicts with the original fixed order"
    );
    let peel: Vec<String> = serde_json::from_value(source_guide["peel_order"].clone())?;
    ensure!(
        !peel.is_empty() && peel.len() <= 4,
        "at most four lattice axes"
    );
    let mut prepared = original.clone();
    prepared.solver.engine = "local".into();
    prepared.solver.ordering = "given".into();
    prepared.solver.order = order.clone();
    prepared.solver.positivity = true;
    prepared.solver.local_adjacent_pairs = true;
    prepared.solver.homogeneous_anchor = None;
    prepared.solver.homogeneous_block.clear();
    prepared.solver.negative_linear_guide = None;
    prepared.validate()?;
    let normalized = normalize(&prepared, &order)?;
    let algebra = &normalized.algebra;
    let targets = normalized
        .positives
        .iter()
        .filter(|p| coordinate(p).is_none())
        .collect::<Vec<_>>();
    ensure!(
        targets.len() == 1,
        "one strict non-coordinate target required"
    );
    for axis in 0..order.len() {
        ensure!(
            normalized
                .positives
                .iter()
                .any(|p| coordinate(p) == Some(axis)),
            "literal positive coordinate missing"
        );
    }
    let target = targets[0];
    let guide = algebra.parse(source_guide["guide"].as_str().context("guide missing")?)?;
    ensure!(bounded(target) && bounded(&guide));
    let axes = peel
        .iter()
        .map(|name| {
            order
                .iter()
                .position(|v| v == name)
                .context("unknown peel axis")
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        axes.iter().copied().collect::<BTreeSet<_>>().len() == axes.len(),
        "duplicate lattice axis"
    );
    let state_count = 3_usize.pow(axes.len() as u32);
    ensure!(state_count <= 128);
    let mut hints = Vec::new();
    let mut provenance = Vec::new();
    let mut seen = BTreeSet::new();
    let mut attempts = 0;
    let mut strict_families = 0;
    let mut rootless_factor_pairs_skipped = 0;
    let mut capped_hints_skipped = 0;
    for code in 0..state_count {
        ensure!(
            started.elapsed().as_secs_f64() < 55.,
            "hint-generation soft deadline"
        );
        let mut digits = code;
        let mut state = Vec::new();
        let mut p = target.clone();
        let mut u = guide.clone();
        for &axis in &axes {
            let operation = digits % 3;
            digits /= 3;
            match operation {
                1 => {
                    p = p.derivative(axis);
                    u = u.derivative(axis);
                }
                2 => {
                    p = p.replace(axis, &Rational::zero());
                    u = u.replace(axis, &Rational::zero());
                }
                _ => {}
            }
            state.push(operation);
        }
        if p.is_zero() || u.is_zero() {
            continue;
        }
        for (slot, &axis) in axes.iter().enumerate() {
            if state[slot] != 0 || p.degree(axis) != 1 || u.degree(axis) != 1 {
                continue;
            }
            attempts += 1;
            ensure!(attempts <= 256);
            let a = p.derivative(axis);
            let b = p.replace(axis, &Rational::zero());
            let c = u.derivative(axis);
            let d = u.replace(axis, &Rational::zero());
            if coefficient_sign(&c) != 1 || coefficient_sign(&d) != 1 || a.is_zero() || b.is_zero()
            {
                continue;
            }
            if b.nterms().saturating_mul(c.nterms()) > MAX_PRODUCT
                || a.nterms().saturating_mul(d.nterms()) > MAX_PRODUCT
            {
                continue;
            }
            let residual = &b * &c - &a * &d;
            let Some((multiplier, square)) = strict_square(&residual)? else {
                continue;
            };
            strict_families += 1;
            let af = factors_checked(&[a.clone()])?;
            let bf = factors_checked(&[b.clone()])?;
            for first in &af {
                for second in &bf {
                    // A shared factor may be globally nonzero on the positive
                    // domain. There is no self-pair resultant to replace.
                    if first == second {
                        continue;
                    }
                    // Uniform coefficients already establish no positive roots;
                    // such a cut never reaches the positive-bearing pair loop.
                    if coefficient_sign(first) != 0 || coefficient_sign(second) != 0 {
                        rootless_factor_pairs_skipped += 1;
                        continue;
                    }
                    let Some(hint) = canonical_pair_hint(
                        algebra,
                        &a,
                        &b,
                        &c,
                        &d,
                        first,
                        second,
                        &multiplier,
                        &square,
                    )?
                    else {
                        continue;
                    };
                    if !seen.insert(hint.polynomials.clone()) {
                        continue;
                    }
                    if hints.len() == MAX_HINTS {
                        capped_hints_skipped += 1;
                        continue;
                    }
                    provenance.push(json!({"hint_index":hints.len(),"state":peel.iter().zip(&state).map(|(v,o)|json!({"axis":v,"operation":match o{1=>"derivative",2=>"zero_endpoint",_=>"unchanged"}})).collect::<Vec<_>>(),"family_axis":order[axis],"a":algebra.format(&a),"b":algebra.format(&b),"c":algebra.format(&c),"d":algebra.format(&d),"division_identity_verified":true}));
                    hints.push(hint);
                }
            }
        }
    }
    let hint_bytes = serde_json::to_vec(&hints)?;
    ensure!(
        hint_bytes.len() <= 4 * 1024 * 1024,
        "hint byte cap exceeded"
    );
    let mut value = serde_json::to_value(&prepared)?;
    fs::write(
        args.output.join("problem-no-hints.toml"),
        toml::to_string_pretty(&prepared)?,
    )?;
    value["solver"]["local_strict_bezout_hints"] = serde_json::to_value(&hints)?;
    fs::write(
        args.output.join("problem.json"),
        serde_json::to_vec_pretty(&value)?,
    )?;
    let mut toml_value = toml::Value::try_from(&prepared)?;
    toml_value
        .get_mut("solver")
        .and_then(toml::Value::as_table_mut)
        .context("solver table missing")?
        .insert(
            "local_strict_bezout_hints".into(),
            toml::Value::try_from(&hints)?,
        );
    fs::write(
        args.output.join("problem.toml"),
        toml::to_string_pretty(&toml_value)?,
    )?;
    fs::write(
        args.output.join("hints.json"),
        serde_json::to_vec_pretty(&hints)?,
    )?;
    let report = json!({"source":args.input,"source_sha256":hash_file(&args.input)?,"guide_source":args.guide,"guide_sha256":hash_file(&args.guide)?,"helper_sha256":hash_file(&std::env::current_exe()?)?,"order":order,"lattice_axes":peel,"states":state_count,"family_attempts":attempts,"strict_families":strict_families,"hints":hints.len(),"hint_bytes":hint_bytes.len(),"rootless_factor_pairs_skipped":rootless_factor_pairs_skipped,"capped_hints_skipped":capped_hints_skipped,"provenance":provenance,"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false,"scope":"Exact identities only. Production must independently verify every identity and positive-domain premise; no saved algebra result is trusted."});
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        return generate(&args);
    }
    ensure!(!args.output.exists(), "output directory must be fresh");
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(&args.input)
        .arg("--guide")
        .arg(&args.guide)
        .arg("--output")
        .arg(&args.output)
        .arg("--worker");
    let process = symgcad::benchmark::run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 60.,
            memory_mib: 4096,
            workers: 1,
            max_cells: None,
        },
    )?;
    fs::create_dir_all(&args.output)?;
    fs::write(
        args.output.join("process.json"),
        serde_json::to_vec_pretty(&process)?,
    )?;
    ensure!(
        process.status == "completed" && process.exit_code == Some(0),
        "hint generator stopped: {}",
        process.status
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_proposal_rejects_interior_zeros_and_negative_unit() -> Result<()> {
        let algebra = Algebra::new(&["x".into(), "y".into()])?;
        assert!(strict_square(&algebra.parse("7*(x+y)^2/3")?)?.is_some());
        assert!(strict_square(&algebra.parse("(x-y)^2")?)?.is_none());
        assert!(strict_square(&algebra.parse("-(x+y)^2")?)?.is_none());
        assert!(strict_square(&algebra.parse("0")?)?.is_none());
        Ok(())
    }

    #[test]
    fn canonical_factor_cofactors_preserve_negative_scalars_and_multiplicities() -> Result<()> {
        let algebra = Algebra::new(&["x".into(), "y".into()])?;
        let a = algebra.parse("-2*(x-y)^2")?;
        let b = algebra.parse("3*(x+y)^2")?;
        let c = algebra.parse("2*(x-y)^2+1")?;
        let d = b.clone();
        let residual = &b * &c - &a * &d;
        // The direct cofactor test need not require positive guide coefficients:
        // it checks the exact identity supplied to the production verifier.
        let p = canonical(&algebra.parse("x-y")?);
        let q = canonical(&algebra.parse("x+y")?);
        let square = algebra.parse("x+y")?;
        // Choose C so the residual is exactly3*(x+y)^2, with D=3*(x+y)^2.
        let c = &a + &a.one();
        let residual2 = &b * &c - &a * &d;
        assert_ne!(residual, residual2);
        let hint = canonical_pair_hint(
            &algebra,
            &a,
            &b,
            &c,
            &d,
            &p,
            &q,
            &Rational::from(3),
            &square,
        )?
        .unwrap();
        let hp = algebra.parse(&hint.polynomials[0])?;
        let hq = algebra.parse(&hint.polynomials[1])?;
        let l = algebra.parse(&hint.multipliers[0])?;
        let m = algebra.parse(&hint.multipliers[1])?;
        assert_eq!(&hp * &l + &hq * &m, residual2);
        Ok(())
    }
}
