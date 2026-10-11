use symgcad::{
    Problem,
    local_projection::{self, LocalEvidence, LocalOptions},
    output::Status,
    solver::solve,
    verify::verify,
};

fn problem() -> Problem {
    let mut p = Problem {
        variables: vec!["a".into(), "z".into()],
        constraints: vec!["a>17/8".into(), "a<5/2".into(), "z>0".into()],
        split: vec!["(z-1)*(z-2)*(z^2+3*z+a)+1/1000".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.local_descartes_witnesses = true;
    p.solver.local_descartes_witness_max_terms = 8192;
    p
}

#[test]
fn configured_budget_reaches_solver_and_roundtrips_with_exact_proof() {
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
    assert_eq!(options.descartes_witness_max_terms, 8192);
    assert!(report.descartes_witness_discriminants_omitted > 0);
    let restored: symgcad::output::SolveResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(verify(&restored).unwrap().verified);
    for limit in [0, 1, 65537] {
        let mut changed = restored.clone();
        let LocalEvidence::Local { options, .. } = changed.local.as_deref_mut().unwrap() else {
            panic!()
        };
        options.descartes_witness_max_terms = limit;
        assert!(verify(&changed).is_err(), "changed evidence cap {limit}");
    }
    let mut missing = restored;
    missing.cells.pop();
    assert!(verify(&missing).is_err());
}

#[test]
fn invalid_budgets_are_rejected_even_when_witnesses_are_disabled() {
    for limit in [0, 65537, usize::MAX] {
        let mut p = problem();
        p.solver.local_descartes_witnesses = false;
        p.solver.local_descartes_witness_max_terms = limit;
        assert!(p.validate().is_err());
        p.solver.local_descartes_witness_max_terms = 4096;
        let options = LocalOptions {
            descartes_witnesses: false,
            descartes_witness_max_terms: limit,
            ..Default::default()
        };
        assert!(local_projection::solve_local(&p, &options).is_err());
    }
}

#[test]
fn absent_historical_caps_default_to_4096_and_default_serialization_stays_omitted() {
    let p = Problem::from_toml("variables=['x']\nconstraints=['x>0']\nsplit=['x^2-2']").unwrap();
    assert_eq!(p.solver.local_descartes_witness_max_terms, 4096);
    assert!(
        !toml::to_string(&p)
            .unwrap()
            .contains("local_descartes_witness_max_terms")
    );
    let options: LocalOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(options.descartes_witness_max_terms, 4096);
    assert!(
        !serde_json::to_string(&options)
            .unwrap()
            .contains("descartes_witness_max_terms")
    );
    let r = local_projection::solve_local(&p, &options).unwrap();
    let restored: local_projection::LocalSolveResult =
        serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();
    assert!(local_projection::verify_local(&restored).unwrap().verified);
    let LocalEvidence::Local { options, .. } = restored.evidence else {
        panic!()
    };
    assert_eq!(options.descartes_witness_max_terms, 4096);
}
