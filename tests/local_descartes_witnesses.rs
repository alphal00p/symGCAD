use symgcad::{
    Problem,
    local_projection::{self, LocalEvidence, LocalNode, LocalOptions, PositiveRayCase},
    output::{RootIndexDomain, Status},
};

fn problem() -> Problem {
    Problem {
        variables: vec!["a".into(), "z".into()],
        constraints: vec!["a>999/1000".into(), "a<1001/1000".into(), "z>0".into()],
        split: vec!["(z-1)*(z-2)*(z-3)*(z-4)*(z^2+2*z+a)+1/1000".into()],
        limits: symgcad::Limits {
            wall_time_secs: 15.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn options() -> LocalOptions {
    LocalOptions {
        descartes_witnesses: true,
        fallback_to_global: false,
        ..Default::default()
    }
}

fn solve(p: &Problem, options: &LocalOptions) -> local_projection::LocalSolveResult {
    let result = local_projection::solve_local(p, options).unwrap();
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
                if let PositiveRayCase::DescartesWitnesses {
                    separators,
                    evaluation_signs,
                    coefficient_signs,
                } = &mut rule.case
                {
                    match which {
                        0 => separators[0] = "0".into(),
                        1 => evaluation_signs[1] *= -1,
                        2 => coefficient_signs[0] *= -1,
                        _ => {
                            separators.swap(0, 1);
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

#[test]
fn four_positive_roots_cross_negative_collisions_without_discriminants() {
    let result = solve(&problem(), &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.descartes_witness_discriminants_omitted > 0);
    assert_eq!(result.solution.cells.len(), 5);
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|cell| cell.axes[0].sample == "1")
    );
    let indices = result
        .solution
        .cells
        .iter()
        .filter_map(|cell| cell.axes[1].upper.as_ref())
        .map(|b| (b.index_domain, b.index))
        .collect::<Vec<_>>();
    assert_eq!(
        indices,
        (0..4)
            .map(|i| (RootIndexDomain::Positive, i))
            .collect::<Vec<_>>()
    );
    let value = serde_json::to_vec(&result).unwrap();
    let restored = serde_json::from_slice(&value).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
    let uncached = solve(
        &problem(),
        &LocalOptions {
            reuse_completed_subtrees: false,
            reuse_learned_cuts: false,
            ..options()
        },
    );
    assert_eq!(result.solution.cells, uncached.solution.cells);
    for which in 0..4 {
        let mut changed = result.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(tamper(root, which));
        assert!(local_projection::verify_local(&changed).is_err());
    }
    let mut changed = result;
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.descartes_witnesses = false;
    assert!(local_projection::verify_local(&changed).is_err());
}

#[test]
fn witnesses_and_full_discriminants_have_exact_common_coverage() {
    let p = problem();
    let fast = solve(&p, &options());
    let ordinary = solve(
        &p,
        &LocalOptions {
            descartes_witnesses: false,
            ..options()
        },
    );
    assert!(ordinary.solution.cells.len() > fast.solution.cells.len());
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
fn nonsaturation_and_sample_zero_coefficients_do_not_create_witnesses() {
    let mut p = problem();
    p.split = vec!["z^4-a*z^3+z^2-z+1".into()];
    let result = solve(&p, &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert_eq!(report.descartes_witness_discriminants_omitted, 0);
    p.constraints = vec!["a>-1/100".into(), "a<1/100".into(), "z>0".into()];
    p.split = vec!["(z-1)*(z-2)*(z-3)*(z-4)+a*z^5".into()];
    let result = solve(&p, &options());
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|cell| cell.axes[0].sample != "0")
    );
}

#[test]
fn finite_separator_stock_finishes_colliding_and_translating_families() {
    for (constraints, polynomial) in [
        (vec!["a>0", "a<1", "z>0"], "(z^2-2)^2-a"),
        (vec!["a>0", "z>0"], "(z-a-1)*(z-a-2)*(z-a-3)*(z-a-4)+1/1000"),
    ] {
        let mut p = problem();
        p.constraints = constraints.into_iter().map(str::to_owned).collect();
        p.split = vec![polynomial.into()];
        let bounded = solve(&p, &options());
        let LocalEvidence::Local { report, .. } = &bounded.evidence else {
            panic!()
        };
        assert!(report.descartes_witness_budget_fallbacks > 0);
        assert!(report.descartes_witness_templates <= 2);
        assert!(report.speculative_nodes < 10000);
        if polynomial == "(z^2-2)^2-a" {
            let plain = solve(
                &p,
                &LocalOptions {
                    descartes_witnesses: false,
                    ..options()
                },
            );
            let attach = |mut r: local_projection::LocalSolveResult| {
                r.solution.local = Some(Box::new(r.evidence));
                r.solution
            };
            assert_eq!(
                symgcad::comparison::compare_generic(&attach(bounded), &attach(plain), &p.limits)
                    .unwrap()
                    .equivalent,
                Some(true)
            );
        }
    }
}

#[test]
fn historical_evidence_omits_the_opt_in_without_changing_its_meaning() {
    let p = Problem {
        variables: vec!["x".into()],
        constraints: vec!["x>0".into()],
        split: vec!["x^2-2".into()],
        ..Default::default()
    };
    let result = solve(&p, &LocalOptions::default());
    let mut encoded = serde_json::to_value(&result).unwrap();
    encoded["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("descartes_witnesses");
    let restored = serde_json::from_value(encoded).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
    let LocalEvidence::Local { options, .. } = restored.evidence else {
        panic!()
    };
    assert!(!options.descartes_witnesses);
}
