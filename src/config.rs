use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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
    /// Experimental exact binary segmentation of bounded two-parameter
    /// pair resultants. Static declines retain the ordinary exact backend.
    #[serde(skip_serializing_if = "is_false")]
    pub binary_resultants: bool,
    pub positivity: bool,
    pub input_positivity: bool,
    pub prune_operations: bool,
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
    /// Exact finite rational interval covers with zero/one Descartes variation.
    #[serde(skip_serializing_if = "is_false")]
    pub local_descartes_intervals: bool,
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
    /// Isolate the first-axis root union without constant projection algebra.
    #[serde(skip_serializing_if = "is_true")]
    pub local_base_point_projection: bool,
    /// Experimental projection through adjacent relevant root branches only.
    #[serde(skip_serializing_if = "is_false")]
    pub local_adjacent_pairs: bool,
    #[serde(skip_serializing_if = "is_thirty_two")]
    pub local_subtree_cache_mib: usize,
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
            binary_resultants: false,
            positivity: false,
            input_positivity: false,
            prune_operations: false,
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
            local_descartes_intervals: false,
            local_descartes_interval_template_limit: 2,
            local_positive_pair_witnesses: false,
            local_strict_bezout_hints: Vec::new(),
            local_base_point_projection: true,
            local_adjacent_pairs: false,
            local_subtree_cache_mib: 32,
            local_learned_cut_cache_mib: 8,
            local_max_refinements: None,
            local_max_nodes: None,
            homogeneous_block: vec![],
            homogeneous_anchor: None,
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
                && self.limits.wall_time_secs <= 3600.0,
            "wall_time_secs must be in (0,3600]"
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
                "bilinear_positive"
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
                    "bilinear_positive"
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
