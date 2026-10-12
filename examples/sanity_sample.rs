//! Supplemental exact randomized sampling of exported cells; never a coverage proof.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits, Problem,
    algebra::{Algebra, Poly, rational},
    benchmark::run_bounded,
    output::{Cell, RootBound, RootIndexDomain, Status},
    roots::{FiberRoot, isolate_union, refine_once, roots_in_open_interval},
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    result: PathBuf,
    #[arg(long)]
    original: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 20261010)]
    seed: u64,
    #[arg(long, default_value_t = 2)]
    points_per_cell: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, default_value_t = 4096)]
    memory_mib: u64,
    #[arg(long, hide = true)]
    worker: bool,
}
#[derive(Deserialize)]
struct Export {
    schema_version: u32,
    status: Status,
    problem: Problem,
    order: Vec<String>,
    polynomials: Vec<String>,
    cells: Vec<Cell>,
    #[serde(default)]
    merged_cells: Option<Vec<Cell>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PointCheck {
    collection: String,
    cell: usize,
    point_index: usize,
    coordinates: BTreeMap<String, String>,
    interior_bounds: Vec<Value>,
    constraints: Vec<Value>,
    splits: Vec<Value>,
    passed: bool,
    failures: Vec<String>,
}
#[derive(Serialize)]
struct Report {
    schema_version: u32,
    kind: &'static str,
    coverage_proof: bool,
    status: String,
    seed: u64,
    rng: &'static str,
    points_per_cell: usize,
    raw_cells: usize,
    merged_cells: usize,
    cells_checked: usize,
    cells_passed: usize,
    cells_unchecked: usize,
    points_attempted: usize,
    points_checked: usize,
    requested_points: usize,
    original_evaluations: usize,
    constraint_checks: usize,
    split_evaluations: usize,
    failures: usize,
    untested_points: usize,
    original: PathBuf,
    original_sha256: String,
    result: PathBuf,
    result_sha256: String,
    executable_sha256: String,
    parameters: BTreeMap<String, String>,
    order: Vec<String>,
    elapsed_seconds: f64,
    diagnostic: Option<String>,
    checks: Vec<PointCheck>,
}
fn hash(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let temp = path.with_extension("tmp");
    let mut f = BufWriter::new(fs::File::create(&temp)?);
    serde_json::to_writer_pretty(&mut f, value)?;
    f.flush()?;
    fs::rename(temp, path)?;
    Ok(())
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn fraction(&mut self) -> Rational {
        Rational::from((self.next() % 65535 + 1) as i64) / Rational::from(65536)
    }
}
// This evaluator deliberately does not use normalization, fiber specialization,
// reconstructed split polynomials, chart formulas or solver sign evaluation.
fn evaluate(p: &Poly, values: &[Rational]) -> Result<Rational> {
    ensure!(
        p.nvars() == values.len(),
        "original evaluation variable map mismatch"
    );
    let mut total = Rational::zero();
    for (coefficient, exponents) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut term = coefficient.clone();
        for (x, &power) in values.iter().zip(exponents) {
            term *= x.pow(power as u64);
        }
        total += term;
    }
    Ok(total)
}
struct Constraint {
    left: Poly,
    right: Poly,
    greater: bool,
}
struct OriginalEvaluator {
    names: Vec<String>,
    constraints: Vec<Constraint>,
    splits: Vec<Poly>,
    parameters: BTreeMap<String, Rational>,
}
impl OriginalEvaluator {
    fn new(p: &Problem) -> Result<Self> {
        p.validate()?;
        let mut names = p.variables.clone();
        names.extend(p.parameters.keys().cloned());
        let a = Algebra::new(&names)?;
        let constraints = p
            .constraints
            .iter()
            .map(|text| -> Result<_> {
                let s = text.trim();
                if s == "true" || s == "false" {
                    return Ok(Constraint {
                        left: a.parse(if s == "true" { "1" } else { "0" })?,
                        right: a.parse("0")?,
                        greater: true,
                    });
                }
                ensure!(
                    !s.contains(['=', '!']),
                    "only original strict inequalities are supported"
                );
                let ops = s.match_indices(['<', '>']).collect::<Vec<_>>();
                ensure!(
                    ops.len() == 1,
                    "original constraint must have one strict comparison"
                );
                let (i, op) = ops[0];
                Ok(Constraint {
                    left: a.parse(&s[..i])?,
                    right: a.parse(&s[i + 1..])?,
                    greater: op == ">",
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let splits = p
            .split
            .iter()
            .map(|s| a.parse(s))
            .collect::<Result<Vec<_>>>()?;
        let parameters = p
            .parameters
            .iter()
            .map(|(k, v)| Ok((k.clone(), rational(v)?)))
            .collect::<Result<_>>()?;
        Ok(Self {
            names,
            constraints,
            splits,
            parameters,
        })
    }
    fn values(&self, point: &BTreeMap<String, String>) -> Result<Vec<Rational>> {
        self.names
            .iter()
            .map(|n| {
                self.parameters.get(n).cloned().map(Ok).unwrap_or_else(|| {
                    rational(
                        point
                            .get(n)
                            .context("missing sampled original coordinate")?,
                    )
                })
            })
            .collect()
    }
    fn check(&self, point: &mut PointCheck, expected: &[i8]) -> Result<usize> {
        ensure!(
            expected.len() == self.splits.len(),
            "exported split sign count differs from original"
        );
        let values = self.values(&point.coordinates)?;
        for (i, c) in self.constraints.iter().enumerate() {
            let left = evaluate(&c.left, &values)?;
            let right = evaluate(&c.right, &values)?;
            let passed = if c.greater {
                left > right
            } else {
                left < right
            };
            point.constraints.push(json!({"index":i,"left":left.to_string(),"right":right.to_string(),"relation":if c.greater {">"} else {"<"},"passed":passed}));
            if !passed {
                point
                    .failures
                    .push(format!("original strict constraint {i} failed"));
            }
        }
        for (i, p) in self.splits.iter().enumerate() {
            let value = evaluate(p, &values)?;
            let sign = if value.is_zero() {
                0
            } else if value > Rational::zero() {
                1
            } else {
                -1
            };
            let passed = sign == expected[i];
            point.splits.push(json!({"index":i,"value":value.to_string(),"sign":sign,"expected":expected[i],"passed":passed}));
            if !passed {
                point.failures.push(format!(
                    "original split polynomial {i} sign mismatch: expected {}, got {sign}",
                    expected[i]
                ));
            }
        }
        Ok(self.constraints.len() * 2 + self.splits.len())
    }
}
/// Keep exactly the strictly positive roots, refining until each isolating
/// interval is classified.
fn positive_roots(mut roots: Vec<FiberRoot>) -> Result<Vec<FiberRoot>> {
    for root in &mut roots {
        for _ in 0..512 {
            if root.interval.0 >= Rational::zero() || root.interval.1 <= Rational::zero() {
                break;
            }
            refine_once(root);
        }
        ensure!(
            root.interval.0 >= Rational::zero() || root.interval.1 <= Rational::zero(),
            "positive-root classification refinement budget exhausted"
        );
    }
    roots.retain(|r| r.interval.0 >= Rational::zero() && r.interval.1 > Rational::zero());
    Ok(roots)
}
/// A finite boundary root at a fresh prefix. Projective sections, which may
/// be +infinity, go through [`selected_section`].
fn selected_root(
    a: &Algebra,
    polys: &[Poly],
    bound: &RootBound,
    level: usize,
    prefix: &[Rational],
) -> Result<FiberRoot> {
    let p = polys
        .get(bound.polynomial)
        .context("boundary polynomial ID out of range")?;
    let fiber = a.specialize_univariate(p, level, prefix)?;
    ensure!(
        !fiber.is_zero() && fiber.degree() > 0,
        "boundary nullified or lost all roots at fresh prefix"
    );
    let mut roots = isolate_union(vec![(bound.polynomial, fiber)])?;
    match bound.index_domain {
        RootIndexDomain::Real => {}
        RootIndexDomain::Positive => roots = positive_roots(roots)?,
        RootIndexDomain::RealDescending => roots.reverse(),
        RootIndexDomain::PositiveOrInfinity => {
            anyhow::bail!("projective sections are selected by selected_section")
        }
    }
    roots
        .get(bound.index)
        .cloned()
        .context("declared boundary root does not exist at fresh prefix")
}
/// A boundary at a fresh prefix: Some(root), or None for +infinity, which only
/// an opt-in projective section (`PositiveOrInfinity`, the index-th strictly
/// positive root or +infinity where fewer exist) may select.
fn selected_section(
    a: &Algebra,
    polys: &[Poly],
    bound: &RootBound,
    level: usize,
    prefix: &[Rational],
) -> Result<Option<FiberRoot>> {
    if !bound.index_domain.is_projective() {
        return selected_root(a, polys, bound, level, prefix).map(Some);
    }
    let p = polys
        .get(bound.polynomial)
        .context("boundary polynomial ID out of range")?;
    let fiber = a.specialize_univariate(p, level, prefix)?;
    ensure!(
        !fiber.is_zero(),
        "projective boundary nullified at fresh prefix"
    );
    if fiber.degree() == 0 {
        return Ok(None);
    }
    let roots = positive_roots(isolate_union(vec![(bound.polynomial, fiber)])?)?;
    Ok(roots.get(bound.index).cloned())
}
fn sample_axis(
    lower: &mut Option<FiberRoot>,
    upper: &mut Option<FiberRoot>,
    rng: &mut Rng,
    forbidden: &[Rational],
) -> Result<Rational> {
    if let (Some(lo), Some(hi)) = (lower.as_mut(), upper.as_mut()) {
        for _ in 0..512 {
            if lo.interval.1 < hi.interval.0 {
                break;
            }
            if lo.interval.0 == lo.interval.1 && hi.interval.0 == hi.interval.1 {
                break;
            }
            refine_once(lo);
            refine_once(hi);
        }
        ensure!(
            lo.interval.1 < hi.interval.0,
            "fresh cell bounds have no certified strict gap"
        );
    }
    for _ in 0..128 {
        let u = rng.fraction();
        let x = match (lower.as_ref(), upper.as_ref()) {
            (Some(l), Some(h)) => &l.interval.1 + &((&h.interval.0 - &l.interval.1) * &u),
            (Some(l), None) => &l.interval.1 + &(&u / &(Rational::one() - &u)),
            (None, Some(h)) => &h.interval.0 - &(&u / &(Rational::one() - &u)),
            (None, None) => {
                let x = &u / &(Rational::one() - &u);
                if rng.next() & 1 == 0 { x } else { -x }
            }
        };
        if forbidden.contains(&x) {
            continue;
        }
        ensure!(
            lower.as_ref().is_none_or(|r| x > r.interval.1)
                && upper.as_ref().is_none_or(|r| x < r.interval.0),
            "rational sample not strictly interior"
        );
        return Ok(x);
    }
    anyhow::bail!("distinct sample generation budget exhausted")
}
// Separate candidate validation: recompute the fiber from the polynomial and
// use exact Sturm counts, rather than trusting the generator's intervals.
fn strict_membership(
    a: &Algebra,
    polys: &[Poly],
    bound: &RootBound,
    level: usize,
    prefix: &[Rational],
    x: &Rational,
    lower: bool,
) -> Result<()> {
    let p = polys
        .get(bound.polynomial)
        .context("membership polynomial missing")?;
    let fiber = a.specialize_univariate(p, level, prefix)?;
    // A projective section may lose every root (+infinity); it is still an
    // upper bound only.
    ensure!(
        !fiber.is_zero() && (fiber.degree() > 0 || bound.index_domain.is_projective()),
        "membership boundary degenerated"
    );
    ensure!(
        !(lower && bound.index_domain.is_projective()),
        "projective sections may only be upper bounds"
    );
    ensure!(
        !fiber.evaluate(x).is_zero(),
        "candidate is exactly on a boundary"
    );
    let lc = fiber.coefficients().last().unwrap().abs();
    let cauchy = fiber
        .coefficients()
        .iter()
        .take(fiber.degree())
        .map(|c| c.abs() / &lc)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let above = match bound.index_domain {
        RootIndexDomain::Real => {
            roots_in_open_interval(&fiber, &(-cauchy).min(x.clone() - Rational::one()), x)
                > bound.index
        }
        // A missing projective root is +infinity: never below x, which the
        // positive-root count already expresses.
        RootIndexDomain::Positive | RootIndexDomain::PositiveOrInfinity
            if x > &Rational::zero() =>
        {
            roots_in_open_interval(&fiber, &Rational::zero(), x) > bound.index
        }
        RootIndexDomain::Positive | RootIndexDomain::PositiveOrInfinity => false,
        RootIndexDomain::RealDescending => {
            roots_in_open_interval(&fiber, x, &cauchy.max(x.clone() + Rational::one()))
                <= bound.index
        }
    };
    ensure!(
        above == lower,
        "candidate fails independent strict root-count membership"
    );
    Ok(())
}
fn check_point(
    export: &Export,
    a: &Algebra,
    polys: &[Poly],
    original: &OriginalEvaluator,
    cell: &Cell,
    collection: &str,
    index: usize,
    point_index: usize,
    previous: &[PointCheck],
    rng: &mut Rng,
    started: Instant,
    seconds: f64,
) -> (PointCheck, usize) {
    let mut point = PointCheck {
        collection: collection.into(),
        cell: index,
        point_index,
        coordinates: BTreeMap::new(),
        interior_bounds: vec![],
        constraints: vec![],
        splits: vec![],
        passed: false,
        failures: vec![],
    };
    let mut evaluations = 0;
    let result = (|| -> Result<()> {
        ensure!(
            cell.axes.len() == export.order.len(),
            "exported cell omits an axis"
        );
        let mut prefix = Vec::new();
        for (level, axis) in cell.axes.iter().enumerate() {
            ensure!(
                started.elapsed().as_secs_f64() < seconds,
                "sampling deadline exhausted"
            );
            ensure!(
                axis.variable == export.order[level],
                "cell axis order differs from exported order"
            );
            ensure!(
                axis.lower
                    .as_ref()
                    .is_none_or(|b| !b.index_domain.is_projective()),
                "projective sections may only be upper bounds"
            );
            let mut lower = axis
                .lower
                .as_ref()
                .map(|b| selected_root(a, polys, b, level, &prefix))
                .transpose()?;
            // An opt-in projective upper section may be +infinity (None) at
            // this fresh prefix; the axis is then sampled as unbounded above.
            let mut upper = axis
                .upper
                .as_ref()
                .map(|b| selected_section(a, polys, b, level, &prefix))
                .transpose()?
                .flatten();
            let mut forbidden = vec![rational(&axis.sample)?];
            for prior in previous {
                if let Some(x) = prior.coordinates.get(&axis.variable) {
                    forbidden.push(rational(x)?);
                }
            }
            let x = sample_axis(&mut lower, &mut upper, rng, &forbidden)?;
            let record = |bound: &Option<RootBound>, root: &Option<FiberRoot>| -> Value {
                match (bound, root) {
                    (Some(b), Some(r)) => {
                        json!({"polynomial":b.polynomial,"index":b.index,"index_domain":b.index_domain,"fresh_isolation":[r.interval.0.to_string(),r.interval.1.to_string()]})
                    }
                    (Some(b), None) => {
                        json!({"polynomial":b.polynomial,"index":b.index,"index_domain":b.index_domain,"fresh_isolation":"+inf"})
                    }
                    _ => Value::Null,
                }
            };
            point.interior_bounds.push(json!({"variable":axis.variable,"sample":x.to_string(),"lower":record(&axis.lower,&lower),"upper":record(&axis.upper,&upper),"strict_interior":false,"independent_sturm_membership":false}));
            point
                .coordinates
                .insert(axis.variable.clone(), x.to_string());
            for (bound, is_lower) in [(&axis.lower, true), (&axis.upper, false)] {
                if let Some(bound) = bound {
                    strict_membership(a, polys, bound, level, &prefix, &x, is_lower)?;
                }
            }
            point.interior_bounds.last_mut().unwrap()["strict_interior"] = json!(true);
            point.interior_bounds.last_mut().unwrap()["independent_sturm_membership"] = json!(true);
            prefix.push(x);
        }
        evaluations = original.check(&mut point, &cell.signs)?;
        Ok(())
    })();
    if let Err(e) = result {
        point.failures.push(format!("{e:#}"));
    }
    point.passed = point.failures.is_empty();
    (point, evaluations)
}
fn validate_original(e: &Export, p: &Problem) -> Result<()> {
    ensure!(
        !e.order.is_empty(),
        "zero-dimensional export cannot supply distinct fresh points"
    );
    ensure!(
        e.schema_version == 1 && e.status == Status::CompleteGeneric,
        "sampling requires a complete version-1 export"
    );
    ensure!(
        e.problem.name == p.name
            && e.problem.variables == p.variables
            && e.problem.constraints == p.constraints
            && e.problem.split == p.split
            && e.problem.parameters == p.parameters
            && e.problem.variable_groups == p.variable_groups,
        "export is not the supplied original mathematical input"
    );
    let mut ordered = p.clone();
    ordered.solver.order = e.order.clone();
    ordered.validate()?;
    ensure!(
        e.order.len() == p.variables.len()
            && e.order.iter().collect::<BTreeSet<_>>()
                == p.variables.iter().collect::<BTreeSet<_>>(),
        "invalid exported variable order"
    );
    ensure!(
        p.solver.order.is_empty() || p.solver.order == e.order,
        "export changes the original fixed order"
    );
    Ok(())
}
fn worker(args: &Args) -> Result<()> {
    let started = Instant::now();
    let original_path = args.original.canonicalize()?;
    let result_path = args.result.canonicalize()?;
    let original_hash = hash(&original_path)?;
    let result_hash = hash(&result_path)?;
    let original = Problem::from_toml(&fs::read_to_string(&original_path)?)?;
    let export: Export = serde_json::from_reader(BufReader::new(fs::File::open(&result_path)?))?;
    validate_original(&export, &original)?;
    let a = Algebra::new(&export.order)?;
    let polys = export
        .polynomials
        .iter()
        .map(|p| a.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let evaluator = OriginalEvaluator::new(&original)?;
    let merged = export.merged_cells.as_deref().unwrap_or(&[]);
    let total = export
        .cells
        .len()
        .checked_add(merged.len())
        .context("cell count overflow")?;
    let requested = total
        .checked_mul(args.points_per_cell)
        .context("point count overflow")?;
    let mut report = Report {
        schema_version: 1,
        kind: "supplemental_exact_random_sampling",
        coverage_proof: false,
        status: "incomplete".into(),
        seed: args.seed,
        rng: "splitmix64-v1; 16-bit open rational fractions",
        points_per_cell: args.points_per_cell,
        raw_cells: export.cells.len(),
        merged_cells: merged.len(),
        cells_checked: 0,
        cells_passed: 0,
        cells_unchecked: total,
        points_attempted: 0,
        points_checked: 0,
        requested_points: requested,
        original_evaluations: 0,
        constraint_checks: 0,
        split_evaluations: 0,
        failures: 0,
        untested_points: requested,
        original: original_path,
        original_sha256: original_hash,
        result: result_path,
        result_sha256: result_hash,
        executable_sha256: hash(&std::env::current_exe()?)?,
        parameters: original.parameters.clone(),
        order: export.order.clone(),
        elapsed_seconds: started.elapsed().as_secs_f64(),
        diagnostic: None,
        checks: vec![],
    };
    save(&args.output, &report)?;
    let mut rng = Rng(args.seed);
    let fully_checked = |point: &PointCheck| {
        point.coordinates.len() == export.order.len()
            && point.interior_bounds.len() == export.order.len()
            && point
                .interior_bounds
                .iter()
                .all(|b| b["independent_sturm_membership"] == true)
            && point.constraints.len() == evaluator.constraints.len()
            && point.splits.len() == evaluator.splits.len()
    };
    'collections: for (collection, cells) in [("raw", export.cells.as_slice()), ("merged", merged)]
    {
        for (index, cell) in cells.iter().enumerate() {
            let mut previous = Vec::new();
            for i in 0..args.points_per_cell {
                if started.elapsed().as_secs_f64() >= args.seconds {
                    report.diagnostic = Some(
                        "sampling deadline exhausted; untested points do not count as passed"
                            .into(),
                    );
                    break 'collections;
                }
                let (point, evaluations) = check_point(
                    &export,
                    &a,
                    &polys,
                    &evaluator,
                    cell,
                    collection,
                    index,
                    i,
                    &previous,
                    &mut rng,
                    started,
                    args.seconds,
                );
                report.original_evaluations += evaluations;
                report.points_attempted += 1;
                if fully_checked(&point) {
                    report.points_checked += 1;
                }
                report.constraint_checks += point.constraints.len();
                report.split_evaluations += point.splits.len();
                report.untested_points -= 1;
                report.failures += usize::from(!point.passed);
                previous.push(point.clone());
                report.checks.push(point);
                report.elapsed_seconds = started.elapsed().as_secs_f64();
                save(&args.output, &report)?;
            }
            if previous.len() == args.points_per_cell && previous.iter().all(fully_checked) {
                report.cells_checked += 1;
                report.cells_unchecked -= 1;
            }
            if previous.len() == args.points_per_cell && previous.iter().all(|p| p.passed) {
                report.cells_passed += 1;
            }
            report.elapsed_seconds = started.elapsed().as_secs_f64();
            save(&args.output, &report)?;
        }
    }
    ensure!(
        hash(&args.original)? == report.original_sha256
            && hash(&args.result)? == report.result_sha256,
        "source input changed during sampling"
    );
    report.status =
        if report.untested_points > 0 || started.elapsed().as_secs_f64() >= args.seconds {
            "incomplete"
        } else if report.failures > 0 {
            "failed"
        } else {
            "passed"
        }
        .into();
    report.elapsed_seconds = started.elapsed().as_secs_f64();
    save(&args.output, &report)?;
    println!(
        "{}",
        json!({"status":report.status,"cells_checked":report.cells_checked,"points_checked":report.points_checked,"evaluations":report.original_evaluations,"failures":report.failures,"untested_points":report.untested_points})
    );
    ensure!(
        report.status == "passed",
        "supplemental sampling did not pass every exported cell"
    );
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.points_per_cell >= 2 && args.points_per_cell <= 16,
        "points_per_cell must be2..16"
    );
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.memory_mib > 0,
        "invalid resource bounds"
    );
    if args.worker {
        return worker(&args);
    }
    ensure!(
        !args.output.exists(),
        "refusing stale output; choose a fresh report path"
    );
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--worker")
        .arg("--result")
        .arg(&args.result)
        .arg("--original")
        .arg(&args.original)
        .arg("--output")
        .arg(&args.output)
        .arg("--seed")
        .arg(args.seed.to_string())
        .arg("--points-per-cell")
        .arg(args.points_per_cell.to_string())
        .arg("--seconds")
        .arg(args.seconds.to_string())
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: args.seconds,
            memory_mib: args.memory_mib,
            workers: 1,
            max_cells: None,
        },
    )?;
    save(&args.output.with_extension("process.json"), &process)?;
    ensure!(
        process.status == "completed" && process.exit_code == Some(0),
        "sampling worker {} (see process report and any partial checks)",
        process.status
    );
    let report: Value = serde_json::from_reader(BufReader::new(fs::File::open(&args.output)?))?;
    ensure!(
        report["status"] == "passed" && report["failures"] == 0 && report["untested_points"] == 0,
        "not every requested point passed"
    );
    println!("{}", process.stdout.trim());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::output::Axis;
    fn fixture() -> Result<(Export, Problem)> {
        let p = Problem::from_toml(
            "name='toy'\nvariables=['x','y']\nconstraints=['x>0','y>0']\nsplit=['m*y-x^2']\n[parameters]\nm='2'",
        )?;
        let bound = |polynomial, index| RootBound {
            polynomial,
            index,
            index_domain: RootIndexDomain::Real,
            lower: "999".into(),
            upper: "1000".into(),
        };
        let cell = Cell {
            axes: vec![
                Axis {
                    variable: "x".into(),
                    sample: "1".into(),
                    lower: Some(bound(0, 0)),
                    upper: None,
                },
                Axis {
                    variable: "y".into(),
                    sample: "1".into(),
                    lower: Some(bound(1, 0)),
                    upper: None,
                },
            ],
            signs: vec![1],
        };
        Ok((
            Export {
                schema_version: 1,
                status: Status::CompleteGeneric,
                problem: p.clone(),
                order: p.variables.clone(),
                polynomials: vec!["x".into(), "2*y-x^2".into()],
                cells: vec![cell],
                merged_cells: None,
            },
            p,
        ))
    }
    fn point(e: &Export, p: &Problem, seed: u64) -> Result<PointCheck> {
        let a = Algebra::new(&e.order)?;
        let polys = e
            .polynomials
            .iter()
            .map(|p| a.parse(p))
            .collect::<Result<Vec<_>>>()?;
        Ok(check_point(
            e,
            &a,
            &polys,
            &OriginalEvaluator::new(p)?,
            &e.cells[0],
            "raw",
            0,
            0,
            &[],
            &mut Rng(seed),
            Instant::now(),
            10.,
        )
        .0)
    }
    #[test]
    fn constant_zero_split_after_parameter_substitution_matches_zero_annotation() -> Result<()> {
        let p = Problem::from_toml(
            "variables=['x']\nconstraints=['x>0']\nsplit=['m*x']\n[parameters]\nm='0'\n[solver]\nengine='local'\npositivity=true",
        )?;
        let result = symgcad::solver::solve(&p)?;
        assert_eq!(result.status, Status::CompleteGeneric);
        assert_eq!(result.cells.len(), 1);
        assert_eq!(result.cells[0].signs, vec![0]);
        let mut export: Export = serde_json::from_value(serde_json::to_value(result)?)?;
        validate_original(&export, &p)?;
        let sampled = point(&export, &p, 42)?;
        assert!(sampled.passed, "{:?}", sampled.failures);
        assert_eq!(sampled.splits[0]["value"], "0");
        assert_eq!(sampled.splits[0]["sign"], 0);
        assert_eq!(sampled.splits[0]["expected"], 0);
        export.cells[0].signs[0] = 1;
        assert!(!point(&export, &p, 42)?.passed);
        Ok(())
    }
    #[test]
    fn fresh_prefix_roots_ignore_stored_intervals_and_original_evaluation_is_exact() -> Result<()> {
        let (e, p) = fixture()?;
        let q = point(&e, &p, 42)?;
        assert!(q.passed, "{:?}", q.failures);
        assert_ne!(q.coordinates["x"], "1");
        assert_ne!(q.coordinates["y"], "1");
        let x = rational(&q.coordinates["x"])?;
        let y = rational(&q.coordinates["y"])?;
        assert_eq!(
            rational(q.splits[0]["value"].as_str().unwrap())?,
            Rational::from(2) * y - x.pow(2)
        );
        assert_eq!(
            serde_json::to_value(&q)?,
            serde_json::to_value(point(&e, &p, 42)?)?
        );
        assert_ne!(q.coordinates, point(&e, &p, 43)?.coordinates);
        Ok(())
    }
    #[test]
    fn wrong_original_sign_or_domain_is_an_explicit_counterexample() -> Result<()> {
        let (mut e, p) = fixture()?;
        e.cells[0].signs[0] = -1;
        let q = point(&e, &p, 42)?;
        assert!(!q.passed);
        assert!(!q.coordinates.is_empty());
        assert!(q.failures.iter().any(|x| x.contains("sign mismatch")));
        e.cells[0].signs[0] = 1;
        let mut other = p.clone();
        other.constraints.push("x<0".into());
        assert!(!point(&e, &other, 42)?.passed);
        assert!(validate_original(&e, &other).is_err());
        Ok(())
    }
    #[test]
    fn positive_and_descending_indices_count_distinct_roots_with_multiplicity() -> Result<()> {
        let a = Algebra::new(&["z".into()])?;
        let polys = vec![a.parse("(z+3)^2*z^2*(z-2)^2*(z-5)")?];
        let b = |d, i| RootBound {
            polynomial: 0,
            index: i,
            index_domain: d,
            lower: "0".into(),
            upper: "0".into(),
        };
        let r = selected_root(&a, &polys, &b(RootIndexDomain::Positive, 0), 0, &[])?;
        assert!(r.interval.0 <= Rational::from(2) && r.interval.1 >= Rational::from(2));
        let r = selected_root(&a, &polys, &b(RootIndexDomain::RealDescending, 0), 0, &[])?;
        assert!(r.interval.0 <= Rational::from(5) && r.interval.1 >= Rational::from(5));
        assert!(selected_root(&a, &polys, &b(RootIndexDomain::Positive, 2), 0, &[]).is_err());
        Ok(())
    }
    #[test]
    fn projective_sections_select_positive_roots_or_infinity() -> Result<()> {
        let a = Algebra::new(&["t".into(), "z".into()])?;
        let polys = vec![a.parse("t*z+1")?, a.parse("(z-1)*(z-3)*(z+2)")?];
        let b = |polynomial, index| RootBound {
            polynomial,
            index,
            index_domain: RootIndexDomain::PositiveOrInfinity,
            lower: "+inf".into(),
            upper: "+inf".into(),
        };
        // t<0: the root -1/t; t>0: no positive root; t=0: constant fiber.
        let r = selected_section(&a, &polys, &b(0, 0), 1, &[Rational::from(-2)])?.unwrap();
        assert!(r.interval.0 <= Rational::from(1) / Rational::from(2));
        assert!(r.interval.1 >= Rational::from(1) / Rational::from(2));
        for t in [0, 2] {
            assert!(selected_section(&a, &polys, &b(0, 0), 1, &[Rational::from(t)])?.is_none());
        }
        // Indices count distinct strictly positive roots; beyond them +inf.
        let prefix = [Rational::one()];
        let second = selected_section(&a, &polys, &b(1, 1), 1, &prefix)?.unwrap();
        assert!(second.interval.0 <= Rational::from(3) && second.interval.1 >= Rational::from(3));
        assert!(selected_section(&a, &polys, &b(1, 2), 1, &prefix)?.is_none());
        // Finite selectors refuse to be read as projective and vice versa.
        assert!(selected_root(&a, &polys, &b(0, 0), 1, &[Rational::from(-2)]).is_err());
        // Membership below +infinity, below a finite section, never as lower.
        let two = Rational::from(2);
        assert!(
            strict_membership(
                &a,
                &polys,
                &b(0, 0),
                1,
                &[two.clone()],
                &Rational::from(9),
                false
            )
            .is_ok()
        );
        assert!(
            strict_membership(
                &a,
                &polys,
                &b(0, 0),
                1,
                &[Rational::zero()],
                &Rational::from(9),
                false
            )
            .is_ok()
        );
        assert!(
            strict_membership(
                &a,
                &polys,
                &b(0, 0),
                1,
                &[-two.clone()],
                &Rational::from(9),
                false
            )
            .is_err()
        );
        assert!(
            strict_membership(&a, &polys, &b(0, 0), 1, &[two], &Rational::from(9), true).is_err()
        );
        Ok(())
    }
    #[test]
    fn projective_bilinear_cells_sample_with_infinite_and_finite_sections() -> Result<()> {
        // The Gluza-shaped synthetic corner chain of tests/bilinear_general.rs.
        let target = "(u-3)*x*y+(u-3+(v-1)^2)*x+(u-3+(u-v)^2)*y+(u-3+(v-1)^2+(u-v)^2)";
        let p = Problem::from_toml(&format!(
            "name='projective chain'\nvariables=['u','v','y','x']\nconstraints=['u>0','v>0','y>0','x>0','({target})>0']\nsplit=[]\n[solver]\nengine='bilinear_general'\nordering='given'\nbilinear_general_guide='(x+1)*(y+1)'\nprojective_sections=true\n[limits]\nwall_time_secs=60.0"
        ))?;
        let result = symgcad::solver::solve(&p)?;
        assert_eq!(result.status, Status::CompleteGeneric);
        assert!(symgcad::verify::verify(&result)?.verified);
        assert!(result.bilinear_general.as_ref().unwrap().plan.projective);
        let export: Export = serde_json::from_value(serde_json::to_value(&result)?)?;
        validate_original(&export, &p)?;
        let a = Algebra::new(&export.order)?;
        let polys = export
            .polynomials
            .iter()
            .map(|p| a.parse(p))
            .collect::<Result<Vec<_>>>()?;
        let eval = OriginalEvaluator::new(&p)?;
        let (mut infinite, mut finite) = (0, 0);
        for (index, cell) in export.cells.iter().enumerate() {
            let mut rng = Rng(20261010 + index as u64);
            let mut previous: Vec<PointCheck> = vec![];
            for i in 0..2 {
                let q = check_point(
                    &export,
                    &a,
                    &polys,
                    &eval,
                    cell,
                    "raw",
                    index,
                    i,
                    &previous,
                    &mut rng,
                    Instant::now(),
                    30.,
                )
                .0;
                assert!(q.passed, "cell {index}: {:?}", q.failures);
                for axis in &q.interior_bounds[2..] {
                    if axis["upper"]["fresh_isolation"] == "+inf" {
                        infinite += 1;
                    } else {
                        finite += 1;
                    }
                }
                previous.push(q);
            }
        }
        assert!(infinite > 0 && finite > 0, "{infinite} {finite}");
        // A projective lower bound is never accepted.
        let mut forged = export.cells[0].clone();
        forged.axes[3].lower = forged.axes[3].upper.clone();
        let q = check_point(
            &export,
            &a,
            &polys,
            &eval,
            &forged,
            "raw",
            0,
            0,
            &[],
            &mut Rng(1),
            Instant::now(),
            30.,
        )
        .0;
        assert!(!q.passed);
        Ok(())
    }
    #[test]
    fn independent_membership_rejects_outside_and_exact_boundary_points() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let polys = vec![a.parse("y^2-x")?];
        let b = RootBound {
            polynomial: 0,
            index: 0,
            index_domain: RootIndexDomain::Positive,
            lower: "999".into(),
            upper: "1000".into(),
        };
        let prefix = [Rational::from(2)];
        assert!(strict_membership(&a, &polys, &b, 1, &prefix, &Rational::from(2), true).is_ok());
        assert!(strict_membership(&a, &polys, &b, 1, &prefix, &Rational::one(), true).is_err());
        assert!(
            strict_membership(
                &a,
                &polys,
                &b,
                1,
                &[Rational::from(4)],
                &Rational::from(2),
                true
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn invalid_or_collapsed_bounds_and_deadlines_never_pass() -> Result<()> {
        let (mut e, p) = fixture()?;
        e.cells[0].axes[0].upper = e.cells[0].axes[0].lower.clone();
        assert!(!point(&e, &p, 7)?.passed);
        e.cells[0].axes[0].upper = None;
        e.cells[0].axes[1].lower.as_mut().unwrap().index = 12;
        assert!(!point(&e, &p, 7)?.passed);
        let a = Algebra::new(&e.order)?;
        let polys = e
            .polynomials
            .iter()
            .map(|p| a.parse(p))
            .collect::<Result<Vec<_>>>()?;
        let q = check_point(
            &e,
            &a,
            &polys,
            &OriginalEvaluator::new(&p)?,
            &e.cells[0],
            "raw",
            0,
            0,
            &[],
            &mut Rng(1),
            Instant::now(),
            0.,
        )
        .0;
        assert!(!q.passed);
        assert!(q.coordinates.is_empty());
        Ok(())
    }
    #[test]
    fn second_point_differs_from_first_and_original_parameter_tokens_are_separate() -> Result<()> {
        let (e, p) = fixture()?;
        let a = Algebra::new(&e.order)?;
        let polys = e
            .polynomials
            .iter()
            .map(|p| a.parse(p))
            .collect::<Result<Vec<_>>>()?;
        let eval = OriginalEvaluator::new(&p)?;
        let mut rng = Rng(123);
        let first = check_point(
            &e,
            &a,
            &polys,
            &eval,
            &e.cells[0],
            "raw",
            0,
            0,
            &[],
            &mut rng,
            Instant::now(),
            10.,
        )
        .0;
        let second = check_point(
            &e,
            &a,
            &polys,
            &eval,
            &e.cells[0],
            "raw",
            0,
            1,
            std::slice::from_ref(&first),
            &mut rng,
            Instant::now(),
            10.,
        )
        .0;
        assert!(first.passed && second.passed);
        assert_ne!(first.coordinates, second.coordinates);
        let p = Problem::from_toml(
            "variables=['x','x1']\nconstraints=['x1>x']\nsplit=['m*x1-x']\n[parameters]\nm='3/2'",
        )?;
        let eval = OriginalEvaluator::new(&p)?;
        let values = eval.values(&BTreeMap::from([
            ("x".into(), "2".into()),
            ("x1".into(), "4".into()),
        ]))?;
        assert_eq!(evaluate(&eval.splits[0], &values)?, Rational::from(4));
        Ok(())
    }
}
