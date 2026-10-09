//! Completion ledgers and end-to-end ablations, separate from baseline parity.
use crate::{
    Limits, Problem,
    benchmark::{ProcessOutcome, run_bounded},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CampaignConfig {
    pub mode: String,
    pub cases: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub equivalence_manifests: Vec<PathBuf>,
    pub variants: Vec<Variant>,
    pub repeats: usize,
    pub solve_timeout_secs: f64,
    pub verify_timeout_secs: f64,
    pub campaign_time_secs: f64,
    pub memory_mib: u64,
    pub workers: usize,
    pub resume: bool,
    /// `auto` uses cold caches for ablations and shared checkpoints for completion.
    pub cache_policy: String,
}
impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            mode: "completion".into(),
            cases: vec![],
            output_dir: "artifacts/completion".into(),
            equivalence_manifests: vec![],
            variants: vec![Variant::default()],
            repeats: 3,
            solve_timeout_secs: 60.0,
            verify_timeout_secs: 60.0,
            campaign_time_secs: 3600.0,
            memory_mib: 8192,
            workers: 1,
            resume: true,
            cache_policy: "auto".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Variant {
    pub name: String,
    /// Partial solver options; unspecified fields retain each input's policy.
    pub solver: BTreeMap<String, toml::Value>,
}
impl Default for Variant {
    fn default() -> Self {
        Self {
            name: "input".into(),
            solver: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trial {
    pub repetition: usize,
    #[serde(default)]
    pub input_file: Option<PathBuf>,
    #[serde(default)]
    pub input_sha256: Option<String>,
    pub status: String,
    pub solve: ProcessOutcome,
    pub result_file: PathBuf,
    pub result_sha256: Option<String>,
    pub result_status: Option<String>,
    pub statistics: Option<Value>,
    pub verification: Option<ProcessOutcome>,
    pub verification_file: PathBuf,
    pub verification_sha256: Option<String>,
    pub verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseRun {
    pub identity_sha256: String,
    pub input_sha256: String,
    pub executable_sha256: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub equivalence_sha256: Option<String>,
    pub source: PathBuf,
    pub input_file: PathBuf,
    pub variant: String,
    pub mode: String,
    pub requested_repeats: usize,
    pub status: String,
    pub solve_median_secs: Option<f64>,
    pub peak_rss_kib: u64,
    pub trials: Vec<Trial>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledCase {
    pub identity_sha256: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub variant: String,
    pub status: String,
    pub resumed: bool,
    pub run_index: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionReport {
    pub schema_version: u32,
    pub mode: String,
    pub provenance: Value,
    pub scheduled: Vec<ScheduledCase>,
    /// Append-only attempt history: incomplete attempts survive retries.
    pub runs: Vec<CaseRun>,
    pub complete_verified: usize,
    pub resumed: usize,
    pub campaign_exhausted: bool,
    pub elapsed_secs: f64,
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let mut writer = BufWriter::new(fs::File::create(&temporary)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.flush()?;
    fs::rename(temporary, path)?;
    Ok(())
}
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 0 {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    } else {
        values[n / 2]
    })
}

fn fresh_attempt_directory(output: &Path, index: usize, identity: &str) -> Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    for collision in 0u64.. {
        let path = output.join(format!(
            "{index:05}-{}-{stamp}-{collision}",
            &identity[..16]
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!()
}

fn apply_variant(input: &Problem, variant: &Variant, limits: &Limits) -> Result<Problem> {
    let mut problem = input.clone();
    let mut options = serde_json::to_value(&problem.solver)?;
    for (key, value) in &variant.solver {
        // Unknown fields are rejected by SolverOptions' deserializer below.
        options
            .as_object_mut()
            .unwrap()
            .insert(key.clone(), serde_json::to_value(value)?);
    }
    problem.solver = serde_json::from_value(options)?;
    problem.limits = limits.clone();
    problem.validate()?;
    Ok(problem)
}

/// Give every cold measurement a fresh persistent-cache namespace, while
/// allowing its ordering trials and final solve to share completed work.
fn trial_input(input: &str, run_dir: &Path, repetition: usize, cold: bool) -> Result<PathBuf> {
    if !cold {
        return Ok(run_dir.join("input.toml"));
    }
    let trial_dir = run_dir.join(format!("{repetition}-cold"));
    fs::create_dir(&trial_dir)?;
    let mut problem = Problem::from_toml(input)?;
    // This also overrides an explicitly supplied warm checkpoint directory.
    // MCTS otherwise silently enables the shared artifacts/checkpoints path.
    if problem.solver.engine == "optimized" || problem.solver.checkpoint_dir.is_some() {
        problem.solver.checkpoint_dir = Some(trial_dir.join("checkpoints"));
    }
    let path = trial_dir.join("input.toml");
    fs::write(&path, toml::to_string_pretty(&problem)?)?;
    Ok(path)
}

#[derive(Deserialize)]
struct ResultSummary {
    status: String,
    #[serde(default)]
    statistics: Option<Value>,
}
fn result_summary(path: &Path) -> Result<ResultSummary> {
    // Unknown (potentially enormous) cell/trace arrays are skipped by serde.
    Ok(serde_json::from_reader(BufReader::new(fs::File::open(
        path,
    )?))?)
}

/// A verified row is reusable only while every input, executable, result and
/// verification artifact agrees with the recorded content hashes.
fn reusable(run: &CaseRun, identity: &str, binary_hash: &str) -> bool {
    if run.identity_sha256 != identity
        || run.executable_sha256 != binary_hash
        || run.status != "complete_verified"
        || run.trials.len() != run.requested_repeats
        || hash_file(&run.input_file).ok().as_ref() != Some(&run.input_sha256)
    {
        return false;
    }
    run.trials.iter().all(|trial| {
        trial.verified
            && match (&trial.input_file, &trial.input_sha256) {
                (Some(path), Some(hash)) => hash_file(path).ok().as_ref() == Some(hash),
                (None, None) => true,
                _ => false,
            }
            && trial.status == "complete_verified"
            && trial.result_sha256.is_some()
            && hash_file(&trial.result_file).ok() == trial.result_sha256
            && trial.verification_sha256.is_some()
            && hash_file(&trial.verification_file).ok() == trial.verification_sha256
            && fs::File::open(&trial.verification_file)
                .ok()
                .and_then(|file| serde_json::from_reader::<_, Value>(file).ok())
                .is_some_and(|value| value["verified"] == true)
    })
}

#[derive(Clone, Default)]
struct Aliases {
    names: Vec<String>,
    hash: Option<String>,
}
fn read_aliases(base: &Path, paths: &[PathBuf]) -> Result<BTreeMap<String, Aliases>> {
    let mut aliases = BTreeMap::new();
    for path in paths {
        let value: Value = serde_json::from_reader(fs::File::open(resolve(base, path))?)?;
        for group in value["groups"]
            .as_array()
            .context("equivalence manifest lacks groups")?
        {
            let names: Vec<String> = serde_json::from_value(group["identifiers"].clone())?;
            let entry = Aliases {
                names: names.clone(),
                hash: group["normalized_sha256"].as_str().map(str::to_owned),
            };
            for name in names {
                if let Some(previous) = aliases.insert(name.clone(), entry.clone()) {
                    ensure!(
                        previous.hash == entry.hash,
                        "conflicting equivalence groups for {name}"
                    );
                }
            }
        }
    }
    Ok(aliases)
}

pub fn run_campaign(config_path: &Path, executable: &Path) -> Result<CompletionReport> {
    let config: CampaignConfig = toml::from_str(&fs::read_to_string(config_path)?)?;
    ensure!(
        ["completion", "ablation"].contains(&config.mode.as_str()),
        "mode must be completion or ablation"
    );
    ensure!(
        !config.cases.is_empty() && !config.variants.is_empty(),
        "campaign needs cases and variants"
    );
    let names: BTreeSet<_> = config.variants.iter().map(|v| &v.name).collect();
    ensure!(
        names.len() == config.variants.len() && !names.contains(&String::new()),
        "variant names must be nonempty and unique"
    );
    ensure!(
        config.campaign_time_secs.is_finite()
            && config.campaign_time_secs > 0.0
            && config.campaign_time_secs <= 3600.0,
        "campaign budget must be in (0,3600]"
    );
    ensure!(
        config.solve_timeout_secs.is_finite()
            && config.solve_timeout_secs > 0.0
            && config.solve_timeout_secs <= 3600.0,
        "invalid solve timeout"
    );
    ensure!(
        config.verify_timeout_secs.is_finite()
            && config.verify_timeout_secs > 0.0
            && config.verify_timeout_secs <= 3600.0,
        "invalid verification timeout"
    );
    ensure!(
        config.memory_mib > 0
            && config.memory_mib <= 65536
            && config.workers > 0
            && config.workers <= 16,
        "invalid resources"
    );
    let repeats = if config.mode == "completion" {
        1
    } else {
        config.repeats
    };
    ensure!(repeats > 0, "repeats must be positive");
    ensure!(
        ["auto", "cold", "shared"].contains(&config.cache_policy.as_str()),
        "cache_policy must be auto, cold, or shared"
    );
    let cold = config.cache_policy == "cold"
        || (config.cache_policy == "auto" && config.mode == "ablation");
    let cache_policy = if cold { "cold" } else { "shared" };
    let base = config_path.canonicalize()?.parent().unwrap().to_owned();
    let output = resolve(&base, &config.output_dir);
    fs::create_dir_all(&output)?;
    let output = output.canonicalize()?;
    let executable = executable.canonicalize()?;
    let binary_hash = hash_file(&executable)?;
    // A developer may rebuild target/{debug,release}/symgcad while this
    // campaign runs. Freeze the executable so later trials use the same code.
    let binaries = output.join("executables");
    fs::create_dir_all(&binaries)?;
    let frozen_executable = binaries.join(format!("symgcad-{binary_hash}"));
    if hash_file(&frozen_executable).ok().as_ref() != Some(&binary_hash) {
        let temporary = binaries.join(format!("{binary_hash}.tmp"));
        fs::copy(&executable, &temporary)?;
        ensure!(
            hash_file(&temporary)? == binary_hash,
            "executable changed while freezing campaign"
        );
        fs::rename(temporary, &frozen_executable)?;
    }
    let aliases = read_aliases(&base, &config.equivalence_manifests)?;
    let limits = Limits {
        wall_time_secs: config.solve_timeout_secs,
        memory_mib: config.memory_mib,
        workers: config.workers,
        max_cells: None,
    };
    let ledger = output.join("ledger.json");
    let history = if config.resume && ledger.exists() {
        let prior: CompletionReport = serde_json::from_reader(fs::File::open(&ledger)?)?;
        ensure!(prior.schema_version == 1, "unsupported ledger schema");
        prior.runs
    } else {
        vec![]
    };
    let mut report = CompletionReport {
        schema_version: 1,
        mode: config.mode.clone(),
        provenance: serde_json::json!({
        "executable":executable,"frozen_executable":frozen_executable,"executable_sha256":binary_hash,"config_sha256":hash_file(config_path)?,
            "timing_policy":"fresh public solve process, including order search, preprocessing, normalization, solve and export; verification separate",
            "operation":"solve","workers":config.workers,"memory_mib":config.memory_mib,
            "cache_policy":cache_policy,
            "solve_timeout_secs":config.solve_timeout_secs,"verify_timeout_secs":config.verify_timeout_secs,
            "harness_profile":if cfg!(debug_assertions){"debug"}else{"release"},
            "worker_profile":"not inferred; identify the frozen executable by its content hash",
            "cpu_model":fs::read_to_string("/proc/cpuinfo").ok().and_then(|s|s.lines().find(|l|l.starts_with("model name")).map(str::to_owned))
        }),
        scheduled: vec![],
        runs: history,
        complete_verified: 0,
        resumed: 0,
        campaign_exhausted: false,
        elapsed_secs: 0.0,
    };
    let mut jobs = Vec::new();
    for path in &config.cases {
        let source = resolve(&base, path).canonicalize()?;
        let input = Problem::from_toml(&fs::read_to_string(&source)?)?;
        let name = input
            .name
            .clone()
            .unwrap_or_else(|| source.display().to_string());
        let mut alias = aliases.get(&name).cloned().unwrap_or_default();
        if alias.names.is_empty() {
            alias.names.push(name.clone());
        }
        for variant in &config.variants {
            let problem = apply_variant(&input, variant, &limits)
                .with_context(|| format!("{name}, variant {}", variant.name))?;
            let input_text = toml::to_string_pretty(&problem)?;
            let input_hash = hash_bytes(input_text.as_bytes());
            let identity = hash_bytes(&serde_json::to_vec(&serde_json::json!([
                input_hash,
                binary_hash,
                config.mode,
                variant.name,
                repeats,
                cache_policy
            ]))?);
            report.scheduled.push(ScheduledCase {
                identity_sha256: identity.clone(),
                name: name.clone(),
                aliases: alias.names.clone(),
                variant: variant.name.clone(),
                status: "not_started".into(),
                resumed: false,
                run_index: None,
            });
            jobs.push((
                source.clone(),
                name.clone(),
                variant.name.clone(),
                alias.clone(),
                input_text,
                input_hash,
                identity,
            ));
        }
    }
    write_json(&ledger, &report)?;
    let started = Instant::now();
    for (index, (source, name, variant, alias, input_text, input_hash, identity)) in
        jobs.into_iter().enumerate()
    {
        if let Some(prior) = report
            .runs
            .iter()
            .rposition(|run| reusable(run, &identity, &binary_hash))
        {
            report.scheduled[index].status = "complete_verified".into();
            report.scheduled[index].resumed = true;
            report.scheduled[index].run_index = Some(prior);
            report.resumed += 1;
            report.complete_verified += 1;
            write_json(&ledger, &report)?;
            continue;
        }
        let remaining = || (config.campaign_time_secs - started.elapsed().as_secs_f64()).max(0.0);
        if remaining() < 0.05 {
            report.campaign_exhausted = true;
            report.scheduled[index].status = "campaign_exhausted".into();
            continue;
        }
        let run_dir = fresh_attempt_directory(&output, report.runs.len(), &identity)?;
        let input_file = run_dir.join("input.toml");
        fs::write(&input_file, &input_text)?;
        let mut run = CaseRun {
            identity_sha256: identity,
            input_sha256: input_hash,
            executable_sha256: binary_hash.clone(),
            name,
            aliases: alias.names,
            equivalence_sha256: alias.hash,
            source,
            input_file,
            variant,
            mode: config.mode.clone(),
            requested_repeats: repeats,
            status: "running".into(),
            solve_median_secs: None,
            peak_rss_kib: 0,
            trials: vec![],
        };
        for repetition in 0..repeats {
            if remaining() < 0.05 {
                report.campaign_exhausted = true;
                run.status = "campaign_exhausted".into();
                break;
            }
            let result_file = run_dir.join(format!("{repetition}-result.json"));
            let verification_file = run_dir.join(format!("{repetition}-verification.json"));
            let input_file = trial_input(&input_text, &run_dir, repetition, cold)?;
            let mut command = Command::new(&frozen_executable);
            command
                .arg("solve")
                .arg(&input_file)
                .arg("--output")
                .arg(&result_file);
            let mut child_limits = limits.clone();
            child_limits.wall_time_secs = config.solve_timeout_secs.min(remaining());
            let solve = run_bounded(&mut command, &child_limits)?;
            run.peak_rss_kib = run.peak_rss_kib.max(solve.peak_rss_kib);
            let summary = result_summary(&result_file).ok();
            let mut trial = Trial {
                repetition,
                input_sha256: Some(hash_file(&input_file)?),
                input_file: Some(input_file),
                status: solve.status.clone(),
                solve,
                result_sha256: hash_file(&result_file).ok(),
                result_status: summary.as_ref().map(|s| s.status.clone()),
                statistics: summary.and_then(|s| s.statistics),
                result_file,
                verification: None,
                verification_file,
                verification_sha256: None,
                verified: false,
            };
            if trial.solve.status == "completed"
                && trial.result_status.as_deref() == Some("complete_generic")
            {
                if remaining() >= 0.05 {
                    let mut verify = Command::new(&frozen_executable);
                    verify
                        .arg("_worker")
                        .arg("verify")
                        .arg(&trial.result_file)
                        .arg("--output")
                        .arg(&trial.verification_file);
                    child_limits.wall_time_secs = config.verify_timeout_secs.min(remaining());
                    let outcome = run_bounded(&mut verify, &child_limits)?;
                    trial.verified = outcome.status == "completed"
                        && fs::File::open(&trial.verification_file)
                            .ok()
                            .and_then(|f| serde_json::from_reader::<_, Value>(f).ok())
                            .is_some_and(|v| v["verified"] == true);
                    trial.status = if trial.verified {
                        "complete_verified".into()
                    } else if outcome.status == "completed" {
                        "verification_failed".into()
                    } else {
                        format!("verification_{}", outcome.status)
                    };
                    run.peak_rss_kib = run.peak_rss_kib.max(outcome.peak_rss_kib);
                    trial.verification_sha256 = hash_file(&trial.verification_file).ok();
                    trial.verification = Some(outcome);
                } else {
                    report.campaign_exhausted = true;
                    trial.status = "verification_budget_exhausted".into();
                }
            } else if trial.solve.status == "completed" {
                trial.status = trial
                    .result_status
                    .clone()
                    .unwrap_or_else(|| "invalid_result".into());
            }
            let failed = !trial.verified;
            run.trials.push(trial);
            if failed {
                break;
            }
        }
        if run.trials.len() == repeats && run.trials.iter().all(|t| t.verified) {
            run.status = "complete_verified".into();
            run.solve_median_secs =
                median(run.trials.iter().map(|t| t.solve.wall_time_secs).collect());
            report.complete_verified += 1;
        } else if run.status != "campaign_exhausted" {
            run.status = run
                .trials
                .last()
                .map(|t| t.status.clone())
                .unwrap_or_else(|| "not_started".into());
        }
        report.scheduled[index].status = run.status.clone();
        report.scheduled[index].run_index = Some(report.runs.len());
        report.runs.push(run);
        report.elapsed_secs = started.elapsed().as_secs_f64();
        write_json(&ledger, &report)?;
    }
    report.elapsed_secs = started.elapsed().as_secs_f64();
    write_json(&ledger, &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_variant_preserves_order_policy_and_rejects_typos() {
        let mut problem = Problem::default();
        problem.variables = vec!["x".into()];
        problem.constraints = vec!["x > 0".into()];
        problem.solver.ordering = "mods".into();
        let mut variant = Variant::default();
        variant
            .solver
            .insert("engine".into(), toml::Value::String("optimized".into()));
        let changed = apply_variant(&problem, &variant, &Limits::default()).unwrap();
        assert_eq!(changed.solver.engine, "optimized");
        assert_eq!(changed.solver.ordering, "mods");
        variant
            .solver
            .insert("typo".into(), toml::Value::Boolean(true));
        assert!(apply_variant(&problem, &variant, &Limits::default()).is_err());
    }
    #[test]
    fn result_summary_skips_trace_and_cells() {
        let summary:ResultSummary=serde_json::from_str(r#"{"status":"complete_generic","cells":[{"large":"ignored"}],"statistics":{"raw_cells":1}}"#).unwrap();
        assert_eq!(summary.status, "complete_generic");
        assert_eq!(summary.statistics.unwrap()["raw_cells"], 1);
    }
    #[test]
    fn cold_measurements_isolate_explicit_and_automatic_checkpoints() {
        let root = std::env::temp_dir().join(format!(
            "symgcad-cold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut problem = Problem::default();
        problem.variables = vec!["x".into()];
        problem.constraints = vec!["x > 0".into()];
        problem.solver.engine = "optimized".into();
        problem.solver.ordering = "mcts".into();
        let text = toml::to_string_pretty(&problem).unwrap();
        fs::write(root.join("input.toml"), &text).unwrap();
        let first = trial_input(&text, &root, 0, true).unwrap();
        let first = Problem::from_toml(&fs::read_to_string(first).unwrap()).unwrap();
        let first_cache = first.solver.checkpoint_dir.unwrap();
        fs::create_dir_all(&first_cache).unwrap();
        fs::write(first_cache.join("existing.json"), "completed work").unwrap();
        problem.solver.checkpoint_dir = Some(first_cache.clone());
        let warm_text = toml::to_string_pretty(&problem).unwrap();
        let second = trial_input(&warm_text, &root, 1, true).unwrap();
        let second = Problem::from_toml(&fs::read_to_string(second).unwrap()).unwrap();
        let second_cache = second.solver.checkpoint_dir.unwrap();
        assert_ne!(first_cache, second_cache);
        assert!(!second_cache.exists());
        assert!(first_cache.join("existing.json").exists());
        assert_eq!(
            trial_input(&warm_text, &root, 2, false).unwrap(),
            root.join("input.toml")
        );
        assert!(!root.join("2-cold").exists());
        assert!(trial_input(&text, &root, 0, true).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn resume_requires_unchanged_input_binary_and_verified_artifacts() {
        let dir = std::env::temp_dir().join(format!(
            "symgcad-ledger-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.toml");
        let result = dir.join("result.json");
        let proof = dir.join("verification.json");
        fs::write(&input, "variables = []\n").unwrap();
        fs::write(&result, r#"{"status":"complete_generic"}"#).unwrap();
        fs::write(&proof, r#"{"verified":true}"#).unwrap();
        let outcome = ProcessOutcome {
            status: "completed".into(),
            wall_time_secs: 0.1,
            user_cpu_secs: 0.05,
            system_cpu_secs: 0.01,
            peak_rss_kib: 1,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            output_truncated: false,
        };
        let trial = Trial {
            repetition: 0,
            input_file: Some(input.clone()),
            input_sha256: Some(hash_file(&input).unwrap()),
            status: "complete_verified".into(),
            solve: outcome.clone(),
            result_file: result.clone(),
            result_sha256: Some(hash_file(&result).unwrap()),
            result_status: Some("complete_generic".into()),
            statistics: None,
            verification: Some(outcome),
            verification_file: proof.clone(),
            verification_sha256: Some(hash_file(&proof).unwrap()),
            verified: true,
        };
        let run = CaseRun {
            identity_sha256: "identity".into(),
            input_sha256: hash_file(&input).unwrap(),
            executable_sha256: "binary".into(),
            name: "case".into(),
            aliases: vec![],
            equivalence_sha256: None,
            source: input.clone(),
            input_file: input.clone(),
            variant: "input".into(),
            mode: "completion".into(),
            requested_repeats: 1,
            status: "complete_verified".into(),
            solve_median_secs: Some(0.1),
            peak_rss_kib: 1,
            trials: vec![trial],
        };
        assert!(reusable(&run, "identity", "binary"));
        assert!(!reusable(&run, "identity", "new-binary"));
        assert!(!reusable(&run, "other-input", "binary"));
        fs::write(&proof, r#"{"verified":false}"#).unwrap();
        assert!(!reusable(&run, "identity", "binary"));
        fs::write(&proof, r#"{"verified":true}"#).unwrap();
        fs::write(&result, r#"{"status":"incomplete"}"#).unwrap();
        assert!(!reusable(&run, "identity", "binary"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fresh_runs_cannot_reuse_stale_outputs_when_history_is_reset() {
        let parent = std::env::temp_dir().join(format!(
            "symgcad-attempts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&parent).unwrap();
        let identity = "0123456789abcdef0123456789abcdef";
        let first = fresh_attempt_directory(&parent, 0, identity).unwrap();
        fs::write(
            first.join("0-result.json"),
            r#"{"status":"complete_generic"}"#,
        )
        .unwrap();
        // resume=false resets history length to zero with the same input hash.
        let retry = fresh_attempt_directory(&parent, 0, identity).unwrap();
        assert_ne!(first, retry);
        assert!(!retry.join("0-result.json").exists());
        assert!(first.join("0-result.json").exists());
        fs::remove_dir_all(parent).unwrap();
    }
}
