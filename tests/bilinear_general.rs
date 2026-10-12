use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use symgcad::{
    Limits, Problem,
    algebra::Algebra,
    benchmark::run_bounded,
    bilinear_general::{
        self, Atom, Condition, EmptyClaim, Execution, PieceSpec, Separator, SignPremise, fiber_rows,
    },
    comparison::compare_generic,
    output::{LiftTrace, SolveResult, Status},
    solver::{selected_order, solve},
    verify::verify,
};

fn problem(variables: &[&str], target: &str, guide: Option<&str>) -> Problem {
    let mut constraints: Vec<String> = variables.iter().map(|v| format!("{v}>0")).collect();
    constraints.push(format!("({target})>0"));
    let mut p = Problem {
        variables: variables.iter().map(|v| v.to_string()).collect(),
        constraints,
        ..Default::default()
    };
    p.solver.engine = "bilinear_general".into();
    p.solver.ordering = "given".into();
    p.solver.bilinear_general_guide = guide.map(str::to_owned);
    p.limits.wall_time_secs = 60.;
    p
}

fn checked(p: &Problem) -> SolveResult {
    let r = solve(p).unwrap();
    assert_eq!(r.status, Status::CompleteGeneric, "{:?}", r.diagnostic);
    assert!(verify(&r).unwrap().verified);
    // Sequential timings: every piece ran inside this solve.
    let timings = r.statistics.bilinear_general_pieces.as_ref().unwrap();
    let bases = &r.bilinear_general.as_ref().unwrap().bases;
    assert_eq!(timings.execution, Execution::Sequential);
    assert_eq!(
        timings.piece_seconds,
        bases
            .iter()
            .map(|b| b.statistics.total_seconds)
            .collect::<Vec<_>>()
    );
    assert!(r.statistics.total_seconds >= timings.piece_seconds_sum);
    assert!(r.statistics.total_seconds >= timings.planning_seconds);
    assert!(r.statistics.ordering_seconds >= timings.planning_seconds);
    assert_eq!(selected_order(p).unwrap(), r.order);
    assert!(r.bilinear_general.is_some());
    assert!(r.trace.is_none() && r.local.is_none() && r.homogeneous.is_none());
    let roundtrip: SolveResult = serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();
    assert!(verify(&roundtrip).unwrap().verified);
    r
}

fn compare_baseline(p: &Problem, r: &SolveResult) {
    let mut baseline = p.clone();
    baseline.solver.engine = "baseline".into();
    baseline.solver.bilinear_general_guide = None;
    baseline.solver.bilinear_general_base_anchor = None;
    baseline.solver.bilinear_general_base_block.clear();
    let reference = solve(&baseline).unwrap();
    assert!(verify(&reference).unwrap().verified);
    let comparison = compare_generic(r, &reference, &p.limits).unwrap();
    assert_eq!(
        comparison.equivalent,
        Some(true),
        "{:?}",
        comparison.counterexample
    );
}

fn rejected(r: &SolveResult) {
    assert!(verify(r).map_or(true, |v| !v.verified));
}

fn cond(atom: Atom, sign: i8) -> Condition {
    Condition { atom, sign }
}

fn toy() -> Problem {
    problem(
        &["t", "y", "x"],
        "(t-3)*x*y+(t-1)*x+(t-2)*y+t",
        Some("(x+1)*(y+1)"),
    )
}

fn has_exceptional(r: &SolveResult, expression: &str) -> bool {
    let algebra = Algebra::new(&r.order).unwrap();
    let wanted = algebra.parse(expression).unwrap();
    r.exceptional_polynomials.iter().any(|&id| {
        let p = algebra.parse(&r.polynomials[id]).unwrap();
        p == wanted || p == -wanted.clone()
    })
}

#[test]
fn all_sixteen_constant_sign_patterns_match_the_baseline() {
    for mask in 0..16u32 {
        let s: [i64; 4] = std::array::from_fn(|i| if mask >> i & 1 == 1 { 1 } else { -1 });
        let both = s[0] * s[1] < 0 && s[2] * s[3] < 0;
        let magnitudes: &[[i64; 4]] = if both {
            &[[1, 2, 3, 5], [1, 3, 2, 7]]
        } else {
            &[[1, 2, 3, 5]]
        };
        for m in magnitudes {
            let k: [i64; 4] = std::array::from_fn(|i| s[i] * m[i]);
            let target = format!("({})*x*y+({})*x+({})*y+({})", k[0], k[1], k[2], k[3]);
            let p = problem(&["y", "x"], &target, None);
            let r = checked(&p);
            let delta = k[1] * k[2] - k[0] * k[3];
            let expected = fiber_rows(
                [s[0] as i8, s[1] as i8, s[2] as i8, s[3] as i8],
                delta.signum() as i8,
            )
            .unwrap()
            .len();
            assert_eq!(r.cells.len(), expected, "{target}");
            let plan = &r.bilinear_general.as_ref().unwrap().plan;
            assert!(plan.pieces.iter().all(|p| p.conditions.is_empty()));
            assert_eq!(plan.pieces.len() + plan.empty.len(), 1);
            match &plan.separator {
                Separator::Square { .. } => assert!(delta > 0, "{target}"),
                Separator::NegativeSquare { .. } => assert!(delta < 0, "{target}"),
                Separator::Delta { .. } => panic!("a constant Delta is a constant square"),
            }
            compare_baseline(&p, &r);
        }
    }
}

#[test]
fn guided_one_parameter_toy_uses_ratio_premises_and_matches_every_base_engine() {
    for engine in ["baseline", "optimized", "local"] {
        let mut p = toy();
        p.solver.bilinear_general_base_engine = engine.into();
        let r = checked(&p);
        let evidence = r.bilinear_general.as_ref().unwrap();
        let ratio = evidence
            .plan
            .premises
            .iter()
            .filter(|q| matches!(q, SignPremise::RatioOrder { .. }))
            .count();
        assert_eq!(ratio, 6);
        assert_eq!(
            evidence.plan.pieces,
            vec![
                PieceSpec {
                    conditions: vec![cond(Atom::C, 1)],
                    split: vec![Atom::A]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::C, -1)],
                    split: vec![Atom::B]
                },
            ]
        );
        assert!(evidence.plan.empty.is_empty());
        for (base, spec) in evidence.bases.iter().zip(&evidence.plan.pieces) {
            assert_eq!(base.problem.solver.engine, engine);
            assert_eq!(base.problem.variables, vec!["t"]);
            assert_eq!(base.problem.constraints.len(), 2);
            assert_eq!(base.problem.split.len(), spec.split.len());
        }
        for seam in ["t-1", "t-2", "t-3"] {
            assert!(has_exceptional(&r, seam), "{seam} missing");
        }
        compare_baseline(&p, &r);
    }
}

#[test]
fn gluza_shaped_synthetic_chain_solves_four_pieces_and_an_empty_claim() {
    // a=p, b=p+s^2, c=p+r^2, d=p+s^2+r^2 with U=(x+1)*(y+1): exactly the
    // corner chain a<=b<=d, a<=c<=d, and Delta=(s*r)^2 with p=u-3, s=v-1,
    // r=u-v. Every atom changes sign on the positive quadrant.
    let target = "(u-3)*x*y+(u-3+(v-1)^2)*x+(u-3+(u-v)^2)*y+(u-3+(v-1)^2+(u-v)^2)";
    for engine in ["optimized", "local"] {
        let mut p = problem(&["u", "v", "y", "x"], target, Some("(x+1)*(y+1)"));
        p.solver.bilinear_general_base_engine = engine.into();
        let r = checked(&p);
        let plan = &r.bilinear_general.as_ref().unwrap().plan;
        assert_eq!(
            plan.pieces,
            vec![
                PieceSpec {
                    conditions: vec![cond(Atom::B, 1), cond(Atom::C, 1)],
                    split: vec![Atom::A]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::B, 1), cond(Atom::C, -1)],
                    split: vec![Atom::G]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::B, -1), cond(Atom::C, 1)],
                    split: vec![]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::B, -1), cond(Atom::C, -1), cond(Atom::D, 1)],
                    split: vec![]
                },
            ]
        );
        assert_eq!(plan.empty.len(), 1);
        let bases = &r.bilinear_general.as_ref().unwrap().bases;
        assert!(bases.iter().all(|b| !b.cells.is_empty()));
        for seam in ["u-3", "u-3+(v-1)^2", "u-3+(u-v)^2", "u-3+(v-1)^2+(u-v)^2"] {
            assert!(has_exceptional(&r, seam), "{seam} missing");
        }
        compare_baseline(&p, &r);
        // Without the guide the same set is decomposed by more pieces.
        let mut unguided = p.clone();
        unguided.solver.bilinear_general_guide = None;
        let plain = checked(&unguided);
        assert!(plain.bilinear_general.as_ref().unwrap().plan.pieces.len() > 4);
        assert_eq!(
            compare_generic(&r, &plain, &p.limits).unwrap().equivalent,
            Some(true)
        );
    }
}

#[test]
fn separator_pinch_is_a_retained_seam_in_square_and_delta_modes() {
    // a=-1, b=s, c=-t, d=s^2-s*t+t^2: Delta=(s-t)^2 pinches both breakpoints.
    let p = problem(&["s", "t", "y", "x"], "-x*y+s*x-t*y+s^2-s*t+t^2", None);
    let r = checked(&p);
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    let Separator::Square { multiplier, square } = &plan.separator else {
        panic!("expected a square separator")
    };
    assert_eq!(multiplier, "1");
    let algebra = Algebra::new(&r.order).unwrap();
    let g = algebra.parse(square).unwrap();
    assert!(g == algebra.parse("s-t").unwrap() || g == algebra.parse("t-s").unwrap());
    assert_eq!(
        plan.pieces,
        vec![PieceSpec {
            conditions: vec![],
            split: vec![Atom::G]
        }]
    );
    assert!(has_exceptional(&r, "s-t"));
    assert_eq!(r.cells.len(), 4);
    compare_baseline(&p, &r);

    // A non-square, sign-changing Delta=s+t-s*t is used directly.
    let p = problem(&["s", "t", "y", "x"], "-x*y+s*x-t*y+s+t", None);
    let r = checked(&p);
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    assert!(matches!(plan.separator, Separator::Delta { .. }));
    assert_eq!(plan.pieces[0].split, vec![Atom::G]);
    assert!(has_exceptional(&r, "s+t-s*t"));
    compare_baseline(&p, &r);
}

#[test]
fn inner_homogeneous_anchor_matches_plain_pieces_and_baseline() {
    let target = "(t-3*s)*x*y+s*(t-s)*x+s*(t-2*s)*y+s^2*t";
    let mut anchored = problem(&["s", "t", "y", "x"], target, Some("(x+s)*(y+s)"));
    anchored.solver.bilinear_general_base_anchor = Some("s".into());
    let r = checked(&anchored);
    let evidence = r.bilinear_general.as_ref().unwrap();
    // Corners (1,s,s,s^2) certify only the four Dodgson-like orders
    // a<=b, a<=c, b<=d, c<=d (a-d and b-c give odd powers of s), so this is
    // the Gluza33 chain with d>0 and G=s^2>0 globally certified.
    let ratios = evidence
        .plan
        .premises
        .iter()
        .filter_map(|q| match q {
            SignPremise::RatioOrder { lower, upper, .. } => Some((*lower, *upper)),
            SignPremise::GlobalSign { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ratios,
        vec![
            (Atom::A, Atom::B),
            (Atom::A, Atom::C),
            (Atom::B, Atom::D),
            (Atom::C, Atom::D)
        ]
    );
    // With d>0 known globally, three pieces of region-weighted cost 12 remain.
    assert_eq!(evidence.plan.pieces.len(), 3);
    assert!(evidence.plan.empty.is_empty());
    for base in &evidence.bases {
        assert_eq!(base.problem.solver.homogeneous_anchor.as_deref(), Some("s"));
        assert_eq!(base.problem.solver.homogeneous_block, vec!["s", "t"]);
        assert!(base.homogeneous.is_some());
    }
    compare_baseline(&anchored, &r);
    let mut plain = anchored.clone();
    plain.solver.bilinear_general_base_anchor = None;
    let unanchored = checked(&plain);
    assert!(
        unanchored
            .bilinear_general
            .as_ref()
            .unwrap()
            .bases
            .iter()
            .all(|b| b.homogeneous.is_none())
    );
    assert_eq!(
        compare_generic(&r, &unanchored, &plain.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
    // The anchor must be the first base variable.
    let mut late = anchored;
    late.solver.bilinear_general_base_anchor = Some("t".into());
    assert!(solve(&late).is_err());
}

#[test]
fn premise_piece_empty_status_and_cell_forgeries_are_rejected() {
    let r = checked(&toy());
    let original = serde_json::to_value(&r).unwrap();
    let premises = &r.bilinear_general.as_ref().unwrap().plan.premises;
    let ratio = premises
        .iter()
        .position(|q| matches!(q, SignPremise::RatioOrder { .. }))
        .unwrap();
    let global = premises
        .iter()
        .position(|q| matches!(q, SignPremise::GlobalSign { .. }))
        .unwrap();
    let SignPremise::RatioOrder { lower, upper, .. } = &premises[ratio] else {
        unreachable!()
    };
    let swapped = json!(upper);
    let other = json!(lower);
    let pr = |field: &str| format!("/bilinear_general/plan/premises/{ratio}/{field}");
    let pg = |field: &str| format!("/bilinear_general/plan/premises/{global}/{field}");
    let mutations: Vec<(String, serde_json::Value)> = vec![
        ("/bilinear_general/plan/target_constraint".into(), json!(0)),
        ("/bilinear_general/plan/schema_version".into(), json!(2)),
        ("/bilinear_general/plan/coefficients/0".into(), json!("t-4")),
        ("/bilinear_general/plan/guide".into(), json!("(x+2)*(y+1)")),
        ("/bilinear_general/plan/guide".into(), json!(null)),
        (
            "/bilinear_general/plan/guide_positivity/0/certificate/sign".into(),
            json!(-1),
        ),
        (
            "/bilinear_general/plan/guide_positivity/0/corner".into(),
            json!("g"),
        ),
        (pr("lower"), swapped.clone()),
        (pr("upper"), other),
        (pr("multiplier"), json!("-1")),
        (pr("multiplier"), json!("0")),
        (pr("multiplier"), json!("7")),
        (pr("square"), json!("t")),
        (pr("square"), json!("y")),
        (pg("atom"), json!("a")),
        (pg("certificate/sign"), json!(-1)),
        (
            "/bilinear_general/plan/separator/multiplier".into(),
            json!("1"),
        ),
        ("/bilinear_general/plan/separator/square".into(), json!("t")),
        (
            "/bilinear_general/plan/pieces/0/conditions/0/sign".into(),
            json!(-1),
        ),
        (
            "/bilinear_general/plan/pieces/0/conditions".into(),
            json!([]),
        ),
        (
            "/bilinear_general/plan/pieces/1/conditions/0/atom".into(),
            json!("b"),
        ),
        ("/bilinear_general/plan/pieces/0/split".into(), json!([])),
        ("/bilinear_general/plan/pieces/1/split".into(), json!(["d"])),
        (
            "/bilinear_general/plan/empty".into(),
            json!([{"conditions": [{"atom": "c", "sign": 1}]}]),
        ),
        (
            "/bilinear_general/plan/empty".into(),
            json!([{"conditions": [{"atom": "a", "sign": -1}]}]),
        ),
        (
            "/bilinear_general/bases/0/status".into(),
            json!("incomplete"),
        ),
        (
            "/bilinear_general/bases/0/problem/constraints/1".into(),
            json!("t>1"),
        ),
        (
            "/bilinear_general/bases/1/problem/split/0".into(),
            json!("t-2"),
        ),
        (
            "/bilinear_general/bases/0/problem/limits/wall_time_secs".into(),
            json!(1e9),
        ),
        (
            "/bilinear_general/bases/0/problem/solver/engine".into(),
            json!("baseline"),
        ),
        ("/cells/0/axes/1/sample".into(), json!("0")),
        ("/cells/0/axes/2/lower/index".into(), json!(1)),
        ("/cells/0/axes/0/sample".into(), json!("123456789")),
        ("/cells/1/axes/2/sample".into(), json!("1/1000000007")),
        ("/exceptional_polynomials".into(), json!([])),
        ("/polynomials/0".into(), json!("1")),
        ("/statistics/raw_cells".into(), json!(0)),
        ("/status".into(), json!("incomplete")),
        ("/order/0".into(), json!("y")),
    ];
    for (path, replacement) in mutations {
        let mut forged = original.clone();
        *forged
            .pointer_mut(&path)
            .unwrap_or_else(|| panic!("missing evidence field {path}")) = replacement;
        let forged: SolveResult = serde_json::from_value(forged).unwrap();
        assert!(verify(&forged).is_err(), "accepted modification at {path}");
    }

    let mut bad = r.clone();
    bad.cells.pop();
    rejected(&bad);
    let mut bad = r.clone();
    bad.cells.push(bad.cells[0].clone());
    rejected(&bad);
    let mut bad = r.clone();
    bad.cells.swap(0, 1);
    rejected(&bad);
    let mut bad = r.clone();
    bad.exceptional_polynomials.pop();
    rejected(&bad);
    let mut bad = r.clone();
    bad.trace = Some(LiftTrace::Accepted { cell: 0 });
    rejected(&bad);
    let mut bad = r.clone();
    bad.merged_cells = Some(vec![]);
    rejected(&bad);
    let mut bad = r.clone();
    bad.bilinear_general.as_mut().unwrap().bases.reverse();
    rejected(&bad);
    let mut bad = r.clone();
    bad.bilinear_general.as_mut().unwrap().plan.pieces.reverse();
    rejected(&bad);
    let mut bad = r.clone();
    bad.bilinear_general.as_mut().unwrap().bases.pop();
    rejected(&bad);
    let mut bad = r.clone();
    bad.bilinear_general
        .as_mut()
        .unwrap()
        .plan
        .empty
        .push(EmptyClaim { conditions: vec![] });
    rejected(&bad);
    let mut bad = r.clone();
    bad.bilinear_general.as_mut().unwrap().plan.premises.clear();
    rejected(&bad);
    let mut bad = r.clone();
    bad.bilinear_general.as_mut().unwrap().bases[0]
        .cells
        .clear();
    rejected(&bad);
    let mut bad = r.clone();
    let nested = r.bilinear_general.clone();
    bad.bilinear_general.as_mut().unwrap().bases[0].bilinear_general = nested;
    rejected(&bad);
    let mut bad = r.clone();
    bad.local = r.bilinear_general.as_ref().unwrap().bases[0].local.clone();
    bad.homogeneous = Some(Box::new(symgcad::homogeneity::HomogeneousEvidence {
        anchor: "t".into(),
        block: vec![],
        degrees: vec![],
        forward_ratios: vec![],
        inverse_coordinates: vec![],
        reduced: Box::new(r.clone()),
    }));
    rejected(&bad);
    // Removing a piece and its result leaves sign vectors unclaimed.
    let mut bad = r.clone();
    let evidence = bad.bilinear_general.as_mut().unwrap();
    evidence.plan.pieces.pop();
    evidence.bases.pop();
    rejected(&bad);
}

#[test]
fn incomplete_pieces_cell_and_wall_limits_never_claim_completion() {
    let mut p = toy();
    p.limits.max_cells = Some(1);
    let r = solve(&p).unwrap();
    assert_eq!(r.status, Status::Incomplete);
    assert!(r.cells.len() <= 1);
    rejected(&r);
    let mut p = toy();
    p.limits.wall_time_secs = 1e-12;
    let r = solve(&p).unwrap();
    assert_eq!(r.status, Status::Incomplete);
    rejected(&r);
    let mut p = toy();
    p.solver.bilinear_general_base_engine = "local".into();
    p.solver.local_max_nodes = Some(1);
    let r = solve(&p).unwrap();
    assert_eq!(r.status, Status::Incomplete);
    assert!(r.cells.is_empty());
    rejected(&r);
}

#[test]
fn degenerate_or_unsupported_inputs_decline() {
    // a identically zero.
    assert!(solve(&problem(&["y", "x"], "x+y-1", None)).is_err());
    // Delta=b*c-a*d identically zero: F=(x-1)*(y+2).
    assert!(solve(&problem(&["y", "x"], "(x-1)*(y+2)", None)).is_err());
    // Not affine in the last axis, split, outer anchor, extra fiber constraint.
    assert!(solve(&problem(&["y", "x"], "x^2*y-1", None)).is_err());
    let mut p = toy();
    p.split.push("t-1".into());
    assert!(solve(&p).is_err());
    let mut p = toy();
    p.solver.homogeneous_anchor = Some("t".into());
    p.solver.homogeneous_block = p.variables.clone();
    assert!(solve(&p).is_err());
    let mut p = toy();
    p.constraints.push("y<2".into());
    assert!(solve(&p).is_err());
    let mut p = toy();
    p.constraints.retain(|c| c != "x>0");
    assert!(solve(&p).is_err());
    // A base-only extra constraint is retained in every piece.
    let mut p = toy();
    p.constraints.push("t<5/2".into());
    let r = checked(&p);
    compare_baseline(&p, &r);
}

#[test]
fn an_unrelated_guide_proves_nothing_and_still_matches_baseline() {
    // The guide x*y+1 has corner coefficients 1,0,0,1: the zero corners are
    // never certified, so only the exact a<=d comparison survives.
    let mut p = toy();
    p.solver.bilinear_general_guide = Some("x*y+1".into());
    let r = checked(&p);
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    assert_eq!(plan.guide_positivity.len(), 2);
    assert!(plan.premises.iter().all(|q| match q {
        SignPremise::RatioOrder { lower, upper, .. } => (*lower, *upper) == (Atom::A, Atom::D),
        SignPremise::GlobalSign { .. } => true,
    }));
    compare_baseline(&p, &r);
    // A guide that is not affine in the fiber axes is refused before planning.
    let mut p = toy();
    p.solver.bilinear_general_guide = Some("(x+1)^2*(y+1)".into());
    assert!(solve(&p).is_err());
    let mut p = toy();
    p.solver.bilinear_general_guide = None;
    let r = checked(&p);
    compare_baseline(&p, &r);
}

fn gluza33(order: &[&str]) -> Problem {
    let mut p = Problem::from_toml(include_str!("fixtures/gluza33_positive.toml")).unwrap();
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../reference/gluza33_rayleigh_guide.json")).unwrap();
    p.solver.engine = "bilinear_general".into();
    p.solver.ordering = "given".into();
    p.solver.order = order.iter().map(|v| v.to_string()).collect();
    // The guide is the first Symanzik polynomial U=det(M) of the loop matrix
    // in reference/GLUZA33_RAYLEIGH_REDUCTION.md: input data, not dispatch.
    p.solver.bilinear_general_guide = Some(reference["guide"].as_str().unwrap().into());
    p.solver.bilinear_general_base_engine = "local".into();
    p.solver.bilinear_general_base_anchor = Some(order[0].into());
    p.limits.wall_time_secs = 60.;
    p
}

#[test]
fn gluza33_plan_has_four_square_premises_and_four_sign_pieces() {
    for order in [
        ["x4", "x7", "x2", "x1", "x0", "x6", "x3", "x5"],
        ["x4", "x7", "x2", "x1", "x0", "x6", "x5", "x3"],
    ] {
        let p = gluza33(&order);
        let planned = bilinear_general::plan(&p).unwrap();
        let plan = &planned.plan;
        let algebra = Algebra::new(&plan.order).unwrap();
        assert_eq!(plan.coefficients.len(), 4);
        // a is the bubble A^2-8AB+B^2 with A=x0+x2+x7, B=x1+x4+x6.
        assert_eq!(
            algebra.parse(&plan.coefficients[0]).unwrap(),
            algebra
                .parse("(x0+x2+x7)^2-8*(x0+x2+x7)*(x1+x4+x6)+(x1+x4+x6)^2")
                .unwrap()
        );
        let Separator::Square { multiplier, square } = &plan.separator else {
            panic!("Gluza33 Delta must be a square")
        };
        assert_eq!(multiplier, "1");
        assert_eq!(algebra.parse(square).unwrap().nterms(), 26);
        assert_eq!(plan.guide_positivity.len(), 4);
        let mut ratios = plan
            .premises
            .iter()
            .map(|q| match q {
                SignPremise::RatioOrder {
                    lower,
                    upper,
                    multiplier,
                    square,
                    strict,
                } => {
                    assert_eq!(multiplier, "10");
                    (
                        *lower,
                        *upper,
                        algebra.parse(square).unwrap().nterms(),
                        strict.is_some(),
                    )
                }
                SignPremise::GlobalSign { .. } => panic!("no Gluza33 atom has a global sign"),
            })
            .collect::<Vec<_>>();
        ratios.sort();
        assert_eq!(
            ratios.iter().map(|r| (r.0, r.1)).collect::<Vec<_>>(),
            vec![
                (Atom::A, Atom::B),
                (Atom::A, Atom::C),
                (Atom::B, Atom::D),
                (Atom::C, Atom::D)
            ]
        );
        let mut sizes = ratios.iter().map(|r| r.2).collect::<Vec<_>>();
        sizes.sort();
        assert_eq!(sizes, vec![5, 5, 6, 6]);
        // The b<=d and c<=d witnesses have positive coefficients.
        assert_eq!(
            ratios.iter().map(|r| r.3).collect::<Vec<_>>(),
            vec![false, false, true, true]
        );
        assert_eq!(
            plan.pieces,
            vec![
                PieceSpec {
                    conditions: vec![cond(Atom::B, 1), cond(Atom::C, 1)],
                    split: vec![Atom::A]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::B, 1), cond(Atom::C, -1)],
                    split: vec![Atom::G]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::B, -1), cond(Atom::C, 1)],
                    split: vec![]
                },
                PieceSpec {
                    conditions: vec![cond(Atom::B, -1), cond(Atom::C, -1), cond(Atom::D, 1)],
                    split: vec![]
                },
            ]
        );
        assert_eq!(
            plan.empty,
            vec![EmptyClaim {
                conditions: vec![cond(Atom::B, -1), cond(Atom::C, -1), cond(Atom::D, -1)]
            }]
        );
        assert_eq!(planned.problems.len(), 4);
        for piece in &planned.problems {
            assert_eq!(piece.variables, order[..6]);
            assert_eq!(piece.solver.order, order[..6]);
            assert_eq!(piece.solver.engine, "local");
            assert_eq!(piece.solver.homogeneous_anchor.as_deref(), Some("x4"));
            assert_eq!(piece.solver.homogeneous_block, order[..6]);
            assert!(piece.solver.bilinear_general_guide.is_none());
        }
        assert_eq!(
            planned
                .problems
                .iter()
                .map(|p| (p.constraints.len(), p.split.len()))
                .collect::<Vec<_>>(),
            vec![(8, 1), (8, 1), (8, 0), (9, 0)]
        );
        // Planning is replayable and the piece problems are re-derived exactly.
        let replayed = bilinear_general::piece_problems(&p, plan).unwrap();
        assert_eq!(
            serde_json::to_value(&replayed).unwrap(),
            serde_json::to_value(&planned.problems).unwrap()
        );
    }
}

fn run_bounded_cli(arguments: &[&std::ffi::OsStr]) -> symgcad::benchmark::ProcessOutcome {
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command.args(arguments);
    run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 120.0,
            workers: 1,
            memory_mib: 4096,
            ..Default::default()
        },
    )
    .unwrap()
}

fn run(arguments: &[&std::ffi::OsStr]) -> String {
    let process = run_bounded_cli(arguments);
    assert_eq!(process.status, "completed", "{}", process.stderr);
    assert_eq!(process.exit_code, Some(0), "{}", process.stderr);
    process.stdout
}

/// The command must fail, mentioning `expected` on stderr.
fn run_failing(arguments: &[&std::ffi::OsStr], expected: &str) {
    let process = run_bounded_cli(arguments);
    assert_ne!(
        process.exit_code,
        Some(0),
        "unexpected success: {}",
        process.stdout
    );
    assert!(
        process.stderr.contains(expected),
        "expected {expected:?} in {}",
        process.stderr
    );
}

fn scratch() -> PathBuf {
    let directory = PathBuf::from("artifacts/tests").join(format!(
        "bilinear-general-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn os(path: &Path) -> &std::ffi::OsStr {
    path.as_os_str()
}

fn assemble_arguments<'a>(
    input: &'a Path,
    plan: &'a Path,
    results: &'a [PathBuf],
    output: &'a Path,
) -> Vec<&'a std::ffi::OsStr> {
    let mut arguments: Vec<&std::ffi::OsStr> = vec![
        "pieces".as_ref(),
        "assemble".as_ref(),
        os(input),
        "--plan".as_ref(),
        os(plan),
        "--results".as_ref(),
    ];
    arguments.extend(results.iter().map(|r| os(r)));
    arguments.extend(["--output".as_ref(), os(output)]);
    arguments
}

#[test]
fn cli_plans_separate_piece_processes_and_assembles_a_verified_result() {
    let directory = scratch();
    let input = directory.join("input.toml");
    let mut p = toy();
    p.name = Some("toy".into());
    fs::write(&input, toml::to_string(&p).unwrap()).unwrap();
    let pieces = directory.join("pieces");
    // A reused directory: stale piece inputs of an older plan are removed,
    // other files are kept.
    fs::create_dir_all(&pieces).unwrap();
    fs::write(pieces.join("piece_009.toml"), "stale").unwrap();
    fs::write(pieces.join("notes.txt"), "kept").unwrap();
    fs::write(pieces.join("piece_x.toml"), "kept").unwrap();
    let summary: serde_json::Value = serde_json::from_str(&run(&[
        "pieces".as_ref(),
        "plan".as_ref(),
        os(&input),
        "--out-dir".as_ref(),
        os(&pieces),
    ]))
    .unwrap();
    assert_eq!(summary["removed_stale"], json!(["piece_009.toml"]));
    assert!(!pieces.join("piece_009.toml").exists());
    assert!(pieces.join("notes.txt").exists() && pieces.join("piece_x.toml").exists());
    let plan_path = pieces.join("plan.json");
    let plan: serde_json::Value = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    let count = plan["pieces"].as_array().unwrap().len();
    assert_eq!(count, 2);
    assert!(plan["planning_worker"]["wall_seconds"].as_f64().unwrap() > 0.0);
    let mut results = Vec::new();
    // Solve in reverse order: assembly matches results to exact piece problems.
    for i in (0..count).rev() {
        let piece = pieces.join(format!("piece_{i:03}.toml"));
        let output = directory.join(format!("r{i}.json"));
        run(&["solve".as_ref(), os(&piece), "-o".as_ref(), os(&output)]);
        results.push(output);
    }
    let assembled = directory.join("result.json");
    run(&assemble_arguments(
        &input, &plan_path, &results, &assembled,
    ));
    let verification: serde_json::Value =
        serde_json::from_str(&run(&["verify".as_ref(), os(&assembled)])).unwrap();
    assert_eq!(verification["verified"], true);
    let result: SolveResult = serde_json::from_slice(&fs::read(&assembled).unwrap()).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    let sequential = checked(&p);
    assert_eq!(result.cells, sequential.cells);
    assert_eq!(result.polynomials, sequential.polynomials);

    // Separate-process timings are never summed into the assembly's wall time.
    // (JSON float parsing is not bit-exact, hence the tolerance.)
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * (1.0 + a.abs().max(b.abs()));
    let timings = result.statistics.bilinear_general_pieces.as_ref().unwrap();
    let bases = &result.bilinear_general.as_ref().unwrap().bases;
    assert_eq!(timings.execution, Execution::SeparateProcesses);
    assert!(close(
        timings.planning_seconds,
        plan["planning_seconds"].as_f64().unwrap()
    ));
    assert_eq!(timings.piece_seconds.len(), bases.len());
    for (reported, base) in timings.piece_seconds.iter().zip(bases) {
        assert!(close(*reported, base.statistics.total_seconds));
    }
    assert!(close(
        timings.piece_seconds_sum,
        timings.piece_seconds.iter().sum::<f64>()
    ));
    assert!(close(
        timings.piece_seconds_max,
        timings.piece_seconds.iter().copied().fold(0.0, f64::max)
    ));
    assert!(close(
        result.statistics.total_seconds,
        timings.assembly_seconds
    ));
    assert_eq!(result.statistics.ordering_seconds, 0.0);
    assert_eq!(result.statistics.projection_seconds, 0.0);

    // Duplicate, missing and foreign piece results are refused.
    let duplicate = vec![results[0].clone(), results[0].clone()];
    run_failing(
        &assemble_arguments(&input, &plan_path, &duplicate, &directory.join("dup.json")),
        "matches no remaining piece problem",
    );
    run_failing(
        &assemble_arguments(
            &input,
            &plan_path,
            &results[..1],
            &directory.join("miss.json"),
        ),
        "expected 2 piece results, got 1",
    );
    let foreign = directory.join("foreign.json");
    fs::write(&foreign, serde_json::to_vec(&sequential).unwrap()).unwrap();
    let mixed = vec![results[0].clone(), foreign];
    run_failing(
        &assemble_arguments(
            &input,
            &plan_path,
            &mixed,
            &directory.join("foreign_out.json"),
        ),
        "matches no remaining piece problem",
    );
    assert!(!directory.join("dup.json").exists() && !directory.join("miss.json").exists());
    // A plan file with a forged planning time is refused.
    let mut forged = plan.clone();
    forged["planning_seconds"] = json!(-1.0);
    let forged_path = directory.join("forged_plan.json");
    fs::write(&forged_path, serde_json::to_vec(&forged).unwrap()).unwrap();
    run_failing(
        &assemble_arguments(&input, &forged_path, &results, &directory.join("f.json")),
        "planning_seconds",
    );
    // The planning worker honours the input's own deadline.
    let mut hurried = p.clone();
    hurried.limits.wall_time_secs = 1e-6;
    let hurried_input = directory.join("hurried.toml");
    fs::write(&hurried_input, toml::to_string(&hurried).unwrap()).unwrap();
    run_failing(
        &[
            "pieces".as_ref(),
            "plan".as_ref(),
            os(&hurried_input),
            "--out-dir".as_ref(),
            os(&directory.join("hurried")),
        ],
        "pieces_plan worker",
    );
    fs::remove_dir_all(directory).unwrap();
}

fn premise_atoms(r: &SolveResult) -> Vec<Atom> {
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    plan.premises
        .iter()
        .flat_map(|q| match q {
            SignPremise::RatioOrder { lower, upper, .. } => vec![*lower, *upper],
            SignPremise::GlobalSign { atom, .. } => vec![*atom],
        })
        .chain(
            plan.pieces
                .iter()
                .flat_map(|p| p.conditions.iter().map(|c| c.atom).chain(p.split.clone())),
        )
        .collect()
}

#[test]
fn identically_zero_b_c_or_d_take_the_partner_sign_and_match_the_baseline() {
    // Constant coefficients: b, c or d zero alone, or b and c together.
    // (c,d or b,d zero together make Delta vanish and are declined.)
    let mut cases = 0;
    for zero_mask in [0b0010u32, 0b0100, 0b1000, 0b0110] {
        for mask in 0..16u32 {
            if mask & zero_mask != 0 {
                continue;
            }
            let k: [i64; 4] = std::array::from_fn(|i| {
                let magnitude = [1, 2, 3, 5][i];
                match (zero_mask >> i & 1, mask >> i & 1) {
                    (1, _) => 0,
                    (_, 1) => magnitude,
                    _ => -magnitude,
                }
            });
            let target = format!("({})*x*y+({})*x+({})*y+({})", k[0], k[1], k[2], k[3]);
            let p = problem(&["y", "x"], &target, None);
            let r = checked(&p);
            compare_baseline(&p, &r);
            cases += 1;
        }
    }
    assert_eq!(cases, 8 * 3 + 4);
    // Parameter-dependent coefficients, with and without a guide. A zero
    // coefficient never appears in a premise, piece condition or split.
    for (target, zero) in [
        ("(t-3)*x*y+(t-2)*y+(t-1)", Atom::B),
        ("(t-3)*x*y+(t-1)*x+(t-2)", Atom::C),
        ("(t-3)*x*y+(t-1)*x+(t-2)*y", Atom::D),
        ("(t-3)*x*y+(t-2)", Atom::B),
    ] {
        for guide in [None, Some("(x+1)*(y+1)")] {
            let p = problem(&["t", "y", "x"], target, guide);
            let r = checked(&p);
            assert!(!premise_atoms(&r).contains(&zero), "{target}");
            assert!(r.cells.len() > 0, "{target}");
            compare_baseline(&p, &r);
        }
    }
    // c and d vanishing together (F=x*(a*y+b)) still declines.
    assert!(solve(&problem(&["t", "y", "x"], "(t-3)*x*y+(t-1)*x", None)).is_err());
    // A forged ratio order on the zero coefficient is rejected.
    let r = checked(&problem(
        &["t", "y", "x"],
        "(t-3)*x*y+(t-1)*x+(t-2)*y",
        Some("(x+1)*(y+1)"),
    ));
    let mut forged = serde_json::to_value(&r).unwrap();
    let premises = forged
        .pointer_mut("/bilinear_general/plan/premises")
        .unwrap()
        .as_array_mut()
        .unwrap();
    // d=0: d*u_c - c*u_d = -(t-2) is no square, but a*u_d - d*u_a... any
    // premise naming d must fail its zero-coefficient admission first.
    premises.push(json!({
        "kind": "ratio_order",
        "lower": "a",
        "upper": "d",
        "multiplier": "1",
        "square": "1",
        "strict": null
    }));
    let forged: SolveResult = serde_json::from_value(forged).unwrap();
    let error = verify(&forged).unwrap_err().to_string();
    assert!(error.contains("identically zero"), "{error}");
}

#[test]
fn negative_square_separator_orders_both_breakpoints_and_keeps_its_seam() {
    // a=-1, b=s, c=-t, d=-s^2+3*s*t-t^2: Delta=-(s-t)^2 <= 0, so both
    // positive breakpoints are ordered -d/c < -b/a off the seam s=t.
    let p = problem(&["s", "t", "y", "x"], "-x*y+s*x-t*y-s^2+3*s*t-t^2", None);
    let r = checked(&p);
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    let Separator::NegativeSquare { multiplier, square } = &plan.separator else {
        panic!(
            "expected a negative square separator, got {:?}",
            plan.separator
        )
    };
    assert_eq!(multiplier, "1");
    let algebra = Algebra::new(&r.order).unwrap();
    let g = algebra.parse(square).unwrap();
    assert!(g == algebra.parse("s-t").unwrap() || g == algebra.parse("t-s").unwrap());
    assert!(
        plan.pieces
            .iter()
            .any(|piece| piece.split.contains(&Atom::G))
    );
    assert!(has_exceptional(&r, "s-t"));
    compare_baseline(&p, &r);
    // Swapping the separator kind is rejected in both directions.
    let original = serde_json::to_value(&r).unwrap();
    for replacement in [
        json!({"kind": "square", "multiplier": multiplier, "square": square}),
        json!({"kind": "delta", "polynomial": "-s^2+2*s*t-t^2"}),
    ] {
        let mut forged = original.clone();
        *forged
            .pointer_mut("/bilinear_general/plan/separator")
            .unwrap() = replacement.clone();
        let forged: SolveResult = serde_json::from_value(forged).unwrap();
        assert!(verify(&forged).is_err(), "accepted separator {replacement}");
    }
    // The square pinch target of the square-mode test: a negative-square or
    // Delta claim is rejected as well.
    let pinch = checked(&problem(
        &["s", "t", "y", "x"],
        "-x*y+s*x-t*y+s^2-s*t+t^2",
        None,
    ));
    let original = serde_json::to_value(&pinch).unwrap();
    let Separator::Square { multiplier, square } =
        &pinch.bilinear_general.as_ref().unwrap().plan.separator
    else {
        panic!("expected a square separator")
    };
    for replacement in [
        json!({"kind": "negative_square", "multiplier": multiplier, "square": square}),
        json!({"kind": "delta", "polynomial": "s^2-2*s*t+t^2"}),
    ] {
        let mut forged = original.clone();
        *forged
            .pointer_mut("/bilinear_general/plan/separator")
            .unwrap() = replacement.clone();
        let forged: SolveResult = serde_json::from_value(forged).unwrap();
        assert!(verify(&forged).is_err(), "accepted separator {replacement}");
    }
}

#[test]
fn atoms_objective_merges_two_chain_pieces_and_covers_the_same_set() {
    let target = "(u-3)*x*y+(u-3+(v-1)^2)*x+(u-3+(u-v)^2)*y+(u-3+(v-1)^2+(u-v)^2)";
    let p = problem(&["u", "v", "y", "x"], target, Some("(x+1)*(y+1)"));
    let weighted = checked(&p);
    assert_eq!(
        weighted
            .bilinear_general
            .as_ref()
            .unwrap()
            .plan
            .pieces
            .len(),
        4
    );
    let mut q = p.clone();
    q.solver.bilinear_general_plan_objective = "atoms".into();
    let atoms = checked(&q);
    let evidence = atoms.bilinear_general.as_ref().unwrap();
    assert_eq!(
        evidence.plan.pieces,
        vec![
            PieceSpec {
                conditions: vec![cond(Atom::B, 1), cond(Atom::C, 1)],
                split: vec![Atom::A]
            },
            PieceSpec {
                conditions: vec![cond(Atom::B, 1), cond(Atom::C, -1)],
                split: vec![Atom::G]
            },
            PieceSpec {
                conditions: vec![cond(Atom::B, -1), cond(Atom::D, 1)],
                split: vec![Atom::C]
            },
        ]
    );
    assert_eq!(
        evidence.plan.empty,
        vec![EmptyClaim {
            conditions: vec![cond(Atom::B, -1), cond(Atom::D, -1)]
        }]
    );
    // Pieces never inherit the planner objective.
    assert!(
        evidence
            .bases
            .iter()
            .all(|b| b.problem.solver.bilinear_general_plan_objective == "region_weighted")
    );
    assert_eq!(
        compare_generic(&atoms, &weighted, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
    compare_baseline(&q, &atoms);
    let mut bad = p.clone();
    bad.solver.bilinear_general_plan_objective = "fewest".into();
    assert!(bad.validate().is_err());
}

#[test]
fn inner_anchor_block_keeps_a_kinematic_parameter_outside_the_scale_block() {
    // The base-only constraint t<m*s is homogeneous in the block {s,t} but
    // not in {m,s,t}: m is a kinematic parameter outside the scale block.
    let target = "(t-3*s)*x*y+s*(t-s)*x+s*(t-2*s)*y+s^2*t";
    let mut p = problem(&["m", "s", "t", "y", "x"], target, Some("(x+s)*(y+s)"));
    p.constraints.push("t<m*s".into());
    p.solver.bilinear_general_base_anchor = Some("s".into());
    p.solver.bilinear_general_base_block = vec!["t".into(), "s".into()];
    let r = checked(&p);
    let evidence = r.bilinear_general.as_ref().unwrap();
    assert!(!evidence.bases.is_empty());
    for base in &evidence.bases {
        assert_eq!(base.problem.solver.homogeneous_anchor.as_deref(), Some("s"));
        // Canonical base order, and never the outer option itself.
        assert_eq!(base.problem.solver.homogeneous_block, vec!["s", "t"]);
        assert!(base.problem.solver.bilinear_general_base_block.is_empty());
        assert!(base.homogeneous.is_some());
    }
    compare_baseline(&p, &r);
    // The default block scales m too: the piece inputs are not homogeneous.
    let mut all = p.clone();
    all.solver.bilinear_general_base_block.clear();
    assert!(
        solve(&all).map_or(true, |r| r.status != Status::CompleteGeneric),
        "a non-homogeneous block must not complete"
    );
    // A block member before the anchor, a fiber axis, or a block without an
    // anchor is refused.
    let mut early = p.clone();
    early.solver.bilinear_general_base_block = vec!["m".into(), "s".into()];
    assert!(solve(&early).is_err());
    let mut fiber = p.clone();
    fiber.solver.bilinear_general_base_block = vec!["s".into(), "y".into()];
    assert!(solve(&fiber).is_err());
    let mut orphan = p.clone();
    orphan.solver.bilinear_general_base_anchor = None;
    assert!(orphan.validate().is_err());
}

#[test]
fn strict_witness_and_delegated_wall_time_forgeries_are_rejected() {
    // Corners (1,s,s,s^2): the a<=b witness is s with a strict certificate.
    let p = problem(
        &["s", "t", "y", "x"],
        "(t-3*s)*x*y+s*(t-s)*x+s*(t-2*s)*y+s^2*t",
        Some("(x+s)*(y+s)"),
    );
    let r = checked(&p);
    let plan = &r.bilinear_general.as_ref().unwrap().plan;
    let strict = plan
        .premises
        .iter()
        .position(|q| {
            matches!(
                q,
                SignPremise::RatioOrder {
                    strict: Some(_),
                    ..
                }
            )
        })
        .expect("a strict ratio witness");
    let corner_d = plan
        .guide_positivity
        .iter()
        .find(|c| c.corner == Atom::D)
        .unwrap();
    let original = serde_json::to_value(&r).unwrap();
    let path = |field: &str| format!("/bilinear_general/plan/premises/{strict}/strict{field}");
    for (field, replacement) in [
        // The certificate of u_d=s^2 proves another polynomial.
        ("", serde_json::to_value(&corner_d.certificate).unwrap()),
        ("/sign", json!(-1)),
        ("/polynomial", json!("t")),
        ("/domain/empty", json!(true)),
    ] {
        let mut forged = original.clone();
        *forged
            .pointer_mut(&path(field))
            .unwrap_or_else(|| panic!("missing {field}")) = replacement;
        let forged: SolveResult = serde_json::from_value(forged).unwrap();
        assert!(verify(&forged).is_err(), "accepted strict{field}");
    }
    // Dropping the optional strict witness is harmless: strict sign vectors
    // never use it.
    let mut dropped = original.clone();
    *dropped.pointer_mut(&path("")).unwrap() = json!(null);
    let dropped: SolveResult = serde_json::from_value(dropped).unwrap();
    assert!(verify(&dropped).unwrap().verified);

    // A delegated piece wall time must be finite, positive and not larger
    // than the derived one (a smaller deadline only censors).
    for value in [
        0.0,
        -0.0,
        -1.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1e9,
    ] {
        let mut bad = r.clone();
        bad.bilinear_general.as_mut().unwrap().bases[0]
            .problem
            .limits
            .wall_time_secs = value;
        assert!(verify(&bad).is_err(), "accepted wall time {value}");
    }
    let mut smaller = r.clone();
    smaller.bilinear_general.as_mut().unwrap().bases[0]
        .problem
        .limits
        .wall_time_secs = 1.5;
    assert!(verify(&smaller).unwrap().verified);
}
