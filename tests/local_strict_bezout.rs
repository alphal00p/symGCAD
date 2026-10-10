//! Independent integration checks for proposed strict pair identities.
use serde_json::Value;
use symgcad::{
    Problem,
    algebra::{Algebra, canonical},
    local_projection::{
        self, LocalEvidence, LocalNode, LocalOptions, LocalReport, LocalSolveResult,
        PositiveRayProjection, PositiveStrictBezout, StrictBezoutHint,
    },
    output::{SolveResult, Status},
};

fn problem(split: [&str; 2]) -> Problem {
    let mut problem = Problem {
        variables: vec!["a".into(), "z".into()],
        constraints: vec!["a>0".into(), "a<1".into(), "z>0".into()],
        split: split.into_iter().map(str::to_owned).collect(),
        ..Default::default()
    };
    problem.solver.engine = "local".into();
    problem.limits.wall_time_secs = 15.0;
    problem
}

fn hint(pair: [&str; 2], multipliers: [&str; 2], square: &str, k: &str) -> StrictBezoutHint {
    // Proposals name the canonical factors used by local projection. Preserve
    // the exact identity when canonicalization changes a sign or scalar.
    let algebra = Algebra::new(&["a".into(), "z".into(), "u".into()]).unwrap();
    let canonical_pair = pair.map(|p| canonical(&algebra.parse(p).unwrap()));
    let canonical_multipliers = std::array::from_fn::<_, 2, _>(|index| {
        let original = algebra.parse(pair[index]).unwrap();
        let scale = original.lcoeff() / canonical_pair[index].lcoeff();
        algebra.parse(multipliers[index]).unwrap().mul_coeff(scale)
    });
    StrictBezoutHint {
        schema_version: 1,
        polynomials: canonical_pair.each_ref().map(|p| algebra.format(p)),
        multipliers: canonical_multipliers.each_ref().map(|p| algebra.format(p)),
        square: square.into(),
        multiplier: k.into(),
    }
}

fn options(enabled: bool) -> LocalOptions {
    LocalOptions {
        strict_bezout: enabled,
        positive_pair_separation: false,
        positive_pair_witnesses: false,
        adjacent_pairs: false,
        domain_pruning: false,
        fallback_to_global: false,
        ..Default::default()
    }
}

fn solve(problem: &Problem, enabled: bool) -> LocalSolveResult {
    let result = local_projection::solve_local(problem, &options(enabled)).unwrap();
    assert_eq!(result.solution.status, Status::CompleteGeneric);
    assert!(local_projection::verify_local(&result).unwrap().verified);
    result
}

fn report(result: &LocalSolveResult) -> &LocalReport {
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!("local proof required")
    };
    report
}

fn attach(mut result: LocalSolveResult) -> SolveResult {
    result.solution.local = Some(Box::new(result.evidence));
    result.solution
}

fn pair_ids(result: &LocalSolveResult, pair: &[String; 2]) -> [usize; 2] {
    let algebra = Algebra::new(&result.solution.order).unwrap();
    let mut ids = pair.each_ref().map(|text| {
        let expected = canonical(&algebra.parse(text).unwrap());
        result
            .solution
            .polynomials
            .iter()
            .position(|text| algebra.parse(text).unwrap() == expected)
            .expect("pair polynomial is registered")
    });
    ids.sort_unstable();
    ids
}

fn edit_ray(
    node: &mut LocalNode,
    predicate: &impl Fn(&PositiveRayProjection) -> bool,
    edit: &mut impl FnMut(&mut PositiveRayProjection),
) -> bool {
    let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    else {
        return false;
    };
    if let Some(ray) = &mut projection.positive_ray {
        if predicate(ray) {
            edit(ray);
            return true;
        }
    }
    sectors
        .iter_mut()
        .any(|sector| edit_ray(&mut sector.child, predicate, edit))
}

fn edit_first_proof(
    result: &mut LocalSolveResult,
    edit: &mut impl FnMut(&mut PositiveStrictBezout),
) {
    let LocalEvidence::Local { root, .. } = &mut result.evidence else {
        panic!()
    };
    assert!(edit_ray(
        root,
        &|ray| !ray.strict_bezout.is_empty(),
        &mut |ray| {
            edit(&mut ray.strict_bezout[0]);
        }
    ));
}

fn inject(result: &mut LocalSolveResult, proposed: StrictBezoutHint, support: Vec<usize>) {
    let ids = pair_ids(result, &proposed.polynomials);
    let LocalEvidence::Local { root, options, .. } = &mut result.evidence else {
        panic!()
    };
    options.strict_bezout = true;
    fn insert(
        node: &mut LocalNode,
        ids: [usize; 2],
        hint: &StrictBezoutHint,
        support: &[usize],
    ) -> bool {
        let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        else {
            return false;
        };
        if ids.iter().all(|id| projection.root_bearing.contains(id)) {
            if let Some(ray) = &mut projection.positive_ray {
                ray.strict_bezout.push(PositiveStrictBezout {
                    polynomials: ids,
                    hint: hint.clone(),
                    supporting_constraints: support.to_vec(),
                });
                return true;
            }
        }
        sectors
            .iter_mut()
            .any(|sector| insert(&mut sector.child, ids, hint, support))
    }
    assert!(insert(root, ids, &proposed, &support));
}

fn accepted_fixture() -> (Problem, StrictBezoutHint) {
    let pair = ["z^2-a", "z^2-a+a^2"];
    let proposed = hint(pair, ["-1", "1"], "a", "1");
    let mut p = problem(pair);
    p.solver.local_strict_bezout_hints.push(proposed.clone());
    (p, proposed)
}

#[test]
fn exact_identity_covers_ancestor_current_and_joint_positive_support() {
    for (pair, h, k, multipliers, expected_support) in [
        (["z^2-a", "z^2-a+a^2"], "a", "1", ["-1", "1"], vec![0]),
        (["z^2-a", "2*z^2-a"], "z", "1", ["-1", "1"], vec![2]),
        (
            ["z^2-a", "2*z^2+2*a*z+a^2-a"],
            "a+z",
            "1",
            ["-1", "1"],
            vec![0, 2],
        ),
        (
            ["z^2-a", "z^2-a+a^2"],
            "-a",
            "5/7",
            ["-5/7", "5/7"],
            vec![0],
        ),
    ] {
        let mut p = problem(pair);
        p.solver
            .local_strict_bezout_hints
            .push(hint(pair, multipliers, h, k));
        let mut result = solve(&p, true);
        assert_eq!(report(&result).strict_bezout_hints_accepted, 1);
        assert!(report(&result).strict_bezout_resultants_omitted > 0);
        edit_first_proof(&mut result, &mut |proof| {
            assert_eq!(proof.supporting_constraints, expected_support);
        });
        let decoded = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert!(local_projection::verify_local(&decoded).unwrap().verified);
    }
}

#[test]
fn exact_common_refinement_agrees_with_full_pair_projection_and_baseline() {
    let (p, _) = accepted_fixture();
    let fast = solve(&p, true);
    let ordinary = solve(&p, false);
    let mut baseline_problem = p.clone();
    baseline_problem.solver.engine = "baseline".into();
    baseline_problem.solver.local_strict_bezout_hints.clear();
    let baseline = symgcad::solver::solve(&baseline_problem).unwrap();
    assert!(symgcad::verify::verify(&baseline).unwrap().verified);
    let fast = attach(fast);
    for other in [attach(ordinary), baseline] {
        let comparison = symgcad::comparison::compare_generic(&fast, &other, &p.limits).unwrap();
        assert_eq!(comparison.equivalent, Some(true));
    }
}

#[test]
fn solver_dispatch_and_toml_preserve_untrusted_proposals_and_exact_proofs() {
    let (p, _) = accepted_fixture();
    let decoded = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    assert_eq!(
        decoded.solver.local_strict_bezout_hints,
        p.solver.local_strict_bezout_hints
    );
    let result = symgcad::solver::solve(&decoded).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let LocalEvidence::Local {
        report, options, ..
    } = result.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(options.strict_bezout);
    assert_eq!(report.strict_bezout_hints_accepted, 1);
}

#[test]
fn malformed_or_false_proposals_decline_without_pruning() {
    let (mut p, original) = accepted_fixture();
    let ordinary = solve(&p, false);
    let mut proposals = Vec::new();
    for version in [0, 2] {
        let mut h = original.clone();
        h.schema_version = version;
        proposals.push(h);
    }
    let mut h = original.clone();
    h.multipliers[0] = format!("({})+1", h.multipliers[0]);
    proposals.push(h);
    let mut h = original.clone();
    h.square = "unknown_symbol".into();
    proposals.push(h);
    let mut h = original.clone();
    h.polynomials[1] = h.polynomials[0].clone();
    proposals.push(h);
    // Exact zero syzygies remain invalid: k=0 or H=0 cannot separate roots.
    let mut h = original.clone();
    h.multipliers = [
        original.polynomials[1].clone(),
        format!("-({})", original.polynomials[0]),
    ];
    h.multiplier = "0".into();
    proposals.push(h.clone());
    h.multiplier = "1".into();
    h.square = "0".into();
    proposals.push(h);
    let mut h = original.clone();
    h.multipliers = h.multipliers.map(|text| format!("-({text})"));
    h.multiplier = "-1".into();
    proposals.push(h);
    let mut h = original;
    h.square = "a^33".into();
    proposals.push(h);
    for proposed in proposals {
        p.solver.local_strict_bezout_hints = vec![proposed];
        let result = solve(&p, true);
        assert_eq!(report(&result).strict_bezout_hints_accepted, 0);
        assert_eq!(report(&result).strict_bezout_hints_rejected, 1);
        assert_eq!(report(&result).strict_bezout_resultants_omitted, 0);
        assert_eq!(result.solution.cells, ordinary.solution.cells);
    }
}

#[test]
fn kernel_rejects_exact_but_non_strict_squared_identity_at_a_positive_collision() {
    let pair = ["z^2-a", "z^2+a^2-3*a+1"];
    let proposed = hint(pair, ["-1", "1"], "a-1", "1");
    let mut p = problem(pair);
    p.constraints[1] = "a<2".into();
    p.solver.local_strict_bezout_hints.push(proposed.clone());
    let result = solve(&p, true);
    assert_eq!(report(&result).strict_bezout_hints_rejected, 1);
    // The identity is true, but H=0 and both p=q=0 at the positive point a=z=1.
    let algebra = Algebra::new(&p.variables).unwrap();
    assert_eq!(
        &algebra.parse(pair[1]).unwrap() - &algebra.parse(pair[0]).unwrap(),
        algebra.parse("(a-1)^2").unwrap()
    );
    let collision: [_; 2] = std::array::from_fn(|_| symgcad::algebra::rational("1").unwrap());
    for expression in [pair[0], pair[1], "a-1"] {
        assert_eq!(
            symgcad::solver::evaluate(&algebra.parse(expression).unwrap(), &collision).to_string(),
            "0"
        );
    }
    // Inject the algebraically correct weak proof directly into replay evidence.
    // This exercises the proof kernel, independently of proposal rejection.
    let mut forged = result;
    inject(&mut forged, proposed, vec![0]);
    let error = local_projection::verify_local(&forged)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("uniform nonzero coefficient signs"),
        "{error}"
    );
}

#[test]
fn original_positive_premises_and_axis_scope_are_mandatory() {
    let (mut missing_ancestor, proposed) = accepted_fixture();
    missing_ancestor.constraints.remove(0);
    let missing = solve(&missing_ancestor, true);
    assert_eq!(report(&missing).strict_bezout_hints_rejected, 1);
    assert_eq!(report(&missing).strict_bezout_resultants_omitted, 0);

    let pair = ["z^2-a", "2*z^2-a"];
    let mut no_current = problem(pair);
    no_current.constraints.pop();
    no_current
        .solver
        .local_strict_bezout_hints
        .push(hint(pair, ["-1", "1"], "z", "1"));
    let result = solve(&no_current, true);
    assert_eq!(report(&result).strict_bezout_hints_rejected, 1);
    assert_eq!(report(&result).strict_bezout_resultants_omitted, 0);

    let pair = ["z^2-a", "z^2-a+1"];
    let mut later = problem(pair);
    later.variables.push("u".into());
    later.constraints = vec!["a>1".into(), "a<2".into(), "z>0".into(), "u>0".into()];
    later
        .solver
        .local_strict_bezout_hints
        .push(hint(pair, ["-u^2", "u^2"], "u", "1"));
    let result = solve(&later, true);
    assert_eq!(report(&result).strict_bezout_hints_rejected, 1);
    assert!(
        report(&result)
            .strict_bezout_rejections
            .iter()
            .any(|reason| reason.contains("later axis"))
    );
    assert_eq!(report(&result).strict_bezout_resultants_omitted, 0);

    // A forged original-premise index is checked independently of the proposal.
    let (p, _) = accepted_fixture();
    let mut forged = solve(&p, true);
    edit_first_proof(&mut forged, &mut |proof| {
        proof.supporting_constraints = vec![2]
    });
    assert!(local_projection::verify_local(&forged).is_err());
    let mut forged = missing;
    inject(&mut forged, proposed, vec![]);
    let error = local_projection::verify_local(&forged)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("original positive coordinate premise"),
        "{error}"
    );
}

#[test]
fn replay_checks_identity_pair_inventory_support_and_opt_in_independently() {
    let (p, _) = accepted_fixture();
    let result = solve(&p, true);
    for change in 0..9 {
        let mut forged = result.clone();
        edit_first_proof(&mut forged, &mut |proof| match change {
            0 => proof.hint.multipliers[0] = format!("({})+1", proof.hint.multipliers[0]),
            1 => proof.hint.multiplier = "0".into(),
            2 => proof.hint.square = "0".into(),
            3 => proof.hint.schema_version = 2,
            4 => proof.polynomials.swap(0, 1),
            5 => proof.polynomials[1] = proof.polynomials[0],
            6 => proof.supporting_constraints.clear(),
            7 => proof.hint.polynomials[0] = format!("({})+1", proof.hint.polynomials[0]),
            _ => proof.hint.square = "z".into(),
        });
        assert!(
            local_projection::verify_local(&forged).is_err(),
            "tamper {change}"
        );
    }
    let mut forged = result.clone();
    let LocalEvidence::Local { root, .. } = &mut forged.evidence else {
        panic!()
    };
    assert!(edit_ray(
        root,
        &|ray| !ray.strict_bezout.is_empty(),
        &mut |ray| ray.strict_bezout.push(ray.strict_bezout[0].clone())
    ));
    assert!(local_projection::verify_local(&forged).is_err());

    // The independently checked identity may be presented with its two terms
    // exchanged. Input proposals are discovery hints, not replay authority.
    let mut equivalent = result.clone();
    equivalent
        .solution
        .problem
        .solver
        .local_strict_bezout_hints
        .clear();
    edit_first_proof(&mut equivalent, &mut |proof| {
        proof.hint.polynomials.swap(0, 1);
        proof.hint.multipliers.swap(0, 1);
    });
    assert!(
        local_projection::verify_local(&equivalent)
            .unwrap()
            .verified
    );

    let mut forged = result;
    let LocalEvidence::Local { options, .. } = &mut forged.evidence else {
        panic!()
    };
    options.strict_bezout = false;
    assert!(local_projection::verify_local(&forged).is_err());
}

fn remove_guard(node: &mut LocalNode, id: usize) -> bool {
    let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    else {
        return false;
    };
    if projection
        .positive_ray
        .as_ref()
        .is_some_and(|ray| !ray.strict_bezout.is_empty())
        && projection.guards.contains(&id)
    {
        projection.guards.retain(|&guard| guard != id);
        return true;
    }
    sectors
        .iter_mut()
        .any(|sector| remove_guard(&mut sector.child, id))
}

#[test]
fn pair_identity_does_not_replace_individual_leading_or_discriminant_guards() {
    for (pair, multipliers, h, upper, guard) in [
        (
            ["a*z^2-z^2-1", "a*z^2-z^2-1-a^2"],
            ["1", "-1"],
            "a",
            "2",
            "a-1",
        ),
        (["z^3-a*z+1", "z^3-a*z+2"], ["-1", "1"], "1", "4", "a-3"),
    ] {
        let mut p = problem(pair);
        p.constraints[1] = format!("a<{upper}");
        p.solver
            .local_strict_bezout_hints
            .push(hint(pair, multipliers, h, "1"));
        let mut configured = options(true);
        configured.descartes_one = false;
        let mut result = local_projection::solve_local(&p, &configured).unwrap();
        assert_eq!(result.solution.status, Status::CompleteGeneric);
        assert!(local_projection::verify_local(&result).unwrap().verified);
        assert!(report(&result).strict_bezout_resultants_omitted > 0);
        let algebra = Algebra::new(&result.solution.order).unwrap();
        let expected = canonical(&algebra.parse(guard).unwrap());
        let id = result
            .solution
            .polynomials
            .iter()
            .position(|p| algebra.parse(p).unwrap() == expected)
            .expect("individual projection guard retained");
        let LocalEvidence::Local { root, .. } = &mut result.evidence else {
            panic!()
        };
        assert!(remove_guard(root, id));
        assert!(local_projection::verify_local(&result).is_err());
    }
}

#[test]
fn lower_positive_premise_is_returned_even_when_all_ordinary_pair_guards_are_constant() {
    let pair = ["z-1", "z-2"];
    let mut p = problem(pair);
    p.solver
        .local_strict_bezout_hints
        .push(hint(pair, ["a^2", "-a^2"], "a", "1"));
    let mut result = solve(&p, true);
    assert!(report(&result).strict_bezout_resultants_omitted > 0);
    let algebra = Algebra::new(&result.solution.order).unwrap();
    let a = algebra.parse("a").unwrap();
    let guard = result
        .solution
        .polynomials
        .iter()
        .position(|text| algebra.parse(text).unwrap() == a)
        .unwrap();
    let LocalEvidence::Local { root, .. } = &mut result.evidence else {
        panic!()
    };
    assert!(
        remove_guard(root, guard),
        "the original a>0 premise must become a nonzero ancestor guard"
    );
    assert!(local_projection::verify_local(&result).is_err());
}

fn strip_new_fields(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.retain(|key, _| {
                key != "strict_bezout"
                    && !key.starts_with("strict_bezout_")
                    && key != "local_strict_bezout_hints"
            });
            for child in fields.values_mut() {
                strip_new_fields(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                strip_new_fields(child);
            }
        }
        _ => {}
    }
}

#[test]
fn historical_evidence_without_any_bezout_fields_keeps_full_pair_semantics() {
    assert!(!LocalOptions::default().strict_bezout);
    let (mut p, _) = accepted_fixture();
    p.solver.local_strict_bezout_hints.clear();
    let ordinary = solve(&p, false);
    let mut serialized = serde_json::to_value(&ordinary).unwrap();
    strip_new_fields(&mut serialized);
    let historical: LocalSolveResult = serde_json::from_value(serialized).unwrap();
    let LocalEvidence::Local { options, .. } = &historical.evidence else {
        panic!()
    };
    assert!(!options.strict_bezout);
    assert!(
        local_projection::verify_local(&historical)
            .unwrap()
            .verified
    );
    assert_eq!(historical.solution.cells, ordinary.solution.cells);
}
