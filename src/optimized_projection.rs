//! Exact projection shortcuts and bounded reusable algebra/checkpoint caches.
//!
//! Ordinary optimized projection has the same factor sets as baseline GPROJ.
//! Optional domain pruning records exact nonvanishing certificates for generated
//! factors. Optional input pruning preserves every original coordinate-affine
//! factor bounding the domain. Checkpoints accelerate computation and are not
//! a replacement for proof replay.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symbolica::{domains::InternalOrdering, prelude::*};

use crate::{
    algebra::{Algebra, MAX_DEGREE, Poly, canonical, factors, factors_checked},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    projection::{Projection, ProjectionStep, discriminant},
};

const CACHE_SCHEMA: u32 = 1;
const ALGORITHM: &str = "symgcad-exact-gproj-v1";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OptimizedProjectionOptions {
    pub checkpoint_dir: Option<PathBuf>,
    /// Atomically replaced after each completed algebra stage. This report
    /// remains readable if a bounded ordering worker is subsequently killed.
    /// It has no effect on the persistent algebra cache keys.
    pub progress_path: Option<PathBuf>,
    /// Conservative modeled retention budget; process RSS limits remain the
    /// hard bound, including transient Symbolica allocations and the result.
    pub memory_cache_bytes: usize,
    pub max_checkpoint_bytes: usize,
    /// Include the complete normalized domain when domain certificates are
    /// enabled by the caller. The default algorithm does not use a domain.
    pub domain_fingerprint: String,
}

impl Default for OptimizedProjectionOptions {
    fn default() -> Self {
        Self {
            checkpoint_dir: None,
            progress_path: None,
            memory_cache_bytes: 128 * 1024 * 1024,
            max_checkpoint_bytes: 512 * 1024 * 1024,
            domain_fingerprint: "unrestricted".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub factor_hits: usize,
    pub factor_misses: usize,
    pub resultant_hits: usize,
    pub resultant_misses: usize,
    pub discriminant_hits: usize,
    pub discriminant_misses: usize,
    pub prefix_memory_hits: usize,
    pub prefix_disk_hits: usize,
    pub prefix_misses: usize,
    pub invalid_checkpoints: usize,
    pub written_checkpoints: usize,
    pub skipped_degree_zero_pairs: usize,
    pub evictions: usize,
    pub retained_bytes_estimate: usize,
    pub peak_retained_bytes_estimate: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionCheckpoint {
    /// None denotes the initial square-free factor basis.
    pub eliminated_variable: Option<usize>,
    pub key: String,
    pub input_hash: String,
    pub output_hash: String,
    pub polynomial_count: usize,
    pub term_count: usize,
    pub max_degree: u32,
    pub max_coefficient_bits: u64,
    pub elapsed_seconds: f64,
    pub cache_hit: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OptimizedProjectionReport {
    pub cache: CacheMetrics,
    pub checkpoints: Vec<ProjectionCheckpoint>,
    #[serde(default)]
    pub removals: Vec<ProjectionRemoval>,
    /// Exact nonvanishing evidence for input factors omitted before projection.
    #[serde(default)]
    pub initial_removals: Vec<DomainCertificate>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DomainPruningOptions {
    /// Also remove certified input factors, preserving coordinate-affine ones.
    /// Every accepted lifted cell must still satisfy the original constraints.
    pub prune_input: bool,
    pub excluded_sign_constraints: Vec<usize>,
    pub certificates: CertificateOptions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRemoval {
    /// Intermediate output level, before the final positive-degree filter.
    /// The removed factor may depend on only a subset of this level's variables.
    pub level: usize,
    /// Variable index in the stable original variable map.
    pub eliminated_variable: usize,
    /// Polynomial and domain in that same original map, not the lifting order.
    pub certificate: DomainCertificate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointCost {
    pub polynomial_count: usize,
    pub term_count: usize,
    pub max_degree: u32,
    pub max_coefficient_bits: u64,
    pub elapsed_seconds: f64,
}

fn checkpoint_cost(output: &[Poly], elapsed_seconds: f64) -> CheckpointCost {
    CheckpointCost {
        polynomial_count: output.len(),
        term_count: output.iter().map(Poly::nterms).sum(),
        max_degree: output
            .iter()
            .flat_map(|p| (0..p.nvars()).map(|i| p.degree(i)))
            .max()
            .unwrap_or(0),
        max_coefficient_bits: output
            .iter()
            .flat_map(|p| &p.coefficients)
            .map(|c| {
                c.numerator_ref()
                    .significant_bits()
                    .max(c.denominator_ref().significant_bits())
            })
            .max()
            .unwrap_or(0),
        elapsed_seconds,
    }
}

#[derive(Serialize, Deserialize)]
struct CheckpointFile {
    schema_version: u32,
    algorithm: String,
    backend_version: String,
    coefficient_domain: String,
    variable_names: Vec<String>,
    domain_fingerprint: String,
    key: String,
    input_hash: String,
    eliminated_variable: Option<usize>,
    output: Vec<String>,
    output_hash: String,
    cost: CheckpointCost,
}

fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn write_atomic(destination: &Path, bytes: &[u8]) -> Result<()> {
    let directory = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)
        .with_context(|| format!("create output directory {}", directory.display()))?;
    let temporary = directory.join(format!(
        ".symgcad.{}.{}.tmp",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let written = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, destination)?;
        File::open(directory)?.sync_all()?;
        Ok(())
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written.with_context(|| format!("atomically save {}", destination.display()))
}

fn sorted_unique(polynomials: impl IntoIterator<Item = Poly>) -> Vec<Poly> {
    let mut result: Vec<_> = polynomials
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    result.sort_by(|a, b| a.internal_cmp(b));
    result
}

/// Conservatively protect every coordinate-affine input factor. In particular,
/// this includes each original strict affine constraint used by derive_domain,
/// so accepted cells cannot cross a face of the certificate rectangle.
fn is_coordinate_affine(polynomial: &Poly) -> bool {
    let mut support = (0..polynomial.nvars()).filter(|&v| polynomial.degree(v) > 0);
    support.next().is_some_and(|v| polynomial.degree(v) == 1) && support.next().is_none()
}

fn polynomial_bytes(polynomial: &Poly) -> usize {
    let mut bytes = 256usize.saturating_add(polynomial.exponents.len().saturating_mul(4));
    for coefficient in &polynomial.coefficients {
        bytes = bytes.saturating_add(64).saturating_add(
            ((coefficient.numerator_ref().significant_bits()
                + coefficient.denominator_ref().significant_bits())
                / 8
                + 32) as usize,
        );
    }
    bytes
}

fn list_bytes(polynomials: &[Poly]) -> usize {
    polynomials.iter().fold(64usize, |bytes, p| {
        bytes.saturating_add(polynomial_bytes(p))
    })
}

/// Reuse one instance across alternative eliminations in the same variable map.
/// Resultants and factors use canonical exact content keys; completed projection
/// levels additionally have portable cryptographic checkpoint keys.
pub struct ProjectionCache<'a> {
    algebra: &'a Algebra,
    options: OptimizedProjectionOptions,
    factor_cache: HashMap<Poly, Vec<Poly>>,
    resultant_cache: HashMap<(usize, Poly, Poly), Poly>,
    discriminant_cache: HashMap<(usize, Poly), Poly>,
    prefix_cache: HashMap<String, Vec<Poly>>,
    pub report: OptimizedProjectionReport,
}

impl<'a> ProjectionCache<'a> {
    pub fn new(algebra: &'a Algebra, options: &OptimizedProjectionOptions) -> Self {
        Self {
            algebra,
            options: options.clone(),
            factor_cache: HashMap::new(),
            resultant_cache: HashMap::new(),
            discriminant_cache: HashMap::new(),
            prefix_cache: HashMap::new(),
            report: OptimizedProjectionReport::default(),
        }
    }

    fn reserve(&mut self, bytes: usize) -> bool {
        if bytes > self.options.memory_cache_bytes {
            return false;
        }
        if self
            .report
            .cache
            .retained_bytes_estimate
            .saturating_add(bytes)
            > self.options.memory_cache_bytes
        {
            // Replacing maps also releases bucket allocations. Deterministic
            // whole-cache eviction avoids an unbounded bookkeeping structure.
            self.factor_cache = HashMap::new();
            self.resultant_cache = HashMap::new();
            self.discriminant_cache = HashMap::new();
            self.prefix_cache = HashMap::new();
            self.report.cache.retained_bytes_estimate = 0;
            self.report.cache.evictions += 1;
        }
        self.report.cache.retained_bytes_estimate += bytes;
        self.report.cache.peak_retained_bytes_estimate = self
            .report
            .cache
            .peak_retained_bytes_estimate
            .max(self.report.cache.retained_bytes_estimate);
        true
    }

    fn factor_basis(&mut self, polynomials: &[Poly]) -> Result<Vec<Poly>> {
        let mut result = Vec::new();
        for polynomial in polynomials {
            if polynomial.is_constant() {
                continue;
            }
            let polynomial = canonical(polynomial);
            let fs = if let Some(fs) = self.factor_cache.get(&polynomial) {
                self.report.cache.factor_hits += 1;
                fs.clone()
            } else {
                self.report.cache.factor_misses += 1;
                let fs = factors(std::slice::from_ref(&polynomial))?;
                let bytes = 128usize
                    .saturating_add(polynomial_bytes(&polynomial))
                    .saturating_add(list_bytes(&fs));
                if self.reserve(bytes) {
                    self.factor_cache.insert(polynomial, fs.clone());
                }
                fs
            };
            result.extend(fs);
        }
        Ok(sorted_unique(result))
    }

    fn resultant(&mut self, a: &Poly, b: &Poly, variable: usize) -> Result<Poly> {
        let (a, b) = if a.internal_cmp(b).is_gt() {
            (b, a)
        } else {
            (a, b)
        };
        let key = (variable, a.clone(), b.clone());
        if let Some(value) = self.resultant_cache.get(&key) {
            self.report.cache.resultant_hits += 1;
            return Ok(value.clone());
        }
        self.report.cache.resultant_misses += 1;
        let m = a.degree(variable) as u64;
        let n = b.degree(variable) as u64;
        ensure!(
            m <= MAX_DEGREE && n <= MAX_DEGREE,
            "resultant elimination degree exceeds supported range"
        );
        for i in 0..a.nvars() {
            if i != variable {
                ensure!(
                    m * b.degree(i) as u64 + n * a.degree(i) as u64 <= MAX_DEGREE,
                    "resultant degree bound exceeds supported range"
                );
            }
        }
        let value = canonical(&<Q as PolynomialResultant<u32>>::resultant(a, b, variable));
        let bytes = 128usize
            .saturating_add(polynomial_bytes(a))
            .saturating_add(polynomial_bytes(b))
            .saturating_add(polynomial_bytes(&value));
        if self.reserve(bytes) {
            self.resultant_cache.insert(key, value.clone());
        }
        Ok(value)
    }

    fn discriminant(&mut self, polynomial: &Poly, variable: usize) -> Result<Poly> {
        let key = (variable, polynomial.clone());
        if let Some(value) = self.discriminant_cache.get(&key) {
            self.report.cache.discriminant_hits += 1;
            return Ok(value.clone());
        }
        self.report.cache.discriminant_misses += 1;
        let value = canonical(&discriminant(polynomial, variable)?);
        let bytes = 128usize
            .saturating_add(polynomial_bytes(polynomial))
            .saturating_add(polynomial_bytes(&value));
        if self.reserve(bytes) {
            self.discriminant_cache.insert(key, value.clone());
        }
        Ok(value)
    }

    fn key(&self, polynomials: &[Poly], variable: Option<usize>) -> Result<(String, String)> {
        let input = polynomials
            .iter()
            .map(|p| self.algebra.format(p))
            .collect::<Vec<_>>();
        let input_hash = digest(&input)?;
        let key = digest(&(
            CACHE_SCHEMA,
            ALGORITHM,
            LicenseManager::get_version(),
            "Q",
            "symgcad",
            &self.algebra.names,
            &self.options.domain_fingerprint,
            variable,
            &input_hash,
        ))?;
        Ok((key, input_hash))
    }

    fn load_checkpoint(
        &mut self,
        key: &str,
        input_hash: &str,
        variable: Option<usize>,
        inputs: &[Poly],
    ) -> Result<Option<Vec<Poly>>> {
        let Some(directory) = &self.options.checkpoint_dir else {
            return Ok(None);
        };
        let path = directory.join(format!("{key}.json"));
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                self.report.cache.invalid_checkpoints += 1;
                return Ok(None);
            }
        };
        let loaded = (|| -> Result<Vec<Poly>> {
            ensure!(
                metadata.len() <= self.options.max_checkpoint_bytes as u64,
                "oversized checkpoint"
            );
            let saved: CheckpointFile = serde_json::from_slice(&fs::read(&path)?)?;
            ensure!(
                saved.schema_version == CACHE_SCHEMA && saved.algorithm == ALGORITHM,
                "checkpoint schema/algorithm mismatch"
            );
            ensure!(
                saved.backend_version == LicenseManager::get_version()
                    && saved.coefficient_domain == "Q",
                "checkpoint backend/domain mismatch"
            );
            ensure!(
                saved.variable_names == self.algebra.names
                    && saved.domain_fingerprint == self.options.domain_fingerprint,
                "checkpoint variable/domain mismatch"
            );
            ensure!(
                saved.key == key
                    && saved.input_hash == input_hash
                    && saved.eliminated_variable == variable,
                "checkpoint input key mismatch"
            );
            ensure!(
                digest(&saved.output)? == saved.output_hash,
                "checkpoint output checksum mismatch"
            );
            let output = saved
                .output
                .iter()
                .map(|text| self.algebra.parse(text))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                output
                    .iter()
                    .all(|p| !p.is_constant() && canonical(p) == *p),
                "checkpoint output is not a primitive nonconstant basis"
            );
            if let Some(variable) = variable {
                ensure!(
                    output.iter().all(|p| p.degree(variable) == 0),
                    "checkpoint retained the eliminated variable"
                );
            }
            for variable in 0..self.algebra.names.len() {
                if inputs.iter().all(|p| p.degree(variable) == 0) {
                    ensure!(
                        output.iter().all(|p| p.degree(variable) == 0),
                        "checkpoint introduced a variable absent from its inputs"
                    );
                }
            }
            ensure!(
                sorted_unique(output.clone()) == output,
                "checkpoint basis order or duplicates invalid"
            );
            let cost = checkpoint_cost(&output, saved.cost.elapsed_seconds);
            ensure!(
                cost.elapsed_seconds.is_finite() && cost.elapsed_seconds >= 0.0,
                "invalid checkpoint time"
            );
            ensure!(
                cost.polynomial_count == saved.cost.polynomial_count
                    && cost.term_count == saved.cost.term_count
                    && cost.max_degree == saved.cost.max_degree
                    && cost.max_coefficient_bits == saved.cost.max_coefficient_bits,
                "checkpoint cost metadata differs from its polynomials"
            );
            Ok(output)
        })();
        match loaded {
            Ok(output) => {
                self.report.cache.prefix_disk_hits += 1;
                Ok(Some(output))
            }
            Err(_) => {
                self.report.cache.invalid_checkpoints += 1;
                Ok(None)
            }
        }
    }

    fn save_checkpoint(
        &mut self,
        key: &str,
        input_hash: &str,
        variable: Option<usize>,
        output: &[Poly],
        elapsed_seconds: f64,
    ) -> Result<()> {
        let Some(directory) = &self.options.checkpoint_dir else {
            return Ok(());
        };
        let cost = checkpoint_cost(output, elapsed_seconds);
        let output = output
            .iter()
            .map(|p| self.algebra.format(p))
            .collect::<Vec<_>>();
        if output
            .iter()
            .map(String::len)
            .sum::<usize>()
            .saturating_add(4096)
            > self.options.max_checkpoint_bytes
        {
            return Ok(());
        }
        let saved = CheckpointFile {
            schema_version: CACHE_SCHEMA,
            algorithm: ALGORITHM.into(),
            backend_version: LicenseManager::get_version().into(),
            coefficient_domain: "Q".into(),
            variable_names: self.algebra.names.clone(),
            domain_fingerprint: self.options.domain_fingerprint.clone(),
            key: key.into(),
            input_hash: input_hash.into(),
            eliminated_variable: variable,
            output_hash: digest(&output)?,
            output,
            cost,
        };
        let bytes = serde_json::to_vec(&saved)?;
        if bytes.len() > self.options.max_checkpoint_bytes {
            return Ok(());
        }
        let destination = directory.join(format!("{key}.json"));
        write_atomic(&destination, &bytes)?;
        self.report.cache.written_checkpoints += 1;
        Ok(())
    }

    fn save_progress(&self) -> Result<()> {
        if let Some(path) = &self.options.progress_path {
            write_atomic(path, &serde_json::to_vec(&self.report)?)?;
        }
        Ok(())
    }

    fn completed(
        &mut self,
        key: String,
        input_hash: String,
        variable: Option<usize>,
        output: &[Poly],
        started: Instant,
        hit: bool,
    ) -> Result<()> {
        let output_hash = digest(
            &output
                .iter()
                .map(|p| self.algebra.format(p))
                .collect::<Vec<_>>(),
        )?;
        self.report.checkpoints.push(ProjectionCheckpoint {
            eliminated_variable: variable,
            key: key.clone(),
            input_hash,
            output_hash,
            polynomial_count: output.len(),
            term_count: output.iter().map(Poly::nterms).sum(),
            max_degree: output
                .iter()
                .flat_map(|p| (0..p.nvars()).map(|i| p.degree(i)))
                .max()
                .unwrap_or(0),
            max_coefficient_bits: output
                .iter()
                .flat_map(|p| &p.coefficients)
                .map(|c| {
                    c.numerator_ref()
                        .significant_bits()
                        .max(c.denominator_ref().significant_bits())
                })
                .max()
                .unwrap_or(0),
            elapsed_seconds: started.elapsed().as_secs_f64(),
            cache_hit: hit,
        });
        if !self.prefix_cache.contains_key(&key)
            && self.reserve(
                128usize
                    .saturating_add(key.len())
                    .saturating_add(list_bytes(output)),
            )
        {
            self.prefix_cache.insert(key, output.to_vec());
        }
        self.save_progress()
    }

    fn step(&mut self, polynomials: &[Poly], variable: Option<usize>) -> Result<Vec<Poly>> {
        ensure!(
            polynomials
                .iter()
                .all(|p| p.variables() == &self.algebra.variables),
            "optimized projection input has a different variable map"
        );
        if let Some(variable) = variable {
            ensure!(
                variable < self.algebra.names.len(),
                "projection variable out of range"
            );
        }
        let started = Instant::now();
        let polynomials = sorted_unique(
            polynomials
                .iter()
                .filter(|p| !p.is_constant())
                .map(canonical),
        );
        let (key, input_hash) = self.key(&polynomials, variable)?;
        let cached = if let Some(output) = self.prefix_cache.get(&key) {
            self.report.cache.prefix_memory_hits += 1;
            Some(output.clone())
        } else {
            self.load_checkpoint(&key, &input_hash, variable, &polynomials)?
        };
        if let Some(output) = cached {
            self.completed(key, input_hash, variable, &output, started, true)?;
            return Ok(output);
        }
        self.report.cache.prefix_misses += 1;
        let output = if let Some(variable) = variable {
            let mut raw = Vec::new();
            for (i, p) in polynomials.iter().enumerate() {
                raw.push(p.univariate_lcoeff(variable));
                let degree = p.degree(variable);
                if degree >= 2 {
                    raw.push(self.discriminant(p, variable)?);
                }
                for other in &polynomials[i + 1..] {
                    if degree == 0 || other.degree(variable) == 0 {
                        // Res(c,q)=c^deg(q): c is already present as a leading
                        // coefficient, so its irreducible factors are retained.
                        self.report.cache.skipped_degree_zero_pairs += 1;
                    } else {
                        raw.push(self.resultant(p, other, variable)?);
                    }
                }
            }
            let raw = sorted_unique(
                raw.into_iter()
                    .filter(|p| !p.is_constant())
                    .map(|p| canonical(&p)),
            );
            self.factor_basis(&raw)?
        } else {
            self.factor_basis(&polynomials)?
        };
        self.save_checkpoint(
            &key,
            &input_hash,
            variable,
            &output,
            started.elapsed().as_secs_f64(),
        )?;
        self.completed(key, input_hash, variable, &output, started, false)?;
        Ok(output)
    }

    pub fn initial_basis(&mut self, polynomials: &[Poly]) -> Result<Vec<Poly>> {
        self.step(polynomials, None)
    }

    /// Obtain the initial basis used by certified projection, retaining its
    /// cache and initial proof evidence for ordering exploration and replay.
    pub fn initial_basis_certified(
        &mut self,
        original_constraints: &[Poly],
        split: &[Poly],
        pruning: &DomainPruningOptions,
    ) -> Result<Vec<Poly>> {
        let domain = derive_domain(
            self.algebra,
            original_constraints,
            &pruning.excluded_sign_constraints,
        )?;
        self.report.initial_removals.clear();
        self.report.removals.clear();
        let all = original_constraints
            .iter()
            .chain(split)
            .cloned()
            .collect::<Vec<_>>();
        let initial = self.initial_basis(&all)?;
        if !pruning.prune_input {
            return Ok(initial);
        }
        let mut retained = Vec::with_capacity(initial.len());
        for polynomial in initial {
            if !is_coordinate_affine(&polynomial)
                && let Some(certificate) =
                    certify_nonzero(self.algebra, &polynomial, &domain, &pruning.certificates)?
            {
                self.report.initial_removals.push(certificate);
            } else {
                retained.push(polynomial);
            }
        }
        self.save_progress()?;
        Ok(retained)
    }

    /// Input must be a square-free pairwise-coprime factor basis, as for PR.
    pub fn project_one(&mut self, polynomials: &[Poly], variable: usize) -> Result<Vec<Poly>> {
        self.step(polynomials, Some(variable))
    }

    /// Retain the same cache while choosing an order and doing final projection.
    /// Projection arithmetic stays in the original map; returned polynomials
    /// are remapped to the requested lifting order.
    pub fn project_ordered(&mut self, polynomials: &[Poly], order: &[usize]) -> Result<Projection> {
        validate_projection_input(self.algebra, polynomials, order)?;
        self.report.removals.clear();
        self.report.initial_removals.clear();
        if order.is_empty() {
            return Ok(Projection {
                levels: vec![],
                steps: vec![],
            });
        }
        let initial = self.initial_basis(polynomials)?;
        self.project_from_initial(initial, order, |_, _, generated| Ok(generated))
    }

    /// Certified variant retaining the algebra cache used during ordering.
    /// The constructor's domain fingerprint should identify the original
    /// constraints/splits; checkpoint payloads themselves remain unpruned.
    pub fn project_ordered_certified(
        &mut self,
        original_constraints: &[Poly],
        split: &[Poly],
        order: &[usize],
        pruning: &DomainPruningOptions,
    ) -> Result<Projection> {
        let algebra = self.algebra;
        let all = original_constraints
            .iter()
            .chain(split)
            .cloned()
            .collect::<Vec<_>>();
        validate_projection_input(algebra, &all, order)?;
        let domain = derive_domain(
            algebra,
            original_constraints,
            &pruning.excluded_sign_constraints,
        )?;
        self.report.removals.clear();
        self.report.initial_removals.clear();
        if order.is_empty() {
            return Ok(Projection {
                levels: vec![],
                steps: vec![],
            });
        }
        let initial = self.initial_basis_certified(original_constraints, split, pruning)?;
        let protected = initial.iter().cloned().collect::<HashSet<_>>();
        let mut removals = Vec::new();
        let projection =
            self.project_from_initial(initial, order, |level, eliminated_variable, generated| {
                let mut retained = Vec::with_capacity(generated.len());
                for polynomial in generated {
                    if !protected.contains(&polynomial)
                        && let Some(certificate) =
                            certify_nonzero(algebra, &polynomial, &domain, &pruning.certificates)?
                    {
                        removals.push(ProjectionRemoval {
                            level,
                            eliminated_variable,
                            certificate,
                        });
                    } else {
                        retained.push(polynomial);
                    }
                }
                Ok(retained)
            })?;
        self.report.removals = removals;
        self.save_progress()?;
        Ok(projection)
    }

    fn project_from_initial(
        &mut self,
        initial: Vec<Poly>,
        order: &[usize],
        mut retain: impl FnMut(usize, usize, Vec<Poly>) -> Result<Vec<Poly>>,
    ) -> Result<Projection> {
        let n = order.len();
        let mut levels = vec![vec![]; n];
        let mut steps = Vec::with_capacity(n.saturating_sub(1));
        if n == 0 {
            return Ok(Projection { levels, steps });
        }
        levels[n - 1] = initial;
        for position in (1..n).rev() {
            let started = Instant::now();
            let variable = order[position];
            let generated = self.project_one(&levels[position], variable)?;
            let output = retain(position - 1, variable, generated)?;
            steps.push(ProjectionStep {
                variable: position,
                input_count: levels[position].len(),
                output_count: output.len(),
                input_terms: levels[position].iter().map(Poly::nterms).sum(),
                output_terms: output.iter().map(Poly::nterms).sum(),
                elapsed_seconds: started.elapsed().as_secs_f64(),
            });
            levels[position - 1] = output;
        }
        finish_projection(self.algebra, levels, steps, order)
    }
}

fn validate_projection_input(
    algebra: &Algebra,
    polynomials: &[Poly],
    order: &[usize],
) -> Result<()> {
    ensure!(
        polynomials
            .iter()
            .all(|p| p.variables() == &algebra.variables),
        "optimized projection input has a different variable map"
    );
    let n = algebra.names.len();
    ensure!(
        order.len() == n
            && order.iter().all(|&i| i < n)
            && order.iter().copied().collect::<HashSet<_>>().len() == n,
        "projection order must be a permutation of the variable indices"
    );
    Ok(())
}

fn finish_projection(
    algebra: &Algebra,
    mut levels: Vec<Vec<Poly>>,
    steps: Vec<ProjectionStep>,
    order: &[usize],
) -> Result<Projection> {
    let variables = order
        .iter()
        .map(|&i| algebra.variables[i].clone())
        .collect::<Vec<_>>();
    let reordered = order.iter().enumerate().any(|(i, &j)| i != j);
    for (position, level) in levels.iter_mut().enumerate() {
        level.retain(|p| p.degree(order[position]) > 0);
        if reordered {
            *level = sorted_unique(
                level
                    .iter()
                    .map(|p| {
                        p.rearrange_with_growth(&variables)
                            .map(|p| canonical(&p))
                            .map_err(anyhow::Error::msg)
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
        }
    }
    Ok(Projection { levels, steps })
}

pub fn project_optimized(
    algebra: &Algebra,
    polynomials: &[Poly],
    options: &OptimizedProjectionOptions,
) -> Result<(Projection, OptimizedProjectionReport)> {
    project_ordered_optimized(
        algebra,
        polynomials,
        &(0..algebra.names.len()).collect::<Vec<_>>(),
        options,
    )
}

/// Project in a stable original variable map, sharing prefix checkpoints
/// between order trials. Returned levels and polynomials use the requested
/// lifting order, so they can be consumed by the ordinary lifter/verifier.
/// Checkpoint eliminated-variable indices refer to the original map.
pub fn project_ordered_optimized(
    algebra: &Algebra,
    polynomials: &[Poly],
    order: &[usize],
    options: &OptimizedProjectionOptions,
) -> Result<(Projection, OptimizedProjectionReport)> {
    let mut cache = ProjectionCache::new(algebra, options);
    let projection = cache.project_ordered(polynomials, order)?;
    Ok((projection, cache.report))
}

/// Prune generated factors (and optionally input factors) proved nonzero on a
/// rectangular superset of the original domain. A factor independent of eliminated variables
/// inherits this certificate on the projected rectangle. Original coordinate
/// boundaries remain protected, so full-real-line lifting cannot accept a cell
/// crossing out of the rectangle on which its projection was certified.
pub fn project_ordered_certified(
    algebra: &Algebra,
    original_constraints: &[Poly],
    split: &[Poly],
    order: &[usize],
    options: &OptimizedProjectionOptions,
    pruning: &DomainPruningOptions,
) -> Result<(Projection, OptimizedProjectionReport)> {
    let all = original_constraints
        .iter()
        .chain(split)
        .cloned()
        .collect::<Vec<_>>();
    validate_projection_input(algebra, &all, order)?;
    let domain = derive_domain(
        algebra,
        original_constraints,
        &pruning.excluded_sign_constraints,
    )?;
    let mut options = options.clone();
    options.domain_fingerprint = digest(&(
        "certified-factors-v2",
        &options.domain_fingerprint,
        original_constraints
            .iter()
            .map(|p| algebra.format(p))
            .collect::<Vec<_>>(),
        split.iter().map(|p| algebra.format(p)).collect::<Vec<_>>(),
        &domain,
        pruning.prune_input,
        &pruning.certificates,
    ))?;
    let mut cache = ProjectionCache::new(algebra, &options);
    let projection =
        cache.project_ordered_certified(original_constraints, split, order, pruning)?;
    Ok((projection, cache.report))
}

/// Independently replay the exact PR/SFRP obligations at each retained stage.
/// This path does not read caches or rerun certificate discovery. It verifies
/// each recorded witness and rejects removal of any original factor.
pub fn replay_certified_projection(
    algebra: &Algebra,
    original_constraints: &[Poly],
    split: &[Poly],
    order: &[usize],
    excluded_sign_constraints: &[usize],
    removals: &[ProjectionRemoval],
) -> Result<Projection> {
    replay_certified_projection_with_inputs(
        algebra,
        original_constraints,
        split,
        order,
        excluded_sign_constraints,
        &[],
        removals,
    )
}

/// Replay initial-factor certificates followed by the exact retained PR/SFRP
/// obligations. Coordinate-affine input factors cannot be omitted, even with
/// a valid strict-domain nonvanishing certificate, because lifting needs the
/// original rectangular faces. No caches or certificate discovery are used.
pub fn replay_certified_projection_with_inputs(
    algebra: &Algebra,
    original_constraints: &[Poly],
    split: &[Poly],
    order: &[usize],
    excluded_sign_constraints: &[usize],
    initial_removals: &[DomainCertificate],
    removals: &[ProjectionRemoval],
) -> Result<Projection> {
    let all = original_constraints
        .iter()
        .chain(split)
        .cloned()
        .collect::<Vec<_>>();
    validate_projection_input(algebra, &all, order)?;
    derive_domain(algebra, original_constraints, excluded_sign_constraints)?;
    let n = order.len();
    for removal in removals {
        ensure!(
            removal.level < n.saturating_sub(1),
            "certificate removal names an invalid projection level"
        );
        ensure!(
            removal.eliminated_variable == order[removal.level + 1],
            "certificate removal names the wrong elimination variable"
        );
    }
    if n == 0 {
        ensure!(
            initial_removals.is_empty(),
            "zero-dimensional projection has no input factors to remove"
        );
        return Ok(Projection {
            levels: vec![],
            steps: vec![],
        });
    }
    let mut initial = factors_checked(&all)?;
    let mut removed = HashSet::new();
    for certificate in initial_removals {
        let polynomial = algebra.parse(&certificate.polynomial)?;
        ensure!(
            initial.contains(&polynomial),
            "initial certificate removes a factor absent from the input basis"
        );
        ensure!(
            !is_coordinate_affine(&polynomial),
            "initial certificate cannot remove a coordinate-affine domain boundary factor"
        );
        ensure!(
            removed.insert(polynomial.clone()),
            "duplicate initial factor removal"
        );
        verify_certificate(
            algebra,
            &polynomial,
            original_constraints,
            excluded_sign_constraints,
            certificate,
        )?;
    }
    initial.retain(|p| !removed.contains(p));
    let protected = initial.iter().cloned().collect::<HashSet<_>>();
    let mut levels = vec![vec![]; n];
    let mut steps = Vec::with_capacity(n - 1);
    levels[n - 1] = initial;
    for position in (1..n).rev() {
        let started = Instant::now();
        let variable = order[position];
        let generated = factors_checked(&crate::projection::projection_polynomials(
            &levels[position],
            variable,
        )?)?;
        let mut removed = HashSet::new();
        for removal in removals.iter().filter(|r| r.level == position - 1) {
            let polynomial = algebra.parse(&removal.certificate.polynomial)?;
            ensure!(
                generated.contains(&polynomial),
                "certificate removes a factor absent from this projection obligation"
            );
            ensure!(
                !protected.contains(&polynomial),
                "certificate cannot remove an original constraint or split factor"
            );
            ensure!(
                removed.insert(polynomial.clone()),
                "duplicate projection-factor removal"
            );
            verify_certificate(
                algebra,
                &polynomial,
                original_constraints,
                excluded_sign_constraints,
                &removal.certificate,
            )?;
        }
        let retained = generated
            .into_iter()
            .filter(|p| !removed.contains(p))
            .collect::<Vec<_>>();
        steps.push(ProjectionStep {
            variable: position,
            input_count: levels[position].len(),
            output_count: retained.len(),
            input_terms: levels[position].iter().map(Poly::nterms).sum(),
            output_terms: retained.iter().map(Poly::nterms).sum(),
            elapsed_seconds: started.elapsed().as_secs_f64(),
        });
        levels[position - 1] = retained;
    }
    finish_projection(algebra, levels, steps, order)
}
