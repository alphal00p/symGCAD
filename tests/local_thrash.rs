//! Regressions for local-projection refinement thrash: retained sibling proofs
//! across refinement rounds, and the cost/budget admission policy for
//! sample-dependent pair-witness templates. Every completed result is checked
//! by the independent local replay and compared with an ordinary local proof
//! by exact common refinement; none of these fixtures is dispatched by name.
use symgcad::{
    Problem,
    local_projection::{
        LocalEvidence, LocalNode, LocalOptions, LocalReport, LocalSolveResult, solve_local,
        verify_local,
    },
    output::{SolveResult, Status},
};

fn problem(variables: &[&str], constraints: &[&str], split: &[&str]) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|s| (*s).into()).collect(),
        constraints: constraints.iter().map(|s| (*s).into()).collect(),
        split: split.iter().map(|s| (*s).into()).collect(),
        ..Default::default()
    };
    p.limits.wall_time_secs = 30.0;
    p
}

fn options(stable: bool, retain: bool) -> LocalOptions {
    LocalOptions {
        stable_sector_samples: stable,
        retain_sibling_proofs: retain,
        fallback_to_global: false,
        ..Default::default()
    }
}

fn report(local: &LocalSolveResult) -> &LocalReport {
    let LocalEvidence::Local { report, .. } = &local.evidence else {
        panic!("missing local proof: {:?}", local.solution.diagnostic)
    };
    report
}

fn certified(p: &Problem, options: &LocalOptions) -> LocalSolveResult {
    let local = solve_local(p, options).unwrap();
    assert_eq!(
        local.solution.status,
        Status::CompleteGeneric,
        "{:?}",
        local.solution.diagnostic
    );
    assert!(verify_local(&local).unwrap().verified);
    // The replay must not depend on any in-memory discovery state.
    let restored: LocalSolveResult =
        serde_json::from_slice(&serde_json::to_vec(&local).unwrap()).unwrap();
    assert!(verify_local(&restored).unwrap().verified);
    local
}

fn solution(local: &LocalSolveResult) -> SolveResult {
    let mut solution = local.solution.clone();
    solution.local = Some(Box::new(local.evidence.clone()));
    solution
}

fn equivalent(a: &LocalSolveResult, b: &LocalSolveResult) {
    assert_eq!(
        symgcad::comparison::compare_generic(
            &solution(a),
            &solution(b),
            &symgcad::Limits::default()
        )
        .unwrap()
        .equivalent,
        Some(true)
    );
}

fn fixtures() -> Vec<Problem> {
    vec![
        problem(
            &["x", "y", "z", "w"],
            &[],
            &["x", "y", "z-x", "w^2-z", "w-y", "w-2"],
        ),
        problem(&["x", "y", "z"], &[], &["y^2-x", "z^2-y", "z-x", "z-2"]),
        problem(
            &["x", "y", "z"],
            &["x>0", "y>0", "z>0"],
            &["z^2-x*y", "z-x", "z-y", "z-1"],
        ),
        problem(
            &["a", "x", "y", "z"],
            &["a>0", "x>0", "y>0", "z>0"],
            &["z^2-x*y+a", "z-x-y", "y^2-a*x", "x-1"],
        ),
    ]
}

#[test]
fn retained_siblings_preserve_coverage_and_skip_revisiting_completed_children() {
    let mut reused = 0;
    let mut saved = 0usize;
    for p in fixtures() {
        let plain = certified(&p, &options(false, false));
        let stable = certified(&p, &options(true, false));
        let retained = certified(&p, &options(true, true));
        equivalent(&retained, &plain);
        equivalent(&retained, &stable);
        let (r, s) = (report(&retained), report(&stable));
        eprintln!(
            "retained {} nodes/{} refinements/{} reuses; stable {} nodes/{} refinements",
            r.speculative_nodes,
            r.refinements,
            r.retained_sibling_proofs,
            s.speculative_nodes,
            s.refinements
        );
        assert_eq!(s.retained_sibling_proofs, 0);
        reused += r.retained_sibling_proofs;
        saved += s.speculative_nodes.saturating_sub(r.speculative_nodes);
    }
    assert!(reused > 0, "fixtures did not exercise sibling retention");
    assert!(saved > 0, "retention did not avoid any revisited child");
}

#[test]
fn deferred_discovery_retains_siblings_with_identical_coverage() {
    let mut reused = 0;
    for p in fixtures() {
        let plain = certified(&p, &options(false, false));
        let deferred = LocalOptions {
            defer_projection: true,
            ..options(true, false)
        };
        let retained = certified(
            &p,
            &LocalOptions {
                retain_sibling_proofs: true,
                ..deferred.clone()
            },
        );
        let without = certified(&p, &deferred);
        equivalent(&retained, &plain);
        equivalent(&without, &plain);
        assert!(report(&retained).deferred_stacks > 0);
        reused += report(&retained).retained_sibling_proofs;
    }
    assert!(reused > 0, "fixtures did not exercise deferred retention");
}

#[test]
fn retention_requires_the_strict_interior_sample_policy() {
    let p = problem(&["x", "y"], &[], &["y^2-x", "y-1"]);
    assert!(solve_local(&p, &options(false, true)).is_err());
    let mut configured = p.clone();
    configured.solver.engine = "local".into();
    configured.solver.local_retain_siblings = true;
    assert!(configured.validate().is_err());
    configured.solver.local_stable_samples = true;
    configured.validate().unwrap();
    // Default configurations do not serialize the new option.
    assert!(
        serde_json::to_value(&p).unwrap()["solver"]
            .get("local_retain_siblings")
            .is_none()
    );
}

#[test]
fn reused_children_keep_exact_samples_and_forgeries_are_rejected() {
    let p = fixtures().remove(0);
    let local = certified(&p, &options(true, true));
    assert!(report(&local).retained_sibling_proofs > 0);
    // Moving any exported sample onto a boundary or outside its sector, or
    // dropping a sector, must fail the independent replay.
    fn first_stack(
        node: &mut LocalNode,
    ) -> Option<&mut Vec<symgcad::local_projection::LocalSector>> {
        let LocalNode::Stack { sectors, .. } = node else {
            return None;
        };
        if sectors.len() > 2 {
            return Some(sectors);
        }
        sectors.iter_mut().find_map(|s| first_stack(&mut s.child))
    }
    for change in 0..2 {
        let mut forged = local.clone();
        let LocalEvidence::Local { root, .. } = &mut forged.evidence else {
            panic!()
        };
        let sectors = first_stack(root).expect("nontrivial stack");
        if change == 0 {
            let other = sectors[2].sample.clone();
            sectors[1].sample = other;
        } else {
            sectors.pop();
        }
        assert!(verify_local(&forged).is_err());
    }
}

#[test]
fn retention_memory_budget_only_declines_reuse_never_coverage() {
    let mut declined = 0;
    for mut p in fixtures() {
        let unlimited = certified(&p, &options(true, true));
        // One MiB of process memory allows 64 retained proof nodes in total.
        p.limits.memory_mib = 1;
        let tight = certified(&p, &options(true, true));
        equivalent(&tight, &unlimited);
        let r = report(&tight);
        assert_eq!(r.retained_sibling_budget, 64);
        assert!(r.retained_sibling_peak_weight <= 64);
        declined += r.retained_sibling_budget_drops;
        // Once the pool refuses children, rounds stop at the ordinary
        // discovery thresholds again: a binding budget must not make the
        // search larger than stable samples alone at the same memory limit
        // (which also sizes the subtree and learned-cut caches).
        let stable = certified(&p, &options(true, false));
        eprintln!(
            "tight retention {} nodes ({} drops); stable {} nodes",
            r.speculative_nodes,
            r.retained_sibling_budget_drops,
            report(&stable).speculative_nodes
        );
        assert!(r.speculative_nodes <= report(&stable).speculative_nodes);
    }
    assert!(declined > 0, "fixtures did not exercise the retention budget");
}

#[test]
fn retention_never_hides_resource_stops() {
    let p = fixtures().remove(1);
    for o in [
        LocalOptions {
            max_nodes: 3,
            ..options(true, true)
        },
        LocalOptions {
            max_refinements: 0,
            ..options(true, true)
        },
    ] {
        let stopped = solve_local(&p, &o).unwrap();
        assert_eq!(stopped.solution.status, Status::Incomplete);
        assert!(stopped.solution.cells.is_empty());
        assert!(verify_local(&stopped).is_err());
    }
}

fn witness_problem() -> Problem {
    let mut p = problem(&["a", "z"], &["a>1", "a<2", "z>0"], &["z^2-2", "z^2+a*z-4"]);
    p.limits.wall_time_secs = 15.0;
    p
}

fn witness_options(min_terms: u64, budget: usize) -> LocalOptions {
    LocalOptions {
        positive_pair_witnesses: true,
        positive_pair_separation: false,
        domain_pruning: false,
        pair_witness_min_resultant_terms: min_terms,
        pair_witness_axis_budget: budget,
        ..options(false, false)
    }
}

#[test]
fn cheap_resultants_decline_new_witness_templates_without_changing_coverage() {
    let p = witness_problem();
    let historical = certified(&p, &witness_options(0, 0));
    let gated = certified(&p, &witness_options(4096, 0));
    let ordinary = certified(
        &p,
        &LocalOptions {
            positive_pair_witnesses: false,
            ..witness_options(0, 0)
        },
    );
    let (h, g) = (report(&historical), report(&gated));
    assert!(h.pair_witness_templates > 0);
    assert_eq!(h.pair_witness_cheap_resultant_declines, 0);
    assert_eq!(g.pair_witness_templates, 0);
    assert!(g.pair_witness_cheap_resultant_declines > 0);
    assert!(g.projected_pairs > 0);
    equivalent(&gated, &historical);
    equivalent(&gated, &ordinary);
    // A threshold below the bound of this pair keeps the historical behaviour.
    let low = certified(&p, &witness_options(1, 0));
    assert_eq!(
        report(&low).pair_witness_templates,
        h.pair_witness_templates
    );
}

#[test]
fn per_axis_witness_budget_bounds_new_templates() {
    let p = problem(
        &["a", "z"],
        &["a>2", "a<100", "z>0"],
        &["(z-a)^2-2", "(z-a-1)^2-3"],
    );
    let unlimited = certified(&p, &witness_options(0, 0));
    let budgeted = certified(&p, &witness_options(0, 1));
    assert!(report(&unlimited).pair_witness_templates >= 2);
    assert_eq!(report(&budgeted).pair_witness_templates, 1);
    assert!(report(&budgeted).pair_witness_axis_budget_declines > 0);
    equivalent(&budgeted, &unlimited);
}

/// Gluza33 base piece {a>0} (six homogeneous variables, anchor x4). With
/// pair witnesses enabled and the historical always-attempt policy (still the
/// default) this spends millions of speculative nodes; the opt-in admission
/// gate keeps its sample-independent resultants and completes immediately.
fn gluza_base_piece() -> Problem {
    let a = "x0^2-8*x0*x1+x1^2+2*x0*x2-8*x1*x2+x2^2-8*x0*x4+2*x1*x4-8*x2*x4+x4^2-8*x0*x6+2*x1*x6-8*x2*x6+2*x4*x6+x6^2+2*x0*x7-8*x1*x7+2*x2*x7-8*x4*x7-8*x6*x7+x7^2";
    let mut p = problem(
        &["x0", "x1", "x2", "x4", "x6", "x7"],
        &[
            &format!("{a} > 0"),
            "x0 > 0",
            "x1 > 0",
            "x2 > 0",
            "x4 > 0",
            "x6 > 0",
            "x7 > 0",
        ],
        &[],
    );
    p.variable_groups = vec![p.variables.clone()];
    let s = &mut p.solver;
    s.engine = "local".into();
    s.ordering = "given".into();
    s.order = ["x4", "x7", "x1", "x2", "x6", "x0"]
        .map(String::from)
        .to_vec();
    s.positivity = true;
    s.local_descartes_witnesses = true;
    s.local_descartes_intervals = true;
    s.local_positive_pair_witnesses = true;
    s.local_adjacent_pairs = true;
    s.local_max_nodes = Some(200_000);
    s.homogeneous_block = p.variables.clone();
    s.homogeneous_anchor = Some("x4".into());
    p.limits.wall_time_secs = 60.0;
    p
}

fn reduced_report(result: &SolveResult) -> &LocalReport {
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let Some(LocalEvidence::Local { report, .. }) = reduced.local.as_deref() else {
        panic!("missing reduced local proof: {:?}", reduced.diagnostic)
    };
    report
}

#[test]
fn gluza33_positive_base_piece_with_pair_witnesses_completes_under_the_gate() {
    let mut p = gluza_base_piece();
    // The gate is opt-in: the default keeps the historical policy.
    assert_eq!(p.solver.local_pair_witness_min_resultant_terms, 0);
    p.solver.local_pair_witness_min_resultant_terms = 4096;
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric, "{:?}", result.diagnostic);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let r = reduced_report(&result);
    assert!(r.pair_witness_cheap_resultant_declines > 0);
    assert!(r.speculative_nodes < 10_000, "{}", r.speculative_nodes);
    // Here the gate admits no template at all, so the search is exactly the
    // witness-free one: same nodes and the same exact coverage. The gate's
    // benefit over simply disabling witnesses is unmeasured on such inputs.
    assert_eq!(r.pair_witness_templates, 0);
    let mut ordinary = p.clone();
    ordinary.solver.local_positive_pair_witnesses = false;
    let baseline = symgcad::solver::solve(&ordinary).unwrap();
    assert!(symgcad::verify::verify(&baseline).unwrap().verified);
    assert_eq!(
        reduced_report(&baseline).speculative_nodes,
        r.speculative_nodes
    );
    assert_eq!(
        symgcad::comparison::compare_generic(&result, &baseline, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn default_witness_inputs_run_and_serialize_as_before_the_gate() {
    // A pre-gate problem file with witnesses on keeps the always-attempt
    // policy and evidence that older verifiers (deny_unknown_fields) accept.
    let mut p = witness_problem();
    p.solver.engine = "local".into();
    p.solver.positivity = false;
    p.solver.local_positive_pair_witnesses = true;
    let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    assert!(
        !toml::to_string(&restored)
            .unwrap()
            .contains("pair_witness_min_resultant_terms")
    );
    let result = symgcad::solver::solve(&restored).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric, "{:?}", result.diagnostic);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let evidence = serde_json::to_value(result.local.as_deref().unwrap()).unwrap();
    let options = &evidence["options"];
    assert_eq!(options["positive_pair_witnesses"], true);
    assert!(options.get("pair_witness_min_resultant_terms").is_none());
    assert!(options.get("pair_witness_axis_budget").is_none());
    let LocalEvidence::Local { report, .. } = result.local.as_deref().unwrap() else {
        panic!()
    };
    assert_eq!(report.pair_witness_cheap_resultant_declines, 0);
}

/// Review reproducer: a retained sample inside a new root's coarse isolating
/// interval but on the far side of the root must not be reused there.
#[test]
fn retained_children_are_reused_only_after_an_exact_membership_check() {
    let p = Problem::from_toml(include_str!(
        "fixtures/local_retained_sibling_wide_interval.toml"
    ))
    .unwrap();
    assert!(p.solver.local_retain_siblings);
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric, "{:?}", result.diagnostic);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let LocalEvidence::Local { report: cli, .. } = result.local.as_deref().unwrap() else {
        panic!("missing local proof")
    };
    assert!(cli.retained_sibling_proofs > 0);
    // The library engine agrees with and without retention.
    let retained = certified(&p, &options(true, true));
    assert!(report(&retained).retained_sibling_proofs > 0);
    equivalent(&retained, &certified(&p, &options(false, false)));
}

#[test]
fn witness_policy_is_recorded_only_when_witnesses_are_enabled() {
    let mut p = gluza_base_piece();
    p.solver.local_positive_pair_witnesses = false;
    let result = symgcad::solver::solve(&p).unwrap();
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let evidence = serde_json::to_value(reduced.local.as_deref().unwrap()).unwrap();
    assert!(evidence["options"].get("pair_witness_min_resultant_terms").is_none());
    assert!(evidence["options"].get("retain_sibling_proofs").is_none());
    // Historical evidence without the policy fields still deserializes to the
    // always-attempt policy and replays unchanged.
    let restored: symgcad::local_projection::LocalEvidence =
        serde_json::from_value(evidence).unwrap();
    let LocalEvidence::Local { options, .. } = restored else {
        panic!()
    };
    assert_eq!(options.pair_witness_min_resultant_terms, 0);
    assert_eq!(options.pair_witness_axis_budget, 0);
    assert!(!options.retain_sibling_proofs);
    assert!(
        serde_json::to_value(&gluza_base_piece()).unwrap()["solver"]
            .get("local_pair_witness_min_resultant_terms")
            .is_none()
    );
}

/// Random review-fuzz seed 25 (a 64-node retention budget binds). Before the
/// early break was restored under memory refusals, retention finished every
/// round and needed 14,609 speculative nodes against 8,585 for stable samples
/// alone at the same memory limit; it must not exceed the latter now.
#[test]
fn binding_retention_budget_keeps_the_ordinary_early_break() {
    let mut p = problem(
        &["x", "y", "z"],
        &["-1-3*x+3*x+2*z > 0"],
        &[
            "-3*x^3+x*y^2-y^2*z+z",
            "x+2*y+z",
            "2*y^2+2*x-3*x*y-2*x^2+z",
            "-1-x+z",
        ],
    );
    p.solver.engine = "local".into();
    p.solver.ordering = "given".into();
    p.solver.order = ["x", "y", "z"].map(String::from).to_vec();
    p.solver.local_stable_samples = true;
    p.limits.memory_mib = 1;
    let solve = |p: &Problem| {
        let result = symgcad::solver::solve(p).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric, "{:?}", result.diagnostic);
        assert!(symgcad::verify::verify(&result).unwrap().verified);
        result
    };
    let local_report = |result: &SolveResult| {
        let Some(LocalEvidence::Local { report, .. }) = result.local.as_deref() else {
            panic!("missing local proof")
        };
        report.clone()
    };
    let stable = solve(&p);
    p.solver.local_retain_siblings = true;
    let retained = solve(&p);
    let (s, r) = (local_report(&stable), local_report(&retained));
    eprintln!(
        "retained {} nodes ({} reuses, {} budget drops); stable {} nodes",
        r.speculative_nodes,
        r.retained_sibling_proofs,
        r.retained_sibling_budget_drops,
        s.speculative_nodes
    );
    assert_eq!(r.retained_sibling_budget, 64);
    assert!(r.retained_sibling_budget_drops > 0);
    assert!(r.speculative_nodes <= s.speculative_nodes);
    // Both are independently verified; an exact common refinement of these
    // 3,344-cell decompositions takes minutes, so only the counts are compared.
    assert_eq!(retained.cells.len(), stable.cells.len());
}
