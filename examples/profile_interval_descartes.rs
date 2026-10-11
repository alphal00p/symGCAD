//! Research only: bounded rational interval Descartes guard discovery/costs.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Algebra, Poly, canonical, factors_checked, rational},
    benchmark::run_bounded,
    optimized_projection::OperationCapture,
    roots::{isolate_union, refine_once, sector_sample},
};

const DEFAULT_MAX_TERMS: usize = 4096;
const RESEARCH_MAX_TERMS: usize = 65_536;
const DEFAULT_MAX_TOTAL_GUARD_TERMS: usize = 65_536;
const MAX_TOTAL_GUARD_TERMS: usize = 262_144;

fn validate_max_terms(max_terms: usize) -> Result<()> {
    ensure!(
        (1..=RESEARCH_MAX_TERMS).contains(&max_terms),
        "research max-terms must be between 1 and 65536"
    );
    Ok(())
}
fn validate_total_guard_terms(limit: usize) -> Result<()> {
    ensure!(
        (1..=MAX_TOTAL_GUARD_TERMS).contains(&limit),
        "max-total-guard-terms must be between 1 and 262144"
    );
    Ok(())
}

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    prefix: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    factor_guards: bool,
    /// Research-only finite rational-cover search; production budgets stay fixed.
    #[arg(long)]
    compact_cover_search: bool,
    /// Research source admission only; production guard limits remain separate.
    #[arg(long, default_value_t = DEFAULT_MAX_TERMS)]
    max_terms: usize,
    /// Separate explicit guard resource policy; historical/default eligibility
    /// remains in the report even when the selected budget admits factoring.
    #[arg(long, default_value_t = DEFAULT_MAX_TOTAL_GUARD_TERMS)]
    max_total_guard_terms: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 0)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}
fn sign(x: &Rational) -> i8 {
    if x.is_zero() {
        0
    } else if x.is_negative() {
        -1
    } else {
        1
    }
}
fn variations(values: &[Rational]) -> usize {
    values
        .iter()
        .map(sign)
        .filter(|&s| s != 0)
        .fold((0, 0), |(last, n), s| {
            (s, n + usize::from(last != 0 && last != s))
        })
        .1
}
fn choose(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let mut c = 1;
    for i in 1..=k.min(n - k) {
        c = c * (n + 1 - i) as u64 / i as u64;
    }
    c
}
fn powers(x: &Rational, d: usize) -> Vec<Rational> {
    let mut p = vec![Rational::one()];
    for i in 1..=d {
        p.push(&p[i - 1] * x);
    }
    p
}
/// Rows are transformed coefficients, columns are original coefficients.
/// Finite: (1+u)^d p((a+b*u)/(1+u)); tail: p(a+u).
fn weights(d: usize, a: &Rational, b: Option<&Rational>) -> Vec<Vec<Rational>> {
    let ap = powers(a, d);
    let bp = b.map(|b| powers(b, d));
    let mut w = vec![vec![Rational::zero(); d + 1]; d + 1];
    for (k, row) in w.iter_mut().enumerate() {
        for (j, entry) in row.iter_mut().enumerate() {
            if let Some(bp) = &bp {
                for l in k.saturating_sub(d - j)..=j.min(k) {
                    *entry +=
                        (&ap[j - l] * &bp[l]) * Rational::from(choose(j, l) * choose(d - j, k - l));
                }
            } else if j >= k {
                *entry = &ap[j - k] * &Rational::from(choose(j, k));
            }
        }
    }
    w
}
fn transformed(f: &UnivariatePolynomial<Q>, w: &[Vec<Rational>]) -> Vec<Rational> {
    w.iter()
        .map(|row| {
            row.iter()
                .zip(f.coefficients())
                .fold(Rational::zero(), |a, (w, c)| a + w * c)
        })
        .collect()
}
fn height(x: &Rational) -> u64 {
    x.numerator_ref().significant_bits() + x.denominator_ref().significant_bits()
}

#[derive(Clone)]
struct Interval {
    a: Rational,
    b: Option<Rational>,
    depth: usize,
}
struct Probe {
    report: Value,
    guards: Vec<Poly>,
}
fn full_cover(intervals: &[Interval]) -> bool {
    !intervals.is_empty()
        && intervals[0].a.is_zero()
        && intervals.last().unwrap().b.is_none()
        && intervals.iter().enumerate().all(|(i, interval)| {
            interval.a >= Rational::zero()
                && match &interval.b {
                    Some(b) => {
                        *b > interval.a && intervals.get(i + 1).is_some_and(|next| next.a == *b)
                    }
                    None => i + 1 == intervals.len(),
                }
        })
}

fn register_guard(p: &Poly, guards: &mut Vec<Poly>, registry: &mut HashMap<Poly, usize>) -> usize {
    let p = canonical(p);
    if let Some(&index) = registry.get(&p) {
        index
    } else {
        let index = guards.len();
        registry.insert(p.clone(), index);
        guards.push(p);
        index
    }
}

fn probe(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    max_terms: usize,
) -> Result<Probe> {
    validate_max_terms(max_terms)?;
    ensure!(
        variable < algebra.names.len()
            && prefix.len() == variable
            && p.variables() == &algebra.variables,
        "source axis, prefix, or full variable map mismatch"
    );
    ensure!(
        p.degree(variable) <= 32 && p.nterms() <= max_terms,
        "research degree/term cap"
    );
    let started = Instant::now();
    let f = algebra.specialize_univariate(p, variable, prefix)?;
    ensure!(
        !f.is_zero()
            && f.degree() == p.degree(variable) as usize
            && !f.evaluate(&Rational::zero()).is_zero(),
        "full degree/nonzero endpoint required"
    );
    let d = f.degree();
    let mut roots = isolate_union(vec![(0, f.clone())])?;
    let mut zero_refinements = 0;
    for root in &mut roots {
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            ensure!(zero_refinements < 256, "zero refinement budget");
            refine_once(root);
            zero_refinements += 1;
        }
    }
    roots.retain(|r| r.interval.0 > Rational::zero());
    let isolation_seconds = started.elapsed().as_secs_f64();
    let mut boundaries = vec![Rational::zero()];
    boundaries.extend(
        roots
            .windows(2)
            .map(|pair| sector_sample(Some(&pair[0]), Some(&pair[1]))),
    );
    let mut pending = VecDeque::new();
    for (i, a) in boundaries.iter().enumerate() {
        pending.push_back(Interval {
            a: a.clone(),
            b: boundaries.get(i + 1).cloned(),
            depth: 0,
        });
    }
    let mut accepted = Vec::new();
    let mut attempted = 0;
    let mut decline = None;
    while let Some(interval) = pending.pop_front() {
        attempted += 1;
        let w = weights(d, &interval.a, interval.b.as_ref());
        let values = transformed(&f, &w);
        let v = variations(&values);
        if v <= 1 {
            accepted.push((interval, w, values, v));
            continue;
        }
        if accepted.len() + pending.len() + 2 > 64 || interval.depth >= 32 {
            decline = Some("interval/depth budget");
            break;
        }
        let mut cut = if let Some(b) = &interval.b {
            (&interval.a + b) / Rational::from(2)
        } else {
            let after_roots = roots
                .last()
                .map(|r| sector_sample(Some(r), None))
                .unwrap_or_else(Rational::one);
            if after_roots > interval.a {
                after_roots
            } else {
                &interval.a * &Rational::from(2) + Rational::one()
            }
        };
        // Never put a rational root at an unguarded interval endpoint.
        let mut shifts = 0;
        while f.evaluate(&cut).is_zero() && shifts < 32 {
            cut = (&interval.a + &cut) / Rational::from(2);
            shifts += 1;
        }
        if height(&cut) > 256 || f.evaluate(&cut).is_zero() {
            decline = Some("separator height/root budget");
            break;
        }
        pending.push_back(Interval {
            a: interval.a.clone(),
            b: Some(cut.clone()),
            depth: interval.depth + 1,
        });
        pending.push_back(Interval {
            a: cut,
            b: interval.b,
            depth: interval.depth + 1,
        });
    }
    accepted.sort_by(|a, b| a.0.a.cmp(&b.0.a));
    let count = accepted.iter().map(|x| x.3).sum::<usize>();
    let report = json!({"eligible":false,"degree":d,"terms":p.nterms(),"research_max_terms":max_terms,"production_source_term_limit":DEFAULT_MAX_TERMS,"global_variations":variations(f.coefficients()),"positive_distinct_roots":roots.len(),"isolation_seconds":isolation_seconds,"numeric_discovery_seconds":started.elapsed().as_secs_f64(),"attempted_intervals":attempted,"accepted_intervals":accepted.len(),"sum_interval_variations":count,"declined":decline});
    if decline.is_some() || count != roots.len() {
        return Ok(Probe {
            report,
            guards: vec![],
        });
    }
    construct_cover(algebra, p, variable, prefix, roots.len(), report, accepted)
}

type NumericInterval = (Interval, Vec<Vec<Rational>>, Vec<Rational>, usize);

fn construct_cover(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    positive_roots: usize,
    mut report: Value,
    accepted: Vec<NumericInterval>,
) -> Result<Probe> {
    let d = p.degree(variable) as usize;
    let construction = Instant::now();
    let cover = accepted.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
    ensure!(
        full_cover(&cover),
        "discovered intervals do not cover the full positive ray"
    );
    report["full_positive_cover"] = json!(true);
    let zero = algebra.parse("0")?;
    let mut coefficients = vec![zero.clone(); d + 1];
    for (c, e) in p.to_univariate_polynomial_list(variable) {
        coefficients[e as usize] = c;
    }
    let mut guards = Vec::new();
    let mut registry = HashMap::new();
    let mut intervals = Vec::new();
    let mut zero_obstructions = 0;
    // Explicit degree, coordinate endpoint and seam obligations, matching
    // production replay even when a transformed coefficient is structural zero.
    let endpoint = p.replace(variable, &Rational::zero());
    let leading = p.univariate_lcoeff(variable);
    let mut mandatory = vec![("endpoint", endpoint), ("leading", leading)];
    for interval in &cover {
        if let Some(b) = &interval.b {
            mandatory.push(("seam", p.replace(variable, b)));
        }
    }
    let mut mandatory_indices = Vec::new();
    for (kind, guard) in mandatory {
        if guard.replace_all(prefix).is_zero() {
            zero_obstructions += 1;
        }
        mandatory_indices.push(
            json!({"kind":kind,"guard_index":register_guard(&guard, &mut guards, &mut registry)}),
        );
    }
    for (interval, w, values, v) in accepted {
        let mut indices = Vec::new();
        let mut signs = Vec::new();
        for (k, row) in w.iter().enumerate() {
            let q = row
                .iter()
                .zip(&coefficients)
                .filter(|(weight, _)| !weight.is_zero())
                .fold(zero.clone(), |acc, (weight, c)| {
                    acc + c.clone().mul_coeff(weight.clone())
                });
            ensure!(
                q.replace_all(prefix) == values[k],
                "symbolic interval coefficient differs from numeric transform"
            );
            if q.is_zero() {
                indices.push(None);
                signs.push(0);
                continue;
            }
            if values[k].is_zero() {
                zero_obstructions += 1;
            }
            signs.push(sign(&values[k]));
            let index = register_guard(&q, &mut guards, &mut registry);
            indices.push(Some(index));
        }
        intervals.push(json!({"lower":interval.a.to_string(),"upper":interval.b.map(|x|x.to_string()),"variations":v,"coefficient_signs":signs,"guard_indices":indices,"subdivision_depth":interval.depth}));
    }
    report["intervals"] = json!(intervals);
    report["mandatory_guard_indices"] = json!(mandatory_indices);
    report["symbolic_guard_seconds"] = json!(construction.elapsed().as_secs_f64());
    report["sample_zero_guard_obstructions"] = json!(zero_obstructions);
    report["unique_guards"] = json!(guards.len());
    report["guards"]=json!(guards.iter().enumerate().map(|(index,g)|json!({"index":index,"terms":g.nterms(),"maximum_coefficient_bits":g.coefficients.iter().map(height).max().unwrap_or(0),"degrees":(0..g.nvars()).map(|i|g.degree(i)).collect::<Vec<_>>(),"expression":algebra.format(g)})).collect::<Vec<_>>());
    report["eligible"] = json!(zero_obstructions == 0);
    let coefficient_slots = cover.len().saturating_mul(d + 1);
    let total_terms = guards.iter().map(|g| g.nterms()).sum::<usize>();
    let maximum_terms = guards.iter().map(|g| g.nterms()).max().unwrap_or(0);
    let coefficient_bits = guards
        .iter()
        .flat_map(|g| g.coefficients.iter())
        .map(height)
        .max()
        .unwrap_or(0);
    let separator_bits = cover
        .iter()
        .flat_map(|i| std::iter::once(&i.a).chain(i.b.iter()))
        .map(height)
        .max()
        .unwrap_or(0);
    let checks = json!({
        "degree":(3..=32).contains(&d),"source_terms":p.nterms()<=DEFAULT_MAX_TERMS,
        "intervals":cover.len()<=64,"coefficient_slots":coefficient_slots<=256,
        "guards":guards.len()<=256,"max_guard_terms":maximum_terms<=DEFAULT_MAX_TERMS,
        "total_guard_terms":total_terms<=65536,"coefficient_bits":coefficient_bits<=512,
        "separator_bits":separator_bits<=256,"positive_roots":positive_roots<=8,
        "sample_guards_nonzero":zero_obstructions==0,"full_positive_cover":true,
    });
    let production_eligible = checks.as_object().unwrap().values().all(|v| v == true);
    let non_source_eligible = checks
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| key.as_str() != "source_terms")
        .all(|(_, v)| v == true);
    report["production_limits"] = json!({"degree_min":3,"degree_max":32,"source_terms":4096,"intervals":64,"coefficient_slots":256,"guards":256,"max_guard_terms":4096,"total_guard_terms":65536,"coefficient_bits":512,"separator_bits":256,"positive_roots":8});
    report["production_inventory"] = json!({"coefficient_slots":coefficient_slots,"guards":guards.len(),"max_guard_terms":maximum_terms,"total_guard_terms":total_terms,"max_coefficient_bits":coefficient_bits,"max_separator_bits":separator_bits});
    report["production_limit_checks"] = checks;
    report["production_eligible"] = json!(production_eligible);
    report["production_non_source_limits_satisfied"] = json!(non_source_eligible);
    Ok(Probe { report, guards })
}

const MAX_SEARCH_KNOTS: usize = 32;
const MAX_SYMBOLIC_CANDIDATES: usize = 4;

/// A finite menu includes the bisection cover itself, permitting coalescence
/// of adjacent pieces; the bounded path ranking need not explore every cover.
/// Extra simple rationals adapt that menu
/// to the existing separators; no fixture or polynomial identity is consulted.
fn separator_menu(f: &UnivariatePolynomial<Q>, initial: &[Interval]) -> Vec<Rational> {
    let mut essential = initial
        .iter()
        .filter_map(|i| i.b.clone())
        .collect::<Vec<_>>();
    essential.sort();
    essential.dedup();
    let mut extra = Vec::new();
    for exponent in -8i32..=8 {
        extra.push(if exponent >= 0 {
            Rational::from(1u64 << exponent as u32)
        } else {
            Rational::one() / Rational::from(1u64 << (-exponent) as u32)
        });
    }
    for knot in &essential {
        for (numerator, denominator) in [(1, 2), (3, 4), (5, 4), (3, 2), (2, 1)] {
            extra.push(knot * &Rational::from((numerator, denominator)));
        }
    }
    for pair in essential.windows(2) {
        extra.push((&pair[0] + &pair[1]) / Rational::from(2));
    }
    extra.sort_by(|a, b| height(a).cmp(&height(b)).then_with(|| a.cmp(b)));
    for knot in extra {
        if essential.len() == MAX_SEARCH_KNOTS {
            break;
        }
        if knot > Rational::zero()
            && height(&knot) <= 256
            && !f.evaluate(&knot).is_zero()
            && !essential.contains(&knot)
        {
            essential.push(knot);
        }
    }
    essential.retain(|x| *x > Rational::zero() && !f.evaluate(x).is_zero());
    essential.sort();
    essential.dedup();
    essential.truncate(MAX_SEARCH_KNOTS);
    essential
}

#[derive(Clone)]
struct CoverPath {
    /// Indices in [0, positive rational knots..., +infinity].
    vertices: Vec<usize>,
    roots: usize,
    separator_bits: u64,
}

fn numeric_interval(f: &UnivariatePolynomial<Q>, interval: Interval) -> Option<NumericInterval> {
    if f.evaluate(&interval.a).is_zero()
        || interval.b.as_ref().is_some_and(|b| f.evaluate(b).is_zero())
    {
        return None;
    }
    let matrix = weights(f.degree(), &interval.a, interval.b.as_ref());
    let values = transformed(f, &matrix);
    let count = variations(&values);
    (count <= 1).then_some((interval, matrix, values, count))
}

/// Exact 0/1-variation edges form a finite directed acyclic graph. Each
/// complete path covers (0,+infinity), with no gaps, overlaps or root seams.
/// The ranking uses interval count and rational height only: symbolic cost
/// remains unknown until every coefficient identity has been reconstructed.
fn compact_candidates(
    f: &UnivariatePolynomial<Q>,
    knots: &[Rational],
    positive_roots: usize,
) -> Result<(Vec<Vec<NumericInterval>>, Value)> {
    ensure!(knots.len() <= MAX_SEARCH_KNOTS, "rational-menu cap");
    ensure!(
        knots.iter().all(|x| *x > Rational::zero()) && knots.windows(2).all(|x| x[0] < x[1]),
        "strictly increasing positive rational menu required"
    );
    let mut finite = vec![Rational::zero()];
    finite.extend_from_slice(knots);
    let terminal = finite.len();
    let mut paths: Vec<Vec<CoverPath>> = vec![Vec::new(); terminal + 1];
    paths[0].push(CoverPath {
        vertices: vec![0],
        roots: 0,
        separator_bits: 0,
    });
    let mut attempted_edges = 0;
    let mut accepted_edges = 0;
    for upper in 1..=terminal {
        for lower in 0..upper {
            if paths[lower].is_empty() {
                continue;
            }
            attempted_edges += 1;
            let interval = Interval {
                a: finite[lower].clone(),
                b: finite.get(upper).cloned(),
                depth: 0,
            };
            let Some((_, _, _, count)) = numeric_interval(f, interval) else {
                continue;
            };
            accepted_edges += 1;
            for mut path in paths[lower].clone() {
                path.roots += count;
                if path.roots > positive_roots
                    || path.vertices.len() * (f.degree() + 1) > 256
                    || (upper == terminal && path.roots != positive_roots)
                {
                    continue;
                }
                path.vertices.push(upper);
                path.separator_bits += finite.get(upper).map(height).unwrap_or(0);
                paths[upper].push(path);
            }
        }
        paths[upper].sort_by(|a, b| {
            a.vertices
                .len()
                .cmp(&b.vertices.len())
                .then_with(|| a.separator_bits.cmp(&b.separator_bits))
                .then_with(|| a.vertices.cmp(&b.vertices))
        });
        paths[upper].dedup_by(|a, b| a.vertices == b.vertices);
        paths[upper].truncate(MAX_SYMBOLIC_CANDIDATES);
    }
    let selected = paths[terminal]
        .iter()
        .map(|path| {
            path.vertices
                .windows(2)
                .map(|v| {
                    numeric_interval(
                        f,
                        Interval {
                            a: finite[v[0]].clone(),
                            b: finite.get(v[1]).cloned(),
                            depth: 0,
                        },
                    )
                    .context("selected exact cover edge disappeared")
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let report = json!({
        "separator_menu":knots.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "max_menu_knots":MAX_SEARCH_KNOTS,
        "max_symbolic_candidates":MAX_SYMBOLIC_CANDIDATES,
        "numeric_edges_attempted":attempted_edges,
        "numeric_edges_accepted":accepted_edges,
        "numeric_complete_candidates":selected.len(),
        "ranking":"fewest intervals, then rational height; prediction only",
        "scope":"bounded finite-menu search; failure does not prove no admissible cover exists"
    });
    Ok((selected, report))
}

fn compact_probe(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    initial: Probe,
) -> Result<Probe> {
    let started = Instant::now();
    let Some(raw_intervals) = initial.report["intervals"].as_array() else {
        return Ok(initial);
    };
    let cover = raw_intervals
        .iter()
        .map(|v| {
            Ok(Interval {
                a: rational(v["lower"].as_str().context("initial lower endpoint")?)?,
                b: v["upper"].as_str().map(rational).transpose()?,
                depth: 0,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if cover.len().saturating_sub(1) > MAX_SEARCH_KNOTS {
        let mut initial = initial;
        initial.report["compact_cover_search"] =
            json!({"declined":"initial separator menu exceeds finite search cap"});
        return Ok(initial);
    }
    let f = algebra.specialize_univariate(p, variable, prefix)?;
    let positive_roots = initial.report["positive_distinct_roots"]
        .as_u64()
        .context("positive root count")? as usize;
    let knots = separator_menu(&f, &cover);
    let (candidates, mut search) = compact_candidates(&f, &knots, positive_roots)?;
    search["numeric_search_seconds"] = json!(started.elapsed().as_secs_f64());
    search["baseline_inventory"] = initial.report["production_inventory"].clone();
    search["baseline_intervals"] = initial.report["intervals"].clone();
    let mut selected = initial;
    let mut measurements = Vec::new();
    let rank = |report: &Value| {
        (
            !report["production_limit_checks"]
                .as_object()
                .is_some_and(|checks| {
                    checks
                        .iter()
                        .filter(|(name, _)| {
                            name.as_str() != "source_terms" && name.as_str() != "total_guard_terms"
                        })
                        .all(|(_, ok)| ok == true)
                }),
            report["production_inventory"]["total_guard_terms"]
                .as_u64()
                .unwrap_or(u64::MAX),
            report["production_inventory"]["max_coefficient_bits"]
                .as_u64()
                .unwrap_or(u64::MAX),
        )
    };
    for (index, candidate) in candidates.into_iter().enumerate() {
        let count = candidate.iter().map(|x| x.3).sum::<usize>();
        ensure!(
            count == positive_roots,
            "cover does not retain exact positive root inventory"
        );
        let mut metadata = selected.report.clone();
        metadata["accepted_intervals"] = json!(candidate.len());
        metadata["sum_interval_variations"] = json!(count);
        metadata["cover_selection"] = json!("bounded_rational_search");
        let attempt = construct_cover(
            algebra,
            p,
            variable,
            prefix,
            positive_roots,
            metadata,
            candidate,
        )?;
        measurements.push(
            json!({"candidate":index,"intervals":attempt.report["intervals"],
            "inventory":attempt.report["production_inventory"],
            "non_source_limits_satisfied":attempt.report["production_non_source_limits_satisfied"],
            "symbolic_guard_seconds":attempt.report["symbolic_guard_seconds"]}),
        );
        if rank(&attempt.report) < rank(&selected.report) {
            selected = attempt;
        }
    }
    search["symbolic_candidates"] = json!(measurements);
    search["total_search_seconds"] = json!(started.elapsed().as_secs_f64());
    selected.report["compact_cover_search"] = search;
    Ok(selected)
}

fn apply_guard_policy(report: &mut Value, source_terms: usize, total_terms: usize) -> Result<()> {
    validate_max_terms(source_terms)?;
    validate_total_guard_terms(total_terms)?;
    let Some(mut checks) = report["production_limit_checks"].as_object().cloned() else {
        report["configured_policy_eligible"] = json!(false);
        return Ok(());
    };
    checks.insert(
        "source_terms".into(),
        json!(
            report["terms"]
                .as_u64()
                .is_some_and(|n| n <= source_terms as u64)
        ),
    );
    checks.insert(
        "total_guard_terms".into(),
        json!(
            report["production_inventory"]["total_guard_terms"]
                .as_u64()
                .is_some_and(|n| n <= total_terms as u64)
        ),
    );
    report["configured_policy_eligible"] = json!(checks.values().all(|v| v == true));
    report["configured_limit_checks"] = json!(checks);
    report["configured_source_term_limit"] = json!(source_terms);
    report["configured_total_guard_term_limit"] = json!(total_terms);
    Ok(())
}
fn write_atomic(path: &std::path::Path, value: &Value) -> Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
fn work(args: &Args) -> Result<Value> {
    let bytes = std::fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    let operands = capture
        .operands
        .as_ref()
        .context("capture omitted operands")?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "capture input checksum"
    );
    ensure!(operands.len() == 1, "single discriminant operand required");
    let algebra = Algebra::new(&capture.variable_names)?;
    let variable = capture.variable.context("variable missing")?;
    let p = algebra.parse(&operands[0])?;
    let prefix = args
        .prefix
        .iter()
        .map(|x| rational(x))
        .collect::<Result<Vec<_>>>()?;
    let initial = probe(&algebra, &p, variable, &prefix, args.max_terms)?;
    let Probe { mut report, guards } = if args.compact_cover_search {
        compact_probe(&algebra, &p, variable, &prefix, initial)?
    } else {
        initial
    };
    report["capture_sha256"] = json!(format!("{:x}", Sha256::digest(bytes)));
    report["input_hash"] = json!(capture.input_hash);
    report["variable"] = json!(variable);
    report["variables"] = json!(capture.variable_names);
    report["prefix"] = json!(args.prefix);
    apply_guard_policy(&mut report, args.max_terms, args.max_total_guard_terms)?;
    report["factorization"] = json!([]);
    report["factorization_complete"] = json!(false);
    let progress = args.output.with_extension("progress.json");
    write_atomic(&progress, &report)?;
    if args.factor_guards
        && report["eligible"] == true
        && report["configured_policy_eligible"] == true
    {
        for (index, g) in guards.iter().enumerate() {
            report["active_guard"] = json!(index);
            write_atomic(&progress, &report)?;
            let started = Instant::now();
            let factors = factors_checked(&[g.clone()])?;
            report["factorization"].as_array_mut().unwrap().push(json!({"index":index,"seconds":started.elapsed().as_secs_f64(),"factors":factors.len(),"factor_terms":factors.iter().map(|f|f.nterms()).collect::<Vec<_>>()}));
            write_atomic(&progress, &report)?;
        }
        report["factorization_complete"] = json!(true);
        report["active_guard"] = Value::Null;
        write_atomic(&progress, &report)?;
    }
    Ok(report)
}
fn main() -> Result<()> {
    let args = Args::parse();
    validate_max_terms(args.max_terms)?;
    validate_total_guard_terms(args.max_total_guard_terms)?;
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "deadline<=120s required"
    );
    ensure!(
        !args.compact_cover_search || args.seconds <= 60.0,
        "compact cover deadline<=60s required"
    );
    if args.worker {
        println!("{}", work(&args)?);
        return Ok(());
    }
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&executable)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let mut outcome = run_bounded(&mut command, &limits)?;
    let result = serde_json::from_str::<Value>(&outcome.stdout).ok();
    if result.is_some() {
        outcome.stdout.clear();
    }
    let report = json!({"process":outcome,"result":result,"limits":limits,"cpu":args.cpu,"research_max_terms":args.max_terms,"binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(executable)?)),"note":"Research guard-cost diagnostic; no production certificate or original completion claimed."});
    write_atomic(&args.output, &report)?;
    println!(
        "{}",
        json!({"process":report["process"]["status"],"eligible":report["result"]["eligible"],"intervals":report["result"]["accepted_intervals"],"guards":report["result"]["unique_guards"],"factorization_complete":report["result"]["factorization_complete"]})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn attempt(expression: &str) -> Result<Probe> {
        let a = Algebra::new(&["s".into(), "x".into()])?;
        probe(
            &a,
            &a.parse(expression)?,
            1,
            &[Rational::from(2)],
            DEFAULT_MAX_TERMS,
        )
    }
    #[test]
    fn compact_search_coalesces_exact_rootless_pieces() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        let p = a.parse("x^4+x^2+1")?;
        let f = a.specialize_univariate(&p, 0, &[])?;
        let knots = [
            Rational::from((1, 4)),
            Rational::from((1, 2)),
            Rational::one(),
        ];
        let (covers, report) = compact_candidates(&f, &knots, 0)?;
        assert!(!covers.is_empty());
        assert_eq!(covers[0].len(), 1);
        assert!(full_cover(
            &covers[0].iter().map(|x| x.0.clone()).collect::<Vec<_>>()
        ));
        assert_eq!(covers[0][0].3, 0);
        assert!(report["numeric_edges_accepted"].as_u64().unwrap() > 0);
        Ok(())
    }
    #[test]
    fn explicit_guard_budget_preserves_default_failure_and_other_limits() -> Result<()> {
        let mut report = json!({"terms":19942,"production_inventory":{"total_guard_terms":79798},
            "production_non_source_limits_satisfied":false,
            "production_limit_checks":{"source_terms":false,"total_guard_terms":false,"max_guard_terms":true}});
        apply_guard_policy(&mut report, 32768, 65536)?;
        assert_eq!(report["configured_policy_eligible"], false);
        apply_guard_policy(&mut report, 32768, 131072)?;
        assert_eq!(report["configured_policy_eligible"], true);
        assert_eq!(report["production_non_source_limits_satisfied"], false);
        report["production_limit_checks"]["max_guard_terms"] = json!(false);
        apply_guard_policy(&mut report, 32768, 131072)?;
        assert_eq!(report["configured_policy_eligible"], false);
        for invalid in [0, MAX_TOTAL_GUARD_TERMS + 1, usize::MAX] {
            assert!(validate_total_guard_terms(invalid).is_err());
        }
        Ok(())
    }

    #[test]
    fn compact_cover_replays_all_original_guards_and_preserves_two_roots() -> Result<()> {
        let a = Algebra::new(&["s".into(), "x".into(), "unused".into()])?;
        let p = a.parse("(x-1)*(x-2)*(x^2-x+s)")?;
        let prefix = [Rational::from(2)];
        let baseline = probe(&a, &p, 1, &prefix, 4096)?;
        let result = compact_probe(&a, &p, 1, &prefix, baseline)?;
        assert_eq!(result.report["sum_interval_variations"], 2);
        assert_eq!(result.report["full_positive_cover"], true);
        assert_eq!(
            result.report["production_non_source_limits_satisfied"],
            true
        );
        assert!(result.guards.iter().all(|g| g.variables() == &a.variables));
        assert!(result.guards.contains(&canonical(&p.univariate_lcoeff(1))));
        assert!(
            result
                .guards
                .contains(&canonical(&p.replace(1, &Rational::zero())))
        );
        for interval in result.report["intervals"].as_array().unwrap() {
            if let Some(endpoint) = interval["upper"].as_str() {
                let guard = p.replace(1, &rational(endpoint)?);
                assert!(!guard.replace_all(&prefix).is_zero());
                assert!(result.guards.contains(&canonical(&guard)));
            }
        }
        Ok(())
    }

    #[test]
    fn compact_search_rejects_root_seams_repeated_roots_and_invalid_menus() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        let p = a.parse("(x-1)^2*(x^2+1)")?;
        let f = a.specialize_univariate(&p, 0, &[])?;
        assert!(
            numeric_interval(
                &f,
                Interval {
                    a: Rational::zero(),
                    b: Some(Rational::one()),
                    depth: 0
                }
            )
            .is_none()
        );
        let (covers, _) = compact_candidates(&f, &[Rational::from((1, 2)), Rational::from(2)], 1)?;
        assert!(covers.is_empty());
        for menu in [
            vec![Rational::zero()],
            vec![Rational::one(), Rational::one()],
            vec![Rational::from(2), Rational::one()],
            vec![Rational::one(); MAX_SEARCH_KNOTS + 1],
        ] {
            assert!(compact_candidates(&f, &menu, 1).is_err());
        }
        let menu = separator_menu(
            &f,
            &[
                Interval {
                    a: Rational::zero(),
                    b: Some(Rational::one()),
                    depth: 0,
                },
                Interval {
                    a: Rational::one(),
                    b: None,
                    depth: 0,
                },
            ],
        );
        assert!(!menu.contains(&Rational::one()));
        assert!(menu.len() <= MAX_SEARCH_KNOTS);
        Ok(())
    }
    #[test]
    fn mobius_and_tail_weights_match_exact_cleared_evaluations() -> Result<()> {
        let a = Algebra::new(&["x".into()])?;
        let p = a.parse("2*x^4-3*x^2+x-7")?;
        let f = a.specialize_univariate(&p, 0, &[])?;
        let lo = rational("2/3")?;
        let hi = rational("7/2")?;
        for upper in [None, Some(&hi)] {
            let c = transformed(&f, &weights(f.degree(), &lo, upper));
            for point in ["0", "1/3", "2", "-1/2"] {
                let u = rational(point)?;
                let lhs = c.iter().rev().fold(Rational::zero(), |a, c| a * &u + c);
                let rhs = if let Some(b) = upper {
                    f.evaluate(&((&lo + &(b * &u)) / (Rational::one() + &u)))
                        * powers(&(Rational::one() + &u), f.degree())[f.degree()].clone()
                } else {
                    f.evaluate(&(&lo + &u))
                };
                assert_eq!(lhs, rhs);
            }
        }
        Ok(())
    }
    #[test]
    fn unsaturated_global_variations_reduce_to_zero_one_intervals() -> Result<()> {
        let p = attempt("(x-1)*(x-2)*(x^2-x+1)")?;
        assert_eq!(p.report["global_variations"], 4);
        assert_eq!(p.report["positive_distinct_roots"], 2);
        assert_eq!(p.report["eligible"], true);
        assert_eq!(p.report["sum_interval_variations"], 2);
        let empty = attempt("x^2-x+1")?;
        assert_eq!(empty.report["eligible"], true);
        assert_eq!(empty.report["sum_interval_variations"], 0);
        Ok(())
    }
    #[test]
    fn repeated_positive_root_does_not_get_a_simple_root_certificate() -> Result<()> {
        assert_eq!(attempt("(x-1)^2*(x^2+1)")?.report["eligible"], false);
        Ok(())
    }

    #[test]
    fn enlarged_source_budget_records_independent_production_limits() -> Result<()> {
        let algebra = Algebra::new(&["a".into(), "b".into(), "z".into(), "unused".into()])?;
        let support = (0..=32)
            .flat_map(|i| (0..=32).map(move |j| format!("a^{i}*b^{j}")))
            .collect::<Vec<_>>()
            .join("+");
        let p = algebra.parse(&format!("({support})*(z-1)*(z-2)*(z^2-z+1)+a"))?;
        assert_eq!(p.nterms(), 5445);
        let prefix = [Rational::zero(), Rational::one()];
        assert!(probe(&algebra, &p, 2, &prefix, DEFAULT_MAX_TERMS).is_err());
        let result = probe(&algebra, &p, 2, &prefix, 8192)?;
        assert_eq!(result.report["eligible"], true);
        assert_eq!(result.report["full_positive_cover"], true);
        assert_eq!(result.report["global_variations"], 4);
        assert_eq!(result.report["sum_interval_variations"], 2);
        assert_eq!(result.report["production_eligible"], false);
        assert_eq!(
            result.report["production_limit_checks"]["source_terms"],
            false
        );
        assert_eq!(
            result.report["production_non_source_limits_satisfied"],
            true
        );
        assert!(
            result
                .guards
                .iter()
                .all(|g| g.variables() == &algebra.variables)
        );
        for guard in [p.replace(2, &Rational::zero()), p.univariate_lcoeff(2)] {
            assert!(result.guards.contains(&canonical(&guard)));
        }
        assert!(
            result.report["mandatory_guard_indices"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["kind"] == "seam")
        );
        let mut previous = Rational::zero();
        let intervals = result.report["intervals"].as_array().unwrap();
        for (index, interval) in intervals.iter().enumerate() {
            let lower = rational(interval["lower"].as_str().unwrap())?;
            assert_eq!(lower, previous);
            if let Some(upper) = interval["upper"].as_str() {
                previous = rational(upper)?;
                assert!(previous > lower);
                assert!(!p.replace(2, &previous).replace_all(&prefix).is_zero());
            } else {
                assert_eq!(index + 1, intervals.len());
            }
        }
        Ok(())
    }

    #[test]
    fn source_budget_rejects_invalid_limits_maps_and_prefixes() -> Result<()> {
        let algebra = Algebra::new(&["s".into(), "z".into()])?;
        let p = algebra.parse("z^3+1")?;
        for cap in [0, RESEARCH_MAX_TERMS + 1, usize::MAX] {
            assert!(probe(&algebra, &p, 1, &[Rational::one()], cap).is_err());
        }
        assert!(probe(&algebra, &p, 2, &[Rational::one()], 4096).is_err());
        assert!(probe(&algebra, &p, 1, &[], 4096).is_err());
        let other = Algebra::new(&["t".into(), "z".into()])?;
        assert!(probe(&other, &p, 1, &[Rational::one()], 4096).is_err());
        let args = Args::try_parse_from(["probe", "capture.json", "--output", "result.json"])?;
        assert_eq!(args.max_terms, 4096);
        let args = Args::try_parse_from([
            "probe",
            "capture.json",
            "--output",
            "result.json",
            "--max-terms",
            "32768",
            "--worker",
        ])?;
        assert_eq!(args.max_terms, 32768);
        assert!(args.worker);
        Ok(())
    }

    #[test]
    fn symbolic_coefficient_replay_matches_full_transform_identity() -> Result<()> {
        let algebra = Algebra::new(&["s".into(), "z".into(), "u".into()])?;
        let p = algebra.parse("(s^2+1)*z^5-(s-2)*z^3+3/7*z-5")?;
        let u = algebra.parse("u")?;
        let one = algebra.parse("1")?;
        let coefficients = p.to_univariate_polynomial_list(1);
        let a = rational("2/3")?;
        for b in [None, Some(rational("13/5")?)] {
            let matrix = weights(5, &a, b.as_ref());
            let actual = matrix.iter().enumerate().fold(p.zero(), |sum, (k, row)| {
                let c = coefficients.iter().fold(p.zero(), |sum, (c, j)| {
                    sum + c.clone().mul_coeff(row[*j as usize].clone())
                });
                sum + c * &u.pow(k)
            });
            let expected = coefficients.iter().fold(p.zero(), |sum, (c, j)| {
                let numerator = one.clone().mul_coeff(a.clone())
                    + u.clone().mul_coeff(b.clone().unwrap_or_else(Rational::one));
                let mut term = c * &numerator.pow(*j as usize);
                if b.is_some() {
                    term = term * &(&one + &u).pow(5 - *j as usize);
                }
                sum + term
            });
            assert_eq!(actual, expected);
        }
        Ok(())
    }

    #[test]
    fn full_cover_rejects_missing_endpoints_gaps_overlaps_and_early_tails() {
        let interval = |a: i64, b: Option<i64>| Interval {
            a: Rational::from(a),
            b: b.map(Rational::from),
            depth: 0,
        };
        assert!(full_cover(&[interval(0, Some(2)), interval(2, None)]));
        for invalid in [
            vec![],
            vec![interval(1, None)],
            vec![interval(0, Some(2))],
            vec![interval(0, Some(2)), interval(3, None)],
            vec![interval(0, Some(2)), interval(1, None)],
            vec![interval(0, None), interval(2, None)],
            vec![interval(0, Some(0)), interval(0, None)],
        ] {
            assert!(!full_cover(&invalid));
        }
    }
}
