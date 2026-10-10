use symgcad::{
    Problem,
    local_projection::{
        LocalEvidence, LocalNode, LocalOptions, LocalReport, solve_local, verify_local,
    },
    output::{SolveResult, Status},
};

fn problem(variables: &[&str], constraints: &[&str], split: &[&str]) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|s| (*s).into()).collect(),
        constraints: constraints.iter().map(|s| (*s).into()).collect(),
        split: split.iter().map(|s| (*s).into()).collect(),
        ..Default::default()
    };
    p.limits.wall_time_secs = 15.0;
    p
}
fn options(stable: bool) -> LocalOptions {
    LocalOptions {
        stable_sector_samples: stable,
        batch_sibling_discovery: false,
        reuse_learned_cuts: false,
        fallback_to_global: false,
        ..Default::default()
    }
}
fn report(result: &SolveResult) -> &LocalReport {
    let Some(LocalEvidence::Local { report, .. }) = result.local.as_deref() else {
        panic!("missing local proof")
    };
    report
}
fn certified(p: &Problem, stable: bool) -> SolveResult {
    let local = solve_local(p, &options(stable)).unwrap();
    assert_eq!(
        local.solution.status,
        Status::CompleteGeneric,
        "{:?}",
        local.solution.diagnostic
    );
    assert!(verify_local(&local).unwrap().verified);
    let mut solution = local.solution;
    solution.local = Some(Box::new(local.evidence));
    solution
}

#[test]
fn stable_samples_preserve_coverage_and_reduce_visited_child_work() {
    let problems = [
        problem(
            &["x", "y", "z", "w"],
            &[],
            &["x", "y", "z-x", "w^2-z", "w-y", "w-2"],
        ),
        problem(&["x", "y", "z"], &[], &["y^2-x", "z^2-y", "z-x", "z-2"]),
        problem(
            &["x", "y", "z"],
            &["x>0", "y>0", "z>0"],
            &["z^2-x*y", "z-x", "z-y", "z-1"],
        ),
    ];
    let mut saved_nodes = 0;
    for p in problems {
        let stable = certified(&p, true);
        let plain = certified(&p, false);
        let a = report(&stable);
        let b = report(&plain);
        eprintln!(
            "stable={}ref/{}nodes/{}hits plain={}ref/{}nodes",
            a.refinements,
            a.speculative_nodes,
            a.stable_sample_hits,
            b.refinements,
            b.speculative_nodes
        );
        assert!(a.stable_sample_hits > 0);
        assert!(a.stable_sample_peak_bytes <= 32 * 1024);
        assert_eq!(b.stable_sample_hits, 0);
        assert_eq!(
            symgcad::comparison::compare_generic(&stable, &plain, &symgcad::Limits::default())
                .unwrap()
                .equivalent,
            Some(true)
        );
        saved_nodes += b.speculative_nodes.saturating_sub(a.speculative_nodes);
    }
    assert!(
        saved_nodes > 0,
        "fixture did not demonstrate reused child work"
    );
}

#[test]
fn forged_samples_on_a_boundary_or_outside_the_sector_are_rejected() {
    let p = problem(&["x"], &[], &["x-1"]);
    let original = solve_local(&p, &options(true)).unwrap();
    assert!(verify_local(&original).unwrap().verified);
    for value in ["1", "2"] {
        let mut changed = original.clone();
        let LocalEvidence::Local {
            root: LocalNode::Stack { sectors, .. },
            ..
        } = &mut changed.evidence
        else {
            panic!()
        };
        sectors[0].sample = value.into();
        let error = verify_local(&changed).unwrap_err();
        assert!(error.to_string().contains("strictly inside"), "{error:#}");
    }
}

#[test]
fn stable_samples_never_replace_exhaustive_sectors_or_resource_checks() {
    let p = problem(&["x", "y"], &[], &["y^2-x", "y-1"]);
    let mut local = solve_local(&p, &options(true)).unwrap();
    assert!(verify_local(&local).unwrap().verified);
    let LocalEvidence::Local {
        root: LocalNode::Stack { sectors, .. },
        ..
    } = &mut local.evidence
    else {
        panic!()
    };
    sectors.pop();
    assert!(verify_local(&local).is_err());
    for o in [
        LocalOptions {
            max_nodes: 0,
            ..options(true)
        },
        LocalOptions {
            max_refinements: 0,
            ..options(true)
        },
    ] {
        let stopped = solve_local(&p, &o).unwrap();
        assert_eq!(stopped.solution.status, Status::Incomplete);
        assert!(stopped.solution.cells.is_empty());
        assert!(verify_local(&stopped).is_err());
    }
}

#[test]
fn sampling_option_roundtrips_and_survives_homogeneity() {
    let mut p = problem(&["r", "x", "y"], &["r>0", "x>0", "y>0"], &["y^2-r*x"]);
    assert!(
        serde_json::to_value(&p).unwrap()["solver"]
            .get("local_stable_samples")
            .is_none()
    );
    p.solver.engine = "local".into();
    p.solver.local_stable_samples = true;
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.homogeneous_anchor = Some("r".into());
    let p = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    assert!(p.solver.local_stable_samples);
    let result = symgcad::solver::solve(&p).unwrap();
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert!(reduced.problem.solver.local_stable_samples);
    let LocalEvidence::Local { options, .. } = reduced.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(options.stable_sector_samples);
}

#[test]
fn historical_options_default_off_without_changing_certificate_meaning() {
    let original = solve_local(&problem(&["x", "y"], &[], &["y^2-x"]), &options(false)).unwrap();
    let mut value = serde_json::to_value(original).unwrap();
    value["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("stable_sector_samples");
    let restored: symgcad::local_projection::LocalSolveResult =
        serde_json::from_value(value).unwrap();
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert!(!options.stable_sector_samples);
    assert!(verify_local(&restored).unwrap().verified);
}
