//! Durable, bounded discovery hints. No proof or runtime registry ID is stored.
//! Every imported polynomial must be factored again and every seeded stack is
//! projected again. This module is deliberately absent from certificate replay.
use super::{LocalOptions, highest};
use crate::{
    Problem,
    algebra::{Algebra, Poly, canonical, rational},
    problem::NormalizedProblem,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use symbolica::prelude::*;

const VERSION: &str = "local-cut-hints-v1";
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_POLYS: usize = 2048;
const MAX_HINTS: usize = 2048;
const MAX_IDS: usize = 2048;
const MAX_TERMS: usize = 2048;
const MAX_TOTAL_TERMS: usize = 32768;
const MAX_POLY_BYTES: usize = 128 * 1024;
const MAX_AXES: usize = 64;
const SHALLOW_LEVELS: usize = 4;
const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HintArchiveReport {
    pub active: bool,
    pub reason: Option<String>,
    pub context: String,
    pub path: Option<PathBuf>,
    pub load_status: String,
    pub loaded_bytes: usize,
    pub loaded_polynomials: usize,
    pub loaded_prefixes: usize,
    pub load_seconds: f64,
    pub prefix_hits: usize,
    pub attempted_polynomials: usize,
    pub rejected_polynomials: usize,
    pub seeded_cuts: usize,
    pub construction_seconds: f64,
    pub factor_seconds: f64,
    pub preprocessing_budget_exhausted: bool,
    pub snapshots: usize,
    pub written_bytes: usize,
    pub snapshot_seconds: f64,
    pub skipped_snapshot_entries: usize,
    pub last_error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Term {
    coefficient: String,
    exponents: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PortablePolynomial {
    expression: String,
    terms: Vec<Term>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hint {
    prefix: Vec<String>,
    polynomials: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    version: String,
    context: String,
    dictionary: BTreeMap<String, PortablePolynomial>,
    hints: Vec<Hint>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    sha256: String,
    payload: Payload,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn short_error(error: impl std::fmt::Display) -> String {
    error.to_string().chars().take(256).collect()
}
fn integer_text(text: &str, max_digits: usize) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.len() <= max_digits && digits.bytes().all(|b| b.is_ascii_digit())
}
fn prefix_number(text: &str) -> Result<Rational> {
    let mut parts = text.split('/');
    let numerator = parts.next().unwrap_or("");
    ensure!(
        integer_text(numerator, 155),
        "invalid or oversized prefix numerator"
    );
    if let Some(denominator) = parts.next() {
        ensure!(
            integer_text(denominator, 155)
                && !denominator.starts_with('-')
                && denominator.bytes().any(|b| b != b'0'),
            "invalid prefix denominator"
        );
    }
    ensure!(parts.next().is_none(), "invalid prefix fraction");
    let value = rational(text)?;
    ensure!(
        value.to_string() == text
            && value.numerator_ref().significant_bits() <= 512
            && value.denominator_ref().significant_bits() <= 512,
        "noncanonical or oversized prefix"
    );
    Ok(value)
}
fn valid_prefix(prefix: &[Rational]) -> bool {
    prefix.len() <= MAX_AXES
        && prefix.iter().all(|x| {
            x.numerator_ref().significant_bits() <= 512
                && x.denominator_ref().significant_bits() <= 512
        })
        && prefix
            .iter()
            .map(|x| x.numerator_ref().significant_bits() + x.denominator_ref().significant_bits())
            .sum::<u64>()
            <= 8192
}
impl PortablePolynomial {
    fn preflight(&self, axes: usize) -> Result<()> {
        ensure!(
            !self.expression.is_empty() && self.expression.len() <= MAX_POLY_BYTES,
            "polynomial expression size"
        );
        ensure!(
            !self.terms.is_empty() && self.terms.len() <= MAX_TERMS,
            "polynomial term limit"
        );
        let mut bytes = self.expression.len();
        for term in &self.terms {
            ensure!(
                integer_text(&term.coefficient, 1234),
                "invalid or oversized integer coefficient"
            );
            ensure!(
                term.exponents.len() == axes && term.exponents.iter().all(|&e| e <= 64),
                "invalid polynomial support"
            );
            ensure!(
                term.exponents.iter().map(|&e| u64::from(e)).sum::<u64>() <= 64,
                "polynomial degree limit"
            );
            bytes = bytes.saturating_add(term.coefficient.len() + axes * 4 + 64);
        }
        ensure!(bytes <= MAX_POLY_BYTES, "polynomial payload size");
        Ok(())
    }
    fn from_poly(algebra: &Algebra, p: &Poly) -> Result<Self> {
        ensure!(p.nterms() <= MAX_TERMS, "polynomial term limit");
        let mut text_bound = 0usize;
        for (c, exponents) in p.coefficients.iter().zip(p.exponents_iter()) {
            ensure!(
                c.denominator_ref().is_one() && c.numerator_ref().significant_bits() <= 4096,
                "polynomial coefficient limit"
            );
            ensure!(
                exponents.iter().map(|&e| u64::from(e)).sum::<u64>() <= 64,
                "polynomial degree limit"
            );
            text_bound = text_bound
                .saturating_add((c.numerator_ref().significant_bits() as usize).div_ceil(3) + 16);
            for (name, &e) in algebra.names.iter().zip(exponents) {
                if e != 0 {
                    text_bound = text_bound.saturating_add(name.len() + 16);
                }
            }
            ensure!(text_bound <= MAX_POLY_BYTES / 2, "polynomial display bound");
        }
        let terms = p
            .coefficients
            .iter()
            .zip(p.exponents_iter())
            .map(|(c, e)| Term {
                coefficient: c.to_string(),
                exponents: e.to_vec(),
            })
            .collect();
        let result = Self {
            expression: algebra.format(p),
            terms,
        };
        result.preflight(algebra.names.len())?;
        Ok(result)
    }
    pub(super) fn reconstruct(&self, algebra: &Algebra, level: usize) -> Result<Poly> {
        self.preflight(algebra.names.len())?;
        let mut p = algebra.parse("0")?;
        for term in &self.terms {
            let c = rational(&term.coefficient)?;
            ensure!(
                !c.is_zero()
                    && c.denominator_ref().is_one()
                    && c.numerator_ref().significant_bits() <= 4096
                    && c.to_string() == term.coefficient,
                "invalid canonical integer coefficient"
            );
            p.append_monomial(c, &term.exponents);
        }
        ensure!(
            p.nterms() == self.terms.len()
                && highest(&p) == Some(level)
                && canonical(&p) == p
                && algebra.format(&p) == self.expression,
            "noncanonical hint or wrong support axis"
        );
        Ok(p)
    }
}

fn context(
    problem: &Problem,
    normalized: &NormalizedProblem,
    options: &LocalOptions,
) -> Result<String> {
    let mut solver = serde_json::to_value(&problem.solver)?;
    // Storage location and resource budgets do not change proof obligations.
    if let Some(map) = solver.as_object_mut() {
        for key in [
            "checkpoint_dir",
            "cache_memory_mib",
            "local_hint_archive",
            "local_max_nodes",
            "local_max_refinements",
            "local_subtree_cache_mib",
            "local_subtree_cache_entry_nodes",
            "local_learned_cut_cache_mib",
        ] {
            map.remove(key);
        }
    }
    let mut local = serde_json::to_value(options)?;
    if let Some(map) = local.as_object_mut() {
        for key in [
            "max_nodes",
            "max_refinements",
            "subtree_cache_mib",
            "subtree_cache_entry_nodes",
            "learned_cut_cache_mib",
        ] {
            map.remove(key);
        }
    }
    Ok(digest(&serde_json::to_vec(&(
        VERSION,
        LicenseManager::get_version(),
        &normalized.algebra.names,
        normalized
            .positives
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect::<Vec<_>>(),
        normalized
            .split
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect::<Vec<_>>(),
        &problem.parameters,
        &problem.variable_groups,
        solver,
        local,
        crate::domain::CertificateOptions::default(),
    ))?))
}

fn read_bounded(path: &Path, deadline: Instant) -> Result<Vec<u8>> {
    // Do not block opening a FIFO or follow an archive symlink.
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= MAX_BYTES as u64,
        "archive must be a bounded regular file"
    );
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut block = [0u8; 65536];
    loop {
        ensure!(Instant::now() < deadline, "archive read time limit");
        let count = file.read(&mut block)?;
        if count == 0 {
            break;
        }
        ensure!(
            bytes.len().saturating_add(count) <= MAX_BYTES,
            "archive grew beyond byte limit"
        );
        bytes.extend_from_slice(&block[..count]);
    }
    Ok(bytes)
}
fn atomic_write(path: &Path, bytes: &[u8], deadline: Instant) -> Result<()> {
    ensure!(
        bytes.len() <= MAX_BYTES && Instant::now() < deadline,
        "archive write budget"
    );
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("archive parent missing"))?;
    std::fs::create_dir_all(parent)?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = path.with_extension(format!("{}-{nonce}.tmp", std::process::id()));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        for chunk in bytes.chunks(65536) {
            ensure!(Instant::now() < deadline, "archive write time limit");
            file.write_all(chunk)?;
        }
        file.sync_all()?;
        ensure!(Instant::now() < deadline, "archive publish time limit");
        std::fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

pub(super) struct HintArchive {
    pub report: HintArchiveReport,
    dictionary: HashMap<String, Arc<PortablePolynomial>>,
    pending: HashMap<Vec<Rational>, Vec<String>>,
    validated: HashMap<String, Option<usize>>,
    shallow: BTreeMap<usize, (Vec<Rational>, Vec<usize>)>,
    deadline: Instant,
    preprocessing_limit: f64,
    last_snapshot: Instant,
    dirty: bool,
}
pub(super) enum Candidate {
    Known(usize),
    New(String, Arc<PortablePolynomial>),
}
impl HintArchive {
    pub(super) fn new(
        problem: &Problem,
        normalized: &NormalizedProblem,
        options: &LocalOptions,
        started: Instant,
    ) -> Option<Self> {
        if !problem.solver.local_hint_archive {
            return None;
        }
        let mut result = Self {
            report: HintArchiveReport::default(),
            dictionary: HashMap::new(),
            pending: HashMap::new(),
            validated: HashMap::new(),
            shallow: BTreeMap::new(),
            deadline: started + Duration::from_secs_f64(problem.limits.wall_time_secs),
            preprocessing_limit: (problem.limits.wall_time_secs * 0.05).min(2.0),
            last_snapshot: Instant::now(),
            dirty: false,
        };
        let reason = if problem.solver.checkpoint_dir.is_none() {
            Some("checkpoint_dir is required")
        } else if options.descartes_witnesses
            || options.descartes_intervals
            || options.positive_pair_witnesses
        {
            Some(
                "sample-dependent witness templates require a persistent admission ledger; archive disabled",
            )
        } else if !options.cache_enabled || !options.reuse_learned_cuts {
            Some("local caches and learned cuts must be enabled")
        } else if normalized.algebra.names.len() > MAX_AXES {
            Some("archive variable-map limit")
        } else {
            None
        };
        if let Some(reason) = reason {
            result.report.reason = Some(reason.into());
            return Some(result);
        }
        let load_started = Instant::now();
        match context(problem, normalized, options) {
            Ok(key) => result.report.context = key,
            Err(error) => {
                result.report.reason = Some(short_error(error));
                return Some(result);
            }
        }
        result.report.active = true;
        let path = problem
            .solver
            .checkpoint_dir
            .as_ref()
            .unwrap()
            .join("local-hints")
            .join(format!("{}.json", result.report.context));
        result.report.path = Some(path.clone());
        let load = result.load(
            &path,
            normalized.algebra.names.len(),
            load_started + Duration::from_secs_f64(result.preprocessing_limit),
        );
        result.report.load_seconds = load_started.elapsed().as_secs_f64();
        match load {
            Ok(()) => result.report.load_status = "loaded".into(),
            Err(error) => {
                result.dictionary.clear();
                result.pending.clear();
                result.report.load_status = if !path.exists() {
                    "missing"
                } else {
                    "rejected"
                }
                .into();
                if path.exists() {
                    result.report.last_error = Some(short_error(error));
                }
            }
        }
        Some(result)
    }
    fn load(&mut self, path: &Path, axes: usize, deadline: Instant) -> Result<()> {
        let bytes = read_bounded(path, deadline.min(self.deadline))?;
        self.report.loaded_bytes = bytes.len();
        let envelope: Envelope = serde_json::from_slice(&bytes)?;
        let payload = envelope.payload;
        ensure!(
            payload.version == VERSION && payload.context == self.report.context,
            "archive context/version mismatch"
        );
        ensure!(
            digest(&serde_json::to_vec(&payload)?) == envelope.sha256,
            "archive checksum mismatch"
        );
        ensure!(
            payload.dictionary.len() <= MAX_POLYS && payload.hints.len() <= MAX_HINTS,
            "archive inventory limit"
        );
        let mut terms = 0usize;
        for (key, p) in payload.dictionary {
            ensure!(
                Instant::now() < deadline,
                "archive preprocessing time limit"
            );
            p.preflight(axes)?;
            terms += p.terms.len();
            ensure!(
                terms <= MAX_TOTAL_TERMS && digest(&serde_json::to_vec(&p)?) == key,
                "archive dictionary checksum/term limit"
            );
            self.dictionary.insert(key, Arc::new(p));
        }
        for hint in payload.hints {
            ensure!(
                Instant::now() < deadline
                    && hint.prefix.len() < axes
                    && hint.polynomials.len() <= MAX_IDS,
                "archive hint limit"
            );
            let prefix = hint
                .prefix
                .iter()
                .map(|x| prefix_number(x))
                .collect::<Result<Vec<_>>>()?;
            ensure!(valid_prefix(&prefix), "archive prefix limit");
            ensure!(
                hint.polynomials
                    .iter()
                    .all(|id| self.dictionary.contains_key(id)),
                "missing dictionary entry"
            );
            ensure!(
                hint.polynomials.iter().collect::<BTreeSet<_>>().len() == hint.polynomials.len(),
                "duplicate polynomial reference"
            );
            ensure!(
                self.pending.insert(prefix, hint.polynomials).is_none(),
                "duplicate prefix"
            );
        }
        self.report.loaded_polynomials = self.dictionary.len();
        self.report.loaded_prefixes = self.pending.len();
        Ok(())
    }
    pub(super) fn budget_available(&mut self) -> bool {
        let available = self.report.load_seconds
            + self.report.construction_seconds
            + self.report.factor_seconds
            < self.preprocessing_limit
            && Instant::now() < self.deadline;
        if !available {
            self.report.preprocessing_budget_exhausted = true;
        }
        available
    }
    pub(super) fn candidates(&mut self, prefix: &[Rational]) -> Vec<Candidate> {
        if !self.report.active || !self.budget_available() {
            return Vec::new();
        }
        let Some(ids) = self.pending.remove(prefix) else {
            return Vec::new();
        };
        self.report.prefix_hits += 1;
        ids.into_iter()
            .filter_map(|id| match self.validated.get(&id) {
                Some(Some(id)) => Some(Candidate::Known(*id)),
                Some(None) => None,
                None => self
                    .dictionary
                    .get(&id)
                    .map(|p| Candidate::New(id, p.clone())),
            })
            .collect()
    }
    pub(super) fn validated(&mut self, hash: String, id: Option<usize>) {
        self.report.attempted_polynomials += 1;
        self.report.rejected_polynomials += usize::from(id.is_none());
        self.validated.insert(hash, id);
    }
    pub(super) fn observe(&mut self, prefix: &[Rational], cuts: &BTreeSet<usize>) {
        if !self.report.active || !valid_prefix(prefix) {
            return;
        }
        self.dirty = true;
        if prefix.len() < SHALLOW_LEVELS {
            self.shallow.insert(
                prefix.len(),
                (
                    prefix.to_vec(),
                    cuts.iter().take(MAX_IDS).copied().collect(),
                ),
            );
        }
    }
    pub(super) fn snapshot<'a>(
        &mut self,
        entries: impl Iterator<Item = (&'a [Rational], &'a [usize])>,
        polynomials: &[Poly],
        algebra: &Algebra,
        force: bool,
    ) {
        if !self.report.active
            || !self.dirty
            || Instant::now() >= self.deadline
            || (!force && self.last_snapshot.elapsed() < SNAPSHOT_INTERVAL)
        {
            return;
        }
        let started = Instant::now();
        let result = self.make_snapshot(entries, polynomials, algebra);
        self.report.snapshot_seconds += started.elapsed().as_secs_f64();
        self.last_snapshot = Instant::now();
        match result {
            Ok(bytes) => {
                self.report.snapshots += 1;
                self.report.written_bytes += bytes;
                self.dirty = false;
            }
            Err(error) => self.report.last_error = Some(short_error(error)),
        }
    }
    fn make_snapshot<'a>(
        &mut self,
        entries: impl Iterator<Item = (&'a [Rational], &'a [usize])>,
        polynomials: &[Poly],
        algebra: &Algebra,
    ) -> Result<usize> {
        let mut payload = Payload {
            version: VERSION.into(),
            context: self.report.context.clone(),
            dictionary: BTreeMap::new(),
            hints: Vec::new(),
        };
        let mut seen = BTreeSet::new();
        let mut runtime_keys = HashMap::<usize, Option<String>>::new();
        let mut total_terms = 0;
        let mut estimated_bytes = 0usize;
        let snapshot_deadline = self
            .deadline
            .min(Instant::now() + Duration::from_millis(250));
        for (prefix, ids) in self
            .shallow
            .values()
            .cloned()
            .chain(entries.map(|(p, c)| (p.to_vec(), c.iter().take(MAX_IDS).copied().collect())))
            .take(MAX_HINTS * 2)
        {
            if Instant::now() >= snapshot_deadline || payload.hints.len() >= MAX_HINTS {
                break;
            }
            if !valid_prefix(&prefix)
                || prefix.len() >= algebra.names.len()
                || !seen.insert(prefix.to_vec())
            {
                continue;
            }
            let mut keys = BTreeSet::new();
            for &id in ids.iter().take(MAX_IDS) {
                if Instant::now() >= snapshot_deadline {
                    break;
                }
                if let Some(key) = runtime_keys.get(&id) {
                    if let Some(key) = key {
                        keys.insert(key.clone());
                    }
                    continue;
                }
                let candidate = polynomials
                    .get(id)
                    .filter(|p| highest(p) == Some(prefix.len()))
                    .and_then(|p| PortablePolynomial::from_poly(algebra, p).ok());
                let mut key = None;
                if let Some(p) = candidate {
                    let bytes = serde_json::to_vec(&p)?;
                    if total_terms + p.terms.len() <= MAX_TOTAL_TERMS
                        && payload.dictionary.len() < MAX_POLYS
                        && estimated_bytes + bytes.len() < MAX_BYTES / 2
                    {
                        let hash = digest(&bytes);
                        total_terms += p.terms.len();
                        estimated_bytes += bytes.len();
                        payload.dictionary.insert(hash.clone(), p);
                        keys.insert(hash.clone());
                        key = Some(hash);
                    }
                }
                if key.is_none() {
                    self.report.skipped_snapshot_entries += 1;
                }
                runtime_keys.insert(id, key);
            }
            if !keys.is_empty() {
                estimated_bytes += keys.len() * 68 + prefix.len() * 320 + 128;
                if estimated_bytes > MAX_BYTES * 3 / 4 {
                    break;
                }
                payload.hints.push(Hint {
                    prefix: prefix.iter().map(ToString::to_string).collect(),
                    polynomials: keys.into_iter().collect(),
                });
            }
        }
        ensure!(!payload.hints.is_empty(), "no bounded hints to publish");
        let sha256 = digest(&serde_json::to_vec(&payload)?);
        let bytes = serde_json::to_vec(&Envelope { sha256, payload })?;
        atomic_write(self.report.path.as_ref().unwrap(), &bytes, self.deadline)?;
        Ok(bytes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_projection::{LocalEvidence, solve_local, verify_local};
    use crate::output::Status;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "symgcad-hints-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(directory: &Directory) -> (Problem, NormalizedProblem, LocalOptions) {
        let mut p = Problem {
            variables: vec!["x".into(), "y".into()],
            constraints: vec!["x>0".into(), "y>0".into()],
            split: vec!["y-x".into()],
            ..Default::default()
        };
        p.solver.checkpoint_dir = Some(directory.0.clone());
        p.solver.local_hint_archive = true;
        p.limits.wall_time_secs = 20.0;
        let n = crate::problem::normalize(&p, &p.variables).unwrap();
        (p, n, LocalOptions::default())
    }
    fn put(archive: &HintArchive, algebra: &Algebra, entries: &[(&[&str], &[&str])]) {
        let mut payload = Payload {
            version: VERSION.into(),
            context: archive.report.context.clone(),
            dictionary: BTreeMap::new(),
            hints: Vec::new(),
        };
        for (prefix, expressions) in entries {
            let mut ids = Vec::new();
            for expression in *expressions {
                let p = PortablePolynomial::from_poly(
                    algebra,
                    &canonical(&algebra.parse(expression).unwrap()),
                )
                .unwrap();
                let hash = digest(&serde_json::to_vec(&p).unwrap());
                ids.push(hash.clone());
                payload.dictionary.insert(hash, p);
            }
            payload.hints.push(Hint {
                prefix: prefix.iter().map(|x| x.to_string()).collect(),
                polynomials: ids,
            });
        }
        let sha256 = digest(&serde_json::to_vec(&payload).unwrap());
        atomic_write(
            archive.report.path.as_ref().unwrap(),
            &serde_json::to_vec(&Envelope { sha256, payload }).unwrap(),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
    }
    fn report(result: &super::super::LocalSolveResult) -> &HintArchiveReport {
        match &result.evidence {
            LocalEvidence::Local { report, .. } => report.hint_archive.as_ref().unwrap(),
            LocalEvidence::Unfinished {
                report: Some(report),
                ..
            } => report.hint_archive.as_ref().unwrap(),
            _ => panic!("missing local archive report"),
        }
    }
    #[test]
    fn disabled_serialization_and_applicability_preserve_old_defaults() {
        let mut p = Problem::default();
        assert!(
            !serde_json::to_value(&p.solver)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("local_hint_archive")
        );
        p.solver.local_hint_archive = true;
        p.variables = vec!["x".into()];
        let n = crate::problem::normalize(&p, &p.variables).unwrap();
        let a = HintArchive::new(&p, &n, &LocalOptions::default(), Instant::now()).unwrap();
        assert!(!a.report.active);
        assert!(a.report.reason.unwrap().contains("checkpoint_dir"));
        let d = Directory::new();
        let (p, n, o) = fixture(&d);
        for flag in 0..3 {
            let mut o = o.clone();
            match flag {
                0 => o.descartes_witnesses = true,
                1 => o.descartes_intervals = true,
                _ => o.positive_pair_witnesses = true,
            }
            let a = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
            assert!(!a.report.active);
            assert!(a.report.reason.unwrap().contains("admission ledger"));
        }
    }
    #[test]
    fn strict_context_changes_for_math_and_proof_but_not_storage_or_budget() {
        let d = Directory::new();
        let (p, n, o) = fixture(&d);
        let key = context(&p, &n, &o).unwrap();
        let mut changed = p.clone();
        changed.solver.checkpoint_dir = Some("elsewhere".into());
        changed.limits.wall_time_secs = 100.0;
        changed.solver.local_hint_archive = false;
        changed.solver.local_subtree_cache_entry_nodes = 1024;
        let mut changed_options = o.clone();
        changed_options.max_nodes += 100;
        changed_options.subtree_cache_entry_nodes = 1024;
        assert_eq!(context(&changed, &n, &changed_options).unwrap(), key);
        changed.constraints.push("x<2".into());
        let changed_n = crate::problem::normalize(&changed, &changed.variables).unwrap();
        assert_ne!(context(&changed, &changed_n, &o).unwrap(), key);
        let reversed = crate::problem::normalize(&p, &["y".into(), "x".into()]).unwrap();
        assert_ne!(context(&p, &reversed, &o).unwrap(), key);
        let mut changed = p.clone();
        changed.split.push("x-3".into());
        let changed_n = crate::problem::normalize(&changed, &changed.variables).unwrap();
        assert_ne!(context(&changed, &changed_n, &o).unwrap(), key);
        let mut changed = o.clone();
        changed.positive_ray = false;
        assert_ne!(context(&p, &n, &changed).unwrap(), key);
    }
    #[test]
    fn sparse_polynomials_reject_expressions_powers_axis_and_nonprimitive_values() {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let good = PortablePolynomial::from_poly(&algebra, &algebra.parse("x-2").unwrap()).unwrap();
        assert!(good.reconstruct(&algebra, 0).is_ok());
        let mut bad = good.clone();
        bad.terms[0].coefficient = "2^999999999".into();
        assert!(bad.reconstruct(&algebra, 0).is_err());
        let mut bad = good.clone();
        bad.terms[0].exponents[0] = u32::MAX;
        assert!(bad.reconstruct(&algebra, 0).is_err());
        let mut bad = good.clone();
        bad.expression = "x-3".into();
        assert!(bad.reconstruct(&algebra, 0).is_err());
        assert!(good.reconstruct(&algebra, 1).is_err());
        let bad =
            PortablePolynomial::from_poly(&algebra, &algebra.parse("2*x-4").unwrap()).unwrap();
        assert!(bad.reconstruct(&algebra, 0).is_err());
        for text in ["1/0", "x", "1+1", "1/2/3", "0001", "-0", "1/-2"] {
            assert!(prefix_number(text).is_err(), "{text}");
        }
    }
    #[test]
    fn snapshot_remaps_runtime_ids_and_requires_the_exact_prefix() {
        let d = Directory::new();
        let (p, n, o) = fixture(&d);
        let mut archive = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        let polys = vec![
            n.algebra.parse("y-3").unwrap(),
            n.algebra.parse("x-2").unwrap(),
        ];
        archive.observe(&[], &BTreeSet::from([1]));
        archive.observe(&[Rational::from(2)], &BTreeSet::from([0]));
        archive.snapshot(std::iter::empty(), &polys, &n.algebra, true);
        assert_eq!(archive.report.snapshots, 1);
        let mut loaded = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        assert_eq!(loaded.report.load_status, "loaded");
        assert!(loaded.candidates(&[Rational::from(3)]).is_empty());
        let candidates = loaded.candidates(&[]);
        assert_eq!(candidates.len(), 1);
        let Candidate::New(hash, poly) = &candidates[0] else {
            panic!()
        };
        assert_eq!(poly.reconstruct(&n.algebra, 0).unwrap(), polys[1]);
        loaded.validated(hash.clone(), Some(88));
        assert_eq!(loaded.validated[hash], Some(88));
        assert_eq!(loaded.candidates(&[Rational::from(2)]).len(), 1);
        let text = std::fs::read_to_string(loaded.report.path.unwrap()).unwrap();
        assert!(!text.contains("runtime"));
    }
    #[test]
    fn malformed_truncated_oversized_and_context_mismatched_archives_are_misses() {
        let d = Directory::new();
        let (p, n, o) = fixture(&d);
        let a = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        put(&a, &n.algebra, &[(&[], &["x-2"])]);
        let path = a.report.path.as_ref().unwrap();
        let bytes = std::fs::read(path).unwrap();
        for invalid in [&b"{"[..], &bytes[..bytes.len() / 2], &b"{}"[..]] {
            std::fs::write(path, invalid).unwrap();
            let a = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
            assert_eq!(a.report.load_status, "rejected");
            assert!(a.pending.is_empty());
        }
        let file = File::create(path).unwrap();
        file.set_len((MAX_BYTES + 1) as u64).unwrap();
        assert!(read_bounded(path, Instant::now() + Duration::from_secs(1)).is_err());
        let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
        envelope.payload.context = "wrong".into();
        envelope.sha256 = digest(&serde_json::to_vec(&envelope.payload).unwrap());
        std::fs::write(path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert_eq!(
            HintArchive::new(&p, &n, &o, Instant::now())
                .unwrap()
                .report
                .load_status,
            "rejected"
        );
        std::fs::write(path, bytes).unwrap();
        assert!(read_bounded(path, Instant::now() - Duration::from_secs(1)).is_err());
    }
    #[test]
    fn atomic_failure_retains_previous_complete_archive_and_ignores_temporary_files() {
        let d = Directory::new();
        let path = d.0.join("test.json");
        std::fs::write(&path, b"previous").unwrap();
        assert!(
            atomic_write(
                &path,
                b"replacement",
                Instant::now() - Duration::from_secs(1)
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        let (p, n, o) = fixture(&d);
        let a = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        put(&a, &n.algebra, &[(&[], &["x-2"])]);
        std::fs::write(
            a.report
                .path
                .as_ref()
                .unwrap()
                .with_extension("abandoned.tmp"),
            b"{",
        )
        .unwrap();
        assert_eq!(
            HintArchive::new(&p, &n, &o, Instant::now())
                .unwrap()
                .report
                .load_status,
            "loaded"
        );
    }
    #[test]
    fn imported_composite_and_wrong_axis_are_rejected_and_additive_valid_cut_is_verified() {
        let d = Directory::new();
        let (p, n, o) = fixture(&d);
        let a = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        put(&a, &n.algebra, &[(&[], &["x-2", "x^2-1", "y-3"])]);
        let result = solve_local(&p, &o).unwrap();
        assert_eq!(result.solution.status, Status::CompleteGeneric);
        assert!(verify_local(&result).unwrap().verified);
        assert_eq!(report(&result).rejected_polynomials, 2);
        assert!(report(&result).seeded_cuts >= 1);
        // Proof replay is self-contained, even with the complete archive gone.
        std::fs::remove_dir_all(&d.0).unwrap();
        assert!(verify_local(&result).unwrap().verified);
        let mut plain = p.clone();
        plain.solver.local_hint_archive = false;
        plain.solver.checkpoint_dir = None;
        let plain = solve_local(&plain, &o).unwrap();
        assert!(verify_local(&plain).unwrap().verified);
        assert!(result.solution.cells.len() > plain.solution.cells.len());
        let mut with_hints = result.solution.clone();
        with_hints.local = Some(Box::new(result.evidence));
        let mut without_hints = plain.solution.clone();
        without_hints.local = Some(Box::new(plain.evidence));
        assert_eq!(
            crate::comparison::compare_generic(&with_hints, &without_hints, &p.limits)
                .unwrap()
                .equivalent,
            Some(true)
        );
    }
    #[test]
    fn soft_stop_publishes_hints_but_cannot_claim_completion() {
        let d = Directory::new();
        let (p, _, mut o) = fixture(&d);
        o.max_nodes = 1;
        let result = solve_local(&p, &o).unwrap();
        assert_eq!(result.solution.status, Status::Incomplete);
        assert_eq!(report(&result).snapshots, 1);
        assert!(report(&result).path.as_ref().unwrap().exists());
        assert!(verify_local(&result).is_err());
        let restarted = solve_local(&p, &LocalOptions::default()).unwrap();
        assert_eq!(restarted.solution.status, Status::CompleteGeneric);
        assert_eq!(report(&restarted).load_status, "loaded");
        assert!(report(&restarted).prefix_hits > 0 && report(&restarted).attempted_polynomials > 0);
        assert!(verify_local(&restarted).unwrap().verified);
    }
    #[test]
    fn shallow_inventory_survives_an_empty_or_evicted_fifo() {
        let d = Directory::new();
        let (p, n, o) = fixture(&d);
        let mut a = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        let polynomials = vec![n.algebra.parse("x-2").unwrap()];
        a.observe(&[], &BTreeSet::from([0]));
        a.snapshot(std::iter::empty(), &polynomials, &n.algebra, true);
        let loaded = HintArchive::new(&p, &n, &o, Instant::now()).unwrap();
        assert!(loaded.pending.contains_key(&Vec::<Rational>::new()));
    }
}
