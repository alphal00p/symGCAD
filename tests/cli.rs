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
