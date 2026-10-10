use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use symgcad::search::{
    SearchCheckpointOptions, SearchOptions, TrialOutcome, TrialStatus, search, search_checkpointed,
};

static NONCE: AtomicU64 = AtomicU64::new(0);
fn directory() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "symgcad-search-test-{}-{}",
        std::process::id(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    p
}
fn options() -> SearchOptions {
    SearchOptions {
        max_rollouts: 32,
        wall_time_secs: 10.0,
        candidate_timeouts_secs: vec![0.01, 0.02, 0.03],
        promotion_interval: 2,
        ..Default::default()
    }
}
fn outcome(status: TrialStatus) -> TrialOutcome {
    TrialOutcome {
        status,
        elapsed_secs: 0.001,
        cells: Some(3),
        progress_score: Some(1.0),
        promotion_score: None,
        message: None,
        abandoned_prefix: None,
    }
}

#[test]
fn promotion_scores_survive_replay_but_policy_context_changes_start_fresh() {
    let dir = directory();
    let mut opts = options();
    opts.max_rollouts = 1;
    let mut context = checkpoint(&dir);
    context.context = format!("exact-problem:{}", symgcad::search::local_progress::POLICY);
    let callback = |_: &symgcad::search::TrialRequest| {
        let mut r = outcome(TrialStatus::Timeout);
        r.promotion_score = Some(17.0);
        Ok(r)
    };
    search_checkpointed(&[vec![0, 1]], &[1, 0], &[], &opts, &context, callback).unwrap();
    let resumed = search_checkpointed(&[vec![0, 1]], &[1, 0], &[], &opts, &context, |_| {
        Ok(outcome(TrialStatus::Timeout))
    })
    .unwrap();
    assert_eq!(resumed.checkpoint.as_ref().unwrap().loaded_trials, 1);
    assert_eq!(resumed.trials[0].outcome.promotion_score, Some(17.0));
    assert!(resumed.best_order.is_none());
    context.context = "exact-problem:local-inventory-log-future-policy".into();
    let fresh = search_checkpointed(&[vec![0, 1]], &[1, 0], &[], &opts, &context, |_| {
        Ok(outcome(TrialStatus::Timeout))
    })
    .unwrap();
    assert_eq!(fresh.checkpoint.unwrap().loaded_trials, 0);
    assert!(fresh.trials[0].outcome.promotion_score.is_none());
    fs::remove_dir_all(dir).unwrap();
}
fn checkpoint(p: &Path) -> SearchCheckpointOptions {
    SearchCheckpointOptions {
        directory: p.into(),
        context: "normalized math and exact solver version v1".into(),
    }
}
fn payload_file(p: &Path) -> PathBuf {
    fs::read_dir(p.join("searches"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .unwrap()
}
fn mutate(p: &Path, change: impl FnOnce(&mut serde_json::Value), checksum: bool) {
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(p).unwrap()).unwrap();
    change(&mut v);
    if checksum {
        v["sha256"] = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&v["archive"]).unwrap())
        )
        .into();
    }
    fs::write(p, serde_json::to_vec(&v).unwrap()).unwrap();
}

#[test]
fn checkpoint_child() {
    let Some(dir) = std::env::var_os("SYMGCAD_SEARCH_CHILD_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let mut calls = 0;
    search_checkpointed(
        &[vec![0, 1, 2, 3]],
        &[3, 2, 1, 0],
        &[],
        &options(),
        &checkpoint(&dir),
        |_| {
            calls += 1;
            if calls == 5 {
                fs::write(
                    dir.join("ready"),
                    b"worker interrupted after four published trials",
                )
                .unwrap();
                loop {
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
            Ok(outcome(TrialStatus::Timeout))
        },
    )
    .unwrap();
}

#[test]
fn killed_worker_resumes_exact_rng_tree_and_promotions() {
    let dir = directory();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "checkpoint_child", "--nocapture"])
        .env("SYMGCAD_SEARCH_CHILD_DIR", &dir)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !dir.join("ready").exists() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(5));
    }
    let ready = dir.join("ready").exists();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(ready, "child must publish checkpoint before kill");
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(payload_file(&dir)).unwrap()).unwrap();
    assert_eq!(saved["archive"]["records"].as_array().unwrap().len(), 4);
    // Unpublished temporary output from an interrupted rename must be ignored.
    fs::write(
        dir.join("searches/.search-interrupted.tmp"),
        b"partial garbage",
    )
    .unwrap();
    let mut fresh = Vec::new();
    let resumed = search_checkpointed(
        &[vec![0, 1, 2, 3]],
        &[3, 2, 1, 0],
        &[],
        &options(),
        &checkpoint(&dir),
        |r| {
            fresh.push((r.order.clone(), r.tier));
            Ok(outcome(TrialStatus::Timeout))
        },
    )
    .unwrap();
    let plain = search(&[vec![0, 1, 2, 3]], &[3, 2, 1, 0], &[], &options(), |_| {
        Ok(outcome(TrialStatus::Timeout))
    })
    .unwrap();
    let inventory = |r: &symgcad::search::SearchReport| {
        r.trials
            .iter()
            .map(|t| {
                (
                    t.request.order.clone(),
                    t.request.elimination_prefix.clone(),
                    t.request.tier,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(inventory(&resumed), inventory(&plain));
    assert_eq!(resumed.iterations, plain.iterations);
    assert_eq!(fresh.len() + 4, plain.trials.len());
    assert_eq!(resumed.checkpoint.unwrap().loaded_trials, 4);
    assert!(resumed.best_order.is_none());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn forged_completed_claim_is_only_a_hint_until_fresh_exact_success() {
    let dir = directory();
    let mut opts = options();
    opts.max_rollouts = 1;
    search_checkpointed(&[vec![0]], &[0], &[], &opts, &checkpoint(&dir), |_| {
        Ok(outcome(TrialStatus::Completed))
    })
    .unwrap();
    mutate(
        &payload_file(&dir),
        |v| {
            v["archive"]["records"][0]["outcome"]["cells"] = 999_999.into();
        },
        true,
    );
    let rejected = search_checkpointed(&[vec![0]], &[0], &[], &opts, &checkpoint(&dir), |r| {
        assert!(r.timeout_secs <= opts.wall_time_secs);
        Ok(outcome(TrialStatus::Failed))
    })
    .unwrap();
    assert!(rejected.best_order.is_none());
    assert_eq!(rejected.trials[0].outcome.status, TrialStatus::Unverified);
    assert_eq!(rejected.revalidations.len(), 1);
    assert_eq!(rejected.checkpoint.unwrap().loaded_trials, 1);
    let recovered = search_checkpointed(&[vec![0]], &[0], &[], &opts, &checkpoint(&dir), |_| {
        Ok(outcome(TrialStatus::Completed))
    })
    .unwrap();
    assert_eq!(recovered.best_order, Some(vec![0]));
    assert_eq!(
        recovered.revalidations.last().unwrap().outcome.cells,
        Some(3)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn checksum_context_version_and_scheduler_tampering_start_fresh() {
    for kind in 0..4 {
        let dir = directory();
        let mut opts = options();
        opts.max_rollouts = 1;
        search_checkpointed(
            &[vec![0, 1]],
            &[1, 0],
            &[vec![0, 1]],
            &opts,
            &checkpoint(&dir),
            |_| Ok(outcome(TrialStatus::Timeout)),
        )
        .unwrap();
        mutate(
            &payload_file(&dir),
            |v| match kind {
                0 => v["sha256"] = "not a checksum".into(),
                1 => v["archive"]["context_sha256"] = "wrong mathematical context".into(),
                2 => v["archive"]["schema_version"] = 999.into(),
                _ => v["archive"]["records"][0]["request"]["order"] = serde_json::json!([1, 0]),
            },
            kind != 0,
        );
        let mut calls = 0;
        let r = search_checkpointed(
            &[vec![0, 1]],
            &[1, 0],
            &[vec![0, 1]],
            &opts,
            &checkpoint(&dir),
            |_| {
                calls += 1;
                Ok(outcome(TrialStatus::Timeout))
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert!(r.best_order.is_none());
        let c = r.checkpoint.unwrap();
        assert_eq!(c.loaded_trials, 0);
        assert!(c.rejected.is_some());
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn changed_context_uses_separate_checkpoint_and_current_rollout_bound() {
    let dir = directory();
    let mut opts = options();
    opts.max_rollouts = 3;
    let old = search_checkpointed(
        &[vec![0, 1, 2]],
        &[2, 1, 0],
        &[],
        &opts,
        &checkpoint(&dir),
        |_| Ok(outcome(TrialStatus::Timeout)),
    )
    .unwrap();
    let mut other = checkpoint(&dir);
    other.context.push_str("; different domain");
    let new = search_checkpointed(&[vec![0, 1, 2]], &[2, 1, 0], &[], &opts, &other, |_| {
        Ok(outcome(TrialStatus::Timeout))
    })
    .unwrap();
    assert_ne!(
        old.checkpoint.as_ref().unwrap().path,
        new.checkpoint.as_ref().unwrap().path
    );
    assert_eq!(new.checkpoint.unwrap().loaded_trials, 0);
    opts.max_rollouts = 1;
    let clipped = search_checkpointed(
        &[vec![0, 1, 2]],
        &[2, 1, 0],
        &[],
        &opts,
        &checkpoint(&dir),
        |_| Ok(outcome(TrialStatus::Timeout)),
    )
    .unwrap();
    assert_eq!(clipped.iterations, 1);
    assert!(
        clipped
            .checkpoint
            .unwrap()
            .rejected
            .unwrap()
            .contains("bounds")
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn oversized_truncated_and_symlink_archives_are_not_loaded() {
    for kind in 0..3 {
        let dir = directory();
        let mut opts = options();
        opts.max_rollouts = 1;
        search_checkpointed(&[vec![0]], &[0], &[], &opts, &checkpoint(&dir), |_| {
            Ok(outcome(TrialStatus::Completed))
        })
        .unwrap();
        let path = payload_file(&dir);
        match kind {
            0 => fs::write(&path, b"{\"archive\":").unwrap(),
            1 => {
                std::fs::File::create(&path)
                    .unwrap()
                    .set_len(8 * 1024 * 1024 + 1)
                    .unwrap();
            }
            _ => {
                let copy = dir.join("outside.json");
                fs::copy(&path, &copy).unwrap();
                fs::remove_file(&path).unwrap();
                std::os::unix::fs::symlink(&copy, &path).unwrap();
            }
        }
        let r = search_checkpointed(&[vec![0]], &[0], &[], &opts, &checkpoint(&dir), |_| {
            Ok(outcome(TrialStatus::Failed))
        })
        .unwrap();
        assert!(r.best_order.is_none());
        let c = r.checkpoint.unwrap();
        assert_eq!(c.loaded_trials, 0);
        assert!(c.rejected.is_some());
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn callback_failure_preserves_last_published_iteration() {
    let dir = directory();
    let mut calls = 0;
    let result = search_checkpointed(
        &[vec![0, 1, 2]],
        &[2, 1, 0],
        &[],
        &options(),
        &checkpoint(&dir),
        |_| {
            calls += 1;
            if calls == 3 {
                anyhow::bail!("interrupted worker")
            };
            Ok(outcome(TrialStatus::Timeout))
        },
    );
    assert!(result.is_err());
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(payload_file(&dir)).unwrap()).unwrap();
    assert_eq!(saved["archive"]["records"].as_array().unwrap().len(), 2);
    let r = search_checkpointed(
        &[vec![0, 1, 2]],
        &[2, 1, 0],
        &[],
        &options(),
        &checkpoint(&dir),
        |_| Ok(outcome(TrialStatus::Timeout)),
    )
    .unwrap();
    assert_eq!(r.checkpoint.unwrap().loaded_trials, 2);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unwritable_checkpoint_does_not_change_search_truth() {
    let dir = directory();
    fs::write(dir.join("searches"), b"not a directory").unwrap();
    let mut opts = options();
    opts.max_rollouts = 1;
    let r = search_checkpointed(&[vec![0]], &[0], &[], &opts, &checkpoint(&dir), |_| {
        Ok(outcome(TrialStatus::Completed))
    })
    .unwrap();
    assert_eq!(r.best_order, Some(vec![0]));
    assert!(r.checkpoint.unwrap().write_error.is_some());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn revalidations_share_the_current_callback_budget_and_deadline() {
    let dir = directory();
    let mut opts = options();
    opts.max_rollouts = 3;
    search_checkpointed(
        &[vec![0, 1, 2]],
        &[2, 1, 0],
        &[],
        &opts,
        &checkpoint(&dir),
        |_| Ok(outcome(TrialStatus::Completed)),
    )
    .unwrap();
    opts.wall_time_secs = 0.05;
    let mut calls = 0;
    let r = search_checkpointed(
        &[vec![0, 1, 2]],
        &[2, 1, 0],
        &[],
        &opts,
        &checkpoint(&dir),
        |request| {
            calls += 1;
            assert!(request.timeout_secs <= 0.05);
            Ok(outcome(TrialStatus::Timeout))
        },
    )
    .unwrap();
    assert!(calls > 0 && calls <= opts.max_rollouts);
    assert!(r.best_order.is_none());
    assert_eq!(r.revalidations.len(), calls);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn malformed_caller_groups_are_rejected_before_archive_indexing() {
    let dir = directory();
    assert!(
        search_checkpointed(
            &[vec![9]],
            &[0],
            &[],
            &options(),
            &checkpoint(&dir),
            |_| panic!("invalid context must not reach worker")
        )
        .is_err()
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn failed_revalidation_keeps_the_best_historical_restart_hint_only() {
    let dir = directory();
    let mut opts = options();
    opts.max_rollouts = 2;
    let seeds = [vec![0, 1], vec![1, 0]];
    search_checkpointed(
        &[vec![0, 1]],
        &[1, 0],
        &seeds,
        &opts,
        &checkpoint(&dir),
        |r| {
            let mut o = outcome(TrialStatus::Completed);
            o.elapsed_secs = if r.order == [1, 0] { 0.001 } else { 0.01 };
            Ok(o)
        },
    )
    .unwrap();
    let resumed = search_checkpointed(
        &[vec![0, 1]],
        &[1, 0],
        &seeds,
        &opts,
        &checkpoint(&dir),
        |_| Ok(outcome(TrialStatus::Failed)),
    )
    .unwrap();
    assert!(resumed.best_order.is_none());
    assert!(resumed.best_elapsed_secs.is_none());
    assert_eq!(resumed.suggested_order, [1, 0]);
    assert_eq!(resumed.revalidations.len(), 2);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn public_cli_resumes_the_exact_context_then_exports_a_fresh_verified_result() {
    let dir = directory();
    let input = dir.join("problem.toml");
    let mut problem = symgcad::Problem {
        variables: vec!["z".into()],
        constraints: vec!["z>0".into(), "1-z>0".into()],
        ..Default::default()
    };
    problem.solver.ordering = "mcts".into();
    problem.solver.max_rollouts = 2;
    // Context construction hashes the full debug executable inside the bounded
    // seed worker. Leave its maximum five-second setup allowance available on
    // loaded CI hosts; this one-order search still exits as soon as exhausted.
    // Dedicated scheduler tests exercise the short-deadline behavior.
    problem.solver.search_time_secs = Some(30.0);
    problem.solver.checkpoint_dir = Some(dir.join("cache"));
    problem.limits = symgcad::Limits {
        wall_time_secs: 150.0,
        memory_mib: 1024,
        workers: 1,
        ..Default::default()
    };
    fs::write(&input, toml::to_string(&problem).unwrap()).unwrap();
    let call = |operation: &str, output: &Path| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
        command
            .arg(operation)
            .arg(&input)
            .arg("--output")
            .arg(output);
        let process = symgcad::benchmark::run_bounded(&mut command, &problem.limits).unwrap();
        assert_eq!(process.status, "completed", "{}", process.stderr);
        serde_json::from_slice::<serde_json::Value>(&fs::read(output).unwrap()).unwrap()
    };
    let first = call("order", &dir.join("first.json"));
    assert_eq!(first["search"]["checkpoint"]["loaded_trials"], 0);
    assert!(
        first["search"]["checkpoint"]["writes"].as_u64().unwrap() > 0,
        "{first}"
    );
    let second = call("order", &dir.join("second.json"));
    assert!(
        second["search"]["checkpoint"]["loaded_trials"]
            .as_u64()
            .unwrap()
            > 0,
        "{second}"
    );
    assert_eq!(
        second["search"]["trials"][0]["outcome"]["status"],
        "unverified"
    );
    assert_eq!(
        second["search"]["revalidations"][0]["outcome"]["status"],
        "completed"
    );
    let final_result = call("solve", &dir.join("result.json"));
    let result: symgcad::output::SolveResult = serde_json::from_value(final_result).unwrap();
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    assert_eq!(result.cells.len(), 1);
    fs::remove_dir_all(dir).unwrap();
}
