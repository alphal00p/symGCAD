use symgcad::{
    Problem,
    local_projection::{
        self, LocalEvidence, LocalNode, LocalOptions, PositiveIntervalEndpoint, PositiveRayCase,
    },
    output::{RootIndexDomain, Status},
};

fn problem() -> Problem {
    Problem {
        variables: vec!["a".into(), "z".into()],
        constraints: vec!["a>999/1000".into(), "a<1001/1000".into(), "z>0".into()],
        split: vec!["(z-1)*(z-2)*(z^2-z+a)+1/1000".into()],
        limits: symgcad::Limits {
            wall_time_secs: 15.0,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn options() -> LocalOptions {
    LocalOptions {
        descartes_intervals: true,
        fallback_to_global: false,
        ..Default::default()
    }
}
fn solve(p: &Problem, o: &LocalOptions) -> local_projection::LocalSolveResult {
    let result = local_projection::solve_local(p, o).unwrap();
    assert_eq!(result.solution.status, Status::CompleteGeneric);
    assert!(local_projection::verify_local(&result).unwrap().verified);
    result
}
fn tamper(node: &mut LocalNode, which: usize) -> bool {
    if let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    {
        if let Some(ray) = &mut projection.positive_ray {
            for rule in &mut ray.rules {
                if let PositiveRayCase::DescartesIntervals { intervals } = &mut rule.case {
                    match which {
                        0 => {
                            intervals.remove(0);
                        }
                        1 => intervals[0].coefficient_signs[0] *= -1,
                        2 => {
                            intervals[0].interval.upper = PositiveIntervalEndpoint::PositiveInfinity
                        }
                        _ => {
                            intervals[0].coefficient_signs.pop();
                        }
                    }
                    return true;
                }
            }
        }
        return sectors.iter_mut().any(|s| tamper(&mut s.child, which));
    }
    false
}
fn compare(
    a: local_projection::LocalSolveResult,
    b: local_projection::LocalSolveResult,
    p: &Problem,
) {
    let attach = |mut r: local_projection::LocalSolveResult| {
        r.solution.local = Some(Box::new(r.evidence));
        r.solution
    };
    assert_eq!(
        symgcad::comparison::compare_generic(&attach(a), &attach(b), &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn unsaturated_intervals_verify_persist_and_reject_tampered_covers() {
    let p = problem();
    let result = solve(&p, &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.descartes_interval_discriminants_omitted > 0);
    assert_eq!(result.solution.cells.len(), 3);
    let indices = result
        .solution
        .cells
        .iter()
        .filter_map(|c| c.axes[1].upper.as_ref())
        .map(|b| (b.index_domain, b.index))
        .collect::<Vec<_>>();
    assert_eq!(
        indices,
        vec![
            (RootIndexDomain::Positive, 0),
            (RootIndexDomain::Positive, 1)
        ]
    );
    let restored = serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
    for which in 0..4 {
        let mut changed = result.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(tamper(root, which));
        assert!(local_projection::verify_local(&changed).is_err());
    }
    let uncached = solve(
        &p,
        &LocalOptions {
            reuse_completed_subtrees: false,
            reuse_learned_cuts: false,
            ..options()
        },
    );
    assert_eq!(result.solution.cells, uncached.solution.cells);
    let ordinary = solve(
        &p,
        &LocalOptions {
            descartes_intervals: false,
            ..options()
        },
    );
    compare(result.clone(), ordinary.clone(), &p);
    // Enabling an additional optional theorem cannot invalidate an existing
    // stronger full-real proof. It must not force new template discovery.
    let mut stronger = ordinary;
    let LocalEvidence::Local { options, .. } = &mut stronger.evidence else {
        panic!()
    };
    options.descartes_intervals = true;
    assert!(local_projection::verify_local(&stronger).unwrap().verified);
    let mut disabled = result;
    let LocalEvidence::Local { options, .. } = &mut disabled.evidence else {
        panic!()
    };
    options.descartes_intervals = false;
    assert!(local_projection::verify_local(&disabled).is_err());
}

#[test]
fn zero_positive_roots_and_negative_collisions_need_no_positive_boundaries() {
    let mut p = problem();
    p.split = vec!["(z^2-z+1)*(z^2+1)+a/1000".into()];
    let result = solve(&p, &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.descartes_interval_discriminants_omitted > 0);
    assert_eq!(result.solution.cells.len(), 1);
    assert!(result.solution.cells[0].axes[1].upper.is_none());
    compare(
        result,
        solve(
            &p,
            &LocalOptions {
                descartes_intervals: false,
                ..options()
            },
        ),
        &p,
    );
    p.constraints = vec!["a>2249/1000".into(), "a<2251/1000".into(), "z>0".into()];
    p.split = vec!["(z-1)*(z-2)*(z^2+3*z+a)+1/100000".into()];
    let result = solve(&p, &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.descartes_interval_discriminants_omitted > 0);
    compare(
        result,
        solve(
            &p,
            &LocalOptions {
                descartes_intervals: false,
                ..options()
            },
        ),
        &p,
    );
}

#[test]
fn finite_interval_stock_terminates_at_an_irrational_collision_and_translating_family() {
    for (constraints, split) in [
        (vec!["a>0", "a<1", "z>0"], "(z^2-2)^2-a"),
        (vec!["a>0", "z>0"], "(z-a-1)*(z-a-2)*(z-a-3)*(z-a-4)+1/1000"),
    ] {
        let mut p = problem();
        p.constraints = constraints.into_iter().map(str::to_owned).collect();
        p.split = vec![split.into()];
        let result = solve(&p, &options());
        let LocalEvidence::Local { report, .. } = &result.evidence else {
            panic!()
        };
        assert!(report.descartes_interval_templates <= 2);
        assert!(report.descartes_interval_budget_fallbacks > 0);
        assert!(report.speculative_nodes < 10000);
        compare(
            result,
            solve(
                &p,
                &LocalOptions {
                    descartes_intervals: false,
                    ..options()
                },
            ),
            &p,
        );
    }
}

#[test]
fn historical_missing_interval_option_preserves_proof_meaning() {
    let p = Problem {
        variables: vec!["z".into()],
        constraints: vec!["z>0".into()],
        split: vec!["z^2-2".into()],
        ..Default::default()
    };
    let ordinary = solve(&p, &LocalOptions::default());
    let mut encoded = serde_json::to_value(ordinary).unwrap();
    encoded["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("descartes_intervals");
    let restored = serde_json::from_value(encoded).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
    let LocalEvidence::Local { options, .. } = restored.evidence else {
        panic!()
    };
    assert!(!options.descartes_intervals);
}
