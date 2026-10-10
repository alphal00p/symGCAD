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
    // Both a backend factorization and a freshly proved primitive-linear
    // identity are computations avoided by a memory-cache hit.
    assert!(
        hit.cache.factor_misses + hit.cache.primitive_linear_identities
            < miss.cache.factor_misses + miss.cache.primitive_linear_identities
    );
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
    assert!(report.cache.factor_misses + report.cache.primitive_linear_identities > 0);
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

fn compare_positive_ray(p: &Problem) -> symgcad::output::SolveResult {
    let ray = local_solution(p, &LocalOptions::default());
    let full = local_solution(
        p,
        &LocalOptions {
            positive_ray: false,
            ..LocalOptions::default()
        },
    );
    let mut baseline_problem = p.clone();
    baseline_problem.solver.engine = "baseline".into();
    baseline_problem.solver.positivity = false;
    let baseline = symgcad::solver::solve(&baseline_problem).unwrap();
    for other in [&full, &baseline] {
        let comparison = symgcad::comparison::compare_generic(
            &ray,
            other,
            &symgcad::Limits {
                wall_time_secs: 30.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(comparison.equivalent, Some(true));
    }
    ray
}

fn ray_cases(node: &LocalNode, cases: &mut Vec<local_projection::PositiveRayCase>) {
    if let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    {
        if let Some(ray) = &projection.positive_ray {
            cases.extend(ray.rules.iter().map(|r| r.case.clone()));
        }
        for sector in sectors {
            ray_cases(&sector.child, cases);
        }
    }
}

#[test]
fn positive_ray_quadratic_sign_cases_match_full_real_and_global_coverage() {
    use local_projection::PositiveRayCase::*;
    let mut cases = Vec::new();
    // Each polynomial is irreducible over the parameter field. This includes
    // both signs of the leading coefficient and linear endpoint crossings.
    for expression in [
        "z^2+x*z-1",
        "-z^2+x*z+1",
        "z^2+x*z+1",
        "x*z^2-z+1",
        "x*z+1",
        "z^2+x",
        "z^3+x*z+1",
    ] {
        let result = compare_positive_ray(&problem(&["x", "z"], &["z>0"], &[expression]));
        let LocalEvidence::Local { root, .. } = result.local.as_deref().unwrap() else {
            panic!()
        };
        ray_cases(root, &mut cases);
    }
    assert!(
        cases
            .iter()
            .any(|c| matches!(c, QuadraticOpposite { leading_sign: 1 }))
    );
    assert!(
        cases
            .iter()
            .any(|c| matches!(c, QuadraticOpposite { leading_sign: -1 }))
    );
    assert!(cases.iter().any(|c| matches!(
        c,
        QuadraticPositive {
            linear_zero: false,
            ..
        }
    )));
    assert!(cases.iter().any(|c| matches!(
        c,
        QuadraticPositive {
            linear_zero: true,
            ..
        }
    )));
    assert!(cases.iter().any(|c| matches!(
        c,
        QuadraticDiscriminant {
            discriminant_sign: 1,
            positive_roots: 2,
            ..
        }
    )));
    assert!(cases.iter().any(|c| matches!(
        c,
        QuadraticDiscriminant {
            discriminant_sign: -1,
            positive_roots: 0,
            ..
        }
    )));
    assert!(cases.iter().any(|c| matches!(c, Linear { .. })));
    assert!(cases.iter().any(|c| matches!(c, FullReal)));
}

#[test]
fn negative_root_collisions_do_not_cut_the_positive_axis() {
    let p = problem(&["x", "z"], &["x>0", "z>0"], &["z^2+2*z+x"]);
    let result = compare_positive_ray(&p);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![1]);
    // D=4-4*x vanishes at x=1, where both roots collide at -1. There is no
    // positive-root graph and hence no reason to split the requested domain.
    let algebra = symgcad::algebra::Algebra::new(&result.order).unwrap();
    let seam = symgcad::algebra::canonical(&algebra.parse("x-1").unwrap());
    assert!(
        result
            .exceptional_polynomials
            .iter()
            .all(|&id| algebra.parse(&result.polynomials[id]).unwrap() != seam)
    );
    let LocalEvidence::Local { report, .. } = result.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(report.quadratic_discriminants_omitted > 0);
}

#[test]
fn positive_ray_preserves_endpoint_crossings_pair_crossings_and_zero_split_annotations() {
    compare_positive_ray(&problem(&["x", "y", "z"], &["z>0"], &["z^2-x", "z-y", "0"]));
    compare_positive_ray(&problem(&["x", "z"], &["z>0"], &["z*(z+x)"]));
    let result = compare_positive_ray(&problem(&["x", "z"], &["z>0"], &["z^2+x"]));
    assert!(result.cells.iter().all(|c| c.axes[0].sample != "0"));
    let indexed = compare_positive_ray(&problem(&["x", "z"], &["x>0", "z>0"], &["z^2-x"]));
    let algebra = symgcad::algebra::Algebra::new(&indexed.order).unwrap();
    let curve = symgcad::algebra::canonical(&algebra.parse("z^2-x").unwrap());
    let id = indexed
        .polynomials
        .iter()
        .position(|p| algebra.parse(p).unwrap() == curve)
        .unwrap();
    let bounds = indexed
        .cells
        .iter()
        .flat_map(|cell| cell.axes[1].lower.iter().chain(cell.axes[1].upper.iter()))
        .filter(|bound| bound.polynomial == id)
        .collect::<Vec<_>>();
    assert!(!bounds.is_empty());
    assert!(bounds.iter().all(|bound| bound.index == 1));
}

#[test]
fn positive_ray_requires_an_original_strict_positive_coordinate_bound() {
    for constraints in [vec!["z<0"], vec!["z+1>0"], vec!["x*z>0"], vec![]] {
        let result = local(&problem(&["x", "z"], &constraints, &["z^2+x"]));
        let LocalEvidence::Local { root, .. } = &result.evidence else {
            panic!()
        };
        let mut cases = Vec::new();
        ray_cases(root, &mut cases);
        assert!(cases.is_empty());
    }
    compare_positive_ray(&problem(&["x", "z"], &["3*z/2>0"], &["z^2+x"]));
}

#[test]
fn positive_ray_replay_rejects_changed_premise_rule_and_negative_halfline() {
    let original = local(&problem(&["x", "z"], &["x>0", "z>0"], &["z^2+2*z+x"]));
    fn change(node: &mut LocalNode, modification: usize) -> bool {
        if let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        {
            if let Some(ray) = &mut projection.positive_ray {
                if ray.rules.iter().any(|r| {
                    matches!(
                        r.case,
                        local_projection::PositiveRayCase::QuadraticPositive { .. }
                    )
                }) {
                    match modification {
                        0 => ray.constraint = 0,
                        1 => ray.coordinate = usize::MAX,
                        2 => ray.rules.clear(),
                        3 => sectors[0].child = LocalNode::Rejected { constraint: 0 },
                        _ => unreachable!(),
                    }
                    return true;
                }
            }
            return sectors
                .iter_mut()
                .any(|s| change(&mut s.child, modification));
        }
        false
    }
    for modification in 0..4 {
        let mut changed = original.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(change(root, modification));
        assert!(
            verify_local(&changed).is_err(),
            "modification {modification}"
        );
    }
    let mut changed = original;
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.positive_ray = false;
    assert!(verify_local(&changed).is_err());
}

#[test]
fn positive_ray_and_sturm_guard_proofs_preserve_independent_obligations() {
    let p = domain_problem(&["x", "z"], &["x>0", "z>0"], &["(x^4-x^2+1)*z-1"]);
    let original = compare_positive_ray(&p);
    let LocalEvidence::Local {
        guard_certificates,
        root,
        ..
    } = original.local.as_deref().unwrap()
    else {
        panic!()
    };
    let record = guard_certificates
        .iter()
        .find(|r| {
            matches!(
                r.certificate.method,
                symgcad::domain::CertificateMethod::UnivariateSturm { variable: 0 }
            )
        })
        .unwrap();
    assert_eq!(record.supporting_constraints, vec![0]);
    assert!(!record.supporting_factors.is_empty());
    let mut cases = Vec::new();
    ray_cases(root, &mut cases);
    assert!(
        cases
            .iter()
            .any(|c| matches!(c, local_projection::PositiveRayCase::Linear { .. }))
    );
    let mut changed = original;
    let LocalEvidence::Local {
        guard_certificates, ..
    } = changed.local.as_deref_mut().unwrap()
    else {
        panic!()
    };
    let record = guard_certificates
        .iter_mut()
        .find(|r| {
            matches!(
                r.certificate.method,
                symgcad::domain::CertificateMethod::UnivariateSturm { .. }
            )
        })
        .unwrap();
    record.certificate.sign *= -1;
    assert!(symgcad::verify::verify(&changed).is_err());
}

#[test]
fn descartes_zero_variations_omit_only_negative_root_collisions() {
    for expression in ["z^3+3*z^2+3*z+x", "z^4+4*z^3+6*z^2+4*z+x"] {
        let result = compare_positive_ray(&problem(&["x", "z"], &["x>0", "z>0"], &[expression]));
        assert_eq!(result.cells.len(), 1);
        assert_eq!(result.cells[0].signs, vec![1]);
        let LocalEvidence::Local { root, report, .. } = result.local.as_deref().unwrap() else {
            panic!()
        };
        let mut cases = Vec::new();
        ray_cases(root, &mut cases);
        assert!(cases.iter().any(|c| matches!(
            c,
            local_projection::PositiveRayCase::DescartesZero {
                coefficient_sign: 1
            }
        )));
        assert!(report.descartes_discriminants_omitted > 0);
    }
    // A nonconstant coefficient vanishing at the tentative x=0 prefix must
    // become an explicit ancestor cut, not silently count as an absent term.
    let result = compare_positive_ray(&problem(&["x", "z"], &["z>0"], &["z^3+x*z^2+z+1"]));
    assert!(result.cells.iter().all(|cell| cell.axes[0].sample != "0"));
    let mut changed = result;
    fn alter(node: &mut LocalNode) -> bool {
        if let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        {
            if let Some(ray) = &mut projection.positive_ray {
                if let Some(rule) = ray.rules.iter_mut().find(|rule| {
                    matches!(
                        rule.case,
                        local_projection::PositiveRayCase::DescartesZero { .. }
                    )
                }) {
                    rule.case = local_projection::PositiveRayCase::DescartesZero {
                        coefficient_sign: -1,
                    };
                    return true;
                }
            }
            return sectors.iter_mut().any(|sector| alter(&mut sector.child));
        }
        false
    }
    let LocalEvidence::Local { root, .. } = changed.local.as_deref_mut().unwrap() else {
        panic!()
    };
    assert!(alter(root));
    assert!(symgcad::verify::verify(&changed).is_err());
}

fn pair_witnesses<'a>(
    node: &'a LocalNode,
    output: &mut Vec<&'a local_projection::PositivePairSeparation>,
) {
    if let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    {
        if let Some(ray) = &projection.positive_ray {
            output.extend(&ray.separations);
        }
        for sector in sectors {
            pair_witnesses(&sector.child, output);
        }
    }
}

#[test]
fn positive_pair_separation_omits_only_negative_intersections() {
    let p = problem(&["x", "z"], &["x>0", "z>0"], &["z^2-x", "z^2-z-x-1"]);
    let separated = compare_positive_ray(&p);
    let ordinary_pairs = local_solution(
        &p,
        &LocalOptions {
            positive_pair_separation: false,
            ..LocalOptions::default()
        },
    );
    assert_eq!(separated.cells.len(), 3);
    assert!(ordinary_pairs.cells.len() > separated.cells.len());
    assert_eq!(
        symgcad::comparison::compare_generic(
            &separated,
            &ordinary_pairs,
            &symgcad::Limits::default()
        )
        .unwrap()
        .equivalent,
        Some(true)
    );
    let algebra = symgcad::algebra::Algebra::new(&separated.order).unwrap();
    let irrelevant = symgcad::algebra::canonical(&algebra.parse("x-1").unwrap());
    assert!(
        separated
            .exceptional_polynomials
            .iter()
            .all(|&id| algebra.parse(&separated.polynomials[id]).unwrap() != irrelevant)
    );
    let LocalEvidence::Local { root, report, .. } = separated.local.as_deref().unwrap() else {
        panic!()
    };
    let mut proofs = Vec::new();
    pair_witnesses(root, &mut proofs);
    assert!(!proofs.is_empty());
    assert!(proofs.iter().all(|proof| proof.shift == 0));
    assert!(report.pair_resultants_omitted > 0);
}

#[test]
fn positive_pair_separation_handles_unequal_degrees_and_negative_leading_signs() {
    for cubic in ["z^3-(x+1)*z-1", "-z^3+(x+1)*z+1"] {
        let p = problem(&["x", "z"], &["x>0", "z>0"], &[cubic, "z^2-x"]);
        let separated = compare_positive_ray(&p);
        let ordinary = local_solution(
            &p,
            &LocalOptions {
                positive_pair_separation: false,
                ..LocalOptions::default()
            },
        );
        assert_eq!(
            symgcad::comparison::compare_generic(
                &separated,
                &ordinary,
                &symgcad::Limits::default()
            )
            .unwrap()
            .equivalent,
            Some(true)
        );
        let LocalEvidence::Local { root, .. } = separated.local.as_deref().unwrap() else {
            panic!()
        };
        let mut proofs = Vec::new();
        pair_witnesses(root, &mut proofs);
        assert!(proofs.iter().any(|proof| proof.shift == 1));
    }
}

#[test]
fn positive_pair_mixed_and_zero_coefficient_samples_use_the_ordinary_resultant() {
    let p = problem(&["x", "z"], &["x>0", "z>0"], &["z^2-x", "z^2+z-x-1"]);
    let result = compare_positive_ray(&p);
    assert!(result.cells.iter().all(|cell| cell.axes[0].sample != "1"));
    compare_positive_ray(&problem(
        &["x", "y", "z"],
        &["y>0", "z>0"],
        &["z^2-y", "z^2-x*z-y-1"],
    ));
}

#[test]
fn positive_pair_replay_rejects_changed_identity_sign_and_missing_coefficient_obligations() {
    let p = problem(&["x", "y", "z"], &["y>0", "z>0"], &["z^2-y", "z^2-x*z-y-1"]);
    let original = local(&p);
    fn alter(node: &mut LocalNode, modification: usize) -> bool {
        if let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        {
            if let Some(ray) = &mut projection.positive_ray {
                if let Some(proof) = ray.separations.first_mut() {
                    match modification {
                        0 => proof.coefficient_sign *= -1,
                        1 => proof.shift += 1,
                        2 => proof.higher = proof.lower,
                        3 => ray.separations.clear(),
                        4 => projection.guards.clear(),
                        _ => unreachable!(),
                    }
                    return true;
                }
            }
            return sectors
                .iter_mut()
                .any(|sector| alter(&mut sector.child, modification));
        }
        false
    }
    for modification in 0..5 {
        let mut changed = original.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(alter(root, modification));
        assert!(
            verify_local(&changed).is_err(),
            "modification {modification}"
        );
    }
    let mut changed = original;
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.positive_pair_separation = false;
    assert!(verify_local(&changed).is_err());
}

#[test]
fn evidence_without_the_new_pair_option_replays_its_original_resultants() {
    let original = solve_local(
        &problem(&["x", "z"], &["x>0", "z>0"], &["z^2-x", "z^2-z-x-1"]),
        &LocalOptions {
            positive_pair_separation: false,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    let mut serialized = serde_json::to_value(&original).unwrap();
    serialized["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("positive_pair_separation");
    let restored: local_projection::LocalSolveResult = serde_json::from_value(serialized).unwrap();
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert!(!options.positive_pair_separation);
    assert!(verify_local(&restored).unwrap().verified);
}

#[test]
fn second_pair_cancellation_matches_one_step_and_global_generic_coverage() {
    let p = problem(&["x", "z"], &["x>1", "x<4", "z>0"], &["z^2-x", "z^2-2*z-1"]);
    let second = compare_positive_ray(&p);
    let first = local_solution(
        &p,
        &LocalOptions {
            positive_pair_cancellations: 1,
            ..LocalOptions::default()
        },
    );
    assert_eq!(
        symgcad::comparison::compare_generic(&second, &first, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
    let LocalEvidence::Local { root, report, .. } = second.local.as_deref().unwrap() else {
        panic!()
    };
    let mut proofs = Vec::new();
    pair_witnesses(root, &mut proofs);
    assert!(proofs.iter().any(|p| p.cancellations == 2));
    assert!(report.second_cancellation_pairs_omitted > 0);
    // Without the upper bound, the two positive graphs really intersect at
    // x=3+2*sqrt(2). The second cancellation must fall back on that region.
    compare_positive_ray(&problem(
        &["x", "z"],
        &["x>0", "z>0"],
        &["z^2-x", "z^2-2*z-1"],
    ));
}

#[test]
fn second_pair_replay_rejects_changed_count_sign_and_missing_guards() {
    let p = problem(&["x", "z"], &["x>1", "x<4", "z>0"], &["z^2-x", "z^2-2*z-1"]);
    let original = local(&p);
    fn alter(node: &mut LocalNode, modification: usize) -> bool {
        if let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        {
            if let Some(ray) = &mut projection.positive_ray {
                if let Some(proof) = ray.separations.iter_mut().find(|p| p.cancellations == 2) {
                    match modification {
                        0 => proof.cancellations = 1,
                        1 => proof.cancellations = 3,
                        2 => proof.coefficient_sign *= -1,
                        3 => projection.guards.clear(),
                        _ => unreachable!(),
                    }
                    return true;
                }
            }
            return sectors
                .iter_mut()
                .any(|sector| alter(&mut sector.child, modification));
        }
        false
    }
    for modification in 0..4 {
        let mut changed = original.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(alter(root, modification));
        assert!(verify_local(&changed).is_err());
    }
    let mut changed = original;
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.positive_pair_cancellations = 1;
    assert!(verify_local(&changed).is_err());
}

#[test]
fn pair_cancellation_config_roundtrips_and_survives_homogeneous_reduction() {
    let mut p = problem(
        &["r", "x", "z"],
        &["r>0", "x>r", "x<4*r", "z>0"],
        &["z^2-r*x", "z^2-2*r*z-r^2"],
    );
    p.solver.engine = "local".into();
    p.solver.homogeneous_block = vec!["r".into(), "x".into(), "z".into()];
    p.solver.homogeneous_anchor = Some("r".into());
    let value = serde_json::to_value(&p).unwrap();
    assert!(value["solver"].get("local_pair_cancellations").is_none());
    let restored: Problem = serde_json::from_value(value).unwrap();
    assert_eq!(restored.solver.local_pair_cancellations, 2);
    for count in [1, 2] {
        p.solver.local_pair_cancellations = count;
        let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
        let result = symgcad::solver::solve(&restored).unwrap();
        assert_eq!(result.status, Status::CompleteGeneric);
        let reduced = &result.homogeneous.as_ref().unwrap().reduced;
        assert_eq!(reduced.problem.solver.local_pair_cancellations, count);
        let LocalEvidence::Local { options, .. } = reduced.local.as_deref().unwrap() else {
            panic!()
        };
        assert_eq!(options.positive_pair_cancellations, count);
        assert!(symgcad::verify::verify(&result).unwrap().verified);
    }
    for count in [0, 3] {
        p.solver.local_pair_cancellations = count;
        assert!(p.validate().is_err());
    }
}

#[test]
fn historical_one_cancellation_evidence_replays_without_new_count_fields() {
    let original = solve_local(
        &problem(&["x", "z"], &["x>0", "z>0"], &["z^2-x", "z^2-z-x-1"]),
        &LocalOptions {
            positive_pair_cancellations: 1,
            ..LocalOptions::default()
        },
    )
    .unwrap();
    let mut serialized = serde_json::to_value(&original).unwrap();
    fn remove_new_fields(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) => {
                fields.remove("positive_pair_cancellations");
                fields.remove("cancellations");
                for child in fields.values_mut() {
                    remove_new_fields(child);
                }
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(remove_new_fields),
            _ => {}
        }
    }
    remove_new_fields(&mut serialized);
    let restored: local_projection::LocalSolveResult = serde_json::from_value(serialized).unwrap();
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert_eq!(options.positive_pair_cancellations, 1);
    assert!(verify_local(&restored).unwrap().verified);
}

#[test]
fn sibling_discovery_batches_new_cuts_and_matches_unbatched_coverage() {
    let p = problem(&["x", "y"], &[], &["x", "y^2+x", "y^2-x", "y-1", "y-2"]);
    let batched = local_solution(&p, &LocalOptions::default());
    let unbatched = local_solution(
        &p,
        &LocalOptions {
            batch_sibling_discovery: false,
            ..LocalOptions::default()
        },
    );
    let report = |result: &symgcad::output::SolveResult| {
        let LocalEvidence::Local { report, .. } = result.local.as_deref().unwrap() else {
            panic!()
        };
        report.clone()
    };
    let batch = report(&batched);
    let plain = report(&unbatched);
    assert!(batch.discovery_siblings > 0);
    assert!(batch.multi_cut_refinements > 0);
    assert!(
        batch.refinements < plain.refinements,
        "{batch:?} vs {plain:?}"
    );
    assert_eq!(plain.discovery_siblings, 0);
    let comparison =
        symgcad::comparison::compare_generic(&batched, &unbatched, &symgcad::Limits::default())
            .unwrap();
    assert_eq!(comparison.equivalent, Some(true));
}

#[test]
fn sibling_batching_propagates_earlier_axis_guards_and_never_accepts_partial_stacks() {
    let p = problem(
        &["a", "x", "y"],
        &[],
        &["x", "y^2+a*x", "y^2-a*x", "y-1", "y-2"],
    );
    let batched = local_solution(&p, &LocalOptions::default());
    let unbatched = local_solution(
        &p,
        &LocalOptions {
            batch_sibling_discovery: false,
            ..LocalOptions::default()
        },
    );
    assert!(batched.cells.iter().all(|c| c.axes[0].sample != "0"));
    assert_eq!(
        symgcad::comparison::compare_generic(&batched, &unbatched, &symgcad::Limits::default(),)
            .unwrap()
            .equivalent,
        Some(true),
    );
    for options in [
        LocalOptions {
            max_refinements: 0,
            ..Default::default()
        },
        LocalOptions {
            max_nodes: 3,
            ..Default::default()
        },
    ] {
        let interrupted = solve_local(&p, &options).unwrap();
        assert_eq!(interrupted.solution.status, Status::Incomplete);
        assert!(interrupted.solution.cells.is_empty());
        assert!(matches!(
            interrupted.evidence,
            LocalEvidence::Unfinished { .. }
        ));
        assert!(verify_local(&interrupted).is_err());
    }
}

#[test]
fn old_local_evidence_without_batching_option_still_replays() {
    let original = local(&problem(&["x", "y"], &[], &["y^2+x"]));
    let mut value = serde_json::to_value(&original).unwrap();
    value["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("batch_sibling_discovery");
    let restored: local_projection::LocalSolveResult = serde_json::from_value(value).unwrap();
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert!(!options.batch_sibling_discovery);
    assert!(verify_local(&restored).unwrap().verified);
}

#[test]
fn sibling_batch_config_roundtrips_and_survives_homogeneous_reduction() {
    let mut p = problem(&["r", "x", "y"], &["r>0", "x>0", "y>0"], &["y^2-r*x"]);
    p.solver.engine = "local".into();
    p.solver.homogeneous_block = vec!["r".into(), "x".into(), "y".into()];
    p.solver.homogeneous_anchor = Some("r".into());
    let default_json = serde_json::to_value(&p).unwrap();
    assert!(default_json["solver"].get("local_batch_siblings").is_none());
    assert!(default_json["solver"].get("local_reuse_subtrees").is_none());
    let restored: Problem = serde_json::from_value(default_json).unwrap();
    assert!(restored.solver.local_batch_siblings);
    p.solver.local_batch_siblings = false;
    p.solver.local_reuse_subtrees = false;
    let config = toml::to_string(&p).unwrap();
    let restored = Problem::from_toml(&config).unwrap();
    assert!(!restored.solver.local_batch_siblings);
    let result = symgcad::solver::solve(&restored).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert!(!reduced.problem.solver.local_batch_siblings);
    let LocalEvidence::Local {
        options, report, ..
    } = reduced.local.as_deref().unwrap()
    else {
        panic!()
    };
    assert!(!options.batch_sibling_discovery);
    assert!(!options.reuse_completed_subtrees);
    assert_eq!(report.discovery_siblings, 0);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
}

#[test]
fn completed_subtree_reuse_preserves_conditional_proofs_and_reduces_restarts() {
    let p = problem(
        &["x", "y", "z", "w"],
        &[],
        &["x", "y", "z-x", "w^2-z", "w-y", "w-2"],
    );
    let cached_options = LocalOptions {
        reuse_learned_cuts: false,
        ..Default::default()
    };
    let cached = solve_local(&p, &cached_options).unwrap();
    let plain_options = LocalOptions {
        reuse_completed_subtrees: false,
        reuse_learned_cuts: false,
        ..Default::default()
    };
    let plain = solve_local(&p, &plain_options).unwrap();
    assert!(verify_local(&cached).unwrap().verified);
    assert!(verify_local(&plain).unwrap().verified);
    let cached_report = local_report(&cached);
    let plain_report = local_report(&plain);
    assert!(cached_report.subtree_cache_hits > 0, "{cached_report:?}");
    assert!(
        cached_report.refinements < plain_report.refinements,
        "{cached_report:?} vs {plain_report:?}"
    );
    assert!(cached_report.subtree_cache_peak_bytes <= 32 * 1024 * 1024);
    assert!(cached_report.subtree_cache_peak_nodes <= 32768);
    assert_eq!(plain_report.subtree_cache_hits, 0);
    let budget = cached_report.refinements;
    let cached_limited = solve_local(
        &p,
        &LocalOptions {
            max_refinements: budget,
            ..cached_options.clone()
        },
    )
    .unwrap();
    assert_eq!(cached_limited.solution.status, Status::CompleteGeneric);
    assert!(verify_local(&cached_limited).unwrap().verified);
    let plain_limited = solve_local(
        &p,
        &LocalOptions {
            max_refinements: budget,
            ..plain_options
        },
    )
    .unwrap();
    assert_eq!(plain_limited.solution.status, Status::Incomplete);
    assert!(verify_local(&plain_limited).is_err());
    let cached_solution = local_solution(&p, &cached_options);
    let plain_solution = local_solution(
        &p,
        &LocalOptions {
            reuse_completed_subtrees: false,
            reuse_learned_cuts: false,
            ..Default::default()
        },
    );
    assert_eq!(
        symgcad::comparison::compare_generic(
            &cached_solution,
            &plain_solution,
            &symgcad::Limits::default()
        )
        .unwrap()
        .equivalent,
        Some(true)
    );
}

#[test]
fn reused_proofs_remain_exhaustive_and_reject_tampering_or_exhausted_budgets() {
    let mut p = problem(&["x", "y", "z"], &[], &["z^2-x", "z-y", "x", "y"]);
    let original = solve_local(&p, &LocalOptions::default()).unwrap();
    assert!(verify_local(&original).unwrap().verified);
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
    fn drop_obligations(node: &mut LocalNode) -> bool {
        if let LocalNode::Stack {
            required_below,
            sectors,
            ..
        } = node
        {
            if !required_below.is_empty() {
                required_below.clear();
                return true;
            }
            return sectors.iter_mut().any(|s| drop_obligations(&mut s.child));
        }
        false
    }
    let mut changed = original.clone();
    let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
        panic!()
    };
    assert!(drop_obligations(root));
    assert!(verify_local(&changed).is_err());
    p.limits.max_cells = Some(1);
    let limited = solve_local(&p, &LocalOptions::default()).unwrap();
    assert_eq!(limited.solution.status, Status::Incomplete);
    assert!(limited.solution.cells.is_empty());
    assert!(verify_local(&limited).is_err());
    let mut old = serde_json::to_value(&original).unwrap();
    old["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("reuse_completed_subtrees");
    let restored: local_projection::LocalSolveResult = serde_json::from_value(old).unwrap();
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert!(!options.reuse_completed_subtrees);
    assert!(verify_local(&restored).unwrap().verified);
}

#[test]
fn configured_local_caps_keep_defaults_and_report_exhaustion_without_completion() {
    let mut p = problem(&["x", "y"], &[], &["y^2+x"]);
    p.solver.engine = "local".into();
    let serialized = serde_json::to_value(&p).unwrap();
    assert!(serialized["solver"].get("local_max_refinements").is_none());
    assert!(serialized["solver"].get("local_max_nodes").is_none());
    let normal = symgcad::solver::solve(&p).unwrap();
    let LocalEvidence::Local { options, .. } = normal.local.as_deref().unwrap() else {
        panic!()
    };
    assert_eq!(options.max_refinements, 10_000);
    assert_eq!(options.max_nodes, 100_000);
    assert!(symgcad::verify::verify(&normal).unwrap().verified);
    for (refinements, nodes) in [(Some(0), None), (None, Some(1))] {
        p.solver.local_max_refinements = refinements;
        p.solver.local_max_nodes = nodes;
        let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
        let result = symgcad::solver::solve(&restored).unwrap();
        assert_eq!(result.status, Status::Incomplete);
        assert!(result.cells.is_empty());
        assert!(matches!(
            result.local.as_deref(),
            Some(LocalEvidence::Unfinished { .. })
        ));
        assert!(symgcad::verify::verify(&result).is_err());
    }
}

#[test]
fn configured_local_caps_survive_homogeneous_reduction_and_toml() {
    let mut p = problem(&["r", "x", "y"], &["r>0"], &["y^2+r*x"]);
    p.solver.engine = "local".into();
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    p.solver.local_max_refinements = Some(123);
    p.solver.local_max_nodes = Some(456);
    let restored = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    let result = symgcad::solver::solve(&restored).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    assert_eq!(reduced.problem.solver.local_max_refinements, Some(123));
    assert_eq!(reduced.problem.solver.local_max_nodes, Some(456));
    let LocalEvidence::Local { options, .. } = reduced.local.as_deref().unwrap() else {
        panic!()
    };
    assert_eq!(options.max_refinements, 123);
    assert_eq!(options.max_nodes, 456);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    for (refinements, nodes) in [(Some(0), None), (None, Some(1))] {
        p.solver.local_max_refinements = refinements;
        p.solver.local_max_nodes = nodes;
        let result = symgcad::solver::solve(&p).unwrap();
        assert_eq!(result.status, Status::Incomplete);
        assert!(result.cells.is_empty());
        assert!(symgcad::verify::verify(&result).is_err());
    }
}

#[test]
fn descartes_one_tracks_the_positive_root_across_negative_root_collisions() {
    use symgcad::output::RootIndexDomain;
    for polynomial in ["z^3+a*z^2-z-1", "z^5+a*z^4-z-1"] {
        let p = problem(&["a", "z"], &["a>0", "z>0"], &[polynomial]);
        let result = local(&p);
        assert_eq!(result.solution.cells.len(), 2);
        assert!(local_report(&result).descartes_one_discriminants_omitted > 0);
        assert!(
            result
                .solution
                .cells
                .iter()
                .all(|cell| cell.axes[0].sample == "1")
        );
        let root = result.solution.cells[0].axes[1].upper.as_ref().unwrap();
        assert_eq!(root.index_domain, RootIndexDomain::Positive);
        assert_eq!(root.index, 0);
        assert_eq!(root.lower, "1");
        assert_eq!(root.upper, "1");
    }
    let p = problem(&["a", "z"], &["a>0", "z>0"], &["z^3+a*z^2-z-1"]);
    let positive = local_solution(&p, &LocalOptions::default());
    let ordinary = local_solution(
        &p,
        &LocalOptions {
            descartes_one: false,
            ..Default::default()
        },
    );
    assert!(ordinary.cells.len() > positive.cells.len());
    assert_eq!(
        symgcad::comparison::compare_generic(&positive, &ordinary, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
}

#[test]
fn descartes_one_refines_zero_coefficients_and_rejects_wrong_scope_or_index() {
    use symgcad::output::RootIndexDomain;
    let p = problem(&["a", "z"], &["z>0"], &["z^3+a*z^2-z-1"]);
    let result = local(&p);
    assert!(
        result
            .solution
            .cells
            .iter()
            .all(|cell| cell.axes[0].sample != "0")
    );
    fn alter(node: &mut LocalNode, change_index: bool) -> bool {
        if let LocalNode::Stack { roots, sectors, .. } = node {
            if let Some(root) = roots
                .iter_mut()
                .find(|r| r.index_domain == RootIndexDomain::Positive)
            {
                if change_index {
                    root.index = 1;
                } else {
                    root.index_domain = RootIndexDomain::Real;
                }
                return true;
            }
            return sectors
                .iter_mut()
                .any(|s| alter(&mut s.child, change_index));
        }
        false
    }
    for change_index in [false, true] {
        let mut changed = result.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(alter(root, change_index));
        assert!(verify_local(&changed).is_err());
    }
    let mut changed = result.clone();
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.descartes_one = false;
    assert!(verify_local(&changed).is_err());
}

#[test]
fn positive_root_scope_survives_positive_homogeneous_scaling() {
    use symgcad::output::RootIndexDomain;
    let mut p = problem(
        &["a", "r", "z"],
        &["a>0", "r>0", "z>0"],
        &["z^3+a*r*z^2-r^2*z-r^3"],
    );
    p.solver.engine = "local".into();
    p.solver.homogeneous_block = vec!["r".into(), "z".into()];
    p.solver.homogeneous_anchor = Some("r".into());
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let reduced = &result.homogeneous.as_ref().unwrap().reduced;
    let root = result.cells[0].axes[2].upper.as_ref().unwrap();
    let original = reduced.cells[0].axes[1].upper.as_ref().unwrap();
    assert_eq!(root.index_domain, RootIndexDomain::Positive);
    assert_eq!(root.index_domain, original.index_domain);
    assert_eq!(root.index, original.index);
}

#[test]
fn old_evidence_defaults_to_full_real_indices_and_original_discriminants() {
    let p = problem(&["a", "z"], &["a>0", "z>0"], &["z^3+a*z^2-z-1"]);
    let result = solve_local(
        &p,
        &LocalOptions {
            descartes_one: false,
            ..Default::default()
        },
    )
    .unwrap();
    let mut value = serde_json::to_value(&result).unwrap();
    value["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("descartes_one");
    assert!(
        !serde_json::to_string(&value)
            .unwrap()
            .contains("index_domain")
    );
    let restored: local_projection::LocalSolveResult = serde_json::from_value(value).unwrap();
    assert!(verify_local(&restored).unwrap().verified);
    let LocalEvidence::Local { options, .. } = &restored.evidence else {
        panic!()
    };
    assert!(!options.descartes_one);
}

#[test]
fn global_trace_rejects_positive_scope_and_merge_does_not_confuse_ordinals() {
    use symgcad::output::{Axis, Cell, LiftTrace, RootBound, RootIndexDomain};
    let mut global = symgcad::solver::solve(&problem(&["x"], &[], &["x-1"])).unwrap();
    let Some(LiftTrace::Stack { roots, .. }) = &mut global.trace else {
        panic!()
    };
    roots[0].index_domain = RootIndexDomain::Positive;
    assert!(symgcad::verify::verify(&global).is_err());
    let left = RootBound {
        polynomial: 0,
        index: 0,
        index_domain: RootIndexDomain::Real,
        lower: "-1".into(),
        upper: "-1".into(),
    };
    let right = RootBound {
        polynomial: 0,
        index: 0,
        index_domain: RootIndexDomain::Positive,
        lower: "1".into(),
        upper: "1".into(),
    };
    let cells = vec![
        Cell {
            axes: vec![Axis {
                variable: "x".into(),
                sample: "-2".into(),
                lower: None,
                upper: Some(left),
            }],
            signs: vec![1],
        },
        Cell {
            axes: vec![Axis {
                variable: "x".into(),
                sample: "2".into(),
                lower: Some(right),
                upper: None,
            }],
            signs: vec![1],
        },
    ];
    assert_eq!(symgcad::solver::merge(&cells).len(), 2);
}

#[test]
fn base_point_root_union_handles_duplicate_shared_and_repeated_roots() {
    let p = problem(
        &["x"],
        &[],
        &["(x-1)^2*(x+1)", "(x^2-1)^3", "x^2-2", "2*(x^2-2)", "0"],
    );
    let fast = local(&p);
    assert_eq!(fast.solution.cells.len(), 5);
    assert!(fast.solution.cells.iter().all(|cell| cell.signs[4] == 0));
    let LocalEvidence::Local {
        root:
            LocalNode::Stack {
                projection,
                roots,
                sectors,
                ..
            },
        report,
        ..
    } = &fast.evidence
    else {
        panic!()
    };
    assert!(projection.base_point);
    assert!(projection.root_bearing.is_empty());
    assert!(projection.guards.is_empty());
    assert!(projection.positive_ray.is_none());
    assert_eq!(roots.len(), 4);
    assert_eq!(sectors.len(), 5);
    assert!(
        roots
            .iter()
            .all(|r| r.index_domain == symgcad::output::RootIndexDomain::Real)
    );
    assert_eq!(report.cache.resultant_misses, 0);
    assert_eq!(report.cache.discriminant_misses, 0);
    assert!(report.base_point_candidate_pairs_skipped > 0);

    let ordinary = local_solution(
        &p,
        &LocalOptions {
            base_point_projection: false,
            ..Default::default()
        },
    );
    let fast = local_solution(&p, &LocalOptions::default());
    let global = symgcad::solver::solve(&p).unwrap();
    for other in [&ordinary, &global] {
        assert_eq!(
            symgcad::comparison::compare_generic(&fast, other, &symgcad::Limits::default())
                .unwrap()
                .equivalent,
            Some(true)
        );
    }
}

#[test]
fn base_point_preserves_strict_boundaries_and_positive_ray_domain() {
    for p in [
        problem(&["x"], &["x>0"], &["(x+3)*(x+1)*(x-2)"]),
        problem(&["x"], &["x<0"], &["x^2-2"]),
        problem(&["x"], &["0>0"], &["x"]),
        problem(&["x"], &["x>0", "-x>0"], &["0"]),
        problem(&["x"], &["x^2>0"], &["x"]),
    ] {
        let fast = local_solution(&p, &LocalOptions::default());
        let ordinary = local_solution(
            &p,
            &LocalOptions {
                base_point_projection: false,
                ..Default::default()
            },
        );
        assert_eq!(
            symgcad::comparison::compare_generic(&fast, &ordinary, &symgcad::Limits::default())
                .unwrap()
                .equivalent,
            Some(true)
        );
    }
    let positive = local(&problem(&["x"], &["x>0"], &["x^2-2"]));
    assert_eq!(positive.solution.cells.len(), 2);
    assert!(positive.solution.cells.iter().all(|cell| {
        symgcad::algebra::rational(&cell.axes[0].sample).unwrap()
            > symgcad::algebra::rational("0").unwrap()
    }));
    let LocalEvidence::Local {
        root: LocalNode::Stack { roots, sectors, .. },
        ..
    } = &positive.evidence
    else {
        panic!()
    };
    assert_eq!(roots.len(), 3); // -sqrt(2), 0, sqrt(2), with every real sector present.
    assert!(
        sectors
            .iter()
            .take(2)
            .all(|sector| matches!(sector.child, LocalNode::Rejected { .. }))
    );
}

#[test]
fn base_point_verifier_rejects_missing_roots_sectors_and_false_scope() {
    let original = local(&problem(&["x"], &[], &["x^2-2", "x-1"]));
    for modification in 0..5 {
        let mut changed = original.clone();
        let LocalEvidence::Local {
            root:
                LocalNode::Stack {
                    projection,
                    roots,
                    sectors,
                    ..
                },
            options,
            ..
        } = &mut changed.evidence
        else {
            panic!()
        };
        match modification {
            0 => {
                roots.pop();
            }
            1 => {
                sectors.pop();
            }
            2 => {
                projection.root_bearing.push(0);
            }
            3 => {
                options.base_point_projection = false;
            }
            4 => {
                projection.base_point = false;
            }
            _ => unreachable!(),
        }
        assert!(verify_local(&changed).is_err(), "tamper {modification}");
    }
    let mut deeper = local(&problem(&["x", "y"], &[], &["y^2-x"]));
    fn wrong_level(node: &mut LocalNode, depth: usize) -> bool {
        if let LocalNode::Stack {
            projection,
            sectors,
            ..
        } = node
        {
            if depth > 0 {
                projection.base_point = true;
                projection.positive_ray = None;
                projection.root_bearing.clear();
                projection.guards.clear();
                projection.removals.clear();
                return true;
            }
            return sectors
                .iter_mut()
                .any(|sector| wrong_level(&mut sector.child, depth + 1));
        }
        false
    }
    let LocalEvidence::Local { root, .. } = &mut deeper.evidence else {
        panic!()
    };
    assert!(wrong_level(root, 0));
    assert!(verify_local(&deeper).is_err());
}

#[test]
fn base_point_verifier_rejects_a_cut_with_later_parameter_dependence() {
    let mut changed = local(&problem(&["x", "y"], &[], &["y^2-x"]));
    let algebra = symgcad::algebra::Algebra::new(&changed.solution.order).unwrap();
    let later = changed
        .solution
        .polynomials
        .iter()
        .position(|text| algebra.parse(text).unwrap().degree(1) > 0)
        .unwrap();
    let LocalEvidence::Local {
        root: LocalNode::Stack { cuts, .. },
        ..
    } = &mut changed.evidence
    else {
        panic!()
    };
    cuts.push(later);
    cuts.sort_unstable();
    cuts.dedup();
    assert!(verify_local(&changed).is_err());
}

#[test]
fn base_point_historical_evidence_and_solver_ablation_remain_explicit() {
    let mut p = problem(&["x", "y"], &["x>0", "y>0"], &["y^2-x"]);
    p.solver.engine = "local".into();
    p.solver.local_base_point_projection = false;
    let ordinary = symgcad::solver::solve(&p).unwrap();
    let LocalEvidence::Local { options, .. } = ordinary.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(!options.base_point_projection);
    assert!(symgcad::verify::verify(&ordinary).unwrap().verified);
    let mut old = serde_json::to_value(&ordinary).unwrap();
    fn drop_new_fields(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("base_point");
                map.remove("base_point_projection");
                map.remove("local_base_point_projection");
                for value in map.values_mut() {
                    drop_new_fields(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    drop_new_fields(value);
                }
            }
            _ => {}
        }
    }
    drop_new_fields(&mut old);
    let restored: symgcad::output::SolveResult = serde_json::from_value(old).unwrap();
    let LocalEvidence::Local { options, .. } = restored.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(!options.base_point_projection);
    assert!(symgcad::verify::verify(&restored).unwrap().verified);
    p.solver.local_base_point_projection = true;
    let text = toml::to_string(&p).unwrap();
    assert!(!text.contains("local_base_point_projection"));
    assert!(
        Problem::from_toml(&text)
            .unwrap()
            .solver
            .local_base_point_projection
    );
    let fast = symgcad::solver::solve(&p).unwrap();
    let LocalEvidence::Local { options, .. } = fast.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(options.base_point_projection);
    assert!(symgcad::verify::verify(&fast).unwrap().verified);
}

fn adjacent_options(positive: bool) -> LocalOptions {
    LocalOptions {
        adjacent_pairs: true,
        positive_ray: positive,
        fallback_to_global: false,
        ..Default::default()
    }
}

#[test]
fn adjacent_pairs_match_full_pairs_and_global_cad_across_crossings() {
    let cases = [
        (
            problem(&["a", "z"], &[], &["z^2+a*z-4", "z+1", "z-1", "z-a"]),
            false,
        ),
        (
            problem(&["a", "z"], &["a>0", "z>0"], &["z^2-a", "z^2-z-a-1", "z-3"]),
            true,
        ),
        (
            problem(
                &["a", "z"],
                &["a>0", "z>0"],
                &["z^3+a*z^2+z-1", "z-2", "z-3"],
            ),
            true,
        ),
    ];
    let mut omitted = 0;
    for (p, positive) in cases {
        let options = adjacent_options(positive);
        let adjacent = local_solution(&p, &options);
        let full = local_solution(
            &p,
            &LocalOptions {
                adjacent_pairs: false,
                ..options
            },
        );
        let global = symgcad::solver::solve(&p).unwrap();
        let LocalEvidence::Local { report, .. } = adjacent.local.as_deref().unwrap() else {
            panic!()
        };
        omitted += report.nonadjacent_pairs_omitted;
        for other in [&full, &global] {
            let comparison =
                symgcad::comparison::compare_generic(&adjacent, other, &symgcad::Limits::default())
                    .unwrap();
            assert_eq!(comparison.equivalent, Some(true));
        }
    }
    assert!(omitted > 0);
}

#[test]
fn adjacent_pairs_replay_sibling_refinement_and_cache_ablations() {
    let p = problem(
        &["x", "y", "z"],
        &["x>0", "y>0", "z>0"],
        &["z^2-x", "z-y", "z-2"],
    );
    let cached = local_solution(&p, &adjacent_options(true));
    let uncached = local_solution(
        &p,
        &LocalOptions {
            cache_enabled: false,
            reuse_completed_subtrees: false,
            reuse_learned_cuts: false,
            batch_sibling_discovery: false,
            ..adjacent_options(true)
        },
    );
    assert_eq!(
        symgcad::comparison::compare_generic(&cached, &uncached, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
    let limited = solve_local(
        &p,
        &LocalOptions {
            max_nodes: 1,
            ..adjacent_options(true)
        },
    )
    .unwrap();
    assert_eq!(limited.solution.status, Status::Incomplete);
    assert!(verify_local(&limited).is_err());
}

fn change_adjacent_witness(node: &mut LocalNode, modification: usize) -> bool {
    let LocalNode::Stack {
        projection,
        sectors,
        ..
    } = node
    else {
        return false;
    };
    if let Some(witness) = &mut projection.adjacent_pairs {
        if witness.selected_pairs.is_empty() || witness.branch_order.len() < 2 {
            return sectors
                .iter_mut()
                .any(|s| change_adjacent_witness(&mut s.child, modification));
        }
        match modification {
            0 => witness.selected_pairs.clear(),
            1 => witness.branch_order.swap(0, 1),
            2 => witness.distinct_root_counts[0].1 += 1,
            3 => witness.coincident_fallback = !witness.coincident_fallback,
            4 => witness.positive = !witness.positive,
            5 => projection.adjacent_pairs = None,
            _ => unreachable!(),
        }
        true
    } else {
        sectors
            .iter_mut()
            .any(|s| change_adjacent_witness(&mut s.child, modification))
    }
}

#[test]
fn adjacent_pair_verifier_rejects_omitted_pairs_changed_order_and_scope() {
    let p = problem(&["a", "z"], &["a>0", "z>0"], &["z^2-a", "z-2", "z-3"]);
    let result = solve_local(&p, &adjacent_options(true)).unwrap();
    assert!(verify_local(&result).unwrap().verified);
    for modification in 0..6 {
        let mut changed = result.clone();
        let LocalEvidence::Local { root, .. } = &mut changed.evidence else {
            panic!()
        };
        assert!(change_adjacent_witness(root, modification));
        assert!(verify_local(&changed).is_err(), "tamper {modification}");
    }
    let mut changed = result.clone();
    let LocalEvidence::Local { options, .. } = &mut changed.evidence else {
        panic!()
    };
    options.adjacent_pairs = false;
    assert!(verify_local(&changed).is_err());
}

#[test]
fn adjacent_pairs_are_opt_in_and_historical_proofs_replay_full_pairs() {
    let mut p = problem(&["a", "z"], &[], &["z-a", "z-1", "z+1"]);
    p.solver.engine = "local".into();
    assert!(!p.solver.local_adjacent_pairs);
    assert!(
        !toml::to_string(&p)
            .unwrap()
            .contains("local_adjacent_pairs")
    );
    let ordinary = symgcad::solver::solve(&p).unwrap();
    let mut old = serde_json::to_value(&ordinary).unwrap();
    fn erase(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("adjacent_pairs");
                map.remove("local_adjacent_pairs");
                for value in map.values_mut() {
                    erase(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    erase(value);
                }
            }
            _ => {}
        }
    }
    erase(&mut old);
    let restored: symgcad::output::SolveResult = serde_json::from_value(old).unwrap();
    assert!(symgcad::verify::verify(&restored).unwrap().verified);
    let LocalEvidence::Local { options, .. } = restored.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(!options.adjacent_pairs);
    p.solver.local_adjacent_pairs = true;
    let parsed = Problem::from_toml(&toml::to_string(&p).unwrap()).unwrap();
    assert!(parsed.solver.local_adjacent_pairs);
    let adjacent = symgcad::solver::solve(&parsed).unwrap();
    assert!(symgcad::verify::verify(&adjacent).unwrap().verified);
    let LocalEvidence::Local { options, .. } = adjacent.local.as_deref().unwrap() else {
        panic!()
    };
    assert!(options.adjacent_pairs);
}

#[test]
fn adjacent_pair_degree_and_endpoint_obstructions_refine_before_projection() {
    let p = problem(&["a", "z"], &["z>0"], &["a*z+1", "z-a", "z-1", "z-2"]);
    let fast = local_solution(&p, &adjacent_options(true));
    let global = symgcad::solver::solve(&p).unwrap();
    assert_eq!(
        symgcad::comparison::compare_generic(&fast, &global, &symgcad::Limits::default())
            .unwrap()
            .equivalent,
        Some(true)
    );
    let mut changed = solve_local(&p, &adjacent_options(true)).unwrap();
    let LocalEvidence::Local {
        root: LocalNode::Stack { sectors, .. },
        ..
    } = &mut changed.evidence
    else {
        panic!()
    };
    sectors.pop();
    assert!(verify_local(&changed).is_err());
}
