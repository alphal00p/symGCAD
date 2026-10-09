//! Audit corpus coverage and aggregate one immutable worker's parity reports.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "artifacts/corpus/fixed/equivalence.json")]
    fixed_manifest: PathBuf,
    #[arg(long, default_value = "artifacts/corpus/symbolic/equivalence.json")]
    symbolic_manifest: PathBuf,
    #[arg(long = "report", required = true)]
    reports: Vec<PathBuf>,
    #[arg(long, default_value = "artifacts/bench/parity-aggregate.json")]
    output: PathBuf,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn positive_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|x| x.is_finite() && *x > 0.0)
}
fn main() -> Result<()> {
    let args = Args::parse();
    let mut names = BTreeMap::<String, String>::new();
    let mut expected = BTreeMap::<String, Value>::new();
    let mut manifests = Vec::new();
    let mut expected_groups = 0;
    for (scope, path) in [
        ("fixed", &args.fixed_manifest),
        ("symbolic_core", &args.symbolic_manifest),
    ] {
        let bytes = fs::read(path)?;
        let manifest: Value = serde_json::from_slice(&bytes)?;
        let groups = manifest["groups"]
            .as_array()
            .context("equivalence groups missing")?;
        expected_groups += groups.len();
        manifests.push(json!({"path":path,"sha256":digest(&bytes),"groups":groups.len()}));
        for (index, group) in groups.iter().enumerate() {
            let aliases: Vec<String> = serde_json::from_value(group["identifiers"].clone())?;
            ensure!(!aliases.is_empty(), "empty equivalence group");
            let fixture = aliases[0].ends_with("/fixed") || aliases[0].ends_with("/symbolic");
            ensure!(
                aliases
                    .iter()
                    .all(|s| (s.ends_with("/fixed") || s.ends_with("/symbolic")) == fixture),
                "mixed fixture/API group"
            );
            // This imported corpus has one split polynomial per ZZ fixture and
            // no split polynomial in its core API inputs. Preserve both signs.
            let branches: &[&str] = if fixture { &["-", "+"] } else { &[""] };
            for branch in branches {
                let key = format!("{scope}:{index}:{branch}");
                let branch_names: Vec<String> = aliases
                    .iter()
                    .map(|name| {
                        if branch.is_empty() {
                            name.clone()
                        } else {
                            format!("{name}/signs={branch}")
                        }
                    })
                    .collect();
                for name in &branch_names {
                    ensure!(
                        names.insert(name.clone(), key.clone()).is_none(),
                        "ambiguous expected identifier {name}"
                    );
                }
                expected.insert(key.clone(), json!({"key":key,"identifier":branch_names[0],"aliases":branch_names,"normalized_sha256":group["normalized_sha256"],"scope":scope,"branch":branch}));
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut worker_hash: Option<String> = None;
    let mut qualified = 0;
    let mut verified = 0;
    let mut ratios = Vec::new();
    let mut walls = Vec::new();
    let mut rows = Vec::new();
    let mut report_sources = Vec::new();
    let mut incomplete_native = BTreeMap::<String, usize>::new();
    let mut all_qualification_attempted = true;
    for path in &args.reports {
        let bytes = fs::read(path)?;
        let report: Value = serde_json::from_slice(&bytes)?;
        let hash = report["provenance"]["executable_sha256"]
            .as_str()
            .context("report lacks immutable worker hash")?;
        ensure!(
            worker_hash
                .as_deref()
                .is_none_or(|previous| previous == hash),
            "reports use different worker executables"
        );
        worker_hash = Some(hash.into());
        ensure!(report["repeats"] == 3, "parity requires three repetitions");
        report_sources.push(json!({"path":path,"sha256":digest(&bytes)}));
        for case in report["cases"].as_array().context("report cases missing")? {
            let name = case["name"].as_str().context("case identifier missing")?;
            let key = names
                .get(name)
                .with_context(|| format!("unexpected corpus case {name}"))?;
            ensure!(
                seen.insert(key.clone()),
                "duplicate equivalence group/branch {name}"
            );
            let mut row = expected[key].clone();
            row["measured_identifier"] = json!(name);
            row["report"] = json!(path);
            row["status"] = case["status"].clone();
            let reference_complete = case["reference_qualified"]
                .as_bool()
                .context("qualification status missing")?;
            row["reference_qualified"] = json!(reference_complete);
            if reference_complete {
                qualified += 1;
                let checked = case["status"] == "complete"
                    && case["verified"] == true
                    && case["raw_cells_match"] == true
                    && case["merged_cells_match"] != false;
                verified += usize::from(checked);
                row["verified_and_matched"] = json!(checked);
                if let (Some(native), Some(rust)) = (
                    positive_number(&case["reference_solver_median_secs"]),
                    positive_number(&case["rust_solver_median_secs"]),
                ) {
                    let ratio = rust / native;
                    ensure!(
                        positive_number(&case["solver_ratio"])
                            .is_some_and(|stored| (stored - ratio).abs() <= 1e-10 * ratio.max(1.0)),
                        "inconsistent phase ratio for {name}"
                    );
                    ratios.push(ratio);
                    row["solver_ratio"] = json!(ratio);
                }
                if let (Some(native), Some(rust)) = (
                    positive_number(&case["reference_median_secs"]),
                    positive_number(&case["rust_median_secs"]),
                ) {
                    walls.push(rust / native);
                }
            } else {
                let status = case["status"].as_str().unwrap_or("missing_status");
                *incomplete_native.entry(status.into()).or_default() += 1;
                all_qualification_attempted &= status.starts_with("reference_");
            }
            rows.push(row);
        }
    }
    let missing: Vec<_> = expected
        .iter()
        .filter(|(key, _)| !seen.contains(*key))
        .map(|(_, value)| value.clone())
        .collect();
    let geo = |values: &[f64]| {
        (!values.is_empty() && values.len() == qualified)
            .then(|| (values.iter().map(|x| x.ln()).sum::<f64>() / values.len() as f64).exp())
    };
    let complete_coverage = missing.is_empty() && all_qualification_attempted;
    let solver_geo = complete_coverage.then(|| geo(&ratios)).flatten();
    let wall_geo = complete_coverage.then(|| geo(&walls)).flatten();
    let outliers: Vec<_> = rows
        .iter()
        .filter(|r| r["solver_ratio"].as_f64().is_some_and(|x| x > 2.0))
        .cloned()
        .collect();
    let passed =
        complete_coverage && verified == qualified && solver_geo.is_some_and(|x| x <= 1.25);
    let summary = json!({"expected_equivalence_groups":expected_groups,"expected_sign_expanded_cases":expected.len(),"observed_cases":seen.len(),"missing_cases":missing.len(),"qualification_coverage_complete":complete_coverage,"reference_complete_cases":qualified,"verified_matching_cases":verified,"reference_incomplete":incomplete_native,"geometric_mean_solver_ratio":solver_geo,"geometric_mean_end_to_end_ratio":wall_geo,"outliers_above_2":outliers.len(),"parity_target":1.25,"parity_passed":passed});
    let output = json!({"schema_version":1,"summary":summary,"executable_sha256":worker_hash,"equivalence_manifests":manifests,"reports":report_sources,"cases":rows,"missing":missing,"outliers_above_2":outliers,"scope":"Original solve cases plus the thirteen imported ordering-only problems as supplemental solves. Original ordering/projection API assertions and aggressive-merge replays are separate reports. Native timeouts remain unresolved and do not count as Rust completions."});
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_vec_pretty(&output)?)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
