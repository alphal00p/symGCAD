//! The refinement diagnostic (`SYMGCAD_LOCAL_THRASH_DIAG`) must be read-only:
//! same proof tree, same search counters and the same operation-cache
//! counters (it may not factor anything, including base-univariate sources
//! that `guard_basis` deliberately leaves unfactored). This lives in its own
//! test binary with a single test because it toggles a process environment
//! variable.
use serde_json::Value;
use symgcad::{
    Problem,
    local_projection::{LocalEvidence, LocalOptions, LocalReport, LocalSolveResult, solve_local},
    output::Status,
};

const DIAG: &str = "SYMGCAD_LOCAL_THRASH_DIAG";

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

fn cases() -> Vec<(Problem, LocalOptions)> {
    let base = LocalOptions {
        fallback_to_global: false,
        ..Default::default()
    };
    let mut univariate = problem(&["x", "z"], &[], &["z^2-(x-1)*(x-2)", "z-x"]);
    univariate.solver.engine = "local".into();
    univariate.solver.ordering = "given".into();
    univariate.solver.local_base_univariate_basis = true;
    vec![
        // Base-univariate sources are never factored by guard_basis.
        (
            univariate,
            LocalOptions {
                base_univariate_basis: true,
                base_point_projection: true,
                domain_pruning: false,
                positive_ray: false,
                ..base.clone()
            },
        ),
        // Pair-witness separators and coefficient attribution.
        (
            problem(&["a", "z"], &["a>1", "a<2", "z>0"], &["z^2-2", "z^2+a*z-4"]),
            LocalOptions {
                positive_pair_witnesses: true,
                positive_pair_separation: false,
                domain_pruning: false,
                ..base.clone()
            },
        ),
        // Refinement restarts with retained siblings.
        (
            problem(
                &["x", "y", "z", "w"],
                &[],
                &["x", "y", "z-x", "w^2-z", "w-y", "w-2"],
            ),
            LocalOptions {
                stable_sector_samples: true,
                retain_sibling_proofs: true,
                ..base
            },
        ),
    ]
}

fn report(local: &LocalSolveResult) -> &LocalReport {
    let LocalEvidence::Local { report, .. } = &local.evidence else {
        panic!("missing local proof: {:?}", local.solution.diagnostic)
    };
    report
}

fn cache_counters(report: &LocalReport) -> Value {
    let mut cache = serde_json::to_value(&report.cache).unwrap();
    // Only counters; drop any timing-like floats should they ever be added.
    if let Value::Object(map) = &mut cache {
        map.retain(|_, v| !v.is_f64());
    }
    cache
}

fn proof(local: &LocalSolveResult) -> Value {
    let LocalEvidence::Local { root, .. } = &local.evidence else {
        panic!()
    };
    serde_json::to_value(root).unwrap()
}

fn solve(p: &Problem, options: &LocalOptions, diagnostic: bool) -> LocalSolveResult {
    // SAFETY: this binary has exactly one test, so no other thread reads the
    // environment concurrently.
    unsafe {
        if diagnostic {
            std::env::set_var(DIAG, "1");
        } else {
            std::env::remove_var(DIAG);
        }
    }
    let local = solve_local(p, options).unwrap();
    assert_eq!(
        local.solution.status,
        Status::CompleteGeneric,
        "{:?}",
        local.solution.diagnostic
    );
    assert!(symgcad::local_projection::verify_local(&local).unwrap().verified);
    local
}

#[test]
fn thrash_diagnostic_is_read_only() {
    for (p, options) in cases() {
        let plain = solve(&p, &options, false);
        let diag = solve(&p, &options, true);
        let (a, b) = (report(&plain), report(&diag));
        assert!(a.thrash.is_none());
        let thrash = b.thrash.as_ref().expect("diagnostic report");
        assert!(thrash.stack_visits_by_level.iter().sum::<usize>() > 0);
        assert_eq!(proof(&plain), proof(&diag));
        assert_eq!(a.speculative_nodes, b.speculative_nodes);
        assert_eq!(a.refinements, b.refinements);
        assert_eq!(a.refinement_cuts, b.refinement_cuts);
        assert_eq!(a.retained_sibling_proofs, b.retained_sibling_proofs);
        assert_eq!(cache_counters(a), cache_counters(b));
        // Distinct cuts never exceed addition events.
        let events: usize = thrash.additions_by_origin.values().sum();
        let distinct: usize = thrash.distinct_additions_by_origin.values().sum();
        assert!(distinct <= events);
        assert_eq!(distinct > 0, a.refinement_cuts > 0);
    }
    unsafe { std::env::remove_var(DIAG) };
}
