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
    pub checkpoint_dir: Option<PathBuf>,
    pub cache_memory_mib: usize,
    pub positivity: bool,
    pub input_positivity: bool,
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
            checkpoint_dir: None,
            cache_memory_mib: 128,
            positivity: false,
            input_positivity: false,
            homogeneous_block: vec![],
            homogeneous_anchor: None,
        }
    }
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
            ["baseline", "optimized", "monotone"].contains(&self.solver.engine.as_str()),
            "unknown engine"
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
                || ["optimized", "monotone"].contains(&self.solver.engine.as_str()),
            "positivity certificates require an optimized engine"
        );
        ensure!(
            !self.solver.input_positivity || self.solver.positivity,
            "input_positivity requires positivity"
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
