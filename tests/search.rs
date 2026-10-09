use symgcad::search::{SearchOptions, SearchStop, TrialOutcome, TrialStatus, search};

fn outcome(status: TrialStatus, elapsed_secs: f64) -> TrialOutcome {
    TrialOutcome {
        status,
        elapsed_secs,
        cells: Some(1),
        progress_score: None,
        message: None,
    }
}

#[test]
fn finds_best_completed_order_and_preserves_parameter_groups() {
    let report = search(
        &[vec![0], vec![1, 2]],
        &[2, 1, 0],
        &[vec![0, 1, 2]],
        &SearchOptions::default(),
        |request| {
            assert_eq!(request.order[0], 0);
            Ok(outcome(
                TrialStatus::Completed,
                if request.order == [0, 2, 1] { 1.0 } else { 5.0 },
            ))
        },
    )
    .unwrap();
    assert_eq!(report.best_order, Some(vec![0, 2, 1]));
    assert_eq!(report.best_elapsed_secs, Some(1.0));
    assert_eq!(report.stopped, SearchStop::Exhausted);
    assert_eq!(report.trials.len(), 2);
}

#[test]
fn timeouts_are_censored_and_promoted_without_claiming_completion() {
    let report = search(
        &[vec![0, 1]],
        &[1, 0],
        &[vec![0, 1]],
        &SearchOptions::default(),
        |request| Ok(outcome(TrialStatus::Timeout, request.timeout_secs)),
    )
    .unwrap();
    assert!(report.best_order.is_none());
    assert!(report.best_elapsed_secs.is_none());
    assert_eq!(report.stopped, SearchStop::Exhausted);
    assert_eq!(report.trials.len(), 6);
    for tier in 0..3 {
        assert_eq!(
            report
                .trials
                .iter()
                .filter(|r| r.request.tier == tier)
                .count(),
            2
        );
    }
}

#[test]
fn structural_pruning_is_not_a_completed_solution() {
    let report = search(
        &[vec![0, 1]],
        &[1, 0],
        &[],
        &SearchOptions::default(),
        |_| Ok(outcome(TrialStatus::Pruned, 0.01)),
    )
    .unwrap();
    assert!(report.best_order.is_none());
    assert_eq!(report.trials.len(), 2);
}

#[test]
fn fixed_seed_reproduces_trial_orders() {
    let opts = SearchOptions {
        max_rollouts: 32,
        ..SearchOptions::default()
    };
    let run = || {
        search(
            &[vec![0, 1, 2, 3, 4]],
            &[4, 3, 2, 1, 0],
            &[],
            &opts,
            |request| {
                let cost = request
                    .order
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| (i + 1) * (v + 1))
                    .sum::<usize>() as f64;
                Ok(outcome(TrialStatus::Completed, cost))
            },
        )
        .unwrap()
        .trials
        .into_iter()
        .map(|r| (r.request.order, r.request.tier))
        .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn every_worker_deadline_is_clipped_to_remaining_search_budget() {
    let opts = SearchOptions {
        wall_time_secs: 0.1,
        max_rollouts: 2,
        ..SearchOptions::default()
    };
    let report = search(&[vec![0, 1, 2]], &[2, 1, 0], &[], &opts, |request| {
        assert!(request.timeout_secs > 0.0 && request.timeout_secs <= 0.1);
        Ok(outcome(TrialStatus::Completed, 0.01))
    })
    .unwrap();
    assert!(report.trials.len() <= 2);
    assert_eq!(report.iterations, 2);
}

#[test]
fn rejects_illegal_seed_and_invalid_worker_metrics() {
    let opts = SearchOptions::default();
    assert!(
        search(
            &[vec![0], vec![1]],
            &[1, 0],
            &[vec![1, 0]],
            &opts,
            |_| panic!("invalid order must not be evaluated")
        )
        .is_err()
    );
    assert!(
        search(&[vec![0]], &[0], &[], &opts, |_| Ok(outcome(
            TrialStatus::Completed,
            f64::NAN
        )))
        .is_err()
    );
}

#[test]
fn campaign_defaults_reserve_most_time_for_the_selected_exact_solve() {
    assert_eq!(SearchOptions::for_campaign(100.0).wall_time_secs, 20.0);
    assert_eq!(SearchOptions::for_campaign(3600.0).wall_time_secs, 300.0);
    assert_eq!(
        SearchOptions::default().candidate_timeouts_secs,
        vec![1.0, 10.0, 60.0]
    );
}
