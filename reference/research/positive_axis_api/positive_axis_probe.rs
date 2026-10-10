//! Isolated research harness; never a production CAD or root-index change.
use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
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
// Resolved after this artifact is copied to the isolated examples/ directory.
#[path = "../src/roots/integer_sign.rs"]
mod integer_sign;
use symgcad::algebra;
use symgcad::{
    algebra::{Algebra, rational},
    roots::{FiberRoot, refine_once},
};

const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_REPORT: u64 = 8 * 1024 * 1024;
const MAX_DEGREE: usize = 4096;
const MAX_HEIGHT: u64 = 20_000;
const MAX_INPUT_BITS: u64 = 64_000_000;
const MAX_ENDPOINT_BITS: u64 = 131_072;
const MAX_REFINEMENTS: usize = 4096;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Full,
    Positive,
    Compare,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum RingMode {
    Integer,
    Rational,
}
#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    input_sha256: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_enum)]
    mode: Mode,
    #[arg(long, value_enum, default_value = "integer")]
    ring: RingMode,
    #[arg(long)]
    left: Option<PathBuf>,
    #[arg(long)]
    left_sha256: Option<String>,
    #[arg(long)]
    right: Option<PathBuf>,
    #[arg(long)]
    right_sha256: Option<String>,
    #[arg(long, default_value_t = 2)]
    cpu: usize,
    #[arg(long, default_value_t = 1800.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Term {
    coefficient: String,
    exponents: Vec<usize>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u32,
    variables: Vec<String>,
    axis: usize,
    terms: Vec<Term>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedRoot {
    lower: String,
    upper: String,
    multiplicity: Option<usize>,
}
#[derive(Clone, Debug)]
struct Root {
    lower: Rational,
    upper: Rational,
    multiplicity: Option<usize>,
}
impl Root {
    fn saved(&self) -> SavedRoot {
        SavedRoot {
            lower: self.lower.to_string(),
            upper: self.upper.to_string(),
            multiplicity: self.multiplicity,
        }
    }
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
    let mut f = options.open(path)?;
    ensure!(
        f.metadata()?.is_file() && f.metadata()?.len() <= cap,
        "regular bounded input required"
    );
    let mut bytes = Vec::new();
    (&mut f).take(cap + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= cap, "input grew beyond byte cap");
    Ok(bytes)
}
fn file_hash(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = f.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn save(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() as u64 <= MAX_REPORT, "output byte cap");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    Ok(())
}
fn literal_integer(s: &str, bits: u64) -> Result<Integer> {
    let d = s.strip_prefix('-').unwrap_or(s);
    ensure!(
        !d.is_empty()
            && d.len() as u64 <= bits * 30103 / 100000 + 1
            && d.bytes().all(|b| b.is_ascii_digit()),
        "bounded integer literal required"
    );
    let r = rational(s)?;
    ensure!(
        r.denominator_ref().is_one() && r.numerator_ref().significant_bits() <= bits,
        "integer height cap"
    );
    Ok(r.numerator_ref().clone())
}
fn literal_rational(s: &str) -> Result<Rational> {
    let pieces = s.split('/').collect::<Vec<_>>();
    ensure!((1..=2).contains(&pieces.len()), "rational literal grammar");
    let n = literal_integer(pieces[0], MAX_ENDPOINT_BITS)?;
    let d = if pieces.len() == 2 {
        literal_integer(pieces[1], MAX_ENDPOINT_BITS)?
    } else {
        Integer::one()
    };
    ensure!(
        d > Integer::zero(),
        "positive rational denominator required"
    );
    Ok(Rational::from(n) / Rational::from(d))
}
fn parse_input(bytes: &[u8], expected: &str) -> Result<(Input, UnivariatePolynomial<Z>, String)> {
    ensure!(
        bytes.len() as u64 <= MAX_BYTES && expected.len() == 64 && hash(bytes) == expected,
        "input hash/byte binding"
    );
    let input: Input = serde_json::from_slice(bytes)?;
    ensure!(
        input.schema_version == 1
            && !input.variables.is_empty()
            && input.variables.len() <= 16
            && input.axis < input.variables.len()
            && input.terms.len() <= MAX_DEGREE + 1,
        "input schema/map/term cap"
    );
    let mut names = BTreeSet::new();
    for n in &input.variables {
        ensure!(
            !n.is_empty()
                && n.len() <= 64
                && n.as_bytes()[0].is_ascii_alphabetic()
                && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && names.insert(n),
            "distinct bounded variable names"
        );
    }
    let mut previous = None;
    let mut estimated_bits = 0u64;
    for term in &input.terms {
        ensure!(
            term.exponents.len() == input.variables.len()
                && term
                    .exponents
                    .iter()
                    .enumerate()
                    .all(|(a, e)| if a == input.axis {
                        *e <= MAX_DEGREE
                    } else {
                        *e == 0
                    }),
            "full univariate support required"
        );
        let e = term.exponents[input.axis];
        ensure!(
            previous.is_none_or(|p| p < e),
            "ordered unique exponents required"
        );
        previous = Some(e);
        let d = term
            .coefficient
            .strip_prefix('-')
            .unwrap_or(&term.coefficient);
        ensure!(
            !d.is_empty()
                && d.len() as u64 <= MAX_HEIGHT * 30103 / 100000 + 1
                && d.bytes().all(|b| b.is_ascii_digit()),
            "coefficient literal preflight"
        );
        estimated_bits = estimated_bits
            .checked_add(4 * d.len() as u64)
            .context("input size overflow")?;
        ensure!(estimated_bits <= MAX_INPUT_BITS, "input storage cap");
    }
    let mut coefficients = vec![Integer::zero(); previous.map_or(0, |e| e + 1)];
    let mut literal = String::new();
    for (i, t) in input.terms.iter().enumerate() {
        let c = literal_integer(&t.coefficient, MAX_HEIGHT)?;
        ensure!(!c.is_zero(), "sparse zero term");
        coefficients[t.exponents[input.axis]] = c;
        if i > 0 {
            literal.push('+');
        }
        literal.push_str(&format!(
            "({})*{}^{}",
            t.coefficient, input.variables[input.axis], t.exponents[input.axis]
        ));
    }
    let algebra = Algebra::new(&input.variables)?;
    let p = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(algebra.variables[input.axis].clone()),
    );
    Ok((input, p, hash(literal.as_bytes())))
}
fn validate(roots: &[Root], degree: usize, positive: bool) -> Result<()> {
    ensure!(roots.len() <= degree, "root inventory exceeds degree");
    for (i, r) in roots.iter().enumerate() {
        ensure!(
            r.lower <= r.upper && r.multiplicity.is_none_or(|m| m > 0 && m <= degree),
            "invalid isolating interval/multiplicity"
        );
        if positive {
            ensure!(
                r.lower >= Rational::zero() && r.upper > Rational::zero(),
                "nonpositive root in positive inventory"
            );
        }
        if i > 0 {
            ensure!(
                roots[i - 1].upper <= r.lower
                    && !(roots[i - 1].lower == roots[i - 1].upper
                        && r.lower == r.upper
                        && roots[i - 1].lower == r.lower),
                "inventory intervals overlap or duplicate root"
            );
        }
    }
    Ok(())
}
fn isolate(p: &UnivariatePolynomial<Z>, mode: Mode, ring: RingMode) -> Result<(Vec<Root>, f64)> {
    let q =
        matches!(ring, RingMode::Rational).then(|| p.map_coeff(|c| Rational::from(c.clone()), Q));
    let start = Instant::now();
    let rows = match ring {
        RingMode::Integer => match mode {
            Mode::Full => p.isolate_real_root_intervals(),
            Mode::Positive => p.isolate_positive_real_root_intervals(),
            Mode::Compare => anyhow::bail!("invalid isolation mode"),
        },
        RingMode::Rational => {
            let q = q.as_ref().expect("rational conversion prepared");
            match mode {
                Mode::Full => q.isolate_real_root_intervals(),
                Mode::Positive => q.isolate_positive_real_root_intervals(),
                Mode::Compare => anyhow::bail!("invalid isolation mode"),
            }
        }
    };
    let seconds = start.elapsed().as_secs_f64();
    let roots = rows
        .into_iter()
        .map(|(lower, upper, m)| Root {
            lower,
            upper,
            multiplicity: Some(m),
        })
        .collect::<Vec<_>>();
    validate(&roots, p.degree(), mode == Mode::Positive)?;
    Ok((roots, seconds))
}
fn positive_part(roots: &[Root]) -> Result<Vec<Root>> {
    let mut out = Vec::new();
    for r in roots {
        if r.upper <= Rational::zero() {
            continue;
        }
        ensure!(
            r.lower >= Rational::zero(),
            "interval straddles zero; classification must be refined explicitly"
        );
        out.push(r.clone());
    }
    Ok(out)
}
fn load_inventory(
    path: &Path,
    expected: &str,
    input: &Input,
    input_hash: &str,
    literal_hash: &str,
    degree: usize,
) -> Result<(Vec<Root>, bool, Mode)> {
    let bytes = read(path, MAX_REPORT)?;
    ensure!(
        expected.len() == 64 && hash(&bytes) == expected,
        "inventory SHA binding"
    );
    let value: Value = serde_json::from_slice(&bytes)?;
    ensure!(value["complete"] == true, "incomplete source inventory");
    let legacy = value["kind"] == "native_single_block_root_inventory";
    let mut roots = Vec::new();
    if legacy {
        ensure!(
            value["schema_version"] == 1
                && value["binding"]["product"]["full_variable_map"] == json!(input.variables)
                && value["block_metadata"]["polynomial"]["sparse_expression_sha256"]
                    == literal_hash
                && value["block_metadata"]["polynomial"]["degree"] == degree,
            "legacy full-map/literal polynomial binding"
        );
        for (index, row) in value["intervals"]
            .as_array()
            .context("legacy intervals")?
            .iter()
            .enumerate()
        {
            ensure!(row["full_real_index"] == index, "legacy index inventory");
            roots.push(Root {
                lower: literal_rational(row["lower"].as_str().context("lower")?)?,
                upper: literal_rational(row["upper"].as_str().context("upper")?)?,
                multiplicity: None,
            });
        }
        ensure!(
            value["distinct_roots_in_this_block"] == roots.len(),
            "legacy root count"
        );
    } else {
        ensure!(
            value["schema_version"] == 1
                && value["kind"] == "positive_axis_native_inventory"
                && value["input_sha256"] == input_hash
                && value["literal_expression_sha256"] == literal_hash
                && value["variables"] == json!(input.variables)
                && value["axis"] == input.axis
                && value["degree"] == degree,
            "native inventory binding"
        );
        let rows: Vec<SavedRoot> = serde_json::from_value(value["intervals"].clone())?;
        for row in rows {
            ensure!(row.multiplicity.is_some(), "native multiplicity missing");
            roots.push(Root {
                lower: literal_rational(&row.lower)?,
                upper: literal_rational(&row.upper)?,
                multiplicity: row.multiplicity,
            });
        }
        ensure!(value["root_count"] == roots.len(), "native root count");
        ensure!(
            value["mode"] == "full" || value["mode"] == "positive",
            "native mode binding"
        );
        validate(&roots, degree, value["mode"] == "positive")?;
    }
    validate(&roots, degree, false)?;
    let mode = if legacy || value["mode"] == "full" {
        Mode::Full
    } else {
        Mode::Positive
    };
    Ok((roots, legacy, mode))
}
fn sign(p: &UnivariatePolynomial<Q>, x: &Rational) -> i8 {
    integer_sign::sign(p, x)
}
fn contains(r: &Root, x: &Rational) -> bool {
    if r.lower == r.upper {
        r.lower == *x
    } else {
        r.lower < *x && *x < r.upper
    }
}
fn refine(root: &mut Root, p: &Arc<UnivariatePolynomial<Q>>) -> Result<()> {
    let mut f = FiberRoot {
        polynomial: 0,
        index: 0,
        interval: (root.lower.clone(), root.upper.clone()),
        fiber: p.clone(),
    };
    refine_once(&mut f);
    root.lower = f.interval.0;
    root.upper = f.interval.1;
    ensure!(
        [&root.lower, &root.upper]
            .iter()
            .all(
                |r| r.numerator_ref().significant_bits() <= MAX_ENDPOINT_BITS
                    && r.denominator_ref().significant_bits() <= MAX_ENDPOINT_BITS
            ),
        "refined endpoint height cap"
    );
    Ok(())
}
fn same_root(a: &Root, b: &Root, p: &Arc<UnivariatePolynomial<Q>>) -> Result<(Value, usize)> {
    let mut a = a.clone();
    let mut b = b.clone();
    for steps in 0..=MAX_REFINEMENTS {
        for r in [&a, &b] {
            if r.lower == r.upper
                && contains(&a, &r.lower)
                && contains(&b, &r.lower)
                && sign(p, &r.lower) == 0
            {
                return Ok((
                    json!({"method":"exact_rational_root","root":r.lower.to_string()}),
                    steps,
                ));
            }
        }
        let lo = a.lower.clone().max(b.lower.clone());
        let hi = a.upper.clone().min(b.upper.clone());
        ensure!(
            lo < hi,
            "corresponding isolating intervals contain different roots"
        );
        let sl = sign(p, &lo);
        let sh = sign(p, &hi);
        if sl != 0 && sh != 0 && sl != sh {
            return Ok((
                json!({"method":"sign_changing_intersection","lower":lo.to_string(),"upper":hi.to_string(),"lower_sign":sl,"upper_sign":sh}),
                steps,
            ));
        }
        ensure!(steps < MAX_REFINEMENTS, "matching refinement cap");
        refine(&mut a, p)?;
        refine(&mut b, p)?;
    }
    unreachable!()
}
fn compare(
    p: &UnivariatePolynomial<Z>,
    left: &[Root],
    right: &[Root],
    all_real: bool,
) -> Result<Value> {
    let start = Instant::now();
    let repeated = left
        .iter()
        .chain(right)
        .any(|r| r.multiplicity.is_some_and(|m| m != 1));
    let mut polynomial = p.map_coeff(|c| Rational::from(c.clone()), Q);
    let mut gcd_degree = None;
    if repeated {
        let g = polynomial.gcd(&polynomial.derivative());
        gcd_degree = Some(g.degree());
        let (sqf, rem) = polynomial.quot_rem(&g);
        ensure!(rem.is_zero(), "square-free exact division failed");
        polynomial = sqf;
    }
    let squarefree_seconds = start.elapsed().as_secs_f64();
    let polynomial = Arc::new(polynomial);
    let lp = if all_real {
        left.to_vec()
    } else {
        positive_part(left)?
    };
    let rp = if all_real {
        right.to_vec()
    } else {
        positive_part(right)?
    };
    ensure!(
        lp.len() == rp.len(),
        "requested root inventory counts differ"
    );
    let start = Instant::now();
    let mut matches = Vec::new();
    let mut multiplicities = true;
    for (index, (a, b)) in lp.iter().zip(&rp).enumerate() {
        if let (Some(am), Some(bm)) = (a.multiplicity, b.multiplicity) {
            ensure!(am == bm, "root multiplicities differ");
        } else {
            multiplicities = false;
        }
        let (proof, refinements) = same_root(a, b, &polynomial)?;
        matches.push(json!({"index":index,"left":a.saved(),"right":b.saved(),"proof":proof,"refinements":refinements}));
    }
    Ok(
        json!({"scope":if all_real { "all_real" } else { "strictly_positive" },"matched_roots":matches.len(),"multiplicities_compared":multiplicities,"squarefree_gcd_computed":repeated,"squarefree_gcd_degree":gcd_degree,"squarefree_preparation_seconds":squarefree_seconds,"matching_seconds":start.elapsed().as_secs_f64(),"matches":matches,"proof_basis":"source native unique-root isolating contracts plus exact rational root or sign-changing intersection; legacy outer block multiplicity is not a native root multiplicity"}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 1800.0,
        "wall cap must be <=1800 seconds"
    );
    ensure!(!args.output.exists(), "fresh output required");
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    if !args.worker {
        let sidecar = args.output.with_extension("process.json");
        ensure!(!sidecar.exists(), "fresh process evidence required");
        let mut cmd = Command::new("taskset");
        cmd.args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .args(std::env::args_os().skip(1))
            .arg("--worker")
            .env("RAYON_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1");
        let outcome = symgcad::benchmark::run_bounded(
            &mut cmd,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 4096,
                workers: 1,
                max_cells: None,
            },
        )?;
        save(&sidecar, &serde_json::to_value(&outcome)?)?;
        ensure!(
            outcome.status == "completed"
                && outcome.exit_code == Some(0)
                && !outcome.output_truncated
                && args.output.is_file(),
            "bounded measurement incomplete; process evidence retained"
        );
        println!("bounded isolated native measurement completed");
        return Ok(());
    }
    let bytes = read(&args.input, MAX_BYTES)?;
    let (input, p, literal_hash) = parse_input(&bytes, &args.input_sha256)?;
    let executable_sha256 = file_hash(&std::env::current_exe()?)?;
    let integer_sign_source_sha256 = hash(include_bytes!("../src/roots/integer_sign.rs"));
    let report = if args.mode == Mode::Compare {
        let left_path = args.left.as_deref().context("compare requires --left")?;
        let right_path = args.right.as_deref().context("compare requires --right")?;
        let lh = args.left_sha256.as_deref().context("left SHA required")?;
        let rh = args.right_sha256.as_deref().context("right SHA required")?;
        let (left, ll, lm) = load_inventory(
            left_path,
            lh,
            &input,
            &args.input_sha256,
            &literal_hash,
            p.degree(),
        )?;
        let (right, rl, rm) = load_inventory(
            right_path,
            rh,
            &input,
            &args.input_sha256,
            &literal_hash,
            p.degree(),
        )?;
        json!({"schema_version":1,"kind":"positive_axis_inventory_comparison","complete":true,"input_sha256":args.input_sha256,"literal_expression_sha256":literal_hash,"executable_sha256":executable_sha256,"integer_sign_source_sha256":integer_sign_source_sha256,"left_sha256":lh,"right_sha256":rh,"left_legacy":ll,"right_legacy":rl,"polynomial_is_zero":p.is_zero(),"comparison":compare(&p,&left,&right,lm == Mode::Full && rm == Mode::Full)?,"cad_coverage":false})
    } else {
        ensure!(
            args.left.is_none()
                && args.right.is_none()
                && args.left_sha256.is_none()
                && args.right_sha256.is_none(),
            "inventory arguments only in compare mode"
        );
        let (roots, seconds) = isolate(&p, args.mode, args.ring)?;
        json!({"schema_version":1,"kind":"positive_axis_native_inventory","complete":true,"input_sha256":args.input_sha256,"literal_expression_sha256":literal_hash,"executable_sha256":executable_sha256,"integer_sign_source_sha256":integer_sign_source_sha256,"variables":input.variables,"axis":input.axis,"mode":args.mode,"ring":args.ring,"polynomial_is_zero":p.is_zero(),"zero_polynomial_semantics":"empty finite isolated-root inventory is not a claim that the zero polynomial has no zeros","degree":p.degree(),"root_count":roots.len(),"native_isolation_seconds":seconds,"intervals":roots.iter().map(Root::saved).collect::<Vec<_>>(),"cad_coverage":false})
    };
    save(&args.output, &report)?;
    println!("native inventory or exact matching completed; no CAD claim");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn poly(c: &[i64]) -> UnivariatePolynomial<Z> {
        let a = Algebra::new(&["x".into()]).unwrap();
        UnivariatePolynomial::from_coefficients(
            &Z,
            c.iter().map(|c| Integer::from(*c)).collect(),
            Arc::new(a.variables[0].clone()),
        )
    }
    #[test]
    fn full_positive_both_rings_and_multiplicities() -> Result<()> {
        let x = poly(&[0, 1]);
        let a = poly(&[-2, 0, 1]);
        let b = poly(&[-1, 1]);
        let c = poly(&[1, 1]);
        let repeated = &(&(&a * &a) * &(&b * &b)) * &(&(&x * &x) * &c);
        for p in [
            poly(&[]),
            poly(&[3]),
            poly(&[-5]),
            poly(&[0, 1]),
            poly(&[-2, 0, 1]),
            repeated,
            poly(&[1000001, -2000001, 1000000]),
        ] {
            for ring in [RingMode::Integer, RingMode::Rational] {
                let (all, _) = isolate(&p, Mode::Full, ring)?;
                let (positive, _) = isolate(&p, Mode::Positive, ring)?;
                compare(&p, &all, &positive, false)?;
                compare(&p, &all, &all, true)?;
            }
        }
        Ok(())
    }
    #[test]
    fn cross_squarefree_singleton_containment_stress() -> Result<()> {
        let irrational = poly(&[-2, 0, 1]);
        for singleton in [poly(&[-1, 1]), poly(&[1, 1])] {
            let p = &irrational * &(&singleton * &singleton);
            for ring in [RingMode::Integer, RingMode::Rational] {
                let (full, _) = isolate(&p, Mode::Full, ring)?;
                let (positive, _) = isolate(&p, Mode::Positive, ring)?;
                ensure!(
                    full.len() == 3,
                    "lost a root from distinct square-free factors"
                );
                compare(&p, &full, &positive, false)?;
            }
        }
        Ok(())
    }
    #[test]
    fn full_comparison_detects_negative_and_zero_omissions() -> Result<()> {
        let p = poly(&[0, -1, 0, 1]);
        let (full, _) = isolate(&p, Mode::Full, RingMode::Integer)?;
        let (positive, _) = isolate(&p, Mode::Positive, RingMode::Integer)?;
        ensure!(full.len() == 3 && positive.len() == 1);
        compare(&p, &full, &positive, false)?;
        ensure!(compare(&p, &full, &positive, true).is_err());
        Ok(())
    }
    #[test]
    fn overlapping_distinct_roots_are_not_equal() -> Result<()> {
        let p = Arc::new(poly(&[2, -3, 1]).map_coeff(|c| Rational::from(c.clone()), Q));
        let a = Root {
            lower: rational("1/2")?,
            upper: rational("3/2")?,
            multiplicity: Some(1),
        };
        let b = Root {
            lower: rational("7/5")?,
            upper: rational("5/2")?,
            multiplicity: Some(1),
        };
        ensure!(same_root(&a, &b, &p).is_err(), "overlap alone accepted");
        Ok(())
    }
    #[test]
    fn literal_input_rejects_shape_hash_and_expression_bombs() -> Result<()> {
        let input = Input {
            schema_version: 1,
            variables: vec!["s".into(), "t".into()],
            axis: 0,
            terms: vec![
                Term {
                    coefficient: "-2".into(),
                    exponents: vec![0, 0],
                },
                Term {
                    coefficient: "1".into(),
                    exponents: vec![2, 0],
                },
            ],
        };
        let bytes = serde_json::to_vec(&input)?;
        parse_input(&bytes, &hash(&bytes))?;
        ensure!(parse_input(&bytes, &"0".repeat(64)).is_err());
        for text in ["2^1000000000", "1/0", "1+1", ""] {
            let mut bad = input.clone();
            bad.terms[0].coefficient = text.into();
            let bytes = serde_json::to_vec(&bad)?;
            ensure!(parse_input(&bytes, &hash(&bytes)).is_err());
        }
        let mut bad = input;
        bad.terms[1].exponents[1] = 1;
        let bytes = serde_json::to_vec(&bad)?;
        ensure!(parse_input(&bytes, &hash(&bytes)).is_err());
        Ok(())
    }
}
