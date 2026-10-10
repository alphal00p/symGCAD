use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use symgcad::{
    Limits, Problem,
    benchmark::run_bounded,
    output::{SolveResult, Status},
};

fn directory() -> PathBuf {
    let directory = PathBuf::from("artifacts/tests").join(format!(
        "cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn invalid_toml_and_polynomial_have_explicit_status() {
    let directory = directory();
    let input = directory.join("invalid.toml");
    for contents in ["variables = [", "variables=['x']\nconstraints=['sin(x)>0']"] {
        fs::write(&input, contents).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
        command.arg("solve").arg(&input);
        let process = run_bounded(
            &mut command,
            &Limits {
                wall_time_secs: 10.0,
                workers: 1,
                memory_mib: 1024,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(process.status, "completed", "{}", process.stderr);
        let result: SolveResult = serde_json::from_str(&process.stdout).unwrap();
        assert_eq!(result.status, Status::InvalidInput);
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn bounded_mcts_honors_scale_order_and_exports_search_costs() {
    let directory = directory();
    let input = directory.join("search.toml");
    let output = directory.join("result.json");
    let mut problem = Problem {
        variables: vec!["r".into(), "x".into(), "y".into()],
        constraints: vec!["r>0".into(), "x>0".into(), "y>0".into()],
        split: vec!["x+y-r".into()],
        ..Default::default()
    };
    problem.solver.engine = "optimized".into();
    problem.solver.ordering = "mcts".into();
    problem.solver.max_rollouts = 2;
    problem.solver.search_time_secs = Some(2.0);
    problem.solver.positivity = true;
    problem.solver.homogeneous_anchor = Some("r".into());
    problem.solver.homogeneous_block = problem.variables.clone();
    problem.solver.checkpoint_dir = Some(directory.join("checkpoints"));
    problem.limits = Limits {
        wall_time_secs: 10.0,
        workers: 1,
        memory_mib: 1024,
        ..Default::default()
    };
    fs::write(&input, toml::to_string(&problem).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command
        .arg("solve")
        .arg(&input)
        .arg("--output")
        .arg(&output);
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 15.0,
            ..problem.limits.clone()
        },
    )
    .unwrap();
    assert_eq!(process.status, "completed", "{}", process.stderr);
    let result: SolveResult = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(
        result.status,
        Status::CompleteGeneric,
        "{:?}",
        result.diagnostic
    );
    assert_eq!(result.order[0], "r");
    assert!(result.statistics.search_seconds > 0.0);
    assert!(result.statistics.total_seconds >= result.statistics.search_seconds);
    let report = result.statistics.ordering_search.as_ref().unwrap();
    assert!(!report["search"]["trials"].as_array().unwrap().is_empty());
    for trial in report["search"]["trials"].as_array().unwrap() {
        assert_eq!(trial["request"]["order"][0], 0);
    }
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn interrupted_worker_reports_its_elapsed_wall_time() {
    let directory = directory();
    let input = directory.join("deadline.toml");
    let output = directory.join("result.json");
    let problem = Problem {
        variables: vec!["x".into()],
        constraints: vec!["x>0".into()],
        split: vec!["x^2-2".into()],
        limits: Limits {
            wall_time_secs: 0.000_001,
            memory_mib: 1024,
            workers: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    fs::write(&input, toml::to_string(&problem).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command
        .arg("solve")
        .arg(&input)
        .arg("--output")
        .arg(&output);
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 10.0,
            ..problem.limits.clone()
        },
    )
    .unwrap();
    assert_eq!(process.status, "completed", "{}", process.stderr);
    let result: SolveResult = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(result.statistics.total_seconds > 0.0);
    assert!(result.statistics.total_seconds <= process.wall_time_secs);
    assert_eq!(result.statistics.search_seconds, 0.0);
    assert_eq!(result.statistics.projection_seconds, 0.0);
    assert_eq!(result.statistics.lifting_seconds, 0.0);
    assert!(result.cells.is_empty() && result.trace.is_none() && result.local.is_none());
    assert!(symgcad::verify::verify(&result).is_err());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn explicit_two_hour_campaign_accepts_problem_and_verifies_result() {
    let directory = directory();
    let input = directory.join("input.toml");
    let campaign = directory.join("campaign.toml");
    let mut problem = Problem {
        variables: vec!["x".into()],
        constraints: vec!["x>0".into()],
        split: vec!["x^2-2".into()],
        limits: Limits {
            wall_time_secs: 7200.0,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    problem.validate().unwrap();
    fs::write(&input, toml::to_string(&problem).unwrap()).unwrap();
    for invalid in [7200.1, f64::INFINITY, f64::NAN, 0.0, -1.0] {
        problem.limits.wall_time_secs = invalid;
        assert!(problem.validate().is_err());
    }
    fs::write(
        &campaign,
        "mode='completion'\ncases=['input.toml']\noutput_dir='results'\n\
         campaign_time_secs=7200.0\nsolve_timeout_secs=7200.0\n\
         verify_timeout_secs=7200.0\nmemory_mib=1024\nworkers=1\n",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command.arg("bench").arg(&campaign);
    // The configured limit is tested without leaving a long-running test.
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 15.0,
            memory_mib: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(process.status, "completed", "{}", process.stderr);
    assert_eq!(process.exit_code, Some(0), "{}", process.stderr);
    let ledger: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("results/ledger.json")).unwrap()).unwrap();
    assert_eq!(ledger["complete_verified"], 1);
    assert_eq!(ledger["runs"][0]["status"], "complete_verified");
    assert_eq!(ledger["runs"][0]["trials"][0]["verified"], true);
    fs::remove_dir_all(directory).unwrap();
}
