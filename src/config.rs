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
    pub checkpoint_dir: Option<PathBuf>,
    pub cache_memory_mib: usize,
    pub positivity: bool,
    pub input_positivity: bool,
    pub prune_operations: bool,
    /// Engine for the exact endpoint problem after monotone peeling.
    #[serde(skip_serializing_if = "is_optimized")]
    pub monotone_base_engine: String,
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
    /// Bounded leading-term cancellations used to separate positive roots.
    #[serde(skip_serializing_if = "is_two")]
    pub local_pair_cancellations: usize,
    /// Use a positive-root ordinal for Descartes-one local cuts.
    #[serde(skip_serializing_if = "is_true")]
    pub local_descartes_one: bool,
    /// Isolate the first-axis root union without constant projection algebra.
    #[serde(skip_serializing_if = "is_true")]
    pub local_base_point_projection: bool,
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
            checkpoint_dir: None,
            cache_memory_mib: 128,
            positivity: false,
            input_positivity: false,
            prune_operations: false,
            monotone_base_engine: "optimized".into(),
            local_batch_siblings: true,
            local_reuse_subtrees: true,
            local_reuse_learned_cuts: true,
            local_pair_cancellations: 2,
            local_descartes_one: true,
            local_base_point_projection: true,
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
            ["baseline", "optimized", "monotone", "local"].contains(&self.solver.engine.as_str()),
            "unknown engine"
        );
        ensure!(
            ["optimized", "local"].contains(&self.solver.monotone_base_engine.as_str()),
            "monotone_base_engine must be optimized or local"
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
            !self.solver.positivity
                || ["optimized", "monotone", "local"].contains(&self.solver.engine.as_str()),
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
