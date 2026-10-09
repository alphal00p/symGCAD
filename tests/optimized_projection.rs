use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use symgcad::{
    algebra::{Algebra, canonical},
    optimized_projection::{
        DomainPruningOptions, OptimizedProjectionOptions, OptimizedProjectionReport,
        ProjectionCache, project_optimized, project_ordered_certified, project_ordered_optimized,
        replay_certified_projection, replay_certified_projection_with_inputs,
        replay_certified_projection_with_operations,
    },
    projection::project,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "symgcad-projection-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn algebra() -> Algebra {
    Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap()
}

#[test]
fn optimized_factor_sets_equal_baseline_with_independent_factors() {
    let a = algebra();
    for expressions in [
        vec!["(z^2-x)^2", "y-z", "x-2"],
        vec!["x*y*z+z+1", "z-y", "x+1", "y-2"],
        vec!["z^3+y*z+x", "z^2+x*y", "y^2-x"],
    ] {
        let input = expressions
            .iter()
            .map(|s| a.parse(s).unwrap())
            .collect::<Vec<_>>();
        let baseline = project(&a, &input).unwrap();
        let (optimized, report) =
            project_optimized(&a, &input, &OptimizedProjectionOptions::default()).unwrap();
        assert_eq!(optimized.levels, baseline.levels);
        assert!(report.cache.skipped_degree_zero_pairs > 0);
        assert_eq!(report.checkpoints.len(), 3);
    }
}

#[test]
fn completed_prefix_and_overlapping_algebra_are_reused() {
    let a = algebra();
    let mut cache = ProjectionCache::new(&a, &OptimizedProjectionOptions::default());
    let mut basis = cache
        .initial_basis(&[a.parse("z^2+x").unwrap(), a.parse("z+y").unwrap()])
        .unwrap();
    let first = cache.project_one(&basis, 2).unwrap();
    let misses = cache.report.cache.resultant_misses;
    assert_eq!(cache.project_one(&basis, 2).unwrap(), first);
    assert_eq!(cache.report.cache.resultant_misses, misses);
    assert_eq!(cache.report.cache.prefix_memory_hits, 1);
    basis.push(canonical(&a.parse("z+x").unwrap()));
    cache.project_one(&basis, 2).unwrap();
    assert!(cache.report.cache.resultant_hits >= 1);
    assert!(cache.report.cache.discriminant_hits >= 1);
    assert!(cache.report.cache.factor_hits >= 1);
}

#[test]
fn completed_steps_survive_new_cache_instances() {
    let directory = Directory::new();
    let a = algebra();
    let input = vec![a.parse("z^2-x").unwrap(), a.parse("y-z").unwrap()];
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        memory_cache_bytes: 0,
        ..Default::default()
    };
    let (first, first_report) = project_optimized(&a, &input, &options).unwrap();
    let (second, second_report) = project_optimized(&a, &input, &options).unwrap();
    assert_eq!(first.levels, second.levels);
    assert_eq!(first_report.cache.written_checkpoints, 3);
    assert_eq!(second_report.cache.prefix_disk_hits, 3);
    assert_eq!(second_report.cache.factor_misses, 0);
    assert_eq!(second_report.cache.resultant_misses, 0);
    assert_eq!(second_report.cache.retained_bytes_estimate, 0);
    assert!(
        fs::read_dir(&directory.0)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
            .all(|path| path.extension().unwrap() == "json")
    );
}

#[test]
fn damaged_input_keys_are_recomputed_and_replaced() {
    let directory = Directory::new();
    let a = algebra();
    let input = vec![a.parse("z^2-x").unwrap(), a.parse("y-z").unwrap()];
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        memory_cache_bytes: 0,
        ..Default::default()
    };
    let (first, _) = project_optimized(&a, &input, &options).unwrap();
    for entry in fs::read_dir(&directory.0).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let mut saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        saved["input_hash"] = "damaged".into();
        fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    }
    let (second, report) = project_optimized(&a, &input, &options).unwrap();
    assert_eq!(first.levels, second.levels);
    assert_eq!(report.cache.invalid_checkpoints, 3);
    assert_eq!(report.cache.prefix_disk_hits, 0);
    assert_eq!(report.cache.written_checkpoints, 3);
}

#[test]
fn output_checksums_and_variable_maps_are_validated() {
    let directory = Directory::new();
    let a = algebra();
    let input = vec![a.parse("z^2-x").unwrap(), a.parse("y-z").unwrap()];
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        memory_cache_bytes: 0,
        ..Default::default()
    };
    let (first, initial) = project_optimized(&a, &input, &options).unwrap();
    for (i, checkpoint) in initial.checkpoints.iter().enumerate() {
        let path = directory.0.join(format!("{}.json", checkpoint.key));
        let mut saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if i == 0 {
            saved["variable_names"] = serde_json::json!(["x", "z", "y"]);
        } else {
            saved["output_hash"] = "damaged".into();
        }
        fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    }
    let (second, report) = project_optimized(&a, &input, &options).unwrap();
    assert_eq!(first.levels, second.levels);
    assert_eq!(report.cache.invalid_checkpoints, 3);
}

#[test]
fn memory_budget_and_domain_context_are_respected() {
    let directory = Directory::new();
    let a = algebra();
    let input = vec![a.parse("z^2-x").unwrap(), a.parse("y-z").unwrap()];
    let mut options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        memory_cache_bytes: 1800,
        ..Default::default()
    };
    let (first, report) = project_optimized(&a, &input, &options).unwrap();
    assert!(report.cache.peak_retained_bytes_estimate <= 1800);
    assert!(report.cache.evictions > 0);
    options.domain_fingerprint = "different-domain".into();
    let (second, report) = project_optimized(&a, &input, &options).unwrap();
    assert_eq!(first.levels, second.levels);
    assert_eq!(report.cache.prefix_disk_hits, 0);
    assert_eq!(report.cache.written_checkpoints, 3);
}

#[test]
fn order_trials_share_original_map_prefixes_and_return_remapped_levels() {
    let directory = Directory::new();
    let a = algebra();
    let expressions = ["z^2-x", "y-z", "x-2"];
    let input = expressions
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        ..Default::default()
    };
    project_ordered_optimized(&a, &input, &[0, 1, 2], &options).unwrap();
    let (result, report) = project_ordered_optimized(&a, &input, &[1, 0, 2], &options).unwrap();
    let reordered = Algebra::new(&["y".into(), "x".into(), "z".into()]).unwrap();
    let expected = project(
        &reordered,
        &expressions
            .iter()
            .map(|s| reordered.parse(s).unwrap())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(result.levels, expected.levels);
    assert_eq!(report.cache.prefix_disk_hits, 2);
    assert_eq!(report.cache.written_checkpoints, 1);
    let (cycled, _) = project_ordered_optimized(&a, &input, &[2, 0, 1], &options).unwrap();
    let reordered = Algebra::new(&["z".into(), "x".into(), "y".into()]).unwrap();
    let expected = project(
        &reordered,
        &expressions
            .iter()
            .map(|s| reordered.parse(s).unwrap())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(cycled.levels, expected.levels);
}

#[test]
fn matching_output_checksum_does_not_bypass_polynomial_validation() {
    let directory = Directory::new();
    let a = algebra();
    let input = vec![a.parse("z^2-x").unwrap(), a.parse("y-z").unwrap()];
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        memory_cache_bytes: 0,
        ..Default::default()
    };
    let (expected, initial) = project_optimized(&a, &input, &options).unwrap();
    for (i, checkpoint) in initial.checkpoints.iter().take(2).enumerate() {
        let path = directory.0.join(format!("{}.json", checkpoint.key));
        let mut saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        saved["output"] = serde_json::json!([if i == 0 { "undeclared_variable+1" } else { "z" }]);
        saved["output_hash"] = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&saved["output"]).unwrap())
        )
        .into();
        fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    }
    let (actual, report) = project_optimized(&a, &input, &options).unwrap();
    assert_eq!(actual.levels, expected.levels);
    assert_eq!(report.cache.invalid_checkpoints, 2);
    assert_eq!(report.cache.prefix_disk_hits, 1);
}

fn two_variables() -> Algebra {
    Algebra::new(&["x".into(), "y".into()]).unwrap()
}

#[test]
fn certified_pruning_drops_only_generated_nonzero_factors() {
    let a = two_variables();
    let constraints = ["x", "y", "y^2+x+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let baseline = project(&a, &constraints).unwrap();
    let (pruned, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(baseline.levels[0].len(), 2);
    assert_eq!(pruned.levels[0], vec![a.parse("x").unwrap()]);
    assert_eq!(pruned.levels[1], baseline.levels[1]);
    assert_eq!(report.removals.len(), 1);
    assert_eq!(report.removals[0].level, 0);
    assert_eq!(report.removals[0].eliminated_variable, 1);
    assert_eq!(
        a.parse(&report.removals[0].certificate.polynomial).unwrap(),
        a.parse("x+1").unwrap()
    );
    let replay =
        replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &report.removals).unwrap();
    assert_eq!(replay.levels, pruned.levels);
}

#[test]
fn original_constraint_and_split_factors_are_always_protected() {
    let a = two_variables();
    for as_split in [false, true] {
        let mut constraints = ["x", "y", "y^2+x+1"]
            .iter()
            .map(|s| a.parse(s).unwrap())
            .collect::<Vec<_>>();
        let mut split = vec![];
        if as_split {
            split.push(a.parse("x+1").unwrap());
        } else {
            constraints.push(a.parse("x+1").unwrap());
        }
        let all = constraints
            .iter()
            .chain(&split)
            .cloned()
            .collect::<Vec<_>>();
        let baseline = project(&a, &all).unwrap();
        let (pruned, report) = project_ordered_certified(
            &a,
            &constraints,
            &split,
            &[0, 1],
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert_eq!(pruned.levels, baseline.levels);
        assert!(report.removals.is_empty());
    }
}

#[test]
fn bounded_negative_certificates_prune_but_interior_zero_candidates_stay() {
    let a = two_variables();
    let constraints = ["x", "1-x", "y", "y^2+2-x"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (pruned, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    let removal = report
        .removals
        .iter()
        .find(|r| a.parse(&r.certificate.polynomial).unwrap() == a.parse("x-2").unwrap())
        .unwrap();
    assert_eq!(removal.certificate.sign, -1);
    assert_eq!(
        pruned.levels,
        replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &report.removals)
            .unwrap()
            .levels
    );
    let constraints = ["x", "y", "y^2+x-1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (pruned, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    assert!(report.removals.is_empty());
    assert!(pruned.levels[0].contains(&a.parse("x-1").unwrap()));
}

#[test]
fn proof_replay_rejects_forged_protected_duplicate_and_wrong_level_removals() {
    let a = two_variables();
    let constraints = ["x", "y", "y^2+x+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (_, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    let valid = report.removals;
    let mut changed = valid.clone();
    changed[0].certificate.polynomial = "x".into();
    assert!(replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &changed).is_err());
    let mut changed = valid.clone();
    changed[0].certificate.sign *= -1;
    assert!(replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &changed).is_err());
    let mut changed = valid.clone();
    changed.push(changed[0].clone());
    assert!(replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &changed).is_err());
    let mut changed = valid.clone();
    changed[0].level = 1;
    assert!(replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &changed).is_err());
    let mut changed = valid;
    changed[0].eliminated_variable = 0;
    assert!(replay_certified_projection(&a, &constraints, &[], &[0, 1], &[], &changed).is_err());
}

#[test]
fn sequential_removals_replay_and_survive_checkpoint_restarts() {
    let a = algebra();
    let constraints = ["x", "y", "z", "z^2+x+1", "y^2+x+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let directory = Directory::new();
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.clone()),
        ..Default::default()
    };
    let (pruned, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1, 2],
        &options,
        &Default::default(),
    )
    .unwrap();
    assert!(report.removals.iter().any(|r| r.level == 1));
    assert!(report.removals.iter().any(|r| r.level == 0));
    assert_eq!(
        pruned.levels,
        replay_certified_projection(&a, &constraints, &[], &[0, 1, 2], &[], &report.removals)
            .unwrap()
            .levels
    );
    let (warm, warm_report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1, 2],
        &options,
        &Default::default(),
    )
    .unwrap();
    assert_eq!(pruned.levels, warm.levels);
    assert_eq!(report.removals, warm_report.removals);
    assert_eq!(warm_report.cache.prefix_disk_hits, 3);
}

#[test]
fn excluded_sign_constraint_keeps_the_shared_domain_conservative() {
    let a = two_variables();
    let constraints = ["x", "y", "x-2", "y^2+x-1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (_, narrow) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    assert!(
        narrow
            .removals
            .iter()
            .any(|r| a.parse(&r.certificate.polynomial).unwrap() == a.parse("x-1").unwrap())
    );
    let options = DomainPruningOptions {
        excluded_sign_constraints: vec![2],
        ..Default::default()
    };
    let (shared, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &options,
    )
    .unwrap();
    assert!(
        !report
            .removals
            .iter()
            .any(|r| a.parse(&r.certificate.polynomial).unwrap() == a.parse("x-1").unwrap())
    );
    assert!(shared.levels[0].contains(&a.parse("x-1").unwrap()));
    assert_eq!(
        shared.levels,
        replay_certified_projection(&a, &constraints, &[], &[0, 1], &[2], &report.removals)
            .unwrap()
            .levels
    );
}

#[test]
fn cached_chooser_work_is_reused_by_certified_and_ordinary_final_projection() {
    let a = two_variables();
    let constraints = ["x", "y", "y^2+x+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let mut cache = ProjectionCache::new(&a, &Default::default());
    let basis = cache.initial_basis(&constraints).unwrap();
    cache.project_one(&basis, 1).unwrap();
    let misses = cache.report.cache.resultant_misses;
    let pruned = cache
        .project_ordered_certified(&constraints, &[], &[0, 1], &Default::default())
        .unwrap();
    assert_eq!(cache.report.cache.resultant_misses, misses);
    assert_eq!(pruned.levels[0], vec![a.parse("x").unwrap()]);
    assert_eq!(cache.report.removals.len(), 1);
    let full = cache.project_ordered(&constraints, &[0, 1]).unwrap();
    assert_eq!(full.levels, project(&a, &constraints).unwrap().levels);
    assert!(cache.report.removals.is_empty());
}

#[test]
fn progress_survives_cache_drop_and_does_not_change_checkpoint_keys() {
    let a = algebra();
    let directory = Directory::new();
    let progress_path = directory.0.join("progress/first.json");
    let mut options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.join("checkpoints")),
        progress_path: Some(progress_path.clone()),
        ..Default::default()
    };
    let inputs = ["z^2+x", "z+y"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let read_progress = |path: &PathBuf| -> OptimizedProjectionReport {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    };
    {
        let mut cache = ProjectionCache::new(&a, &options);
        let basis = cache.initial_basis(&inputs).unwrap();
        let initial = read_progress(&progress_path);
        assert_eq!(initial.checkpoints.len(), 1);
        assert_eq!(initial.checkpoints[0].eliminated_variable, None);
        assert_eq!(initial.checkpoints[0].polynomial_count, basis.len());
        let output = cache.project_one(&basis, 2).unwrap();
        let after_step = read_progress(&progress_path);
        assert_eq!(after_step.checkpoints.len(), 2);
        assert_eq!(after_step.checkpoints[1].eliminated_variable, Some(2));
        assert_eq!(after_step.checkpoints[1].polynomial_count, output.len());
        assert!(after_step.checkpoints[1].term_count > 0);
        assert!(after_step.checkpoints[1].max_coefficient_bits > 0);
        assert_eq!(after_step.cache.written_checkpoints, 2);
    }
    let saved = read_progress(&progress_path);
    options.progress_path = Some(directory.0.join("progress/second.json"));
    let mut restarted = ProjectionCache::new(&a, &options);
    let basis = restarted.initial_basis(&inputs).unwrap();
    restarted.project_one(&basis, 2).unwrap();
    let warm = read_progress(options.progress_path.as_ref().unwrap());
    assert_eq!(warm.cache.prefix_disk_hits, 2);
    assert_eq!(
        saved.checkpoints.iter().map(|c| &c.key).collect::<Vec<_>>(),
        warm.checkpoints.iter().map(|c| &c.key).collect::<Vec<_>>()
    );
    assert_eq!(
        fs::read_dir(directory.0.join("progress")).unwrap().count(),
        2
    );
    assert_eq!(read_progress(&progress_path).checkpoints.len(), 2);
}

#[test]
fn final_progress_includes_certified_removals() {
    let a = two_variables();
    let directory = Directory::new();
    let progress_path = directory.0.join("progress.json");
    let options = OptimizedProjectionOptions {
        progress_path: Some(progress_path.clone()),
        ..Default::default()
    };
    let constraints = ["x", "y", "y^2+x+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (_, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &options,
        &Default::default(),
    )
    .unwrap();
    let saved: OptimizedProjectionReport =
        serde_json::from_slice(&fs::read(progress_path).unwrap()).unwrap();
    assert_eq!(saved.removals, report.removals);
    assert_eq!(saved.removals.len(), 1);
}

fn input_pruning() -> DomainPruningOptions {
    DomainPruningOptions {
        prune_input: true,
        ..Default::default()
    }
}

#[test]
fn initial_positive_factors_are_certified_and_domain_faces_are_preserved() {
    let a = two_variables();
    let constraints = ["2*x", "3-3*x", "y", "2-y", "x^2+y^2+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let split = vec![a.parse("x+y+1").unwrap(), a.parse("y+1").unwrap()];
    let (pruned, report) = project_ordered_certified(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &Default::default(),
        &input_pruning(),
    )
    .unwrap();
    assert_eq!(report.initial_removals.len(), 2);
    for expression in ["x", "x-1"] {
        assert!(pruned.levels[0].contains(&a.parse(expression).unwrap()));
    }
    for expression in ["y", "y-2", "y+1"] {
        assert!(pruned.levels[1].contains(&a.parse(expression).unwrap()));
    }
    assert_eq!(pruned.levels[0].len(), 2);
    assert_eq!(pruned.levels[1].len(), 3);
    let replay = replay_certified_projection_with_inputs(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &[],
        &report.initial_removals,
        &report.removals,
    )
    .unwrap();
    assert_eq!(pruned.levels, replay.levels);
    let (_, default_report) = project_ordered_certified(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    assert!(default_report.initial_removals.is_empty());
}

#[test]
fn initial_removal_preserves_original_signs_and_does_not_certify_empty_boxes() {
    let a = two_variables();
    let constraints = ["x", "y", "-x^2-y^2-1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (pruned, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &input_pruning(),
    )
    .unwrap();
    assert_eq!(report.initial_removals.len(), 1);
    assert_eq!(
        pruned.levels,
        vec![vec![a.parse("x").unwrap()], vec![a.parse("y").unwrap()]]
    );
    // Basis representatives are canonically positive; exact original leaf
    // signs still reject this contradictory conjunction after input pruning.
    let sample = [
        symgcad::algebra::rational("1").unwrap(),
        symgcad::algebra::rational("1").unwrap(),
    ];
    assert_eq!(
        symgcad::solver::sign(&symgcad::solver::evaluate(&constraints[2], &sample)),
        -1
    );
    let mut empty_box = constraints.clone();
    empty_box.push(a.parse("-x-1").unwrap());
    let (_, report) = project_ordered_certified(
        &a,
        &empty_box,
        &[],
        &[0, 1],
        &Default::default(),
        &input_pruning(),
    )
    .unwrap();
    assert!(report.initial_removals.is_empty());
    assert!(report.removals.is_empty());
}

#[test]
fn shared_domain_exclusions_and_interior_seams_prevent_initial_removals() {
    let a = two_variables();
    let constraints = ["x", "y", "x-2", "x+y-2"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (_, narrow) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &input_pruning(),
    )
    .unwrap();
    assert_eq!(narrow.initial_removals.len(), 1);
    let shared_options = DomainPruningOptions {
        excluded_sign_constraints: vec![2],
        ..input_pruning()
    };
    let (shared, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &shared_options,
    )
    .unwrap();
    assert!(report.initial_removals.is_empty());
    assert!(shared.levels[1].contains(&a.parse("x+y-2").unwrap()));
    assert!(
        replay_certified_projection_with_inputs(
            &a,
            &constraints,
            &[],
            &[0, 1],
            &[2],
            &narrow.initial_removals,
            &narrow.removals,
        )
        .is_err()
    );
    let seam_constraints = ["x", "1-x", "y", "1-y", "(2*x-1)^2+(2*y-1)^2"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (seam, seam_report) = project_ordered_certified(
        &a,
        &seam_constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &input_pruning(),
    )
    .unwrap();
    assert!(seam_report.initial_removals.is_empty());
    assert!(seam.levels[1].contains(&canonical(&seam_constraints[4])));
}

#[test]
fn initial_replay_rejects_boundary_duplicate_absent_and_forged_certificates() {
    let a = two_variables();
    let constraints = ["x", "y", "x^2+y^2+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let (_, report) = project_ordered_certified(
        &a,
        &constraints,
        &[],
        &[0, 1],
        &Default::default(),
        &input_pruning(),
    )
    .unwrap();
    let check = |certificates: &[_]| {
        replay_certified_projection_with_inputs(
            &a,
            &constraints,
            &[],
            &[0, 1],
            &[],
            certificates,
            &report.removals,
        )
    };
    let proof = report.initial_removals[0].clone();
    assert!(check(&[proof.clone(), proof.clone()]).is_err());
    let mut forged = proof.clone();
    forged.sign *= -1;
    assert!(check(&[forged]).is_err());
    let mut absent = proof.clone();
    absent.polynomial = "x^2+y^2+2".into();
    assert!(check(&[absent]).is_err());
    let domain = symgcad::domain::derive_domain(&a, &constraints, &[]).unwrap();
    let boundary =
        symgcad::domain::certify_nonzero(&a, &constraints[0], &domain, &Default::default())
            .unwrap()
            .unwrap();
    assert!(check(&[boundary]).is_err());
    let mut forged_domain = proof;
    forged_domain.domain.bounds[0].lower = Some("2".into());
    assert!(check(&[forged_domain]).is_err());
}

#[test]
fn input_certificates_survive_restarts_and_plain_projection_clears_them() {
    let a = two_variables();
    let directory = Directory::new();
    let options = OptimizedProjectionOptions {
        checkpoint_dir: Some(directory.0.join("checkpoints")),
        progress_path: Some(directory.0.join("progress.json")),
        ..Default::default()
    };
    let constraints = ["x", "y", "x^2+y^2+1"]
        .iter()
        .map(|s| a.parse(s).unwrap())
        .collect::<Vec<_>>();
    let mut cache = ProjectionCache::new(&a, &options);
    let basis = cache
        .initial_basis_certified(&constraints, &[], &input_pruning())
        .unwrap();
    assert_eq!(basis.len(), 2);
    cache.project_one(&basis, 1).unwrap();
    let misses = cache.report.cache.prefix_misses;
    let pruned = cache
        .project_ordered_certified(&constraints, &[], &[0, 1], &input_pruning())
        .unwrap();
    assert_eq!(cache.report.cache.prefix_misses, misses);
    let initial_removals = cache.report.initial_removals.clone();
    let saved: OptimizedProjectionReport =
        serde_json::from_slice(&fs::read(options.progress_path.as_ref().unwrap()).unwrap())
            .unwrap();
    assert_eq!(saved.initial_removals, initial_removals);
    drop(cache);
    let mut restarted = ProjectionCache::new(&a, &options);
    let warm = restarted
        .project_ordered_certified(&constraints, &[], &[0, 1], &input_pruning())
        .unwrap();
    assert_eq!(warm.levels, pruned.levels);
    assert_eq!(restarted.report.initial_removals, initial_removals);
    assert_eq!(restarted.report.cache.prefix_disk_hits, 2);
    let full = restarted.project_ordered(&constraints, &[0, 1]).unwrap();
    assert_eq!(full.levels, project(&a, &constraints).unwrap().levels);
    assert!(restarted.report.initial_removals.is_empty());
}

fn operation_pruning() -> DomainPruningOptions {
    DomainPruningOptions {
        prune_operations: true,
        ..Default::default()
    }
}

#[test]
fn whole_operation_certificates_skip_factoring_and_carry_original_faces() {
    let a = two_variables();
    let constraints = ["x", "y"]
        .iter()
        .map(|p| a.parse(p).unwrap())
        .collect::<Vec<_>>();
    let split = vec![a.parse("y^2+x+1").unwrap()];
    let (old, old_report) = project_ordered_certified(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    let (new, report) = project_ordered_certified(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &Default::default(),
        &operation_pruning(),
    )
    .unwrap();
    assert_eq!(new.levels, old.levels);
    assert_eq!(new.levels[0], vec![a.parse("x").unwrap()]);
    assert!(!report.operation_removals.is_empty());
    assert!(
        report
            .operation_removals
            .iter()
            .any(|r| a.parse(&r.certificate.polynomial).unwrap() == a.parse("x").unwrap())
    );
    assert!(report.cache.factor_misses < old_report.cache.factor_misses);
    let replay = replay_certified_projection_with_operations(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &[],
        &report.initial_removals,
        &report.removals,
        &report.operation_removals,
    )
    .unwrap();
    assert_eq!(replay.levels, new.levels);
}

#[test]
fn whole_operation_carries_survive_multiple_stages_and_order_remapping() {
    let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
    let constraints = ["x", "1-x", "y", "z"]
        .iter()
        .map(|p| a.parse(p).unwrap())
        .collect::<Vec<_>>();
    let split = ["z^2+x*y+1", "x-1/2"]
        .iter()
        .map(|p| a.parse(p).unwrap())
        .collect::<Vec<_>>();
    for order in [[0, 1, 2], [1, 0, 2]] {
        let (new, report) = project_ordered_certified(
            &a,
            &constraints,
            &split,
            &order,
            &Default::default(),
            &operation_pruning(),
        )
        .unwrap();
        let replay = replay_certified_projection_with_operations(
            &a,
            &constraints,
            &split,
            &order,
            &[],
            &report.initial_removals,
            &report.removals,
            &report.operation_removals,
        )
        .unwrap();
        assert_eq!(new.levels, replay.levels);
        let (old, _) = project_ordered_certified(
            &a,
            &constraints,
            &split,
            &order,
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert_eq!(new.levels, old.levels);
        assert!(
            !report
                .operation_removals
                .iter()
                .any(|r| a.parse(&r.certificate.polynomial).unwrap() == a.parse("x-1/2").unwrap())
        );
    }
}

#[test]
fn whole_operation_replay_rejects_changed_missing_and_duplicate_evidence() {
    let a = two_variables();
    let constraints = ["x", "y"]
        .iter()
        .map(|p| a.parse(p).unwrap())
        .collect::<Vec<_>>();
    let split = vec![a.parse("y^2+x+1").unwrap()];
    let (new, report) = project_ordered_certified(
        &a,
        &constraints,
        &split,
        &[0, 1],
        &Default::default(),
        &operation_pruning(),
    )
    .unwrap();
    let check = |records: &[_]| {
        replay_certified_projection_with_operations(
            &a,
            &constraints,
            &split,
            &[0, 1],
            &[],
            &[],
            &report.removals,
            records,
        )
    };
    let mut wrong = report.operation_removals.clone();
    wrong[0].level = 99;
    assert!(check(&wrong).is_err());
    wrong = report.operation_removals.clone();
    wrong[0].eliminated_variable = 0;
    assert!(check(&wrong).is_err());
    wrong = report.operation_removals.clone();
    wrong[0].certificate.sign *= -1;
    assert!(check(&wrong).is_err());
    wrong = report.operation_removals.clone();
    wrong[0].certificate.polynomial = "x+777".into();
    assert!(check(&wrong).is_err());
    wrong = report.operation_removals.clone();
    wrong.push(wrong[0].clone());
    assert!(check(&wrong).is_err());
    // With all witnesses deleted the checker must restore the missing cuts.
    assert_ne!(check(&[]).unwrap().levels, new.levels);
}

#[test]
fn whole_operation_solver_proof_replays_and_requires_opt_in() {
    let mut problem = symgcad::Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x>0".into(), "y>0".into()],
        split: vec!["y^2+x+1".into()],
        ..Default::default()
    };
    problem.solver.engine = "optimized".into();
    problem.solver.prune_operations = true;
    assert!(symgcad::solver::solve(&problem).is_err());
    problem.solver.positivity = true;
    let mut result = symgcad::solver::solve(&problem).unwrap();
    symgcad::verify::verify(&result).unwrap();
    assert!(
        !result
            .statistics
            .optimized_projection
            .as_ref()
            .unwrap()
            .operation_removals
            .is_empty()
    );
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.cells[0].signs, vec![1]);
    result.problem.solver.prune_operations = false;
    assert!(symgcad::verify::verify(&result).is_err());
}
