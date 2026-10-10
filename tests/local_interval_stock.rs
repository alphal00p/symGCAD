use symgcad::{
    Problem,
    local_projection::{self, LocalEvidence, LocalOptions},
    output::Status,
};

fn problem() -> Problem {
    let mut p = Problem {
        variables: vec!["a".into(), "z".into()],
        constraints: vec!["a>999/1000".into(), "a<1001/1000".into(), "z>0".into()],
        split: vec!["(z-1)*(z-2)*(z^2-z+a)+1/1000".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.local_descartes_intervals = true;
    p.limits.wall_time_secs = 20.;
    p
}

#[test]
fn configured_limit_propagates_and_preserves_verified_cells() {
    let mut p = problem();
    let small = symgcad::solver::solve(&p).unwrap();
    assert_eq!(small.status, Status::CompleteGeneric);
    let mut historical = serde_json::to_value(&small).unwrap();
    historical["local"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("descartes_interval_template_limit");
    let historical = serde_json::from_value(historical).unwrap();
    assert!(symgcad::verify::verify(&historical).unwrap().verified);
    p.solver.local_descartes_interval_template_limit = 8;
    let large = symgcad::solver::solve(&p).unwrap();
    assert_eq!(large.status, Status::CompleteGeneric);
    assert!(symgcad::verify::verify(&large).unwrap().verified);
    assert_eq!(small.cells, large.cells);
    let LocalEvidence::Local {
        options, report, ..
    } = large.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert_eq!(options.descartes_interval_template_limit, 8);
    assert_eq!(report.descartes_interval_template_limit, 8);
    assert!(report.descartes_interval_discriminants_omitted > 0);
    assert_eq!(
        symgcad::comparison::compare_generic(&small, &large, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
    // Admission history is not proof authority: any supplied complete witness
    // is checked algebraically, even with the historical discovery budget.
    let mut replay = large;
    let LocalEvidence::Local { options, .. } = replay.local.as_deref_mut().unwrap() else {
        panic!()
    };
    options.descartes_interval_template_limit = 2;
    assert!(symgcad::verify::verify(&replay).unwrap().verified);
}

#[test]
fn old_missing_field_defaults_to_two_and_invalid_limits_are_rejected() {
    let p = problem();
    let json = serde_json::to_value(&p.solver).unwrap();
    assert!(
        json.get("local_descartes_interval_template_limit")
            .is_none()
    );
    let restored: symgcad::SolverOptions = serde_json::from_value(json).unwrap();
    assert_eq!(restored.local_descartes_interval_template_limit, 2);
    let mut old_options = serde_json::to_value(LocalOptions::default()).unwrap();
    assert!(
        old_options
            .get("descartes_interval_template_limit")
            .is_none()
    );
    assert!(old_options.get("defer_projection").is_none());
    old_options
        .as_object_mut()
        .unwrap()
        .remove("descartes_interval_template_limit");
    let restored: LocalOptions = serde_json::from_value(old_options).unwrap();
    assert_eq!(restored.descartes_interval_template_limit, 2);
    for invalid in [0, 1, 9, usize::MAX] {
        let mut changed = p.clone();
        changed.solver.local_descartes_interval_template_limit = invalid;
        assert!(changed.validate().is_err());
        assert!(
            local_projection::solve_local(
                &p,
                &LocalOptions {
                    descartes_interval_template_limit: invalid,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let mut result = symgcad::solver::solve(&p).unwrap();
    let LocalEvidence::Local { options, .. } = result.local.as_deref_mut().unwrap() else {
        panic!()
    };
    options.descartes_interval_template_limit = 9;
    assert!(symgcad::verify::verify(&result).is_err());
}

#[test]
fn enlarged_interval_stock_still_disables_durable_template_free_archive() {
    let mut p = problem();
    p.solver.local_descartes_interval_template_limit = 8;
    p.solver.local_hint_archive = true;
    let path = std::env::temp_dir().join(format!("symgcad-interval-stock-{}", std::process::id()));
    assert!(!path.exists());
    std::fs::create_dir(&path).unwrap();
    p.solver.checkpoint_dir = Some(path.clone());
    let result = symgcad::solver::solve(&p).unwrap();
    let LocalEvidence::Local { report, .. } = result.local.as_deref().unwrap() else {
        panic!()
    };
    let archive = report.hint_archive.as_ref().unwrap();
    assert!(!archive.active);
    assert!(
        archive
            .reason
            .as_deref()
            .unwrap()
            .contains("admission ledger")
    );
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    std::fs::remove_dir_all(path).unwrap();
}
