//! Audit retained full-problem completion proofs without loading large cell trees.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{
    Deserialize,
    de::{IgnoredAny, SeqAccess, Visitor},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use symgcad::{Problem, campaign::CompletionReport};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "artifacts/corpus/fixed/equivalence.json")]
    fixed_manifest: PathBuf,
    #[arg(long, default_value = "artifacts/corpus/symbolic/equivalence.json")]
    symbolic_manifest: PathBuf,
    #[arg(long = "ledger", required = true)]
    ledgers: Vec<PathBuf>,
    #[arg(long = "supplemental")]
    supplemental: Vec<PathBuf>,
    #[arg(long, default_value = "artifacts/completion-aggregate.json")]
    output: PathBuf,
}

struct HashReader<R> {
    reader: R,
    hash: Sha256,
}
impl<R: Read> Read for HashReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(buffer)?;
        self.hash.update(&buffer[..n]);
        Ok(n)
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_file(path: &Path) -> Result<String> {
    let mut reader = HashReader {
        reader: fs::File::open(path)?,
        hash: Sha256::new(),
    };
    std::io::copy(&mut reader, &mut std::io::sink())?;
    Ok(format!("{:x}", reader.hash.finalize()))
}
fn read_hashed<T: serde::de::DeserializeOwned>(path: &Path) -> Result<(T, String)> {
    let mut reader = BufReader::with_capacity(
        65536,
        HashReader {
            reader: fs::File::open(path)?,
            hash: Sha256::new(),
        },
    );
    let value = serde_json::from_reader(&mut reader)?;
    std::io::copy(&mut reader, &mut std::io::sink())?;
    Ok((value, format!("{:x}", reader.into_inner().hash.finalize())))
}
fn count_array<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<usize, D::Error> {
    struct Counter;
    impl<'de> Visitor<'de> for Counter {
        type Value = usize;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an array")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<usize, A::Error> {
            let mut n = 0;
            while seq.next_element::<IgnoredAny>()?.is_some() {
                n += 1;
            }
            Ok(n)
        }
    }
    d.deserialize_seq(Counter)
}
#[derive(Deserialize)]
struct ProofHeader {
    schema_version: u32,
    status: String,
    problem: Problem,
    order: Vec<String>,
    normalized_constraints: Vec<String>,
    split_polynomials: Vec<String>,
    #[serde(deserialize_with = "count_array")]
    cells: usize,
}
#[derive(Deserialize)]
struct Check {
    verified: bool,
    cells: usize,
}

/// Strict domains and all requested signs, independent of algorithm settings.
fn mathematical_key(problem: &Problem) -> Result<String> {
    let normalized = symgcad::problem::normalize(problem, &problem.variables)?;
    let mut constraints: Vec<_> = normalized
        .positives
        .iter()
        .map(|p| normalized.algebra.format(p))
        .collect();
    constraints.sort();
    constraints.dedup();
    let split: Vec<_> = normalized
        .split
        .iter()
        .map(|p| normalized.algebra.format(p))
        .collect();
    Ok(serde_json::to_string(
        &json!({"variables":problem.variables,"variable_groups":problem.variable_groups,"constraints":constraints,"split":split}),
    )?)
}
struct Expected {
    data: Value,
    problem: Problem,
    math: String,
    attempts: Vec<Value>,
    verified: usize,
}
#[derive(Clone)]
struct Supplement {
    source: PathBuf,
    label: String,
    value: Value,
    executable_sha256: String,
}
fn canonical(path: &Path) -> Result<PathBuf> {
    Ok(fs::canonicalize(path).with_context(|| format!("missing {}", path.display()))?)
}
fn required_str<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key].as_str().with_context(|| format!("missing {key}"))
}
fn check_order(expected: &Problem, result: &ProofHeader) -> Result<()> {
    let mut check = result.problem.clone();
    check.solver.order = result.order.clone();
    ensure!(
        result.order.len() == result.problem.variables.len(),
        "incomplete proof order"
    );
    check.validate()?;
    if !result.problem.solver.order.is_empty() {
        ensure!(
            result.problem.solver.order == result.order,
            "proof order differs from embedded fixed order"
        );
    } else if result.problem.solver.ordering == "given" {
        let requested = if result.problem.variable_groups.is_empty() {
            result.problem.variables.clone()
        } else {
            result
                .problem
                .variable_groups
                .iter()
                .flatten()
                .cloned()
                .collect()
        };
        ensure!(
            requested == result.order,
            "proof order differs from embedded given order"
        );
    }
    if !expected.solver.order.is_empty() {
        ensure!(
            expected.solver.order == result.order,
            "proof does not preserve original explicit fixed order"
        );
    }
    Ok(())
}
fn check_proof(expected: &Expected, input: &Problem, result: &ProofHeader) -> Result<()> {
    ensure!(
        result.schema_version == 1 && result.status == "complete_generic",
        "proof is not complete_generic schema1"
    );
    ensure!(
        mathematical_key(input)? == expected.math,
        "attempt input differs from original full mathematical problem"
    );
    ensure!(
        mathematical_key(&result.problem)? == expected.math,
        "proof input differs from original full mathematical problem"
    );
    check_order(&expected.problem, result)?;
    // Check the original-coordinate polynomial summary even for transformed proofs.
    let mut summary = result.problem.clone();
    summary.parameters.clear();
    summary.constraints = result
        .normalized_constraints
        .iter()
        .map(|p| format!("({p}) > 0"))
        .collect();
    summary.split = result.split_polynomials.clone();
    ensure!(
        mathematical_key(&summary)? == expected.math,
        "proof polynomial summary differs from original problem"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut expected = Vec::<Expected>::new();
    let mut aliases = BTreeMap::<String, usize>::new();
    let auditor = std::env::current_exe()?;
    let mut sources =
        vec![json!({"kind":"audit_executable","path":auditor,"sha256":hash_file(&auditor)?})];
    for (scope, path) in [
        ("fixed", &args.fixed_manifest),
        ("symbolic_core", &args.symbolic_manifest),
    ] {
        let bytes = fs::read(path)?;
        let manifest: Value = serde_json::from_slice(&bytes)?;
        let directory = path.parent().context("manifest parent missing")?;
        let mut inputs = BTreeMap::<String, (PathBuf, Problem, String)>::new();
        for entry in fs::read_dir(directory)? {
            let input = entry?.path();
            if input.extension().is_some_and(|x| x == "toml") {
                let bytes = fs::read(&input)?;
                let problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
                let name = problem.name.clone().context("corpus input lacks name")?;
                ensure!(
                    inputs
                        .insert(name.clone(), (input, problem, digest(&bytes)))
                        .is_none(),
                    "duplicate corpus name {name}"
                );
            }
        }
        for group in manifest["groups"]
            .as_array()
            .context("manifest groups missing")?
        {
            let names: Vec<String> = serde_json::from_value(group["identifiers"].clone())?;
            let first = names.first().context("empty equivalence group")?;
            let (input_path, problem, input_hash) =
                inputs.get(first).context("representative input missing")?;
            let math = mathematical_key(problem)?;
            let mut original_inputs = Vec::new();
            for name in &names {
                let (p, candidate, h) = inputs.get(name).context("alias input missing")?;
                ensure!(
                    mathematical_key(candidate)? == math
                        && candidate.solver.order == problem.solver.order,
                    "alias mathematical/fixed-order mismatch {name}"
                );
                ensure!(
                    aliases.insert(name.clone(), expected.len()).is_none(),
                    "duplicate alias {name}"
                );
                original_inputs.push(json!({"path":p,"sha256":h}));
            }
            expected.push(Expected { data: json!({"scope":scope,"identifier":first,"aliases":names,"manifest_normalized_sha256":group["normalized_sha256"],"mathematical_sha256":digest(math.as_bytes()),"original_fixed_order":problem.solver.order,"original_input":input_path,"original_input_sha256":input_hash,"original_inputs":original_inputs}), problem:problem.clone(), math, attempts:vec![], verified:0 });
        }
        sources.push(json!({"kind":"equivalence_manifest","path":path,"sha256":digest(&bytes)}));
    }
    ensure!(
        expected.len() == 331,
        "expected all331 distinct corpus groups, got {}",
        expected.len()
    );

    let mut supplements = BTreeMap::<PathBuf, Vec<Supplement>>::new();
    for path in &args.supplemental {
        let bytes = fs::read(path)?;
        let value: Value = serde_json::from_slice(&bytes)?;
        let binary = required_str(&value, "executable_sha256")?;
        for (label, record) in value
            .as_object()
            .context("supplemental report is not an object")?
        {
            if record.get("result").is_none() {
                continue;
            }
            let result = canonical(Path::new(required_str(record, "result")?))?;
            supplements.entry(result).or_default().push(Supplement {
                source: path.clone(),
                label: label.clone(),
                value: record.clone(),
                executable_sha256: binary.into(),
            });
        }
        sources
            .push(json!({"kind":"supplemental_verification","path":path,"sha256":digest(&bytes)}));
    }
    let mut reports = Vec::new();
    let mut binaries = BTreeMap::<String, PathBuf>::new();
    for path in &args.ledgers {
        let bytes = fs::read(path)?;
        let report: CompletionReport = serde_json::from_slice(&bytes)?;
        let binary_hash = required_str(&report.provenance, "executable_sha256")?;
        let binary_path = PathBuf::from(required_str(&report.provenance, "frozen_executable")?);
        if !binaries.contains_key(binary_hash) {
            ensure!(
                hash_file(&binary_path)? == binary_hash,
                "executable hash mismatch {}",
                binary_path.display()
            );
            binaries.insert(binary_hash.into(), binary_path);
        }
        // A resumed ledger can retain runs made by older frozen executables.
        // Validate each such snapshot rather than attributing history to the
        // latest campaign's provenance.
        for run in &report.runs {
            if !binaries.contains_key(&run.executable_sha256) {
                let prior = path
                    .parent()
                    .context("ledger parent missing")?
                    .join("executables")
                    .join(format!("symgcad-{}", run.executable_sha256));
                ensure!(
                    hash_file(&prior)? == run.executable_sha256,
                    "historical executable hash mismatch {}",
                    prior.display()
                );
                binaries.insert(run.executable_sha256.clone(), prior);
            }
        }
        sources.push(json!({"kind":"completion_ledger","path":path,"sha256":digest(&bytes),"scheduled":report.scheduled.len(),"recorded_runs":report.runs.len(),"provenance":report.provenance}));
        reports.push((path.clone(), report));
    }
    let mut unmapped = Vec::new();
    let mut used_supplements = 0;
    for (path, report) in reports {
        for (run_index, run) in report.runs.iter().enumerate() {
            let Some(&group) = aliases.get(&run.name) else {
                unmapped.push(json!({"ledger":path,"run_index":run_index,"name":run.name,"status":run.status}));
                continue;
            };
            for (trial_index, trial) in run.trials.iter().enumerate() {
                let mut errors = Vec::new();
                let input_path = trial.input_file.as_ref().unwrap_or(&run.input_file);
                let input_hash = trial.input_sha256.as_ref().unwrap_or(&run.input_sha256);
                let input = (|| -> Result<Problem> {
                    ensure!(
                        binaries.contains_key(&run.executable_sha256),
                        "unvalidated run executable"
                    );
                    ensure!(
                        hash_file(&run.input_file)? == run.input_sha256,
                        "run input hash mismatch"
                    );
                    ensure!(
                        trial.input_file.is_some() == trial.input_sha256.is_some(),
                        "incomplete trial input provenance"
                    );
                    let bytes = fs::read(input_path)?;
                    ensure!(digest(&bytes) == *input_hash, "input hash mismatch");
                    let problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
                    ensure!(
                        mathematical_key(&problem)? == expected[group].math,
                        "attempt input differs from original full mathematical problem"
                    );
                    ensure!(
                        problem.variable_groups == expected[group].problem.variable_groups,
                        "original variable groups changed"
                    );
                    Ok(problem)
                })();
                if let Err(error) = &input {
                    errors.push(error.to_string());
                }
                let result_path = canonical(&trial.result_file).ok();
                let additional = result_path
                    .as_ref()
                    .and_then(|p| supplements.remove(p))
                    .unwrap_or_default();
                used_supplements += additional.len();
                let candidate = trial.verified || !additional.is_empty();
                if candidate
                    && (trial.solve.status != "completed" || trial.solve.exit_code != Some(0))
                {
                    errors.push("candidate solve process did not complete successfully".into());
                }
                let mut proof = None;
                let mut actual_result_hash = None;
                if let Some(recorded_hash) = &trial.result_sha256 {
                    let checked = (|| -> Result<()> {
                        if candidate {
                            let (header, hash) = read_hashed::<ProofHeader>(&trial.result_file)?;
                            actual_result_hash = Some(hash.clone());
                            ensure!(hash == *recorded_hash, "result hash mismatch");
                            check_proof(
                                &expected[group],
                                input.as_ref().map_err(|e| anyhow::anyhow!(e.to_string()))?,
                                &header,
                            )?;
                            proof = Some(header);
                        } else {
                            let hash = hash_file(&trial.result_file)?;
                            actual_result_hash = Some(hash.clone());
                            ensure!(hash == *recorded_hash, "result hash mismatch");
                        }
                        Ok(())
                    })();
                    if let Err(error) = checked {
                        errors.push(error.to_string());
                    }
                } else if candidate {
                    errors.push("candidate lacks recorded result hash".into());
                }
                let mut checks = Vec::new();
                if trial.verified {
                    checks.push((json!({"kind":"campaign","verification_file":trial.verification_file,"verification_sha256":trial.verification_sha256,"executable_sha256":run.executable_sha256}), trial.verification_file.clone(), trial.verification_sha256.clone(), trial.verification.as_ref().is_some_and(|v| v.status == "completed" && v.exit_code == Some(0))));
                }
                for supplement in additional {
                    let check_path =
                        PathBuf::from(required_str(&supplement.value, "verification")?);
                    let hash = required_str(&supplement.value, "verification_sha256")?.to_owned();
                    let admitted = supplement.value["verified"] == true
                        && binaries.contains_key(&supplement.executable_sha256);
                    checks.push((json!({"kind":"supplemental","report":supplement.source,"label":supplement.label,"verification_file":check_path,"verification_sha256":hash,"executable_sha256":supplement.executable_sha256}), check_path, Some(hash), admitted));
                }
                let mut verifications = Vec::new();
                let mut verified = false;
                for (mut record, check_path, recorded_hash, admitted) in checks {
                    let checked = (|| -> Result<()> {
                        ensure!(admitted, "verification execution/provenance not successful");
                        let (check, hash) = read_hashed::<Check>(&check_path)?;
                        ensure!(
                            Some(&hash) == recorded_hash.as_ref(),
                            "verification hash mismatch"
                        );
                        ensure!(
                            check.verified,
                            "verification artifact does not say verified"
                        );
                        let header = proof.as_ref().context("no valid matching proof")?;
                        ensure!(
                            check.cells == header.cells,
                            "verification/proof cell count mismatch"
                        );
                        ensure!(errors.is_empty(), "attempt integrity check failed");
                        Ok(())
                    })();
                    record["accepted"] = json!(checked.is_ok());
                    if let Err(error) = checked {
                        record["error"] = json!(error.to_string());
                    } else {
                        verified = true;
                    }
                    verifications.push(record);
                }
                expected[group].verified += usize::from(verified);
                expected[group].attempts.push(json!({"ledger":path,"run_index":run_index,"trial_index":trial_index,"name":run.name,"variant":run.variant,"recorded_status":trial.status,"run_status":run.status,"solve_status":trial.solve.status,"input_file":input_path,"input_sha256":input_hash,"executable_sha256":run.executable_sha256,"result_file":trial.result_file,"recorded_result_sha256":trial.result_sha256,"actual_result_sha256":actual_result_hash,"proof_cells":proof.as_ref().map(|p|p.cells),"proof_order":proof.as_ref().map(|p|&p.order),"audit_errors":errors,"verifications":verifications,"verified_matching_full_problem":verified}));
            }
        }
    }
    let mut rows = Vec::new();
    let mut unresolved = Vec::new();
    let mut complete = 0;
    let mut attempted = 0;
    let mut audit_errors = 0;
    let mut rejected_verifications = 0;
    let mut counts = BTreeMap::<String, Value>::new();
    for item in expected {
        let mut row = item.data;
        let solved = item.verified > 0;
        complete += usize::from(solved);
        attempted += usize::from(!item.attempts.is_empty());
        audit_errors += item
            .attempts
            .iter()
            .filter(|a| a["audit_errors"].as_array().is_some_and(|e| !e.is_empty()))
            .count();
        rejected_verifications += item
            .attempts
            .iter()
            .flat_map(|a| a["verifications"].as_array().into_iter().flatten())
            .filter(|v| v["accepted"] != true)
            .count();
        row["complete_verified"] = json!(solved);
        row["verified_attempts"] = json!(item.verified);
        row["attempts"] = json!(item.attempts);
        if !solved {
            unresolved.push(row.clone());
        }
        let scope = row["scope"].as_str().unwrap().to_owned();
        let counter = counts
            .entry(scope)
            .or_insert(json!({"expected":0,"complete_verified":0}));
        counter["expected"] = json!(counter["expected"].as_u64().unwrap() + 1);
        counter["complete_verified"] =
            json!(counter["complete_verified"].as_u64().unwrap() + u64::from(solved));
        rows.push(row);
    }
    let summary = json!({"expected_groups":rows.len(),"attempted_groups":attempted,"complete_verified":complete,"unresolved":unresolved.len(),"all_complete":complete==rows.len() && unmapped.is_empty() && supplements.is_empty(),"attempts_with_audit_errors":audit_errors,"rejected_verifications":rejected_verifications,"unmapped_runs":unmapped.len(),"used_supplemental_verifications":used_supplements,"unmatched_supplemental_verifications":supplements.values().map(Vec::len).sum::<usize>(),"scopes":counts});
    let report = json!({"schema_version":1,"summary":summary,"sources":sources,"executables":binaries,"groups":rows,"unresolved":unresolved,"unmapped_runs":unmapped,"unmatched_supplemental_results":supplements.keys().collect::<Vec<_>>(),"audit_policy":"Stream-check recorded executable/input/result/verification SHA-256 and verification success; compare original exact normalized full constraints, ordered split list, variables, variable groups and explicit fixed order against attempt input and original-coordinate proof metadata. Preserve every attempt. This checks retained verification evidence, not a new mathematical verifier run. A sign-branch baseline result is not a full split-problem certificate."});
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_ignores_strategy_but_preserves_full_split_domain() {
        let mut p = Problem {
            variables: vec!["x".into()],
            constraints: vec!["2*x > 0".into()],
            split: vec!["x-1".into()],
            ..Default::default()
        };
        let original = mathematical_key(&p).unwrap();
        p.constraints = vec!["x>0".into()];
        p.solver.engine = "optimized".into();
        p.solver.ordering = "mods".into();
        assert_eq!(original, mathematical_key(&p).unwrap());
        p.constraints.push("x-1>0".into());
        p.split.clear();
        assert_ne!(original, mathematical_key(&p).unwrap());
    }
    #[test]
    fn streamed_header_skips_proof_trees_and_counts_cells() {
        let text = r#"{"schema_version":1,"status":"complete_generic","problem":{"variables":["x"],"constraints":["x>0"]},"order":["x"],"normalized_constraints":["x"],"split_polynomials":[],"cells":[{"large":[1,2,3]},{}],"trace":{"nested":[[1,2],[3]]}}"#;
        let header: ProofHeader = serde_json::from_str(text).unwrap();
        assert_eq!(header.cells, 2);
        let mut expected = header.problem.clone();
        expected.solver.order = vec!["x".into()];
        check_order(&expected, &header).unwrap();
    }
    #[test]
    fn original_fixed_order_and_full_problem_are_mandatory() {
        let mut original = Problem {
            variables: vec!["x".into(), "y".into()],
            constraints: vec!["x>0".into()],
            split: vec!["x-y".into()],
            ..Default::default()
        };
        original.solver.order = vec!["x".into(), "y".into()];
        let expected = Expected {
            data: Value::Null,
            math: mathematical_key(&original).unwrap(),
            problem: original.clone(),
            attempts: vec![],
            verified: 0,
        };
        let mut proof = ProofHeader {
            schema_version: 1,
            status: "complete_generic".into(),
            problem: original.clone(),
            order: original.solver.order.clone(),
            normalized_constraints: vec!["x".into()],
            split_polynomials: vec!["x-y".into()],
            cells: 0,
        };
        check_proof(&expected, &original, &proof).unwrap();
        proof.problem.solver.order.reverse();
        proof.order.reverse();
        assert!(
            check_proof(&expected, &original, &proof)
                .unwrap_err()
                .to_string()
                .contains("original explicit fixed order")
        );
        proof.problem = original.clone();
        proof.order = original.solver.order.clone();
        proof.split_polynomials.clear();
        assert!(check_proof(&expected, &original, &proof).is_err());
        proof.split_polynomials = vec!["x-y".into()];
        let mut branch = original.clone();
        branch.constraints.push("x-y>0".into());
        branch.split.clear();
        assert!(check_proof(&expected, &branch, &proof).is_err());
    }
}
