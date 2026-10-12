//! Opt-in projective sections (`solver.projective_sections`) of the
//! bilinear_general engine: exact membership against the baseline, forgery
//! rejection, strict fallback and serde round trips.
use serde_json::json;
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    bilinear_general::{
        self, Atom, Condition, PieceSpec, Separator, SignPremise, projective_empty_claim,
        projective_piece,
    },
    comparison::compare_generic,
    output::{
        PROJECTIVE_INFINITY, RootBound, RootIndexDomain, SolveResult, Status, has_projective_bounds,
    },
    roots::roots_in_open_interval,
    solver::solve,
    verify::verify,
};

/// a=p, b=p+s^2, c=p+r^2, d=p+s^2+r^2 with U=(x+1)*(y+1): the corner chain
/// a<=b<=d, a<=c<=d and Delta=(s*r)^2 with p=u-3, s=v-1, r=u-v (the
/// Gluza-shaped synthetic chain of tests/bilinear_general.rs).
const CHAIN: &str = "(u-3)*x*y+(u-3+(v-1)^2)*x+(u-3+(u-v)^2)*y+(u-3+(v-1)^2+(u-v)^2)";

fn problem(variables: &[&str], target: &str, guide: Option<&str>) -> Problem {
    let mut constraints: Vec<String> = variables.iter().map(|v| format!("{v}>0")).collect();
    constraints.push(format!("({target})>0"));
    let mut p = Problem {
        name: Some("projective sections".into()),
        variables: variables.iter().map(|v| v.to_string()).collect(),
        constraints,
        ..Default::default()
    };
    p.solver.engine = "bilinear_general".into();
    p.solver.ordering = "given".into();
    p.solver.bilinear_general_guide = guide.map(str::to_owned);
    p.limits.wall_time_secs = 120.;
    p
}

fn chain(projective: bool) -> Problem {
    let mut p = problem(&["u", "v", "y", "x"], CHAIN, Some("(x+1)*(y+1)"));
    p.solver.projective_sections = projective;
    p
}

fn checked(p: &Problem) -> SolveResult {
    let r = solve(p).unwrap();
    assert_eq!(r.status, Status::CompleteGeneric, "{:?}", r.diagnostic);
    assert!(verify(&r).unwrap().verified);
    let roundtrip: SolveResult = serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();
    assert!(verify(&roundtrip).unwrap().verified);
    assert_eq!(
        first_difference(
            &serde_json::to_value(&r).unwrap(),
            &serde_json::to_value(&roundtrip).unwrap(),
            String::new()
        ),
        None
    );
    r
}

/// The first JSON pointer at which two values differ.
fn first_difference(a: &serde_json::Value, b: &serde_json::Value, path: String) -> Option<String> {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys()) {
                let child = format!("{path}/{key}");
                match (x.get(key), y.get(key)) {
                    (Some(p), Some(q)) => {
                        if let Some(d) = first_difference(p, q, child) {
                            return Some(d);
                        }
                    }
                    _ => return Some(child),
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .enumerate()
            .find_map(|(i, (p, q))| first_difference(p, q, format!("{path}/{i}"))),
        _ if a == b => None,
        // serde_json's default float parser may move a timing by one ulp.
        (Value::Number(x), Value::Number(y))
            if x.is_f64()
                && y.is_f64()
                && (x.as_f64().unwrap() - y.as_f64().unwrap()).abs()
                    <= 1e-12 * x.as_f64().unwrap().abs().max(1.0) =>
        {
            None
        }
        _ => Some(format!("{path}: {a} vs {b}")),
    }
}

fn rejected(r: &SolveResult) {
    assert!(verify(r).map_or(true, |v| !v.verified));
}

fn baseline(p: &Problem) -> SolveResult {
    let mut b = p.clone();
    b.solver.engine = "baseline".into();
    b.solver.bilinear_general_guide = None;
    b.solver.projective_sections = false;
    let r = solve(&b).unwrap();
    assert_eq!(r.status, Status::CompleteGeneric);
    assert!(verify(&r).unwrap().verified);
    r
}

/// Independent exact side test of one bound at a point: Some(true) when the
/// coordinate lies above the selected root, None on the boundary itself.
fn above(
    algebra: &Algebra,
    polys: &[Poly],
    bound: &RootBound,
    level: usize,
    point: &[Rational],
) -> Option<bool> {
    let fiber = algebra
        .specialize_univariate(&polys[bound.polynomial], level, &point[..level])
        .unwrap();
    assert!(!fiber.is_zero(), "nullified boundary");
    let x = &point[level];
    if fiber.evaluate(x).is_zero() {
        return None;
    }
    let below_x = |lo: &Rational| roots_in_open_interval(&fiber, lo, x);
    Some(match bound.index_domain {
        RootIndexDomain::Real => {
            let lc = fiber.coefficients().last().unwrap().abs();
            let cauchy = fiber
                .coefficients()
                .iter()
                .map(|c| c.abs() / &lc)
                .fold(Rational::zero(), |m, c| if c > m { c } else { m })
                + Rational::from(2);
            let left = if -cauchy.clone() < x.clone() - Rational::one() {
                -cauchy
            } else {
                x.clone() - Rational::one()
            };
            below_x(&left) > bound.index
        }
        // A missing projective root is +infinity, never below x.
        RootIndexDomain::Positive | RootIndexDomain::PositiveOrInfinity => {
            x > &Rational::zero() && below_x(&Rational::zero()) > bound.index
        }
        RootIndexDomain::RealDescending => unreachable!("not produced here"),
    })
}

/// Cells of `r` containing `point` (None if the point lies on a boundary).
fn containing(
    r: &SolveResult,
    polys: &[Poly],
    algebra: &Algebra,
    point: &[Rational],
) -> Option<usize> {
    let mut count = 0;
    'cells: for cell in &r.cells {
        for (level, axis) in cell.axes.iter().enumerate() {
            for (bound, lower) in [(&axis.lower, true), (&axis.upper, false)] {
                let Some(bound) = bound else { continue };
                match above(algebra, polys, bound, level, point) {
                    None => return None,
                    Some(side) if side != lower => continue 'cells,
                    Some(_) => {}
                }
            }
        }
        count += 1;
    }
    Some(count)
}

fn on_exceptional(r: &SolveResult, algebra: &Algebra, point: &[Rational]) -> bool {
    r.exceptional_polynomials.iter().any(|&id| {
        let p = algebra.parse(&r.polynomials[id]).unwrap();
        symgcad::solver::evaluate(&p, point).is_zero()
    })
}

struct SplitMix(u64);
impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    /// A positive rational with a large odd denominator, in (0, 6).
    fn positive(&mut self) -> Rational {
        Rational::from((self.next() % 60_000 + 1) as i64) / Rational::from(10_007)
    }
}

/// Exact membership of S={all coordinates>0, F>0} against both cell sets on
/// a dense rational grid and random rational points: no point lies in two
/// cells of either result, and every point off the exceptional sets lies in
/// exactly one cell of each result iff it lies in S.
fn compare_membership(p: &Problem, projective: &SolveResult, reference: &SolveResult) -> usize {
    let algebra = Algebra::new(&projective.order).unwrap();
    assert_eq!(projective.order, reference.order);
    let polys = |r: &SolveResult| {
        r.polynomials
            .iter()
            .map(|t| algebra.parse(t).unwrap())
            .collect::<Vec<_>>()
    };
    let (left, right) = (polys(projective), polys(reference));
    let normalized = symgcad::problem::normalize(p, &projective.order).unwrap();
    let in_s = |point: &[Rational]| {
        normalized
            .positives
            .iter()
            .all(|q| symgcad::solver::evaluate(q, point) > Rational::zero())
    };
    let grid: Vec<Rational> = [1, 2, 4, 5, 7, 8, 10, 13]
        .iter()
        .map(|&k| Rational::from(k) / Rational::from(3))
        .collect();
    let dims = projective.order.len();
    let mut points: Vec<Vec<Rational>> = vec![vec![]];
    for _ in 0..dims {
        points = points
            .into_iter()
            .flat_map(|prefix| {
                grid.iter().map(move |g| {
                    let mut point = prefix.clone();
                    point.push(g.clone());
                    point
                })
            })
            .collect();
    }
    let mut rng = SplitMix(20261012);
    for _ in 0..400 {
        points.push((0..dims).map(|_| rng.positive()).collect());
    }
    let mut generic = 0;
    for point in &points {
        let inside = in_s(point);
        for (r, table) in [(projective, &left), (reference, &right)] {
            let count = containing(r, table, &algebra, point);
            if let Some(count) = count {
                assert!(count <= 1, "point {point:?} lies in {count} cells");
                if !on_exceptional(r, &algebra, point) {
                    assert_eq!(count == 1, inside, "membership differs at {point:?}");
                }
            }
        }
        if !on_exceptional(projective, &algebra, point)
            && !on_exceptional(reference, &algebra, point)
        {
            generic += 1;
        }
    }
    generic
}

fn assert_projective_cells(r: &SolveResult) -> (usize, usize) {
    let n = r.order.len();
    let (mut infinite, mut finite) = (0, 0);
    for cell in &r.cells {
        for axis in &cell.axes[..n - 2] {
            for b in [&axis.lower, &axis.upper].into_iter().flatten() {
                assert!(!b.index_domain.is_projective());
            }
        }
        for axis in &cell.axes[n - 2..] {
            let lower = axis.lower.as_ref().unwrap();
            assert_eq!(lower.index_domain, RootIndexDomain::Real);
            assert_eq!((lower.lower.as_str(), lower.upper.as_str()), ("0", "0"));
            let upper = axis.upper.as_ref().unwrap();
            assert_eq!(upper.index_domain, RootIndexDomain::PositiveOrInfinity);
            assert_eq!(upper.index, 0);
            if upper.lower == PROJECTIVE_INFINITY {
                assert_eq!(upper.upper, PROJECTIVE_INFINITY);
                infinite += 1;
            } else {
                finite += 1;
            }
        }
    }
    (infinite, finite)
}

#[test]
fn synthetic_chain_projective_plan_matches_baseline_membership() {
    let reference = baseline(&chain(false));
    for engine in ["optimized", "local", "baseline"] {
        let mut p = chain(true);
        p.solver.bilinear_general_base_engine = engine.into();
        let r = checked(&p);
        let evidence = r.bilinear_general.as_ref().unwrap();
        assert!(evidence.plan.projective);
        assert_eq!(evidence.plan.pieces, vec![projective_piece()]);
        assert_eq!(evidence.plan.empty, vec![projective_empty_claim()]);
        assert!(matches!(evidence.plan.separator, Separator::Square { .. }));
        assert_eq!(evidence.bases.len(), 1);
        // One cell per base cell of {d>0}.
        assert_eq!(r.cells.len(), evidence.bases[0].cells.len());
        let (infinite, finite) = assert_projective_cells(&r);
        assert!(infinite > 0 && finite > 0, "{infinite} {finite}");
        let generic = compare_membership(&p, &r, &reference);
        assert!(generic > 1500, "{generic}");
        // The certified common refinement agrees as well.
        let comparison = compare_generic(&r, &reference, &p.limits).unwrap();
        assert_eq!(
            comparison.equivalent,
            Some(true),
            "{:?}",
            comparison.counterexample
        );
    }
    // The strict plan of the same problem needs four pieces and more cells.
    let strict = checked(&chain(false));
    let strict_plan = &strict.bilinear_general.as_ref().unwrap().plan;
    assert!(!strict_plan.projective && strict_plan.pieces.len() == 4);
    let projective = checked(&chain(true));
    assert!(projective.cells.len() < strict.cells.len());
}

#[test]
fn option_off_is_unchanged_and_option_on_without_premises_falls_back() {
    // Off: no projective plan, no projective bound, no serialized flag.
    let off = checked(&chain(false));
    assert!(!has_projective_bounds(&off));
    let encoded = serde_json::to_value(&off).unwrap();
    assert!(
        encoded["bilinear_general"]["plan"]
            .get("projective")
            .is_none()
    );
    assert!(
        encoded["problem"]["solver"]
            .get("projective_sections")
            .is_none()
    );
    // On, but without a guide there are no ratio premises: the strict plan,
    // with identical pieces and cells.
    for guide in [None, Some("(x+2)*(y+1)")] {
        let mut on = chain(true);
        on.solver.bilinear_general_guide = guide.map(str::to_owned);
        let mut strict = on.clone();
        strict.solver.projective_sections = false;
        let fallback = checked(&on);
        let reference = checked(&strict);
        let plan = &fallback.bilinear_general.as_ref().unwrap().plan;
        if guide.is_none() {
            assert!(!plan.projective);
            assert!(
                plan.premises
                    .iter()
                    .all(|q| !matches!(q, SignPremise::RatioOrder { .. }))
            );
        }
        if !plan.projective {
            assert!(!has_projective_bounds(&fallback));
            assert_eq!(
                serde_json::to_value(plan).unwrap(),
                serde_json::to_value(&reference.bilinear_general.as_ref().unwrap().plan).unwrap()
            );
            assert_eq!(fallback.cells, reference.cells);
            assert_eq!(fallback.polynomials, reference.polynomials);
        }
    }
    // A Delta that is not a positive square (here Delta=-(s-t)^2) keeps the
    // strict plan even with the option on and a guide.
    let mut negative = problem(
        &["s", "t", "y", "x"],
        "-x*y+s*x-t*y-s^2+3*s*t-t^2",
        Some("(x+1)*(y+1)"),
    );
    negative.solver.projective_sections = true;
    let r = checked(&negative);
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    assert!(matches!(plan.separator, Separator::NegativeSquare { .. }));
    assert!(!plan.projective && !has_projective_bounds(&r));
    let mut off = negative.clone();
    off.solver.projective_sections = false;
    assert_eq!(checked(&off).cells, r.cells);
}

#[test]
fn one_parameter_toy_with_a_constant_square_delta_is_projective() {
    // Delta=(t-1)*(t-2)-(t-3)*t=2: a constant positive square; with U=(x+1)*
    // (y+1) the chain a<=b<=d, a<=c<=d holds with constant gaps.
    let mut p = problem(
        &["t", "y", "x"],
        "(t-3)*x*y+(t-1)*x+(t-2)*y+t",
        Some("(x+1)*(y+1)"),
    );
    p.solver.projective_sections = true;
    let r = checked(&p);
    assert!(r.bilinear_general.as_ref().unwrap().plan.projective);
    assert_eq!(
        assert_projective_cells(&r).0 + assert_projective_cells(&r).1,
        2 * r.cells.len()
    );
    let reference = baseline(&p);
    assert!(compare_membership(&p, &r, &reference) > 500);
    let comparison = compare_generic(&r, &reference, &p.limits).unwrap();
    assert_eq!(
        comparison.equivalent,
        Some(true),
        "{:?}",
        comparison.counterexample
    );
}

#[test]
fn projective_sections_are_validated_as_a_bilinear_general_extension() {
    let mut p = chain(true);
    p.validate().unwrap();
    p.solver.merge = "aggressive".into();
    assert!(p.validate().is_err());
    let mut p = chain(true);
    p.solver.engine = "optimized".into();
    assert!(p.validate().is_err());
    // TOML round trip: the flag is explicit only when on.
    let p = chain(true);
    let text = toml::to_string(&p).unwrap();
    assert!(text.contains("projective_sections = true"));
    assert_eq!(
        serde_json::to_value(Problem::from_toml(&text).unwrap()).unwrap(),
        serde_json::to_value(&p).unwrap()
    );
    assert!(
        !toml::to_string(&chain(false))
            .unwrap()
            .contains("projective_sections")
    );
}

#[test]
fn separately_solved_projective_piece_assembles_and_plan_round_trips() {
    let p = chain(true);
    let planned = bilinear_general::plan(&p).unwrap();
    assert!(planned.plan.projective);
    assert_eq!(planned.problems.len(), 1);
    let piece = &planned.problems[0];
    assert!(!piece.solver.projective_sections);
    assert_eq!(piece.variables, ["u", "v"]);
    assert_eq!(piece.constraints.len(), 3);
    assert!(piece.split.is_empty());
    let text = serde_json::to_string(&planned.plan).unwrap();
    assert!(text.contains("\"projective\":true"));
    let plan: bilinear_general::BilinearGeneralPlan = serde_json::from_str(&text).unwrap();
    assert_eq!(serde_json::to_string(&plan).unwrap(), text);
    assert_eq!(
        serde_json::to_value(bilinear_general::piece_problems(&p, &plan).unwrap()).unwrap(),
        serde_json::to_value(&planned.problems).unwrap()
    );
    let base = solve(piece).unwrap();
    assert_eq!(base.status, Status::CompleteGeneric);
    let assembled = bilinear_general::assemble(&p, plan, vec![base], 0.0).unwrap();
    assert!(verify(&assembled).unwrap().verified);
    assert_eq!(assembled.cells, checked(&p).cells);
}

#[test]
fn projective_forgeries_are_rejected() {
    let p = chain(true);
    let r = checked(&p);
    let original = serde_json::to_value(&r).unwrap();
    let premises = &r.bilinear_general.as_ref().unwrap().plan.premises;
    let order_index = |l: Atom, u: Atom| {
        premises
            .iter()
            .position(|q| matches!(q, SignPremise::RatioOrder { lower, upper, .. } if *lower == l && *upper == u))
            .unwrap()
    };
    let n = r.order.len();
    let (y, x) = (n - 2, n - 1);
    // The true Delta: a valid strict separator, but not a projective one.
    let delta = {
        let algebra = Algebra::new(&r.order).unwrap();
        let k: Vec<Poly> = r
            .bilinear_general
            .as_ref()
            .unwrap()
            .plan
            .coefficients
            .iter()
            .map(|t| algebra.parse(t).unwrap())
            .collect();
        algebra.format(&(&(&k[1] * &k[2]) - &(&k[0] * &k[3])))
    };
    let mut mutations: Vec<(String, serde_json::Value)> = vec![
        // The option is the user's explicit opt-in.
        ("/problem/solver/projective_sections".into(), json!(false)),
        // A projective plan must be marked, exact, and backed by Delta>=0.
        ("/bilinear_general/plan/projective".into(), json!(false)),
        ("/bilinear_general/plan/pieces/0/split".into(), json!(["a"])),
        (
            "/bilinear_general/plan/pieces/0/conditions/0/sign".into(),
            json!(-1),
        ),
        (
            "/bilinear_general/plan/pieces/0/conditions/0/atom".into(),
            json!("c"),
        ),
        ("/bilinear_general/plan/empty".into(), json!([])),
        (
            "/bilinear_general/plan/empty/0/conditions/0/atom".into(),
            json!("b"),
        ),
        (
            "/bilinear_general/plan/separator".into(),
            json!({"kind": "delta", "polynomial": delta}),
        ),
        // Wrong section polynomial, index or domain; projective lower bound.
        (format!("/cells/0/axes/{y}/upper/polynomial"), json!(0)),
        (format!("/cells/0/axes/{x}/upper/index"), json!(1)),
        (format!("/cells/1/axes/{y}/upper/index"), json!(1)),
        (
            format!("/cells/0/axes/{x}/upper/index_domain"),
            json!("positive"),
        ),
        (
            format!("/cells/0/axes/{x}/lower/index_domain"),
            json!("positive_or_infinity"),
        ),
        (
            format!("/cells/0/axes/{y}/lower/index_domain"),
            json!("positive_or_infinity"),
        ),
        (format!("/cells/0/axes/{x}/upper"), json!(null)),
        (format!("/cells/0/axes/{y}/upper/lower"), json!("12345/7")),
        (format!("/cells/0/axes/{x}/sample"), json!("1000000")),
        // A projective selector smuggled into the ordinary base piece.
        (
            "/bilinear_general/bases/0/cells/0/axes/1/lower/index_domain".into(),
            json!("positive_or_infinity"),
        ),
        (
            "/bilinear_general/bases/0/problem/solver/projective_sections".into(),
            json!(true),
        ),
    ];
    for (l, u) in [(Atom::A, Atom::C), (Atom::B, Atom::D), (Atom::C, Atom::D)] {
        let i = order_index(l, u);
        // Missing premise: replace it by a harmless duplicate of another.
        let j = (i + 1) % premises.len();
        mutations.push((
            format!("/bilinear_general/plan/premises/{i}"),
            serde_json::to_value(&premises[j]).unwrap(),
        ));
        mutations.push((
            format!("/bilinear_general/plan/premises/{i}/multiplier"),
            json!("-1"),
        ));
    }
    for (path, replacement) in mutations {
        let mut forged = original.clone();
        // Replace the field, or insert it where serialization omits a default
        // (a real index domain, a false option).
        let (parent, key) = path.rsplit_once('/').unwrap();
        let slot = forged
            .pointer_mut(parent)
            .unwrap_or_else(|| panic!("missing evidence field {parent}"));
        match slot {
            serde_json::Value::Object(map) => {
                map.insert(key.to_string(), replacement);
            }
            serde_json::Value::Array(items) => items[key.parse::<usize>().unwrap()] = replacement,
            other => panic!("cannot mutate {path} in {other}"),
        }
        let forged: SolveResult = serde_json::from_value(forged).unwrap();
        let error = verify(&forged)
            .expect_err(&format!("accepted modification at {path}"))
            .to_string();
        // Each forgery fails for its own reason, not an unrelated one.
        let expected = if path.starts_with("/cells/") {
            "fiber restoration changed"
        } else if path.ends_with("/multiplier") {
            "multiplier"
        } else if path.contains("/premises/") {
            "ratio order"
        } else if path.contains("/pieces/") || path.contains("/empty") {
            "exactly the piece {d>0}"
        } else if path.ends_with("/separator") {
            "Delta = m*G^2"
        } else if path.contains("/bases/0/problem/") {
            "problem or options changed"
        } else {
            "projective"
        };
        assert!(error.contains(expected), "{path}: {error}");
    }
    // Removing a needed premise outright.
    for (l, u) in [(Atom::A, Atom::C), (Atom::B, Atom::D), (Atom::C, Atom::D)] {
        let mut bad = r.clone();
        bad.bilinear_general
            .as_mut()
            .unwrap()
            .plan
            .premises
            .remove(order_index(l, u));
        rejected(&bad);
    }
    // The projective plan with the strict four-piece inventory, and vice versa.
    let strict = checked(&chain(false));
    let mut bad = strict.clone();
    bad.problem.solver.projective_sections = true;
    bad.bilinear_general.as_mut().unwrap().plan.projective = true;
    rejected(&bad);
    let mut bad = r.clone();
    let evidence = bad.bilinear_general.as_mut().unwrap();
    evidence.plan.pieces = vec![PieceSpec {
        conditions: vec![Condition {
            atom: Atom::D,
            sign: 1,
        }],
        split: vec![Atom::C],
    }];
    rejected(&bad);
    // A strict result whose bound is relabeled projective, with or without
    // the option, and a generic baseline result with a projective selector.
    for option in [false, true] {
        let mut bad = strict.clone();
        bad.problem.solver.projective_sections = option;
        let axis = &mut bad.cells[0].axes[x];
        let bound = axis.upper.as_mut().or(axis.lower.as_mut()).unwrap();
        bound.index_domain = RootIndexDomain::PositiveOrInfinity;
        rejected(&bad);
    }
    let mut generic = baseline(&chain(false));
    let axis = &mut generic.cells[0].axes[x];
    if let Some(b) = axis.upper.as_mut() {
        b.index_domain = RootIndexDomain::PositiveOrInfinity;
    } else {
        axis.lower.as_mut().unwrap().index_domain = RootIndexDomain::PositiveOrInfinity;
    }
    let error = verify(&generic).unwrap_err().to_string();
    assert!(error.contains("projective"), "{error}");
    // Projective cells never take part in a compatibility merge.
    assert_eq!(symgcad::solver::merge(&r.cells), r.cells);
    // Cells do not overlap: membership of every cell's own sample.
    let algebra = Algebra::new(&r.order).unwrap();
    let polys: Vec<Poly> = r
        .polynomials
        .iter()
        .map(|t| algebra.parse(t).unwrap())
        .collect();
    for cell in &r.cells {
        let sample: Vec<Rational> = cell
            .axes
            .iter()
            .map(|a| symgcad::algebra::rational(&a.sample).unwrap())
            .collect();
        assert_eq!(containing(&r, &polys, &algebra, &sample), Some(1));
    }
}

#[test]
fn gluza33_projective_plan_has_one_piece_and_the_strict_plan_keeps_four() {
    let mut p = Problem::from_toml(include_str!("fixtures/gluza33_positive.toml")).unwrap();
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../reference/gluza33_rayleigh_guide.json")).unwrap();
    p.solver.engine = "bilinear_general".into();
    p.solver.ordering = "given".into();
    p.solver.bilinear_general_guide = Some(reference["guide"].as_str().unwrap().into());
    p.solver.bilinear_general_base_engine = "optimized".into();
    p.solver.positivity = true;
    p.limits.wall_time_secs = 60.;
    for order in [
        ["x4", "x7", "x2", "x1", "x0", "x6", "x3", "x5"],
        ["x4", "x7", "x2", "x1", "x0", "x6", "x5", "x3"],
    ] {
        let mut q = p.clone();
        q.solver.order = order.iter().map(|v| v.to_string()).collect();
        q.solver.bilinear_general_base_anchor = Some("x4".into());
        let strict = bilinear_general::plan(&q).unwrap();
        assert!(!strict.plan.projective);
        assert_eq!(strict.plan.pieces.len(), 4);
        assert_eq!(strict.plan.empty.len(), 1);
        q.solver.projective_sections = true;
        let planned = bilinear_general::plan(&q).unwrap();
        assert!(planned.plan.projective);
        assert_eq!(planned.plan.pieces, vec![projective_piece()]);
        assert_eq!(planned.plan.empty, vec![projective_empty_claim()]);
        // Premises and separator are the strict plan's, replayed unchanged.
        assert_eq!(
            serde_json::to_value(&planned.plan.premises).unwrap(),
            serde_json::to_value(&strict.plan.premises).unwrap()
        );
        assert_eq!(planned.plan.separator, strict.plan.separator);
        assert_eq!(planned.problems.len(), 1);
        let piece = &planned.problems[0];
        assert_eq!(piece.variables, order[..6]);
        assert_eq!((piece.constraints.len(), piece.split.len()), (7, 0));
        assert_eq!(piece.solver.homogeneous_anchor.as_deref(), Some("x4"));
        assert!(!piece.solver.projective_sections);
        // d is the same polynomial in both variable orders (F(x3=0,x5=0)).
        let algebra = Algebra::new(&planned.plan.order).unwrap();
        let d = algebra.parse(&planned.plan.coefficients[3]).unwrap();
        assert_eq!(
            algebra
                .parse(piece.constraints.last().unwrap().trim_end_matches(">0"))
                .unwrap(),
            d
        );
    }
}
