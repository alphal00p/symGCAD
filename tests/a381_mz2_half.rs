use symgcad::{Problem, output::Status, solver::solve, verify::verify};

#[test]
fn colleague_a381_point_has_complete_verified_coverage_for_both_signs() {
    let problem: Problem = toml::from_str(include_str!("fixtures/a381_mz2_half.toml")).unwrap();
    let original: Problem = toml::from_str(include_str!("fixtures/a381_fixed.toml")).unwrap();
    // Keep the original polynomial and full positive orthant. Only the Z mass
    // changes mathematically; the solver options select the certified peels.
    assert_eq!(problem.variables, original.variables);
    assert_eq!(problem.constraints, original.constraints);
    assert_eq!(problem.split, original.split);
    assert_eq!(problem.variable_groups, original.variable_groups);
    let mut parameters = original.parameters.clone();
    parameters.insert("mz2".into(), "1/2".into());
    assert_eq!(problem.parameters, parameters);

    let result = solve(&problem).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert_eq!(result.order, problem.solver.order);
    assert_eq!(result.problem.parameters, parameters);
    assert_eq!(result.cells.len(), 836);
    assert!(result.cells.iter().any(|cell| cell.signs == [-1]));
    assert!(result.cells.iter().any(|cell| cell.signs == [1]));
    assert_eq!(result.statistics.search_seconds, 0.0);

    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let steps = &reduced.monotone.as_ref().unwrap().steps;
    assert_eq!(
        steps
            .iter()
            .map(|step| step.variable.as_str())
            .collect::<Vec<_>>(),
        ["x3", "x5"]
    );
    assert!(steps.iter().all(|step| step.conditional_linear.is_some()));

    // Replay the exported coverage certificate, including homogeneous transport
    // and both conditional quadratic proofs, rather than only checking samples.
    let exported = serde_json::to_string(&result).unwrap();
    let decoded = serde_json::from_str(&exported).unwrap();
    let checked = verify(&decoded).unwrap();
    assert!(checked.verified);
    assert_eq!(checked.cells, result.cells.len());
}
