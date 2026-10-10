use symgcad::{
    Problem,
    local_projection::{LocalEvidence, LocalOptions, LocalReport, solve_local, verify_local},
    search::{
        SearchOptions, TrialOutcome, TrialStatus,
        local_progress::{LocalInventory, promotion_score},
        search,
    },
};

fn outcome(score: Option<f64>) -> TrialOutcome {
    TrialOutcome {
        status: TrialStatus::Timeout,
        elapsed_secs: 0.001,
        cells: None,
        progress_score: None,
        promotion_score: score,
        message: None,
        abandoned_prefix: None,
    }
}

#[test]
fn componentwise_complexity_increases_promotion_cost_and_extremes_stay_finite() {
    let base = LocalInventory {
        schema_version: 1,
        polynomial_count: 3,
        term_count: 9,
        max_total_degree: 4,
        max_coefficient_bits: 8,
    };
    let score = promotion_score(&base, 7).unwrap();
    for field in 0..5 {
        let mut inventory = base.clone();
        let mut refinements = 7;
        match field {
            0 => inventory.polynomial_count += 1,
            1 => inventory.term_count += 1,
            2 => inventory.max_total_degree += 1,
            3 => inventory.max_coefficient_bits += 1,
            _ => refinements += 1,
        }
        assert!(promotion_score(&inventory, refinements).unwrap() > score);
    }
    let largest = LocalInventory {
        schema_version: 1,
        polynomial_count: u64::MAX,
        term_count: u64::MAX,
        max_total_degree: u64::MAX,
        max_coefficient_bits: u64::MAX,
    };
    assert!(promotion_score(&largest, usize::MAX).unwrap().is_finite());
    assert!(promotion_score(&LocalInventory::default(), 0).is_none());
    let mut future = base;
    future.schema_version = 999;
    assert!(promotion_score(&future, 1).is_none());
}

#[test]
fn promotion_ranks_lower_complexity_but_preserves_uct_and_uncompleted_fallback() {
    let run = |enabled: bool, promotion_interval: usize| {
        search(
            &[vec![0, 1, 2]],
            &[2, 1, 0],
            &[vec![0, 1, 2], vec![2, 1, 0]],
            &SearchOptions {
                max_rollouts: 10,
                promotion_interval,
                candidate_timeouts_secs: vec![1.0, 2.0],
                ..Default::default()
            },
            |request| {
                Ok(outcome(enabled.then_some(if request.order == [2, 1, 0] {
                    1.0
                } else {
                    100.0
                })))
            },
        )
        .unwrap()
    };
    let scored = run(true, 2);
    assert_eq!(scored.trials[2].request.tier, 1);
    assert_eq!(scored.trials[2].request.order, [2, 1, 0]);
    assert!(scored.best_order.is_none());
    // Before promotions, scores must not influence UCT tree selection. One
    // invocation stops after four fresh trials, before exhausting six orders.
    let fresh = |enabled: bool| {
        search(
            &[vec![0, 1, 2]],
            &[2, 1, 0],
            &[],
            &SearchOptions {
                max_rollouts: 4,
                promotion_interval: 100,
                ..Default::default()
            },
            |request| {
                Ok(outcome(enabled.then_some(if request.order[0] == 0 {
                    0.0
                } else {
                    999.0
                })))
            },
        )
        .unwrap()
    };
    let old = fresh(false);
    let new = fresh(true);
    assert_eq!(
        old.trials
            .iter()
            .map(|t| &t.request.order)
            .collect::<Vec<_>>(),
        new.trials
            .iter()
            .map(|t| &t.request.order)
            .collect::<Vec<_>>()
    );
    assert_eq!(old.suggested_order, new.suggested_order);
    assert!(new.best_order.is_none());
}

#[test]
fn impossible_metrics_rejected_and_scores_never_choose_a_completed_winner() {
    for score in [f64::INFINITY, f64::NAN, -1.0] {
        assert!(
            search(&[vec![0]], &[0], &[], &SearchOptions::default(), |_| Ok(
                outcome(Some(score))
            ))
            .is_err()
        );
    }
    let report = search(
        &[vec![0, 1]],
        &[1, 0],
        &[vec![0, 1]],
        &SearchOptions::default(),
        |request| {
            let mut result = outcome(Some(if request.order == [0, 1] { 0.0 } else { 1e9 }));
            result.status = TrialStatus::Completed;
            result.cells = Some(1);
            result.elapsed_secs = if request.order == [0, 1] { 0.5 } else { 0.1 };
            Ok(result)
        },
    )
    .unwrap();
    assert_eq!(report.best_order, Some(vec![1, 0]));
}

#[test]
fn inventory_is_opt_in_counts_unique_registry_and_is_not_proof_evidence() {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "x>0".into(), "y>0".into()],
        split: vec!["y^2-x".into(), "y-x-1".into()],
        ..Default::default()
    };
    let plain = solve_local(&p, &LocalOptions::default()).unwrap();
    let LocalEvidence::Local { report, .. } = &plain.evidence else {
        panic!()
    };
    assert!(report.search_inventory.is_none());
    assert!(
        serde_json::to_value(report)
            .unwrap()
            .get("search_inventory")
            .is_none()
    );
    assert!(
        serde_json::to_value(&p.solver)
            .unwrap()
            .get("search_local_progress")
            .is_none()
    );
    p.solver.search_local_progress = true;
    let mut measured = solve_local(&p, &LocalOptions::default()).unwrap();
    assert!(verify_local(&measured).unwrap().verified);
    assert_eq!(
        serde_json::to_value(&plain.solution.cells).unwrap(),
        serde_json::to_value(&measured.solution.cells).unwrap()
    );
    let LocalEvidence::Local { report, .. } = &mut measured.evidence else {
        panic!()
    };
    let inventory = report.search_inventory.as_ref().unwrap();
    assert_eq!(
        inventory.polynomial_count as usize,
        measured.solution.polynomials.len()
    );
    let algebra = symgcad::algebra::Algebra::new(&measured.solution.order).unwrap();
    let polys = measured
        .solution
        .polynomials
        .iter()
        .map(|p| algebra.parse(p).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        inventory.term_count as usize,
        polys.iter().map(|p| p.nterms()).sum::<usize>()
    );
    assert!(promotion_score(inventory, report.refinements).is_some());
    // The mathematical checker must not trust or depend on ranking telemetry.
    report.search_inventory.as_mut().unwrap().schema_version = 999;
    report.search_inventory.as_mut().unwrap().polynomial_count = u64::MAX;
    assert!(verify_local(&measured).unwrap().verified);
    let old: LocalReport = serde_json::from_value(serde_json::json!({"refinements":1})).unwrap();
    assert!(old.search_inventory.is_none());
    let old: TrialOutcome = serde_json::from_value(serde_json::json!({"status":"timeout","elapsed_secs":1,"cells":null,"progress_score":null,"message":null})).unwrap();
    assert!(old.promotion_score.is_none());
}

#[test]
fn public_local_order_records_opt_in_scores_and_separates_checkpoint_contexts() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let directory = std::env::temp_dir().join(format!(
        "symgcad-local-search-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let mut p = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "y>0".into()],
        split: vec!["y^2-x".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.ordering = "mcts".into();
    // Executable hashing is part of the killable context worker. Give the
    // debug executable a realistic seed budget; the four tiny trials still
    // finish immediately without consuming the outer allowance.
    p.solver.search_time_secs = Some(30.0);
    p.solver.max_rollouts = 4;
    p.solver.checkpoint_dir = Some(directory.join("checkpoints"));
    p.limits.wall_time_secs = 150.0;
    p.limits.memory_mib = 1024;
    let mut checkpoint_paths = Vec::new();
    for enabled in [false, true] {
        p.solver.search_local_progress = enabled;
        let input = directory.join(format!("input-{enabled}.toml"));
        let output = directory.join(format!("order-{enabled}.json"));
        fs::write(&input, toml::to_string(&p).unwrap()).unwrap();
        let process = Command::new(env!("CARGO_BIN_EXE_symgcad"))
            .arg("order")
            .arg(input)
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            process.status.success(),
            "{}",
            String::from_utf8_lossy(&process.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        let trials = report["search"]["trials"].as_array().unwrap();
        assert!(!trials.is_empty());
        assert_eq!(
            trials
                .iter()
                .any(|t| t["outcome"]["promotion_score"].as_f64().is_some()),
            enabled
        );
        assert!(
            trials
                .iter()
                .all(|t| t["outcome"]["progress_score"].is_null())
        );
        checkpoint_paths.push(report["search"]["checkpoint"]["path"].clone());
    }
    assert!(checkpoint_paths.iter().all(|p| p.as_str().is_some()));
    assert_ne!(checkpoint_paths[0], checkpoint_paths[1]);
    fs::remove_dir_all(directory).unwrap();
}
