//! Durable scheduler hints. No archived status is a mathematical certificate.
//!
//! A bounded journal plus deterministic replay reconstructs the RNG, UCT tree,
//! duplicate visits and promotion state. Only completed iteration boundaries are
//! published, so killing an in-flight worker repeats that trial after restart.

use super::*;
use anyhow::{Context, bail};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const VERSION: u32 = 1;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_RECORDS: usize = 4096;
const MAX_VARIABLES: usize = 64;
static NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct SearchCheckpointOptions {
    pub directory: PathBuf,
    /// Exact caller context: normalized mathematical input, variable mapping,
    /// domain, transformation/proof options, backend and executable identity.
    /// Volatile output/cache paths and invocation wall budgets may be omitted.
    pub context: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CheckpointReport {
    pub path: PathBuf,
    pub loaded_iterations: usize,
    pub loaded_trials: usize,
    pub restored_completion_claims: usize,
    pub writes: usize,
    pub bytes_written: u64,
    pub io_seconds: f64,
    pub load_seconds: f64,
    pub rejected: Option<String>,
    pub write_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Archive {
    schema_version: u32,
    context_sha256: String,
    pub iterations: usize,
    pub records: Vec<TrialRecord>,
    pub revalidations: Vec<TrialRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    sha256: String,
    archive: Archive,
}

pub(super) struct Journal {
    context: String,
    pub report: CheckpointReport,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn archive_digest(archive: &Archive) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&serde_json::to_value(
        archive,
    )?)?))
}

impl Journal {
    pub(super) fn save(
        &mut self,
        iterations: usize,
        records: &[TrialRecord],
        revalidations: &[TrialRecord],
    ) {
        if self.report.write_error.is_some() {
            return;
        }
        let start = Instant::now();
        let result = (|| -> Result<usize> {
            ensure!(
                iterations <= MAX_RECORDS && records.len() + revalidations.len() <= MAX_RECORDS,
                "search journal record limit reached; previous checkpoint retained"
            );
            // Text is diagnostic, never scheduler input. Bound it before cloning.
            let bounded = |records: &[TrialRecord]| {
                records
                    .iter()
                    .map(|r| {
                        let message = r.outcome.message.as_ref().map(|message| {
                            let mut n = message.len().min(4096);
                            while !message.is_char_boundary(n) {
                                n -= 1;
                            }
                            message[..n].to_owned()
                        });
                        TrialRecord {
                            request: r.request.clone(),
                            outcome: TrialOutcome {
                                status: r.outcome.status,
                                elapsed_secs: r.outcome.elapsed_secs,
                                cells: r.outcome.cells,
                                progress_score: r.outcome.progress_score,
                                message,
                                abandoned_prefix: r.outcome.abandoned_prefix.clone(),
                            },
                        }
                    })
                    .collect()
            };
            let archive = Archive {
                schema_version: VERSION,
                context_sha256: self.context.clone(),
                iterations,
                records: bounded(records),
                revalidations: bounded(revalidations),
            };
            let payload = serde_json::to_vec(&archive)?;
            ensure!(
                payload.len() <= MAX_BYTES - 256,
                "search journal byte limit reached; previous checkpoint retained"
            );
            let bytes = serde_json::to_vec(&Envelope {
                sha256: archive_digest(&archive)?,
                archive,
            })?;
            ensure!(
                bytes.len() <= MAX_BYTES,
                "search journal byte limit reached"
            );
            atomic_write(&self.report.path, &bytes)?;
            Ok(bytes.len())
        })();
        self.report.io_seconds += start.elapsed().as_secs_f64();
        match result {
            Ok(bytes) => {
                self.report.writes += 1;
                self.report.bytes_written += bytes as u64
            }
            Err(e) => self.report.write_error = Some(e.to_string()),
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("search checkpoint has no parent")?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".search-{}-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = options.open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn read_archive(
    path: &Path,
    context: &str,
    variables: usize,
    options: &SearchOptions,
    groups: &[Vec<usize>],
) -> Result<Archive> {
    let mut open = OpenOptions::new();
    open.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = open.open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= MAX_BYTES as u64,
        "search checkpoint is not a bounded regular file"
    );
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_BYTES,
        "search checkpoint grew beyond byte limit"
    );
    let envelope: Envelope = serde_json::from_slice(&bytes)?;
    ensure!(
        archive_digest(&envelope.archive)? == envelope.sha256,
        "search checkpoint checksum mismatch"
    );
    let a = envelope.archive;
    ensure!(
        a.schema_version == VERSION,
        "unsupported search checkpoint version"
    );
    ensure!(
        a.context_sha256 == context,
        "search checkpoint context mismatch"
    );
    ensure!(
        a.iterations > 0
            && a.iterations <= options.max_rollouts
            && a.iterations <= MAX_RECORDS
            && a.records.len() <= a.iterations
            && a.records.len() + a.revalidations.len() <= MAX_RECORDS,
        "search checkpoint exceeds rollout/record bounds"
    );
    let mut orders = BTreeSet::new();
    for record in &a.records {
        orders.insert(record.request.order.clone());
    }
    for (index, r) in a.records.iter().chain(&a.revalidations).enumerate() {
        validate_order(&r.request.order, groups)?;
        ensure!(
            r.request.elimination_prefix.len() <= variables
                && r.request
                    .order
                    .iter()
                    .rev()
                    .copied()
                    .take(r.request.elimination_prefix.len())
                    .eq(r.request.elimination_prefix.iter().copied()),
            "invalid checkpoint elimination prefix"
        );
        ensure!(
            r.request.tier < options.candidate_timeouts_secs.len()
                && r.request.seed == options.seed
                && r.request.timeout_secs.is_finite()
                && r.request.timeout_secs > 0.0
                && r.request.timeout_secs <= options.candidate_timeouts_secs[r.request.tier],
            "invalid checkpoint request"
        );
        ensure!(
            r.outcome.message.as_ref().is_none_or(|s| s.len() <= 4096),
            "oversized checkpoint diagnostic"
        );
        validate_outcome(&r.request, &r.outcome, variables)?;
        if index >= a.records.len() {
            ensure!(
                orders.contains(&r.request.order),
                "revalidation references an unknown order"
            );
        }
    }
    Ok(a)
}

/// The same exact, killable worker callback as `search`. A restored completion
/// is freshly re-executed through this callback before it can become a winner.
/// Scheduler events replay deterministically; budgets are for this invocation,
/// while the cumulative scheduler iteration limit remains `max_rollouts`.
pub fn search_checkpointed<F>(
    groups: &[Vec<usize>],
    priority: &[usize],
    seeds: &[Vec<usize>],
    options: &SearchOptions,
    checkpoint: &SearchCheckpointOptions,
    mut evaluate: F,
) -> Result<SearchReport>
where
    F: FnMut(&TrialRequest) -> Result<TrialOutcome>,
{
    options.validate()?;
    let started = Instant::now();
    let variables: usize = groups.iter().map(Vec::len).sum();
    validate_groups(groups, variables)?;
    ensure!(
        priority.len() == variables
            && priority.iter().copied().collect::<BTreeSet<_>>() == (0..variables).collect(),
        "elimination priority must be a permutation of variable indices"
    );
    for seed in seeds {
        validate_order(seed, groups)?;
    }
    // Unsupported persistence bounds leave ordinary bounded search available.
    if variables > MAX_VARIABLES
        || options.max_rollouts > MAX_RECORDS
        || seeds.len() > MAX_RECORDS
        || checkpoint.context.len() > 1024 * 1024
    {
        let mut report = search(groups, priority, seeds, options, evaluate)?;
        report.checkpoint = Some(CheckpointReport {
            path: checkpoint.directory.join("searches"),
            rejected: Some(
                "search checkpoint policy bounds exceeded; running without persistence".into(),
            ),
            ..Default::default()
        });
        return Ok(report);
    }
    let policy = serde_json::json!({"version":VERSION,"context":checkpoint.context,"groups":groups,"priority":priority,"seeds":seeds,"seed":options.seed,"exploration":options.exploration_constant,"tiers":options.candidate_timeouts_secs,"promotion":options.promotion_interval});
    let context = digest(&serde_json::to_vec(&policy)?);
    let path = checkpoint
        .directory
        .join("searches")
        .join(format!("{context}.json"));
    let mut journal = Journal {
        context: context.clone(),
        report: CheckpointReport {
            path: path.clone(),
            ..Default::default()
        },
    };
    let restored = match read_archive(&path, &context, variables, options, groups) {
        Ok(a) => {
            let mut replay_options = options.clone();
            replay_options.max_rollouts = a.iterations;
            replay_options.wall_time_secs =
                (options.wall_time_secs - started.elapsed().as_secs_f64()).max(f64::MIN_POSITIVE);
            match search_impl(
                groups,
                priority,
                seeds,
                &replay_options,
                Some(&a),
                None,
                true,
                |_| bail!("checkpoint replay attempted algebra"),
            ) {
                Ok(_) => Some(a),
                Err(e) => {
                    journal.report.rejected =
                        Some(format!("checkpoint scheduler replay rejected: {e}"));
                    None
                }
            }
        }
        Err(e) => {
            if !e
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
            {
                journal.report.rejected = Some(e.to_string());
            }
            None
        }
    };
    journal.report.load_seconds = started.elapsed().as_secs_f64();
    if let Some(a) = &restored {
        journal.report.loaded_iterations = a.iterations;
        journal.report.loaded_trials = a.records.len();
        journal.report.restored_completion_claims = a
            .records
            .iter()
            .filter(|r| r.outcome.status == TrialStatus::Completed)
            .count();
    }
    let mut remaining = options.clone();
    remaining.wall_time_secs =
        (options.wall_time_secs - started.elapsed().as_secs_f64()).max(f64::MIN_POSITIVE);
    let mut report = search_impl(
        groups,
        priority,
        seeds,
        &remaining,
        restored.as_ref(),
        Some(&mut journal),
        false,
        &mut evaluate,
    )?;
    report.wall_time_secs = started.elapsed().as_secs_f64();
    report.checkpoint = Some(journal.report);
    Ok(report)
}
