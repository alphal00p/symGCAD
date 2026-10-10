use symgcad::{Problem, output::Status, solver, verify};

fn homogeneous_target() -> Problem {
    Problem::from_toml(
        r#"
variables = ["r", "z"]
constraints = ["r>0", "z>0", "r^2-r*z-z^2>0"]
[solver]
engine = "monotone"
monotone_negative_target = true
homogeneous_block = ["r", "z"]
homogeneous_anchor = "r"
order = ["r", "z"]
[limits]
wall_time_secs = 15
workers = 1
"#,
    )
    .unwrap()
}

#[test]
fn negative_target_option_is_explicit_and_round_trips() {
    let old: Problem = serde_json::from_str(r#"{"variables":["z"]}"#).unwrap();
    assert!(!old.solver.monotone_negative_target);
    let encoded = serde_json::to_value(&old).unwrap();
    assert!(encoded["solver"].get("monotone_negative_target").is_none());
    let requested = homogeneous_target();
    let decoded = Problem::from_toml(&toml::to_string(&requested).unwrap()).unwrap();
    assert!(decoded.solver.monotone_negative_target);
}

#[test]
fn target_dispatch_reduces_the_original_constrained_homogeneous_problem_first() {
    let problem = homogeneous_target();
    assert_eq!(solver::selected_order(&problem).unwrap(), ["r", "z"]);
    let solution = solver::solve(&problem).unwrap();
    assert_eq!(solution.status, Status::CompleteGeneric);
    assert_eq!(solution.problem.constraints, problem.constraints);
    assert!(solution.problem.split.is_empty());
    assert!(solution.sign_filter.is_none());
    let homogeneous = solution.homogeneous.as_ref().unwrap();
    assert!(homogeneous.reduced.problem.solver.monotone_negative_target);
    assert!(
        homogeneous
            .reduced
            .problem
            .solver
            .homogeneous_anchor
            .is_none()
    );
    assert!(homogeneous.reduced.problem.split.is_empty());
    assert!(homogeneous.reduced.monotone.is_some());
    assert!(homogeneous.reduced.sign_filter.is_none());
    assert!(!solution.cells.is_empty());
    assert!(verify::verify(&solution).unwrap().verified);
}

#[test]
fn target_dispatch_preserves_the_original_homogeneous_order_requirement() {
    let mut problem = homogeneous_target();
    problem.solver.order = vec!["z".into(), "r".into()];
    assert!(solver::selected_order(&problem).is_err());
    assert!(solver::solve(&problem).is_err());
}
