//! Strict-Bezout pair omission in certified global projection: toy
//! identities, independent replay, mutations and agreement with the baseline.
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    output::{LiftTrace, SolveResult, Status},
    problem::normalize,
    roots::{isolate_union, roots_in_open_interval},
    solver::solve,
    strict_bezout_projection::ProjectionBezoutHint,
    verify::verify,
};

const P: &str = "y^2+y-x";
// Q = P + (x+y)^2: the only common zeros are (0,0) and (2,-2), outside the
// open quadrant, but Res_y(P,Q) = P(x,-x)^2 = (x^2-2x)^2 has the positive
// base root x = 2, which no other projection obligation produces.
const Q: &str = "2*y^2+2*x*y+y+x^2-x";

fn square_hint() -> ProjectionBezoutHint {
    ProjectionBezoutHint {
        schema_version: 1,
        polynomials: [P.into(), Q.into()],
        multipliers: ["-1".into(), "1".into()],
        square: Some("x+y".into()),
        positive: None,
        multiplier: "1".into(),
    }
}

fn positive_hint() -> ProjectionBezoutHint {
    // 2*(Q-P) = 2*(x+y)^2, with S itself certified positive.
    ProjectionBezoutHint {
        multipliers: ["-2".into(), "2".into()],
        square: None,
        positive: Some("x^2+2*x*y+y^2".into()),
        multiplier: "2".into(),
        ..square_hint()
    }
}

fn problem(hints: Vec<ProjectionBezoutHint>, prune_operations: bool) -> Problem {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "y>0".into()],
        split: vec![P.into(), Q.into()],
        ..Default::default()
    };
    p.solver.engine = "optimized".into();
    p.solver.positivity = true;
    p.solver.prune_operations = prune_operations;
    p.solver.ordering = "given".into();
    p.solver.order = vec!["x".into(), "y".into()];
    p.solver.projection_strict_bezout_hints = hints;
    p.limits.wall_time_secs = 60.0;
    p
}

/// The accepted cell (with its split signs) containing a generic point. Each
/// trace root is a (polynomial, real-root index) section, re-isolated exactly
/// at the point itself; the sector is the number of sections below the point.
/// This follows the cells' own cylindrical description over their base cell.
/// The union's root count is not used: with an omitted pair resultant, two
/// sections may meet outside the domain (for example at a stack sample).
/// None: rejected or on a boundary.
fn locate(result: &SolveResult, point: &[Rational]) -> Option<Vec<i8>> {
    let normalized = normalize(&result.problem, &result.order).unwrap();
    let algebra: &Algebra = &normalized.algebra;
    let polynomials: Vec<Poly> = result
        .polynomials
        .iter()
        .map(|p| algebra.parse(p).unwrap())
        .collect();
    let mut node = result.trace.as_ref().unwrap();
    let mut prefix = Vec::new();
    loop {
        match node {
            LiftTrace::Rejected { .. } => return None,
            LiftTrace::Accepted { cell } => return Some(result.cells[*cell].signs.clone()),
            LiftTrace::Unfinished { .. } => panic!("unfinished trace"),
            LiftTrace::Stack { roots, sectors } => {
                let level = prefix.len();
                let t = &point[level];
                let mut below = Vec::new();
                for bound in roots {
                    let fiber = algebra
                        .specialize_univariate(&polynomials[bound.polynomial], level, &prefix)
                        .unwrap();
                    let root = isolate_union(vec![(bound.polynomial, fiber)])
                        .unwrap()
                        .into_iter()
                        .find(|r| r.index == bound.index)
                        .expect("a delineable section exists over its base cell");
                    let (lo, hi) = &root.interval;
                    below.push(if lo == hi {
                        if lo == t {
                            return None;
                        }
                        lo < t
                    } else if hi <= t {
                        true
                    } else if lo < t {
                        if root.fiber.evaluate(t).is_zero() {
                            return None;
                        }
                        roots_in_open_interval(&root.fiber, lo, t) == 1
                    } else {
                        false
                    });
                }
                let index = below.iter().filter(|b| **b).count();
                // In the open domain the sections below the point must be a
                // prefix of the stack order; outside it they may be permuted.
                if !below[..index].iter().all(|b| *b) {
                    assert!(point[level] <= Rational::zero(), "{point:?}");
                    return None;
                }
                node = &sectors[index].child;
                prefix.push(t.clone());
            }
        }
    }
}

/// The exported table spelling of a canonical polynomial.
fn formatted(result: &SolveResult, text: &str) -> String {
    let normalized = normalize(&result.problem, &result.order).unwrap();
    let algebra = &normalized.algebra;
    algebra.format(&symgcad::algebra::canonical(&algebra.parse(text).unwrap()))
}

fn grid() -> Vec<Vec<Rational>> {
    let mut points = Vec::new();
    for i in 0..23 {
        for j in 0..19 {
            points.push(vec![
                Rational::from((2 * i + 1) as i64) / Rational::from(13),
                Rational::from((2 * j + 1) as i64) / Rational::from(17),
            ]);
        }
    }
    points
}

#[test]
fn omitted_resultant_is_replayed_and_membership_matches_the_baseline() {
    for prune_operations in [false, true] {
        let baseline = solve(&problem(vec![], prune_operations)).unwrap();
        assert_eq!(baseline.status, Status::CompleteGeneric);
        assert!(verify(&baseline).unwrap().verified);
        assert!(baseline.polynomials.contains(&formatted(&baseline, "x-2")));
        for hint in [square_hint(), positive_hint()] {
            let omitted = solve(&problem(vec![hint], prune_operations)).unwrap();
            assert_eq!(omitted.status, Status::CompleteGeneric);
            assert!(verify(&omitted).unwrap().verified);
            let report = omitted.statistics.optimized_projection.as_ref().unwrap();
            assert_eq!(report.pair_omission_hints_accepted, 1);
            assert_eq!(report.pair_omissions.len(), 1);
            assert_eq!(report.pair_omissions[0].level, 0);
            assert!(report.cache.omitted_pair_resultants >= 1);
            // Only the pair obligation disappears; P and Q themselves stay.
            assert!(!omitted.polynomials.contains(&formatted(&omitted, "x-2")));
            assert!(omitted.polynomials.len() < baseline.polynomials.len());
            assert!(omitted.cells.len() < baseline.cells.len());
            for p in [P, Q] {
                assert!(omitted.polynomials.contains(&formatted(&omitted, p)), "{p}");
            }
            for point in grid() {
                assert_eq!(
                    locate(&baseline, &point),
                    locate(&omitted, &point),
                    "{point:?}"
                );
            }
        }
    }
}

#[test]
fn unmatched_and_rejected_hints_leave_the_baseline_unchanged() {
    let baseline = solve(&problem(vec![], false)).unwrap();
    // A valid identity for a pair absent from the projection basis.
    let unmatched = ProjectionBezoutHint {
        polynomials: [P.into(), "5*y^2+4*x*y+y+x^2-x".into()],
        multipliers: ["-1".into(), "1".into()],
        square: Some("x+2*y".into()),
        ..square_hint()
    };
    let mut altered = square_hint();
    altered.multipliers[0] = "-3".into();
    let mut mixed = square_hint();
    mixed.polynomials[1] = "2*y^2-2*x*y+y+x^2-x".into();
    mixed.square = Some("x-y".into());
    for (hint, accepted) in [(unmatched, 1), (altered, 0), (mixed, 0)] {
        let result = solve(&problem(vec![hint], false)).unwrap();
        let report = result.statistics.optimized_projection.as_ref().unwrap();
        assert_eq!(report.pair_omission_hints_accepted, accepted);
        assert_eq!(report.pair_omission_hints_rejected, 1 - accepted);
        assert!(report.pair_omissions.is_empty());
        assert_eq!(result.polynomials, baseline.polynomials);
        assert_eq!(result.projection_levels, baseline.projection_levels);
        assert_eq!(result.cells, baseline.cells);
        assert!(verify(&result).unwrap().verified);
    }
    // Hints require certified projection.
    let mut uncertified = problem(vec![square_hint()], false);
    uncertified.solver.positivity = false;
    assert!(uncertified.validate().is_err());
}

#[test]
fn forged_or_mutated_omission_evidence_is_rejected() {
    let result = solve(&problem(vec![square_hint()], false)).unwrap();
    assert!(verify(&result).unwrap().verified);
    let mutate = |change: &dyn Fn(&mut SolveResult)| {
        let mut forged = result.clone();
        change(&mut forged);
        forged
    };
    let evidence = |r: &mut SolveResult| {
        r.statistics
            .optimized_projection
            .as_mut()
            .unwrap()
            .pair_omissions[0]
            .clone()
    };
    let set = |r: &mut SolveResult, e| {
        r.statistics
            .optimized_projection
            .as_mut()
            .unwrap()
            .pair_omissions[0] = e;
    };
    let cases: Vec<(&str, SolveResult)> = vec![
        (
            "missing evidence",
            mutate(&|r| {
                r.statistics
                    .optimized_projection
                    .as_mut()
                    .unwrap()
                    .pair_omissions
                    .clear()
            }),
        ),
        (
            "altered multiplier",
            mutate(&|r| {
                let mut e = evidence(r);
                e.hint.multipliers[0] = "-2".into();
                set(r, e)
            }),
        ),
        (
            "altered constant",
            mutate(&|r| {
                let mut e = evidence(r);
                e.hint.multiplier = "3".into();
                set(r, e)
            }),
        ),
        (
            "non-positive kernel",
            mutate(&|r| {
                let mut e = evidence(r);
                e.hint.square = None;
                e.hint.positive = Some("x^2+2*x*y+y^2-1".into());
                set(r, e)
            }),
        ),
        (
            "certificate sign",
            mutate(&|r| {
                let mut e = evidence(r);
                e.certificate.sign = -e.certificate.sign;
                set(r, e)
            }),
        ),
        (
            "certificate polynomial",
            mutate(&|r| {
                let mut e = evidence(r);
                e.certificate.polynomial = "x+y+1".into();
                set(r, e)
            }),
        ),
        (
            "wrong pair",
            mutate(&|r| {
                let mut e = evidence(r);
                e.polynomials[1] = "y".into();
                set(r, e)
            }),
        ),
        (
            "wrong level",
            mutate(&|r| {
                let mut e = evidence(r);
                e.level = 1;
                set(r, e)
            }),
        ),
        (
            "wrong variable",
            mutate(&|r| {
                let mut e = evidence(r);
                e.eliminated_variable = 0;
                set(r, e)
            }),
        ),
        (
            "duplicate",
            mutate(&|r| {
                let e = evidence(r);
                r.statistics
                    .optimized_projection
                    .as_mut()
                    .unwrap()
                    .pair_omissions
                    .push(e)
            }),
        ),
        (
            "uncertified mode",
            mutate(&|r| r.problem.solver.positivity = false),
        ),
    ];
    for (name, forged) in cases {
        assert!(verify(&forged).is_err(), "{name}");
    }
}
