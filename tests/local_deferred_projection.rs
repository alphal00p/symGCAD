use symgcad::{
    Problem,
    local_projection::{self, LocalEvidence, LocalNode, LocalOptions, LocalSolveResult},
    output::{SolveResult, Status},
};

fn options() -> LocalOptions {
    LocalOptions {
        defer_projection: true,
        stable_sector_samples: true,
        fallback_to_global: false,
        ..Default::default()
    }
}
fn problem(variables: &[&str], constraints: &[&str], split: &[&str]) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|v| (*v).into()).collect(),
        constraints: constraints.iter().map(|v| (*v).into()).collect(),
        split: split.iter().map(|v| (*v).into()).collect(),
        ..Default::default()
    };
    p.limits.wall_time_secs = 20.0;
    p
}
fn local(p: &Problem, options: &LocalOptions) -> LocalSolveResult {
    let result = local_projection::solve_local(p, options).unwrap();
    assert_eq!(
        result.solution.status,
        Status::CompleteGeneric,
        "{:?}",
        result.evidence
    );
    assert!(local_projection::verify_local(&result).unwrap().verified);
    result
}
fn proof(mut result: LocalSolveResult) -> SolveResult {
    result.solution.local = Some(Box::new(result.evidence));
    result.solution
}

#[test]
fn deferred_projection_preserves_generic_coverage_with_vanishing_and_coincident_fibers() {
    let fixtures = [
        problem(&["a", "b", "z"], &[], &["a*z+b"]),
        problem(&["a", "z"], &["z>0"], &["z*(z-a)", "z^2-a"]),
        problem(&["x", "y", "z"], &["x>0", "y>0", "z>0"], &["z^2-y", "z-x"]),
    ];
    for p in fixtures {
        for positive_ray in [false, true] {
            let opted = LocalOptions {
                positive_ray,
                ..options()
            };
            let deferred = local(&p, &opted);
            let ordinary = local(
                &p,
                &LocalOptions {
                    defer_projection: false,
                    ..opted
                },
            );
            assert_eq!(
                symgcad::comparison::compare_generic(
                    &proof(deferred),
                    &proof(ordinary),
                    &symgcad::Limits::default()
                )
                .unwrap()
                .equivalent,
                Some(true)
            );
        }
    }
}

#[test]
fn every_rejected_sector_and_all_lower_guard_obligations_remain_exhaustive() {
    let p = problem(&["x", "y", "z"], &["x>0", "y>0", "z>0"], &["z^2-y", "z-x"]);
    let result = local(&p, &options());
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.deferred_projection_calls > 0);
    for mutation in 0..4 {
        let mut forged = result.clone();
        let LocalEvidence::Local {
            root: LocalNode::Stack { sectors, .. },
            ..
        } = &mut forged.evidence
        else {
            panic!()
        };
        match mutation {
            0 => {
                sectors.remove(0);
            }
            1 => {
                sectors[1].sample = "0".into();
            }
            2 => {
                sectors[1].sample = "-1".into();
            }
            3 => {
                sectors.swap(0, 1);
            }
            _ => unreachable!(),
        }
        assert!(
            local_projection::verify_local(&forged).is_err(),
            "mutation {mutation}"
        );
    }
    let mut forged = result.clone();
    fn erase(node: &mut LocalNode) -> bool {
        if let LocalNode::Stack {
            required_below,
            sectors,
            ..
        } = node
        {
            if !required_below.is_empty() {
                required_below.clear();
                return true;
            }
            return sectors.iter_mut().any(|sector| erase(&mut sector.child));
        }
        false
    }
    let LocalEvidence::Local { root, .. } = &mut forged.evidence else {
        panic!()
    };
    assert!(erase(root));
    assert!(local_projection::verify_local(&forged).is_err());
}

#[test]
fn deferred_work_limits_never_export_partial_frontiers_or_start_global_fallback() {
    let mut p = problem(&["a", "z"], &[], &["z^2-a"]);
    for opts in [
        LocalOptions {
            max_nodes: 0,
            fallback_to_global: true,
            ..options()
        },
        LocalOptions {
            max_refinements: 0,
            fallback_to_global: true,
            ..options()
        },
    ] {
        let result = local_projection::solve_local(&p, &opts).unwrap();
        assert_eq!(result.solution.status, Status::Incomplete);
        assert!(result.solution.cells.is_empty());
        assert!(matches!(result.evidence, LocalEvidence::Unfinished { .. }));
        assert!(local_projection::verify_local(&result).is_err());
    }
    p.limits.max_cells = Some(1);
    assert_eq!(
        local_projection::solve_local(&p, &options())
            .unwrap()
            .solution
            .status,
        Status::Incomplete
    );
    p.limits.max_cells = None;
    p.limits.wall_time_secs = 1e-9;
    assert_eq!(
        local_projection::solve_local(&p, &options())
            .unwrap()
            .solution
            .status,
        Status::Incomplete
    );
}

#[test]
fn deferred_option_requires_strict_sample_mode_and_is_backward_compatible() {
    let mut p = problem(&["x", "z"], &["x>0", "z>0"], &["z-x"]);
    assert!(
        local_projection::solve_local(
            &p,
            &LocalOptions {
                stable_sector_samples: false,
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        serde_json::to_value(&p).unwrap()["solver"]
            .get("local_deferred_projection")
            .is_none()
    );
    p.solver.local_deferred_projection = true;
    assert!(p.validate().is_err());
    p.solver.local_stable_samples = true;
    p.solver.engine = "local".into();
    let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    assert!(restored.solver.local_deferred_projection);
    let result = symgcad::solver::solve(&restored).unwrap();
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let ordinary = local(
        &p,
        &LocalOptions {
            defer_projection: false,
            ..options()
        },
    );
    let mut old = serde_json::to_value(&ordinary).unwrap();
    old["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("defer_projection");
    let decoded: LocalSolveResult = serde_json::from_value(old).unwrap();
    let LocalEvidence::Local { options, .. } = &decoded.evidence else {
        panic!()
    };
    assert!(!options.defer_projection);
    assert!(local_projection::verify_local(&decoded).unwrap().verified);
}

#[test]
fn homogeneity_preserves_the_opt_in_and_independent_base_proof() {
    let mut p = problem(&["r", "x", "z"], &["r>0", "x>0", "z>0"], &["z^2-r*x"]);
    p.solver.engine = "local".into();
    p.solver.local_deferred_projection = true;
    p.solver.local_stable_samples = true;
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    let result = symgcad::solver::solve(&p).unwrap();
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert!(reduced.problem.solver.local_deferred_projection);
    let LocalEvidence::Local {
        options, report, ..
    } = reduced.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(options.defer_projection && options.stable_sector_samples);
    assert!(report.deferred_frontiers > 0);
}

#[test]
fn retained_typed_guards_replay_and_a_toy_matches_the_independent_global_solver() {
    use symbolica::prelude::*;
    use symgcad::algebra::{Algebra, canonical};
    use symgcad::local_projection::StrictBezoutHint;

    let mut p = problem(&["a", "z"], &["a>0", "a<1", "z>0"], &["z^2-a", "z^2-a+a^2"]);
    let algebra = Algebra::new(&p.variables).unwrap();
    let originals: Vec<_> = p.split.iter().map(|p| algebra.parse(p).unwrap()).collect();
    let canonical_pair: Vec<_> = originals.iter().map(canonical).collect();
    p.solver.local_strict_bezout_hints.push(StrictBezoutHint {
        schema_version: 1,
        polynomials: std::array::from_fn(|i| algebra.format(&canonical_pair[i])),
        multipliers: std::array::from_fn(|i| {
            let scale = originals[i].lcoeff() / canonical_pair[i].lcoeff();
            (scale * Rational::from(if i == 0 { -1 } else { 1 })).to_string()
        }),
        square: "a".into(),
        multiplier: "1".into(),
    });
    let guarded = local(
        &p,
        &LocalOptions {
            strict_bezout: true,
            positive_pair_separation: false,
            domain_pruning: true,
            ..options()
        },
    );
    let LocalEvidence::Local { report, .. } = &guarded.evidence else {
        panic!()
    };
    assert!(report.strict_bezout_resultants_omitted > 0);

    let mut witness = problem(
        &["a", "z"],
        &["a>999/1000", "a<1001/1000", "z>0"],
        &["(z-1)*(z-2)*(z-3)*(z-4)*(z^2+2*z+a)+1/1000"],
    );
    witness.solver.engine = "local".into();
    witness.solver.positivity = true;
    let witnessed = local(
        &witness,
        &LocalOptions {
            descartes_witnesses: true,
            ..options()
        },
    );
    let LocalEvidence::Local { report, .. } = &witnessed.evidence else {
        panic!()
    };
    assert!(report.descartes_witness_discriminants_omitted > 0);
    assert!(report.certified_guard_removals > 0);
    assert_eq!(witnessed.solution.cells.len(), 5);

    let toy = problem(&["x", "z"], &["x>0", "z>0"], &["z^2-x"]);
    let deferred = proof(local(&toy, &options()));
    let baseline = symgcad::solver::solve(&toy).unwrap();
    assert!(symgcad::verify::verify(&baseline).unwrap().verified);
    assert_eq!(
        symgcad::comparison::compare_generic(&deferred, &baseline, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
}
