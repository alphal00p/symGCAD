//! Bounded exact stationary-face inventory for a quadratic block on a unit cube.
//! Research only: face guards are not a cylindrical projection or CAD proof.
use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, canonical, factors_checked},
    projection::projection_polynomials,
};

const INPUT_BYTES: usize = 256 * 1024;
const OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_VARIABLES: usize = 12;
const INPUT_TERMS: usize = 1024;
const MAX_TERMS: usize = 8192;
// Ordinary projection generates a different, larger obligation set. Its first
// authentic three-axis run exceeded the face-evidence cap before factorization.
const MAX_PROJECTION_TERMS: usize = 65536;
const INPUT_DEGREE: u32 = 12;
const MAX_DEGREE: u32 = 48;
const MAX_WORK: usize = 50_000_000;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Mode {
    Faces,
    Projection,
}

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_enum, default_value = "faces")]
    mode: Mode,
    #[arg(long, default_value_t = 60.)]
    seconds: f64,
    #[arg(long, default_value_t = 3)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    variables: Vec<String>,
    /// Last 1..3 axes in the same order; the conditional block is (0,1)^n.
    block: Vec<String>,
    polynomial: String,
    /// Strict positive polynomials in the prefix only. No conclusion uses samples.
    #[serde(default)]
    prefix_constraints: Vec<String>,
    /// Provenance only; this helper does not authenticate a coordinate map.
    #[serde(default)]
    provenance: BTreeMap<String, String>,
}

#[derive(Default)]
struct Budget {
    product_work: usize,
}
impl Budget {
    fn mul(&mut self, a: &Poly, b: &Poly) -> Result<Poly> {
        self.product_work = self
            .product_work
            .saturating_add(a.nterms().saturating_mul(b.nterms()));
        ensure!(self.product_work <= MAX_WORK, "polynomial product-work cap");
        let p = a * b;
        bounded(&p, MAX_TERMS, MAX_DEGREE)?;
        Ok(p)
    }
}

fn bounded(p: &Poly, terms: usize, degree: u32) -> Result<()> {
    ensure!(
        p.nvars() <= MAX_VARIABLES && p.nterms() <= terms,
        "polynomial support cap"
    );
    ensure!(
        p.exponents_iter()
            .all(|e| e.iter().map(|&v| u64::from(v)).sum::<u64>() <= u64::from(degree)),
        "polynomial degree cap"
    );
    ensure!(
        p.coefficients
            .iter()
            .all(|c| c.numerator_ref().significant_bits() as u64
                + c.denominator_ref().significant_bits() as u64
                <= 2048),
        "coefficient-height cap"
    );
    Ok(())
}

/// Bound unnormalized Symbolica tokens before Atom can expand powers.
fn expression_budget(text: &str, names: &[String]) -> Result<()> {
    use symbolica::parser::{Operator, ParseSettings, Token};
    ensure!(text.len() <= INPUT_BYTES, "polynomial text cap");
    let (mut depth, mut digits, mut operators) = (0usize, 0usize, 0usize);
    for b in text.bytes() {
        digits = if b.is_ascii_digit() { digits + 1 } else { 0 };
        ensure!(digits <= 128, "numeric literal cap");
        if matches!(b, b'+' | b'-' | b'*' | b'/' | b'^') {
            operators += 1;
            ensure!(operators <= 8192, "operator cap");
        }
        if b == b'(' {
            depth += 1;
            ensure!(depth <= 32, "nesting cap");
        }
        if b == b')' {
            depth = depth.checked_sub(1).context("unbalanced expression")?;
        }
    }
    ensure!(depth == 0, "unbalanced expression");
    let token = Token::parse(
        &text.replace("**", "^"),
        ParseSettings::symbolica().convert_mul_to_atom(false),
    )
    .map_err(|e| anyhow::anyhow!("invalid polynomial tokens: {e}"))?;
    fn estimate(
        t: &Token,
        names: &[String],
        visits: &mut usize,
        depth: usize,
    ) -> Result<(usize, u32, usize)> {
        *visits += 1;
        ensure!(*visits <= 16384 && depth <= 64, "token traversal cap");
        let r = match t {
            Token::Number(n, false) => {
                let digits = n
                    .strip_prefix('-')
                    .or_else(|| n.strip_prefix('+'))
                    .unwrap_or(n);
                ensure!(
                    !digits.is_empty()
                        && digits.bytes().all(|b| b.is_ascii_digit())
                        && digits.len() <= 128,
                    "integer literal required"
                );
                (1, 0, n.len().saturating_mul(4))
            }
            Token::ID(name) => {
                ensure!(
                    names.iter().any(|n| n == name.as_str()),
                    "undeclared symbol"
                );
                (1, 1, 1)
            }
            Token::Op(_, _, Operator::Neg, args) if args.len() == 1 => {
                estimate(&args[0], names, visits, depth + 1)?
            }
            Token::Op(_, _, Operator::Add, args) | Token::Op(_, _, Operator::Mul, args) => {
                let mul = matches!(t, Token::Op(_, _, Operator::Mul, _));
                let mut out = (usize::from(mul), 0u32, 0usize);
                for a in args {
                    let (terms, degree, height) = estimate(a, names, visits, depth + 1)?;
                    out.0 = if mul {
                        out.0.saturating_mul(terms)
                    } else {
                        out.0.saturating_add(terms)
                    };
                    out.1 = if mul {
                        out.1.saturating_add(degree)
                    } else {
                        out.1.max(degree)
                    };
                    out.2 = out.2.saturating_add(height + 1);
                }
                out
            }
            Token::Op(_, _, Operator::Inv, args) if args.len() == 1 => {
                let (_, d, h) = estimate(&args[0], names, visits, depth + 1)?;
                ensure!(d == 0, "denominator must be constant");
                (1, 0, h)
            }
            Token::Op(_, _, Operator::Pow, args) if args.len() == 2 => {
                let Token::Number(e, false) = &args[1] else {
                    bail!("literal exponent required")
                };
                let e: u32 = e.parse().context("exponent cap")?;
                ensure!(e <= INPUT_DEGREE, "exponent cap");
                let (t, d, h) = estimate(&args[0], names, visits, depth + 1)?;
                (
                    t.saturating_pow(e),
                    d.saturating_mul(e),
                    h.saturating_mul(e as usize),
                )
            }
            _ => bail!("unsupported polynomial token"),
        };
        ensure!(
            r.0 <= INPUT_TERMS && r.1 <= INPUT_DEGREE && r.2 <= 32768,
            "raw expansion cap"
        );
        Ok(r)
    }
    estimate(&token, names, &mut 0, 0)?;
    Ok(())
}

struct Admitted {
    algebra: Algebra,
    polynomial: Poly,
    prefix: Vec<Poly>,
    axes: Vec<usize>,
}
fn admit(input: &Input) -> Result<Admitted> {
    ensure!(
        serde_json::to_vec(input)?.len() <= INPUT_BYTES,
        "input byte cap"
    );
    ensure!(
        (1..=MAX_VARIABLES).contains(&input.variables.len())
            && input.variables.iter().all(|v| v.len() <= 64),
        "variable cap"
    );
    ensure!(
        (1..=3).contains(&input.block.len()) && input.block.len() <= input.variables.len(),
        "block dimension cap"
    );
    let begin = input.variables.len() - input.block.len();
    ensure!(
        input.variables[begin..] == input.block,
        "block must be final axes in exact order"
    );
    ensure!(input.prefix_constraints.len() <= 32, "constraint cap");
    let algebra = Algebra::new(&input.variables)?;
    let parse = |text: &str| -> Result<Poly> {
        expression_budget(text, &input.variables)?;
        let p = algebra.parse(text)?;
        bounded(&p, INPUT_TERMS, INPUT_DEGREE)?;
        Ok(p)
    };
    let polynomial = parse(&input.polynomial)?;
    let axes: Vec<_> = (begin..input.variables.len()).collect();
    ensure!(
        polynomial
            .exponents_iter()
            .all(|e| axes.iter().map(|&i| e[i]).sum::<u32>() <= 2),
        "not jointly quadratic in the block"
    );
    let prefix = input
        .prefix_constraints
        .iter()
        .map(|s| parse(s))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        prefix
            .iter()
            .all(|p| axes.iter().all(|&i| p.degree(i) == 0)),
        "prefix constraint depends on block"
    );
    Ok(Admitted {
        algebra,
        polynomial,
        prefix,
        axes,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Free,
    Zero,
    One,
}
fn states(mut ordinal: usize, dimension: usize) -> Vec<State> {
    (0..dimension)
        .map(|_| {
            let s = [State::Free, State::Zero, State::One][ordinal % 3];
            ordinal /= 3;
            s
        })
        .collect()
}
fn restrict(p: &Poly, axes: &[usize], states: &[State]) -> Poly {
    axes.iter()
        .zip(states)
        .fold(p.clone(), |q, (&axis, state)| match state {
            State::Free => q,
            State::Zero => q.replace(axis, &Rational::zero()),
            State::One => q.replace(axis, &Rational::one()),
        })
}
fn at_origin(p: &Poly, axes: &[usize]) -> Poly {
    axes.iter()
        .fold(p.clone(), |q, &i| q.replace(i, &Rational::zero()))
}

struct Quadratic {
    h: Vec<Vec<Poly>>,
    l: Vec<Poly>,
    c: Poly,
}
fn derivatives(p: &Poly, free: &[usize]) -> Quadratic {
    let half = Rational::one() / Rational::from(2);
    Quadratic {
        h: free
            .iter()
            .map(|&i| {
                free.iter()
                    .map(|&j| p.derivative(i).derivative(j).mul_coeff(half.clone()))
                    .collect()
            })
            .collect(),
        l: free
            .iter()
            .map(|&i| at_origin(&p.derivative(i), free))
            .collect(),
        c: at_origin(p, free),
    }
}
/// Independent coefficient extraction for serialization replay.
fn coefficients(p: &Poly, free: &[usize]) -> Result<Quadratic> {
    let n = free.len();
    let mut q = Quadratic {
        h: vec![vec![p.zero(); n]; n],
        l: vec![p.zero(); n],
        c: p.zero(),
    };
    for (coefficient, exponent) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut powers = exponent.to_vec();
        let mut support = Vec::new();
        for (i, &axis) in free.iter().enumerate() {
            for _ in 0..powers[axis] {
                support.push(i);
            }
            powers[axis] = 0;
        }
        match support.as_slice() {
            [] => q.c.append_monomial(coefficient.clone(), &powers),
            [i] => q.l[*i].append_monomial(coefficient.clone(), &powers),
            [i, j] if i == j => q.h[*i][*j].append_monomial(coefficient.clone(), &powers),
            [i, j] => {
                let c = coefficient / &Rational::from(2);
                q.h[*i][*j].append_monomial(c.clone(), &powers);
                q.h[*j][*i].append_monomial(c, &powers);
            }
            _ => bail!("joint degree exceeds two"),
        }
    }
    Ok(q)
}
fn submatrix(m: &[Vec<Poly>], rows: &[usize], columns: &[usize]) -> Vec<Vec<Poly>> {
    rows.iter()
        .map(|&i| columns.iter().map(|&j| m[i][j].clone()).collect())
        .collect()
}
fn determinant(m: &[Vec<Poly>], one: &Poly, b: &mut Budget) -> Result<Poly> {
    if m.is_empty() {
        return Ok(one.clone());
    }
    let n = m.len();
    ensure!(n <= 3 && m.iter().all(|r| r.len() == n), "matrix size cap");
    if n == 1 {
        return Ok(m[0][0].clone());
    }
    let rows: Vec<_> = (1..n).collect();
    let mut out = one.zero();
    for j in 0..n {
        let cols: Vec<_> = (0..n).filter(|&k| k != j).collect();
        let minor = determinant(&submatrix(m, &rows, &cols), one, b)?;
        let term = b.mul(&m[0][j], &minor)?;
        out = if j % 2 == 0 {
            &out + &term
        } else {
            &out - &term
        };
    }
    bounded(&out, MAX_TERMS, MAX_DEGREE)?;
    Ok(out)
}
fn adjugate(m: &[Vec<Poly>], one: &Poly, b: &mut Budget) -> Result<Vec<Vec<Poly>>> {
    let n = m.len();
    let mut out = vec![vec![one.zero(); n]; n];
    for i in 0..n {
        for j in 0..n {
            let rows: Vec<_> = (0..n).filter(|&k| k != j).collect();
            let cols: Vec<_> = (0..n).filter(|&k| k != i).collect();
            let p = determinant(&submatrix(m, &rows, &cols), one, b)?;
            out[i][j] = if (i + j) % 2 == 0 { p } else { -p };
        }
    }
    Ok(out)
}
fn quadratic_form(m: &[Vec<Poly>], v: &[Poly], b: &mut Budget) -> Result<Poly> {
    ensure!(!v.is_empty(), "empty quadratic form");
    let mut out = v[0].zero();
    for i in 0..v.len() {
        for j in 0..v.len() {
            let term = b.mul(&v[i], &m[i][j])?;
            out = &out + &b.mul(&term, &v[j])?;
        }
    }
    bounded(&out, MAX_TERMS, MAX_DEGREE)?;
    Ok(out)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Minor {
    matrix: String,
    rows: Vec<usize>,
    columns: Vec<usize>,
    polynomial: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Face {
    states: Vec<State>,
    free_axes: Vec<usize>,
    restricted_polynomial: String,
    h: Vec<Vec<String>>,
    l: Vec<String>,
    c: String,
    determinant: String,
    adjugate: Vec<Vec<String>>,
    stationary_numerators: Vec<String>,
    critical_value_numerator: String,
    critical_value_sign_polynomial: String,
    lower_interior_guards: Vec<String>,
    upper_interior_guards: Vec<String>,
    determinant_identically_zero: bool,
    /// Only identically singular faces need the discovery-time rank inventory.
    /// Every d=0 specialization of all other faces is separately retained below.
    singular_rank_minors: Vec<Minor>,
    singular_fallback: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u32,
    input_sha256: String,
    faces: Vec<Face>,
}
fn semantic_hash(input: &Input) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(input)?)))
}

fn subsets(n: usize, count: usize) -> Vec<Vec<usize>> {
    (0usize..(1 << n))
        .filter(|m| m.count_ones() as usize == count)
        .map(|m| (0..n).filter(|i| m & (1 << i) != 0).collect())
        .collect()
}
fn rank_inventory(a: &Algebra, q: &Quadratic, b: &mut Budget) -> Result<Vec<Minor>> {
    let n = q.l.len();
    let two_h: Vec<Vec<_>> =
        q.h.iter()
            .map(|r| {
                r.iter()
                    .map(|p| p.clone().mul_coeff(Rational::from(2)))
                    .collect()
            })
            .collect();
    let augmented: Vec<Vec<_>> = two_h
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut r = r.clone();
            r.push(-q.l[i].clone());
            r
        })
        .collect();
    let mut out = Vec::new();
    for (name, matrix) in [
        ("gradient_matrix", &two_h),
        ("gradient_augmented_rhs_minus_l", &augmented),
    ] {
        for size in 1..=n {
            for rows in subsets(n, size) {
                for cols in subsets(matrix[0].len(), size) {
                    let d = determinant(&submatrix(matrix, &rows, &cols), &q.c.one(), b)?;
                    out.push(Minor {
                        matrix: name.into(),
                        rows: rows.clone(),
                        columns: cols,
                        polynomial: a.format(&d),
                    });
                }
            }
        }
    }
    Ok(out)
}

fn face_evidence(
    a: &Algebra,
    p: &Poly,
    axes: &[usize],
    state: Vec<State>,
    derivative_path: bool,
    b: &mut Budget,
) -> Result<Face> {
    let p = restrict(p, axes, &state);
    let free: Vec<_> = axes
        .iter()
        .zip(&state)
        .filter_map(|(&i, s)| (*s == State::Free).then_some(i))
        .collect();
    let q = if derivative_path {
        derivatives(&p, &free)
    } else {
        coefficients(&p, &free)?
    };
    let variables = free
        .iter()
        .map(|&i| a.parse(&a.names[i]))
        .collect::<Result<Vec<_>>>()?;
    let mut reconstruction = q.c.clone();
    if !free.is_empty() {
        reconstruction = &reconstruction + &quadratic_form(&q.h, &variables, b)?;
        for (l, v) in q.l.iter().zip(&variables) {
            reconstruction = &reconstruction + &b.mul(l, v)?;
        }
    }
    ensure!(
        p == reconstruction,
        "quadratic coefficient reconstruction failed"
    );
    let d = determinant(&q.h, &p.one(), b)?;
    let adj = adjugate(&q.h, &p.one(), b)?;
    let mut numerators = vec![p.zero(); free.len()];
    for i in 0..free.len() {
        for j in 0..free.len() {
            numerators[i] = &numerators[i] - &b.mul(&adj[i][j], &q.l[j])?;
        }
    }
    let mut g = b.mul(&d, &q.c)?.mul_coeff(Rational::from(4));
    if !free.is_empty() {
        g = &g - &quadratic_form(&adj, &q.l, b)?;
    }
    bounded(&g, MAX_TERMS, MAX_DEGREE)?;
    let mut gradient = q.l.clone();
    for i in 0..free.len() {
        let mut stationary = b.mul(&d, &q.l[i])?.mul_coeff(Rational::from(2));
        for j in 0..free.len() {
            stationary = &stationary
                + &b.mul(&q.h[i][j], &numerators[j])?
                    .mul_coeff(Rational::from(2));
            gradient[i] = &gradient[i]
                + &b.mul(&q.h[i][j], &variables[j])?
                    .mul_coeff(Rational::from(2));
        }
        ensure!(stationary.is_zero(), "stationary numerator identity failed");
        ensure!(
            gradient[i] == p.derivative(free[i]),
            "original derivative binding failed"
        );
    }
    let mut rhs = g.clone();
    if !free.is_empty() {
        rhs = &rhs + &quadratic_form(&adj, &gradient, b)?;
    }
    ensure!(
        b.mul(&d, &p)?.mul_coeff(Rational::from(4)) == rhs,
        "square-completion identity failed"
    );
    let mut lower = Vec::new();
    let mut upper = Vec::new();
    for n in &numerators {
        lower.push(a.format(&b.mul(n, &d)?));
        upper.push(a.format(&b.mul(&(&d.clone().mul_coeff(Rational::from(2)) - n), &d)?));
    }
    let rank = if d.is_zero() && !free.is_empty() {
        rank_inventory(a, &q, b)?
    } else {
        Vec::new()
    };
    let strings = |m: &[Vec<Poly>]| {
        m.iter()
            .map(|r| r.iter().map(|p| a.format(p)).collect())
            .collect()
    };
    Ok(Face { states:state, free_axes:free, restricted_polynomial:a.format(&p),h:strings(&q.h),l:q.l.iter().map(|p|a.format(p)).collect(),c:a.format(&q.c),
        determinant:a.format(&d), adjugate:strings(&adj),stationary_numerators:numerators.iter().map(|p|a.format(p)).collect(),
        critical_value_numerator:a.format(&g),critical_value_sign_polynomial:a.format(&b.mul(&g,&d)?),
        lower_interior_guards:lower,upper_interior_guards:upper,determinant_identically_zero:d.is_zero(),singular_rank_minors:rank,
        singular_fallback:if d.is_zero(){"all prefix fibers require rank/consistency and stationary-family analysis; no candidate count or emptiness claim"}else{"every specialization determinant=0 remains a fallback obligation; generic nonzero determinant does not exclude it"}.into() })
}

fn discover(input: &Input) -> Result<(Evidence, usize)> {
    let n = admit(input)?;
    let mut b = Budget::default();
    let mut faces = Vec::new();
    for ordinal in 0..3usize.pow(n.axes.len() as u32) {
        faces.push(face_evidence(
            &n.algebra,
            &n.polynomial,
            &n.axes,
            states(ordinal, n.axes.len()),
            true,
            &mut b,
        )?);
    }
    Ok((
        Evidence {
            schema_version: 1,
            input_sha256: semantic_hash(input)?,
            faces,
        },
        b.product_work,
    ))
}
fn verify(input: &Input, evidence: &Evidence) -> Result<usize> {
    ensure!(
        evidence.schema_version == 1 && evidence.input_sha256 == semantic_hash(input)?,
        "input/schema binding changed"
    );
    let n = admit(input)?;
    let expected = 3usize.pow(n.axes.len() as u32);
    ensure!(
        evidence.faces.len() == expected,
        "face inventory not exhaustive"
    );
    let mut b = Budget::default();
    for (ordinal, face) in evidence.faces.iter().enumerate() {
        let expected = face_evidence(
            &n.algebra,
            &n.polynomial,
            &n.axes,
            states(ordinal, n.axes.len()),
            false,
            &mut b,
        )?;
        ensure!(*face == expected, "face proof changed at ordinal {ordinal}");
    }
    Ok(b.product_work)
}

fn fresh(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= OUTPUT_BYTES, "output byte cap");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(&bytes)?;
    Ok(())
}
fn progress(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= OUTPUT_BYTES, "progress byte cap");
    let tmp = path.with_extension("progress.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path.with_extension("progress.json"))?;
    Ok(())
}
fn shapes(a: &Algebra, polys: &[Poly]) -> serde_json::Value {
    json!({"count":polys.len(),"terms":polys.iter().map(Poly::nterms).sum::<usize>(),"maximum_polynomial_terms":polys.iter().map(Poly::nterms).max().unwrap_or(0),"maximum_degrees":(0..a.names.len()).map(|i|polys.iter().map(|p|p.degree(i)).max().unwrap_or(0)).collect::<Vec<_>>()})
}
fn ordinary(input: &Input, path: &Path) -> Result<serde_json::Value> {
    let start = Instant::now();
    let n = admit(input)?;
    let mut polynomials = vec![n.polynomial];
    polynomials.extend(n.prefix);
    for &i in &n.axes {
        let v = n.algebra.parse(&n.algebra.names[i])?;
        polynomials.push(v.clone());
        polynomials.push(&v.one() - &v);
    }
    progress(
        path,
        &json!({"stage":"initial_checked_factorization","input":shapes(&n.algebra,&polynomials),"coverage_claim":false}),
    )?;
    polynomials = factors_checked(&polynomials)?;
    let mut stages = Vec::new();
    for &axis in n.axes.iter().rev() {
        ensure!(
            polynomials.len() <= 256
                && polynomials.iter().map(Poly::nterms).sum::<usize>() <= 32768,
            "projection input cap"
        );
        let shape = shapes(&n.algebra, &polynomials);
        let phase = Instant::now();
        progress(
            path,
            &json!({"stage":"ordinary_projection","axis":n.algebra.names[axis],"input":shape,"completed_stages":stages,"coverage_claim":false}),
        )?;
        let raw = projection_polynomials(&polynomials, axis)?;
        let oversized: Vec<_> = raw.iter().enumerate().filter(|(_, p)| p.nterms() > MAX_TERMS)
            .map(|(index, p)| json!({
                "raw_index": index,
                "terms": p.nterms(),
                "degrees": (0..p.nvars()).map(|i| p.degree(i)).collect::<Vec<_>>(),
                "exact_polynomial_sha256": format!("{:x}", Sha256::digest(n.algebra.format(p).as_bytes()))
            })).collect();
        progress(
            path,
            &json!({"stage":"projection_output_admission","axis":n.algebra.names[axis],"raw":shapes(&n.algebra,&raw),"maximum_admitted_polynomial_terms":MAX_PROJECTION_TERMS,"polynomials_above_initial_face_cap":oversized,"completed_stages":stages,"coverage_claim":false}),
        )?;
        ensure!(raw.len() <= 512, "projection output count cap");
        for p in &raw {
            bounded(p, MAX_PROJECTION_TERMS, MAX_DEGREE)?;
        }
        progress(
            path,
            &json!({"stage":"checked_factorization","axis":n.algebra.names[axis],"raw":shapes(&n.algebra,&raw),"polynomials_above_initial_face_cap":oversized,"completed_stages":stages,"coverage_claim":false}),
        )?;
        polynomials = factors_checked(&raw)?;
        stages.push(json!({"axis":n.algebra.names[axis],"input":shape,"output":shapes(&n.algebra,&polynomials),"seconds":phase.elapsed().as_secs_f64()}));
    }
    ensure!(
        polynomials
            .iter()
            .all(|p| n.axes.iter().all(|&i| p.degree(i) == 0)),
        "projection did not reach shared prefix"
    );
    Ok(
        json!({"status":"ordinary_block_projection_completed","coverage_claim":false,"claim":"Exact ordinary projection of the stated block and 0/1 bounds; no prefix decomposition or lifting performed", "input_sha256":semantic_hash(input)?,"variables":input.variables,"block":input.block,"seconds_including_parse":start.elapsed().as_secs_f64(),"stages":stages,"prefix_polynomials":polynomials.iter().map(|p|n.algebra.format(p)).collect::<Vec<_>>() }),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 120.,
        "deadline must be within120s"
    );
    ensure!(!args.output.exists(), "fresh output required");
    if let Some(p) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p)?;
    }
    if !args.worker {
        let sidecar = args.output.with_extension("process.json");
        ensure!(
            !sidecar.exists() && !args.output.with_extension("progress.json").exists(),
            "fresh sidecars required"
        );
        let mut command = Command::new("taskset");
        command
            .args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .args(std::env::args_os().skip(1))
            .arg("--worker")
            .env("RAYON_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1");
        let outcome = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        fresh(&sidecar, &outcome)?;
        ensure!(
            outcome.status == "completed"
                && outcome.exit_code == Some(0)
                && !outcome.output_truncated
                && args.output.is_file(),
            "bounded diagnostic incomplete; process/progress retained"
        );
        println!("bounded research diagnostic completed; no CAD claim");
        return Ok(());
    }
    let file = fs::File::open(&args.input)?;
    ensure!(
        file.metadata()?.len() <= INPUT_BYTES as u64,
        "input byte cap"
    );
    let mut bytes = Vec::new();
    file.take(INPUT_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= INPUT_BYTES, "input grew beyond cap");
    let input: Input = serde_json::from_slice(&bytes)?;
    let start = Instant::now();
    let report = match args.mode {
        Mode::Faces => {
            let (proof, work) = discover(&input)?;
            let discovery = start.elapsed().as_secs_f64();
            let replay = Instant::now();
            let restored: Evidence = serde_json::from_slice(&serde_json::to_vec(&proof)?)?;
            let replay_work = verify(&input, &restored)?;
            let replay_seconds = replay.elapsed().as_secs_f64();
            let statistics = Instant::now();
            let n = admit(&input)?;
            let mut unique = BTreeMap::new();
            let mut guards = 0usize;
            for face in &restored.faces {
                for expression in std::iter::once(&face.determinant)
                    .chain(std::iter::once(&face.critical_value_numerator))
                    .chain(face.lower_interior_guards.iter())
                    .chain(face.upper_interior_guards.iter())
                    .chain(
                        face.singular_rank_minors
                            .iter()
                            .map(|minor| &minor.polynomial),
                    )
                {
                    let p = n.algebra.parse(expression)?;
                    if !p.is_constant() {
                        guards += 1;
                        unique
                            .entry(n.algebra.format(&canonical(&p)))
                            .or_insert(p.nterms());
                    }
                }
            }
            for p in &n.prefix {
                if !p.is_constant() {
                    guards += 1;
                    unique
                        .entry(n.algebra.format(&canonical(p)))
                        .or_insert(p.nterms());
                }
            }
            json!({"status":"quadratic_face_identities_replayed","coverage_claim":false,"claim":"Stationary candidate inventory on every relatively open face of the closed conditional unit cube; all determinant-zero fibers remain fallback obligations; this is not a cylindrical projection", "input_file_sha256":format!("{:x}",Sha256::digest(&bytes)),"discovery_seconds_including_parse":discovery,"serialization_replay_seconds":replay_seconds,"guard_statistics_seconds":statistics.elapsed().as_secs_f64(),"total_seconds_before_serialization":start.elapsed().as_secs_f64(),"discovery_product_work":work,"replay_product_work":replay_work,"nonconstant_guard_occurrences_including_rank_and_prefix":guards,"unique_raw_nonconstant_guard_count":unique.len(),"unique_raw_guard_terms":unique.values().sum::<usize>(),"evidence":restored})
        }
        Mode::Projection => ordinary(&input, &args.output)?,
    };
    fresh(&args.output, &report)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(expression: &str, variables: &[&str], block: &[&str]) -> Input {
        Input {
            variables: variables.iter().map(|s| s.to_string()).collect(),
            block: block.iter().map(|s| s.to_string()).collect(),
            polynomial: expression.into(),
            prefix_constraints: Vec::new(),
            provenance: BTreeMap::new(),
        }
    }
    #[test]
    fn all_faces_replay_and_center_can_cross_zero_without_curvature_change() -> Result<()> {
        let i = input(
            "(x-1/2)^2+(y-1/2)^2+(z-1/2)^2-t",
            &["t", "x", "y", "z"],
            &["x", "y", "z"],
        );
        let (e, _) = discover(&i)?;
        assert_eq!(e.faces.len(), 27);
        verify(&i, &e)?;
        let a = Algebra::new(&i.variables)?;
        let f = &e.faces[0];
        assert_eq!(a.parse(&f.determinant)?, a.parse("1")?);
        assert_eq!(a.parse(&f.critical_value_numerator)?, a.parse("-4*t")?);
        assert_eq!(
            f.stationary_numerators
                .iter()
                .map(|p| a.parse(p))
                .collect::<Result<Vec<_>>>()?,
            vec![a.parse("1")?; 3]
        );
        Ok(())
    }
    #[test]
    fn negative_determinant_and_interior_tests_keep_correct_sign() -> Result<()> {
        let i = input("(x-1/2)^2-(y-1/2)^2+t", &["t", "x", "y"], &["x", "y"]);
        let (e, _) = discover(&i)?;
        verify(&i, &e)?;
        let a = Algebra::new(&i.variables)?;
        let f = &e.faces[0];
        assert_eq!(a.parse(&f.determinant)?, a.parse("-1")?);
        assert_eq!(a.parse(&f.critical_value_sign_polynomial)?, a.parse("4*t")?);
        for p in f
            .lower_interior_guards
            .iter()
            .chain(&f.upper_interior_guards)
        {
            assert_eq!(a.parse(p)?, a.parse("1")?);
        }
        Ok(())
    }
    #[test]
    fn singular_specialization_and_identically_flat_families_are_retained() -> Result<()> {
        for expression in ["t*x^2+y^2", "(x+y-1)^2+t", "t*x", "0"] {
            let i = input(expression, &["t", "x", "y"], &["x", "y"]);
            let (e, _) = discover(&i)?;
            verify(&i, &e)?;
            let f = &e.faces[0];
            if expression == "t*x^2+y^2" {
                assert!(!f.determinant_identically_zero);
                assert!(f.singular_fallback.contains("every specialization"));
            } else {
                assert!(f.determinant_identically_zero);
                assert_eq!(f.singular_rank_minors.len(), 14);
                assert!(f.singular_fallback.contains("no candidate count"));
            }
        }
        Ok(())
    }
    #[test]
    fn vertex_convention_and_affine_exterior_stationary_candidate() -> Result<()> {
        let i = input("(x-2)^2+t", &["t", "x"], &["x"]);
        let (e, _) = discover(&i)?;
        verify(&i, &e)?;
        let a = Algebra::new(&i.variables)?;
        assert_eq!(
            a.parse(&e.faces[0].upper_interior_guards[0])?,
            a.parse("-2")?
        );
        for face in &e.faces[1..] {
            assert!(face.free_axes.is_empty());
            assert_eq!(a.parse(&face.determinant)?, a.parse("1")?);
            assert_eq!(
                a.parse(&face.critical_value_numerator)?,
                a.parse(&face.c)?.mul_coeff(Rational::from(4))
            );
        }
        Ok(())
    }
    #[test]
    fn tampering_missing_faces_and_changed_input_are_rejected() -> Result<()> {
        let i = input("x^2+y^2+t", &["t", "x", "y"], &["x", "y"]);
        let (e, _) = discover(&i)?;
        let mut bad = e.clone();
        bad.faces[0].critical_value_numerator = "0".into();
        assert!(verify(&i, &bad).is_err());
        let mut bad = e.clone();
        bad.faces.pop();
        assert!(verify(&i, &bad).is_err());
        let mut bad = e.clone();
        bad.faces.swap(0, 1);
        assert!(verify(&i, &bad).is_err());
        let mut changed = i.clone();
        changed.polynomial = "x^2+y^2-t".into();
        assert!(verify(&changed, &e).is_err());
        let flat = input("(x+y)^2", &["x", "y"], &["x", "y"]);
        let (mut e, _) = discover(&flat)?;
        e.faces[0].singular_rank_minors[0].polynomial = "7".into();
        assert!(verify(&flat, &e).is_err());
        Ok(())
    }
    #[test]
    fn admission_bombs_cube_order_and_cubic_are_rejected() {
        for expression in ["x^3", "(1+x)^99999999", "1/(1+x)", "unknown+x"] {
            assert!(admit(&input(expression, &["t", "x"], &["x"])).is_err());
        }
        assert!(admit(&input("x^2", &["x", "t"], &["x"])).is_err());
        let mut i = input("x^2", &["t", "x"], &["x"]);
        i.prefix_constraints = vec!["x".into()];
        assert!(admit(&i).is_err());
        let mut b = Budget {
            product_work: MAX_WORK,
        };
        let a = Algebra::new(&["x".into()]).unwrap();
        assert!(
            b.mul(&a.parse("x").unwrap(), &a.parse("x").unwrap())
                .is_err()
        );
    }
}
