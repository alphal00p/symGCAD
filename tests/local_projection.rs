use local_projection::{LocalEvidence, LocalNode, LocalOptions, solve_local, verify_local};
use symgcad::{Problem, local_projection, output::Status};

fn problem(variables: &[&str], constraints: &[&str], split: &[&str]) -> Problem {
    Problem {
        variables: variables.iter().map(|v| (*v).into()).collect(),
        constraints: constraints.iter().map(|v| (*v).into()).collect(),
        split: split.iter().map(|v| (*v).into()).collect(),
        ..Problem::default()
    }
}

fn local(p: &Problem) -> local_projection::LocalSolveResult {
    let result = solve_local(p, &LocalOptions::default()).unwrap();
    assert_eq!(
        result.solution.status,
        Status::CompleteGeneric,
        "{:?}",
        result.evidence
    );
    assert!(matches!(result.evidence, LocalEvidence::Local { .. }));
    assert!(verify_local(&result).unwrap().verified);
    result
}

#[test]
fn univariate_strict_signs_have_all_sectors_and_exclude_roots() {
    let result = local(&problem(&["x"], &["x>0"], &["x^2-2"]));
    assert_eq!(result.solution.cells.len(), 2);
    assert_eq!(result.solution.cells[0].signs, vec![-1]);
    assert_eq!(result.solution.cells[1].signs, vec![1]);
}

#[test]
fn a_vanishing_discriminant_sample_refines_its_owning_ancestor() {
    let result = local(&problem(&["x", "y"], &[], &["y^2+x"]));
    assert_eq!(result.solution.cells.len(), 4);
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.refinements > 0);
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|c| c.axes[0].sample != "0")
    );
}

#[test]
fn leading_coefficient_nullification_refines_instead_of_certifying_a_point() {
    let result = local(&problem(&["a", "b", "z"], &[], &["a*z+b"]));
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|c| c.axes[0].sample != "0")
    );
}

#[test]
fn rootless_cuts_omit_only_unnecessary_pair_resultants() {
    let result = local(&problem(&["x", "y"], &[], &["y^2+x", "y^2+x+1"]));
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!()
    };
    assert!(report.omitted_rootless_pairs > 0);
    assert!(report.projected_pairs > 0);
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|c| c.signs.iter().all(|&s| s != 0))
    );
}

#[test]
fn sibling_boundaries_are_jointly_projected_at_the_ancestor() {
    let result = local(&problem(&["t", "x", "y"], &["x*y+1>0"], &["y^2+t", "y-x"]));
    assert!(result.solution.cells.len() > 3);
}

#[test]
fn early_negative_constraints_do_not_construct_unnecessary_descendants() {
    let result = local(&problem(&["x", "y", "z"], &["x<0", "x>0"], &["z^3+y*z+x"]));
    assert!(result.solution.cells.is_empty());
    let LocalEvidence::Local {
        root: LocalNode::Stack { sectors, .. },
        ..
    } = result.evidence
    else {
        panic!()
    };
    assert!(
        sectors
            .iter()
            .all(|s| matches!(s.child, LocalNode::Rejected { .. }))
    );
}

#[test]
fn zero_split_is_a_constant_sign_annotation() {
    let result = local(&problem(&["x"], &["x>0"], &["0"]));
    assert_eq!(result.solution.cells.len(), 1);
    assert_eq!(result.solution.cells[0].signs, vec![0]);
    let result = local(&problem(&[], &[], &["0", "0"]));
    assert_eq!(result.solution.cells.len(), 1);
    assert_eq!(result.solution.cells[0].signs, vec![0, 0]);
    let result = local(&problem(&["x"], &["x>0"], &["0", "x-1"]));
    assert_eq!(result.solution.cells.len(), 2);
    assert!(result.solution.cells.iter().all(|c| c.signs[0] == 0));
}

#[test]
fn constant_and_zero_dimensional_problems_are_certified() {
    assert_eq!(
        local(&problem(&[], &["2>0"], &["-3"])).solution.cells.len(),
        1
    );
    assert!(
        local(&problem(&[], &["0>0"], &[]))
            .solution
            .cells
            .is_empty()
    );
}

#[test]
fn fixed_order_and_parameter_outer_groups_are_preserved() {
    let mut p = problem(&["z", "t"], &["z>0"], &["z^2+t"]);
    p.variable_groups = vec![vec!["t".into()], vec!["z".into()]];
    let result = local(&p);
    assert_eq!(result.solution.order, vec!["t", "z"]);
    p.solver.order = vec!["z".into(), "t".into()];
    assert!(solve_local(&p, &LocalOptions::default()).is_err());
}

#[test]
fn resource_limits_never_claim_completion_or_start_fallback() {
    let p = problem(&["x", "y"], &[], &["y^2+x"]);
    let result = solve_local(
        &p,
        &LocalOptions {
            max_refinements: 0,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    assert_eq!(result.solution.status, Status::Incomplete);
    assert!(matches!(result.evidence, LocalEvidence::Unfinished { .. }));
    assert!(verify_local(&result).is_err());
    let mut p = p;
    p.limits.max_cells = Some(1);
    let result = solve_local(&p, &LocalOptions::default()).unwrap();
    assert_eq!(result.solution.status, Status::Incomplete);
    p.limits.max_cells = None;
    p.limits.wall_time_secs = 1e-9;
    assert_eq!(
        solve_local(&p, &LocalOptions::default())
            .unwrap()
            .solution
            .status,
        Status::Incomplete
    );
}

#[test]
fn unsupported_dynamic_order_is_an_explicit_verified_global_fallback() {
    let mut p = problem(&["x"], &["x>0"], &["x-1"]);
    p.solver.ordering = "mods".into();
    let result = solve_local(&p, &LocalOptions::default()).unwrap();
    assert!(matches!(
        result.evidence,
        LocalEvidence::GlobalFallback { .. }
    ));
    assert!(verify_local(&result).unwrap().verified);
}

#[test]
fn tampering_with_sectors_guards_and_labels_is_rejected() {
    let original = local(&problem(&["x", "y"], &[], &["y^2+x"]));
    let mut changed = original.clone();
    let LocalEvidence::Local {
        root: LocalNode::Stack { sectors, .. },
        ..
    } = &mut changed.evidence
    else {
        panic!()
    };
    sectors.pop();
    assert!(verify_local(&changed).is_err());
    let mut changed = original.clone();
    changed.solution.cells[0].signs[0] *= -1;
    assert!(verify_local(&changed).is_err());
    let mut changed = original.clone();
    fn remove_guard(node: &mut LocalNode) -> bool {
        if let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        {
            if !projection.guards.is_empty() {
                projection.guards.clear();
                return true;
            }
            return sectors.iter_mut().any(|s| remove_guard(&mut s.child));
        }
        false
    }
    let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
        panic!()
    };
    assert!(remove_guard(root));
    assert!(verify_local(&changed).is_err());
    let mut changed = original;
    changed.solution.exceptional_polynomials.clear();
    assert!(verify_local(&changed).is_err());
}

#[test]
fn certificates_roundtrip_without_a_discovery_trace_or_sample_only_shortcut() {
    let result = local(&problem(&["x", "y"], &["x>0", "y>0"], &["x*y-1"]));
    let decoded = serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    assert!(verify_local(&decoded).unwrap().verified);
}

#[test]
fn local_engine_dispatch_and_homogeneous_outer_reduction_compose() {
    let mut p = problem(&["r", "x", "y"], &["r>0", "x>0", "y>0"], &["x*y-r^2"]);
    p.solver.engine = "local".into();
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.homogeneous_anchor = Some("r".into());
    let result = symgcad::solver::solve(&p).unwrap();
    assert!(result.homogeneous.as_ref().unwrap().reduced.local.is_some());
    assert!(symgcad::verify::verify(&result).unwrap().verified);
}

#[test]
fn production_fallback_is_labeled_without_recursive_verifier_dispatch() {
    let mut p = problem(&["x"], &["x>0"], &["x-1"]);
    p.solver.engine = "local".into();
    p.solver.ordering = "mods".into();
    let result = symgcad::solver::solve(&p).unwrap();
    assert!(matches!(
        result.local.as_deref(),
        Some(LocalEvidence::GlobalFallback { .. })
    ));
    assert_eq!(result.problem.solver.engine, "optimized");
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let result = solve_local(
        &p,
        &LocalOptions {
            fallback_to_global: false,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    assert_eq!(result.solution.status, Status::ComputationFailure);
    assert!(matches!(
        result.evidence,
        LocalEvidence::NotApplicable { .. }
    ));
}

#[test]
fn cheap_local_ordering_preserves_parameter_groups() {
    let mut p = problem(&["x", "y", "s"], &["x>0", "y>0", "s>4"], &["x^2+y-s"]);
    p.solver.engine = "local".into();
    p.variable_groups = vec![vec!["s".into()], vec!["x".into(), "y".into()]];
    for name in ["brown", "sparsity"] {
        p.solver.ordering = name.into();
        let result = symgcad::solver::solve(&p).unwrap();
        assert_eq!(result.order[0], "s");
        assert!(matches!(
            result.local.as_deref(),
            Some(LocalEvidence::Local { .. })
        ));
        assert!(symgcad::verify::verify(&result).unwrap().verified);
    }
}

#[test]
fn cli_local_solve_and_verify_roundtrip() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let directory = std::path::PathBuf::from("artifacts/tests").join(format!(
        "local-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let input = directory.join("input.toml");
    let output = directory.join("result.json");
    let mut p = problem(&["x", "y"], &["x>0"], &["y^2+x", "y-x"]);
    p.solver.engine = "local".into();
    p.limits.wall_time_secs = 10.0;
    p.limits.workers = 1;
    p.limits.memory_mib = 1024;
    fs::write(&input, toml::to_string(&p).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command
        .arg("solve")
        .arg(&input)
        .arg("--output")
        .arg(&output);
    let run = symgcad::benchmark::run_bounded(&mut command, &p.limits).unwrap();
    assert_eq!(run.status, "completed", "{}", run.stderr);
    let result: symgcad::output::SolveResult =
        serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(matches!(
        result.local.as_deref(),
        Some(LocalEvidence::Local { .. })
    ));
    let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
    command.arg("verify").arg(&output);
    let run = symgcad::benchmark::run_bounded(&mut command, &p.limits).unwrap();
    assert_eq!(run.status, "completed", "{}", run.stderr);
    let verification: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(verification["verified"], true);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn rootless_pair_omission_cannot_cross_an_unresolved_discriminant_seam() {
    let result = local(&problem(&["x", "y", "z"], &[], &["z^2-x", "z-y"]));
    // x<0: the quadratic is rootless, y is unsplit, and z has two sectors.
    // x>0: y has three sectors relative to +/-sqrt(x), each with four z sectors.
    assert_eq!(result.solution.cells.len(), 14);
    assert_eq!(
        result
            .solution
            .cells
            .iter()
            .filter(|c| c.axes[0].sample.starts_with('-'))
            .count(),
        2
    );
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|c| c.axes[0].sample != "0")
    );
}

fn local_report(result: &local_projection::LocalSolveResult) -> &local_projection::LocalReport {
    let LocalEvidence::Local { report, .. } = &result.evidence else {
        panic!("not a local completion")
    };
    report
}

fn same_mathematical_result(
    left: &local_projection::LocalSolveResult,
    right: &local_projection::LocalSolveResult,
) {
    assert_eq!(left.solution.cells, right.solution.cells);
    assert_eq!(left.solution.polynomials, right.solution.polynomials);
    assert_eq!(
        left.solution.projection_levels,
        right.solution.projection_levels
    );
    assert_eq!(
        left.solution.exceptional_polynomials,
        right.solution.exceptional_polynomials
    );
    let LocalEvidence::Local {
        root: left_tree,
        input_factors: left_inputs,
        guard_certificates: left_certificates,
        ..
    } = &left.evidence
    else {
        panic!()
    };
    let LocalEvidence::Local {
        root: right_tree,
        input_factors: right_inputs,
        guard_certificates: right_certificates,
        ..
    } = &right.evidence
    else {
        panic!()
    };
    assert_eq!(left_inputs, right_inputs);
    assert_eq!(left_certificates, right_certificates);
    assert_eq!(
        serde_json::to_value(left_tree).unwrap(),
        serde_json::to_value(right_tree).unwrap()
    );
}

#[test]
fn operation_cache_reuses_exact_algebra_without_changing_the_proof_tree() {
    let p = problem(&["x", "y", "z"], &[], &["z^2-x", "z-y"]);
    let cached = solve_local(&p, &LocalOptions::default()).unwrap();
    let uncached = solve_local(
        &p,
        &LocalOptions {
            cache_enabled: false,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    same_mathematical_result(&cached, &uncached);
    let hit = local_report(&cached);
    let miss = local_report(&uncached);
    assert!(hit.cache.factor_hits > 0);
    assert!(hit.cache.discriminant_hits > 0);
    assert!(hit.cache.resultant_hits > 0);
    assert_eq!(
        miss.cache.factor_hits + miss.cache.discriminant_hits + miss.cache.resultant_hits,
        0
    );
    assert!(hit.cache.factor_misses < miss.cache.factor_misses);
    assert!(hit.phases.factor_seconds >= 0.0 && hit.phases.resultant_seconds >= 0.0);
    assert!(hit.phases.root_isolation_seconds > 0.0);
    // Each proof is independently recomputed; no cache data is trusted here.
    assert!(verify_local(&cached).unwrap().verified);
    assert!(verify_local(&uncached).unwrap().verified);
}

#[test]
fn persistent_operations_survive_a_fresh_local_solver_without_changing_obligations() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let directory = std::path::PathBuf::from("artifacts/tests").join(format!(
        "local-cache-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut p = problem(&["x", "y", "z"], &[], &["z^2-x", "z-y"]);
    p.solver.checkpoint_dir = Some(directory.clone());
    p.solver.cache_memory_mib = 0;
    let cold = solve_local(&p, &LocalOptions::default()).unwrap();
    let warm = solve_local(&p, &LocalOptions::default()).unwrap();
    assert!(local_report(&cold).cache.written_operation_checkpoints > 0);
    let warm_metrics = &local_report(&warm).cache;
    assert!(
        warm_metrics.factor_disk_hits
            + warm_metrics.discriminant_disk_hits
            + warm_metrics.resultant_disk_hits
            > 0
    );
    assert_eq!(
        warm_metrics.factor_misses
            + warm_metrics.discriminant_misses
            + warm_metrics.resultant_misses,
        0
    );
    same_mathematical_result(&cold, &warm);
    assert!(verify_local(&warm).unwrap().verified);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn interrupted_local_work_retains_cache_and_phase_metrics_without_a_complete_proof() {
    let p = problem(&["x", "y"], &[], &["y^2+x"]);
    let result = solve_local(
        &p,
        &LocalOptions {
            max_nodes: 1,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    assert_eq!(result.solution.status, Status::Incomplete);
    let LocalEvidence::Unfinished {
        report: Some(report),
        ..
    } = result.evidence
    else {
        panic!()
    };
    assert!(report.cache.factor_misses > 0);
    assert!(report.elapsed_seconds > 0.0);
    assert_eq!(report.active_phase, "incomplete");
}

fn domain_problem(variables: &[&str], constraints: &[&str], split: &[&str]) -> Problem {
    let mut p = problem(variables, constraints, split);
    p.solver.engine = "local".into();
    p.solver.positivity = true;
    p
}

fn local_solution(p: &Problem, options: &LocalOptions) -> symgcad::output::SolveResult {
    let result = solve_local(p, options).unwrap();
    assert_eq!(result.solution.status, Status::CompleteGeneric);
    let mut solution = result.solution;
    solution.local = Some(Box::new(result.evidence));
    assert!(symgcad::verify::verify(&solution).unwrap().verified);
    solution
}

fn compare_three(p: &Problem) -> symgcad::output::SolveResult {
    let pruned = local_solution(p, &LocalOptions::default());
    let plain = local_solution(
        p,
        &LocalOptions {
            domain_pruning: false,
            ..LocalOptions::default()
        },
    );
    let mut global_problem = p.clone();
    global_problem.solver.engine = "baseline".into();
    global_problem.solver.positivity = false;
    let global = symgcad::solver::solve(&global_problem).unwrap();
    let limits = symgcad::Limits {
        wall_time_secs: 30.0,
        ..Default::default()
    };
    for other in [&plain, &global] {
        let comparison = symgcad::comparison::compare_generic(&pruned, other, &limits).unwrap();
        assert_eq!(comparison.equivalent, Some(true));
    }
    pruned
}

#[test]
fn certified_local_guards_export_original_bound_obligations_and_match_global_coverage() {
    let result = compare_three(&domain_problem(
        &["x", "z"],
        &["x>0", "z>0"],
        &["(x+1)*z-1"],
    ));
    let LocalEvidence::Local {
        guard_certificates,
        input_factors,
        ..
    } = result.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(!guard_certificates.is_empty());
    let algebra = symgcad::algebra::Algebra::new(&result.order).unwrap();
    let record = guard_certificates
        .iter()
        .find(|r| {
            algebra.parse(&result.polynomials[r.guard]).unwrap() == algebra.parse("x+1").unwrap()
        })
        .unwrap();
    assert_eq!(record.level, 1);
    assert_eq!(record.supporting_constraints, vec![0]);
    assert_eq!(record.supporting_factors, input_factors[0]);
    assert!(record.certificate.domain.bounds[1].lower.is_none());
    assert!(
        record
            .certificate
            .domain
            .excluded_sign_constraints
            .contains(&1)
    );
}

#[test]
fn finite_box_guard_certificates_preserve_interior_discriminant_seams() {
    let positive = compare_three(&domain_problem(
        &["x", "z"],
        &["x>0", "x<1", "z>0"],
        &["z^2+2+x-x^2"],
    ));
    let LocalEvidence::Local {
        guard_certificates, ..
    } = positive.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(guard_certificates.len() >= 2);
    assert!(
        guard_certificates
            .iter()
            .all(|r| r.supporting_constraints == vec![0, 1])
    );
    let crossing = compare_three(&domain_problem(
        &["x", "z"],
        &["x>0", "x<2", "z>0"],
        &["z^2-(x-1)^2"],
    ));
    let algebra = symgcad::algebra::Algebra::new(&crossing.order).unwrap();
    let seam = symgcad::algebra::canonical(&algebra.parse("x-1").unwrap());
    let seam_id = crossing
        .polynomials
        .iter()
        .position(|p| algebra.parse(p).unwrap() == seam)
        .unwrap();
    assert!(crossing.exceptional_polynomials.contains(&seam_id));
    let LocalEvidence::Local {
        guard_certificates, ..
    } = crossing.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(guard_certificates.iter().all(|r| r.guard != seam_id));
}

#[test]
fn rootless_real_curves_and_later_coordinate_bounds_do_not_supply_false_premises() {
    compare_three(&domain_problem(
        &["x", "y", "z"],
        &["x>0", "x<2", "z>0"],
        &["z^2-x", "z-y"],
    ));
    let result = compare_three(&domain_problem(&["x", "z"], &["z>0"], &["x*z-1"]));
    let LocalEvidence::Local {
        guard_certificates, ..
    } = result.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(guard_certificates.is_empty());
    assert!(result.cells.iter().all(|c| c.axes[0].sample != "0"));
}

#[test]
fn local_guard_pruning_protects_all_original_split_and_coordinate_factors() {
    let result = compare_three(&domain_problem(
        &["x", "z"],
        &["x>0", "z>0"],
        &["(x+1)*z-1", "x+1"],
    ));
    let LocalEvidence::Local {
        guard_certificates,
        input_factors,
        ..
    } = result.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(
        guard_certificates
            .iter()
            .all(|r| !input_factors.iter().flatten().any(|&id| id == r.guard))
    );
    let algebra = symgcad::algebra::Algebra::new(&result.order).unwrap();
    for expression in ["x", "z", "x+1"] {
        let p = algebra.parse(expression).unwrap();
        let id = result
            .polynomials
            .iter()
            .position(|s| algebra.parse(s).unwrap() == p)
            .unwrap();
        assert!(result.exceptional_polynomials.contains(&id));
    }
}

#[test]
fn domain_guard_registry_rejects_changed_bounds_missing_support_and_wrong_scope() {
    let original = local_solution(
        &domain_problem(&["x", "z"], &["x>0", "z>0"], &["(x+1)*z-1"]),
        &LocalOptions::default(),
    );
    for modification in 0..5 {
        let mut changed = original.clone();
        let LocalEvidence::Local {
            guard_certificates,
            options,
            ..
        } = changed.local.as_deref_mut().unwrap()
        else {
            panic!()
        };
        let record = &mut guard_certificates[0];
        match modification {
            0 => record.supporting_factors.clear(),
            1 => record.supporting_constraints.clear(),
            2 => record.level = 0,
            3 => record.certificate.domain.bounds[0].lower = Some("-2".into()),
            4 => options.domain_pruning = false,
            _ => unreachable!(),
        }
        assert!(
            symgcad::verify::verify(&changed).is_err(),
            "tampering {modification}"
        );
    }
    // Keeping a valid global domain certificate is insufficient if the tree
    // drops the ancestor factor obligation establishing its actual premise.
    let mut changed = original;
    fn remove_support(node: &mut LocalNode) -> bool {
        if let LocalNode::Stack {
            projection,
            required_below,
            sectors,
            ..
        } = node
        {
            if !projection.removals.is_empty() {
                required_below.clear();
                return true;
            }
            return sectors.iter_mut().any(|s| remove_support(&mut s.child));
        }
        false
    }
    let LocalEvidence::Local { root, .. } = changed.local.as_deref_mut().unwrap() else {
        panic!()
    };
    assert!(remove_support(root));
    assert!(symgcad::verify::verify(&changed).is_err());
}

#[test]
fn empty_original_boxes_do_not_create_vacuous_guard_certificates() {
    let result = local_solution(
        &domain_problem(&["x", "z"], &["x>2", "x<1", "z>0"], &["(x+1)*z-1"]),
        &LocalOptions::default(),
    );
    assert!(result.cells.is_empty());
    let LocalEvidence::Local {
        guard_certificates, ..
    } = result.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(guard_certificates.is_empty());
}

#[test]
fn cached_domain_certificates_preserve_registry_and_every_bound_obligation() {
    let p = domain_problem(
        &["x", "y", "z"],
        &["x>0", "y>0", "z>0"],
        &["(x+1)*z-1", "z-y"],
    );
    let cached = solve_local(&p, &LocalOptions::default()).unwrap();
    let uncached = solve_local(
        &p,
        &LocalOptions {
            cache_enabled: false,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    same_mathematical_result(&cached, &uncached);
    assert!(local_report(&cached).guard_certificate_hits > 0);
    assert!(verify_local(&cached).unwrap().verified);
    assert!(verify_local(&uncached).unwrap().verified);
}
