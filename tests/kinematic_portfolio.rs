use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use symgcad::{
    Limits, Problem,
    benchmark::{ProcessOutcome, run_bounded},
    kinematic_portfolio::{
        self, Candidate, CandidatePlan, Enumeration, Evaluation, ORDERING, PortfolioReport,
        ProbeScore, candidate_problem, candidates, check_plan, enumerate_in_process,
        evaluate_in_process, kinematic_orders, layout, plan_candidates, probe, resolve,
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

/// A second, five-propagator layout (corpus finiteC_5_412, prepared
/// completion input 0117) with its own anchor certificates.
fn c412_problem() -> Problem {
    Problem::from_toml(
        r#"
name = "finiteC_5_412 portfolio"
variables = ["s", "t", "x2", "x3", "x4", "x7", "x8"]
constraints = ["s > 4", "x2 > 0", "x3 > 0", "x4 > 0", "x7 > 0", "x8 > 0"]
split = ["(mt2)*x2^2*x4 + ( mt2)*x2^2*x7 + ( mt2)*x2^2*x8 + ( 2*mt2)*x2*x3*x4 + ( 2*mt2)*x2*x3*x7 + ( 2*mt2)*x2*x3*x8 + ( mt2)*x2*x4^2 + ( 2*mt2 - t)*x2*x4*x7 + ( 2*mt2 - mz2)*x2*x4*x8 + ( mt2)*x3^2*x4 + ( mt2)*x3^2*x7 + ( mt2)*x3^2*x8 + ( mt2)*x3*x4^2 + ( 2*mt2 - mz2)*x3*x4*x7 + ( 2*mt2 - 2*mz2 + s + t)*x3*x4*x8 + ( mt2)*x4^2*x7 + ( mt2)*x4^2*x8"]
variable_groups = [["s", "t"], ["x2", "x3", "x4", "x7", "x8"]]

[parameters]
mt2 = "1"
mz2 = "5/18"

[solver]
engine = "monotone"
ordering = "kinematic_portfolio"
positivity = true
input_positivity = true
homogeneous_block = ["x2", "x3", "x4", "x7", "x8"]
homogeneous_anchor = "x4"
portfolio_fiber_samples = 0

[limits]
wall_time_secs = 120.0
memory_mib = 4096
workers = 1
"#,
    )
    .unwrap()
}

fn names(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn in_process(p: &Problem) -> (Problem, PortfolioReport) {
    resolve(
        p,
        |q| Ok(enumerate_in_process(q)),
        |q| Ok(evaluate_in_process(q)),
    )
    .unwrap()
}

/// The resolved problem may differ from the input only in the ordering
/// fields; constraints, splits, groups, block, parameters and limits stay.
fn assert_only_order_fields_changed(input: &Problem, resolved: &Problem) {
    let strip = |p: &Problem| {
        let mut value = serde_json::to_value(p).unwrap();
        let solver = value["solver"].as_object_mut().unwrap();
        for key in ["ordering", "order", "homogeneous_anchor"] {
            solver.remove(key);
        }
        value
    };
    assert_eq!(strip(input), strip(resolved));
}

fn completed_plan(plan: CandidatePlan) -> Enumeration {
    Enumeration {
        status: "completed".into(),
        plan: Some(plan),
        ..Default::default()
    }
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
        assert!(probe_problem.solver.checkpoint_dir.is_none());
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

    // The worker's plan is the same enumeration; the supervisor admits it
    // structurally and rejects malformed plans without any algebra.
    let plan = plan_candidates(&p).unwrap();
    assert_eq!(plan.candidates, all);
    assert!(plan.decoupled_kinematic_variables.is_empty());
    check_plan(&p, &layout, &plan).unwrap();
    let malformed: [fn(&mut CandidatePlan); 5] = [
        |plan| {
            plan.candidates.remove(0);
        },
        |plan| plan.candidates[1].anchor = Some("s".into()),
        |plan| plan.candidates[1].anchor = None,
        |plan| plan.candidates[1].kinematic_order = Some(names(&["s", "x0"])),
        |plan| plan.decoupled_kinematic_variables = names(&["x1"]),
    ];
    for mutate in malformed {
        let mut bad = plan.clone();
        mutate(&mut bad);
        assert!(check_plan(&p, &layout, &bad).is_err(), "{bad:?}");
    }
}

#[test]
fn explicit_orders_and_unsupported_layouts_are_never_overridden() {
    let mut fixed = box_problem();
    fixed.solver.order = names(&["t", "s", "x1", "x0", "x2", "x3"]);
    fixed.solver.homogeneous_anchor = Some("x1".into());
    assert!(!kinematic_portfolio::requested(&fixed));
    assert!(layout(&fixed).is_err());
    let mut calls = 0;
    let (resolved, report) = resolve(
        &fixed,
        |_| panic!("no enumeration expected"),
        |_| {
            calls += 1;
            Ok(Evaluation::default())
        },
    )
    .unwrap();
    assert_eq!(calls, 0);
    assert!(!report.applicable);
    assert_eq!(resolved.solver.order, fixed.solver.order);
    assert_eq!(resolved.solver.ordering, "given");
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x1"));
    assert_only_order_fields_changed(&fixed, &resolved);

    // No kinematic group before the block: MODS default, no worker at all.
    let mut flat = box_problem();
    flat.variable_groups = vec![names(&["s", "t", "x0", "x1", "x2", "x3"])];
    let (resolved, report) = resolve(
        &flat,
        |_| panic!("no enumeration expected"),
        |_| panic!("no probe expected"),
    )
    .unwrap();
    assert!(!report.applicable && report.fallback_to_default);
    assert_eq!(resolved.solver.ordering, "mods");
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x0"));
    assert_only_order_fields_changed(&flat, &resolved);

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
    // Checked on two corpus layouts with different blocks and anchors.
    for p in [box_problem(), c412_problem()] {
        let layout = layout(&p).unwrap();
        let all = candidates(&p, &layout).unwrap();
        assert!(all.len() > 2, "{}", p.name.as_deref().unwrap());
        for candidate in all {
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
}

#[test]
fn in_process_portfolio_selects_a_verified_order_deterministically() {
    let p = box_problem();
    let (resolved, report) = in_process(&p);
    assert!(report.applicable && !report.fallback_to_default);
    assert_eq!(report.enumeration.as_ref().unwrap().status, "completed");
    assert_only_order_fields_changed(&p, &resolved);
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
    // A result whose recorded problem still requests the portfolio is not a
    // replayable record and is refused by the verifier.
    let mut unresolved = result.clone();
    unresolved.problem.solver.ordering = ORDERING.into();
    unresolved.problem.solver.order.clear();
    let error = verify(&unresolved).unwrap_err().to_string();
    assert!(error.contains("kinematic_portfolio"), "{error}");
    // Same candidates, same scores, same choice (budgets aside).
    let (_, again) = in_process(&p);
    assert_eq!(again.selected, report.selected);
    let rescored: Vec<_> = again
        .candidates
        .iter()
        .map(|r| r.score.as_ref().map(|s| s.kinematic_cells))
        .collect();
    assert_eq!(rescored, scores);
}

/// Box variants: the optimized engine, no anchor (no scale reduction),
/// singleton kinematic groups, and three kinematic variables in two groups.
#[test]
fn engines_anchors_and_group_layouts_resolve_to_verified_orders() {
    let optimized = {
        let mut p = box_problem();
        p.solver.engine = "optimized".into();
        p
    };
    let unanchored = {
        let mut p = box_problem();
        p.solver.homogeneous_anchor = None;
        p
    };
    let singletons = {
        let mut p = box_problem();
        p.variable_groups = vec![
            names(&["s"]),
            names(&["t"]),
            names(&["x0", "x1", "x2", "x3"]),
        ];
        p
    };
    let three = {
        let mut p = box_problem();
        p.variables = names(&["s", "t", "m", "x0", "x1", "x2", "x3"]);
        p.constraints.push("m > 0".into());
        p.split = vec!["mt2*(x0+x1+x2+x3)^2-(s)*x0*x2-(m)*x0*x3-(2*m-s-t)*x1*x3-(m)*x2*x3".into()];
        p.parameters.remove("mz2");
        p.variable_groups = vec![
            names(&["s", "t"]),
            names(&["m"]),
            names(&["x0", "x1", "x2", "x3"]),
        ];
        p.validate().unwrap();
        p
    };
    for p in [optimized, unanchored, singletons, three] {
        let layout = layout(&p).unwrap();
        let (resolved, report) = in_process(&p);
        let label = format!("{:?} {:?}", p.solver.engine, p.variable_groups);
        assert!(report.applicable && !report.fallback_to_default, "{label}");
        assert_only_order_fields_changed(&p, &resolved);
        let plan_orders: BTreeSet<_> = report
            .candidates
            .iter()
            .filter_map(|r| r.candidate.kinematic_order.clone())
            .collect();
        assert_eq!(
            plan_orders,
            kinematic_orders(&layout.kinematic_groups)
                .into_iter()
                .collect(),
            "{label}"
        );
        if p.solver.homogeneous_anchor.is_none() {
            assert!(
                report
                    .candidates
                    .iter()
                    .all(|r| r.candidate.anchor.is_none())
            );
        }
        // The resolved order respects every input group.
        let depth = layout.kinematic_variables();
        let mut start = 0;
        for group in &p.variable_groups {
            let lifted: BTreeSet<_> = resolved.solver.order[start..start + group.len()]
                .iter()
                .collect();
            assert_eq!(lifted, group.iter().collect(), "{label}");
            start += group.len();
        }
        assert!(depth >= 2);
        let result = solve(&resolved).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric, "{label}");
        assert!(verify(&result).unwrap().verified, "{label}");
    }
}

#[test]
fn alternatives_are_bounded_by_the_default_lifting_estimate() {
    let mut p = box_problem();
    p.solver.portfolio_fiber_samples = 8;
    p.solver.portfolio_lifting_budget_factor = 1e-9;
    p.solver.portfolio_min_budget_secs = 1e-6;
    let mut probes = 0;
    let (resolved, report) = resolve(
        &p,
        |q| Ok(enumerate_in_process(q)),
        |probe| {
            probes += 1;
            // Only the default probe samples fibers.
            assert_eq!(
                probe.solver.portfolio_fiber_samples,
                if probes == 1 { 8 } else { 0 }
            );
            Ok(evaluate_in_process(probe))
        },
    )
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
fn unscored_defaults_bound_the_alternatives_by_their_probe_time() {
    // Wall 1000 s: default budget 60 s, portfolio total 250 s.
    let mut p = box_problem();
    p.limits.wall_time_secs = 1000.0;
    let plan = plan_candidates(&p).unwrap();
    let alternatives = plan.candidates.len() - 1;
    for (status, default_wall, pool) in [
        ("timeout", 60.0, Some(60.0)),
        ("failed", 0.01, Some(0.25)),
        // The default solve would exceed the same memory limit: only the
        // portfolio total bounds the alternatives.
        ("memory_limit", 12.0, None),
    ] {
        let mut budgets = Vec::new();
        let (resolved, report) = resolve(
            &p,
            |_| Ok(completed_plan(plan.clone())),
            |probe| {
                budgets.push(probe.limits.wall_time_secs);
                Ok(Evaluation {
                    status: if budgets.len() == 1 {
                        status
                    } else {
                        "timeout"
                    }
                    .into(),
                    wall_seconds: if budgets.len() == 1 {
                        default_wall
                    } else {
                        probe.limits.wall_time_secs
                    },
                    ..Default::default()
                })
            },
        )
        .unwrap();
        assert!(report.fallback_to_default && report.selected.is_none());
        assert_eq!(resolved.solver.ordering, "mods");
        assert_eq!(budgets.len(), 1 + alternatives, "{status}");
        assert_eq!(budgets[0], 60.0);
        // Unprobed candidates share the rest evenly (fake probes take no
        // time here, so the shares never shrink).
        match pool {
            Some(pool) => {
                assert!(report.alternatives_budget_seconds.is_some(), "{status}");
                let share = (pool / alternatives as f64).max(0.25).min(pool);
                assert!((budgets[1] - share).abs() < 0.05, "{status} {budgets:?}");
                assert!(budgets[1..].iter().all(|b| *b <= pool), "{budgets:?}");
            }
            None => {
                assert!(report.alternatives_budget_seconds.is_none());
                let share = (250.0 / alternatives as f64).min(60.0);
                assert!((budgets[1] - share).abs() < 0.05, "{budgets:?}");
            }
        }
    }
}

#[test]
fn fixed_budgets_make_the_choice_independent_of_probe_times() {
    let mut p = box_problem();
    p.limits.wall_time_secs = 1000.0;
    p.solver.portfolio_fixed_budgets = true;
    p.solver.portfolio_fiber_samples = 8;
    let plan = plan_candidates(&p).unwrap();
    let count = plan.candidates.len();
    // min(default budget 60, total 250 / (candidates + 1)).
    let fixed = (250.0 / (count + 1) as f64).min(60.0);
    // Deterministic fake scores; the probe times differ between runs.
    let score = |probe: &Problem| {
        let anchor = probe.solver.homogeneous_anchor.clone().unwrap();
        let order = if probe.solver.order.is_empty() {
            let kinematic: Vec<String> = if probe.variable_groups.len() > 2 {
                probe.variable_groups[..2]
                    .iter()
                    .map(|g| g[0].clone())
                    .collect()
            } else {
                names(&["s", "t"])
            };
            let mut order = kinematic;
            order.push(anchor.clone());
            order.extend(
                names(&["x0", "x1", "x2", "x3"])
                    .into_iter()
                    .filter(|v| *v != anchor),
            );
            order
        } else {
            probe.solver.order.clone()
        };
        let cells = match (anchor.as_str(), order[0].as_str()) {
            ("x2", "t") => 5,
            ("x3", "s") => 7,
            _ => 30,
        };
        ProbeScore {
            order,
            kinematic_variables: 2,
            kinematic_cells: cells,
            projection_terms: 100,
            ..Default::default()
        }
    };
    let run = |slow: bool| {
        let mut budgets = Vec::new();
        let (resolved, report) = resolve(
            &p,
            |_| Ok(completed_plan(plan.clone())),
            |probe| {
                assert_eq!(probe.solver.portfolio_fiber_samples, 0);
                budgets.push(probe.limits.wall_time_secs);
                Ok(Evaluation {
                    status: "completed".into(),
                    wall_seconds: if slow {
                        0.9 * probe.limits.wall_time_secs
                    } else {
                        1e-3
                    },
                    score: Some(score(probe)),
                    ..Default::default()
                })
            },
        )
        .unwrap();
        (resolved, report, budgets)
    };
    let (fast_problem, fast_report, fast_budgets) = run(false);
    let (slow_problem, slow_report, slow_budgets) = run(true);
    assert!(fast_report.fixed_budgets);
    assert_eq!(fast_budgets, slow_budgets);
    assert!(fast_budgets.iter().all(|b| (*b - fixed).abs() < 1e-12));
    assert!(fast_report.alternatives_budget_seconds.is_none());
    assert!(fast_report.candidates.iter().all(|r| r.status != "skipped"));
    assert_eq!(fast_report.selected, slow_report.selected);
    assert_eq!(fast_problem.solver.order, slow_problem.solver.order);
    assert_eq!(
        fast_problem.solver.order,
        names(&["t", "s", "x2", "x0", "x1", "x3"])
    );
    assert_eq!(
        fast_problem.solver.homogeneous_anchor.as_deref(),
        Some("x2")
    );
}

#[test]
fn enumeration_failures_fall_back_without_probes() {
    let p = box_problem();
    let plan = plan_candidates(&p).unwrap();
    let mut malformed = plan.clone();
    malformed.candidates.swap(0, 1);
    let outcomes: Vec<(Box<dyn Fn() -> anyhow::Result<Enumeration>>, &str)> = vec![
        (
            Box::new(|| -> anyhow::Result<Enumeration> {
                Ok(Enumeration {
                    status: "timeout".into(),
                    wall_seconds: 6.0,
                    ..Default::default()
                })
            }),
            "timeout",
        ),
        (
            Box::new(|| -> anyhow::Result<Enumeration> {
                Ok(Enumeration {
                    status: "memory_limit".into(),
                    ..Default::default()
                })
            }),
            "memory_limit",
        ),
        (
            Box::new(|| -> anyhow::Result<Enumeration> {
                Ok(Enumeration {
                    status: "completed".into(),
                    ..Default::default()
                })
            }),
            "failed",
        ),
        (
            Box::new(|| -> anyhow::Result<Enumeration> { anyhow::bail!("no temporary file") }),
            "failed",
        ),
        (
            Box::new(move || -> anyhow::Result<Enumeration> {
                Ok(completed_plan(malformed.clone()))
            }),
            "rejected",
        ),
    ];
    for (outcome, status) in outcomes {
        let (resolved, report) =
            resolve(&p, |_| outcome(), |_| panic!("no probe expected")).unwrap();
        let enumeration = report.enumeration.as_ref().unwrap();
        assert_eq!(enumeration.status, status);
        assert!(report.applicable && report.fallback_to_default);
        assert!(report.candidates.is_empty());
        assert!(report.reason.as_ref().unwrap().contains(status));
        assert_eq!(resolved.solver.ordering, "mods");
        assert!(resolved.solver.order.is_empty());
        assert_only_order_fields_changed(&p, &resolved);
    }
}

#[test]
fn resolve_runs_no_algebra_in_the_supervising_process() {
    // `y` is undeclared: the input passes the structural validation, but
    // any normalization fails. A resolve that works on it with worker
    // results did no algebra of its own.
    let mut p = box_problem();
    p.constraints.push("y*x0 > 0".into());
    p.validate().unwrap();
    assert!(symgcad::problem::normalize(&p, &p.variables).is_err());
    let layout = layout(&p).unwrap();
    let plan = CandidatePlan {
        decoupled_kinematic_variables: vec![],
        candidates: vec![
            Candidate {
                anchor: Some("x0".into()),
                kinematic_order: None,
            },
            Candidate {
                anchor: Some("x1".into()),
                kinematic_order: Some(names(&["t", "s"])),
            },
        ],
    };
    check_plan(&p, &layout, &plan).unwrap();
    let orders = [
        names(&["s", "t", "x0", "x1", "x2", "x3"]),
        names(&["t", "s", "x1", "x0", "x2", "x3"]),
    ];
    let mut calls = 0;
    let (resolved, report) = resolve(
        &p,
        |_| Ok(completed_plan(plan.clone())),
        |_| {
            calls += 1;
            Ok(Evaluation {
                status: "completed".into(),
                score: Some(ProbeScore {
                    order: orders[calls - 1].clone(),
                    kinematic_cells: [20, 3][calls - 1],
                    ..Default::default()
                }),
                ..Default::default()
            })
        },
    )
    .unwrap();
    assert_eq!(report.selected, Some(1));
    assert_eq!(resolved.solver.order, orders[1]);
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x1"));
    // The in-process enumeration reports the invalid input as a failure.
    let enumeration = enumerate_in_process(&p);
    assert_eq!(enumeration.status, "failed");
    assert!(enumeration.message.unwrap().contains('y'));
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
    assert_eq!(
        plan_candidates(&p).unwrap().decoupled_kinematic_variables,
        names(&["s"])
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
    let (resolved, report) = resolve(
        &p,
        |q| Ok(enumerate_in_process(q)),
        |_| {
            Ok(Evaluation {
                status: "timeout".into(),
                ..Default::default()
            })
        },
    )
    .unwrap();
    assert!(report.fallback_to_default && report.selected.is_none());
    assert!(report.candidates.iter().all(|r| r.status != "scored"));
    assert_eq!(resolved.solver.ordering, "mods");
    assert!(resolved.solver.order.is_empty());
    assert_eq!(resolved.solver.homogeneous_anchor.as_deref(), Some("x0"));
    assert_only_order_fields_changed(&p, &resolved);
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

fn cli(operation: &str, input: &Path, output: &Path) -> ProcessOutcome {
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command
        .arg(operation)
        .arg(input)
        .arg("--output")
        .arg(output);
    run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 120.0,
            workers: 1,
            memory_mib: 4096,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn cli_portfolio_records_scores_and_solves_a_verified_order() {
    let directory = directory();
    let input = directory.join("box.toml");
    let output = directory.join("result.json");
    fs::write(&input, toml::to_string(&box_problem()).unwrap()).unwrap();
    let process = cli("solve", &input, &output);
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
    assert_eq!(report.enumeration.as_ref().unwrap().status, "completed");
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

    // `project` and `order` record the same choice and the report.
    for operation in ["project", "order"] {
        let output = directory.join(format!("{operation}.json"));
        let process = cli(operation, &input, &output);
        assert_eq!(process.status, "completed", "{}", process.stderr);
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        let recorded: PortfolioReport =
            serde_json::from_value(value["ordering_portfolio"].clone()).unwrap();
        assert_eq!(
            value["order"],
            serde_json::json!(recorded.order),
            "{operation}"
        );
        assert_eq!(
            value["homogeneous_anchor"],
            serde_json::json!(recorded.anchor),
            "{operation}"
        );
        assert_eq!(recorded.order, report.order, "{operation}");
        assert_eq!(recorded.anchor, report.anchor, "{operation}");
    }
    fs::remove_dir_all(directory).unwrap();
}

/// The reviewer's repro: candidate enumeration of this input is expensive
/// (a degree-200 constraint). It must run inside the input's wall and memory
/// limits, never in the supervising process.
#[test]
fn cli_portfolio_keeps_the_hard_limits_of_the_input() {
    let directory = directory();
    let input = directory.join("overrun.toml");
    let mut p = box_problem();
    p.constraints
        .push("(s+t)*(x0+x1+x2+x3)^200 + x0^200 > 0".into());
    p.solver.portfolio_fiber_samples = 8;
    p.limits.wall_time_secs = 3.0;
    p.limits.memory_mib = 1024;
    fs::write(&input, toml::to_string(&p).unwrap()).unwrap();
    for operation in ["solve", "project", "order"] {
        let output = directory.join(format!("{operation}.json"));
        let process = cli(operation, &input, &output);
        assert_eq!(process.status, "completed", "{}", process.stderr);
        // The parent spent 19 s and 1.97 GB here before the fix.
        assert!(
            process.wall_time_secs < 3.0 + 5.0,
            "{operation}: {} s",
            process.wall_time_secs
        );
        assert!(
            process.peak_rss_kib < 1536 * 1024,
            "{operation}: {} KiB",
            process.peak_rss_kib
        );
        let result: SolveResult = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert!(
            [Status::Incomplete, Status::ComputationFailure].contains(&result.status),
            "{operation}: {:?}",
            result.status
        );
        let report = result.statistics.ordering_portfolio.as_ref().unwrap();
        assert!(report.fallback_to_default);
        let enumeration = report.enumeration.as_ref().unwrap();
        assert!(
            ["timeout", "memory_limit"].contains(&enumeration.status.as_str()),
            "{operation}: {enumeration:?}"
        );
        assert!(enumeration.budget_seconds <= 0.75 + 1e-9);
        assert!(report.total_seconds < report.total_budget_seconds + 2.0);
        // The portfolio time was charged to the following worker's limit.
        assert!(result.problem.limits.wall_time_secs <= 3.0 - report.total_seconds + 1e-6);
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cli_portfolio_writes_results_for_invalid_inputs() {
    let directory = directory();
    let input = directory.join("invalid.toml");
    let mut p = box_problem();
    p.constraints.push("y*x0 > 0".into());
    fs::write(&input, toml::to_string(&p).unwrap()).unwrap();
    for operation in ["solve", "project", "order"] {
        let output = directory.join(format!("{operation}.json"));
        let process = cli(operation, &input, &output);
        assert_eq!(process.status, "completed", "{}", process.stderr);
        assert_eq!(process.exit_code, Some(0), "{}", process.stderr);
        let result: SolveResult = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(result.status, Status::InvalidInput, "{operation}");
        let report = result.statistics.ordering_portfolio.as_ref().unwrap();
        assert_eq!(report.enumeration.as_ref().unwrap().status, "failed");
        assert!(report.fallback_to_default);
    }
    fs::remove_dir_all(directory).unwrap();
}
