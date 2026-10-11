use symgcad::{
    Problem,
    local_projection::{self, LocalEvidence, LocalOptions, LocalReport},
    output::{SolveResult, Status},
    solver::solve,
    verify::verify,
};

fn problem() -> Problem {
    let mut p = Problem {
        variables: vec!["a".into(), "z".into()],
        split: vec!["((a-1)*z+a)^2".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.local_subtree_cache_entry_nodes = 1024;
    p
}

#[test]
fn configured_entry_policy_roundtrips_and_remains_independent_of_proof_acceptance() {
    let p = Problem::from_toml(&toml::to_string(&problem()).unwrap()).unwrap();
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(verify(&result).unwrap().verified);
    let LocalEvidence::Local {
        options, report, ..
    } = result.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert_eq!(options.subtree_cache_entry_nodes, 1024);
    assert_eq!(report.subtree_cache_entry_nodes, 1024);
    assert_eq!(report.subtree_cache_budget_bytes, 32 * 1024 * 1024);
    assert_eq!(report.subtree_cache_node_limit, 32768);
    assert!(result.cells.len() > 1); // Preserve the squared polynomial's interior seam.

    let restored: SolveResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(verify(&restored).unwrap().verified);
    let mut changed_policy = restored.clone();
    let LocalEvidence::Local { options, .. } = changed_policy.local.as_deref_mut().unwrap() else {
        panic!()
    };
    // Valid cache admission settings are exploration controls, not assumptions
    // licensing any omitted boundary or proof obligation.
    options.subtree_cache_entry_nodes = 128;
    assert!(verify(&changed_policy).unwrap().verified);
    for limit in [0, 127, 4097] {
        let mut invalid_policy = restored.clone();
        let LocalEvidence::Local { options, .. } = invalid_policy.local.as_deref_mut().unwrap()
        else {
            panic!()
        };
        options.subtree_cache_entry_nodes = limit;
        assert!(
            verify(&invalid_policy).is_err(),
            "invalid saved entry cap {limit}"
        );
    }
    let mut missing_cell = restored;
    missing_cell.cells.pop();
    assert!(verify(&missing_cell).is_err());
}

#[test]
fn homogeneous_reduction_preserves_the_requested_entry_policy() {
    let mut p = Problem {
        variables: vec!["r".into(), "x".into()],
        constraints: vec!["r>0".into(), "x>0".into()],
        split: vec!["(x-r)^2".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.local_subtree_cache_entry_nodes = 4096;
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.homogeneous_anchor = Some("r".into());
    let result = solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(verify(&result).unwrap().verified);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert_eq!(reduced.problem.solver.local_subtree_cache_entry_nodes, 4096);
    let LocalEvidence::Local {
        options, report, ..
    } = reduced.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert_eq!(options.subtree_cache_entry_nodes, 4096);
    assert_eq!(report.subtree_cache_entry_nodes, 4096);
}

#[test]
fn invalid_entry_limits_are_rejected_without_enabling_cache_reuse() {
    for limit in [0, 127, 4097, usize::MAX] {
        let mut p = problem();
        p.solver.local_reuse_subtrees = false;
        p.solver.local_subtree_cache_entry_nodes = limit;
        assert!(p.validate().is_err());
        assert!(
            Problem::from_toml(&format!(
                "variables=['x']\n[solver]\nlocal_subtree_cache_entry_nodes={limit}"
            ))
            .is_err()
        );
        p.solver.local_subtree_cache_entry_nodes = 128;
        let options = LocalOptions {
            reuse_completed_subtrees: false,
            subtree_cache_entry_nodes: limit,
            ..Default::default()
        };
        assert!(local_projection::solve_local(&p, &options).is_err());
    }
}

#[test]
fn historical_omissions_restore_128_and_default_settings_remain_omitted() {
    let p = Problem::from_toml("variables=['x']\nsplit=['x^2-2']").unwrap();
    assert_eq!(p.solver.local_subtree_cache_entry_nodes, 128);
    assert!(
        !toml::to_string(&p)
            .unwrap()
            .contains("local_subtree_cache_entry_nodes")
    );
    let options: LocalOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(options.subtree_cache_entry_nodes, 128);
    assert!(
        !serde_json::to_string(&options)
            .unwrap()
            .contains("subtree_cache_entry_nodes")
    );
    let report: LocalReport = serde_json::from_str("{}").unwrap();
    assert_eq!(report.subtree_cache_entry_nodes, 128);
    let result = local_projection::solve_local(&p, &options).unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("subtree_cache_entry_nodes"));
    let restored: local_projection::LocalSolveResult = serde_json::from_str(&json).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
}
