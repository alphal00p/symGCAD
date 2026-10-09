use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use symgcad::{
    algebra::{Algebra, canonical},
    optimized_projection::{
        OptimizedProjectionOptions, ProjectionCache, project_optimized, project_ordered_optimized,
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
            .all(|entry| entry.unwrap().path().extension().unwrap() == "json")
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
