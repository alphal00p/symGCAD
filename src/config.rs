use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Explicit long-running campaigns may request two hours; defaults stay at one.
pub(crate) const MAX_WALL_TIME_SECS: f64 = 7200.0;

pub(crate) const DEFAULT_DESCARTES_WITNESS_MAX_TERMS: usize = 4096;
pub(crate) const MAX_DESCARTES_WITNESS_SOURCE_TERMS: usize = 65_536;
pub(crate) const DEFAULT_DESCARTES_INTERVAL_MAX_TERMS: usize = 4096;
pub(crate) const MAX_DESCARTES_INTERVAL_SOURCE_TERMS: usize = 65_536;
pub(crate) const DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS: usize = 65_536;
pub(crate) const MAX_DESCARTES_INTERVAL_TOTAL_TERMS: usize = 262_144;
pub(crate) const DEFAULT_SUBTREE_CACHE_ENTRY_NODES: usize = 128;
pub(crate) const MAX_SUBTREE_CACHE_ENTRY_NODES: usize = 4096;

pub(crate) fn default_subtree_cache_entry_nodes() -> usize {
    DEFAULT_SUBTREE_CACHE_ENTRY_NODES
}

pub(crate) fn is_default_subtree_cache_entry_nodes(value: &usize) -> bool {
    *value == DEFAULT_SUBTREE_CACHE_ENTRY_NODES
}

pub(crate) fn validate_subtree_cache_entry_nodes(value: usize) -> Result<()> {
    ensure!(
        (DEFAULT_SUBTREE_CACHE_ENTRY_NODES..=MAX_SUBTREE_CACHE_ENTRY_NODES).contains(&value),
        "local subtree cache entry nodes must be between 128 and 4096"
    );
    Ok(())
}

pub(crate) fn default_descartes_interval_max_total_terms() -> usize {
    DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS
}

pub(crate) fn is_default_descartes_interval_max_total_terms(value: &usize) -> bool {
    *value == DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS
}

pub(crate) fn validate_descartes_interval_max_total_terms(value: usize) -> Result<()> {
    ensure!(
        (1..=MAX_DESCARTES_INTERVAL_TOTAL_TERMS).contains(&value),
        "Descartes interval max total terms must be between 1 and 262144"
    );
    Ok(())
}

pub(crate) fn default_descartes_interval_max_terms() -> usize {
    DEFAULT_DESCARTES_INTERVAL_MAX_TERMS
}

pub(crate) fn is_default_descartes_interval_max_terms(value: &usize) -> bool {
    *value == DEFAULT_DESCARTES_INTERVAL_MAX_TERMS
}

pub(crate) fn validate_descartes_interval_max_terms(value: usize) -> Result<()> {
    ensure!(
        (1..=MAX_DESCARTES_INTERVAL_SOURCE_TERMS).contains(&value),
        "Descartes interval max terms must be between 1 and 65536"
    );
    Ok(())
}

pub(crate) fn default_descartes_witness_max_terms() -> usize {
    DEFAULT_DESCARTES_WITNESS_MAX_TERMS
}

pub(crate) fn is_default_descartes_witness_max_terms(value: &usize) -> bool {
    *value == DEFAULT_DESCARTES_WITNESS_MAX_TERMS
}

pub(crate) fn validate_descartes_witness_max_terms(value: usize) -> Result<()> {
    ensure!(
        (1..=MAX_DESCARTES_WITNESS_SOURCE_TERMS).contains(&value),
        "Descartes witness max terms must be between 1 and 65536"
    );
    Ok(())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Problem {
    pub name: Option<String>,
    pub variables: Vec<String>,
    pub constraints: Vec<String>,
    pub split: Vec<String>,
    pub parameters: BTreeMap<String, String>,
    pub variable_groups: Vec<Vec<String>>,
    pub solver: SolverOptions,
    pub limits: Limits,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SolverOptions {
    pub engine: String,
    pub ordering: String,
    pub order: Vec<String>,
    pub merge: String,
    pub seed: u64,
    pub max_rollouts: usize,
    pub search_time_secs: Option<f64>,
    /// Heuristic abandonment of explosive ordering prefixes. This affects
    /// search effort only; the selected final solve is never pruned this way.
    pub search_pruning: bool,
    /// Experimental local inventory costs rank timeout promotions only.
    #[serde(skip_serializing_if = "is_false")]
    pub search_local_progress: bool,
    pub checkpoint_dir: Option<PathBuf>,
    pub cache_memory_mib: usize,
    /// Exact binary segmentation of bounded two-parameter pairs and one- or
    /// two-parameter discriminants after interpolation declines. Static
    /// declines retain the ordinary exact backend. Default on since the
    /// 2026-10-12 kernel measurements; `binary_resultants = false` opts out.
    /// Only the opt-out is serialized, so it survives worker hand-off.
    #[serde(skip_serializing_if = "is_true")]
    pub binary_resultants: bool,
    /// Experimental exact affine preconditioning of projection factorizations
    /// and their independent verification. Coordinate maps remain unchanged.
    #[serde(skip_serializing_if = "is_false")]
    pub affine_factorization: bool,
    pub positivity: bool,
    pub input_positivity: bool,
    pub prune_operations: bool,
    /// Experimental weighted-square proofs in projection, ordering, and local
    /// guards. Monotone premise discovery keeps its existing portfolio.
    #[serde(skip_serializing_if = "is_false")]
    pub weighted_square_positivity: bool,
    /// Experimental exact PSD quadratic-component residual certificates.
    #[serde(skip_serializing_if = "is_false")]
    pub quadratic_component_positivity: bool,
    /// Engine for the exact endpoint problem after monotone peeling.
    #[serde(skip_serializing_if = "is_optimized")]
    pub monotone_base_engine: String,
    /// For unsplit monotone problems, solve a certified negative target fiber
    /// directly. Unsupported targets retain the ordinary sign-filter fallback.
    #[serde(skip_serializing_if = "is_false")]
    pub monotone_negative_target: bool,
    /// Optional positive linear comparison polynomial for the negative-ray
    /// engine. This is a proposed algebraic witness, never a domain assumption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_linear_guide: Option<String>,
    /// Proposed positive comparison polynomial for the exact two-fiber engine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bilinear_positive_guide: Option<String>,
    /// Engine for the lower-dimensional coefficient-sign decomposition.
    #[serde(skip_serializing_if = "is_optimized")]
    pub bilinear_positive_base_engine: String,
    /// Batch tentative sibling discoveries in the experimental local engine.
    /// Omit the default to keep ordinary inputs readable by older workers.
    #[serde(skip_serializing_if = "is_true")]
    pub local_batch_siblings: bool,
    /// Reuse small completed conditional subtrees within a local solve.
    #[serde(skip_serializing_if = "is_true")]
    pub local_reuse_subtrees: bool,
    /// Retain bounded learned cuts as hints; every stack is re-certified.
    #[serde(skip_serializing_if = "is_true")]
    pub local_reuse_learned_cuts: bool,
    /// Experimental durable cut hints, never cached proofs. Requires an
    /// explicit checkpoint directory and compatible local projection options.
    #[serde(skip_serializing_if = "is_false")]
    pub local_hint_archive: bool,
    /// Retain bounded rational sampling hints across refinements of one stack.
    #[serde(skip_serializing_if = "is_false")]
    pub local_stable_samples: bool,
    /// Experimental deferred stack projection with stable rational samples.
    #[serde(skip_serializing_if = "is_false")]
    pub local_deferred_projection: bool,
    /// Bounded leading-term cancellations used to separate positive roots.
    #[serde(skip_serializing_if = "is_two")]
    pub local_pair_cancellations: usize,
    /// Use a positive-root ordinal for Descartes-one local cuts.
    #[serde(skip_serializing_if = "is_true")]
    pub local_descartes_one: bool,
    /// Certify several positive roots using saturated Descartes bounds and
    /// strict rational-separator evaluation guards.
    #[serde(skip_serializing_if = "is_false")]
    pub local_descartes_witnesses: bool,
    /// Source-term budget for saturated witnesses; does not increase their
    /// permanent two-template stock or remove any exact guard obligation.
    #[serde(skip_serializing_if = "is_default_descartes_witness_max_terms")]
    pub local_descartes_witness_max_terms: usize,
    /// Exact finite rational interval covers with zero/one Descartes variation.
    #[serde(skip_serializing_if = "is_false")]
    pub local_descartes_intervals: bool,
    /// Source admission only; transformed guard and permanent stock limits
    /// remain unchanged. Historical omitted evidence retains 4096.
    #[serde(skip_serializing_if = "is_default_descartes_interval_max_terms")]
    pub local_descartes_interval_max_terms: usize,
    /// Total terms in the distinct canonical guard set for one interval cover.
    /// Other coefficient, cover and permanent stock limits remain unchanged.
    #[serde(skip_serializing_if = "is_default_descartes_interval_max_total_terms")]
    pub local_descartes_interval_max_total_terms: usize,
    /// Lifetime interval-template stock per polynomial (two through eight).
    #[serde(skip_serializing_if = "is_two")]
    pub local_descartes_interval_template_limit: usize,
    /// Separate positive roots of two delineable polynomials with exact
    /// rational-section guards, retaining resultant fallback.
    #[serde(skip_serializing_if = "is_false")]
    pub local_positive_pair_witnesses: bool,
    /// Optional strict polynomial-combination proposals for positive pair
    /// separation. Every used identity and coordinate premise is checked.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub local_strict_bezout_hints: Vec<crate::local_projection::StrictBezoutHint>,
    /// Optional identities p*L+q*M=k*S (S=H^2 or S>0, exactly certified on
    /// the coordinate box) omitting Res(p,q) in certified global projection.
    /// Requires positivity mode; every used identity is replayed on verify.
    /// Used by the optimized engine (directly or as the linear_negative or
    /// monotone base); other engines refuse them. Under `homogeneous_anchor`
    /// projection runs on the reduced problem (anchor = 1, other variables
    /// keep their names), so hints are written in those coordinates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projection_strict_bezout_hints:
        Vec<crate::strict_bezout_projection::ProjectionBezoutHint>,
    /// Isolate the first-axis root union without constant projection algebra.
    #[serde(skip_serializing_if = "is_true")]
    pub local_base_point_projection: bool,
    /// Experimental exact support blocks for generated first-axis guards.
    #[serde(skip_serializing_if = "is_false")]
    pub local_base_univariate_basis: bool,
    /// Experimental projection through adjacent relevant root branches only.
    #[serde(skip_serializing_if = "is_false")]
    pub local_adjacent_pairs: bool,
    #[serde(skip_serializing_if = "is_thirty_two")]
    pub local_subtree_cache_mib: usize,
    /// Admission limit for each cached conditional proof; does not enlarge the
    /// total cache budget or relax proof, depth, or serialized payload checks.
    #[serde(skip_serializing_if = "is_default_subtree_cache_entry_nodes")]
    pub local_subtree_cache_entry_nodes: usize,
    #[serde(skip_serializing_if = "is_eight")]
    pub local_learned_cut_cache_mib: usize,
    /// Per-problem local search limits; omitted values keep LocalOptions defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_max_refinements: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_max_nodes: Option<usize>,
    pub homogeneous_block: Vec<String>,
    pub homogeneous_anchor: Option<String>,
}
impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            engine: "baseline".into(),
            ordering: "given".into(),
            order: vec![],
            merge: "none".into(),
            seed: 0,
            max_rollouts: 256,
            search_time_secs: None,
            search_pruning: true,
            search_local_progress: false,
            checkpoint_dir: None,
            cache_memory_mib: 128,
            binary_resultants: true,
            affine_factorization: false,
            positivity: false,
            input_positivity: false,
            prune_operations: false,
            weighted_square_positivity: false,
            quadratic_component_positivity: false,
            monotone_base_engine: "optimized".into(),
            monotone_negative_target: false,
            negative_linear_guide: None,
            bilinear_positive_guide: None,
            bilinear_positive_base_engine: "optimized".into(),
            local_batch_siblings: true,
            local_reuse_subtrees: true,
            local_reuse_learned_cuts: true,
            local_hint_archive: false,
            local_stable_samples: false,
            local_deferred_projection: false,
            local_pair_cancellations: 2,
            local_descartes_one: true,
            local_descartes_witnesses: false,
            local_descartes_witness_max_terms: DEFAULT_DESCARTES_WITNESS_MAX_TERMS,
            local_descartes_intervals: false,
            local_descartes_interval_max_terms: DEFAULT_DESCARTES_INTERVAL_MAX_TERMS,
            local_descartes_interval_max_total_terms: DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS,
            local_descartes_interval_template_limit: 2,
            local_positive_pair_witnesses: false,
            local_strict_bezout_hints: Vec::new(),
            projection_strict_bezout_hints: Vec::new(),
            local_base_point_projection: true,
            local_base_univariate_basis: false,
            local_adjacent_pairs: false,
            local_subtree_cache_mib: 32,
            local_subtree_cache_entry_nodes: DEFAULT_SUBTREE_CACHE_ENTRY_NODES,
            local_learned_cut_cache_mib: 8,
            local_max_refinements: None,
            local_max_nodes: None,
            homogeneous_block: vec![],
            homogeneous_anchor: None,
        }
    }
}

impl SolverOptions {
    /// Shared optional proof portfolio for projection, ordering, and local guards.
    pub fn certificate_options(&self) -> crate::domain::CertificateOptions {
        crate::domain::CertificateOptions {
            weighted_square_candidates: if self.weighted_square_positivity {
                72
            } else {
                0
            },
            quadratic_component_candidates: if self.quadratic_component_positivity {
                32
            } else {
                0
            },
            ..Default::default()
        }
    }
}

fn is_true(value: &bool) -> bool {
    *value
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn is_thirty_two(value: &usize) -> bool {
    *value == 32
}
fn is_eight(value: &usize) -> bool {
    *value == 8
}
fn is_two(value: &usize) -> bool {
    *value == 2
}
fn is_optimized(value: &str) -> bool {
    value == "optimized"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub wall_time_secs: f64,
    pub memory_mib: u64,
    pub workers: usize,
    pub max_cells: Option<usize>,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            wall_time_secs: 3600.0,
            memory_mib: 65536,
            workers: 16,
            max_cells: None,
        }
    }
}

impl Problem {
    pub fn from_toml(text: &str) -> Result<Self> {
        let problem: Self = toml::from_str(text)?;
        problem.validate()?;
        Ok(problem)
    }
    pub fn validate(&self) -> Result<()> {
        validate_subtree_cache_entry_nodes(self.solver.local_subtree_cache_entry_nodes)?;
        validate_descartes_witness_max_terms(self.solver.local_descartes_witness_max_terms)?;
        validate_descartes_interval_max_terms(self.solver.local_descartes_interval_max_terms)?;
        validate_descartes_interval_max_total_terms(
            self.solver.local_descartes_interval_max_total_terms,
        )?;
        ensure!(
            (1..=2).contains(&self.solver.local_pair_cancellations),
            "local_pair_cancellations must be one or two"
        );
        ensure!(
            (2..=8).contains(&self.solver.local_descartes_interval_template_limit),
            "local_descartes_interval_template_limit must be between two and eight"
        );
        ensure!(
            !self.solver.local_deferred_projection || self.solver.local_stable_samples,
            "local_deferred_projection requires local_stable_samples"
        );
        ensure!(
            !self.solver.local_base_univariate_basis || self.solver.local_base_point_projection,
            "local_base_univariate_basis requires local_base_point_projection"
        );
        ensure!(
            !self.solver.local_base_univariate_basis || !self.solver.local_hint_archive,
            "local_base_univariate_basis does not permit local_hint_archive in v1"
        );
        ensure!(
            self.solver.projection_strict_bezout_hints.is_empty() || self.solver.positivity,
            "projection_strict_bezout_hints require certified projection (positivity = true)"
        );
        // Engines without certified global projection would silently ignore
        // the proposals; refuse them instead.
        ensure!(
            self.solver.projection_strict_bezout_hints.is_empty()
                || match self.solver.engine.as_str() {
                    "optimized" | "linear_negative" => true,
                    "monotone" => self.solver.monotone_base_engine == "optimized",
                    _ => false,
                },
            "projection_strict_bezout_hints are only used by certified global projection \
             (engine optimized, linear_negative, or monotone with monotone_base_engine = \
             \"optimized\"); the local engine takes local_strict_bezout_hints"
        );
        let names: BTreeSet<_> = self.variables.iter().collect();
        ensure!(names.len() == self.variables.len(), "duplicate variables");
        ensure!(
            self.variables.iter().all(|s| !s.is_empty()
                && s.chars().enumerate().all(|(i, c)| c == '_'
                    || c.is_ascii_alphabetic()
                    || (i > 0 && c.is_ascii_digit()))),
            "variable names must be identifiers"
        );
        ensure!(
            self.parameters.keys().all(|n| !names.contains(n)),
            "a parameter cannot also be a variable"
        );
        ensure!(
            self.limits.wall_time_secs.is_finite()
                && self.limits.wall_time_secs > 0.0
                && self.limits.wall_time_secs <= MAX_WALL_TIME_SECS,
            "wall_time_secs must be in (0,7200]"
        );
        ensure!(
            self.limits.memory_mib > 0 && self.limits.memory_mib <= 65536,
            "memory_mib must be in 1..=65536"
        );
        ensure!(
            self.limits.workers > 0 && self.limits.workers <= 16,
            "workers must be in 1..=16"
        );
        ensure!(
            self.limits.max_cells != Some(0),
            "max_cells must be positive"
        );
        if !self.solver.order.is_empty() {
            ensure!(
                self.solver.order.len() == names.len()
                    && self.solver.order.iter().collect::<BTreeSet<_>>() == names,
                "order must be a permutation of variables"
            );
        }
        if !self.variable_groups.is_empty() {
            let grouped: Vec<_> = self.variable_groups.iter().flatten().collect();
            ensure!(
                grouped.len() == names.len()
                    && grouped.iter().copied().collect::<BTreeSet<_>>() == names,
                "variable_groups must partition variables"
            );
            if !self.solver.order.is_empty() {
                let group_id: BTreeMap<_, _> = self
                    .variable_groups
                    .iter()
                    .enumerate()
                    .flat_map(|(i, g)| g.iter().map(move |v| (v, i)))
                    .collect();
                ensure!(
                    self.solver
                        .order
                        .windows(2)
                        .all(|w| group_id[&w[0]] <= group_id[&w[1]]),
                    "order violates variable groups"
                );
            }
        }
        ensure!(
            [
                "baseline",
                "optimized",
                "monotone",
                "local",
                "linear_negative",
                "bilinear_positive",
                "quadratic_ladder"
            ]
            .contains(&self.solver.engine.as_str()),
            "unknown engine"
        );
        ensure!(
            ["optimized", "local"].contains(&self.solver.monotone_base_engine.as_str()),
            "monotone_base_engine must be optimized or local"
        );
        ensure!(
            ["baseline", "optimized", "local"]
                .contains(&self.solver.bilinear_positive_base_engine.as_str()),
            "bilinear_positive_base_engine must be baseline, optimized or local"
        );
        ensure!(
            ["none", "aggressive"].contains(&self.solver.merge.as_str()),
            "unknown merge policy"
        );
        ensure!(
            self.solver.cache_memory_mib <= 65536,
            "cache_memory_mib exceeds memory ceiling"
        );
        ensure!(
            self.solver.local_subtree_cache_mib <= 65536
                && self.solver.local_learned_cut_cache_mib <= 65536,
            "local cache budget exceeds memory ceiling"
        );
        ensure!(
            !self.solver.positivity
                || [
                    "optimized",
                    "monotone",
                    "local",
                    "linear_negative",
                    "bilinear_positive",
                    "quadratic_ladder"
                ]
                .contains(&self.solver.engine.as_str()),
            "positivity certificates require an optimized engine"
        );
        ensure!(
            !self.solver.input_positivity || self.solver.positivity,
            "input_positivity requires positivity"
        );
        ensure!(
            !self.solver.prune_operations || self.solver.positivity,
            "prune_operations requires positivity"
        );
        ensure!(
            !self.solver.weighted_square_positivity || self.solver.positivity,
            "weighted_square_positivity requires positivity"
        );
        ensure!(
            !self.solver.quadratic_component_positivity || self.solver.positivity,
            "quadratic_component_positivity requires positivity"
        );
        ensure!(
            self.solver
                .search_time_secs
                .is_none_or(|s| s.is_finite() && s > 0.0),
            "search_time_secs must be finite and positive"
        );
        let scale_block = self
            .solver
            .homogeneous_block
            .iter()
            .collect::<BTreeSet<_>>();
        ensure!(
            scale_block.len() == self.solver.homogeneous_block.len()
                && scale_block.is_subset(&names),
            "homogeneous_block must contain distinct declared variables"
        );
        ensure!(
            self.solver
                .homogeneous_anchor
                .as_ref()
                .is_none_or(|a| scale_block.contains(a)),
            "homogeneous_anchor must belong to homogeneous_block"
        );
        Ok(())
    }
    pub fn effective_order(&self) -> Vec<String> {
        if self.solver.order.is_empty() {
            self.variables.clone()
        } else {
            self.solver.order.clone()
        }
    }
}
