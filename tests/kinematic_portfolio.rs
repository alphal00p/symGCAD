use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use symgcad::{
    Limits, Problem,
    benchmark::run_bounded,
    kinematic_portfolio::{
        self, Candidate, Evaluation, ORDERING, candidate_problem, candidates, evaluate_in_process,
        kinematic_orders, layout, probe, resolve,
    },
    output::{SolveResult, Status},
    solver::solve,
    verify::verify,
};

/// A one-loop box with kinematic group [s, t] lifted before its positive
/// homogeneous Feynman block (corpus finiteE_4_15). The engine and options
/// mirror the prepared completion campaign inputs.
fn box_problem() -> Problem {
    let mut p = Problem::from_toml(
        r#"
name = "kinematic portfolio box"
variables = ["s", "t", "x0", "x1", "x2", "x3"]
constraints = ["s > 4", "x0 > 0", "x1 > 0", "x2 > 0", "x3 > 0"]
split = ["mt2*(x0+x1+x2+x3)^2-(s)*x0*x2-(mz2)*x0*x3-(2*mz2-s-t)*x1*x3-(mz2)*x2*x3"]
variable_groups = [["s", "t"], ["x0", "x1", "x2", "x3"]]

[parameters]
mt2 = "1"
mz2 = "5/18"

[solver]
engine = "monotone"
ordering = "kinematic_portfolio"
positivity = true
input_positivity = true
homogeneous_block = ["x0", "x1", "x2", "x3"]
homogeneous_anchor = "x0"
# No lifting-time bound: explore every alternative of this small default.
portfolio_fiber_samples = 0

[limits]
wall_time_secs = 120.0
memory_mib = 4096
workers = 1
"#,
    )
    .unwrap();
    p.solver.ordering = ORDERING.into();
    p
}

fn names(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn candidates_respect_groups_anchors_and_default_first() {
    let p = box_problem();
    let layout = layout(&p).unwrap();
    assert_eq!(layout.kinematic_groups, vec![names(&["s", "t"])]);
    assert_eq!(layout.kinematic_variables(), 2);
    let all = candidates(&p, &layout).unwrap();
    assert_eq!(
        all[0],
        Candidate {
            anchor: Some("x0".into()),
            kinematic_order: None
        }
    );
    // Every block variable is a certified positive-scale anchor here; other
    // anchors come in block order and the input anchor last, each with both
    // s/t orders (s occurs in the Feynman polynomial, so it is coupled).
    let expected_anchors = ["x1", "x2", "x3", "x0"];
    assert_eq!(all.len(), 1 + 2 * expected_anchors.len());
    for (i, anchor) in expected_anchors.iter().enumerate() {
        assert_eq!(all[1 + 2 * i].anchor.as_deref(), Some(*anchor));
        assert_eq!(all[1 + 2 * i].kinematic_order, Some(names(&["s", "t"])));
        assert_eq!(all[2 + 2 * i].kinematic_order, Some(names(&["t", "s"])));
    }
    for candidate in &all {
        let probe_problem = candidate_problem(&p, &layout, candidate);
        probe_problem.validate().unwrap();
        assert_eq!(probe_problem.solver.ordering, "mods");
        assert!(probe_problem.solver.order.is_empty());
        assert_eq!(probe_problem.solver.homogeneous_anchor, candidate.anchor);
        // The Feynman block stays one group lifted after all kinematics.
        assert_eq!(
            probe_problem.variable_groups.last().unwrap(),
            &names(&["x0", "x1", "x2", "x3"])
        );
        if let Some(order) = &candidate.kinematic_order {
            let forced: Vec<_> = probe_problem.variable_groups[..2]
                .iter()
                .map(|g| g[0].clone())
                .collect();
            assert_eq!(&forced, order);
        } else {
            assert_eq!(probe_problem.variable_groups, p.variable_groups);
        }
    }
    assert_eq!(
        kinematic_orders(&[names(&["s"]), names(&["t"])]),
        vec![names(&["s", "t"])]
    );
}

#[test]
fn explicit_orders_and_unsupported_layouts_are_never_overridden() {
    let mut fixed = box_problem();
    fixed.solver.order = names(&["t", "s", "x1", "x0", "x2", "x3"]);
    fixed.solver.homogeneous_anchor = Some("x1".into());
    assert!(!kinematic_portfolio::requested(&fixed));
    assert!(layout(&fixed).is_err());
    let mut calls = 0;
    let (resolved, report) = resolve(&fixed, |_| {
        calls += 1;
        Ok(Evaluation::default())
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert!(!report.applicable);
    assert_eq!(resolved.solver.order, fixed.solver.order);
    assert_eq!(resolved.solver.ordering, "given");
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x1"));

    // No kinematic group before the block: MODS default, no probe.
    let mut flat = box_problem();
    flat.variable_groups = vec![names(&["s", "t", "x0", "x1", "x2", "x3"])];
    let (resolved, report) = resolve(&flat, |_| panic!("no probe expected")).unwrap();
    assert!(!report.applicable && report.fallback_to_default);
    assert_eq!(resolved.solver.ordering, "mods");
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x0"));

    // Unresolved portfolio orders are refused by the exact library solver.
    let error = solve(&box_problem()).unwrap_err().to_string();
    assert!(error.contains("kinematic_portfolio"), "{error}");
}

#[test]
fn probe_scores_the_projection_the_solver_actually_lifts() {
    let p = box_problem();
    let layout = layout(&p).unwrap();
    let default = candidate_problem(&p, &layout, &candidates(&p, &layout).unwrap()[0]);
    let mut sampled = default.clone();
    sampled.solver.portfolio_fiber_samples = 1000;
    let score = probe(&sampled).unwrap();
    assert_eq!(score.kinematic_variables, 2);
    assert!(!score.kinematic_cells_capped);
    let mut fixed = p.clone();
    fixed.solver.ordering = "given".into();
    fixed.solver.order = score.order.clone();
    let result = solve(&fixed).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let base = &reduced.monotone.as_ref().unwrap().base;
    assert_eq!(base.order, score.projected_order);
    assert_eq!(base.polynomials.len(), score.projection_polynomials);
    assert_eq!(
        base.projection_levels
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        score.level_polynomials
    );
    // Exactly the distinct (s, t) prefixes of the certified cells.
    let prefixes: BTreeSet<_> = base
        .cells
        .iter()
        .map(|c| (c.axes[0].sample.clone(), c.axes[1].sample.clone()))
        .collect();
    assert_eq!(prefixes.len(), score.kinematic_cells);
    // Sampling every kinematic cell lifts exactly the certified base cells.
    let estimate = score.lifting_estimate.as_ref().unwrap();
    // (unless the probe's sampling time limit stopped it early).
    assert!(estimate.sampled_cells <= base.cells.len());
    if !estimate.truncated && estimate.samples == score.kinematic_cells {
        assert_eq!(estimate.sampled_cells, base.cells.len());
    }
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn forced_kinematic_orders_keep_the_mods_feynman_order() {
    // The probe runs MODS with merged kinematic groups and restores the
    // forced kinematic order; plain MODS on the singleton groups agrees.
    let p = box_problem();
    let layout = layout(&p).unwrap();
    for candidate in candidates(&p, &layout).unwrap() {
        let probe_problem = candidate_problem(&p, &layout, &candidate);
        let score = probe(&probe_problem).unwrap();
        assert_eq!(
            score.order,
            symgcad::solver::selected_order(&probe_problem).unwrap(),
            "{candidate:?}"
        );
        if let Some(kinematic) = &candidate.kinematic_order {
            assert_eq!(&score.order[..2], &kinematic[..]);
            let anchor = candidate.anchor.as_ref().unwrap();
            assert_eq!(&score.order[2], anchor);
        }
    }
}

#[test]
fn in_process_portfolio_selects_a_verified_order_deterministically() {
    let p = box_problem();
    let (resolved, report) = resolve(&p, |probe| Ok(evaluate_in_process(probe))).unwrap();
    assert!(report.applicable && !report.fallback_to_default);
    // The default MODS candidate lifts s before t with anchor x0, so the
    // forced (x0, [s, t]) candidate is recognized without another probe.
    assert_eq!(
        &report.candidates[0].score.as_ref().unwrap().order[..2],
        &names(&["s", "t"])[..]
    );
    let twin = report
        .candidates
        .iter()
        .find(|r| {
            r.candidate.anchor.as_deref() == Some("x0")
                && r.candidate.kinematic_order == Some(names(&["s", "t"]))
        })
        .unwrap();
    assert_eq!(twin.status, "duplicate");
    assert!(twin.score.is_none());
    let selected = report.selected.unwrap();
    let scores: Vec<_> = report
        .candidates
        .iter()
        .map(|r| r.score.as_ref().map(|s| s.kinematic_cells))
        .collect();
    let best = report.candidates[selected].score.as_ref().unwrap();
    let default = report.candidates[0].score.as_ref().unwrap();
    assert!(
        best.kinematic_cells <= default.kinematic_cells,
        "{scores:?}"
    );
    assert_eq!(resolved.solver.ordering, "given");
    assert_eq!(resolved.solver.order, best.order);
    assert_eq!(resolved.solver.order, report.order);
    assert_eq!(
        resolved.solver.homogeneous_anchor,
        report.candidates[selected].candidate.anchor
    );
    // The original groups still hold for the resolved explicit order.
    assert_eq!(resolved.variable_groups, p.variable_groups);
    let result = solve(&resolved).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert_eq!(result.order, best.order);
    assert!(verify(&result).unwrap().verified);
    // Same candidates, same scores, same choice (budgets aside).
    let (_, again) = resolve(&p, |probe| Ok(evaluate_in_process(probe))).unwrap();
    assert_eq!(again.selected, report.selected);
    let rescored: Vec<_> = again
        .candidates
        .iter()
        .map(|r| r.score.as_ref().map(|s| s.kinematic_cells))
        .collect();
    assert_eq!(rescored, scores);
}

#[test]
fn alternatives_are_bounded_by_the_default_lifting_estimate() {
    let mut p = box_problem();
    p.solver.portfolio_fiber_samples = 8;
    p.solver.portfolio_lifting_budget_factor = 1e-9;
    p.solver.portfolio_min_budget_secs = 1e-6;
    let mut probes = 0;
    let (resolved, report) = resolve(&p, |probe| {
        probes += 1;
        // Only the default probe samples fibers.
        assert_eq!(
            probe.solver.portfolio_fiber_samples,
            if probes == 1 { 8 } else { 0 }
        );
        Ok(evaluate_in_process(probe))
    })
    .unwrap();
    assert_eq!(probes, 1);
    assert_eq!(report.selected, Some(0));
    assert!(report.alternatives_budget_seconds.is_some());
    assert!(
        report.candidates[1..]
            .iter()
            .all(|r| ["skipped", "duplicate"].contains(&r.status.as_str()))
    );
    let default = report.candidates[0].score.as_ref().unwrap();
    let estimate = default.lifting_estimate.as_ref().unwrap();
    assert_eq!(estimate.samples, 8);
    assert!(estimate.sampled_cells > 0 && estimate.estimated_cells > 0.0);
    assert_eq!(resolved.solver.order, default.order);
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x0"));
    let result = solve(&resolved).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(verify(&result).unwrap().verified);
}

#[test]
fn decoupled_kinematic_variables_are_lifted_last() {
    // s occurs only in s > 4, so (s, t) and (t, s) give the same product
    // kinematic decomposition; only (t, s) avoids constant resultants.
    let mut p = Problem::from_toml(
        r#"
variables = ["s", "t", "x0", "x1"]
constraints = ["s > 4", "x0 > 0", "x1 > 0"]
split = ["x0^2 + x1^2 - t*x0*x1"]
variable_groups = [["s", "t"], ["x0", "x1"]]
[solver]
engine = "monotone"
positivity = true
homogeneous_block = ["x0", "x1"]
homogeneous_anchor = "x0"
"#,
    )
    .unwrap();
    p.solver.ordering = ORDERING.into();
    let layout = layout(&p).unwrap();
    let decoupled = kinematic_portfolio::decoupled_kinematics(&p, &layout).unwrap();
    assert_eq!(decoupled, BTreeSet::from(["s".to_string()]));
    assert_eq!(
        kinematic_portfolio::portfolio_kinematic_orders(&layout, &decoupled),
        vec![names(&["t", "s"])]
    );
    assert!(
        candidates(&p, &layout)
            .unwrap()
            .iter()
            .skip(1)
            .all(|c| c.kinematic_order == Some(names(&["t", "s"])))
    );
    // A box polynomial couples both kinematic variables.
    let boxed = box_problem();
    let layout = kinematic_portfolio::layout(&boxed).unwrap();
    assert!(
        kinematic_portfolio::decoupled_kinematics(&boxed, &layout)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn timeouts_and_failures_fall_back_to_the_default_mods_order() {
    let p = box_problem();
    let (resolved, report) = resolve(&p, |_| {
        Ok(Evaluation {
            status: "timeout".into(),
            ..Default::default()
        })
    })
    .unwrap();
    assert!(report.fallback_to_default && report.selected.is_none());
    assert!(report.candidates.iter().all(|r| r.status != "scored"));
    assert_eq!(resolved.solver.ordering, "mods");
    assert!(resolved.solver.order.is_empty());
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x0"));
}

fn directory() -> PathBuf {
    let directory = PathBuf::from("artifacts/tests").join(format!(
        "kinematic-portfolio-{}-{}",
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
fn cli_portfolio_records_scores_and_solves_a_verified_order() {
    let directory = directory();
    let input = directory.join("box.toml");
    let output = directory.join("result.json");
    fs::write(&input, toml::to_string(&box_problem()).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command
        .arg("solve")
        .arg(&input)
        .arg("--output")
        .arg(&output);
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 120.0,
            workers: 1,
            memory_mib: 4096,
            ..Default::default()
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
    let report = result.statistics.ordering_portfolio.as_ref().unwrap();
    assert!(report.applicable);
    let selected = report.selected.unwrap();
    assert!(
        report
            .candidates
            .iter()
            .filter(|r| r.status == "scored")
            .count()
            >= 2
    );
    assert_eq!(result.order, report.order);
    assert_eq!(result.problem.solver.ordering, "given");
    assert_eq!(result.problem.solver.order, report.order);
    assert_eq!(
        result.problem.solver.homogeneous_anchor,
        report.candidates[selected].candidate.anchor
    );
    assert!(result.statistics.ordering_seconds >= report.total_seconds);
    assert!(result.statistics.total_seconds >= report.total_seconds);
    assert!(verify(&result).unwrap().verified);
    fs::remove_dir_all(directory).unwrap();
}
