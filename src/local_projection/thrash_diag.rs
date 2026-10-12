//! Read-only refinement diagnostics, enabled by `SYMGCAD_LOCAL_THRASH_DIAG`.
//!
//! Nothing recorded here is proof authority or changes a solver choice: the
//! counters only describe how a stack visit restarts, which projection rules
//! have produced each newly requested cut, and why exact-prefix subtree reuse
//! missed. Polynomial origins are attributed by looking up each raw projection
//! obligation's factor basis in the in-memory operation cache only: the
//! diagnostic never factors, interns or changes a cache counter, so an
//! obligation that was not factored (base-univariate sources) or whose basis
//! was evicted simply stays unattributed.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
};
use symbolica::prelude::*;

/// Bit positions for [`ThrashDiag::origins`].
pub(super) const INPUT: u16 = 1 << 0;
pub(super) const COEFFICIENT: u16 = 1 << 1;
pub(super) const DISCRIMINANT: u16 = 1 << 2;
pub(super) const RESULTANT: u16 = 1 << 3;
pub(super) const PAIR_WITNESS: u16 = 1 << 4;
pub(super) const DESCARTES_WITNESS: u16 = 1 << 5;
pub(super) const OTHER_SEPARATOR: u16 = 1 << 6;
pub(super) const DOMAIN_SUPPORT: u16 = 1 << 7;
/// Coefficient of a cut that itself already carried a separator origin.
pub(super) const SEPARATOR_COEFFICIENT: u16 = 1 << 8;
pub(super) const SEPARATORS: u16 = PAIR_WITNESS | DESCARTES_WITNESS | OTHER_SEPARATOR;

const NAMES: [(u16, &str); 9] = [
    (INPUT, "input"),
    (COEFFICIENT, "coefficient"),
    (DISCRIMINANT, "discriminant"),
    (RESULTANT, "pair_resultant"),
    (PAIR_WITNESS, "pair_witness_separator"),
    (DESCARTES_WITNESS, "descartes_witness_separator"),
    (OTHER_SEPARATOR, "interval_separation_or_bezout"),
    (DOMAIN_SUPPORT, "domain_support"),
    (SEPARATOR_COEFFICIENT, "coefficient_of_separator_cut"),
];

pub(super) fn label(bits: u16) -> String {
    if bits == 0 {
        return "unknown".into();
    }
    NAMES
        .iter()
        .filter(|(bit, _)| bits & bit != 0)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join("+")
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThrashReport {
    /// Stack visits (leaves excluded) per lifting level, including cache hits.
    pub stack_visits_by_level: Vec<usize>,
    /// Restarted projection rounds per level.
    pub refinements_by_level: Vec<usize>,
    /// New same-axis cuts requested per level.
    pub additions_by_level: Vec<usize>,
    /// Largest cut set seen in one stack, per level.
    pub max_cuts_by_level: Vec<usize>,
    /// Addition events (one per stack round that requests a cut missing from
    /// its cut set; the same cut requested again in another visit or round
    /// counts again), keyed by the union of every projection rule that had
    /// produced the polynomial anywhere in the solve by then. A combined
    /// label such as `coefficient+pair_witness_separator` means both rules
    /// produced it, not that one rule derived it from the other.
    pub additions_by_origin: BTreeMap<String, usize>,
    /// Same, restricted to failed children whose sample was on that cut.
    pub guard_stop_additions_by_origin: BTreeMap<String, usize>,
    /// Distinct registry cuts by their origin label at their first addition.
    pub distinct_additions_by_origin: BTreeMap<String, usize>,
    /// Completed children discarded by a restart, with their cell counts.
    pub discarded_completed_children: usize,
    pub discarded_completed_cells: usize,
    /// Of restarted-round samples: identical to an earlier round's sample at
    /// the same stack visit (a reuse opportunity) or freshly chosen.
    pub restart_samples_repeated: usize,
    pub restart_samples_repeated_completed: usize,
    pub restart_samples_fresh: usize,
    /// Children reused within a still-active stack visit (retention option).
    pub retained_sibling_reuses: usize,
    pub retained_sibling_cells: usize,
    /// Exact-prefix subtree cache misses: never admitted before, or admitted
    /// and later evicted. Admission declines are counted separately.
    pub subtree_miss_never_admitted: usize,
    pub subtree_miss_after_eviction: usize,
    pub subtree_admission_declines: usize,
    pub subtree_hits_by_level: Vec<usize>,
    /// Stack visits by number of projection rounds (log2 buckets).
    pub rounds_per_visit_log2: Vec<usize>,
    pub max_rounds_in_one_visit: usize,
    /// Distinct registry polynomials with a recorded origin.
    pub origins_recorded: usize,
    /// Per-level projection work (calls, summed cut counts and seconds).
    pub project_calls_by_level: Vec<usize>,
    pub project_cuts_by_level: Vec<usize>,
    pub project_pairs_by_level: Vec<usize>,
    pub project_seconds_by_level: Vec<f64>,
    pub resultant_seconds_by_level: Vec<f64>,
    pub discriminant_seconds_by_level: Vec<f64>,
    pub factor_seconds_by_level: Vec<f64>,
    pub max_single_projection_seconds: f64,
    pub max_single_projection_level: usize,
    pub max_single_projection_cuts: usize,
}

pub(super) struct ProjectionCost {
    pub seconds: f64,
    pub resultant_seconds: f64,
    pub discriminant_seconds: f64,
    pub factor_seconds: f64,
    pub pairs: usize,
    pub cuts: usize,
}

fn grow_f(values: &mut Vec<f64>, level: usize) -> &mut f64 {
    if values.len() <= level {
        values.resize(level + 1, 0.0);
    }
    &mut values[level]
}

#[derive(Default)]
pub(super) struct ThrashDiag {
    pub report: ThrashReport,
    pub origins: HashMap<usize, u16>,
    admitted: HashSet<u64>,
    added: HashSet<usize>,
}

fn grow(values: &mut Vec<usize>, level: usize) -> &mut usize {
    if values.len() <= level {
        values.resize(level + 1, 0);
    }
    &mut values[level]
}

pub(super) fn prefix_hash(prefix: &[Rational]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    prefix.hash(&mut hasher);
    hasher.finish()
}

impl ThrashDiag {
    pub(super) fn enabled() -> bool {
        std::env::var_os("SYMGCAD_LOCAL_THRASH_DIAG").is_some()
    }
    pub(super) fn tag(&mut self, id: usize, bits: u16) {
        *self.origins.entry(id).or_default() |= bits;
        self.report.origins_recorded = self.origins.len();
    }
    pub(super) fn stack_visit(&mut self, level: usize) {
        *grow(&mut self.report.stack_visits_by_level, level) += 1;
    }
    pub(super) fn subtree_hit(&mut self, level: usize) {
        *grow(&mut self.report.subtree_hits_by_level, level) += 1;
    }
    pub(super) fn subtree_miss(&mut self, prefix: &[Rational]) {
        if self.admitted.contains(&prefix_hash(prefix)) {
            self.report.subtree_miss_after_eviction += 1;
        } else {
            self.report.subtree_miss_never_admitted += 1;
        }
    }
    pub(super) fn subtree_insert(&mut self, prefix: &[Rational], admitted: bool) {
        if admitted {
            self.admitted.insert(prefix_hash(prefix));
        } else {
            self.report.subtree_admission_declines += 1;
        }
    }
    pub(super) fn origin(&self, id: usize) -> u16 {
        self.origins.get(&id).copied().unwrap_or(0)
    }
    pub(super) fn addition(&mut self, id: usize, guard_stop: bool) {
        let name = label(self.origin(id));
        *self.report.additions_by_origin.entry(name.clone()).or_default() += 1;
        if self.added.insert(id) {
            *self
                .report
                .distinct_additions_by_origin
                .entry(name.clone())
                .or_default() += 1;
        }
        if guard_stop {
            *self
                .report
                .guard_stop_additions_by_origin
                .entry(name)
                .or_default() += 1;
        }
    }
    pub(super) fn refinement(&mut self, level: usize, additions: usize, cuts: usize) {
        *grow(&mut self.report.refinements_by_level, level) += 1;
        *grow(&mut self.report.additions_by_level, level) += additions;
        let max = grow(&mut self.report.max_cuts_by_level, level);
        *max = (*max).max(cuts);
    }
    pub(super) fn cuts(&mut self, level: usize, cuts: usize) {
        let max = grow(&mut self.report.max_cuts_by_level, level);
        *max = (*max).max(cuts);
    }
    pub(super) fn projection(&mut self, level: usize, cost: &ProjectionCost) {
        let r = &mut self.report;
        *grow(&mut r.project_calls_by_level, level) += 1;
        *grow(&mut r.project_cuts_by_level, level) += cost.cuts;
        *grow(&mut r.project_pairs_by_level, level) += cost.pairs;
        *grow_f(&mut r.project_seconds_by_level, level) += cost.seconds;
        *grow_f(&mut r.resultant_seconds_by_level, level) += cost.resultant_seconds;
        *grow_f(&mut r.discriminant_seconds_by_level, level) += cost.discriminant_seconds;
        *grow_f(&mut r.factor_seconds_by_level, level) += cost.factor_seconds;
        if cost.seconds > r.max_single_projection_seconds {
            r.max_single_projection_seconds = cost.seconds;
            r.max_single_projection_level = level;
            r.max_single_projection_cuts = cost.cuts;
        }
    }
    pub(super) fn rounds(&mut self, rounds: usize) {
        let bucket = usize::BITS - rounds.max(1).leading_zeros() - 1;
        *grow(&mut self.report.rounds_per_visit_log2, bucket as usize) += 1;
        self.report.max_rounds_in_one_visit = self.report.max_rounds_in_one_visit.max(rounds);
    }
}
