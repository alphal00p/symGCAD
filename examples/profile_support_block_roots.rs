//! Bounded per-block real-root inventories after authenticated product replay.
//! Cross-block root coincidences are deliberately unresolved in this diagnostic.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits,
    algebra::{Poly, rational},
    benchmark::run_bounded,
    roots::{FiberRoot, isolate_union_observed, refine_once},
};

#[allow(dead_code)]
#[path = "profile_univariate_content.rs"]
mod content;

const MAX_INPUT: usize = 64 * 1024 * 1024;
const MAX_RECORD: usize = 16 * 1024 * 1024;
const MAX_HISTORY: usize = 64 * 1024 * 1024;
const MAX_ENDPOINT_DIGITS: usize = 20_000;
const POLICY: &str = "per-block-native-isolation-v1";

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    candidate: PathBuf,
    proposal: PathBuf,
    #[arg(long)]
    capture_sha256: String,
    #[arg(long)]
    candidate_sha256: String,
    #[arg(long)]
    proposal_sha256: String,
    /// Must not exist, including for resumed runs.
    #[arg(long)]
    output_dir: PathBuf,
    /// Immutable prior checkpoint; its block files are adjacent to it.
    #[arg(long, requires = "resume_sha256")]
    resume_manifest: Option<PathBuf>,
    #[arg(long, requires = "resume_manifest")]
    resume_sha256: Option<String>,
    #[arg(long, default_value_t = 12)]
    cpu: usize,
    #[arg(long, default_value_t = 3600.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    ensure!(file.metadata()?.is_file(), "regular file required");
    let mut bytes = Vec::new();
    (&mut file).take(cap as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= cap, "file byte cap");
    Ok(bytes)
}
fn write_json(path: &Path, value: &Value, replace: bool) -> Result<String> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= MAX_RECORD, "output record byte cap");
    let temp = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    if replace {
        fs::rename(&temp, path)?;
    } else {
        fs::hard_link(&temp, path)?;
        fs::remove_file(&temp)?;
    }
    Ok(hash(&bytes))
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}
fn validate_args(args: &Args) -> Result<()> {
    ensure!(
        args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 3600.,
        "worker deadline (0,3600]"
    );
    for h in [
        &args.capture_sha256,
        &args.candidate_sha256,
        &args.proposal_sha256,
    ] {
        ensure!(valid_digest(h), "expected SHA256 syntax");
    }
    ensure!(
        args.resume_manifest.is_some() == args.resume_sha256.is_some(),
        "resume path and digest must be paired"
    );
    if let Some(h) = &args.resume_sha256 {
        ensure!(valid_digest(h), "resume SHA256 syntax");
    }
    Ok(())
}
fn expected_binding(args: &Args) -> Value {
    json!({"capture_sha256":args.capture_sha256,"candidate_sha256":args.candidate_sha256,
        "proposal_sha256":args.proposal_sha256,"policy":POLICY})
}
fn block_name(index: usize) -> String {
    format!("block-{index:03}.json")
}

// This is a syntax/shape check for retained native output, not an independent
// root-count proof. It deliberately cannot upgrade historical output authority.
fn endpoint(text: &str) -> Result<Rational> {
    ensure!(
        text.len() <= 2 * MAX_ENDPOINT_DIGITS + 2,
        "endpoint byte cap"
    );
    let parts = text.split('/').collect::<Vec<_>>();
    ensure!(parts.len() <= 2, "rational endpoint grammar");
    for (i, part) in parts.iter().enumerate() {
        let digits = if i == 0 {
            part.strip_prefix('-').unwrap_or(part)
        } else {
            part
        };
        ensure!(
            !digits.is_empty()
                && digits.len() <= MAX_ENDPOINT_DIGITS
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "bounded integer endpoint grammar"
        );
        if i != 0 {
            ensure!(
                digits.bytes().any(|b| b != b'0'),
                "zero endpoint denominator"
            );
        }
    }
    rational(text)
}
fn validate_rows(rows: &Value, degree: usize) -> Result<(usize, usize, usize)> {
    let rows = rows.as_array().context("root interval array")?;
    ensure!(
        rows.len() <= degree && degree <= 4096,
        "root count/degree cap"
    );
    let four = Rational::from(4);
    let mut previous: Option<Rational> = None;
    let mut counts = (0, 0, 0);
    for (i, row) in rows.iter().enumerate() {
        ensure!(
            row["full_real_index"].as_u64() == Some(i as u64),
            "full-real ordinal"
        );
        let lo = endpoint(row["lower"].as_str().context("lower endpoint")?)?;
        let hi = endpoint(row["upper"].as_str().context("upper endpoint")?)?;
        ensure!(
            lo <= hi && previous.as_ref().is_none_or(|old| *old < lo),
            "ordered disjoint intervals"
        );
        let label = if lo == four && hi == four {
            counts.1 += 1;
            "at_four"
        } else if hi <= four {
            counts.0 += 1;
            "below_four"
        } else if lo >= four {
            counts.2 += 1;
            "above_four"
        } else {
            anyhow::bail!("unresolved threshold crossing")
        };
        ensure!(row["relation_to_four"] == label, "threshold classification");
        previous = Some(hi);
    }
    Ok(counts)
}

fn finish_intervals(roots: &mut [FiberRoot], id: usize) -> Result<(Value, usize)> {
    let four = Rational::from(4);
    let mut refinements = 0;
    for (index, root) in roots.iter_mut().enumerate() {
        ensure!(
            root.polynomial == id && root.index == index,
            "singleton root owner/ordinal"
        );
        if root.interval.0 < four && four < root.interval.1 {
            if root.fiber.evaluate(&four).is_zero() {
                // Exactly one root lies strictly inside this certified interval.
                root.interval = (four.clone(), four.clone());
            } else {
                let mut local = 0;
                while root.interval.0 < four && four < root.interval.1 {
                    ensure!(
                        local < 1024 && refinements < 8192,
                        "threshold refinement cap"
                    );
                    refine_once(root);
                    local += 1;
                    refinements += 1;
                }
            }
        }
        if root.interval.0 == root.interval.1 {
            ensure!(
                root.fiber.evaluate(&root.interval.0).is_zero(),
                "false exact rational root"
            );
        }
    }
    let rows = roots.iter().map(|root| {
        let label = if root.interval.0 == four && root.interval.1 == four { "at_four" }
            else if root.interval.1 <= four { "below_four" } else { "above_four" };
        json!({"full_real_index":root.index,"lower":root.interval.0.to_string(),"upper":root.interval.1.to_string(),"relation_to_four":label})
    }).collect::<Vec<_>>();
    let rows = json!(rows);
    validate_rows(&rows, roots.first().map_or(4096, |r| r.fiber.degree()))?;
    Ok((rows, refinements))
}

fn resume_inputs(args: &Args) -> Result<Vec<(PathBuf, String)>> {
    let Some(path) = &args.resume_manifest else {
        return Ok(Vec::new());
    };
    let bytes = read(path, MAX_RECORD)?;
    let digest = args.resume_sha256.as_ref().context("resume hash")?;
    ensure!(hash(&bytes) == *digest, "resume manifest hash");
    let manifest: Value = serde_json::from_slice(&bytes)?;
    ensure!(
        manifest["schema_version"] == 1
            && manifest["kind"] == "per_block_root_checkpoint"
            && manifest["binding"] == expected_binding(args)
            && manifest["cross_block_coincidences_resolved"] == false,
        "resume type/binding"
    );
    let rows = manifest["completed_blocks"]
        .as_array()
        .context("completed blocks")?;
    ensure!(rows.len() <= 64, "checkpoint count cap");
    let mut paths = vec![(path.clone(), digest.clone())];
    let mut seen = BTreeSet::new();
    let mut total = bytes.len();
    for row in rows {
        let index = row["block"].as_u64().context("block index")? as usize;
        ensure!(
            index < 64 && seen.insert(index),
            "unique bounded block index"
        );
        let name = block_name(index);
        ensure!(
            row["file"] == name,
            "checkpoint path must be canonical basename"
        );
        let block_path = path.parent().context("checkpoint parent")?.join(name);
        let block = read(&block_path, MAX_RECORD)?;
        total = total
            .checked_add(block.len())
            .context("checkpoint byte overflow")?;
        ensure!(total <= MAX_HISTORY, "aggregate checkpoint byte cap");
        let digest = row["sha256"].as_str().context("checkpoint digest")?;
        ensure!(
            valid_digest(digest) && hash(&block) == digest,
            "block checkpoint digest"
        );
        paths.push((block_path, digest.to_owned()));
    }
    Ok(paths)
}

fn validate_block(record: &Value, binding: &Value, metadata: &Value) -> Result<()> {
    ensure!(
        record["schema_version"] == 1
            && record["kind"] == "native_single_block_root_inventory"
            && record["complete"] == true
            && record["binding"] == *binding
            && record["block_metadata"] == *metadata
            && record["cross_block_coincidences_resolved"] == false,
        "block checkpoint type/binding"
    );
    ensure!(
        record["native_binary_sha256"]
            .as_str()
            .is_some_and(valid_digest),
        "native executable hash"
    );
    let elapsed = record["isolation_seconds"]
        .as_f64()
        .context("native isolation time")?;
    ensure!(
        elapsed.is_finite() && elapsed >= 0. && elapsed <= 3600.,
        "historical time envelope"
    );
    let degree = metadata["polynomial"]["degree"]
        .as_u64()
        .context("block degree")? as usize;
    let counts = validate_rows(&record["intervals"], degree)?;
    ensure!(
        record["intervals_sha256"] == hash(&serde_json::to_vec(&record["intervals"])?),
        "interval digest"
    );
    ensure!(
        record["distinct_roots_in_this_block"] == counts.0 + counts.1 + counts.2
            && record["below_four"] == counts.0
            && record["at_four"] == counts.1
            && record["above_four"] == counts.2,
        "retained per-block counts"
    );
    Ok(())
}

fn worker(args: &Args) -> Result<()> {
    let started = Instant::now();
    let binding = expected_binding(args);
    let progress_path = args.output_dir.join("progress.json");
    let mut status = json!({"schema_version":1,"binding":binding,"phase":"starting","complete":false,
        "cross_block_coincidences_resolved":false,"original_cad_coverage":false});
    let (replay, factors) = content::replay_for_roots(
        &args.capture,
        &args.candidate,
        &args.proposal,
        &args.capture_sha256,
        &args.candidate_sha256,
        &args.proposal_sha256,
        |phase| {
            status["phase"] = json!(phase);
            status["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
            write_json(&progress_path, &status, true)?;
            Ok(())
        },
    )?;
    write_json(&args.output_dir.join("product-replay.json"), &replay, false)?;
    let metadata = replay["blocks"].as_array().context("replayed blocks")?;
    ensure!(metadata.len() == factors.len(), "block inventory length");
    let input_binding = json!({"inputs":binding,"product":replay["binding"]});
    let mut completed = Vec::<Value>::new();
    let mut completed_ids = BTreeSet::new();
    let prior = resume_inputs(args)?;
    for (path, digest) in prior.iter().skip(1) {
        let bytes = read(path, MAX_RECORD)?;
        ensure!(hash(&bytes) == *digest, "resume input changed");
        let record: Value = serde_json::from_slice(&bytes)?;
        let id = record["block_metadata"]["block"]
            .as_u64()
            .context("prior block index")? as usize;
        ensure!(
            id < factors.len() && !factors[id].0.is_constant() && completed_ids.insert(id),
            "resume block inventory"
        );
        validate_block(&record, &input_binding, &metadata[id])?;
        let name = block_name(id);
        let copied_digest = write_json(&args.output_dir.join(&name), &record, false)?;
        ensure!(
            copied_digest == *digest,
            "checkpoint byte representation changed"
        );
        completed.push(json!({"block":id,"file":name,"sha256":digest,"provenance":"retained_prior_native_output_not_reisolated"}));
    }
    let mut manifest = json!({"schema_version":1,"kind":"per_block_root_checkpoint","binding":binding,
        "completed_blocks":completed,"complete":false,"cross_block_coincidences_resolved":false,
        "resumed_intervals_are_independently_reproved":false,"original_cad_coverage":false});
    write_json(&args.output_dir.join("manifest.json"), &manifest, true)?;
    let mut order: Vec<usize> = (0..factors.len())
        .filter(|i| !factors[*i].0.is_constant())
        .collect();
    order.sort_by_key(|i| (factors[*i].0.degree(0), *i));
    let binary_sha = hash(&fs::read(std::env::current_exe()?)?);
    for id in order {
        if completed_ids.contains(&id) {
            continue;
        }
        ensure!(
            started.elapsed().as_secs_f64() < args.seconds,
            "total worker deadline"
        );
        let p: &Poly = &factors[id].0;
        let isolation_started = Instant::now();
        status["phase"] = json!("native_singleton_isolation");
        status["block"] = json!(id);
        status["block_metadata"] = metadata[id].clone();
        status["completed_blocks"] = json!(completed);
        write_json(&progress_path, &status, true)?;
        let mut events = 0usize;
        let mut roots = isolate_union_observed(
            vec![(id, p.to_univariate_from_univariate(0))],
            |phase, owner, _, roots| {
                events += 1;
                if events == 1 || events % 128 == 0 {
                    status["phase"] = json!(phase);
                    status["block"] = json!(owner);
                    status["observed_roots"] = json!(roots);
                    status["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
                    write_json(&progress_path, &status, true)?;
                }
                Ok(())
            },
        )?;
        let isolation_seconds = isolation_started.elapsed().as_secs_f64();
        status["phase"] = json!("exact_threshold_classification");
        write_json(&progress_path, &status, true)?;
        let threshold_started = Instant::now();
        let (intervals, refinements) = finish_intervals(&mut roots, id)?;
        let counts = validate_rows(&intervals, p.degree(0) as usize)?;
        let record = json!({"schema_version":1,"kind":"native_single_block_root_inventory","complete":true,
            "binding":input_binding,"block_metadata":metadata[id],"native_binary_sha256":binary_sha,
            "isolation_seconds":isolation_seconds,"threshold_seconds":threshold_started.elapsed().as_secs_f64(),
            "threshold_refinements":refinements,"observer_events":events,"intervals":intervals,
            "intervals_sha256":hash(&serde_json::to_vec(&intervals)?),"distinct_roots_in_this_block":roots.len(),
            "below_four":counts.0,"at_four":counts.1,"above_four":counts.2,
            "cross_block_coincidences_resolved":false,"original_cad_coverage":false});
        validate_block(&record, &input_binding, &metadata[id])?;
        let name = block_name(id);
        let digest = write_json(&args.output_dir.join(&name), &record, false)?;
        completed_ids.insert(id);
        completed.push(
            json!({"block":id,"file":name,"sha256":digest,"provenance":"current_native_isolation"}),
        );
        manifest["completed_blocks"] = json!(completed);
        write_json(&args.output_dir.join("manifest.json"), &manifest, true)?;
    }
    manifest["complete"] = json!(true);
    manifest["completed_nonconstant_blocks"] = json!(completed_ids.len());
    manifest["total_worker_seconds"] = json!(started.elapsed().as_secs_f64());
    let manifest_sha = write_json(&args.output_dir.join("manifest.json"), &manifest, true)?;
    status["phase"] = json!("complete");
    status["complete"] = json!(true);
    status["manifest_sha256"] = json!(manifest_sha);
    status["completed_blocks"] = json!(completed);
    status["total_worker_seconds"] = json!(started.elapsed().as_secs_f64());
    status["limitations"] = json!([
        "Per-block exact roots only; common roots between different blocks are unresolved",
        "No sum is claimed as a distinct union count; all multiplicities and same-sign seams remain in source product",
        "Resumed native output is hash-bound historical evidence, not an independent root-count replay",
        "Whole worker wall includes source replay, native isolation, serialization and checkpoints"
    ]);
    write_json(&progress_path, &status, true)?;
    write_json(&args.output_dir.join("worker.json"), &status, false)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    validate_args(&args)?;
    if args.worker {
        return worker(&args);
    }
    ensure!(!args.output_dir.exists(), "fresh output directory required");
    let mut inputs = vec![
        (args.capture.clone(), args.capture_sha256.clone()),
        (args.candidate.clone(), args.candidate_sha256.clone()),
        (args.proposal.clone(), args.proposal_sha256.clone()),
    ];
    inputs.extend(resume_inputs(&args)?);
    for (path, digest) in &inputs {
        ensure!(
            hash(&read(path, MAX_INPUT)?) == *digest,
            "input hash mismatch"
        );
    }
    fs::create_dir_all(&args.output_dir)?;
    let exe = std::env::current_exe()?;
    let mut command = Command::new("taskset");
    command
        .arg("-c")
        .arg(args.cpu.to_string())
        .arg(&exe)
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let limits = Limits {
        wall_time_secs: args.seconds,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    let process = run_bounded(&mut command, &limits)?;
    // Preserve the process outcome even if an input disappeared or changed.
    write_json(
        &args.output_dir.join("process.json"),
        &json!(process),
        false,
    )?;
    let unchanged = inputs
        .iter()
        .all(|(path, digest)| read(path, MAX_INPUT).is_ok_and(|b| hash(&b) == *digest));
    let result = read(&args.output_dir.join("worker.json"), MAX_RECORD)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let manifest = read(&args.output_dir.join("manifest.json"), MAX_RECORD)
        .ok()
        .map(|b| hash(&b));
    let complete = unchanged
        && process.status == "completed"
        && process.exit_code == Some(0)
        && !process.output_truncated
        && result.as_ref().is_some_and(|r| {
            r["complete"] == true
                && manifest
                    .as_ref()
                    .is_some_and(|h| r["manifest_sha256"] == *h)
        });
    let report = json!({"schema_version":1,"complete":complete,"process":process,"limits":limits,"cpu":args.cpu,
        "binary_sha256":hash(&fs::read(exe)?),"inputs":inputs,"inputs_unchanged":unchanged,
        "manifest_sha256":manifest,"result":result,"per_block_only":true,"original_cad_coverage":false});
    write_json(&args.output_dir.join("report.json"), &report, false)?;
    ensure!(
        complete,
        "bounded isolation incomplete; checkpoints and process outcome retained"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::algebra::Algebra;

    #[test]
    fn repeated_root_and_threshold_boundaries_are_retained() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let p = a.parse("(s-4)^2*(s-5)*(s+1)").unwrap();
        let mut roots = isolate_union_observed(
            vec![(7, p.to_univariate_from_univariate(0))],
            |_, _, _, _| Ok(()),
        )
        .unwrap();
        let (rows, _) = finish_intervals(&mut roots, 7).unwrap();
        assert_eq!(validate_rows(&rows, 4).unwrap(), (1, 1, 1));
        assert_eq!(rows[1]["lower"], "4");
        assert_eq!(rows[1]["upper"], "4");
    }

    #[test]
    fn shared_roots_are_not_summed_as_a_distinct_union() {
        let a = Algebra::new(&["s".into()]).unwrap();
        let mut counts = Vec::new();
        for (id, text) in ["(s-1)*(s-2)", "(s-2)*(s-3)"].iter().enumerate() {
            let p = a.parse(text).unwrap();
            let mut roots = isolate_union_observed(
                vec![(id, p.to_univariate_from_univariate(0))],
                |_, _, _, _| Ok(()),
            )
            .unwrap();
            let (rows, _) = finish_intervals(&mut roots, id).unwrap();
            counts.push(validate_rows(&rows, 2).unwrap().0);
        }
        assert_eq!(counts, vec![2, 2]);
        let union = symgcad::roots::isolate_union(
            ["(s-1)*(s-2)", "(s-2)*(s-3)"]
                .iter()
                .enumerate()
                .map(|(id, p)| (id, a.parse(p).unwrap().to_univariate_from_univariate(0)))
                .collect(),
        )
        .unwrap();
        assert_eq!(union.len(), 3);
    }

    #[test]
    fn endpoint_caps_and_resume_shape_fail_closed() {
        assert!(endpoint("1/0").is_err());
        assert!(endpoint("2^1000000").is_err());
        assert!(endpoint(&"1".repeat(MAX_ENDPOINT_DIGITS + 1)).is_err());
        let good =
            json!([{ "full_real_index":0,"lower":"4","upper":"4","relation_to_four":"at_four" }]);
        assert_eq!(validate_rows(&good, 1).unwrap(), (0, 1, 0));
        let mut bad = good.clone();
        bad[0]["full_real_index"] = json!(1);
        assert!(validate_rows(&bad, 1).is_err());
        bad = good.clone();
        bad[0]["relation_to_four"] = json!("above_four");
        assert!(validate_rows(&bad, 1).is_err());
        bad = good;
        bad[0]["lower"] = json!("3");
        bad[0]["upper"] = json!("5");
        assert!(validate_rows(&bad, 1).is_err());
    }
}
