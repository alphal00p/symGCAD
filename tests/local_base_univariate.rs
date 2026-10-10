//! Independent integration checks for the optional first-axis support basis.
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use symgcad::{
    Problem,
    algebra::{Algebra, canonical},
    comparison::compare_generic,
    local_projection::{self, LocalEvidence, LocalNode, LocalOptions, LocalSolveResult},
    output::{RootIndexDomain, SolveResult, Status},
};

fn problem(variables: &[&str], constraints: &[&str], split: &[&str]) -> Problem {
    let mut p = Problem {
        variables: variables.iter().map(|s| (*s).into()).collect(),
        constraints: constraints.iter().map(|s| (*s).into()).collect(),
        split: split.iter().map(|s| (*s).into()).collect(),
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.ordering = "given".into();
    p.solver.local_base_univariate_basis = true;
    p.limits.wall_time_secs = 20.0;
    p
}

fn fixture() -> Problem {
    problem(&["x", "z"], &[], &["z^2-(x-1)*(x-2)"])
}

fn options() -> LocalOptions {
    LocalOptions {
        base_univariate_basis: true,
        base_point_projection: true,
        domain_pruning: false,
        positive_ray: false,
        fallback_to_global: false,
        ..Default::default()
    }
}

fn solve(p: &Problem, options: &LocalOptions) -> LocalSolveResult {
    let result = local_projection::solve_local(p, options).unwrap();
    assert_eq!(
        result.solution.status,
        Status::CompleteGeneric,
        "{:?}",
        result.evidence
    );
    assert!(matches!(result.evidence, LocalEvidence::Local { .. }));
    assert!(local_projection::verify_local(&result).unwrap().verified);
    result
}

fn proof(result: &LocalSolveResult) -> SolveResult {
    let mut solution = result.solution.clone();
    solution.local = Some(Box::new(result.evidence.clone()));
    solution
}

fn compare_ordinary(p: &Problem, result: &LocalSolveResult, options: &LocalOptions) {
    let mut ordinary_problem = p.clone();
    ordinary_problem.solver.local_base_univariate_basis = false;
    let ordinary = solve(
        &ordinary_problem,
        &LocalOptions {
            base_univariate_basis: false,
            ..options.clone()
        },
    );
    assert_eq!(
        compare_generic(&proof(result), &proof(&ordinary), &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
}

fn evidence_json(result: &LocalSolveResult) -> Value {
    serde_json::to_value(&result.evidence).unwrap()
}

fn basis(result: &LocalSolveResult) -> Value {
    let evidence = evidence_json(result);
    let basis = evidence["base_univariate_basis"].clone();
    assert_eq!(basis["schema_version"], 2);
    assert!(!basis["sources"].as_array().unwrap().is_empty());
    basis
}

fn polynomial_id(result: &LocalSolveResult, text: &str) -> usize {
    let algebra = Algebra::new(&result.solution.order).unwrap();
    let wanted = canonical(&algebra.parse(text).unwrap());
    result
        .solution
        .polynomials
        .iter()
        .position(|text| algebra.parse(text).unwrap() == wanted)
        .expect("expected polynomial is absent from the explicit registry")
}

#[test]
fn composite_base_roots_keep_both_seams_and_full_real_owner_ordinals() {
    let p = fixture();
    let result = solve(&p, &options());
    let q = polynomial_id(&result, "(x-1)*(x-2)");
    let basis = basis(&result);
    assert!(basis["sources"].as_array().unwrap().iter().any(|source| {
        source["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|block| block["polynomial"] == q)
    }));
    let LocalEvidence::Local {
        root:
            LocalNode::Stack {
                roots,
                sectors,
                projection,
                ..
            },
        ..
    } = &result.evidence
    else {
        panic!()
    };
    assert!(projection.base_point);
    let owned: Vec<_> = roots.iter().filter(|r| r.polynomial == q).collect();
    assert_eq!(owned.len(), 2);
    assert_eq!(owned.iter().map(|r| r.index).collect::<Vec<_>>(), [0, 1]);
    assert!(
        owned
            .iter()
            .all(|r| r.index_domain == RootIndexDomain::Real)
    );
    assert_eq!(sectors.len(), roots.len() + 1);
    compare_ordinary(&p, &result, &options());
}

#[test]
fn repeated_irrational_zeros_and_rootless_blocks_remain_in_the_exact_inventory() {
    for q in ["(x^2-2)^2*(x^2-3)", "(x^2-2)^2*(x^2+1)"] {
        let p = problem(&["x", "z"], &[], &["z", &format!("z-({q})")]);
        let result = solve(&p, &options());
        let LocalEvidence::Local {
            root: LocalNode::Stack { roots, sectors, .. },
            ..
        } = &result.evidence
        else {
            panic!()
        };
        assert_eq!(roots.len(), if q.contains("x^2-3") { 4 } else { 2 });
        assert_eq!(sectors.len(), roots.len() + 1);
        let basis = basis(&result);
        assert!(basis["sources"].as_array().unwrap().iter().any(|source| {
            source["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|block| block["multiplicity"] == 2)
        }));
        compare_ordinary(&p, &result, &options());
    }
}

#[test]
fn positive_coordinates_and_zero_split_keep_original_labels_and_interior_seams() {
    let p = problem(&["x", "z"], &["x>0", "z>0"], &["0", "z^2-(x-1)*(x-2)"]);
    let result = solve(&p, &options());
    basis(&result);
    assert!(!result.solution.cells.is_empty());
    for cell in &result.solution.cells {
        assert_eq!(cell.signs.len(), 2);
        assert_eq!(cell.signs[0], 0);
        assert!(matches!(cell.signs[1], -1 | 1));
        for axis in &cell.axes {
            assert!(symgcad::algebra::rational(&axis.sample).unwrap() > 0);
        }
    }
    assert_eq!(
        result
            .solution
            .cells
            .iter()
            .map(|c| c.signs[1])
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([-1, 1])
    );
    compare_ordinary(&p, &result, &options());
}

#[test]
fn contradictory_constant_domain_is_complete_empty_with_a_zero_split() {
    let p = problem(
        &["x", "z"],
        &["x>0", "z>0", "0>0"],
        &["0", "z^2-(x-1)*(x-2)"],
    );
    let result = solve(&p, &options());
    assert!(result.solution.cells.is_empty());
    compare_ordinary(&p, &result, &options());
}

#[test]
fn initial_input_factors_remain_protected_when_a_generated_source_overlaps_them() {
    let p = problem(&["x", "z"], &[], &["(x-1)*(x-2)", "z^2-(x-1)*(x-2)"]);
    let result = solve(&p, &options());
    let LocalEvidence::Local { input_factors, .. } = &result.evidence else {
        panic!()
    };
    assert_eq!(input_factors[0].len(), 2);
    let expected = BTreeSet::from([polynomial_id(&result, "x-1"), polynomial_id(&result, "x-2")]);
    assert_eq!(
        input_factors[0].iter().copied().collect::<BTreeSet<_>>(),
        expected
    );
    compare_ordinary(&p, &result, &options());
}

#[test]
fn a_single_active_later_axis_does_not_qualify_as_the_actual_first_axis() {
    let mut p = problem(&["unused", "x", "z"], &[], &["z^2-(x-1)*(x-2)"]);
    p.solver.order = p.variables.clone();
    let result = solve(&p, &options());
    assert_eq!(result.solution.order, p.variables);
    let evidence = evidence_json(&result);
    assert!(
        evidence["base_univariate_basis"].is_null()
            || evidence["base_univariate_basis"]["sources"]
                .as_array()
                .unwrap()
                .is_empty()
    );
    let algebra = Algebra::new(&result.solution.order).unwrap();
    let composite = canonical(&algebra.parse("(x-1)*(x-2)").unwrap());
    assert!(
        result
            .solution
            .polynomials
            .iter()
            .all(|text| algebra.parse(text).unwrap() != composite)
    );
    polynomial_id(&result, "x-1");
    polynomial_id(&result, "x-2");
    compare_ordinary(&p, &result, &options());
}

#[test]
fn deferred_vanishing_leading_coefficient_uses_the_same_complete_base_basis() {
    let p = problem(&["x", "z"], &[], &["x*(x-2)*z+1"]);
    let options = LocalOptions {
        defer_projection: true,
        stable_sector_samples: true,
        ..options()
    };
    let result = solve(&p, &options);
    let LocalEvidence::Local {
        root: LocalNode::Stack { roots, .. },
        report,
        ..
    } = &result.evidence
    else {
        panic!()
    };
    assert_eq!(roots.len(), 2);
    assert!(report.refinements > 0);
    basis(&result);
    compare_ordinary(&p, &result, &options);
}

#[test]
fn independent_replay_rejects_forged_basis_identity_support_version_and_budgets() {
    let result = solve(&fixture(), &options());
    basis(&result);
    for case in 0..12 {
        let mut encoded = evidence_json(&result);
        match case {
            0 => encoded["base_univariate_basis"]["schema_version"] = json!(99),
            1 => encoded["base_univariate_basis"]["sources"][0]["polynomial"] = json!("x+1"),
            2 => {
                encoded["base_univariate_basis"]["sources"][0]["polynomial"] = json!("-(x-1)*(x-2)")
            }
            3 => {
                encoded["base_univariate_basis"]["sources"][0]["blocks"][0]["multiplicity"] =
                    json!(0)
            }
            4 => {
                encoded["base_univariate_basis"]["sources"][0]["blocks"][0]["multiplicity"] =
                    json!(u32::MAX)
            }
            5 => {
                encoded["base_univariate_basis"]["sources"][0]["blocks"][0]["polynomial"] =
                    json!(usize::MAX)
            }
            6 => encoded["base_univariate_basis"]["sources"][0]["blocks"] = json!([]),
            7 => encoded["base_univariate_basis"]["sources"][0]["polynomial"] = json!("x*z+1"),
            8 => encoded["options"]["base_univariate_basis"] = json!(false),
            9 => encoded["base_univariate_basis"] = Value::Null,
            10 => {
                let duplicate = encoded["base_univariate_basis"]["sources"][0].clone();
                encoded["base_univariate_basis"]["sources"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            11 => {
                let duplicate = encoded["base_univariate_basis"]["sources"][0]["blocks"][0].clone();
                encoded["base_univariate_basis"]["sources"][0]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            _ => unreachable!(),
        }
        let evidence: LocalEvidence = serde_json::from_value(encoded).unwrap();
        assert!(
            local_projection::verify_local_parts(&result.solution, &evidence).is_err(),
            "mutation {case}"
        );
    }
}

#[test]
fn missing_sectors_wrong_ordinals_and_a_nonbase_projection_are_rejected() {
    let result = solve(&fixture(), &options());
    for case in 0..4 {
        let mut wrong = result.clone();
        let LocalEvidence::Local {
            root:
                LocalNode::Stack {
                    roots,
                    sectors,
                    projection,
                    ..
                },
            ..
        } = &mut wrong.evidence
        else {
            panic!()
        };
        match case {
            0 => {
                sectors.remove(1);
            }
            1 => {
                roots[1].index = 0;
            }
            2 => {
                roots.remove(1);
                sectors.remove(1);
            }
            3 => {
                projection.base_point = false;
            }
            _ => unreachable!(),
        }
        assert!(
            local_projection::verify_local(&wrong).is_err(),
            "mutation {case}"
        );
    }
}

#[test]
fn serialized_schema_one_and_two_proofs_replay_without_report_authority() {
    let result = solve(&fixture(), &options());
    for version in [1, 2] {
        let mut encoded = serde_json::to_value(&result).unwrap();
        encoded["evidence"]["base_univariate_basis"]["schema_version"] = json!(version);
        // Historical reports did not contain cumulative allocation metadata.
        encoded["evidence"]["report"]["base_univariate_basis"]
            .as_object_mut()
            .unwrap()
            .remove("retained_storage");
        let bytes = serde_json::to_vec(&encoded).unwrap();
        let decoded: LocalSolveResult = serde_json::from_slice(&bytes).unwrap();
        assert!(local_projection::verify_local(&decoded).unwrap().verified);
    }
}

#[test]
fn historical_missing_fields_keep_the_original_irreducible_contract() {
    let mut p = fixture();
    p.solver.local_base_univariate_basis = false;
    let options = LocalOptions {
        base_univariate_basis: false,
        ..options()
    };
    let result = solve(&p, &options);
    let mut encoded = serde_json::to_value(&result).unwrap();
    encoded["evidence"]
        .as_object_mut()
        .unwrap()
        .remove("base_univariate_basis");
    encoded["evidence"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("base_univariate_basis");
    encoded["solution"]["problem"]["solver"]
        .as_object_mut()
        .unwrap()
        .remove("local_base_univariate_basis");
    let decoded: LocalSolveResult = serde_json::from_value(encoded).unwrap();
    assert!(local_projection::verify_local(&decoded).unwrap().verified);
    assert!(!LocalOptions::default().base_univariate_basis);
    assert!(!Problem::default().solver.local_base_univariate_basis);
}

#[test]
fn resource_exhaustion_is_incomplete_even_when_global_fallback_is_enabled() {
    let result = local_projection::solve_local(
        &fixture(),
        &LocalOptions {
            max_refinements: 0,
            fallback_to_global: true,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(result.solution.status, Status::Incomplete);
    assert!(matches!(result.evidence, LocalEvidence::Unfinished { .. }));
    assert!(local_projection::verify_local(&result).is_err());
}

#[test]
fn nested_homogeneous_and_monotone_proofs_replay_the_typed_local_base() {
    let mut p = problem(&["r", "x", "z"], &["r>0"], &["z^2-(x-r)*(x-2*r)"]);
    p.solver.homogeneous_anchor = Some("r".into());
    p.solver.homogeneous_block = p.variables.clone();
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let local = result
        .homogeneous
        .as_ref()
        .unwrap()
        .reduced
        .local
        .as_ref()
        .unwrap();
    assert!(serde_json::to_value(local).unwrap()["base_univariate_basis"].is_object());

    let mut p = problem(&["x", "z", "w"], &["w>0"], &["w+z^2-(x-1)*(x-2)"]);
    p.solver.engine = "monotone".into();
    p.solver.monotone_base_engine = "local".into();
    let result = symgcad::solver::solve(&p).unwrap();
    assert_eq!(result.status, Status::CompleteGeneric);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let base = &result.monotone.as_ref().unwrap().base;
    assert!(
        serde_json::to_value(base.local.as_ref().unwrap()).unwrap()["base_univariate_basis"]
            .is_object()
    );
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "symgcad-base-basis-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn durable_hints_decline_explicitly_and_verification_survives_deleted_checkpoints() {
    let directory = TemporaryDirectory::new();
    let mut p = fixture();
    p.solver.checkpoint_dir = Some(directory.0.clone());
    p.solver.local_hint_archive = true;
    let reason = local_projection::solve_local(&p, &options())
        .unwrap_err()
        .to_string()
        .to_ascii_lowercase();
    assert!(
        reason.contains("univariate") || reason.contains("basis"),
        "{reason}"
    );
    p.solver.local_hint_archive = false;
    let result = solve(&p, &options());
    std::fs::remove_dir_all(&directory.0).unwrap();
    assert!(local_projection::verify_local(&result).unwrap().verified);
}

#[test]
fn persistent_support_proposals_replay_exactly_without_root_or_proof_cache_authority() {
    let directory = TemporaryDirectory::new();
    let mut p = fixture();
    p.solver.checkpoint_dir = Some(directory.0.clone());
    let cold = solve(&p, &options());
    let warm = solve(&p, &options());
    let cold_json = evidence_json(&cold);
    let warm_json = evidence_json(&warm);
    let cold_cache = &cold_json["report"]["base_univariate_basis"]["proposal_cache"];
    let warm_cache = &warm_json["report"]["base_univariate_basis"]["proposal_cache"];
    assert!(cold_cache["writes"].as_u64().unwrap() > 0);
    assert!(warm_cache["hits"].as_u64().unwrap() > 0);
    assert_eq!(
        warm_json["report"]["base_univariate_basis"]["decomposition_seconds"],
        0.0
    );
    assert_eq!(
        cold_json["base_univariate_basis"],
        warm_json["base_univariate_basis"]
    );
    assert_eq!(cold_json["root"], warm_json["root"]);
    assert_eq!(
        serde_json::to_value(&cold.solution.cells).unwrap(),
        serde_json::to_value(&warm.solution.cells).unwrap()
    );
    let mut negative = p.clone();
    negative.split[0] = format!("(-7/3)*({})", negative.split[0]);
    let negative = solve(&negative, &options());
    assert!(
        evidence_json(&negative)["report"]["base_univariate_basis"]["proposal_cache"]["hits"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(basis(&negative), basis(&warm));
    std::fs::remove_dir_all(&directory.0).unwrap();
    assert!(local_projection::verify_local(&warm).unwrap().verified);
    // Stored intervals/sector coverage are independently recomputed after the
    // entire proposal directory is gone.
    let mut forged = warm_json;
    forged["root"]["sectors"].as_array_mut().unwrap().pop();
    let mut altered = warm;
    altered.evidence = serde_json::from_value(forged).unwrap();
    assert!(local_projection::verify_local(&altered).is_err());
}

#[test]
fn disabled_cache_and_corrupt_proposals_preserve_the_ordinary_basis_contract() {
    let directory = TemporaryDirectory::new();
    let mut p = fixture();
    p.solver.checkpoint_dir = Some(directory.0.clone());
    let disabled = solve(
        &p,
        &LocalOptions {
            cache_enabled: false,
            ..options()
        },
    );
    assert!(!directory.0.join("base-support-v1").exists());
    assert_eq!(
        evidence_json(&disabled)["report"]["base_univariate_basis"]["proposal_cache"]["lookups"],
        0
    );
    let cold = solve(&p, &options());
    for entry in std::fs::read_dir(directory.0.join("base-support-v1")).unwrap() {
        let path = entry.unwrap().path();
        let mut record: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        record["binding"]["source_sha256"] = "untrusted mismatch".into();
        std::fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
    }
    let retried = solve(&p, &options());
    assert!(
        evidence_json(&retried)["report"]["base_univariate_basis"]["proposal_cache"]["rejected"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        evidence_json(&cold)["root"],
        evidence_json(&retried)["root"]
    );
}
