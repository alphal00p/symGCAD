use symgcad::{
    Problem,
    local_projection::{self, LocalEvidence, LocalNode, LocalOptions},
    output::Status,
};

fn problem(split: &[&str], lower: &str, upper: &str) -> Problem {
    Problem {
        variables: vec!["a".into(), "z".into()],
        constraints: vec![format!("a>{lower}"), format!("a<{upper}"), "z>0".into()],
        split: split.iter().map(|s| s.to_string()).collect(),
        limits: symgcad::Limits {
            wall_time_secs: 15.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn options() -> LocalOptions {
    LocalOptions {
        positive_pair_witnesses: true,
        positive_pair_separation: false,
        fallback_to_global: false,
        domain_pruning: false,
        ..Default::default()
    }
}

fn solve(p: &Problem, options: &LocalOptions) -> local_projection::LocalSolveResult {
    let result = local_projection::solve_local(p, options).unwrap();
    assert_eq!(result.solution.status, Status::CompleteGeneric);
    assert!(local_projection::verify_local(&result).unwrap().verified);
    result
}

fn mutate(node: &mut LocalNode, change: usize) -> bool {
    if let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    {
        if let Some(ray) = &mut projection.positive_ray {
            if let Some(proof) = ray.pair_witnesses.first_mut() {
                match change {
                    0 => proof.separators[0] = "0".into(),
                    1 => proof.evaluation_signs[0][0] *= -1,
                    2 => proof.branch_order.pop().map(|_| ()).unwrap(),
                    3 => proof.positive_counts[0] += 1,
                    4 => {
                        let duplicate = proof.clone();
                        ray.pair_witnesses.push(duplicate);
                    }
                    _ => {
                        if projection.guards.is_empty() {
                            return false;
                        }
                        projection.guards.pop();
                    }
                }
                return true;
            }
        }
        return sectors.iter_mut().any(|s| mutate(&mut s.child, change));
    }
    false
}

#[test]
fn rational_pair_evidence_persists_and_all_claims_are_checked() {
    let p = problem(&["z^2-2", "z^2+a*z-4"], "1", "2");
    let result = solve(&p, &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.pair_witness_resultants_omitted > 0);
    assert!(report.pair_witness_templates > 0);
    let restored = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
    for change in 0..6 {
        let mut changed = result.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(mutate(root, change));
        assert!(local_projection::verify_local(&changed).is_err());
    }
    let mut changed = result;
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.positive_pair_witnesses = false;
    assert!(local_projection::verify_local(&changed).is_err());
}

#[test]
fn irrational_cross_collision_and_translation_force_finite_resultant_fallback() {
    for p in [
        problem(&["z^2-2", "z^2+a*z-4"], "1", "2"),
        problem(&["(z-a)^2-2", "(z-a-1)^2-3"], "2", "100"),
    ] {
        let result = solve(&p, &options());
        let LocalEvidence::Local { report, .. } = &result.evidence else {
            panic!()
        };
        assert!(report.pair_witness_resultants_omitted > 0);
        assert!(report.pair_witness_budget_fallbacks > 0);
        assert!(report.projected_pairs > 0);
        assert!(report.speculative_nodes < 1000);
        assert_eq!(report.pair_witness_templates, 2);
    }
}

#[test]
fn exact_common_refinement_matches_full_resultants_and_uncached_replay() {
    let p = problem(&["z^2-2", "z^2+a*z-4"], "1", "2");
    let fast = solve(&p, &options());
    let uncached = solve(
        &p,
        &LocalOptions {
            reuse_completed_subtrees: false,
            reuse_learned_cuts: false,
            ..options()
        },
    );
    assert_eq!(fast.solution.cells, uncached.solution.cells);
    let ordinary = solve(
        &p,
        &LocalOptions {
            positive_pair_witnesses: false,
            ..options()
        },
    );
    let attach = |mut r: local_projection::LocalSolveResult| {
        r.solution.local = Some(Box::new(r.evidence));
        r.solution
    };
    assert_eq!(
        symgcad::comparison::compare_generic(
            &attach(fast),
            &attach(ordinary),
            &symgcad::Limits::default()
        )
        .unwrap()
        .equivalent,
        Some(true)
    );
}

#[test]
fn ordinary_pair_certificate_is_valid_strengthening_and_historical_default() {
    assert!(!LocalOptions::default().positive_pair_witnesses);
    let mut value = serde_json::to_value(LocalOptions::default()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("positive_pair_witnesses");
    let historical: LocalOptions = serde_json::from_value(value).unwrap();
    assert!(!historical.positive_pair_witnesses);
    let p = problem(&["z^2-2", "z^2+a*z-4"], "1", "2");
    let mut ordinary = solve(
        &p,
        &LocalOptions {
            positive_pair_witnesses: false,
            ..options()
        },
    );
    let LocalEvidence::Local { options, .. } = &mut ordinary.evidence else {
        panic!()
    };
    options.positive_pair_witnesses = true;
    assert!(local_projection::verify_local(&ordinary).unwrap().verified);
}
