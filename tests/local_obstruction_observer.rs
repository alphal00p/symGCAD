use std::{
    collections::BTreeMap,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
use symgcad::{
    Problem,
    local_projection::{LocalEvidence, LocalOptions, solve_local, verify_local},
    output::Status,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

fn remove_named_diagnostics(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.retain(|key, _| {
                !matches!(
                    key.as_str(),
                    "obstruction_observer" | "observe_obstructions"
                ) && !key.ends_with("_seconds")
            });
            for child in object.values_mut() {
                remove_named_diagnostics(child);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                remove_named_diagnostics(child);
            }
        }
        _ => {}
    }
}

fn operation_records(root: &Path) -> BTreeMap<String, serde_json::Value> {
    fn visit(root: &Path, directory: &Path, records: &mut BTreeMap<String, serde_json::Value>) {
        if !directory.exists() {
            return;
        }
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, records);
            } else if path.extension().is_some_and(|ext| ext == "json") {
                let mut value: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                remove_named_diagnostics(&mut value);
                records.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    value,
                );
            }
        }
    }
    let mut records = BTreeMap::new();
    visit(root, root, &mut records);
    records
}

#[test]
fn observer_preserves_exact_eager_and_deferred_proofs_and_operation_sets() {
    for deferred in [false, true] {
        let root = std::env::temp_dir().join(format!(
            "symgcad-observer-proof-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let cache = root.join("cache");
        let mut problem = Problem {
            variables: vec!["a".into(), "b".into(), "z".into()],
            split: vec!["a*z+b".into(), "z^2-b".into(), "z-1".into()],
            ..Default::default()
        };
        problem.limits.wall_time_secs = 60.0;
        problem.solver.checkpoint_dir = Some(cache.clone());
        // The diagnostic option must not change the archive context namespace.
        problem.solver.local_hint_archive = true;
        let plain_options = LocalOptions {
            fallback_to_global: false,
            defer_projection: deferred,
            stable_sector_samples: deferred,
            ..Default::default()
        };
        let plain = solve_local(&problem, &plain_options).unwrap();
        assert_eq!(plain.solution.status, Status::CompleteGeneric);
        assert!(verify_local(&plain).unwrap().verified);
        let plain_operations = operation_records(&cache);
        assert!(!plain_operations.is_empty());
        std::fs::remove_dir_all(&cache).unwrap();
        let observed = solve_local(
            &problem,
            &LocalOptions {
                observe_obstructions: true,
                ..plain_options
            },
        )
        .unwrap();
        assert_eq!(observed.solution.status, Status::CompleteGeneric);
        assert!(verify_local(&observed).unwrap().verified);
        let LocalEvidence::Local { report, .. } = &observed.evidence else {
            panic!()
        };
        let diagnostics = report.obstruction_observer.as_ref().unwrap();
        assert!(report.hint_archive.as_ref().unwrap().active);
        assert!(diagnostics.failed_visits > 0);
        assert_eq!(diagnostics.invalid_owner, 0);
        assert!(diagnostics.peak_bytes <= diagnostics.byte_limit);
        assert_eq!(plain_operations, operation_records(&cache));
        let mut left = serde_json::to_value(&plain).unwrap();
        let mut right = serde_json::to_value(&observed).unwrap();
        remove_named_diagnostics(&mut left);
        remove_named_diagnostics(&mut right);
        assert_eq!(
            left, right,
            "observer changed exact proof or non-timing counters"
        );
        // Diagnostic data and its enabling flag confer no proof authority.
        let mut changed = observed.clone();
        let LocalEvidence::Local {
            options, report, ..
        } = &mut changed.evidence
        else {
            panic!()
        };
        options.observe_obstructions = false;
        report.obstruction_observer.as_mut().unwrap().failed_visits = usize::MAX;
        assert!(verify_local(&changed).unwrap().verified);
        std::fs::remove_dir_all(&root).unwrap();
    }
}

#[test]
fn historical_options_default_to_no_observation_and_stops_stay_incomplete() {
    let options: LocalOptions = serde_json::from_str("{}").unwrap();
    assert!(!options.observe_obstructions);
    assert!(
        serde_json::to_value(&options)
            .unwrap()
            .get("observe_obstructions")
            .is_none()
    );
    let p = Problem {
        variables: vec!["a".into(), "z".into()],
        split: vec!["a*z+1".into()],
        ..Default::default()
    };
    let result = solve_local(
        &p,
        &LocalOptions {
            observe_obstructions: true,
            max_nodes: 1,
            fallback_to_global: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.solution.status, Status::Incomplete);
    assert!(result.solution.cells.is_empty());
    assert!(verify_local(&result).is_err());
    let LocalEvidence::Unfinished {
        report: Some(report),
        ..
    } = result.evidence
    else {
        panic!()
    };
    assert!(report.obstruction_observer.is_some());
}
