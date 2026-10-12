use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Explicit long-running campaigns may request five hours; defaults stay at one.
pub(crate) const MAX_WALL_TIME_SECS: f64 = 18000.0;

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
    /// Proposed comparison polynomial U, affine in each of the last two axes,
    /// for the exact multi-piece bilinear engine. Only its replayed square
    /// identities and certified positive coefficients are used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bilinear_general_guide: Option<String>,
    /// Engine for every lower-dimensional bilinear general piece.
    #[serde(skip_serializing_if = "is_optimized")]
    pub bilinear_general_base_engine: String,
    /// Optional positive-scale anchor applied inside every piece. It must
    /// precede every other variable of the scale block in the base order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bilinear_general_base_anchor: Option<String>,
    /// Scale block of the inner anchor (base variables only). Empty means
    /// every base variable; kinematic parameters stay outside a declared block.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bilinear_general_base_block: Vec<String>,
    /// Piece-planner objective: `region_weighted` (default) or `atoms`. It
    /// selects among valid plans only; the verifier accepts any valid plan.
    #[serde(skip_serializing_if = "is_region_weighted")]
    pub bilinear_general_plan_objective: String,
    /// OPT-IN CONTRACT EXTENSION (default false; `bilinear_general` only).
    /// A cell boundary may go continuously to +infinity: an upper bound may
    /// be a projective section (`RootIndexDomain::PositiveOrInfinity`), "the
    /// index-th distinct strictly positive root of P, or +infinity where
    /// fewer exist". Cells remain connected open cylindrical cells in the
    /// compactified coordinates u=x/(1+x), with continuous sections into
    /// (0,1]. With replayed corner-chain premises the bilinear_general
    /// engine then needs a single base piece {d>0} (see
    /// `bilinear_general`). When the premises are missing the ordinary strict
    /// plan is used. Results using projective sections verify only with this
    /// option on. Not combinable with `merge = "aggressive"`.
    #[serde(skip_serializing_if = "is_false")]
    pub projective_sections: bool,
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
    /// Reuse completed sibling proofs across refinements of one stack visit.
    /// Requires local_stable_samples; every reused child is replayed by the
    /// independent checker at its exact retained sample.
    #[serde(skip_serializing_if = "is_false")]
    pub local_retain_siblings: bool,
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
    /// Discovery-only: admit a new pair-witness template only if the dense
    /// monomial bound of the replaced resultant exceeds this value (default 0:
    /// the historical always-attempt policy, so existing inputs and evidence
    /// are unchanged).
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub local_pair_witness_min_resultant_terms: u64,
    /// Discovery-only lifetime cap on new pair-witness templates per axis
    /// (0: unlimited). A decline selects the ordinary resultant.
    #[serde(skip_serializing_if = "is_zero_usize")]
    pub local_pair_witness_axis_budget: usize,
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
    /// `ordering = "kinematic_portfolio"`: wall budget of each alternative
    /// candidate probe as a multiple of the default candidate's probe time.
    /// Probes only rank orders; they never certify or remove a region.
    #[serde(skip_serializing_if = "is_default_portfolio_budget_factor")]
    pub portfolio_budget_factor: f64,
    /// Lower bound on every alternative candidate's probe budget. Once a
    /// candidate beats the default, later budgets tighten to
    /// `max(this, factor x that candidate's probe time)`.
    #[serde(skip_serializing_if = "is_default_portfolio_min_budget_secs")]
    pub portfolio_min_budget_secs: f64,
    /// The default candidate's probe lifts the complete fibers above this
    /// many evenly spread kinematic cells to estimate its lifting time
    /// (0 disables the estimate and the bound below).
    #[serde(skip_serializing_if = "is_default_portfolio_fiber_samples")]
    pub portfolio_fiber_samples: usize,
    /// Alternatives may use at most this multiple of the default's
    /// estimated lifting time (never less than `portfolio_min_budget_secs`).
    #[serde(skip_serializing_if = "is_default_portfolio_lifting_budget_factor")]
    pub portfolio_lifting_budget_factor: f64,
    /// Probe budget of the default candidate. Omitted: a tenth of the wall
    /// limit, clamped to [1, 60] seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_default_budget_secs: Option<f64>,
    /// Bound on the whole portfolio, including the default probe. Omitted:
    /// a quarter of the wall limit. Unprobed candidates are recorded as skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_total_budget_secs: Option<f64>,
    /// Stop counting kinematic cells of one candidate at this many. The
    /// portfolio lowers it per candidate to one more than the best count so
    /// far, since a larger count can no longer be selected.
    #[serde(skip_serializing_if = "is_default_portfolio_kinematic_cell_cap")]
    pub portfolio_kinematic_cell_cap: usize,
    /// Give every portfolio candidate the same fixed budget
    /// `min(default budget, total budget / (candidates + 1))`, with no
    /// adaptive tightening or lifting-time bound, so that the choice depends
    /// on wall time only through which probes finish within that budget.
    #[serde(skip_serializing_if = "is_false")]
    pub portfolio_fixed_budgets: bool,
}

pub(crate) const DEFAULT_PORTFOLIO_BUDGET_FACTOR: f64 = 2.0;
pub(crate) const DEFAULT_PORTFOLIO_MIN_BUDGET_SECS: f64 = 0.25;
pub(crate) const DEFAULT_PORTFOLIO_FIBER_SAMPLES: usize = 8;
pub(crate) const DEFAULT_PORTFOLIO_LIFTING_BUDGET_FACTOR: f64 = 1.0;
pub(crate) const DEFAULT_PORTFOLIO_KINEMATIC_CELL_CAP: usize = 1_000_000;

fn is_default_portfolio_budget_factor(value: &f64) -> bool {
    *value == DEFAULT_PORTFOLIO_BUDGET_FACTOR
}
fn is_default_portfolio_min_budget_secs(value: &f64) -> bool {
    *value == DEFAULT_PORTFOLIO_MIN_BUDGET_SECS
}
fn is_default_portfolio_fiber_samples(value: &usize) -> bool {
    *value == DEFAULT_PORTFOLIO_FIBER_SAMPLES
}
fn is_default_portfolio_lifting_budget_factor(value: &f64) -> bool {
    *value == DEFAULT_PORTFOLIO_LIFTING_BUDGET_FACTOR
}
fn is_default_portfolio_kinematic_cell_cap(value: &usize) -> bool {
    *value == DEFAULT_PORTFOLIO_KINEMATIC_CELL_CAP
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
            bilinear_general_guide: None,
            bilinear_general_base_engine: "optimized".into(),
            bilinear_general_base_anchor: None,
            bilinear_general_base_block: vec![],
            bilinear_general_plan_objective: "region_weighted".into(),
            projective_sections: false,
            local_batch_siblings: true,
            local_reuse_subtrees: true,
            local_reuse_learned_cuts: true,
            local_hint_archive: false,
            local_stable_samples: false,
            local_retain_siblings: false,
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
            local_pair_witness_min_resultant_terms: 0,
            local_pair_witness_axis_budget: 0,
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
            portfolio_budget_factor: DEFAULT_PORTFOLIO_BUDGET_FACTOR,
            portfolio_min_budget_secs: DEFAULT_PORTFOLIO_MIN_BUDGET_SECS,
            portfolio_fiber_samples: DEFAULT_PORTFOLIO_FIBER_SAMPLES,
            portfolio_lifting_budget_factor: DEFAULT_PORTFOLIO_LIFTING_BUDGET_FACTOR,
            portfolio_default_budget_secs: None,
            portfolio_total_budget_secs: None,
            portfolio_kinematic_cell_cap: DEFAULT_PORTFOLIO_KINEMATIC_CELL_CAP,
            portfolio_fixed_budgets: false,
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
fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}
fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}
fn is_optimized(value: &str) -> bool {
    value == "optimized"
}
fn is_region_weighted(value: &str) -> bool {
    value == "region_weighted"
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
            !self.solver.local_retain_siblings || self.solver.local_stable_samples,
            "local_retain_siblings requires local_stable_samples"
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
            "wall_time_secs must be in (0,18000]"
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
                "bilinear_general",
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
            ["baseline", "optimized", "local"]
                .contains(&self.solver.bilinear_general_base_engine.as_str()),
            "bilinear_general_base_engine must be baseline, optimized or local"
        );
        ensure!(
            self.solver
                .bilinear_general_base_anchor
                .as_ref()
                .is_none_or(|a| names.contains(a)),
            "bilinear_general_base_anchor must be a declared variable"
        );
        let general_block = self
            .solver
            .bilinear_general_base_block
            .iter()
            .collect::<BTreeSet<_>>();
        ensure!(
            general_block.len() == self.solver.bilinear_general_base_block.len()
                && general_block.is_subset(&names),
            "bilinear_general_base_block must contain distinct declared variables"
        );
        ensure!(
            general_block.is_empty()
                || self
                    .solver
                    .bilinear_general_base_anchor
                    .as_ref()
                    .is_some_and(|a| general_block.contains(a)),
            "bilinear_general_base_block requires bilinear_general_base_anchor inside it"
        );
        ensure!(
            ["region_weighted", "atoms"]
                .contains(&self.solver.bilinear_general_plan_objective.as_str()),
            "bilinear_general_plan_objective must be region_weighted or atoms"
        );
        ensure!(
            ["none", "aggressive"].contains(&self.solver.merge.as_str()),
            "unknown merge policy"
        );
        // The projective-section contract extension is opt-in, belongs to
        // the engine that certifies it, and has no compatibility merge.
        ensure!(
            !self.solver.projective_sections || self.solver.engine == "bilinear_general",
            "projective_sections is a bilinear_general contract extension"
        );
        ensure!(
            !self.solver.projective_sections || self.solver.merge == "none",
            "projective_sections cannot be combined with merge = \"aggressive\""
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
                    "bilinear_general",
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
        ensure!(
            self.solver.portfolio_budget_factor.is_finite()
                && self.solver.portfolio_budget_factor > 0.0
                && self.solver.portfolio_lifting_budget_factor.is_finite()
                && self.solver.portfolio_lifting_budget_factor > 0.0
                && self.solver.portfolio_min_budget_secs.is_finite()
                && self.solver.portfolio_min_budget_secs > 0.0
                && [
                    self.solver.portfolio_default_budget_secs,
                    self.solver.portfolio_total_budget_secs
                ]
                .iter()
                .flatten()
                .all(|s| s.is_finite() && *s > 0.0),
            "portfolio budgets must be finite and positive"
        );
        ensure!(
            self.solver.portfolio_kinematic_cell_cap > 0,
            "portfolio_kinematic_cell_cap must be positive"
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
