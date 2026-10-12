//! Kinematic portfolio ordering for parametric Feynman-parameter problems.
//!
//! An input whose `variable_groups` lift kinematic parameters (for example
//! `s`, `t`) before one positive homogeneous Feynman block admits several
//! legal choices of positive-scale anchor and kinematic lifting order. This
//! module enumerates those choices, lets the existing MODS heuristic choose
//! the Feynman order of each, runs the solver's own preparation pipeline
//! (positive scaling, sign filtering, monotone peeling and the configured
//! global projection with its certified positivity options), and predicts
//! the lifting cost by the exact number of open cells of the kinematic
//! prefix, isolated with the solver's certified root isolation.
//!
//! Cost control (measured on the parametric corpus, see docs/USAGE.md):
//! every probe runs in a killable CLI worker under a hard wall budget of
//! `max(portfolio_min_budget_secs, portfolio_budget_factor x reference)`,
//! where the reference is the default probe's time and, once a candidate
//! beats the default, that candidate's probe time. The default probe also
//! lifts a few complete fibers to estimate its own lifting time, and the
//! alternatives together may use at most `portfolio_lifting_budget_factor`
//! times that estimate, the work they could save at best. Kinematic orders
//! that lift a decoupled kinematic variable first are not probed.
//!
//! Scores are heuristics only. The selected anchor and order are written
//! into the problem as an explicit `given` order, which the ordinary solver
//! then solves and the ordinary verifier replays. Variable groups and an
//! explicit `solver.order` are never overridden, and neither a score nor an
//! abandoned candidate removes any region.

use crate::{
    Problem,
    algebra::Poly,
    domain::{derive_domain, homogeneous_anchor},
    problem::{NormalizedProblem, normalize},
    projection::Projection,
    roots::{isolate_union, sector_sample},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    time::Instant,
};
use symbolica::prelude::*;

/// The `solver.ordering` value that requests this portfolio.
pub const ORDERING: &str = "kinematic_portfolio";
/// Identifies the candidate set, score and tie-break rules in reports.
pub const POLICY: &str = "kinematic-portfolio-v1";
/// Chooses the Feynman order of every candidate and the default candidate.
pub const BASE_ORDERING: &str = "mods";
/// Kinematic lifting orders per anchor; larger groups keep only the listed
/// and reversed order of each group.
pub const MAX_KINEMATIC_ORDERS: usize = 24;
/// Remaining portfolio budget below which further probes are skipped.
const MIN_PROBE_SECS: f64 = 0.01;
/// Bound on the kinematic prefixes kept for fiber sampling.
const MAX_RECORDED_PREFIXES: usize = 4096;

pub(crate) const UNRESOLVED: &str = "ordering \"kinematic_portfolio\" must be resolved by the supervising CLI (kinematic_portfolio::resolve) before an exact solve";

/// True when the portfolio must still choose an order. An explicit
/// `solver.order` is binding and is never replaced by a portfolio choice.
pub fn requested(problem: &Problem) -> bool {
    problem.solver.ordering == ORDERING && problem.solver.order.is_empty()
}

/// Kinematic groups lifted before one final homogeneous Feynman group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Every group before the Feynman group, in lifting order.
    pub kinematic_groups: Vec<Vec<String>>,
    /// The final group, equal as a set to `solver.homogeneous_block`.
    pub feynman: Vec<String>,
}

impl Layout {
    pub fn kinematic_variables(&self) -> usize {
        self.kinematic_groups.iter().map(Vec::len).sum()
    }
}

/// Recognize the supported structure, or explain why it is not applicable.
/// An explicit `solver.order` is binding, so no portfolio applies to it.
pub fn layout(problem: &Problem) -> std::result::Result<Layout, String> {
    if !problem.solver.order.is_empty() {
        return Err("an explicit solver.order is binding".into());
    }
    structure(problem)
}

/// The group structure alone; probes also accept a candidate's fixed order.
fn structure(problem: &Problem) -> std::result::Result<Layout, String> {
    let engine = problem.solver.engine.as_str();
    if !["monotone", "optimized", "baseline"].contains(&engine) {
        return Err(format!(
            "engine {engine:?} has no global projection for the kinematic score"
        ));
    }
    if engine == "monotone" && problem.solver.monotone_base_engine != "optimized" {
        return Err("a local monotone base engine has no global projection to score".into());
    }
    if problem.solver.monotone_negative_target {
        return Err("negative-target monotone planning is not probed".into());
    }
    let block: BTreeSet<_> = problem.solver.homogeneous_block.iter().collect();
    if block.is_empty() {
        return Err("no homogeneous Feynman block is declared".into());
    }
    let Some((feynman, kinematic)) = problem.variable_groups.split_last() else {
        return Err("no variable groups separate kinematics from the Feynman block".into());
    };
    if kinematic.is_empty() {
        return Err("no kinematic variable group precedes the Feynman block".into());
    }
    if feynman.iter().collect::<BTreeSet<_>>() != block {
        return Err("the last variable group is not exactly the homogeneous block".into());
    }
    Ok(Layout {
        kinematic_groups: kinematic.to_vec(),
        feynman: feynman.clone(),
    })
}

fn permutations(items: &[String]) -> Vec<Vec<String>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut result = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let first = rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, first.clone());
            result.push(tail);
        }
    }
    result
}

/// Every lifting order of the kinematic variables that respects the groups,
/// listed order first. Only permutations inside each group are considered.
pub fn kinematic_orders(groups: &[Vec<String>]) -> Vec<Vec<String>> {
    let total = groups.iter().fold(1usize, |total, g| {
        (1..=g.len()).fold(total, |t, k| t.saturating_mul(k))
    });
    let choices: Vec<Vec<Vec<String>>> = groups
        .iter()
        .map(|g| {
            if total <= MAX_KINEMATIC_ORDERS {
                permutations(g)
            } else {
                let reversed: Vec<_> = g.iter().rev().cloned().collect();
                if reversed == *g {
                    vec![g.clone()]
                } else {
                    vec![g.clone(), reversed]
                }
            }
        })
        .collect();
    let mut orders = vec![Vec::new()];
    for options in choices {
        orders = orders
            .into_iter()
            .flat_map(|prefix: Vec<String>| {
                options.iter().map(move |option| {
                    let mut order = prefix.clone();
                    order.extend(option.iter().cloned());
                    order
                })
            })
            .collect();
    }
    orders.truncate(MAX_KINEMATIC_ORDERS);
    orders
}

/// Anchors accepted by the exact positive-scale certificate, input anchor
/// first and then in block order. Without an input anchor no scale reduction
/// is introduced, so the only choice is `None`.
pub fn admissible_anchors(problem: &Problem) -> Result<Vec<Option<String>>> {
    let Some(input) = &problem.solver.homogeneous_anchor else {
        return Ok(vec![None]);
    };
    // Exactly the check of the solver's homogeneous preparation.
    let normalized = normalize(problem, &problem.variables)?;
    let block = problem
        .solver
        .homogeneous_block
        .iter()
        .map(|name| {
            problem
                .variables
                .iter()
                .position(|v| v == name)
                .expect("validated homogeneous block")
        })
        .collect::<Vec<_>>();
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    let domain = derive_domain(&normalized.algebra, &normalized.positives, &[])?;
    let mut anchors = vec![Some(input.clone())];
    for (&index, name) in block.iter().zip(&problem.solver.homogeneous_block) {
        if name != input
            && homogeneous_anchor(&normalized.algebra, &all, &block, index, &domain)?.is_some()
        {
            anchors.push(Some(name.clone()));
        }
    }
    Ok(anchors)
}

/// Kinematic variables that occur only in input polynomials without any
/// other variable (for example `s` when only `s > 4` mentions it). Every
/// projection factor of a higher level is then free of them, so their
/// position among the kinematic axes leaves the kinematic cells a product
/// with the same count, while lifting one before a coupled variable makes
/// projection compute pairwise resultants that are constants.
pub fn decoupled_kinematics(problem: &Problem, layout: &Layout) -> Result<BTreeSet<String>> {
    let normalized = normalize(problem, &problem.variables)?;
    let all: Vec<_> = normalized
        .positives
        .iter()
        .chain(&normalized.split)
        .collect();
    Ok(layout
        .kinematic_groups
        .iter()
        .flatten()
        .filter(|name| {
            let v = problem.variables.iter().position(|n| n == *name).unwrap();
            all.iter().all(|p| {
                p.degree(v) == 0 || (0..problem.variables.len()).all(|w| w == v || p.degree(w) == 0)
            })
        })
        .cloned()
        .collect())
}

/// Kinematic orders for the portfolio: those that lift every decoupled
/// variable after the coupled ones, as far as the groups allow (the minimal
/// number of decoupled-before-coupled pairs). The dominated orders have the
/// same kinematic cell count and only add constant resultants.
pub fn portfolio_kinematic_orders(
    layout: &Layout,
    decoupled: &BTreeSet<String>,
) -> Vec<Vec<String>> {
    let orders = kinematic_orders(&layout.kinematic_groups);
    let inversions = |order: &Vec<String>| {
        (0..order.len())
            .flat_map(|i| (i + 1..order.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| decoupled.contains(&order[i]) && !decoupled.contains(&order[j]))
            .count()
    };
    let fewest = orders.iter().map(inversions).min().unwrap_or(0);
    orders
        .into_iter()
        .filter(|o| inversions(o) == fewest)
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub anchor: Option<String>,
    /// Forced lifting order of the kinematic variables. `None` marks the
    /// default candidate, for which MODS orders the input groups unchanged.
    pub kinematic_order: Option<Vec<String>>,
}

/// The default candidate first, then every other admissible anchor (block
/// order) and finally the input anchor, each with every portfolio kinematic
/// order. Variants of the input anchor share the default's Feynman
/// projection, so they come last: when the default is expensive they are
/// too, and the budget may by then have tightened. The enumeration is
/// deterministic; its index is the final tie-break of the selection.
pub fn candidates(problem: &Problem, layout: &Layout) -> Result<Vec<Candidate>> {
    let mut result = vec![Candidate {
        anchor: problem.solver.homogeneous_anchor.clone(),
        kinematic_order: None,
    }];
    let orders = portfolio_kinematic_orders(layout, &decoupled_kinematics(problem, layout)?);
    let mut anchors = admissible_anchors(problem)?;
    anchors.rotate_left(1);
    for anchor in anchors {
        for order in &orders {
            result.push(Candidate {
                anchor: anchor.clone(),
                kinematic_order: Some(order.clone()),
            });
        }
    }
    Ok(result)
}

/// The probe input of one candidate: MODS ordering, the candidate anchor and,
/// unless it is the default, singleton kinematic groups fixing the order.
pub fn candidate_problem(problem: &Problem, layout: &Layout, candidate: &Candidate) -> Problem {
    let mut probe = problem.clone();
    probe.solver.ordering = BASE_ORDERING.into();
    probe.solver.order.clear();
    probe.solver.homogeneous_anchor = candidate.anchor.clone();
    if let Some(order) = &candidate.kinematic_order {
        probe.variable_groups = order
            .iter()
            .map(|v| vec![v.clone()])
            .chain(std::iter::once(layout.feynman.clone()))
            .collect();
    }
    probe
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProbeScore {
    /// Complete original lifting order chosen by MODS for the candidate.
    pub order: Vec<String>,
    /// Lifting order of the innermost problem that the solver would project.
    pub projected_order: Vec<String>,
    pub kinematic_variables: usize,
    /// Exact number of open kinematic cells satisfying the kinematic-only
    /// constraints, or the cap when `kinematic_cells_capped`.
    pub kinematic_cells: usize,
    pub kinematic_cells_capped: bool,
    pub kinematic_stacks: usize,
    pub projection_polynomials: usize,
    pub projection_terms: usize,
    pub level_polynomials: Vec<usize>,
    pub ordering_seconds: f64,
    pub projection_seconds: f64,
    pub scoring_seconds: f64,
    /// Only requested for the default candidate (`portfolio_fiber_samples`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifting_estimate: Option<FiberEstimate>,
}

/// Descend through the solver's own delegation (see `solver::solve_impl`) to
/// the problem whose global projection is lifted. Fixed orders only.
fn projection_problem(problem: &Problem) -> Result<Problem> {
    problem.validate()?;
    let engine = problem.solver.engine.as_str();
    ensure!(
        ![
            "quadratic_ladder",
            "bilinear_positive",
            "linear_negative",
            "local"
        ]
        .contains(&engine),
        "engine {engine:?} has no global projection for the kinematic score"
    );
    if engine == "monotone" && problem.split.is_empty() {
        ensure!(
            !problem.solver.monotone_negative_target,
            "negative-target monotone planning is not probed"
        );
        return projection_problem(&crate::sign_filter::planned_arrangement(problem)?);
    }
    if problem.solver.homogeneous_anchor.is_some() {
        return projection_problem(&crate::homogeneity::planned_reduction(problem)?);
    }
    if engine == "monotone" {
        return projection_problem(&crate::monotone::planned_base(problem)?);
    }
    Ok(problem.clone())
}

struct CellCounter<'a> {
    normalized: &'a NormalizedProblem,
    projection: &'a Projection,
    depth: usize,
    cap: usize,
    cells: usize,
    stacks: usize,
    /// An evenly thinned sample of leaf prefixes (every `stride`-th leaf),
    /// kept only when fibers will be lifted above some of them.
    prefixes: Option<Vec<Vec<Rational>>>,
    stride: usize,
    /// Optional time limit; a stopped count is only a lower bound.
    deadline: Option<Instant>,
    stopped: bool,
}

impl CellCounter<'_> {
    fn new<'a>(
        normalized: &'a NormalizedProblem,
        projection: &'a Projection,
        depth: usize,
        cap: usize,
    ) -> CellCounter<'a> {
        CellCounter {
            normalized,
            projection,
            depth,
            cap,
            cells: 0,
            stacks: 0,
            prefixes: None,
            stride: 1,
            deadline: None,
            stopped: false,
        }
    }

    fn done(&mut self) -> bool {
        if self.deadline.is_some_and(|d| Instant::now() >= d) {
            self.stopped = true;
        }
        self.stopped || self.cells >= self.cap
    }

    /// The solver's lifting rule truncated after `depth` axes: certified root
    /// unions, rational sector samples, and rejection once a constraint is a
    /// nonpositive constant. Only kinematic-only constraints can reject in
    /// the kinematic prefix.
    fn visit(&mut self, prefix: &mut Vec<Rational>, positives: Vec<Poly>) -> Result<()> {
        if self.done()
            || positives
                .iter()
                .any(|p| p.is_constant() && p.get_constant() <= Rational::zero())
        {
            return Ok(());
        }
        let level = prefix.len();
        if level == self.depth {
            if let Some(prefixes) = &mut self.prefixes {
                if self.cells % self.stride == 0 {
                    prefixes.push(prefix.clone());
                    if prefixes.len() >= 2 * MAX_RECORDED_PREFIXES {
                        let mut i = 0;
                        prefixes.retain(|_| {
                            i += 1;
                            i % 2 == 1
                        });
                        self.stride *= 2;
                    }
                }
            }
            self.cells += 1;
            return Ok(());
        }
        let fibers = self.normalized.algebra.specialize_univariate_batch(
            &self.projection.levels[level],
            level,
            prefix,
        )?;
        let roots = isolate_union(fibers.into_iter().enumerate().collect())?;
        self.stacks += 1;
        for i in 0..=roots.len() {
            let sample = sector_sample(i.checked_sub(1).map(|j| &roots[j]), roots.get(i));
            let next = positives
                .iter()
                .filter(|p| !p.is_constant())
                .map(|p| p.replace(level, &sample))
                .collect();
            prefix.push(sample);
            self.visit(prefix, next)?;
            prefix.pop();
            if self.done() {
                break;
            }
        }
        Ok(())
    }
}

/// Count open cells of the first `depth` lifting axes of a projection.
pub fn count_kinematic_cells(
    normalized: &NormalizedProblem,
    projection: &Projection,
    depth: usize,
    cap: usize,
) -> Result<(usize, bool, usize)> {
    ensure!(
        depth <= projection.levels.len(),
        "kinematic prefix exceeds the projection"
    );
    let mut counter = CellCounter::new(normalized, projection, depth, cap);
    counter.visit(&mut vec![], normalized.positives.clone())?;
    Ok((counter.cells, counter.cells >= cap, counter.stacks))
}

/// Lifting-cost estimate from complete fibers above a few kinematic cells.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FiberEstimate {
    /// Kinematic cells whose complete fibers were lifted (evenly spread).
    pub samples: usize,
    pub sampled_cells: usize,
    pub sampled_seconds: f64,
    /// The time limit stopped a fiber, so the estimate is a lower bound.
    pub truncated: bool,
    /// Kinematic cells times the mean sampled fiber.
    pub estimated_cells: f64,
    pub estimated_lifting_seconds: f64,
}

/// Lift the complete fibers above `samples` of the recorded, evenly spread
/// kinematic prefixes with the counting rule above and extrapolate to all
/// `kinematic_cells`. Only a budget heuristic: it never shortens or
/// replaces the real lifting.
pub fn estimate_lifting(
    normalized: &NormalizedProblem,
    projection: &Projection,
    prefixes: &[Vec<Rational>],
    kinematic_cells: usize,
    samples: usize,
    time_limit: std::time::Duration,
) -> Result<Option<FiberEstimate>> {
    if samples == 0 || prefixes.is_empty() {
        return Ok(None);
    }
    let started = Instant::now();
    let deadline = started + time_limit;
    let wanted = samples.min(prefixes.len());
    let mut estimate = FiberEstimate::default();
    for j in 0..wanted {
        if j > 0 && Instant::now() >= deadline {
            break;
        }
        let prefix = &prefixes[(2 * j + 1) * prefixes.len() / (2 * wanted)];
        let positives = normalized
            .positives
            .iter()
            .map(|p| {
                prefix
                    .iter()
                    .enumerate()
                    .fold(p.clone(), |q, (level, value)| q.replace(level, value))
            })
            .collect();
        let mut fiber =
            CellCounter::new(normalized, projection, projection.levels.len(), usize::MAX);
        fiber.deadline = Some(deadline);
        fiber.visit(&mut prefix.clone(), positives)?;
        estimate.samples += 1;
        estimate.sampled_cells += fiber.cells;
        estimate.truncated |= fiber.stopped;
    }
    estimate.sampled_seconds = started.elapsed().as_secs_f64();
    let scale = kinematic_cells as f64 / estimate.samples.max(1) as f64;
    estimate.estimated_cells = estimate.sampled_cells as f64 * scale;
    estimate.estimated_lifting_seconds = estimate.sampled_seconds * scale;
    Ok(Some(estimate))
}

/// MODS order of a candidate problem. MODS eliminates the Feynman group
/// before any kinematic group and chooses each variable from the current
/// projection only, so its Feynman order does not depend on how the earlier
/// kinematic groups are split. Singleton kinematic groups (a forced order)
/// are therefore merged for the MODS run, avoiding an extra projection onto
/// the first kinematic axis that ordering alone does not need, and the forced
/// kinematic order is restored in front of the MODS Feynman order.
fn feynman_mods_order(problem: &Problem, layout: &Layout) -> Result<Vec<String>> {
    let forced =
        layout.kinematic_groups.len() > 1 && layout.kinematic_groups.iter().all(|g| g.len() == 1);
    if !forced {
        return crate::solver::selected_order(problem);
    }
    let kinematic: Vec<String> = layout.kinematic_groups.iter().flatten().cloned().collect();
    let mut merged = problem.clone();
    merged.variable_groups = vec![kinematic.clone(), layout.feynman.clone()];
    let mods = crate::solver::selected_order(&merged)?;
    ensure!(
        mods[..kinematic.len()].iter().collect::<BTreeSet<_>>()
            == kinematic.iter().collect::<BTreeSet<_>>(),
        "MODS did not lift the kinematic group first"
    );
    Ok(kinematic
        .into_iter()
        .chain(mods[layout.kinematic_variables()..].iter().cloned())
        .collect())
}

/// Score one candidate problem (as built by `candidate_problem`) in process.
/// The supervising CLI runs this in a killable worker under a hard budget.
pub fn probe(problem: &Problem) -> Result<ProbeScore> {
    let layout = structure(problem).map_err(anyhow::Error::msg)?;
    let kinematic: BTreeSet<_> = layout.kinematic_groups.iter().flatten().collect();
    let depth = kinematic.len();
    let started = Instant::now();
    let order = if problem.solver.order.is_empty() {
        feynman_mods_order(problem, &layout)?
    } else {
        // A candidate whose MODS Feynman order is already known.
        problem.solver.order.clone()
    };
    let ordering_seconds = started.elapsed().as_secs_f64();
    let mut fixed = problem.clone();
    fixed.solver.order = order.clone();
    fixed.solver.ordering = "given".into();
    fixed.validate()?;
    let projection_started = Instant::now();
    let inner = projection_problem(&fixed)?;
    let projected_order = crate::solver::selected_order(&inner)?;
    ensure!(
        projected_order.len() >= depth
            && projected_order[..depth].iter().collect::<BTreeSet<_>>() == kinematic,
        "the projected problem does not lift the kinematic variables first"
    );
    let (normalized, projection, _) = crate::solver::prepare_projection(&inner, &projected_order)?;
    let projection_seconds = projection_started.elapsed().as_secs_f64();
    let scoring_started = Instant::now();
    let cap = problem.solver.portfolio_kinematic_cell_cap;
    let mut counter = CellCounter::new(&normalized, &projection, depth, cap);
    if problem.solver.portfolio_fiber_samples > 0 {
        counter.prefixes = Some(Vec::new());
    }
    counter.visit(&mut vec![], normalized.positives.clone())?;
    let (kinematic_cells, kinematic_cells_capped, kinematic_stacks) =
        (counter.cells, counter.cells >= cap, counter.stacks);
    let prefixes = counter.prefixes.take().unwrap_or_default();
    let scoring_seconds = scoring_started.elapsed().as_secs_f64();
    // Sample fibers for at most half the time already spent on this probe.
    let lifting_estimate = if kinematic_cells_capped {
        None
    } else {
        estimate_lifting(
            &normalized,
            &projection,
            &prefixes,
            kinematic_cells,
            problem.solver.portfolio_fiber_samples,
            started
                .elapsed()
                .mul_f64(0.5)
                .max(std::time::Duration::from_millis(20)),
        )?
    };
    let distinct: HashSet<_> = projection.levels.iter().flatten().collect();
    Ok(ProbeScore {
        order,
        projected_order,
        kinematic_variables: depth,
        kinematic_cells,
        kinematic_cells_capped,
        kinematic_stacks,
        projection_polynomials: distinct.len(),
        projection_terms: distinct.iter().map(|p| p.nterms()).sum(),
        level_polynomials: projection.levels.iter().map(Vec::len).collect(),
        ordering_seconds,
        projection_seconds,
        scoring_seconds,
        lifting_estimate,
    })
}

/// Outcome of one probe, normally from a killable worker process.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    /// Worker status: completed, timeout, memory_limit, failed, ...
    pub status: String,
    pub wall_seconds: f64,
    pub cpu_seconds: f64,
    pub peak_rss_kib: u64,
    pub score: Option<ProbeScore>,
    pub message: Option<String>,
}

/// Run `probe` in this process. There is no hard budget: a single native
/// algebra call cannot be interrupted, so production use goes through the
/// CLI worker. Useful for tests and trusted small inputs.
pub fn evaluate_in_process(problem: &Problem) -> Evaluation {
    let started = Instant::now();
    match probe(problem) {
        Ok(score) => Evaluation {
            status: "completed".into(),
            wall_seconds: started.elapsed().as_secs_f64(),
            score: Some(score),
            ..Default::default()
        },
        Err(error) => Evaluation {
            status: "failed".into(),
            wall_seconds: started.elapsed().as_secs_f64(),
            message: Some(format!("{error:#}")),
            ..Default::default()
        },
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub candidate: Candidate,
    /// scored, timeout, memory_limit, failed, skipped, or duplicate (an
    /// earlier candidate already scored the identical anchor and order).
    pub status: String,
    pub budget_seconds: f64,
    pub kinematic_cell_cap: usize,
    pub wall_seconds: f64,
    pub cpu_seconds: f64,
    pub peak_rss_kib: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<ProbeScore>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PortfolioReport {
    pub policy: String,
    pub base_ordering: String,
    pub applicable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub input_anchor: Option<String>,
    /// See `decoupled_kinematics`; their dominated orders are not probed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decoupled_kinematic_variables: Vec<String>,
    pub candidates: Vec<CandidateRecord>,
    /// Index of the selected candidate; `None` when the default heuristic
    /// runs unchanged because nothing (applicable) was scored.
    pub selected: Option<usize>,
    pub fallback_to_default: bool,
    /// The resolved order (empty when MODS still chooses it) and anchor.
    pub order: Vec<String>,
    pub anchor: Option<String>,
    pub default_budget_seconds: f64,
    pub candidate_budget_seconds: f64,
    /// Portfolio clock time after which no alternative starts: the default
    /// probe plus `portfolio_lifting_budget_factor` times its estimated
    /// lifting time. `None` without an estimate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternatives_budget_seconds: Option<f64>,
    pub total_budget_seconds: f64,
    pub total_seconds: f64,
}

/// Selection key: fewest kinematic cells (a capped count is only a lower
/// bound), then fewest projection terms, then enumeration order.
fn key(index: usize, record: &CandidateRecord) -> Option<(usize, bool, usize, usize)> {
    let score = record.score.as_ref()?;
    (record.status == "scored").then_some((
        score.kinematic_cells,
        score.kinematic_cells_capped,
        score.projection_terms,
        index,
    ))
}

pub fn select(records: &[CandidateRecord]) -> Option<usize> {
    records
        .iter()
        .enumerate()
        .filter_map(|(i, r)| key(i, r))
        .min()
        .map(|k| k.3)
}

fn default_resolution(problem: &Problem, report: &mut PortfolioReport) -> Problem {
    let mut resolved = problem.clone();
    resolved.solver.ordering = BASE_ORDERING.into();
    report.fallback_to_default = true;
    report.anchor = problem.solver.homogeneous_anchor.clone();
    resolved
}

/// Choose an anchor and order. `evaluate` receives each candidate's probe
/// problem, whose `limits.wall_time_secs` is that candidate's hard budget
/// and whose `portfolio_kinematic_cell_cap` bounds the count. The returned
/// problem has an explicit `given` order (or the unchanged MODS default) and
/// is solved and verified exactly like any other input.
pub fn resolve(
    problem: &Problem,
    mut evaluate: impl FnMut(&Problem) -> Result<Evaluation>,
) -> Result<(Problem, PortfolioReport)> {
    let started = Instant::now();
    problem.validate()?;
    let options = &problem.solver;
    let mut report = PortfolioReport {
        policy: POLICY.into(),
        base_ordering: BASE_ORDERING.into(),
        input_anchor: options.homogeneous_anchor.clone(),
        ..Default::default()
    };
    if !options.order.is_empty() {
        let mut resolved = problem.clone();
        resolved.solver.ordering = "given".into();
        report.reason = Some("an explicit solver.order is binding".into());
        report.order = options.order.clone();
        report.anchor = options.homogeneous_anchor.clone();
        report.total_seconds = started.elapsed().as_secs_f64();
        return Ok((resolved, report));
    }
    let layout = match layout(problem) {
        Ok(layout) => layout,
        Err(reason) => {
            report.reason = Some(reason);
            let resolved = default_resolution(problem, &mut report);
            report.total_seconds = started.elapsed().as_secs_f64();
            return Ok((resolved, report));
        }
    };
    report.applicable = true;
    report.decoupled_kinematic_variables = decoupled_kinematics(problem, &layout)?
        .into_iter()
        .collect();
    let wall = problem.limits.wall_time_secs;
    report.default_budget_seconds = options
        .portfolio_default_budget_secs
        .unwrap_or((wall * 0.1).clamp(1.0, 60.0))
        .min(wall);
    report.total_budget_seconds = options
        .portfolio_total_budget_secs
        .unwrap_or(wall * 0.25)
        .min(wall);
    let depth = layout.kinematic_variables();
    // MODS Feynman orders already obtained per anchor (see feynman_mods_order).
    let mut tails: Vec<(Option<String>, Vec<String>)> = Vec::new();
    for (index, candidate) in candidates(problem, &layout)?.into_iter().enumerate() {
        let mut probe_problem = candidate_problem(problem, &layout, &candidate);
        let known = candidate.kinematic_order.as_ref().and_then(|kinematic| {
            let (_, tail) = tails.iter().find(|(a, _)| *a == candidate.anchor)?;
            Some(kinematic.iter().chain(tail).cloned().collect::<Vec<_>>())
        });
        if let Some(order) = known {
            let twin = report.candidates.iter().position(|r| {
                r.candidate.anchor == candidate.anchor
                    && r.score.as_ref().is_some_and(|s| s.order == order)
            });
            if let Some(twin) = twin {
                report.candidates.push(CandidateRecord {
                    candidate,
                    status: "duplicate".into(),
                    message: Some(format!("same anchor and order as candidate {twin}")),
                    ..Default::default()
                });
                continue;
            }
            probe_problem.solver.ordering = "given".into();
            probe_problem.solver.order = order;
        }
        let elapsed = started.elapsed().as_secs_f64();
        let remaining = (report.total_budget_seconds - elapsed)
            .min(report.alternatives_budget_seconds.unwrap_or(f64::INFINITY) - elapsed);
        let best_index = select(&report.candidates);
        let budget = if index == 0 {
            report.default_budget_seconds
        } else {
            // Tighten once a candidate beats the default: a probe slower than
            // `factor` times the best probe so far is not paid for in full.
            let best_wall = best_index
                .filter(|&i| i > 0)
                .map(|i| report.candidates[i].wall_seconds);
            match best_wall {
                Some(wall) => report.candidate_budget_seconds.min(
                    (options.portfolio_budget_factor * wall).max(options.portfolio_min_budget_secs),
                ),
                None => report.candidate_budget_seconds,
            }
        }
        .min(remaining);
        let best = best_index.and_then(|i| report.candidates[i].score.as_ref());
        let cap = match best {
            Some(score) if !score.kinematic_cells_capped => score
                .kinematic_cells
                .saturating_add(1)
                .min(options.portfolio_kinematic_cell_cap),
            _ => options.portfolio_kinematic_cell_cap,
        };
        let mut record = CandidateRecord {
            candidate: candidate.clone(),
            budget_seconds: budget.max(0.0),
            kinematic_cell_cap: cap,
            ..Default::default()
        };
        if budget < MIN_PROBE_SECS {
            record.status = "skipped".into();
            record.message = Some("portfolio budget exhausted".into());
        } else {
            probe_problem.limits.wall_time_secs = budget;
            probe_problem.solver.portfolio_kinematic_cell_cap = cap;
            if index > 0 {
                probe_problem.solver.portfolio_fiber_samples = 0;
            }
            let evaluation = evaluate(&probe_problem).unwrap_or_else(|error| Evaluation {
                status: "failed".into(),
                message: Some(format!("{error:#}")),
                ..Default::default()
            });
            record.wall_seconds = evaluation.wall_seconds;
            record.cpu_seconds = evaluation.cpu_seconds;
            record.peak_rss_kib = evaluation.peak_rss_kib;
            record.message = evaluation.message;
            record.status = match (&evaluation.status[..], &evaluation.score) {
                ("completed", Some(_)) => "scored".into(),
                ("completed", None) => "failed".into(),
                (status, _) => status.into(),
            };
            record.score = evaluation.score;
        }
        if let Some(score) = record.score.as_ref().filter(|_| record.status == "scored") {
            if !tails.iter().any(|(a, _)| *a == record.candidate.anchor) {
                tails.push((
                    record.candidate.anchor.clone(),
                    score.order[depth..].to_vec(),
                ));
            }
        }
        if index == 0 {
            // Bounded regret: alternatives may use at most the estimated
            // lifting time of the default, the part they could save.
            let estimate = record
                .score
                .as_ref()
                .filter(|_| record.status == "scored")
                .and_then(|score| score.lifting_estimate.as_ref());
            report.alternatives_budget_seconds = estimate.map(|e| {
                started.elapsed().as_secs_f64()
                    + (options.portfolio_lifting_budget_factor * e.estimated_lifting_seconds)
                        .max(options.portfolio_min_budget_secs)
            });
            report.candidate_budget_seconds = if record.status == "scored" {
                (options.portfolio_budget_factor * record.wall_seconds)
                    .max(options.portfolio_min_budget_secs)
            } else {
                report
                    .default_budget_seconds
                    .max(options.portfolio_min_budget_secs)
            };
        }
        report.candidates.push(record);
    }
    let resolved = match select(&report.candidates) {
        Some(index) => {
            let record = &report.candidates[index];
            let mut resolved = problem.clone();
            resolved.solver.ordering = "given".into();
            resolved.solver.order = record.score.as_ref().unwrap().order.clone();
            resolved.solver.homogeneous_anchor = record.candidate.anchor.clone();
            match resolved.validate() {
                Ok(()) => {
                    report.selected = Some(index);
                    report.order = resolved.solver.order.clone();
                    report.anchor = resolved.solver.homogeneous_anchor.clone();
                    resolved
                }
                Err(error) => {
                    report.reason = Some(format!("selected order is not admissible: {error:#}"));
                    default_resolution(problem, &mut report)
                }
            }
        }
        None => {
            report.reason = Some("no candidate was scored within its budget".into());
            default_resolution(problem, &mut report)
        }
    };
    report.total_seconds = started.elapsed().as_secs_f64();
    Ok((resolved, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn kinematic_orders_permute_only_inside_groups() {
        assert_eq!(
            kinematic_orders(&[names(&["s", "t"])]),
            vec![names(&["s", "t"]), names(&["t", "s"])]
        );
        assert_eq!(
            kinematic_orders(&[names(&["s"]), names(&["t"])]),
            vec![names(&["s", "t"])]
        );
        let orders = kinematic_orders(&[names(&["a", "b", "c"]), names(&["d", "e"])]);
        assert_eq!(orders.len(), 12);
        assert_eq!(orders[0], names(&["a", "b", "c", "d", "e"]));
        assert!(orders.iter().all(|o| {
            o[..3].iter().collect::<BTreeSet<_>>() == names(&["a", "b", "c"]).iter().collect()
        }));
        let large = kinematic_orders(&[names(&["a", "b", "c", "d", "e"])]);
        assert_eq!(
            large,
            vec![
                names(&["a", "b", "c", "d", "e"]),
                names(&["e", "d", "c", "b", "a"])
            ]
        );
    }

    fn record(cells: usize, capped: bool, terms: usize, status: &str) -> CandidateRecord {
        CandidateRecord {
            status: status.into(),
            score: Some(ProbeScore {
                kinematic_cells: cells,
                kinematic_cells_capped: capped,
                projection_terms: terms,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn selection_prefers_cells_then_terms_then_enumeration() {
        assert_eq!(select(&[]), None);
        let records = vec![
            record(30, false, 10, "scored"),
            record(9, false, 50, "scored"),
            record(9, false, 40, "scored"),
            record(9, false, 40, "scored"),
            record(1, false, 1, "timeout"),
            record(9, true, 1, "scored"),
        ];
        assert_eq!(select(&records), Some(2));
        assert_eq!(select(&records[4..5]), None);
    }
}
