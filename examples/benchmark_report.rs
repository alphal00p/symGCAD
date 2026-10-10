//! Generate the publication table from audited full-problem completion and
//! immutable paired baseline evidence. This does not grant new proof credit.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use symgcad::Problem;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "artifacts/completion-aggregate-303.json")]
    completion: PathBuf,
    #[arg(long, default_value = "artifacts/bench/parity-aggregate-9025.json")]
    parity: PathBuf,
    #[arg(long, default_value = "benchmark/data.json")]
    output: PathBuf,
    /// Optional current slow-queue status; diagnostic outcomes never grant proof credit.
    #[arg(long)]
    slow_status: Option<PathBuf>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn str_at<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().with_context(|| format!("missing string {k}"))
}
fn arr<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>> {
    v[k].as_array()
        .with_context(|| format!("missing array {k}"))
}
fn median(mut xs: Vec<f64>) -> Option<f64> {
    xs.retain(|v| v.is_finite());
    xs.sort_by(f64::total_cmp);
    let n = xs.len();
    if n == 0 {
        None
    } else if n % 2 == 1 {
        Some(xs[n / 2])
    } else {
        Some((xs[n / 2 - 1] + xs[n / 2]) / 2.)
    }
}
fn sig(value: Option<f64>, digits: i32) -> String {
    match value {
        Some(v) if v.is_finite() && v > 0. => {
            let decimals = (digits - 1 - v.log10().floor() as i32).clamp(0, 9) as usize;
            format!("{v:.decimals$}")
        }
        Some(0.) => "0".into(),
        _ => "—".into(),
    }
}
fn mathematical_key(problem: &Problem) -> Result<String> {
    let n = symgcad::problem::normalize(problem, &problem.variables)?;
    let mut c = n
        .positives
        .iter()
        .map(|p| n.algebra.format(p))
        .collect::<Vec<_>>();
    c.sort();
    c.dedup();
    Ok(serde_json::to_string(
        &json!({"variables":problem.variables,"variable_groups":problem.variable_groups,"constraints":c,"split":n.split.iter().map(|p|n.algebra.format(p)).collect::<Vec<_>>()}),
    )?)
}
fn checked_input(path: &Path, expected_hash: Option<&str>) -> Result<Problem> {
    let bytes = fs::read(path)?;
    if let Some(h) = expected_hash {
        ensure!(hash(&bytes) == h, "input hash mismatch: {}", path.display());
    }
    Problem::from_toml(std::str::from_utf8(&bytes)?)
}
fn method(p: &Problem) -> String {
    let mut s = p.solver.engine.clone();
    if p.solver.monotone_negative_target {
        s.push_str("−");
    }
    if p.solver.homogeneous_anchor.is_some() {
        s.push_str("+H");
    }
    if p.solver.ordering == "mcts" {
        s.push_str("+MCTS");
    }
    s
}
fn build_label(provenance: &Value, sha: &str) -> String {
    let raw = provenance["executable"].as_str().unwrap_or(sha);
    let name = Path::new(raw)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(sha);
    let commit = name
        .split('-')
        .rev()
        .find(|s| s.len() == 7 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(&sha[..sha.len().min(7)]);
    let debug = name.contains("debug") || provenance["profile"] == "debug";
    format!("{commit} {}", if debug { "D" } else { "R" })
}
struct Sources {
    values: BTreeMap<String, Value>,
    hashes: BTreeMap<String, String>,
}
impl Sources {
    fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            hashes: BTreeMap::new(),
        }
    }
    fn load(&mut self, path: &str, expected: Option<&str>) -> Result<Value> {
        if !self.values.contains_key(path) {
            let bytes = fs::read(path)?;
            self.hashes.insert(path.into(), hash(&bytes));
            self.values
                .insert(path.into(), serde_json::from_slice(&bytes)?);
        }
        if let Some(h) = expected {
            ensure!(self.hashes[path] == h, "source hash mismatch: {path}");
        }
        Ok(self.values[path].clone())
    }
}
fn candidate(
    trials: &[(&Value, u64)],
    input: &Problem,
    sha: &str,
    build: String,
    source: Value,
) -> Result<Value> {
    ensure!(!trials.is_empty(), "empty candidate");
    let cells = trials[0].1;
    ensure!(
        trials.iter().all(|t| t.1 == cells),
        "unequal cell counts in repeated candidate"
    );
    let times = trials
        .iter()
        .filter_map(|(t, _)| t["solve"]["wall_time_secs"].as_f64())
        .collect::<Vec<_>>();
    ensure!(
        times.len() == trials.len() && times.iter().all(|x| x.is_finite() && *x >= 0.),
        "missing solve timing"
    );
    let check = median(
        trials
            .iter()
            .filter_map(|(t, _)| t["verification"]["wall_time_secs"].as_f64())
            .collect(),
    );
    let rss = trials
        .iter()
        .flat_map(|(t, _)| {
            [
                t["solve"]["peak_rss_kib"].as_u64(),
                t["verification"]["peak_rss_kib"].as_u64(),
            ]
        })
        .flatten()
        .max();
    Ok(
        json!({"solve_seconds":median(times),"verify_seconds":check,"peak_rss_mib":rss.map(|x|x as f64/1024.),"memory_complete":trials.iter().all(|(t,_)|t["solve"]["peak_rss_kib"].is_u64()&&t["verification"]["peak_rss_kib"].is_u64()),"cells":cells,"n":trials.len(),"method":method(input),"build":build,"executable_sha256":sha,"order":input.solver.order,"search_seconds":median(trials.iter().filter_map(|(t,_)|t["statistics"]["search_seconds"].as_f64()).collect()),"source":source}),
    )
}
fn add_overlay(
    candidates: &mut Vec<Value>,
    group: &Value,
    source: &Value,
    trials: Vec<&Value>,
    sha: &str,
    build: &str,
) -> Result<()> {
    if trials.is_empty() {
        return Ok(());
    }
    let mut records = Vec::new();
    let mut original_input = None;
    for trial in trials {
        ensure!(
            trial["status"] == "complete_verified" && trial["solve"]["status"] == "completed",
            "overlay not completed"
        );
        let input = checked_input(
            Path::new(str_at(trial, "input_file")?),
            trial["input_sha256"].as_str(),
        )?;
        ensure!(
            hash(mathematical_key(&input)?.as_bytes()) == str_at(group, "mathematical_sha256")?,
            "overlay changes original problem"
        );
        if !arr(group, "original_fixed_order")?.is_empty() {
            ensure!(
                json!(input.solver.order) == group["original_fixed_order"],
                "overlay changes fixed order"
            );
        }
        let vf = str_at(trial, "verification_file")?;
        let bytes = fs::read(vf)?;
        ensure!(
            Some(hash(&bytes).as_str()) == trial["verification_sha256"].as_str(),
            "overlay verification hash mismatch"
        );
        let verification: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            verification["verified"] == true,
            "overlay proof not verified"
        );
        let cells = verification["cells"].as_u64().context("overlay cells")?;
        original_input.get_or_insert(input);
        records.push((trial, cells));
    }
    let mut measured = candidate(
        &records,
        &original_input.unwrap(),
        sha,
        build.into(),
        source.clone(),
    )?;
    // These three explicitly selected controlled comparisons document a fresh
    // isolated checkpoint directory for every repetition.
    measured["cache_policy"] = json!("cold");
    candidates.push(measured);
    Ok(())
}
fn ratio_summary(cases: &[&Value], reports: &mut Sources) -> Result<Value> {
    if cases.is_empty() {
        return Ok(Value::Null);
    }
    let mut rust = 0.;
    let mut native = 0.;
    let mut ratios = Vec::new();
    let mut provenance = Vec::new();
    let qualified = cases
        .iter()
        .filter(|c| c["reference_qualified"] == true)
        .count();
    for c in cases {
        if c["verified_and_matched"] != true {
            continue;
        }
        let report_path = str_at(c, "report")?;
        let report = reports.load(report_path, None)?;
        let name = str_at(c, "measured_identifier")?;
        let measured = arr(&report, "cases")?
            .iter()
            .find(|r| r["name"] == name)
            .context("paired row absent")?;
        ensure!(
            measured["verified"] == true && measured["raw_cells_match"] == true,
            "paired evidence not verified"
        );
        rust += measured["rust_solver_median_secs"]
            .as_f64()
            .context("Rust matched time")?;
        native += measured["reference_solver_median_secs"]
            .as_f64()
            .context("native matched time")?;
        ratios.push(measured["solver_ratio"].as_f64().context("ratio")?);
        provenance.push(json!({"report":report_path,"case":name,"order":measured["frozen_order"],"repeats":report["repeats"],"rust_seconds":measured["rust_solver_median_secs"],"native_seconds":measured["reference_solver_median_secs"],"ratio":measured["solver_ratio"]}));
    }
    let complete = ratios.len() == cases.len();
    Ok(
        json!({"branches":cases.len(),"qualified_branches":qualified,"complete":complete,"ratio":if complete{Some(rust/native)}else{None},"rust_seconds":if complete{Some(rust)}else{None},"native_seconds":if complete{Some(native)}else{None},"status":if complete{"matched"}else if qualified==0{"native timeout"}else{"partially censored"},"aggregation":"sum of paired per-branch median phase times; separate sign branches remain a separate workload","evidence":provenance}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    let mut sources = Sources::new();
    let completion = sources.load(args.completion.to_str().context("path")?, None)?;
    let parity = sources.load(args.parity.to_str().context("path")?, None)?;
    ensure!(
        completion["summary"]["expected_groups"] == 331
            && completion["summary"]["attempts_with_audit_errors"] == 0,
        "completion audit not clean"
    );
    ensure!(
        parity["summary"]["qualification_coverage_complete"] == true
            && parity["summary"]["parity_passed"] == true,
        "baseline audit incomplete"
    );
    for s in arr(&completion, "sources")? {
        if s["kind"] == "completion_ledger" || s["kind"] == "supplemental_verification" {
            sources.load(str_at(s, "path")?, s["sha256"].as_str())?;
        }
    }
    for s in arr(&parity, "reports")? {
        sources.load(str_at(s, "path")?, s["sha256"].as_str())?;
    }
    let conditional_path = "artifacts/bench/conditional-fixed-control-01/comparison.json";
    let conditional = sources.load(conditional_path, None)?;
    let rayleigh_path = "artifacts/bench/gluza-rayleigh-release-01/comparison.json";
    let rayleigh = sources.load(rayleigh_path, None)?;
    let bilinear_path = "artifacts/bench/gluza30-bilinear-positive-release-01/comparison.json";
    let bilinear = sources.load(bilinear_path, None)?;
    let mut rows = Vec::new();
    for group in arr(&completion, "groups")? {
        let mut candidates = Vec::new();
        let mut measured_runs: BTreeMap<(String, usize), Vec<(Value, u64)>> = BTreeMap::new();
        let mut max_timeout: Option<f64> = None;
        let mut last_status = "incomplete".to_string();
        let mut max_rss = 0.;
        // Group only independently accepted trials from the same campaign run.
        for attempt in arr(group, "attempts")? {
            let ledger_path = str_at(attempt, "ledger")?;
            let ri = attempt["run_index"].as_u64().context("run index")? as usize;
            let ti = attempt["trial_index"].as_u64().context("trial index")? as usize;
            let ledger = sources.values.get(ledger_path).context("ledger absent")?;
            let trial = &ledger["runs"][ri]["trials"][ti];
            if attempt["verified_matching_full_problem"] == true {
                let mut verified_trial = trial.clone();
                let accepted = arr(attempt, "verifications")?
                    .iter()
                    .find(|v| v["accepted"] == true)
                    .context("accepted check absent")?;
                if accepted["kind"] == "supplemental" {
                    let report = &sources.values[str_at(accepted, "report")?];
                    let record = accepted["label"]
                        .as_str()
                        .map(|key| &report[key])
                        .unwrap_or(&report["independent"]);
                    verified_trial["verification"] = if record["process"].is_object() {
                        record["process"].clone()
                    } else {
                        json!({"wall_time_secs":record["elapsed_seconds"]})
                    };
                }
                measured_runs
                    .entry((ledger_path.into(), ri))
                    .or_default()
                    .push((
                        verified_trial,
                        attempt["proof_cells"].as_u64().context("proof cells")?,
                    ));
            } else {
                last_status = trial["status"].as_str().unwrap_or("incomplete").to_string();
                if let Some(x) = trial["solve"]["wall_time_secs"].as_f64() {
                    max_timeout = Some(max_timeout.unwrap_or(0.).max(x));
                }
                if let Some(x) = trial["solve"]["peak_rss_kib"].as_f64() {
                    max_rss = f64::max(max_rss, x / 1024.);
                }
            }
        }
        for ((path, ri), trials) in measured_runs {
            let ledger = &sources.values[&path];
            let run = &ledger["runs"][ri];
            let input_path = trials[0].0["input_file"]
                .as_str()
                .or(run["input_file"].as_str())
                .context("input")?;
            let input = checked_input(Path::new(input_path), None)?;
            let sha = str_at(run, "executable_sha256")?;
            let borrowed = trials.iter().map(|(t, c)| (t, *c)).collect::<Vec<_>>();
            let mut measured = candidate(
                &borrowed,
                &input,
                sha,
                build_label(&ledger["provenance"], sha),
                json!({"ledger":path,"run_index":ri}),
            )?;
            measured["cache_policy"] = json!(
                ledger["provenance"]["cache_policy"]
                    .as_str()
                    .unwrap_or("unknown")
            );
            if let Some(attempt) = arr(group, "attempts")?.iter().find(|a| {
                a["ledger"] == path
                    && a["run_index"] == ri
                    && a["verified_matching_full_problem"] == true
            }) {
                measured["order"] = attempt["proof_order"].clone();
            }
            candidates.push(measured);
        }
        let aliases = arr(group, "aliases")?;
        let matches = |name: &Value| aliases.contains(name);
        if group["complete_verified"] == true {
            let selected = arr(&conditional, "rows")?
                .iter()
                .filter(|r| r["label"] == "new" && matches(&r["name"]))
                .collect::<Vec<_>>();
            if !selected.is_empty() {
                let mut trials = Vec::new();
                let mut ledgers = Vec::new();
                for row in &selected {
                    let path = str_at(row, "ledger")?;
                    ledgers.push(sources.load(path, row["ledger_sha256"].as_str())?);
                }
                for ledger in &ledgers {
                    for run in arr(ledger, "runs")? {
                        if matches(&run["name"]) {
                            trials.extend(arr(run, "trials")?);
                        }
                    }
                }
                add_overlay(
                    &mut candidates,
                    group,
                    &json!({"comparison":conditional_path}),
                    trials,
                    str_at(selected[0], "worker_sha256")?,
                    "0a95bc2 R",
                )?;
            }
            for row in arr(&rayleigh, "cases")? {
                if matches(&row["name"]) {
                    add_overlay(
                        &mut candidates,
                        group,
                        &json!({"comparison":rayleigh_path}),
                        arr(row, "trials")?.iter().collect(),
                        str_at(&rayleigh, "worker_sha256")?,
                        "256527b R",
                    )?;
                }
            }
            if matches(&bilinear["problem"]) {
                add_overlay(
                    &mut candidates,
                    group,
                    &json!({"comparison":bilinear_path,"prior_order_search":bilinear["prior_order_search"]}),
                    arr(&bilinear, "rust_trials")?.iter().collect(),
                    str_at(&bilinear["source"], "rust_sha256")?,
                    "ff22294 R",
                )?;
            }
        }
        // Prefer available three-fresh-run controls; otherwise the fastest
        // accepted full-problem campaign run. Never minimize individual repeats.
        candidates.sort_by(|a, b| {
            let ar = a["n"].as_u64().unwrap() >= 3;
            let br = b["n"].as_u64().unwrap() >= 3;
            br.cmp(&ar).then_with(|| {
                a["solve_seconds"]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&b["solve_seconds"].as_f64().unwrap())
            })
        });
        let best = candidates.first().cloned().unwrap_or(Value::Null);
        ensure!(
            best.is_null() == (group["complete_verified"] != true),
            "missing accepted measurement"
        );
        let basecases = arr(&parity, "cases")?
            .iter()
            .filter(|c| {
                c["scope"] == group["scope"]
                    && c["normalized_sha256"] == group["manifest_normalized_sha256"]
            })
            .collect::<Vec<_>>();
        let ratio = ratio_summary(&basecases, &mut sources)?;
        ensure!(
            !ratio.is_null(),
            "missing baseline identity for {}",
            group["identifier"]
        );
        rows.push(json!({"identifier":group["identifier"],"aliases":group["aliases"],"alias_count":aliases.len(),"scope":group["scope"],"mathematical_sha256":group["mathematical_sha256"],"original_input":group["original_input"],"verified":group["complete_verified"],"slow":group["complete_verified"]!=true,"measurement":best,"candidates":candidates,"last_status":last_status,"longest_incomplete_seconds":max_timeout,"incomplete_peak_rss_mib":max_rss,"baseline":ratio}));
    }
    let slow = if let Some(path) = args.slow_status {
        let journal = sources.load(path.to_str().context("slow queue path")?, None)?;
        for entry in arr(&journal, "entries")? {
            let row = rows
                .iter_mut()
                .find(|r| {
                    r["identifier"] == entry["identifier"]
                        || r["aliases"]
                            .as_array()
                            .is_some_and(|a| a.contains(&entry["identifier"]))
                })
                .context("unknown slow queue identity")?;
            row["slow_status"] = entry["status"].clone();
            if entry["status"] == "finished" {
                let path = str_at(entry, "ledger")?;
                let ledger = sources.load(path, Some(str_at(entry, "ledger_sha256")?))?;
                let run = arr(&ledger, "runs")?.last().context("slow run absent")?;
                let trial = arr(run, "trials")?.last().context("slow trial absent")?;
                let input_path = trial["input_file"]
                    .as_str()
                    .or(run["input_file"].as_str())
                    .context("slow input absent")?;
                let input = checked_input(
                    Path::new(input_path),
                    trial["input_sha256"]
                        .as_str()
                        .or(run["input_sha256"].as_str()),
                )?;
                ensure!(
                    hash(mathematical_key(&input)?.as_bytes())
                        == str_at(row, "mathematical_sha256")?,
                    "slow measurement changes original problem"
                );
                if row["verified"] != true {
                    row["slow_status"] = json!("incomplete");
                    row["last_status"] = trial["status"].clone();
                    // A newer attempt can stop early. Retain the longest
                    // observed attempt promised by the table's heading.
                    row["longest_incomplete_seconds"] = json!(
                        row["longest_incomplete_seconds"]
                            .as_f64()
                            .into_iter()
                            .chain(trial["solve"]["wall_time_secs"].as_f64())
                            .reduce(f64::max)
                    );
                    row["incomplete_peak_rss_mib"] = json!(
                        row["incomplete_peak_rss_mib"]
                            .as_f64()
                            .into_iter()
                            .chain(trial["solve"]["peak_rss_kib"].as_f64().map(|x| x / 1024.))
                            .reduce(f64::max)
                    );
                    if trial["verified"] == true {
                        row["slow_status"] = json!("awaiting aggregate audit");
                    }
                }
            }
        }
        Some(json!({"path":path,"data":journal}))
    } else {
        None
    };
    rows.sort_by(|a, b| {
        b["verified"]
            .as_bool()
            .cmp(&a["verified"].as_bool())
            .then_with(|| {
                a["measurement"]["solve_seconds"]
                    .as_f64()
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(
                        &b["measurement"]["solve_seconds"]
                            .as_f64()
                            .unwrap_or(f64::INFINITY),
                    )
            })
            .then_with(|| a["identifier"].as_str().cmp(&b["identifier"].as_str()))
    });
    for (i, row) in rows.iter_mut().enumerate() {
        row["rank"] = json!(i + 1);
        row["display"] = json!({"solve":sig(row["measurement"]["solve_seconds"].as_f64(),3),"verify":sig(row["measurement"]["verify_seconds"].as_f64(),3),"rss":sig(row["measurement"]["peak_rss_mib"].as_f64(),3),"baseline_rust":sig(row["baseline"]["rust_seconds"].as_f64(),2),"baseline_native":sig(row["baseline"]["native_seconds"].as_f64(),2),"ratio":sig(row["baseline"]["ratio"].as_f64(),2),"incomplete":sig(row["longest_incomplete_seconds"].as_f64(),3)});
    }
    let result = json!({"schema_version":1,"title":"symGCAD — corpus benchmark","generator_source_sha256":hash(&fs::read(file!())?),"completion_summary":completion["summary"],"baseline_summary":parity["summary"],"rows":rows,"slow_campaign":slow,"sources":sources.hashes,"policies":{"ordering":"verified original problems fastest to slowest; unresolved last","measurement":"three-run verified control preferred; otherwise fastest audited full-problem campaign run; no per-repeat minimum","solve":"fresh process wall including in-run ordering/search, preprocessing and export; separately prepared research discovery is disclosed","memory":"maximum recorded resident memory across solve and independent verification","baseline_ratio":"ratio of sums of three-repeat median projection+lifting+merging times; independent +/- branch solves when marked2; not the optimized full-split solve"},"machine":{"cpu":"AMD EPYC 9754 128-Core Processor","os":"Linux 6.18.45 x86_64","task_cpu_limit":16,"task_memory_gib":64,"worker_threads":1,"historical_worker_limits":"4–32GiB; exact per-run ceilings retained in source ledgers"}});
    fs::create_dir_all(args.output.parent().context("output parent")?)?;
    fs::write(&args.output, serde_json::to_vec_pretty(&result)?)?;
    println!(
        "{} rows; {} verified; wrote {}",
        result["rows"].as_array().unwrap().len(),
        result["completion_summary"]["complete_verified"],
        args.output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn median_uses_all_repetitions() {
        assert_eq!(median(vec![30., 1., 20.]), Some(20.));
        assert_eq!(median(vec![]), None);
    }
    #[test]
    fn ratios_retain_two_significant_digits() {
        assert_eq!(sig(Some(0.146556), 2), "0.15");
        assert_eq!(sig(Some(1.0), 2), "1.0");
        assert_eq!(sig(Some(3.0738), 2), "3.1");
        assert_eq!(sig(None, 2), "—");
    }
    #[test]
    fn incomplete_branch_cannot_acquire_ratio() -> Result<()> {
        let c = json!({"reference_qualified":false,"verified_and_matched":false});
        let r = ratio_summary(&[&c], &mut Sources::new())?;
        assert!(r["ratio"].is_null());
        assert_eq!(r["status"], "native timeout");
        Ok(())
    }
}
