use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use symgcad::{Problem, algebra::Algebra, fastsecdec, output::Status};

fn sparse(names: &[String], expression: &str) -> Value {
    let algebra = Algebra::new(names).unwrap();
    let polynomial = algebra.parse(expression).unwrap();
    let mut terms = polynomial
        .coefficients
        .iter()
        .zip(polynomial.exponents_iter())
        .map(|(c, e)| (e.to_vec(), c.to_string()))
        .collect::<Vec<_>>();
    terms.sort();
    let expression = if terms.is_empty() {
        "0".into()
    } else {
        terms
            .iter()
            .map(|(exponents, c)| {
                let mut text = format!("({c})");
                for (name, e) in names.iter().zip(exponents) {
                    if *e != 0 {
                        text.push_str(&format!("*{name}^{e}"));
                    }
                }
                text
            })
            .collect::<Vec<_>>()
            .join("+")
    };
    json!({"variables":names,"expression":expression,"terms":terms.into_iter().map(|(e,c)|
        json!({"coefficient":c,"exponents":e})).collect::<Vec<_>>()})
}

/// A native-order one-loop bubble contract. Its mathematical U/F are derived
/// here directly, independently of the consumer's sparse reconstruction.
fn fixture() -> (Value, Problem) {
    let names = vec!["k0".into(), "x0".into(), "x1".into()];
    let u = sparse(&names, "x0+x1");
    let f = sparse(&names, "(x0+x1)^2-k0*x0*x1");
    let fixed = sparse(&names, "x0^2-3*x0*x1+x1^2");
    let problem = Problem {
        name: Some("exact bubble import".into()),
        variables: vec!["x0".into(), "x1".into()],
        variable_groups: vec![vec!["x0".into(), "x1".into()]],
        constraints: vec!["x0 > 0".into(), "x1 > 0".into(), "k0 > 4".into()],
        split: vec![f["expression"].as_str().unwrap().into()],
        parameters: BTreeMap::from([("k0".into(), "5".into())]),
        ..Default::default()
    };
    let graph = json!({"loop_count":1,"propagators":[
        {"edge_id":7,"denominator_index":0,"parameter":"x0","source":0,"target":1,"particle":"scalar","power":1,"denominator":"l^2-1"},
        {"edge_id":9,"denominator_index":1,"parameter":"x1","source":1,"target":0,"particle":"scalar","power":1,"denominator":"(l+p)^2-1"}],
        "numerator_contracted":false,"stable_dot":"digraph G {}","vertices":[0,1],"loop_edges":[7],"external_edges":[],
        "dependent_external_edges":[],"independent_externals":[],"dependent_externals":[],"gram":[],"scalar_bindings":[],"measure_multiplier":"1"});
    (
        json!({"schema":"fastsecdec.symanzik","version":1,"stage":"pre_sector_projective",
        "symbols":[{"name":"k0","native_symbol":"native::s","role":"kinematic"},
            {"name":"x0","native_symbol":"native::a","role":"feynman"},
            {"name":"x1","native_symbol":"native::b","role":"feynman"}],
        "symbolic":{"u":u,"f":f},"specialized":{"u":u,"f":fixed},
        "substitutions":[{"name":"k0","native_symbol":"native::s","value":"5"}],
        "domain":{"source":"projective_simplex","analysis":"positive_orthant","feynman_positive":["x0","x1"],
            "kinematic_constraints":["k0 > 4"],"u_degree":1,"f_degree":2},
        "graph":graph,"problem":{"file":"problem.toml","blake3":"filled by encode"},"provenance":{"producer":"fastsecdec"}}),
        problem,
    )
}

fn encode(mut bundle: Value, problem: &Problem) -> (Vec<u8>, Vec<u8>) {
    let problem = toml::to_string_pretty(problem).unwrap().into_bytes();
    bundle["problem"]["blake3"] = blake3::hash(&problem).to_hex().to_string().into();
    (serde_json::to_vec(&bundle).unwrap(), problem)
}
fn check(bundle: Value, problem: &Problem) -> anyhow::Result<fastsecdec::CheckedImport> {
    let (b, p) = encode(bundle, problem);
    fastsecdec::validate(&b, &p)
}
fn rejected(bundle: Value, problem: &Problem, needle: &str) {
    let error = match check(bundle, problem) {
        Ok(_) => panic!("accepted malformed bundle: {needle}"),
        Err(e) => format!("{e:#}"),
    };
    assert!(error.contains(needle), "{error}");
}
fn directory() -> PathBuf {
    let path = PathBuf::from("artifacts/tests").join(format!(
        "fastsecdec-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn exact_bubble_import_retains_signed_problem_and_independently_verified_cells() {
    let (bundle, problem) = fixture();
    let mut checked = check(bundle, &problem).unwrap();
    assert!(!checked.summary.coverage_claimed && !checked.summary.graph_derivation_verified);
    assert_eq!(checked.summary.specialized_terms, [2, 3]);
    assert_eq!(checked.problem.split, problem.split);
    checked.problem.solver.engine = "optimized".into();
    checked.problem.limits.wall_time_secs = 10.0;
    let result = symgcad::solver::solve(&checked.problem).unwrap();
    assert_eq!(
        result.status,
        Status::CompleteGeneric,
        "{:?}",
        result.diagnostic
    );
    assert_eq!(result.cells.len(), 3);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
}

#[test]
fn signed_scale_is_checked_before_any_primitive_normalization() {
    let (bundle, problem) = fixture();
    let names = vec!["k0".into(), "x0".into(), "x1".into()];
    for expression in ["-x0^2+3*x0*x1-x1^2", "2*x0^2-6*x0*x1+2*x1^2"] {
        let mut bad = bundle.clone();
        bad["specialized"]["f"] = sparse(&names, expression);
        rejected(bad, &problem, "signed F specialization identity failed");
    }
    for expression in ["-((x0+x1)^2-k0*x0*x1)", "2*((x0+x1)^2-k0*x0*x1)"] {
        let mut bad = problem.clone();
        bad.split = vec![expression.into()];
        rejected(bundle.clone(), &bad, "companion signed F split mismatch");
    }
    // A consistently signed, nonprimitive rational scale is valid and must be
    // retained, not normalized by the interchange layer.
    let mut valid = bundle;
    valid["symbolic"]["f"] = sparse(&names, "-3/2*((x0+x1)^2-k0*x0*x1)");
    valid["specialized"]["f"] = sparse(&names, "-3/2*(x0^2-3*x0*x1+x1^2)");
    let mut scaled = problem;
    scaled.split = vec![
        valid["symbolic"]["f"]["expression"]
            .as_str()
            .unwrap()
            .into(),
    ];
    assert!(check(valid, &scaled).is_ok());
}

#[test]
fn edge_parameter_native_alias_and_complete_map_tampering_are_rejected() {
    let (bundle, problem) = fixture();
    let mut bad = bundle.clone();
    bad["graph"]["propagators"][0]["parameter"] = "x1".into();
    bad["graph"]["propagators"][1]["parameter"] = "x0".into();
    rejected(bad, &problem, "denominator/parameter binding mismatch");
    let mut bad = bundle.clone();
    bad["graph"]["propagators"][1]["edge_id"] = 7.into();
    rejected(bad, &problem, "duplicate propagator edge id");
    let mut bad = bundle.clone();
    bad["symbols"][2]["name"] = "x0".into();
    rejected(bad, &problem, "duplicate variable");
    let mut bad = bundle.clone();
    bad["symbols"][2]["native_symbol"] = "native::a".into();
    rejected(bad, &problem, "duplicate native symbol");
    let mut bad = bundle.clone();
    bad["specialized"]["f"]["variables"] = json!(["x0", "x1"]);
    rejected(bad, &problem, "complete alias map");
    let mut bad = bundle;
    bad["specialized"]["f"]["terms"][0]["exponents"] = json!([0, 1]);
    rejected(bad, &problem, "exponent-map length");
}

#[test]
fn missing_wrong_and_inexact_point_bindings_cannot_be_hidden() {
    let (bundle, problem) = fixture();
    let mut bad = bundle.clone();
    bad["substitutions"] = json!([]);
    rejected(bad, &problem, "signed F specialization identity failed");
    let mut bad = bundle.clone();
    bad["substitutions"][0]["native_symbol"] = "other::s".into();
    rejected(bad, &problem, "point/native binding mismatch");
    let mut bad = bundle.clone();
    bad["specialized"]["f"] = bad["symbolic"]["f"].clone();
    rejected(bad, &problem, "still depends on a fixed point");
    for value in ["0.5", "1/0", "2^1000000000", "1/2/3"] {
        let mut bad = bundle.clone();
        bad["substitutions"][0]["value"] = value.into();
        assert!(check(bad, &problem).is_err());
    }
    let mut bad = bundle.clone();
    bad["substitutions"][0]["value"] = json!(5.0);
    rejected(bad, &problem, "invalid fastSecDec bundle");
    let mut bad = problem.clone();
    bad.parameters.clear();
    rejected(bundle, &bad, "companion point bindings mismatch");
}

#[test]
fn sparse_expression_duplicate_monomials_and_degenerate_homogeneity_fail() {
    let (bundle, problem) = fixture();
    let mut bad = bundle.clone();
    bad["specialized"]["f"]["expression"] = "x0^2-3*x0*x1+x1^2".into();
    rejected(bad, &problem, "sparse expression");
    let mut bad = bundle.clone();
    let first = bad["symbolic"]["u"]["terms"][0].clone();
    bad["symbolic"]["u"]["terms"]
        .as_array_mut()
        .unwrap()
        .insert(0, first);
    rejected(bad, &problem, "unique, lexicographically increasing");
    for which in ["u", "f"] {
        let mut bad = bundle.clone();
        bad["specialized"][which]["terms"] = json!([]);
        bad["specialized"][which]["expression"] = "0".into();
        rejected(bad, &problem, "degenerate zero U/F");
    }
    let mut bad = bundle.clone();
    bad["graph"]["loop_count"] = 2.into();
    rejected(bad, &problem, "loop count and U/F degrees disagree");
    let mut bad = bundle.clone();
    bad["symbolic"]["f"]["terms"][0]["exponents"] = json!([0, 0, 3]);
    rejected(bad, &problem, "homogeneity degree");
    let mut bad = bundle;
    bad["symbolic"]["u"]["terms"][0]["coefficient"] = "-1".into();
    rejected(bad, &problem, "U must have positive");
}

#[test]
fn explicit_domain_groups_and_unknown_scientific_fields_are_checked() {
    let (bundle, problem) = fixture();
    let mut normalized = bundle.clone();
    let mut normalized_problem = problem.clone();
    let constraint = "((-4)+(1)*k0^1)>0";
    normalized["domain"]["kinematic_constraints"] = json!([constraint]);
    normalized_problem.constraints[2] = constraint.into();
    assert!(check(normalized, &normalized_problem).is_ok());
    let mut bad = problem.clone();
    bad.constraints.pop();
    rejected(bundle.clone(), &bad, "companion explicit domain mismatch");
    let mut bad = problem.clone();
    bad.variable_groups[0].reverse();
    rejected(
        bundle.clone(),
        &bad,
        "companion variable order/groups mismatch",
    );
    for path in ["version", "stage", "schema"] {
        let mut bad = bundle.clone();
        bad[path] = if path == "version" {
            2.into()
        } else {
            "unknown".into()
        };
        rejected(bad, &problem, "unsupported fastSecDec");
    }
    let mut bad = bundle.clone();
    bad["domain"]["physical_region_inferred"] = true.into();
    rejected(bad, &problem, "unknown field");
    for expression in [
        "x0 > 0",
        "k0^1000000000 > 0",
        "2^(2^30)>0",
        "sin(k0)>0",
        "(k0+1)^32>0",
        "k0>=4",
    ] {
        let mut bad = bundle.clone();
        bad["domain"]["kinematic_constraints"] = json!([expression]);
        let mut p = problem.clone();
        p.constraints[2] = expression.into();
        assert!(check(bad, &p).is_err(), "{expression}");
    }
}

#[test]
fn symbolic_or_negative_rational_point_keeps_kinematics_and_homogeneity_exact() {
    let (mut bundle, mut problem) = fixture();
    bundle["substitutions"] = json!([]);
    bundle["specialized"] = bundle["symbolic"].clone();
    problem.variables.insert(0, "k0".into());
    problem.variable_groups.insert(0, vec!["k0".into()]);
    problem.parameters.clear();
    assert!(check(bundle, &problem).is_ok());
    let (mut bundle, mut problem) = fixture();
    bundle["substitutions"][0]["value"] = "-3/2".into();
    bundle["specialized"]["f"] = sparse(
        &["k0".into(), "x0".into(), "x1".into()],
        "x0^2+7/2*x0*x1+x1^2",
    );
    problem.parameters.insert("k0".into(), "-3/2".into());
    // This explicit domain becomes empty; import preserves it instead of
    // inventing a physical region or silently dropping its inequality.
    assert!(check(bundle, &problem).is_ok());

    for (point, expression) in [("0", "x0^2+2*x0*x1+x1^2"), ("2", "x0^2+x1^2")] {
        let (mut bundle, mut problem) = fixture();
        bundle["substitutions"][0]["value"] = point.into();
        bundle["specialized"]["f"] = sparse(&["k0".into(), "x0".into(), "x1".into()], expression);
        problem.parameters.insert("k0".into(), point.into());
        assert!(check(bundle, &problem).is_ok());
    }
}

#[test]
fn sparse_degree_height_and_specialization_work_are_bounded() {
    let (bundle, problem) = fixture();
    let mut bad = bundle.clone();
    bad["symbolic"]["f"]["terms"][0]["exponents"] = json!([u32::MAX, 0, 2]);
    rejected(bad, &problem, "sparse total degree cap");
    let mut bad = bundle.clone();
    bad["symbolic"]["f"]["terms"][0]["coefficient"] = "9".repeat(4097).into();
    rejected(bad, &problem, "rational literal length cap");
    let mut bad = bundle.clone();
    bad["symbolic"]["f"]["terms"][0]["coefficient"] = "0".into();
    rejected(bad, &problem, "zero sparse coefficient");
    // Every individual literal/degree is supported, but their combination
    // exceeds the preflight bound before any exact power is constructed.
    let mut bad = bundle;
    let names = ["k0".into(), "x0".into(), "x1".into()];
    bad["symbolic"]["f"] = sparse(&names, "k0^62*x0^2+x1^2");
    bad["substitutions"][0]["value"] = "9".repeat(1024).into();
    let mut p = problem;
    p.split = vec![bad["symbolic"]["f"]["expression"].as_str().unwrap().into()];
    p.parameters.insert("k0".into(), "9".repeat(1024));
    rejected(bad, &p, "specialized coefficient height estimate");
}

#[test]
fn public_cli_checks_digest_and_writes_only_fresh_validated_problem() {
    let dir = directory();
    let (bundle, problem) = fixture();
    let (b, p) = encode(bundle, &problem);
    let bundle_path = dir.join("symanzik.json");
    let problem_path = dir.join("problem.toml");
    let output = dir.join("checked.toml");
    fs::write(&bundle_path, &b).unwrap();
    fs::write(&problem_path, &p).unwrap();
    let run = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_symgcad"));
        command
            .arg("import-fastsecdec")
            .arg(&bundle_path)
            .arg("--problem")
            .arg(&problem_path)
            .arg("--output")
            .arg(&output);
        symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: 10.0,
                memory_mib: 1024,
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let result = run();
    assert_eq!(result.status, "completed", "{}", result.stderr);
    let report: Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(report["coverage_claimed"], false);
    let imported = Problem::from_toml(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(imported.split, problem.split);
    assert_ne!(run().status, "completed");
    fs::remove_file(&output).unwrap();
    fs::write(&problem_path, b"variables=['forged']").unwrap();
    let failure = run();
    assert_ne!(failure.status, "completed");
    assert!(failure.stderr.contains("BLAKE3"));
    assert!(!output.exists());
    fs::remove_dir_all(dir).unwrap();
}
