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
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symbolica::{domains::InternalOrdering, prelude::*};

use crate::{
    algebra::{Algebra, MAX_DEGREE, Poly, canonical, primitive_linear_variable},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    projection::{
        Projection, ProjectionStep, discriminant, exact_resultant, exact_resultant_with_statistics,
    },
};

const CACHE_SCHEMA: u32 = 1;
// Exact semantic identity remains compatible with deterministic Ducos v2.
// Earlier automatic/Monte Carlo keys stay invalid. Kernel performance metadata
// is separate so an exact algorithm change does not discard completed algebra.
const ALGORITHM: &str = "symgcad-deterministic-integer-ducos-gproj-v2";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OptimizedProjectionOptions {
    pub checkpoint_dir: Option<PathBuf>,
    /// Atomically replaced after each completed algebra stage. This report
    /// remains readable if a bounded ordering worker is subsequently killed.
    /// It has no effect on the persistent algebra cache keys.
    pub progress_path: Option<PathBuf>,
    /// Opt-in latest-operation snapshots, one atomic file per worker process.
    /// An interrupted operation retains its exact operands for profiling.
    pub capture_dir: Option<PathBuf>,
    pub max_capture_bytes: usize,
    /// Conservative modeled retention budget; process RSS limits remain the
    /// hard bound, including transient Symbolica allocations and the result.
    pub memory_cache_bytes: usize,
    pub max_checkpoint_bytes: usize,
    /// Opt-in exact binary segmentation for bounded pair resultants and
    /// one- or two-parameter discriminants after interpolation statically declines.
    pub binary_resultants: bool,
    /// Include the complete normalized domain when domain certificates are
    /// enabled by the caller. The default algorithm does not use a domain.
    pub domain_fingerprint: String,
}

impl Default for OptimizedProjectionOptions {
    fn default() -> Self {
        Self {
            checkpoint_dir: None,
            progress_path: None,
            capture_dir: None,
            max_capture_bytes: 16 * 1024 * 1024,
            memory_cache_bytes: 128 * 1024 * 1024,
            max_checkpoint_bytes: 512 * 1024 * 1024,
            binary_resultants: false,
            domain_fingerprint: "unrestricted".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheMetrics {
    /// Exact primitive-linear identities computed without persisting an
    /// operation. Historical reports deserialize this as zero.
    pub primitive_linear_identities: usize,
    pub factor_hits: usize,
    pub factor_misses: usize,
    pub resultant_hits: usize,
    pub resultant_misses: usize,
    pub interpolated_resultants: usize,
    pub interpolation_images: usize,
    pub segmented_resultants: usize,
    pub segmented_discriminants: usize,
    pub maximum_segmented_output_bits: usize,
    pub discriminant_hits: usize,
    pub discriminant_misses: usize,
    pub factor_disk_hits: usize,
    pub resultant_disk_hits: usize,
    pub discriminant_disk_hits: usize,
    pub invalid_operation_checkpoints: usize,
    pub written_operation_checkpoints: usize,
    /// Validated records read under the pre-v2 domain-keyed name and migrated
    /// to the domain-free namespace. Included in the per-operation disk hits.
    pub legacy_operation_disk_hits: usize,
    pub oversized_captures: usize,
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
    /// Historical reports omit this performance metadata; cache semantics are
    /// still the deterministic-v2 identity above.
    #[serde(default)]
    pub resultant_kernel: String,
    pub cache: CacheMetrics,
    pub checkpoints: Vec<ProjectionCheckpoint>,
    #[serde(default)]
    pub removals: Vec<ProjectionRemoval>,
    /// Exact nonvanishing evidence for input factors omitted before projection.
    #[serde(default)]
    pub initial_removals: Vec<DomainCertificate>,
    /// Whole raw projection obligations proved nonzero before factorization.
    #[serde(default)]
    pub operation_removals: Vec<ProjectionOperationRemoval>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DomainPruningOptions {
    /// Also remove certified input factors, preserving coordinate-affine ones.
    /// Every accepted lifted cell must still satisfy the original constraints.
    pub prune_input: bool,
    /// Certify raw projection obligations before invoking factorization.
    pub prune_operations: bool,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionOperationRemoval {
    pub level: usize,
    pub eliminated_variable: usize,
    /// Canonical raw LC/discriminant/resultant, before square-free factoring.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlgebraOperation {
    Factor,
    Resultant,
    Discriminant,
}

#[derive(Clone, Serialize, Deserialize)]
struct OperationFile {
    schema_version: u32,
    algorithm: String,
    backend_version: String,
    coefficient_domain: String,
    variable_names: Vec<String>,
    domain_fingerprint: String,
    operation: AlgebraOperation,
    variable: Option<usize>,
    key: String,
    input_hash: String,
    output: Vec<String>,
    output_hash: String,
    multiplicities: Option<Vec<usize>>,
    elapsed_seconds: f64,
}

/// Operation records are pure algebra: a canonical factorization, resultant or
/// discriminant of canonical inputs, computed before any domain certificate,
/// pair omission or positivity filter is applied. Their identity is therefore
/// the full variable map, backend/algorithm version, operation, eliminated
/// axis and exact input, not the domain. Records of this namespace store this
/// constant in place of a domain fingerprint.
const OPERATION_NAMESPACE: &str = "algebra-operation-v2-domain-free";
const OPERATION_DOMAIN: &str = "domain-free-pure-algebra";

#[derive(Clone)]
struct OperationRequest {
    operation: AlgebraOperation,
    variable: Option<usize>,
    key: String,
    /// Pre-v2 records were keyed by the caller's domain fingerprint as well.
    /// They are still read, validated under their original header, and
    /// re-saved under the domain-free key.
    legacy_key: String,
    input_hash: String,
    input: Vec<String>,
}

/// A profiling aid, not an algebra proof. A killed worker leaves complete=false.
/// Oversized operands are omitted explicitly; the key and input hash remain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationCapture {
    pub schema_version: u32,
    pub backend_version: String,
    pub coefficient_domain: String,
    pub variable_names: Vec<String>,
    pub operation: AlgebraOperation,
    pub variable: Option<usize>,
    pub key: String,
    pub input_hash: String,
    pub operands: Option<Vec<String>>,
    pub operands_omitted_for_size: bool,
    pub complete: bool,
    pub elapsed_seconds: Option<f64>,
    pub output_hash: Option<String>,
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

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "checkpoint changed to an oversized file while reading"
    );
    Ok(bytes)
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
            report: OptimizedProjectionReport {
                resultant_kernel: if options.binary_resultants {
                    "deterministic-ducos-bivariate-newton-v1-binary-segmentation-v1".into()
                } else {
                    "deterministic-ducos-bivariate-newton-v1".into()
                },
                ..Default::default()
            },
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

    fn operation_request(
        &self,
        operation: AlgebraOperation,
        variable: Option<usize>,
        inputs: &[&Poly],
    ) -> Result<Option<OperationRequest>> {
        if self.options.checkpoint_dir.is_none() && self.options.capture_dir.is_none() {
            return Ok(None);
        }
        let input = inputs
            .iter()
            .map(|p| self.algebra.format(p))
            .collect::<Vec<_>>();
        let input_hash = digest(&input)?;
        let key = digest(&(
            CACHE_SCHEMA,
            ALGORITHM,
            OPERATION_NAMESPACE,
            operation,
            variable,
            LicenseManager::get_version(),
            "Q",
            "symgcad",
            &self.algebra.names,
            &input_hash,
        ))?;
        let legacy_key = digest(&(
            CACHE_SCHEMA,
            ALGORITHM,
            "algebra-operation-v1",
            operation,
            variable,
            LicenseManager::get_version(),
            "Q",
            "symgcad",
            &self.algebra.names,
            &self.options.domain_fingerprint,
            &input_hash,
        ))?;
        Ok(Some(OperationRequest {
            operation,
            variable,
            key,
            legacy_key,
            input_hash,
            input,
        }))
    }

    fn load_operation(
        &mut self,
        request: &Option<OperationRequest>,
        inputs: &[&Poly],
    ) -> Result<Option<Vec<Poly>>> {
        let (Some(directory), Some(request)) = (&self.options.checkpoint_dir, request) else {
            return Ok(None);
        };
        let directory = directory.join("operations");
        let current = directory.join(format!("{}.json", request.key));
        if let Some(output) =
            self.load_operation_file(&current, request, &request.key, OPERATION_DOMAIN, inputs)?
        {
            return Ok(Some(output.0));
        }
        let legacy = directory.join(format!("{}.json", request.legacy_key));
        let domain = self.options.domain_fingerprint.clone();
        let Some((output, multiplicities, elapsed)) =
            self.load_operation_file(&legacy, request, &request.legacy_key, &domain, inputs)?
        else {
            return Ok(None);
        };
        self.report.cache.legacy_operation_disk_hits += 1;
        // Migrate the validated pure-algebra record to the shared namespace.
        self.save_operation(&Some(request.clone()), &output, multiplicities, elapsed)?;
        Ok(Some(output))
    }

    /// Validate one record under its own key and header. Missing or invalid
    /// files return None; an invalid file is counted and never trusted.
    fn load_operation_file(
        &mut self,
        path: &Path,
        request: &OperationRequest,
        key: &str,
        domain_fingerprint: &str,
        inputs: &[&Poly],
    ) -> Result<Option<(Vec<Poly>, Option<Vec<usize>>, f64)>> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                self.report.cache.invalid_operation_checkpoints += 1;
                return Ok(None);
            }
        };
        let loaded = (|| -> Result<(Vec<Poly>, Option<Vec<usize>>, f64)> {
            ensure!(
                metadata.len() <= self.options.max_checkpoint_bytes as u64,
                "oversized operation checkpoint"
            );
            let saved: OperationFile =
                serde_json::from_slice(&read_bounded(path, self.options.max_checkpoint_bytes)?)?;
            ensure!(
                saved.schema_version == CACHE_SCHEMA && saved.algorithm == ALGORITHM,
                "operation schema/algorithm mismatch"
            );
            ensure!(
                saved.backend_version == LicenseManager::get_version()
                    && saved.coefficient_domain == "Q",
                "operation backend/domain mismatch"
            );
            ensure!(
                saved.variable_names == self.algebra.names
                    && saved.domain_fingerprint == domain_fingerprint,
                "operation variable/domain mismatch"
            );
            ensure!(
                saved.operation == request.operation
                    && saved.variable == request.variable
                    && saved.key == key
                    && saved.input_hash == request.input_hash,
                "operation input key mismatch"
            );
            ensure!(
                saved.elapsed_seconds.is_finite() && saved.elapsed_seconds >= 0.0,
                "invalid operation elapsed time"
            );
            ensure!(
                digest(&(&saved.output, &saved.multiplicities))? == saved.output_hash,
                "operation output checksum mismatch"
            );
            let output = saved
                .output
                .iter()
                .map(|s| self.algebra.parse(s))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                output.iter().all(|p| canonical(p) == *p),
                "operation output is not canonical"
            );
            for variable in 0..self.algebra.names.len() {
                if inputs.iter().all(|p| p.degree(variable) == 0)
                    || request.variable == Some(variable)
                {
                    ensure!(
                        output.iter().all(|p| p.degree(variable) == 0),
                        "operation output contains an absent or eliminated variable"
                    );
                }
            }
            match request.operation {
                AlgebraOperation::Factor => {
                    ensure!(
                        !output.is_empty() && output.iter().all(|p| !p.is_constant()),
                        "invalid factor basis"
                    );
                    ensure!(
                        output.windows(2).all(|p| p[0].internal_cmp(&p[1]).is_lt()),
                        "factor order or duplicates invalid"
                    );
                    let multiplicities = saved
                        .multiplicities
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("factor multiplicities missing"))?;
                    ensure!(
                        multiplicities.len() == output.len()
                            && multiplicities.iter().all(|&m| m > 0),
                        "invalid factor multiplicities"
                    );
                    for variable in 0..self.algebra.names.len() {
                        let degree =
                            output
                                .iter()
                                .zip(multiplicities)
                                .try_fold(0u64, |degree, (p, &m)| {
                                    degree.checked_add(
                                        u64::from(p.degree(variable))
                                            .checked_mul(u64::try_from(m).ok()?)?,
                                    )
                                });
                        ensure!(
                            degree == Some(u64::from(inputs[0].degree(variable))),
                            "factor reconstruction degree mismatch"
                        );
                    }
                    let mut product = inputs[0].one();
                    for (factor, &multiplicity) in output.iter().zip(multiplicities) {
                        product = &product * &factor.pow(multiplicity);
                    }
                    ensure!(
                        canonical(&product) == *inputs[0],
                        "cached factor reconstruction failed"
                    );
                }
                AlgebraOperation::Resultant | AlgebraOperation::Discriminant => {
                    ensure!(
                        output.len() == 1 && saved.multiplicities.is_none(),
                        "invalid scalar operation output"
                    );
                }
            }
            Ok((output, saved.multiplicities, saved.elapsed_seconds))
        })();
        match loaded {
            Ok(output) => {
                match request.operation {
                    AlgebraOperation::Factor => self.report.cache.factor_disk_hits += 1,
                    AlgebraOperation::Resultant => self.report.cache.resultant_disk_hits += 1,
                    AlgebraOperation::Discriminant => self.report.cache.discriminant_disk_hits += 1,
                }
                Ok(Some(output))
            }
            Err(_) => {
                self.report.cache.invalid_operation_checkpoints += 1;
                Ok(None)
            }
        }
    }

    fn save_operation(
        &mut self,
        request: &Option<OperationRequest>,
        output: &[Poly],
        multiplicities: Option<Vec<usize>>,
        elapsed_seconds: f64,
    ) -> Result<()> {
        let (Some(directory), Some(request)) = (&self.options.checkpoint_dir, request) else {
            return Ok(());
        };
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
        let saved = OperationFile {
            schema_version: CACHE_SCHEMA,
            algorithm: ALGORITHM.into(),
            backend_version: LicenseManager::get_version().into(),
            coefficient_domain: "Q".into(),
            variable_names: self.algebra.names.clone(),
            domain_fingerprint: OPERATION_DOMAIN.into(),
            operation: request.operation,
            variable: request.variable,
            key: request.key.clone(),
            input_hash: request.input_hash.clone(),
            output_hash: digest(&(&output, &multiplicities))?,
            output,
            multiplicities,
            elapsed_seconds,
        };
        let bytes = serde_json::to_vec(&saved)?;
        if bytes.len() <= self.options.max_checkpoint_bytes {
            write_atomic(
                &directory
                    .join("operations")
                    .join(format!("{}.json", request.key)),
                &bytes,
            )?;
            self.report.cache.written_operation_checkpoints += 1;
        }
        Ok(())
    }

    fn capture_operation(
        &mut self,
        request: &Option<OperationRequest>,
        completed: Option<(&[Poly], f64)>,
    ) -> Result<()> {
        let (Some(directory), Some(request)) = (&self.options.capture_dir, request) else {
            return Ok(());
        };
        let mut capture = OperationCapture {
            schema_version: CACHE_SCHEMA,
            backend_version: LicenseManager::get_version().into(),
            coefficient_domain: "Q".into(),
            variable_names: self.algebra.names.clone(),
            operation: request.operation,
            variable: request.variable,
            key: request.key.clone(),
            input_hash: request.input_hash.clone(),
            operands: Some(request.input.clone()),
            operands_omitted_for_size: false,
            complete: completed.is_some(),
            elapsed_seconds: completed.map(|(_, seconds)| seconds),
            output_hash: completed
                .map(|(output, _)| {
                    digest(
                        &output
                            .iter()
                            .map(|p| self.algebra.format(p))
                            .collect::<Vec<_>>(),
                    )
                })
                .transpose()?,
        };
        let mut bytes = serde_json::to_vec(&capture)?;
        if bytes.len() > self.options.max_capture_bytes {
            capture.operands = None;
            capture.operands_omitted_for_size = true;
            if completed.is_none() {
                self.report.cache.oversized_captures += 1;
            }
            bytes = serde_json::to_vec(&capture)?;
        }
        let destination = directory.join(format!("active-{}.json", std::process::id()));
        if bytes.len() <= self.options.max_capture_bytes {
            write_atomic(&destination, &bytes)?;
        } else {
            // Do not leave an unrelated prior operation looking current.
            match fs::remove_file(&destination) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    /// Canonical square-free factor basis in this cache's fixed variable map.
    /// Cached factors preserve zero sets; independent proof replay must still
    /// reconstruct the requested factorization without trusting this cache.
    pub fn factor_basis(&mut self, polynomials: &[Poly]) -> Result<Vec<Poly>> {
        ensure!(
            polynomials
                .iter()
                .all(|p| p.variables() == &self.algebra.variables),
            "factor input has a different cache variable map"
        );
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
                // The independent factorization path proves this same exact
                // irreducibility identity. Avoid formatting/persisting its
                // trivial answer, but preserve ordinary warm memory hits and
                // bounded retention. Limit the added work on nonlinear misses.
                let fs = if polynomial
                    .nterms()
                    .checked_mul(polynomial.nvars())
                    .is_some_and(|visits| visits <= 256)
                    && primitive_linear_variable(&polynomial).is_some()
                {
                    self.report.cache.primitive_linear_identities += 1;
                    vec![polynomial.clone()]
                } else {
                    let request =
                        self.operation_request(AlgebraOperation::Factor, None, &[&polynomial])?;
                    if let Some(fs) = self.load_operation(&request, &[&polynomial])? {
                        fs
                    } else {
                        self.report.cache.factor_misses += 1;
                        self.capture_operation(&request, None)?;
                        let started = Instant::now();
                        let mut factorization = HashMap::<Poly, usize>::new();
                        for (factor, multiplicity) in
                            self.algebra.factor_polynomial(&polynomial, false)?
                        {
                            if !factor.is_constant() {
                                *factorization.entry(canonical(&factor)).or_default() +=
                                    multiplicity;
                            }
                        }
                        let fs = sorted_unique(factorization.keys().cloned());
                        let multiplicities = fs.iter().map(|p| factorization[p]).collect();
                        let elapsed = started.elapsed().as_secs_f64();
                        self.capture_operation(&request, Some((&fs, elapsed)))?;
                        self.save_operation(&request, &fs, Some(multiplicities), elapsed)?;
                        fs
                    }
                };
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

    /// Canonical resultant, with nonzero scalar content and sign discarded.
    pub fn resultant(&mut self, a: &Poly, b: &Poly, variable: usize) -> Result<Poly> {
        ensure!(
            a.variables() == &self.algebra.variables && b.variables() == &self.algebra.variables,
            "resultant input has a different cache variable map"
        );
        ensure!(
            variable < self.algebra.names.len(),
            "resultant variable out of range"
        );
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
        let request =
            self.operation_request(AlgebraOperation::Resultant, Some(variable), &[a, b])?;
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
        let value = if let Some(mut output) = self.load_operation(&request, &[a, b])? {
            output.remove(0)
        } else {
            self.report.cache.resultant_misses += 1;
            self.capture_operation(&request, None)?;
            let started = Instant::now();
            // A static applicability decline permits the ordinary backend.
            // Once segmentation starts, errors/cancellation propagate: do not
            // spend the remaining worker budget on an unbounded second attempt.
            let segmented = if self.options.binary_resultants {
                crate::resultant_segmentation::try_resultant(
                    a,
                    b,
                    variable,
                    &crate::resultant_segmentation::Options::default(),
                    || Ok(()),
                )?
            } else {
                None
            };
            let exact = if let Some((exact, statistics)) = segmented {
                self.report.cache.segmented_resultants += 1;
                self.report.cache.maximum_segmented_output_bits = self
                    .report
                    .cache
                    .maximum_segmented_output_bits
                    .max(statistics.plan.output_bit_bound);
                exact
            } else {
                let (exact, interpolation) = exact_resultant_with_statistics(a, b, variable)?;
                if let Some(stats) = interpolation {
                    self.report.cache.interpolated_resultants += 1;
                    self.report.cache.interpolation_images += stats.images;
                }
                exact
            };
            let value = canonical(&exact);
            let elapsed = started.elapsed().as_secs_f64();
            self.capture_operation(&request, Some((std::slice::from_ref(&value), elapsed)))?;
            self.save_operation(&request, std::slice::from_ref(&value), None, elapsed)?;
            value
        };
        let bytes = 128usize
            .saturating_add(polynomial_bytes(a))
            .saturating_add(polynomial_bytes(b))
            .saturating_add(polynomial_bytes(&value));
        if self.reserve(bytes) {
            self.resultant_cache.insert(key, value.clone());
        }
        Ok(value)
    }

    /// Canonical discriminant, with nonzero scalar content and sign discarded.
    pub fn discriminant(&mut self, polynomial: &Poly, variable: usize) -> Result<Poly> {
        ensure!(
            polynomial.variables() == &self.algebra.variables,
            "discriminant input has a different cache variable map"
        );
        ensure!(
            variable < self.algebra.names.len(),
            "discriminant variable out of range"
        );
        let key = (variable, polynomial.clone());
        if let Some(value) = self.discriminant_cache.get(&key) {
            self.report.cache.discriminant_hits += 1;
            return Ok(value.clone());
        }
        let request = self.operation_request(
            AlgebraOperation::Discriminant,
            Some(variable),
            &[polynomial],
        )?;
        let value = if let Some(mut output) = self.load_operation(&request, &[polynomial])? {
            output.remove(0)
        } else {
            self.report.cache.discriminant_misses += 1;
            self.capture_operation(&request, None)?;
            let started = Instant::now();
            let exact = if self.options.binary_resultants {
                let (exact, segmentation) =
                    crate::projection::discriminant_with_binary_statistics(polynomial, variable)?;
                if let Some(statistics) = segmentation {
                    self.report.cache.segmented_discriminants += 1;
                    self.report.cache.maximum_segmented_output_bits = self
                        .report
                        .cache
                        .maximum_segmented_output_bits
                        .max(statistics.plan.output_bit_bound);
                }
                exact
            } else {
                discriminant(polynomial, variable)?
            };
            let value = canonical(&exact);
            let elapsed = started.elapsed().as_secs_f64();
            self.capture_operation(&request, Some((std::slice::from_ref(&value), elapsed)))?;
            self.save_operation(&request, std::slice::from_ref(&value), None, elapsed)?;
            value
        };
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
            let saved: CheckpointFile =
                serde_json::from_slice(&read_bounded(&path, self.options.max_checkpoint_bytes)?)?;
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

    fn raw_projection(&mut self, polynomials: &[Poly], variable: usize) -> Result<Vec<Poly>> {
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
        Ok(raw)
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
            let raw = self.raw_projection(&polynomials, variable)?;
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
        self.report.operation_removals.clear();
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
        self.report.operation_removals.clear();
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
        self.report.operation_removals.clear();
        if order.is_empty() {
            return Ok(Projection {
                levels: vec![],
                steps: vec![],
            });
        }
        let initial = self.initial_basis_certified(original_constraints, split, pruning)?;
        let protected = initial.iter().cloned().collect::<HashSet<_>>();
        if pruning.prune_operations {
            let n = order.len();
            let mut levels = vec![vec![]; n];
            let mut steps = Vec::with_capacity(n.saturating_sub(1));
            levels[n - 1] = initial;
            for position in (1..n).rev() {
                let started = Instant::now();
                let variable = order[position];
                let raw = self.raw_projection(&levels[position], variable)?;
                let mut retained_raw = Vec::new();
                for polynomial in raw {
                    if let Some(certificate) =
                        certify_nonzero(algebra, &polynomial, &domain, &pruning.certificates)?
                    {
                        self.report
                            .operation_removals
                            .push(ProjectionOperationRemoval {
                                level: position - 1,
                                eliminated_variable: variable,
                                certificate,
                            });
                    } else {
                        retained_raw.push(polynomial);
                    }
                }
                self.save_progress()?;
                let mut generated = self.factor_basis(&retained_raw)?;
                // A certified product can contain a protected face even when
                // the whole product is nonzero inside the original open box.
                // Carry every surviving input factor supported on this output
                // space explicitly, including factors from earlier stages.
                generated.extend(
                    protected
                        .iter()
                        .filter(|p| order[position..].iter().all(|&v| p.degree(v) == 0))
                        .cloned(),
                );
                let mut retained = Vec::new();
                for polynomial in sorted_unique(generated) {
                    if !protected.contains(&polynomial)
                        && let Some(certificate) =
                            certify_nonzero(algebra, &polynomial, &domain, &pruning.certificates)?
                    {
                        self.report.removals.push(ProjectionRemoval {
                            level: position - 1,
                            eliminated_variable: variable,
                            certificate,
                        });
                    } else {
                        retained.push(polynomial);
                    }
                }
                let (key, input_hash) = self.key(&levels[position], Some(variable))?;
                let key = digest(&(
                    "whole-operation-certificates-v1",
                    key,
                    &self.report.operation_removals,
                ))?;
                // Prefix output alone lacks the certificates needed for reuse.
                // Reuse persisted individual algebra operations instead.
                self.completed(key, input_hash, Some(variable), &retained, started, false)?;
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
            return finish_projection(algebra, levels, steps, order);
        }
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
        pruning.prune_operations,
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
    replay_certified_projection_with_operations(
        algebra,
        original_constraints,
        split,
        order,
        excluded_sign_constraints,
        initial_removals,
        removals,
        &[],
    )
}

/// Replay whole raw obligations independently, then factor only the obligations
/// not discharged by exact nonvanishing evidence. Protected input factors are
/// carried independently of the raw products in which they occur.
pub fn replay_certified_projection_with_operations(
    algebra: &Algebra,
    original_constraints: &[Poly],
    split: &[Poly],
    order: &[usize],
    excluded_sign_constraints: &[usize],
    initial_removals: &[DomainCertificate],
    removals: &[ProjectionRemoval],
    operation_removals: &[ProjectionOperationRemoval],
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
    for removal in operation_removals {
        ensure!(
            removal.level < n.saturating_sub(1),
            "raw certificate names an invalid projection level"
        );
        ensure!(
            removal.eliminated_variable == order[removal.level + 1],
            "raw certificate names the wrong elimination variable"
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
    let mut initial = algebra.factors_checked(&all)?;
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
        let raw = if operation_removals.is_empty() {
            crate::projection::projection_polynomials(&levels[position], variable)?
        } else {
            // Independently reconstruct the optimized raw basis: degree-zero
            // resultants are powers of an already retained leading coefficient.
            let mut raw = Vec::new();
            for (i, p) in levels[position].iter().enumerate() {
                raw.push(p.univariate_lcoeff(variable));
                if p.degree(variable) >= 2 {
                    raw.push(discriminant(p, variable)?);
                }
                for q in &levels[position][i + 1..] {
                    if p.degree(variable) > 0 && q.degree(variable) > 0 {
                        raw.push(exact_resultant(p, q, variable)?);
                    }
                }
            }
            sorted_unique(
                raw.into_iter()
                    .filter(|p| !p.is_constant())
                    .map(|p| canonical(&p)),
            )
        };
        let mut removed_operations = HashSet::new();
        for removal in operation_removals
            .iter()
            .filter(|r| r.level == position - 1)
        {
            let polynomial = algebra.parse(&removal.certificate.polynomial)?;
            ensure!(
                raw.contains(&polynomial),
                "raw certificate removes an absent projection obligation"
            );
            ensure!(
                removed_operations.insert(polynomial.clone()),
                "duplicate raw projection removal"
            );
            verify_certificate(
                algebra,
                &polynomial,
                original_constraints,
                excluded_sign_constraints,
                &removal.certificate,
            )?;
        }
        let retained_raw = raw
            .into_iter()
            .filter(|p| !removed_operations.contains(p))
            .collect::<Vec<_>>();
        let mut generated = algebra.factors_checked(&retained_raw)?;
        generated.extend(
            protected
                .iter()
                .filter(|p| order[position..].iter().all(|&v| p.degree(v) == 0))
                .cloned(),
        );
        let generated = sorted_unique(generated);
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

#[cfg(test)]
mod operation_tests {
    use super::*;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "symgcad-operations-{}-{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn algebra() -> Algebra {
        Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap()
    }

    #[test]
    fn operations_survive_a_stage_without_a_completed_prefix() {
        let directory = Directory::new();
        let algebra = algebra();
        let input = [
            // Nonprimitive-linear operands must still persist and replay
            // individual operations before a whole prefix is complete.
            algebra.parse("z^2+x^2+1").unwrap(),
            algebra.parse("z^2+y^2+1").unwrap(),
        ];
        let options = OptimizedProjectionOptions {
            checkpoint_dir: Some(directory.0.clone()),
            memory_cache_bytes: 0,
            ..Default::default()
        };
        {
            let mut interrupted = ProjectionCache::new(&algebra, &options);
            interrupted.factor_basis(&input).unwrap();
            interrupted.discriminant(&input[0], 2).unwrap();
            interrupted.resultant(&input[0], &input[1], 2).unwrap();
            assert!(interrupted.report.checkpoints.is_empty());
            assert_eq!(interrupted.report.cache.written_operation_checkpoints, 4);
        }
        let mut resumed = ProjectionCache::new(&algebra, &options);
        let projection = resumed.project_ordered(&input, &[0, 1, 2]).unwrap();
        assert_eq!(
            projection.levels,
            crate::projection::project(&algebra, &input).unwrap().levels
        );
        assert!(resumed.report.cache.factor_disk_hits >= 2);
        assert!(resumed.report.cache.resultant_disk_hits >= 1);
        assert!(resumed.report.cache.discriminant_disk_hits >= 1);
        assert_eq!(resumed.report.cache.prefix_disk_hits, 0);
    }

    #[test]
    fn operation_memory_hits_precede_disk_and_factor_multiplicities_reconstruct() {
        let directory = Directory::new();
        let algebra = algebra();
        let polynomial = algebra.parse("(z+x)^7*(z+1)^3").unwrap();
        let options = OptimizedProjectionOptions {
            checkpoint_dir: Some(directory.0.clone()),
            ..Default::default()
        };
        let mut first = ProjectionCache::new(&algebra, &options);
        let expected = first
            .factor_basis(std::slice::from_ref(&polynomial))
            .unwrap();
        assert_eq!(
            first
                .factor_basis(std::slice::from_ref(&polynomial))
                .unwrap(),
            expected
        );
        assert_eq!(first.report.cache.factor_hits, 1);
        assert_eq!(first.report.cache.factor_disk_hits, 0);
        let mut warm = ProjectionCache::new(&algebra, &options);
        assert_eq!(
            warm.factor_basis(std::slice::from_ref(&polynomial))
                .unwrap(),
            expected
        );
        assert_eq!(warm.report.cache.factor_disk_hits, 1);
        let request = warm
            .operation_request(AlgebraOperation::Factor, None, &[&polynomial])
            .unwrap()
            .unwrap();
        let path = directory
            .0
            .join("operations")
            .join(format!("{}.json", request.key));
        let mut saved: OperationFile = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        saved.output[0] = "z+2".into();
        saved.output_hash = digest(&(&saved.output, &saved.multiplicities)).unwrap();
        fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
        let mut damaged = ProjectionCache::new(&algebra, &options);
        assert_eq!(
            damaged
                .factor_basis(std::slice::from_ref(&polynomial))
                .unwrap(),
            expected
        );
        assert_eq!(damaged.report.cache.invalid_operation_checkpoints, 1);
        assert_eq!(damaged.report.cache.factor_misses, 1);
    }

    #[test]
    fn scalar_operation_checkpoints_reject_corrupt_headers_and_polynomials() {
        let directory = Directory::new();
        let algebra = algebra();
        let left = algebra.parse("z^2+x").unwrap();
        let right = algebra.parse("z+1").unwrap();
        let options = OptimizedProjectionOptions {
            checkpoint_dir: Some(directory.0.clone()),
            memory_cache_bytes: 0,
            ..Default::default()
        };
        let mut original = ProjectionCache::new(&algebra, &options);
        let expected = original.resultant(&left, &right, 2).unwrap();
        let path = fs::read_dir(directory.0.join("operations"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let clean = fs::read(&path).unwrap();
        for corruption in 0..15 {
            let mut saved: OperationFile = serde_json::from_slice(&clean).unwrap();
            match corruption {
                0 => saved.input_hash = "wrong".into(),
                1 => saved.backend_version = "wrong".into(),
                2 => saved.variable_names.swap(0, 1),
                3 => saved.operation = AlgebraOperation::Discriminant,
                4 => saved.variable = Some(1),
                5 => saved.output = vec!["unknown+1".into()],
                6 => saved.output = vec!["z+1".into()],
                7 => saved.output = vec!["y+1".into()],
                8 => saved.output = vec![],
                9 => saved.output = vec!["2*x+2".into()],
                10 => saved.algorithm = "symgcad-exact-gproj-v1".into(),
                11 => saved.domain_fingerprint = "wrong".into(),
                12 => saved.schema_version = 99,
                13 => saved.key = "wrong".into(),
                14 => saved.coefficient_domain = "F_17".into(),
                _ => unreachable!(),
            }
            saved.output_hash = digest(&(&saved.output, &saved.multiplicities)).unwrap();
            fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
            let mut restarted = ProjectionCache::new(&algebra, &options);
            assert_eq!(restarted.resultant(&left, &right, 2).unwrap(), expected);
            assert_eq!(
                restarted.report.cache.invalid_operation_checkpoints, 1,
                "corruption {corruption}"
            );
            assert_eq!(restarted.report.cache.resultant_misses, 1);
        }
    }

    #[test]
    fn zero_and_constant_operations_are_reused_and_item_sizes_are_bounded() {
        let directory = Directory::new();
        let algebra = algebra();
        let p = algebra.parse("z+x").unwrap();
        let q = algebra.parse("z+x+1").unwrap();
        let options = OptimizedProjectionOptions {
            checkpoint_dir: Some(directory.0.clone()),
            memory_cache_bytes: 0,
            max_checkpoint_bytes: 8192,
            ..Default::default()
        };
        let mut first = ProjectionCache::new(&algebra, &options);
        assert!(first.resultant(&p, &p, 2).unwrap().is_zero());
        assert!(first.resultant(&p, &q, 2).unwrap().is_one());
        assert!(first.discriminant(&p.pow(2), 2).unwrap().is_zero());
        let mut warm = ProjectionCache::new(&algebra, &options);
        assert!(warm.resultant(&p, &p, 2).unwrap().is_zero());
        assert!(warm.resultant(&p, &q, 2).unwrap().is_one());
        assert!(warm.discriminant(&p.pow(2), 2).unwrap().is_zero());
        assert_eq!(warm.report.cache.resultant_disk_hits, 2);
        assert_eq!(warm.report.cache.discriminant_disk_hits, 1);
        for file in fs::read_dir(directory.0.join("operations")).unwrap() {
            assert!(file.unwrap().metadata().unwrap().len() <= 8192);
        }
        let tiny = OptimizedProjectionOptions {
            max_checkpoint_bytes: 1,
            ..options
        };
        let mut bounded = ProjectionCache::new(&algebra, &tiny);
        assert!(bounded.resultant(&p, &q, 2).unwrap().is_one());
        assert_eq!(bounded.report.cache.invalid_operation_checkpoints, 1);
        assert_eq!(bounded.report.cache.written_operation_checkpoints, 0);
    }

    #[test]
    fn active_capture_survives_unfinished_cache_and_marks_completion() {
        let directory = Directory::new();
        let algebra = algebra();
        let p = algebra.parse("z^2+x").unwrap();
        let q = algebra.parse("z+y").unwrap();
        let options = OptimizedProjectionOptions {
            capture_dir: Some(directory.0.clone()),
            ..Default::default()
        };
        let path = directory
            .0
            .join(format!("active-{}.json", std::process::id()));
        {
            let mut interrupted = ProjectionCache::new(&algebra, &options);
            let request = interrupted
                .operation_request(AlgebraOperation::Resultant, Some(2), &[&p, &q])
                .unwrap();
            interrupted.capture_operation(&request, None).unwrap();
        }
        let saved: OperationCapture = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(!saved.complete);
        assert_eq!(saved.variable, Some(2));
        assert_eq!(saved.operation, AlgebraOperation::Resultant);
        let operands = saved.operands.unwrap();
        assert_eq!(saved.input_hash, digest(&operands).unwrap());
        assert_eq!(algebra.parse(&operands[0]).unwrap(), p);
        let mut completed = ProjectionCache::new(&algebra, &options);
        completed.resultant(&p, &q, 2).unwrap();
        let saved: OperationCapture = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert!(saved.complete);
        assert!(saved.elapsed_seconds.is_some_and(|t| t >= 0.0));
        assert!(saved.output_hash.is_some());
    }

    #[test]
    fn oversized_capture_omits_operands_and_cannot_leave_stale_snapshots() {
        let directory = Directory::new();
        let algebra = algebra();
        let p = algebra
            .parse(
                &(1..80)
                    .map(|i| format!("z^{i}"))
                    .collect::<Vec<_>>()
                    .join("+"),
            )
            .unwrap();
        let options = OptimizedProjectionOptions {
            capture_dir: Some(directory.0.clone()),
            max_capture_bytes: 800,
            ..Default::default()
        };
        let mut cache = ProjectionCache::new(&algebra, &options);
        let request = cache
            .operation_request(AlgebraOperation::Factor, None, &[&p])
            .unwrap();
        cache.capture_operation(&request, None).unwrap();
        let path = directory
            .0
            .join(format!("active-{}.json", std::process::id()));
        assert!(fs::metadata(&path).unwrap().len() <= 800);
        let saved: OperationCapture = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(saved.operands_omitted_for_size && saved.operands.is_none());
        cache.options.max_capture_bytes = 1;
        cache.capture_operation(&request, None).unwrap();
        assert!(!path.exists());
        let old_metrics: CacheMetrics = serde_json::from_str("{}").unwrap();
        assert_eq!(old_metrics.resultant_disk_hits, 0);
    }

    fn keyed_options(directory: &Directory, fingerprint: &str) -> OptimizedProjectionOptions {
        OptimizedProjectionOptions {
            checkpoint_dir: Some(directory.0.clone()),
            memory_cache_bytes: 0,
            domain_fingerprint: fingerprint.into(),
            ..Default::default()
        }
    }

    #[test]
    fn pure_algebra_records_are_shared_across_domains_but_not_variable_maps() {
        let directory = Directory::new();
        let algebra = algebra();
        let texts = ["z^3+x*z+y", "z^2+y*z+x+1"];
        let operands = |a: &Algebra| -> (Poly, Poly, Poly) {
            let p = a.parse(texts[0]).unwrap();
            let q = a.parse(texts[1]).unwrap();
            let product = &p * &q;
            (p, q, product)
        };
        let (p, q, product) = operands(&algebra);
        let mut first = ProjectionCache::new(&algebra, &keyed_options(&directory, "S1|order-a"));
        let resultant = first.resultant(&p, &q, 2).unwrap();
        let discriminant = first.discriminant(&p, 2).unwrap();
        let factors = first
            .factor_basis(std::slice::from_ref(&product))
            .unwrap();
        assert_eq!(first.report.cache.written_operation_checkpoints, 3);
        // Another constraint set, split and order fingerprint: pure algebra
        // in the same variable map is reused from disk without recomputation.
        let mut second = ProjectionCache::new(&algebra, &keyed_options(&directory, "S2|order-b"));
        assert_eq!(second.resultant(&p, &q, 2).unwrap(), resultant);
        assert_eq!(second.discriminant(&p, 2).unwrap(), discriminant);
        assert_eq!(
            second
                .factor_basis(std::slice::from_ref(&product))
                .unwrap(),
            factors
        );
        assert_eq!(second.report.cache.resultant_disk_hits, 1);
        assert_eq!(second.report.cache.discriminant_disk_hits, 1);
        assert_eq!(second.report.cache.factor_disk_hits, 1);
        assert_eq!(
            second.report.cache.resultant_misses
                + second.report.cache.discriminant_misses
                + second.report.cache.factor_misses,
            0
        );
        // A different eliminated axis is a different operation.
        second.resultant(&p, &q, 1).unwrap();
        assert_eq!(second.report.cache.resultant_misses, 1);
        // Renamed or permuted variable maps never reuse these records, even
        // for textually identical operands.
        for names in [["x", "y", "w"], ["y", "x", "z"]] {
            let other = Algebra::new(&names.map(String::from)).unwrap();
            let texts = texts.map(|t| if names[2] == "w" { t.replace('z', "w") } else { t.into() });
            let p = other.parse(&texts[0]).unwrap();
            let q = other.parse(&texts[1]).unwrap();
            let product = &p * &q;
            let mut foreign = ProjectionCache::new(&other, &keyed_options(&directory, "S1|order-a"));
            foreign.resultant(&p, &q, 2).unwrap();
            foreign.discriminant(&p, 2).unwrap();
            foreign
                .factor_basis(std::slice::from_ref(&product))
                .unwrap();
            assert_eq!(
                foreign.report.cache.resultant_disk_hits
                    + foreign.report.cache.discriminant_disk_hits
                    + foreign.report.cache.factor_disk_hits,
                0,
                "{names:?}"
            );
            assert_eq!(foreign.report.cache.resultant_misses, 1);
            assert_eq!(foreign.report.cache.discriminant_misses, 1);
            assert_eq!(foreign.report.cache.factor_misses, 1);
        }
    }

    #[test]
    fn legacy_domain_keyed_records_are_validated_then_migrated() {
        let directory = Directory::new();
        let algebra = algebra();
        let p = algebra.parse("z^3+x*z+y").unwrap();
        let q = algebra.parse("z^2+y*z+x+1").unwrap();
        let mut original = ProjectionCache::new(&algebra, &keyed_options(&directory, "old"));
        let expected = original.resultant(&p, &q, 2).unwrap();
        // The cache orders resultant operands canonically before keying.
        let (a, b) = if p.internal_cmp(&q).is_gt() {
            (&q, &p)
        } else {
            (&p, &q)
        };
        let request = original
            .operation_request(AlgebraOperation::Resultant, Some(2), &[a, b])
            .unwrap()
            .unwrap();
        let operations = directory.0.join("operations");
        let current = operations.join(format!("{}.json", request.key));
        let legacy = operations.join(format!("{}.json", request.legacy_key));
        // Rewrite the record exactly as the domain-keyed v1 layout stored it.
        let mut saved: OperationFile = serde_json::from_slice(&fs::read(&current).unwrap()).unwrap();
        assert_eq!(saved.domain_fingerprint, OPERATION_DOMAIN);
        saved.key = request.legacy_key.clone();
        saved.domain_fingerprint = "old".into();
        fs::write(&legacy, serde_json::to_vec(&saved).unwrap()).unwrap();
        fs::remove_file(&current).unwrap();
        // A legacy record is only valid for its own domain fingerprint.
        let mut other = ProjectionCache::new(&algebra, &keyed_options(&directory, "new"));
        assert_eq!(other.resultant(&p, &q, 2).unwrap(), expected);
        assert_eq!(other.report.cache.resultant_misses, 1);
        assert_eq!(other.report.cache.legacy_operation_disk_hits, 0);
        fs::remove_file(&current).unwrap();
        // A forged header under the legacy name is rejected, not trusted.
        let mut forged = saved.clone();
        forged.domain_fingerprint = "different".into();
        fs::write(&legacy, serde_json::to_vec(&forged).unwrap()).unwrap();
        let mut rejected = ProjectionCache::new(&algebra, &keyed_options(&directory, "old"));
        assert_eq!(rejected.resultant(&p, &q, 2).unwrap(), expected);
        assert_eq!(rejected.report.cache.invalid_operation_checkpoints, 1);
        assert_eq!(rejected.report.cache.resultant_misses, 1);
        fs::remove_file(&current).unwrap();
        // The genuine legacy record is read once and migrated.
        fs::write(&legacy, serde_json::to_vec(&saved).unwrap()).unwrap();
        let mut migrating = ProjectionCache::new(&algebra, &keyed_options(&directory, "old"));
        assert_eq!(migrating.resultant(&p, &q, 2).unwrap(), expected);
        assert_eq!(migrating.report.cache.legacy_operation_disk_hits, 1);
        assert_eq!(migrating.report.cache.resultant_disk_hits, 1);
        assert_eq!(migrating.report.cache.resultant_misses, 0);
        assert!(current.exists());
        let mut shared = ProjectionCache::new(&algebra, &keyed_options(&directory, "new"));
        assert_eq!(shared.resultant(&p, &q, 2).unwrap(), expected);
        assert_eq!(shared.report.cache.resultant_disk_hits, 1);
        assert_eq!(shared.report.cache.legacy_operation_disk_hits, 0);
    }

    #[test]
    fn public_operation_cache_rejects_foreign_maps_and_invalid_variables() {
        let algebra = algebra();
        let foreign = Algebra::new(&["a".into(), "b".into(), "c".into()]).unwrap();
        let p = algebra.parse("z+x").unwrap();
        let q = foreign.parse("c+a").unwrap();
        let mut cache = ProjectionCache::new(&algebra, &OptimizedProjectionOptions::default());
        assert!(cache.factor_basis(std::slice::from_ref(&q)).is_err());
        assert!(cache.resultant(&p, &q, 2).is_err());
        assert!(cache.discriminant(&q, 2).is_err());
        assert!(cache.resultant(&p, &p, 3).is_err());
        assert!(cache.discriminant(&p, 3).is_err());
    }
}
