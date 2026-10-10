//! Resumable, bounded queue for the classified unresolved original corpus.
//! Solve ledgers retain the standard campaign schema; late verification is a
//! separate bounded worker and supplemental report, never an edited proof.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};
use symgcad::{
    Limits, Problem,
    benchmark::run_bounded,
    campaign::{CampaignConfig, CompletionReport, Variant, run_campaign},
    domain::{derive_domain, homogeneous_anchor},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "tests/corpus_classification.json")]
    classification: PathBuf,
    /// Read-only evidence audit of an existing queue; --output must be fresh.
    #[arg(long)]
    audit_queue: Option<PathBuf>,
    /// Optional additional audit against the private completion aggregate.
    #[arg(long)]
    aggregate: Option<PathBuf>,
    /// Root of a regenerated corpus containing fixed/ and symbolic/ directories.
    #[arg(long)]
    corpus_root: Option<PathBuf>,
    /// Portable solver recipes bound to exact classified mathematics.
    #[arg(long, default_value = "tests/slow_options.json")]
    options: PathBuf,
    /// Export the selected solver recipes (requires --prepare-only).
    #[arg(long)]
    write_options: Option<PathBuf>,
    /// Immutable solver executable. Required for execution, optional for preparation.
    #[arg(long)]
    executable: Option<PathBuf>,
    #[arg(long, default_value = "artifacts/slow-suite-one-hour-01")]
    output: PathBuf,
    /// JSON object mapping an original identifier to a mathematically identical TOML/JSON input.
    #[arg(long)]
    overrides: Option<PathBuf>,
    /// Original identifiers to run first; all other slow cases still follow.
    #[arg(long, value_delimiter = ',')]
    priority: Vec<String>,
    #[arg(long, value_delimiter = ',', default_value = "0,1,2,3")]
    cpus: Vec<usize>,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
    #[arg(long, default_value_t = 600.0)]
    verification_seconds: f64,
    #[arg(long)]
    prepare_only: bool,
}

#[derive(Clone, Deserialize)]
struct Classification {
    schema_version: u32,
    source_aggregate_sha256: String,
    slow_solve_timeout_secs: f64,
    cases: Vec<Classified>,
}
#[derive(Clone, Deserialize)]
struct Classified {
    identifier: String,
    aliases: Vec<String>,
    mathematical_sha256: String,
    original_input: PathBuf,
    original_input_sha256: String,
    classification: String,
}
#[derive(Serialize, Deserialize)]
struct Recipes {
    schema_version: u32,
    priority: Vec<String>,
    recipes: BTreeMap<String, Recipe>,
}
#[derive(Serialize, Deserialize)]
struct Recipe {
    mathematical_sha256: String,
    original_name: Option<String>,
    original_fixed_order: Vec<String>,
    solver: symgcad::config::SolverOptions,
}
#[derive(Clone, Serialize, Deserialize)]
struct Prepared {
    identifier: String,
    aliases: Vec<String>,
    original_input: PathBuf,
    original_input_sha256: String,
    #[serde(default)]
    historical_original_input_sha256: String,
    #[serde(default)]
    source_serialization_changed: bool,
    mathematical_sha256: String,
    prepared_input: PathBuf,
    prepared_input_sha256: String,
    config: PathBuf,
    config_sha256: String,
    ledger: PathBuf,
    selection: Value,
}
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    identifier: String,
    status: String,
    #[serde(default)]
    verdict: Option<String>,
    cpu: Option<usize>,
    ledger: PathBuf,
    ledger_sha256: Option<String>,
    supplemental: Option<PathBuf>,
    error: Option<String>,
    wall_time_secs: Option<f64>,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    schema_version: u32,
    classification_sha256: String,
    aggregate_sha256: String,
    prepared_sha256: String,
    executable: PathBuf,
    executable_sha256: String,
    cpus: Vec<usize>,
    memory_mib: u64,
    solve_seconds: f64,
    verification_seconds: f64,
    entries: Vec<Entry>,
}

fn hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}
fn math_key(p: &Problem) -> Result<String> {
    let n = normalize(p, &p.variables)?;
    let mut constraints = n
        .positives
        .iter()
        .map(|p| n.algebra.format(p))
        .collect::<Vec<_>>();
    constraints.sort();
    constraints.dedup();
    let split = n
        .split
        .iter()
        .map(|p| n.algebra.format(p))
        .collect::<Vec<_>>();
    Ok(serde_json::to_string(
        &json!({"variables":p.variables,"variable_groups":p.variable_groups,"constraints":constraints,"split":split}),
    )?)
}
fn math_hash(p: &Problem) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(math_key(p)?.as_bytes())))
}

fn validate_resources(args: &Args) -> Result<()> {
    ensure!(
        !args.cpus.is_empty() && args.cpus.len() <= 16,
        "one to16 CPU slots required"
    );
    ensure!(
        args.cpus.iter().copied().collect::<BTreeSet<_>>().len() == args.cpus.len(),
        "duplicate CPU slot"
    );
    ensure!(
        args.cpus.iter().all(|&v| v < libc::CPU_SETSIZE as usize),
        "CPU index too large"
    );
    ensure!(
        args.memory_mib > 0
            && args
                .memory_mib
                .checked_mul(args.cpus.len() as u64)
                .is_some_and(|m| m <= 65536),
        "aggregate worker memory exceeds64GiB"
    );
    ensure!(
        args.verification_seconds.is_finite()
            && args.verification_seconds > 0.
            && args.verification_seconds <= 3600.,
        "invalid verification budget"
    );
    Ok(())
}
fn selected(classification: &Classification, aggregate: Option<&Value>) -> Result<Vec<Classified>> {
    ensure!(
        classification.schema_version == 1 && classification.slow_solve_timeout_secs == 3600.,
        "unsupported classification policy"
    );
    ensure!(
        classification.cases.len() == 331,
        "classification must cover331 distinct originals"
    );
    let mut seen = BTreeSet::new();
    for c in &classification.cases {
        ensure!(seen.insert(&c.identifier), "duplicate classified identity");
        ensure!(
            ["slow", "verified"].contains(&c.classification.as_str()),
            "invalid classification"
        );
    }
    let slow = classification
        .cases
        .iter()
        .filter(|c| c.classification == "slow")
        .cloned()
        .collect::<Vec<_>>();
    let Some(aggregate) = aggregate else {
        return Ok(slow);
    };
    let unresolved = aggregate["unresolved"]
        .as_array()
        .context("missing unresolved inventory")?;
    ensure!(
        slow.len() == unresolved.len(),
        "classification/aggregate inventory size mismatch"
    );
    for c in &slow {
        let matches = unresolved
            .iter()
            .filter(|r| r["identifier"] == c.identifier)
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1
                && matches[0]["mathematical_sha256"] == c.mathematical_sha256
                && matches[0]["original_input_sha256"] == c.original_input_sha256,
            "classification/aggregate identity mismatch"
        );
    }
    Ok(slow)
}

fn apply_recipe(source: &Problem, c: &Classified, recipe: &Recipe) -> Result<Problem> {
    ensure!(
        recipe.mathematical_sha256 == c.mathematical_sha256
            && math_hash(source)? == recipe.mathematical_sha256,
        "recipe mathematics mismatch: {}",
        c.identifier
    );
    ensure!(
        source.name == recipe.original_name && source.solver.order == recipe.original_fixed_order,
        "regenerated original name/fixed order differs from recorded source"
    );
    ensure!(
        recipe.solver.checkpoint_dir.is_none(),
        "portable recipe contains a checkpoint path"
    );
    ensure!(
        source.solver.order.is_empty() || recipe.solver.order == source.solver.order,
        "recipe changes original fixed order"
    );
    let mut p = source.clone();
    p.solver = recipe.solver.clone();
    p.validate()?;
    if let Some(anchor) = &p.solver.homogeneous_anchor {
        let n = normalize(&p, &p.variables)?;
        let domain = derive_domain(&n.algebra, &n.positives, &[])?;
        let index = |name: &String| {
            p.variables
                .iter()
                .position(|v| v == name)
                .context("unknown homogeneous axis")
        };
        let block = p
            .solver
            .homogeneous_block
            .iter()
            .map(index)
            .collect::<Result<Vec<_>>>()?;
        let all = n
            .positives
            .iter()
            .chain(&n.split)
            .cloned()
            .collect::<Vec<_>>();
        ensure!(
            homogeneous_anchor(&n.algebra, &all, &block, index(anchor)?, &domain)?.is_some(),
            "recipe homogeneous reduction does not hold on original domain"
        );
    }
    Ok(p)
}

fn generic_options(p: &mut Problem) -> Result<Value> {
    let explicit = p.solver.order.clone();
    p.solver = symgcad::config::SolverOptions::default();
    p.solver.engine = "local".into();
    p.solver.ordering = if explicit.is_empty() { "mcts" } else { "given" }.into();
    p.solver.order = explicit;
    p.solver.positivity = true;
    p.solver.local_adjacent_pairs = true;
    p.solver.local_descartes_witnesses = true;
    p.solver.local_descartes_intervals = true;
    p.solver.local_positive_pair_witnesses = true;
    p.solver.search_local_progress = true;
    p.solver.search_time_secs = Some(300.);
    p.solver.max_rollouts = 256;
    p.solver.seed = 0;
    let normal = normalize(p, &p.variables)?;
    let domain = derive_domain(&normal.algebra, &normal.positives, &[])?;
    let final_group = p.variable_groups.last().unwrap_or(&p.variables);
    let block = (0..p.variables.len())
        .filter(|&i| {
            final_group.contains(&p.variables[i])
                && p.variables[i].starts_with('x')
                && domain.bounds[i].lower.as_deref() == Some("0")
                && domain.bounds[i].upper.is_none()
        })
        .collect::<Vec<_>>();
    let all = normal
        .positives
        .iter()
        .chain(&normal.split)
        .cloned()
        .collect::<Vec<_>>();
    let mut anchors = block.clone();
    anchors.sort_by_key(|&i| (all.iter().map(|p| u64::from(p.degree(i))).sum::<u64>(), i));
    for anchor in anchors {
        if !p.solver.order.is_empty()
            && p.solver
                .order
                .iter()
                .find(|v| block.iter().any(|&i| p.variables[i] == **v))
                != Some(&p.variables[anchor])
        {
            continue;
        }
        if homogeneous_anchor(&normal.algebra, &all, &block, anchor, &domain)?.is_some() {
            p.solver.homogeneous_anchor = Some(p.variables[anchor].clone());
            p.solver.homogeneous_block = block.iter().map(|&i| p.variables[i].clone()).collect();
            break;
        }
    }
    Ok(
        json!({"method":"local MCTS with exact generic-open witness rules; lowest input-degree legal certified homogeneous anchor",
        "anchor":p.solver.homogeneous_anchor,"homogeneous_block":p.solver.homogeneous_block,"search_seconds":p.solver.search_time_secs}),
    )
}

fn prepare(
    args: &Args,
    rows: &[Classified],
    output: &Path,
    recipes: Option<&Recipes>,
) -> Result<Vec<Prepared>> {
    let overrides: BTreeMap<String, PathBuf> = args
        .overrides
        .as_ref()
        .map(|p| -> Result<_> { Ok(serde_json::from_slice(&fs::read(p)?)?) })
        .transpose()?
        .unwrap_or_default();
    ensure!(
        overrides
            .keys()
            .all(|id| rows.iter().any(|r| &r.identifier == id)),
        "override names an unclassified case"
    );
    let mut prepared = Vec::new();
    for (index, c) in rows.iter().enumerate() {
        let original = if let Some(root) = &args.corpus_root {
            root.join(
                c.original_input
                    .strip_prefix("artifacts/corpus")
                    .context("classified source is outside the portable corpus layout")?,
            )
        } else {
            c.original_input.clone()
        }
        .canonicalize()?;
        let actual_source_hash = hash(&original)?;
        let source = Problem::from_toml(&fs::read_to_string(&original)?)?;
        ensure!(
            math_hash(&source)? == c.mathematical_sha256,
            "original mathematics mismatch"
        );
        if let Some(book) = recipes {
            // Name and explicit fixed order are not part of the normalized
            // polynomial key. They must remain bound even if TOML defaults change.
            let recipe = book
                .recipes
                .get(&c.identifier)
                .context("missing classified slow-case recipe")?;
            ensure!(
                source.name == recipe.original_name
                    && source.solver.order == recipe.original_fixed_order,
                "regenerated original name/fixed order mismatch"
            );
        } else {
            ensure!(
                actual_source_hash == c.original_input_sha256,
                "recipe export requires the historically hashed original source"
            );
        }
        let mut p = source.clone();
        let selection = if let Some(path) = overrides.get(&c.identifier) {
            let text = fs::read_to_string(path)?;
            let candidate: Problem = if path.extension().is_some_and(|v| v == "json") {
                serde_json::from_str(&text)?
            } else {
                Problem::from_toml(&text)?
            };
            ensure!(
                math_key(&candidate)? == math_key(&source)?,
                "override changes original domain/signs/groups: {}",
                c.identifier
            );
            ensure!(
                source.solver.order.is_empty() || candidate.solver.order == source.solver.order,
                "override changes fixed order"
            );
            p.solver = candidate.solver;
            json!({"method":"previous full-original prepared solver options","path":path.canonicalize()?,"sha256":hash(path)?})
        } else if let Some(book) = recipes {
            let recipe = book
                .recipes
                .get(&c.identifier)
                .context("missing classified slow-case recipe")?;
            p = apply_recipe(&source, c, recipe)?;
            json!({"method":"tracked original-problem solver recipe","options_sha256":hash(&args.options)?,"mathematical_sha256":recipe.mathematical_sha256})
        } else {
            generic_options(&mut p)?
        };
        p.solver.merge = "none".into();
        p.solver.local_max_refinements = Some(20_000_000);
        p.solver.local_max_nodes = Some(200_000_000);
        p.solver.local_hint_archive = false;
        let dir = output.join(format!("{index:02}"));
        fs::create_dir_all(&dir)?;
        // Explicit namespace causes campaign's cold policy to allocate a fresh
        // checkpoint tree even for local MCTS, whose implicit default is shared.
        p.solver.checkpoint_dir = Some(dir.join("checkpoints"));
        p.limits = Limits {
            wall_time_secs: 3600.,
            memory_mib: args.memory_mib,
            workers: 1,
            max_cells: None,
        };
        p.validate()?;
        ensure!(
            p.name == source.name
                && p.variables == source.variables
                && p.constraints == source.constraints
                && p.split == source.split
                && p.parameters == source.parameters
                && p.variable_groups == source.variable_groups,
            "preparation changed mathematical source fields"
        );
        let input = dir.join("input.toml");
        let text = toml::to_string_pretty(&p)?;
        if input.exists() {
            ensure!(
                fs::read_to_string(&input)? == text,
                "existing prepared input differs; use a fresh output directory"
            );
        } else {
            fs::write(&input, text)?;
        }
        let config = CampaignConfig {
            mode: "completion".into(),
            cases: vec![input.clone()],
            output_dir: dir.join("results"),
            equivalence_manifests: vec![
                original
                    .parent()
                    .context("source lacks corpus directory")?
                    .join("equivalence.json")
                    .canonicalize()?,
            ],
            variants: vec![Variant::default()],
            repeats: 1,
            solve_timeout_secs: 3600.,
            verify_timeout_secs: args.verification_seconds,
            campaign_time_secs: 3600.,
            memory_mib: args.memory_mib,
            workers: 1,
            resume: true,
            cache_policy: "cold".into(),
        };
        let path = dir.join("campaign.toml");
        let text = toml::to_string_pretty(&config)?;
        if path.exists() {
            ensure!(
                fs::read_to_string(&path)? == text,
                "existing campaign differs"
            );
        } else {
            fs::write(&path, text)?;
        }
        prepared.push(Prepared {
            identifier: c.identifier.clone(),
            aliases: c.aliases.clone(),
            original_input: original,
            original_input_sha256: actual_source_hash.clone(),
            historical_original_input_sha256: c.original_input_sha256.clone(),
            source_serialization_changed: actual_source_hash != c.original_input_sha256,
            mathematical_sha256: c.mathematical_sha256.clone(),
            prepared_input: input.clone(),
            prepared_input_sha256: hash(&input)?,
            config: path.clone(),
            config_sha256: hash(&path)?,
            ledger: dir.join("results/ledger.json"),
            selection,
        });
    }
    Ok(prepared)
}

fn pin(cpu: usize) -> Result<()> {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(cpu, &mut set);
        ensure!(
            libc::sched_setaffinity(0, std::mem::size_of_val(&set), &set) == 0,
            "CPU pin failed: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

fn late_verify(
    report: &CompletionReport,
    exe: &Path,
    dir: &Path,
    seconds: f64,
    memory: u64,
) -> Result<Option<PathBuf>> {
    let Some(trial) = report
        .runs
        .last()
        .and_then(|r| r.trials.last())
        .filter(|t| !t.verified && t.result_status.as_deref() == Some("complete_generic"))
    else {
        return Ok(None);
    };
    let result = &trial.result_file;
    let before = hash(result)?;
    let mut i = 0;
    let out = loop {
        let p = dir.join(format!("independent-verification-{i}"));
        match fs::create_dir(&p) {
            Ok(()) => break p,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => i += 1,
            Err(e) => return Err(e.into()),
        }
    };
    let verification = out.join("verification.json");
    let mut command = Command::new(exe);
    command
        .args(["_worker", "verify"])
        .arg(result)
        .arg("--output")
        .arg(&verification)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let process = run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: seconds,
            memory_mib: memory,
            workers: 1,
            max_cells: None,
        },
    )?;
    atomic(&out.join("process.json"), &process)?;
    let check: Option<Value> = fs::File::open(&verification)
        .ok()
        .and_then(|f| serde_json::from_reader(f).ok());
    let same = hash(result)? == before;
    let verified = same
        && process.status == "completed"
        && process.exit_code == Some(0)
        && check.as_ref().is_some_and(|v| v["verified"] == true);
    let path = out.join("supplemental.json");
    atomic(
        &path,
        &json!({"executable_sha256":hash(exe)?,"independent":{
        "result":result,"result_sha256":before,"verification":verification,"verification_sha256":hash(&verification).ok(),
        "proof_unchanged":same,"verified":verified,"process":process}}),
    )?;
    Ok(Some(path))
}

fn count_cells<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<usize, D::Error> {
    struct Counter;
    impl<'de> serde::de::Visitor<'de> for Counter {
        type Value = usize;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("cell array")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut a: A,
        ) -> std::result::Result<usize, A::Error> {
            let mut n = 0;
            while a.next_element::<serde::de::IgnoredAny>()?.is_some() {
                n += 1;
            }
            Ok(n)
        }
    }
    d.deserialize_seq(Counter)
}
#[derive(Deserialize)]
struct ProofHeader {
    status: String,
    problem: Problem,
    order: Vec<String>,
    #[serde(deserialize_with = "count_cells")]
    cells: usize,
}
fn proof_and_check(
    result: &Path,
    result_hash: &str,
    verification: &Path,
    verification_hash: &str,
    mathematical_hash: &str,
    original: &Problem,
) -> Result<bool> {
    ensure!(hash(result)? == result_hash, "result hash mismatch");
    ensure!(
        hash(verification)? == verification_hash,
        "verification hash mismatch"
    );
    let check: Value = serde_json::from_reader(fs::File::open(verification)?)?;
    if check["verified"] != true {
        return Ok(false);
    }
    let proof: ProofHeader =
        serde_json::from_reader(std::io::BufReader::new(fs::File::open(result)?))?;
    ensure!(
        proof.status == "complete_generic",
        "verification refers to incomplete result"
    );
    ensure!(
        math_hash(&proof.problem)? == mathematical_hash && proof.problem.name == original.name,
        "verified proof does not cover the original problem"
    );
    let mut ordered = proof.problem.clone();
    ordered.solver.order = proof.order.clone();
    ordered.validate()?;
    ensure!(
        proof.order.len() == original.variables.len()
            && (original.solver.order.is_empty() || original.solver.order == proof.order),
        "verified proof changes the original fixed order"
    );
    ensure!(
        check["cells"].as_u64() == Some(proof.cells as u64),
        "verification cell count mismatch"
    );
    Ok(true)
}
fn audit_verdict(prepared: &Prepared, entry: &Entry, executable_hash: &str) -> Result<String> {
    if entry.status != "finished" {
        return Ok("infrastructure_failed".into());
    }
    ensure!(
        entry.ledger_sha256.as_deref() == Some(hash(&entry.ledger)?.as_str()),
        "ledger hash mismatch"
    );
    let report: CompletionReport = serde_json::from_reader(fs::File::open(&entry.ledger)?)?;
    let run = report.runs.last().context("campaign has no attempt")?;
    ensure!(
        run.executable_sha256 == executable_hash,
        "ledger executable mismatch"
    );
    let Some(trial) = run.trials.last() else {
        return Ok("incomplete".into());
    };
    if trial.result_status.as_deref() != Some("complete_generic") {
        ensure!(
            !trial.verified,
            "ledger claims verification of incomplete result"
        );
        return Ok("incomplete".into());
    }
    let original = Problem::from_toml(&fs::read_to_string(&prepared.original_input)?)?;
    ensure!(
        hash(&prepared.original_input)? == prepared.original_input_sha256,
        "original input changed after preparation"
    );
    let input = trial.input_file.as_ref().context("missing trial input")?;
    ensure!(
        trial.input_sha256.as_deref() == Some(hash(input)?.as_str()),
        "trial input hash mismatch"
    );
    let input_problem = Problem::from_toml(&fs::read_to_string(input)?)?;
    ensure!(
        math_hash(&input_problem)? == prepared.mathematical_sha256
            && input_problem.name == original.name
            && (original.solver.order.is_empty()
                || input_problem.solver.order == original.solver.order),
        "trial changes original mathematics or fixed order"
    );
    let result_hash = trial
        .result_sha256
        .as_deref()
        .context("missing result hash")?;
    if trial.verified {
        ensure!(
            trial
                .verification
                .as_ref()
                .is_some_and(|p| p.status == "completed" && p.exit_code == Some(0)),
            "successful verification process missing"
        );
        if proof_and_check(
            &trial.result_file,
            result_hash,
            &trial.verification_file,
            trial
                .verification_sha256
                .as_deref()
                .context("missing verification hash")?,
            &prepared.mathematical_sha256,
            &original,
        )? {
            return Ok("verified".into());
        }
    }
    if let Some(path) = &entry.supplemental {
        let v: Value = serde_json::from_reader(fs::File::open(path)?)?;
        let s = &v["independent"];
        if s["verified"] == true {
            ensure!(
                v["executable_sha256"] == executable_hash
                    && s["proof_unchanged"] == true
                    && s["process"]["status"] == "completed"
                    && s["process"]["exit_code"] == 0
                    && s["result"].as_str().map(Path::new) == Some(trial.result_file.as_path())
                    && s["result_sha256"] == result_hash,
                "supplemental provenance mismatch"
            );
            if proof_and_check(
                &trial.result_file,
                result_hash,
                Path::new(
                    s["verification"]
                        .as_str()
                        .context("missing supplemental verification")?,
                ),
                s["verification_sha256"]
                    .as_str()
                    .context("missing supplemental verification hash")?,
                &prepared.mathematical_sha256,
                &original,
            )? {
                return Ok("verified".into());
            }
        }
    }
    Ok("incomplete".into())
}

fn audit_saved_queue(args: &Args, path: &Path) -> Result<()> {
    ensure!(
        !args.output.exists(),
        "read-only audit requires a fresh --output directory"
    );
    let queue_bytes = fs::read(path)?;
    let journal: Journal = serde_json::from_slice(&queue_bytes)?;
    ensure!(journal.schema_version == 1, "unsupported queue schema");
    let prepared_path = path
        .parent()
        .context("queue path has no parent")?
        .join("prepared.json");
    let prepared_bytes = fs::read(&prepared_path)?;
    ensure!(
        format!("{:x}", Sha256::digest(&prepared_bytes)) == journal.prepared_sha256,
        "saved prepared manifest hash mismatch"
    );
    let prepared: Vec<Prepared> = serde_json::from_slice(&prepared_bytes)?;
    let classification_bytes = fs::read(&args.classification)?;
    ensure!(
        format!("{:x}", Sha256::digest(&classification_bytes)) == journal.classification_sha256,
        "use the exact classification snapshot from the audited queue"
    );
    let classification: Classification = serde_json::from_slice(&classification_bytes)?;
    let rows = selected(&classification, None)?;
    ensure!(
        rows.len() == prepared.len() && prepared.len() == journal.entries.len(),
        "saved queue inventory length mismatch"
    );
    ensure!(
        hash(&journal.executable)? == journal.executable_sha256,
        "saved executable has changed"
    );
    let mut seen = BTreeSet::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut entries = Vec::new();
    for (p, e) in prepared.iter().zip(&journal.entries) {
        ensure!(
            seen.insert(p.identifier.clone())
                && p.identifier == e.identifier
                && p.ledger == e.ledger,
            "saved queue identity/order mismatch"
        );
        let c = rows
            .iter()
            .find(|c| c.identifier == p.identifier)
            .context("unclassified saved case")?;
        let historical = if p.historical_original_input_sha256.is_empty() {
            &p.original_input_sha256
        } else {
            &p.historical_original_input_sha256
        };
        ensure!(
            p.mathematical_sha256 == c.mathematical_sha256
                && historical == &c.original_input_sha256,
            "saved case differs from the original classification"
        );
        let (verdict, error) = if e.status == "pending" || e.status == "running" {
            ("pending".to_owned(), None)
        } else {
            match audit_verdict(p, e, &journal.executable_sha256) {
                Ok(v) => (v, None),
                Err(error) => ("infrastructure_failed".into(), Some(format!("{error:#}"))),
            }
        };
        *counts.entry(verdict.clone()).or_default() += 1;
        entries.push(
            json!({"identifier":p.identifier,"scheduler_status":e.status,
            "verdict":verdict,"error":error,"ledger":e.ledger,"ledger_sha256":e.ledger_sha256,
            "supplemental":e.supplemental}),
        );
    }
    fs::create_dir_all(&args.output)?;
    fs::write(args.output.join("queue-snapshot.json"), &queue_bytes)?;
    fs::write(
        args.output.join("classification.json"),
        &classification_bytes,
    )?;
    fs::write(args.output.join("prepared.json"), &prepared_bytes)?;
    let verified = counts.get("verified").copied().unwrap_or(0);
    atomic(
        &args.output.join("audit.json"),
        &json!({"schema_version":1,"read_only":true,
        "source_queue":path.canonicalize()?,"source_queue_sha256":format!("{:x}",Sha256::digest(&queue_bytes)),
        "executable_sha256":journal.executable_sha256,"counts":counts,"entries":entries}),
    )?;
    println!(
        "read-only slow-suite audit: {verified}/{} verified; evidence saved to {}",
        prepared.len(),
        args.output.display()
    );
    ensure!(
        verified == prepared.len(),
        "slow suite still has pending or unverified original problems"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(path) = &args.audit_queue {
        return audit_saved_queue(&args, path);
    }
    validate_resources(&args)?;
    ensure!(
        args.write_options.is_none() || args.prepare_only,
        "--write-options requires --prepare-only"
    );
    let classification_bytes = fs::read(&args.classification)?;
    let classification: Classification = serde_json::from_slice(&classification_bytes)?;
    let aggregate_hash = classification.source_aggregate_sha256.clone();
    let aggregate = args
        .aggregate
        .as_ref()
        .map(|path| -> Result<Value> {
            ensure!(
                hash(path)? == aggregate_hash,
                "classification source aggregate hash mismatch"
            );
            Ok(serde_json::from_reader(fs::File::open(path)?)?)
        })
        .transpose()?;
    let recipes: Option<Recipes> = if args.options.exists() {
        Some(serde_json::from_reader(fs::File::open(&args.options)?)?)
    } else {
        ensure!(
            args.write_options.is_some(),
            "missing portable options file; regenerate repository fixtures or explicitly export recipes"
        );
        None
    };
    if let Some(book) = &recipes {
        ensure!(book.schema_version == 1, "unsupported recipe schema");
    }
    let mut rows = selected(&classification, aggregate.as_ref())?;
    // Old recipe books may include subsequently verified originals. Their
    // priority entries are harmless and are filtered by the current inventory.
    let priority = if args.priority.is_empty() {
        recipes
            .as_ref()
            .map(|b| {
                b.priority
                    .iter()
                    .filter(|id| rows.iter().any(|r| &r.identifier == *id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    } else {
        args.priority.clone()
    };
    ensure!(
        priority.iter().collect::<BTreeSet<_>>().len() == priority.len()
            && priority
                .iter()
                .all(|id| rows.iter().any(|r| &r.identifier == id)),
        "priority identifiers must be distinct classified slow cases"
    );
    rows.sort_by_key(|r| {
        priority
            .iter()
            .position(|id| id == &r.identifier)
            .unwrap_or(usize::MAX)
    });
    fs::create_dir_all(&args.output)?;
    let output = args.output.canonicalize()?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(output.join("queue.lock"))?;
    ensure!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "another slow queue owns this output"
    );
    let snapshot = output.join("classification.json");
    if snapshot.exists() {
        ensure!(
            fs::read(&snapshot)? == classification_bytes,
            "classification snapshot changed"
        );
    } else {
        fs::write(&snapshot, &classification_bytes)?;
    }
    let prepared = prepare(&args, &rows, &output, recipes.as_ref())?;
    atomic(&output.join("prepared.json"), &prepared)?;
    if let Some(path) = &args.write_options {
        ensure!(
            !path.exists(),
            "refusing to overwrite an existing recipe book"
        );
        let recipes = prepared
            .iter()
            .map(|p| -> Result<_> {
                let source = Problem::from_toml(&fs::read_to_string(&p.original_input)?)?;
                let mut solver =
                    Problem::from_toml(&fs::read_to_string(&p.prepared_input)?)?.solver;
                solver.checkpoint_dir = None;
                Ok((
                    p.identifier.clone(),
                    Recipe {
                        mathematical_sha256: p.mathematical_sha256.clone(),
                        original_name: source.name,
                        original_fixed_order: source.solver.order,
                        solver,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        atomic(
            path,
            &Recipes {
                schema_version: 1,
                priority,
                recipes,
            },
        )?;
    }
    println!(
        "prepared {} original slow cases at {}",
        prepared.len(),
        output.display()
    );
    if args.prepare_only {
        return Ok(());
    }
    let exe = args
        .executable
        .context("--executable is required for execution")?
        .canonicalize()?;
    let executable_hash = hash(&exe)?;
    let classification_hash = format!("{:x}", Sha256::digest(classification_bytes));
    let prepared_hash = hash(&output.join("prepared.json"))?;
    let journal_path = output.join("queue.json");
    let mut journal = if journal_path.exists() {
        let j: Journal = serde_json::from_reader(fs::File::open(&journal_path)?)?;
        ensure!(
            j.schema_version == 1
                && j.classification_sha256 == classification_hash
                && j.aggregate_sha256 == aggregate_hash
                && j.prepared_sha256 == prepared_hash
                && j.executable_sha256 == executable_hash
                && j.cpus == args.cpus
                && j.memory_mib == args.memory_mib
                && j.verification_seconds == args.verification_seconds
                && j.entries.len() == prepared.len(),
            "resume context mismatch; preserve old queue and use a new directory"
        );
        j
    } else {
        Journal {
            schema_version: 1,
            classification_sha256: classification_hash,
            aggregate_sha256: aggregate_hash,
            prepared_sha256: prepared_hash,
            executable: exe.clone(),
            executable_sha256: executable_hash,
            cpus: args.cpus.clone(),
            memory_mib: args.memory_mib,
            solve_seconds: 3600.,
            verification_seconds: args.verification_seconds,
            entries: prepared
                .iter()
                .map(|p| Entry {
                    identifier: p.identifier.clone(),
                    status: "pending".into(),
                    verdict: None,
                    cpu: None,
                    ledger: p.ledger.clone(),
                    ledger_sha256: None,
                    supplemental: None,
                    error: None,
                    wall_time_secs: None,
                })
                .collect(),
        }
    };
    for (i, e) in journal.entries.iter_mut().enumerate() {
        ensure!(
            e.identifier == prepared[i].identifier && e.ledger == prepared[i].ledger,
            "resume inventory mismatch"
        );
        if e.status == "finished" {
            ensure!(
                e.ledger_sha256
                    .as_ref()
                    .is_some_and(|h| hash(&e.ledger).ok().as_ref() == Some(h)),
                "finished ledger changed"
            );
        } else {
            e.status = "pending".into();
            e.verdict = None;
        }
    }
    atomic(&journal_path, &journal)?;
    let queue = Arc::new(Mutex::new(journal));
    let executable_digest = hash(&exe)?;
    let next = AtomicUsize::new(0);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for &cpu in &args.cpus {
            let queue = Arc::clone(&queue);
            let next = &next;
            let prepared = &prepared;
            let exe = &exe;
            let journal_path = &journal_path;
            let executable_digest = &executable_digest;
            let seconds = args.verification_seconds;
            let memory = args.memory_mib;
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= prepared.len() {
                        break;
                    }
                    {
                        let mut j = queue.lock().unwrap();
                        if j.entries[i].status == "finished" {
                            continue;
                        }
                        j.entries[i].status = "running".into();
                        j.entries[i].cpu = Some(cpu);
                        if let Err(e) = atomic(journal_path, &*j) {
                            eprintln!("journal write failed: {e}");
                            break;
                        }
                    }
                    let t = Instant::now();
                    let result = (|| -> Result<_> {
                        pin(cpu)?;
                        let report = run_campaign(&prepared[i].config, exe)?;
                        let supplement = late_verify(
                            &report,
                            exe,
                            prepared[i].config.parent().unwrap(),
                            seconds,
                            memory,
                        )?;
                        Ok((hash(&prepared[i].ledger)?, supplement))
                    })();
                    let mut j = queue.lock().unwrap();
                    let entry = &mut j.entries[i];
                    entry.wall_time_secs = Some(t.elapsed().as_secs_f64());
                    match result {
                        Ok((hash, supplement)) => {
                            entry.status = "finished".into();
                            entry.ledger_sha256 = Some(hash);
                            entry.supplemental = supplement;
                            entry.error = None;
                        }
                        Err(e) => {
                            entry.status = "infrastructure_failed".into();
                            entry.verdict = Some("infrastructure_failed".into());
                            entry.error = Some(format!("{e:#}"));
                        }
                    }
                    if entry.status == "finished" {
                        match audit_verdict(&prepared[i], entry, executable_digest) {
                            Ok(v) => entry.verdict = Some(v),
                            Err(error) => {
                                entry.verdict = Some("infrastructure_failed".into());
                                entry.error = Some(format!("evidence audit failed: {error:#}"));
                            }
                        }
                    }
                    println!(
                        "{}: {} / {} ({:.3}s)",
                        entry.identifier,
                        entry.status,
                        entry.verdict.as_deref().unwrap_or("pending"),
                        t.elapsed().as_secs_f64()
                    );
                    if let Err(e) = atomic(journal_path, &*j) {
                        eprintln!("journal write failed: {e}");
                        break;
                    }
                }
            });
        }
    });
    let mut j = queue.lock().unwrap();
    let mut counts = BTreeMap::<String, usize>::new();
    for (p, e) in prepared.iter().zip(&mut j.entries) {
        let verdict = match audit_verdict(p, e, &executable_digest) {
            Ok(v) => v,
            Err(error) => {
                e.error = Some(format!("evidence audit failed: {error:#}"));
                "infrastructure_failed".into()
            }
        };
        *counts.entry(verdict.clone()).or_default() += 1;
        e.verdict = Some(verdict);
    }
    atomic(&journal_path, &*j)?;
    atomic(
        &output.join("summary.json"),
        &json!({"schema_version":1,"counts":counts,
        "elapsed_seconds":started.elapsed().as_secs_f64(),"entries":j.entries}),
    )?;
    let verified = counts.get("verified").copied().unwrap_or(0);
    println!(
        "slow suite: {verified}/{} verified; {} incomplete; {} infrastructure failures",
        prepared.len(),
        counts.get("incomplete").unwrap_or(&0),
        counts.get("infrastructure_failed").unwrap_or(&0)
    );
    ensure!(
        verified == prepared.len(),
        "slow suite has unverified original problems; preserved all attempt evidence"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_resources_are_bounded() {
        let mut a = Args::parse_from(["slow_suite"]);
        assert!(validate_resources(&a).is_ok());
        a.cpus = vec![4, 4];
        assert!(validate_resources(&a).is_err());
        a.cpus = vec![4, 5, 6, 7];
        a.memory_mib = 20000;
        assert!(validate_resources(&a).is_err());
    }
    #[test]
    fn mathematical_key_retains_inequality_and_split_orientation() -> Result<()> {
        let mut p = Problem::from_toml("variables=['x']\nconstraints=['x>0']\nsplit=['x-1']")?;
        let first = math_key(&p)?;
        p.solver.engine = "local".into();
        assert_eq!(first, math_key(&p)?);
        p.constraints[0] = "x<0".into();
        assert_ne!(first, math_key(&p)?);
        Ok(())
    }
    #[test]
    fn prepared_generic_options_preserve_fixed_order_and_domain() -> Result<()> {
        let mut p = Problem::from_toml(
            "variables=['x0','x1']\nconstraints=['x0>0','x1>0']\nsplit=['x0-x1']\n[solver]\norder=['x1','x0']",
        )?;
        let before = math_key(&p)?;
        generic_options(&mut p)?;
        assert_eq!(p.solver.order, vec!["x1", "x0"]);
        assert_eq!(p.solver.homogeneous_anchor.as_deref(), Some("x1"));
        assert_eq!(before, math_key(&p)?);
        Ok(())
    }
    #[test]
    fn classification_alone_selects_slow_cases_without_private_aggregate() -> Result<()> {
        let c = Classification {
            schema_version: 1,
            source_aggregate_sha256: "audit-not-needed-for-selection".into(),
            slow_solve_timeout_secs: 3600.,
            cases: (0..331)
                .map(|i| Classified {
                    identifier: format!("original-{i}"),
                    aliases: vec![],
                    mathematical_sha256: format!("math-{i}"),
                    original_input: format!("case-{i}.toml").into(),
                    original_input_sha256: format!("file-{i}"),
                    classification: if i < 2 { "slow" } else { "verified" }.into(),
                })
                .collect(),
        };
        assert_eq!(selected(&c, None)?.len(), 2);
        assert!(selected(&c, Some(&json!({"unresolved":[]}))).is_err());
        Ok(())
    }
    #[test]
    fn portable_recipe_preserves_domain_and_rejects_wrong_hash_order_or_anchor() -> Result<()> {
        let source = Problem::from_toml(
            "variables=['x0','x1']\nconstraints=['x0>0','x1>0']\nsplit=['x0-x1']\n[solver]\norder=['x1','x0']",
        )?;
        let mut p = source.clone();
        generic_options(&mut p)?;
        let c = Classified {
            identifier: "toy".into(),
            aliases: vec![],
            mathematical_sha256: math_hash(&source)?,
            original_input: "unused.toml".into(),
            original_input_sha256: "unused".into(),
            classification: "slow".into(),
        };
        let mut recipe = Recipe {
            mathematical_sha256: c.mathematical_sha256.clone(),
            original_name: source.name.clone(),
            original_fixed_order: source.solver.order.clone(),
            solver: p.solver,
        };
        let result = apply_recipe(&source, &c, &recipe)?;
        assert_eq!(result.constraints, source.constraints);
        assert_eq!(result.split, source.split);
        recipe.mathematical_sha256 = "changed".into();
        assert!(apply_recipe(&source, &c, &recipe).is_err());
        recipe.mathematical_sha256 = c.mathematical_sha256.clone();
        recipe.solver.order.reverse();
        assert!(apply_recipe(&source, &c, &recipe).is_err());
        recipe.solver.order.reverse();
        recipe.solver.checkpoint_dir = Some("/private/cache".into());
        assert!(apply_recipe(&source, &c, &recipe).is_err());
        recipe.solver.checkpoint_dir = None;
        let mut renamed = source.clone();
        renamed.name = Some("different original".into());
        assert!(apply_recipe(&renamed, &c, &recipe).is_err());
        let mut unfixed = source.clone();
        unfixed.solver.order.clear();
        assert!(apply_recipe(&unfixed, &c, &recipe).is_err());
        let mut nonhomogeneous = source.clone();
        nonhomogeneous.split[0] = "x0-x1+1".into();
        let mut changed = c.clone();
        changed.mathematical_sha256 = math_hash(&nonhomogeneous)?;
        recipe.mathematical_sha256 = changed.mathematical_sha256.clone();
        assert!(apply_recipe(&nonhomogeneous, &changed, &recipe).is_err());
        Ok(())
    }
    #[test]
    fn evidence_audit_rejects_negative_checks_changed_hashes_counts_and_domains() -> Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "symgcad-slow-evidence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::create_dir(&dir)?;
        let p = Problem::from_toml("name='original'\nvariables=['x']\nconstraints=['x>0']")?;
        let result = dir.join("result.json");
        let verification = dir.join("verification.json");
        // Protocol test only: this is not a mathematical proof fixture.
        atomic(
            &result,
            &json!({"status":"complete_generic","problem":p,"order":["x"],"cells":[{}]}),
        )?;
        let rh = hash(&result)?;
        atomic(&verification, &json!({"verified":false,"cells":1}))?;
        assert!(!proof_and_check(
            &result,
            &rh,
            &verification,
            &hash(&verification)?,
            &math_hash(&p)?,
            &p
        )?);
        atomic(&verification, &json!({"verified":true,"cells":2}))?;
        assert!(
            proof_and_check(
                &result,
                &rh,
                &verification,
                &hash(&verification)?,
                &math_hash(&p)?,
                &p
            )
            .is_err()
        );
        atomic(&verification, &json!({"verified":true,"cells":1}))?;
        assert!(
            proof_and_check(&result, &rh, &verification, "tampered", &math_hash(&p)?, &p).is_err()
        );
        assert!(
            proof_and_check(
                &result,
                "tampered",
                &verification,
                &hash(&verification)?,
                &math_hash(&p)?,
                &p
            )
            .is_err()
        );
        assert!(
            proof_and_check(
                &result,
                &rh,
                &verification,
                &hash(&verification)?,
                "different domain",
                &p
            )
            .is_err()
        );
        fs::remove_dir_all(dir)?;
        Ok(())
    }
    #[test]
    fn finished_attempt_with_incomplete_result_has_failing_verdict() -> Result<()> {
        let dir = std::env::temp_dir().join(format!(
            "symgcad-slow-incomplete-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::create_dir(&dir)?;
        let ledger = dir.join("ledger.json");
        let process = json!({"status":"timeout","wall_time_secs":3600.0,"peak_rss_kib":1,
            "exit_code":null,"stdout":"","stderr":"","output_truncated":false});
        let mut report = json!({"schema_version":1,"mode":"completion","provenance":{},"scheduled":[],
            "complete_verified":0,"resumed":0,"campaign_exhausted":true,"elapsed_secs":3600.0,
            "runs":[{"identity_sha256":"id","input_sha256":"input","executable_sha256":"exe",
                "name":"toy","aliases":[],"equivalence_sha256":null,"source":"unused","input_file":"unused",
                "variant":"default","mode":"completion","requested_repeats":1,"status":"incomplete",
                "solve_median_secs":null,"peak_rss_kib":1,"trials":[{"repetition":0,"status":"incomplete",
                    "solve":process,"result_file":"unused","result_sha256":null,"result_status":"incomplete",
                    "statistics":null,"verification":null,"verification_file":"unused","verification_sha256":null,"verified":false}]}]});
        atomic(&ledger, &report)?;
        let p = Prepared {
            identifier: "toy".into(),
            aliases: vec![],
            original_input: "unused".into(),
            original_input_sha256: "unused".into(),
            historical_original_input_sha256: "unused".into(),
            source_serialization_changed: false,
            mathematical_sha256: "unused".into(),
            prepared_input: "unused".into(),
            prepared_input_sha256: "unused".into(),
            config: "unused".into(),
            config_sha256: "unused".into(),
            ledger: ledger.clone(),
            selection: json!({}),
        };
        let mut e = Entry {
            identifier: "toy".into(),
            status: "finished".into(),
            verdict: None,
            cpu: None,
            ledger: ledger.clone(),
            ledger_sha256: Some(hash(&ledger)?),
            supplemental: None,
            error: None,
            wall_time_secs: Some(3600.),
        };
        assert_eq!(audit_verdict(&p, &e, "exe")?, "incomplete");
        // Frozen first-wave helpers predate verdict and serialization fields.
        // Their evidence remains auditable without rewriting their files.
        let mut old_entry = serde_json::to_value(&e)?;
        old_entry.as_object_mut().unwrap().remove("verdict");
        let mut old_prepared = serde_json::to_value(&p)?;
        old_prepared
            .as_object_mut()
            .unwrap()
            .remove("historical_original_input_sha256");
        old_prepared
            .as_object_mut()
            .unwrap()
            .remove("source_serialization_changed");
        let old_entry: Entry = serde_json::from_value(old_entry)?;
        let old_prepared: Prepared = serde_json::from_value(old_prepared)?;
        let before = hash(&ledger)?;
        assert_eq!(
            audit_verdict(&old_prepared, &old_entry, "exe")?,
            "incomplete"
        );
        assert_eq!(before, hash(&ledger)?);
        report["runs"][0]["trials"][0]["verified"] = json!(true);
        atomic(&ledger, &report)?;
        e.ledger_sha256 = Some(hash(&ledger)?);
        assert!(audit_verdict(&p, &e, "exe").is_err());
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
