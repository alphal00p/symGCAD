use symgcad::{
    Problem,
    local_projection::{LocalEvidence, LocalNode, LocalOptions, solve_local, verify_local},
    output::{SolveResult, Status},
};

fn problem() -> Problem {
    Problem {
        variables: ["x", "y", "z", "w"].iter().map(|s| (*s).into()).collect(),
        split: ["x", "y", "z-x", "w^2-z", "w-y", "w-2"]
            .iter()
            .map(|s| (*s).into())
            .collect(),
        ..Default::default()
    }
}

fn options(reuse: bool) -> LocalOptions {
    LocalOptions {
        reuse_learned_cuts: reuse,
        reuse_completed_subtrees: false,
        ..Default::default()
    }
}

fn certified(p: &Problem, options: &LocalOptions) -> SolveResult {
    let result = solve_local(p, options).unwrap();
    assert_eq!(result.solution.status, Status::CompleteGeneric);
    assert!(verify_local(&result).unwrap().verified);
    let mut solution = result.solution;
    solution.local = Some(Box::new(result.evidence));
    solution
}

#[test]
fn learned_cut_hints_reduce_repeated_refinement_and_preserve_full_generic_coverage() {
    let p = problem();
    let cached = certified(&p, &options(true));
    let plain = certified(&p, &options(false));
    let LocalEvidence::Local {
        report: cached_report,
        ..
    } = cached.local.as_deref().unwrap()
    else {
        panic!()
    };
    let LocalEvidence::Local {
        report: plain_report,
        ..
    } = plain.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(
        cached_report.learned_cut_cache_hits > 0,
        "{cached_report:?}"
    );
    assert!(
        cached_report.learned_cut_seeded_cuts > 0,
        "{cached_report:?}"
    );
    assert!(
        cached_report.refinements < plain_report.refinements,
        "{cached_report:?} vs {plain_report:?}"
    );
    assert!(cached_report.learned_cut_cache_peak_bytes <= 8 * 1024 * 1024);
    assert!(cached_report.learned_cut_cache_retained_ids <= 65_536);
    assert!(cached_report.learned_cut_cache_retained_entries <= 4096);
    assert_eq!(plain_report.learned_cut_cache_hits, 0);
    assert_eq!(
        symgcad::comparison::compare_generic(&cached, &plain, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn hints_cannot_replace_exhaustive_sectors_or_turn_budget_exhaustion_into_success() {
    let mut p = problem();
    let original = solve_local(&p, &options(true)).unwrap();
    assert!(verify_local(&original).unwrap().verified);
    let mut changed = original.clone();
    let LocalEvidence::Local {
        root: LocalNode::Stack { sectors, .. },
        ..
    } = &mut changed.evidence
    else {
        panic!()
    };
    sectors.pop();
    assert!(verify_local(&changed).is_err());
    p.limits.max_cells = Some(1);
    let partial = solve_local(&p, &options(true)).unwrap();
    assert_eq!(partial.solution.status, Status::Incomplete);
    assert!(partial.solution.cells.is_empty());
    assert!(verify_local(&partial).is_err());
    let mut old = serde_json::to_value(&original).unwrap();
    old["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("reuse_learned_cuts");
    let restored: symgcad::local_projection::LocalSolveResult =
        serde_json::from_value(old).unwrap();
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert!(!options.reuse_learned_cuts);
    assert!(verify_local(&restored).unwrap().verified);
}

#[test]
fn learned_cut_option_is_explicit_and_survives_homogeneous_reduction() {
    let mut p = Problem {
        variables: vec!["r".into(), "x".into(), "z".into()],
        constraints: vec!["r>0".into(), "x>0".into(), "z>0".into()],
        split: vec!["z^2-r*x".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    assert!(
        serde_json::to_value(&p).unwrap()["solver"]
            .get("local_reuse_learned_cuts")
            .is_none()
    );
    p.solver.local_reuse_learned_cuts = false;
    let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    let result = symgcad::solver::solve(&restored).unwrap();
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert!(!reduced.problem.solver.local_reuse_learned_cuts);
    let LocalEvidence::Local {
        options, report, ..
    } = reduced.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(!options.reuse_learned_cuts);
    assert_eq!(report.learned_cut_cache_hits, 0);
}
