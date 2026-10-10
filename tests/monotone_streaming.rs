//! Coverage regressions for cellwise monotone reconstruction verification.
use symgcad::{Problem, output::Status, solver::solve, verify::verify};

fn branched_problem() -> Problem {
    let mut p = Problem {
        variables: vec!["x".into(), "y".into(), "z".into()],
        constraints: vec!["x>0".into(), "x<2".into(), "y>0".into(), "z>0".into()],
        split: vec!["z^2+z+y^2+y+x-1".into()],
        ..Default::default()
    };
    p.solver.engine = "monotone".into();
    p.limits.wall_time_secs = 15.0;
    p.limits.workers = 1;
    p
}

#[test]
fn several_base_cells_and_sibling_restorations_keep_exact_order() {
    let solution = solve(&branched_problem()).unwrap();
    assert_eq!(solution.status, Status::CompleteGeneric);
    let evidence = solution.monotone.as_ref().unwrap();
    assert_eq!(evidence.steps.len(), 2);
    assert_eq!(evidence.base.cells.len(), 2);
    assert_eq!(solution.cells.len(), 4);
    assert!(verify(&solution).unwrap().verified);

    let mut missing = solution.clone();
    missing.cells.remove(1);
    assert!(verify(&missing).is_err());
    let mut duplicate = solution.clone();
    duplicate.cells.insert(2, duplicate.cells[1].clone());
    assert!(verify(&duplicate).is_err());
    let mut replaced = solution.clone();
    replaced.cells[1] = replaced.cells[0].clone();
    assert!(verify(&replaced).is_err());
    let mut reordered = solution.clone();
    reordered.cells.swap(1, 2);
    assert!(verify(&reordered).is_err());
    let mut trailing = solution.clone();
    trailing.cells.push(trailing.cells.last().unwrap().clone());
    assert!(verify(&trailing).is_err());
}

#[test]
fn zero_endpoint_and_empty_negative_target_replay_without_cells_to_buffer() {
    let mut p = branched_problem();
    p.variables = vec!["y".into(), "z".into()];
    p.constraints = vec!["y>0".into(), "z>0".into()];
    p.split = vec!["y^2+y+z^2+z".into()];
    let historical = solve(&p).unwrap();
    assert_eq!(historical.cells.len(), 1);
    assert_eq!(historical.cells[0].signs, [1]);
    assert!(historical.monotone.as_ref().unwrap().target.is_none());
    assert!(verify(&historical).unwrap().verified);
    let restored = serde_json::from_str(&serde_json::to_string(&historical).unwrap()).unwrap();
    assert!(verify(&restored).unwrap().verified);

    p.constraints
        .push(format!("({})<0", p.split.pop().unwrap()));
    p.solver.monotone_negative_target = true;
    let empty = solve(&p).unwrap();
    assert_eq!(empty.status, Status::CompleteGeneric);
    assert!(empty.cells.is_empty());
    assert!(empty.monotone.as_ref().unwrap().target.is_some());
    assert!(verify(&empty).unwrap().verified);
    let mut extra = empty.clone();
    extra.cells.push(historical.cells[0].clone());
    extra.cells[0].signs.clear();
    extra.statistics.raw_cells = 1;
    assert!(verify(&extra).is_err());
}

#[test]
fn truncated_reconstruction_cannot_be_relabelled_complete() {
    let mut p = branched_problem();
    p.limits.max_cells = Some(2);
    let mut limited = solve(&p).unwrap();
    assert_eq!(limited.status, Status::Incomplete);
    assert_eq!(limited.cells.len(), 2);
    assert!(verify(&limited).is_err());
    limited.status = Status::CompleteGeneric;
    assert!(verify(&limited).is_err());
}

#[test]
fn negative_target_emits_each_base_cell_once_with_empty_split_annotation() {
    let mut p = branched_problem();
    p.constraints = vec![
        "x>0".into(),
        "x<5".into(),
        "y>0".into(),
        "z>0".into(),
        "z^2+z+y^2+y+(x-1)*(x-2)*(x-3)*(x-4)<0".into(),
    ];
    p.split.clear();
    p.solver.monotone_negative_target = true;
    let solution = solve(&p).unwrap();
    let evidence = solution.monotone.as_ref().unwrap();
    assert_eq!(evidence.steps.len(), 2);
    assert_eq!(evidence.base.cells.len(), 2);
    assert_eq!(solution.cells.len(), 2);
    assert!(solution.cells.iter().all(|cell| cell.signs.is_empty()));
    assert!(verify(&solution).unwrap().verified);
    let mut reordered = solution.clone();
    reordered.cells.swap(0, 1);
    assert!(verify(&reordered).is_err());
    let mut missing = solution.clone();
    missing.cells.pop();
    missing.statistics.raw_cells = missing.cells.len();
    assert!(verify(&missing).is_err());
    let mut duplicate = solution.clone();
    duplicate.cells.push(duplicate.cells[0].clone());
    duplicate.statistics.raw_cells = duplicate.cells.len();
    assert!(verify(&duplicate).is_err());
}

#[test]
fn compatibility_merge_and_boundary_inventory_are_still_replayed() {
    let mut p = branched_problem();
    p.solver.merge = "aggressive".into();
    let solution = solve(&p).unwrap();
    assert!(solution.merged_cells.is_some());
    assert!(verify(&solution).unwrap().verified);
    let mut bad = solution.clone();
    bad.merged_cells.as_mut().unwrap().pop();
    assert!(verify(&bad).is_err());
    let mut bad = solution.clone();
    bad.exceptional_polynomials.pop();
    assert!(verify(&bad).is_err());
    let mut bad = solution.clone();
    bad.cells[0].axes[2].upper.as_mut().unwrap().index = 0;
    assert!(verify(&bad).is_err());
}
