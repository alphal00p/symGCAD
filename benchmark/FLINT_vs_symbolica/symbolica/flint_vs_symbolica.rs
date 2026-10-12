//! Symbolica side of the FLINT-vs-Symbolica kernel benchmark
//! (benchmark/FLINT_vs_symbolica/README.md). Benchmark tooling, not production.
//!
//! * `convert` turns a symGCAD operation capture (SYMGCAD_CAPTURE_DIR JSON)
//!   into a plain integer term-list operand file (`symgcad-flint-bench-op 1`).
//! * `derive` writes the operands production actually hands to its numeric
//!   kernels: the Kronecker-encoded univariate integer pair of binary
//!   segmentation, one exact interpolation image, or (P, dP/dv) for gcd.
//! * `run` times one exact operation with one kernel, single threaded, and
//!   dumps the exact output as `coefficient e0 e1 ...` lines for canon.py.
//!
//! Build: cargo build --release --locked --features flint-bench-hooks --example flint_vs_symbolica
use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Instant,
};
use symbolica::{poly::factor::Factorize, prelude::*};
use symgcad::{
    algebra::{Algebra, Poly},
    optimized_projection::{AlgebraOperation, OperationCapture},
    resultant_segmentation::bench_hooks,
};

#[derive(Parser)]
#[command(about = "Symbolica side of benchmark/FLINT_vs_symbolica")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum DeriveKind {
    /// Production-kernel operands of a resultant/discriminant: writes
    /// <prefix>.segenc.op and/or <prefix>.interpimg.op when production would
    /// use binary segmentation / exact interpolation.
    Auto,
    /// gcd(P, dP/dv) of a factor operand (first square-free step).
    Gcd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Capture JSON -> operand file (denominators cleared, integer coefficients).
    Convert {
        capture: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Operand file -> derived operand file(s).
    Derive {
        op: PathBuf,
        #[arg(long, value_enum)]
        kind: DeriveKind,
        /// Output prefix (auto) or output file (gcd).
        #[arg(long)]
        out: PathBuf,
    },
    /// Time one kernel on one operand file.
    Run {
        op: PathBuf,
        /// production | ducos | brown | crt | native
        #[arg(long)]
        kernel: String,
        #[arg(long)]
        dump: Option<PathBuf>,
        #[arg(long)]
        json: PathBuf,
        #[arg(long, default_value_t = 1.0)]
        min_seconds: f64,
        #[arg(long, default_value_t = 25)]
        max_reps: usize,
    },
}

struct Op {
    operation: String,
    names: Vec<String>,
    variable: Option<usize>,
    polys: Vec<Poly>,
}

fn write_poly(out: &mut impl Write, p: &Poly) -> Result<()> {
    writeln!(out, "terms {}", p.nterms())?;
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        ensure!(
            c.is_integer(),
            "operand files hold integer coefficients only"
        );
        write!(out, "{}", c.numerator_ref())?;
        for x in e {
            write!(out, " {x}")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn write_op(
    path: &Path,
    operation: &str,
    names: &[String],
    variable: Option<usize>,
    polys: &[Poly],
) -> Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    writeln!(out, "symgcad-flint-bench-op 1")?;
    writeln!(out, "operation {operation}")?;
    writeln!(out, "variables {}", names.join(" "))?;
    writeln!(out, "variable {}", variable.map(|v| v as i64).unwrap_or(-1))?;
    writeln!(out, "polynomials {}", polys.len())?;
    for p in polys {
        write_poly(&mut out, p)?;
    }
    out.flush()?;
    Ok(())
}

fn read_op(path: &Path) -> Result<Op> {
    let reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut lines = reader.lines();
    let mut next = || -> Result<String> {
        lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("truncated operand file"))?
            .map_err(Into::into)
    };
    ensure!(next()? == "symgcad-flint-bench-op 1", "operand file magic");
    let operation = next()?
        .strip_prefix("operation ")
        .context("operation line")?
        .to_owned();
    let names: Vec<String> = next()?
        .strip_prefix("variables ")
        .context("variables line")?
        .split(' ')
        .map(str::to_owned)
        .collect();
    let variable: i64 = next()?
        .strip_prefix("variable ")
        .context("variable line")?
        .parse()?;
    let count: usize = next()?
        .strip_prefix("polynomials ")
        .context("polynomials line")?
        .parse()?;
    let algebra = Algebra::new(&names)?;
    let mut polys = Vec::with_capacity(count);
    for _ in 0..count {
        let terms: usize = next()?
            .strip_prefix("terms ")
            .context("terms line")?
            .parse()?;
        let mut entries = Vec::with_capacity(terms);
        for _ in 0..terms {
            let line = next()?;
            let mut fields = line.split(' ');
            let c = Integer::from_str(fields.next().context("coefficient")?)
                .map_err(|e| anyhow::anyhow!("coefficient parse: {e:?}"))?;
            let e: Vec<u32> = fields.map(|f| f.parse()).collect::<Result<_, _>>()?;
            ensure!(e.len() == names.len(), "exponent vector length");
            entries.push((e, c));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut p = algebra.parse("0")?;
        for (e, c) in entries {
            p.append_monomial(Rational::from(c), &e);
        }
        polys.push(p);
    }
    Ok(Op {
        operation,
        variable: (variable >= 0).then_some(variable as usize),
        names,
        polys,
    })
}

/// Integer associate obtained by clearing denominators only (no sign/content change
/// unless denominators are present).
fn integral(p: &Poly) -> (Poly, bool) {
    let mut lcm = Integer::one();
    for c in &p.coefficients {
        lcm = lcm.lcm(c.denominator_ref());
    }
    if lcm.is_one() {
        (p.clone(), false)
    } else {
        (p.clone().mul_coeff(Rational::from(lcm)), true)
    }
}

fn univariate_z(p: &Poly, variable: usize) -> UnivariatePolynomial<Z> {
    let image = p.to_univariate_from_univariate(variable);
    UnivariatePolynomial::from_coefficients(
        &Z,
        image
            .coefficients()
            .iter()
            .map(|c| {
                assert!(c.is_integer());
                c.numerator()
            })
            .collect(),
        Arc::new(p.variables()[variable].clone()),
    )
}

fn poly_from_univariate_z(algebra: &Algebra, u: &UnivariatePolynomial<Z>) -> Result<Poly> {
    let mut p = algebra.parse("0")?;
    for (k, c) in u.coefficients().iter().enumerate() {
        if !c.is_zero() {
            p.append_monomial(Rational::from(c.clone()), &[k as u32]);
        }
    }
    Ok(p)
}

fn convert(capture_path: &Path, out: &Path) -> Result<serde_json::Value> {
    let bytes = std::fs::read(capture_path)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("capture omitted operands"))?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture input checksum mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let mut scaled = false;
    let polys = operands
        .iter()
        .map(|s| {
            let (p, changed) = integral(&algebra.parse(s)?);
            scaled |= changed;
            Ok(p)
        })
        .collect::<Result<Vec<_>>>()?;
    let operation = match capture.operation {
        AlgebraOperation::Factor => "factor",
        AlgebraOperation::Resultant => "resultant",
        AlgebraOperation::Discriminant => "discriminant",
        #[allow(unreachable_patterns)]
        _ => bail!("unsupported operation"),
    };
    write_op(
        out,
        operation,
        &capture.variable_names,
        capture.variable,
        &polys,
    )?;
    Ok(
        json!({"capture":capture_path,"capture_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "out":out,"operation":operation,"scaled_to_integer":scaled,
        "capture_complete":capture.complete,"capture_elapsed_seconds":capture.elapsed_seconds,
        "capture_backend":capture.backend_version}),
    )
}

/// Production-kernel operands of a resultant/discriminant operand file. The
/// applicability tests mirror production dispatch (projection.rs): pair
/// resultants try two-parameter segmentation first; discriminants of degree
/// > 4 try exact interpolation, then 1..2-parameter segmentation.
fn derive_auto(op: &Op, prefix: &Path) -> Result<Vec<serde_json::Value>> {
    let mut derived = Vec::new();
    let Some(variable) = op.variable else {
        return Ok(derived);
    };
    let (p, q) = match op.operation.as_str() {
        "resultant" => (op.polys[0].clone(), op.polys[1].clone()),
        "discriminant" => (op.polys[0].clone(), op.polys[0].derivative(variable)),
        _ => return Ok(derived),
    };
    let x = ["X".to_owned()];
    let x_algebra = Algebra::new(&x)?;
    let mut seg = symgcad::resultant_segmentation::Options::default();
    if op.operation == "discriminant" {
        seg.min_parameters = 1;
        seg.max_parameters = 2;
    }
    let interpolation = symgcad::resultant_interpolation::plan(
        &p,
        &q,
        variable,
        &symgcad::resultant_interpolation::Options::default(),
    )?;
    let segmentation_used = if op.operation == "discriminant" {
        p.degree(variable) > 4 && interpolation.is_none()
    } else {
        true
    };
    if segmentation_used
        && let Some((ep, eq)) = bench_hooks::encoded_operands(&p, &q, variable, &seg)?
    {
        let path = PathBuf::from(format!("{}.segenc.op", prefix.display()));
        let polys = [
            poly_from_univariate_z(&x_algebra, &ep)?,
            poly_from_univariate_z(&x_algebra, &eq)?,
        ];
        write_op(&path, "resultant", &x, Some(0), &polys)?;
        derived.push(json!({"kind":"segmentation-encoded","path":path}));
    }
    if (op.operation == "resultant" || p.degree(variable) > 4)
        && let Some(plan) = interpolation
    {
        // One representative image: the middle evaluation point of the
        // degree_bound + 1 points production interpolates through.
        let k = plan.degree_bound.div_ceil(2) as i64;
        let point = Rational::from(k);
        let image = |f: &Poly| integral(&f.replace(plan.parameter, &point)).0;
        let (pi, qi) = (image(&p), image(&q));
        let polys = [
            poly_from_univariate_z(&x_algebra, &univariate_z(&pi, variable))?,
            poly_from_univariate_z(&x_algebra, &univariate_z(&qi, variable))?,
        ];
        let path = PathBuf::from(format!("{}.interpimg.op", prefix.display()));
        write_op(&path, "resultant", &x, Some(0), &polys)?;
        derived.push(json!({"kind":"interpolation-image","path":path,
            "point":k,"images":plan.degree_bound+1,"parameter":plan.parameter}));
    }
    Ok(derived)
}

fn sizes(op: &Op) -> serde_json::Value {
    let n = op.names.len();
    let active: Vec<usize> = (0..n)
        .filter(|&v| op.polys.iter().any(|p| p.degree(v) > 0))
        .collect();
    json!({
        "nvars":n,"active_variables":active.iter().map(|&v|op.names[v].clone()).collect::<Vec<_>>(),
        "terms":op.polys.iter().map(Poly::nterms).collect::<Vec<_>>(),
        "degrees":op.polys.iter().map(|p|active.iter().map(|&v|p.degree(v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "total_degrees":op.polys.iter().map(|p|p.exponents_iter().map(|e|e.iter().map(|&x|x as u64).sum::<u64>()).max().unwrap_or(0)).collect::<Vec<_>>(),
        "elimination_degrees":op.variable.map(|v|op.polys.iter().map(|p|p.degree(v)).collect::<Vec<_>>()),
        "coefficient_bits":op.polys.iter().map(|p|p.coefficients.iter().map(|c|c.numerator_ref().significant_bits()).max().unwrap_or(0)).collect::<Vec<_>>(),
    })
}

enum Output {
    Polys(Vec<Poly>),
    Factors(Vec<(Poly, usize)>),
}

fn discriminant_via(p: &Poly, variable: usize, r: Poly) -> Result<Poly> {
    let r = if r.is_constant() {
        p.constant(r.get_constant())
    } else {
        r
    };
    let (mut d, rem) = r.quot_rem(&p.univariate_lcoeff(variable), false);
    ensure!(rem.is_zero(), "discriminant division not exact");
    let deg = p.degree(variable);
    if deg % 4 == 2 || deg % 4 == 3 {
        d = -d;
    }
    Ok(d)
}

/// One timed call. Note: the production labels below (which backend
/// production dispatched to) are computed inside the timed region exactly as
/// in the reported campaign; the extra static planning is negligible.
fn run_kernel(op: &Op, kernel: &str, label: &mut String) -> Result<Output> {
    use symgcad::projection;
    let v = op.variable;
    Ok(match (op.operation.as_str(), kernel) {
        ("resultant", "production") => {
            let (a, b) = (&op.polys[0], &op.polys[1]);
            let v = v.context("variable")?;
            if a.nvars() == 1 {
                // Production's numeric kernel inside interpolation/segmentation.
                *label = "integer-ducos-univariate".into();
                let r = bench_hooks::integer_resultant(&univariate_z(a, v), &univariate_z(b, v));
                return Ok(Output::Polys(vec![a.constant(Rational::from(r))]));
            }
            if let Some((r, _)) = symgcad::resultant_segmentation::try_resultant(
                a,
                b,
                v,
                &symgcad::resultant_segmentation::Options::default(),
                || Ok(()),
            )? {
                *label = "binary-segmentation".into();
                Output::Polys(vec![r])
            } else {
                let (r, stats) = projection::exact_resultant_with_statistics(a, b, v)?;
                *label = if stats.is_some() {
                    "interpolation".into()
                } else if a.degree(v).min(b.degree(v)) <= 1 {
                    "linear-identity".into()
                } else {
                    "integer-ducos".into()
                };
                Output::Polys(vec![r])
            }
        }
        ("discriminant", "production") => {
            let p = &op.polys[0];
            let v = v.context("variable")?;
            let deg = p.degree(v);
            let (d, seg) = projection::discriminant_with_binary_statistics(p, v)?;
            *label = if deg <= 1 {
                "trivial".into()
            } else if deg <= 4 {
                "formula".into()
            } else if seg.is_some() {
                "binary-segmentation".into()
            } else if symgcad::resultant_interpolation::plan(
                p,
                &p.derivative(v),
                v,
                &symgcad::resultant_interpolation::Options::default(),
            )?
            .is_some()
            {
                "interpolation".into()
            } else {
                "integer-ducos".into()
            };
            Output::Polys(vec![d])
        }
        ("resultant" | "discriminant", "ducos" | "brown" | "crt") => {
            let v = v.context("variable")?;
            let (a, b) = if op.operation == "resultant" {
                (op.polys[0].clone(), op.polys[1].clone())
            } else {
                (op.polys[0].clone(), op.polys[0].derivative(v))
            };
            ensure!(
                a.degree(v) >= 2 && b.degree(v) >= 2,
                "kernel comparison needs elimination degrees >= 2"
            );
            *label = kernel.into();
            let r = match kernel {
                // Production's deterministic fallback (vanishing test + integer Ducos).
                "ducos" => projection::exact_resultant_ducos(&a, &b, v)?,
                // Symbolica's own multivariate alternatives (not used by production).
                "brown" => a.to_univariate(v).resultant_brown(&b.to_univariate(v)),
                "crt" => a.to_univariate(v).resultant_crt(&b.to_univariate(v)),
                _ => unreachable!(),
            };
            let r = if r.is_constant() {
                a.constant(r.get_constant())
            } else {
                r
            };
            if op.operation == "resultant" {
                Output::Polys(vec![r])
            } else {
                Output::Polys(vec![discriminant_via(&op.polys[0], v, r)?])
            }
        }
        ("factor", "production") => {
            *label = "symgcad-factor-basis".into();
            Output::Polys(symgcad::algebra::factors(&op.polys[..1])?)
        }
        ("factor", "native") => {
            *label = "symbolica-factor".into();
            Output::Factors(op.polys[0].factor())
        }
        ("gcd", "production" | "native") => {
            *label = "symbolica-gcd".into();
            Output::Polys(vec![op.polys[0].gcd(&op.polys[1])])
        }
        (o, k) => bail!("unsupported operation/kernel {o}/{k}"),
    })
}

fn dump(path: &Path, output: &Output) -> Result<(usize, u64)> {
    let mut out = BufWriter::new(File::create(path)?);
    let mut terms = 0;
    let mut bits = 0u64;
    let mut write = |out: &mut BufWriter<File>, p: &Poly| -> Result<()> {
        terms += p.nterms();
        for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
            bits = bits.max(c.numerator_ref().significant_bits());
            if c.is_integer() {
                write!(out, "{}", c.numerator_ref())?;
            } else {
                write!(out, "{}/{}", c.numerator_ref(), c.denominator_ref())?;
            }
            for x in e {
                write!(out, " {x}")?;
            }
            writeln!(out)?;
        }
        Ok(())
    };
    match output {
        Output::Polys(ps) => {
            for p in ps {
                writeln!(out, "poly 1")?;
                write(&mut out, p)?;
            }
        }
        Output::Factors(fs) => {
            for (p, m) in fs {
                writeln!(out, "poly {m}")?;
                write(&mut out, p)?;
            }
        }
    }
    out.flush()?;
    Ok((terms, bits))
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::Convert { capture, out } => {
            let report = convert(&capture, &out)?;
            println!("{}", serde_json::to_string(&report)?);
        }
        Cmd::Derive {
            op,
            kind: DeriveKind::Auto,
            out,
        } => {
            let parsed = read_op(&op)?;
            let derived = derive_auto(&parsed, &out)?;
            println!("{}", json!({"op":op,"derived":derived}));
        }
        Cmd::Derive {
            op,
            kind: DeriveKind::Gcd,
            out,
        } => {
            let op = read_op(&op)?;
            ensure!(op.operation == "factor", "gcd derivation needs a factor op");
            let p = &op.polys[0];
            // Square-free step along the variable of largest degree (first on ties).
            let v = (0..op.names.len())
                .max_by_key(|&v| (p.degree(v), std::cmp::Reverse(v)))
                .unwrap();
            ensure!(p.degree(v) > 0, "constant factor operand");
            write_op(
                &out,
                "gcd",
                &op.names,
                Some(v),
                &[p.clone(), p.derivative(v)],
            )?;
            println!("{}", json!({"out":out,"derivative_variable":op.names[v]}));
        }
        Cmd::Run {
            op,
            kernel,
            dump: dump_path,
            json: json_path,
            min_seconds,
            max_reps,
        } => {
            let parse_started = Instant::now();
            let parsed = read_op(&op)?;
            let parse_seconds = parse_started.elapsed().as_secs_f64();
            let mut label = String::new();
            let mut times = Vec::new();
            let mut output;
            let mut total = 0.0;
            loop {
                let started = Instant::now();
                output = run_kernel(&parsed, &kernel, &mut label)?;
                let t = started.elapsed().as_secs_f64();
                times.push(t);
                total += t;
                if total >= min_seconds || times.len() >= max_reps {
                    break;
                }
                drop(output);
            }
            let (terms, bits) = match &dump_path {
                Some(path) => dump(path, &output)?,
                None => (0, 0),
            };
            let mut sorted = times.clone();
            sorted.sort_by(f64::total_cmp);
            let report = json!({
                "side":"symbolica","backend":LicenseManager::get_version(),
                "op":op,"operation":parsed.operation,"kernel":kernel,"chosen":label,
                "sizes":sizes(&parsed),"parse_seconds":parse_seconds,
                "reps":times.len(),"min_seconds":sorted[0],
                "median_seconds":sorted[sorted.len()/2],"times":times,
                "output_terms":terms,"output_max_bits":bits,
            });
            std::fs::write(&json_path, serde_json::to_vec_pretty(&report)?)?;
        }
    }
    Ok(())
}
