//! Experimental generic-open local projection with an exhaustive certificate.
//!
//! This is a conservative stack-saturation variant of Strzebonski's LPCAD
//! (arXiv:1405.4925, Algorithms 8 and 13). Each stack jointly projects every
//! sibling boundary, restarts if a child discovers another boundary, and passes
//! all remaining guards to its parent. Only full-dimensional open sectors are
//! constructed: a guard vanishing at a tentative sample is propagated to the
//! ancestor that owns its highest variable and forces resampling there.
//!
//! All retained projection guards are nonzero on the eventual ancestor cell.
//! Thus leading coefficients and discriminants give delineability, and a
//! polynomial without real roots at the sample remains rootless on that cell.
//! Pair resultants are needed only between the root-bearing polynomials. This
//! generic-open restriction deliberately makes no claims about sections or
//! nullified fibers. Its independent checker replays projection obligations and
//! every sector; checking cell samples alone would not establish coverage.
//!
//! Optional original-domain certificates replace a projection guard only by
//! explicit obligations for the earlier coordinate bounds proving it nonzero.
//! Protected original factors make those premises sign-invariant on the whole
//! ancestor cell. No certificate borrows an unassigned coordinate's bounds,
//! and no input split factor or coordinate boundary is removed.
use crate::{
    Problem,
    algebra::{Algebra, Poly, canonical, factors_checked},
    domain::{
        CertificateOptions, Domain, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
    optimized_projection::{CacheMetrics, OptimizedProjectionOptions, ProjectionCache},
    output::{Axis, Cell, RootBound, SolveResult, Statistics, Status},
    problem::{NormalizedProblem, normalize},
    projection::{discriminant, exact_resultant},
    roots::{isolate_union, sector_sample},
    solver::{bound, evaluate, merge, sign},
    verify::Verification,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    time::Instant,
};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LocalOptions {
    /// Includes speculative stacks discarded when their boundary set grows.
    pub max_nodes: usize,
    pub max_refinements: usize,
    /// Unsupported applicability conditions may use the ordinary exact solver.
    /// Resource exhaustion never starts a second solve with a fresh budget.
    pub fallback_to_global: bool,
    /// Disable both memory and disk algebra reuse for controlled ablations.
    pub cache_enabled: bool,
    /// Permit original-domain guard certificates when solver.positivity is on.
    /// Setting this false gives a local projection ablation with the same input.
    pub domain_pruning: bool,
}
impl Default for LocalOptions {
    fn default() -> Self {
        Self {
            max_nodes: 100_000,
            max_refinements: 10_000,
            fallback_to_global: true,
            cache_enabled: true,
            domain_pruning: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalPhaseCosts {
    pub factor_seconds: f64,
    pub discriminant_seconds: f64,
    pub resultant_seconds: f64,
    pub root_classification_seconds: f64,
    pub root_isolation_seconds: f64,
    pub domain_certificate_seconds: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalReport {
    pub speculative_nodes: usize,
    pub refinements: usize,
    pub projected_pairs: usize,
    pub omitted_rootless_pairs: usize,
    pub cache: CacheMetrics,
    pub phases: LocalPhaseCosts,
    pub elapsed_seconds: f64,
    pub current_level: Option<usize>,
    pub active_phase: String,
    pub guard_certificate_attempts: usize,
    pub guard_certificate_hits: usize,
    pub certified_guard_removals: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalProjectionCertificate {
    /// Exactly those cuts with at least one real root on this sample fiber.
    pub root_bearing: Vec<usize>,
    /// Square-free irreducible factors of all required lc, disc and resultants.
    pub guards: Vec<usize>,
    /// References into LocalEvidence::Local.guard_certificates. Their bound
    /// factors replace the removed guards as explicit ancestor obligations.
    #[serde(default)]
    pub removals: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalGuardRemoval {
    pub level: usize,
    pub guard: usize,
    pub certificate: DomainCertificate,
    pub supporting_constraints: Vec<usize>,
    pub supporting_factors: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocalNode {
    Rejected {
        constraint: usize,
    },
    Accepted {
        signs: Vec<i8>,
    },
    Stack {
        cuts: Vec<usize>,
        projection: LocalProjectionCertificate,
        roots: Vec<RootBound>,
        sectors: Vec<LocalSector>,
        /// Every guard needed below this stack, all in strictly earlier axes.
        required_below: Vec<usize>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalSector {
    pub sample: String,
    pub child: LocalNode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocalEvidence {
    Local {
        options: LocalOptions,
        /// One factor set per normalized constraint, followed by every split.
        input_factors: Vec<Vec<usize>>,
        /// Deduplicated exact proofs, including records from discarded trials.
        /// Only records referenced by the final tree establish an obligation.
        #[serde(default)]
        guard_certificates: Vec<LocalGuardRemoval>,
        root: LocalNode,
        report: LocalReport,
    },
    GlobalFallback {
        reason: String,
    },
    NotApplicable {
        reason: String,
    },
    Unfinished {
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        report: Option<LocalReport>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalSolveResult {
    pub solution: SolveResult,
    pub evidence: LocalEvidence,
}

#[derive(Debug)]
enum Stop {
    /// This nonconstant factor vanishes on the current tentative prefix.
    Guard(usize),
    Resource(String),
    Unsupported(String),
}
type Work<T> = std::result::Result<T, Stop>;
impl From<anyhow::Error> for Stop {
    fn from(error: anyhow::Error) -> Self {
        Self::Unsupported(error.to_string())
    }
}

fn highest(p: &Poly) -> Option<usize> {
    (0..p.nvars()).rfind(|&i| p.degree(i) != 0)
}

/// Evaluate only a polynomial whose complete support is in the prefix. A
/// specialization that happens to become constant is not sufficient evidence.
fn prefix_value(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
    ensure!(
        highest(p).is_none_or(|i| i < prefix.len()),
        "guard depends on a later coordinate"
    );
    let mut value = p.clone();
    for (i, x) in prefix.iter().enumerate() {
        value = value.replace(i, x);
    }
    ensure!(value.is_constant(), "incomplete guard evaluation");
    Ok(value.get_constant())
}

struct GuardDomain {
    domain: Domain,
    excluded: Vec<usize>,
    supporting_constraints: Vec<usize>,
}

/// The local theorem is conditional on a *previously lifted* ancestor cell.
/// Later-coordinate bounds cannot be assumed there. Excluding their original
/// constraints gives a conservative domain in which every bound premise can
/// be discharged by an ancestor nonzero/sign-invariance obligation.
fn guard_domain(normalized: &NormalizedProblem, level: usize) -> Result<GuardDomain> {
    let excluded: Vec<_> = normalized
        .positives
        .iter()
        .enumerate()
        .filter(|(_, p)| highest(p).is_some_and(|i| i >= level))
        .map(|(i, _)| i)
        .collect();
    let domain = derive_domain(&normalized.algebra, &normalized.positives, &excluded)?;
    let supporting_constraints = domain
        .bounds
        .iter()
        .flat_map(|b| [b.lower_constraint, b.upper_constraint])
        .flatten()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(GuardDomain {
        domain,
        excluded,
        supporting_constraints,
    })
}

fn supporting_factors(context: &GuardDomain, input_factors: &[Vec<usize>]) -> Vec<usize> {
    context
        .supporting_constraints
        .iter()
        .flat_map(|&i| input_factors[i].iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

trait ProjectionOperations {
    fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly>;
    fn resultant(&mut self, p: &Poly, q: &Poly, variable: usize) -> Result<Poly>;
}

/// The verifier never reads discovery or persistent operation caches.
struct IndependentOperations;
impl ProjectionOperations for IndependentOperations {
    fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly> {
        discriminant(p, variable)
    }
    fn resultant(&mut self, p: &Poly, q: &Poly, variable: usize) -> Result<Poly> {
        exact_resultant(p, q, variable)
    }
}

struct CachedOperations<'a> {
    cache: ProjectionCache<'a>,
    phases: LocalPhaseCosts,
}
impl CachedOperations<'_> {
    fn factors(&mut self, polynomials: &[Poly]) -> Result<Vec<Poly>> {
        let started = Instant::now();
        let result = self.cache.factor_basis(polynomials);
        self.phases.factor_seconds += started.elapsed().as_secs_f64();
        result
    }
}
impl ProjectionOperations for CachedOperations<'_> {
    fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly> {
        let started = Instant::now();
        let result = self.cache.discriminant(p, variable);
        self.phases.discriminant_seconds += started.elapsed().as_secs_f64();
        result
    }
    fn resultant(&mut self, p: &Poly, q: &Poly, variable: usize) -> Result<Poly> {
        let started = Instant::now();
        let result = self.cache.resultant(p, q, variable);
        self.phases.resultant_seconds += started.elapsed().as_secs_f64();
        result
    }
}

/// Return the generic McCallum local projection before factorization. Keeping
/// this pure lets the checker use checked factor reconstruction independently.
fn local_projection_raw(
    algebra: &Algebra,
    polynomials: &[Poly],
    cuts: &[usize],
    variable: usize,
    prefix: &[Rational],
    operations: &mut impl ProjectionOperations,
) -> Result<(Vec<usize>, Vec<Poly>, usize, usize, f64)> {
    let mut raw = Vec::new();
    let mut bearing = Vec::new();
    let mut root_seconds = 0.0;
    for &id in cuts {
        let p = polynomials
            .get(id)
            .ok_or_else(|| anyhow!("invalid cut id"))?;
        ensure!(
            highest(p) == Some(variable),
            "cut has the wrong highest variable"
        );
        let lc = p.univariate_lcoeff(variable);
        // Root-bearing classification is meaningful only on a full-degree fiber.
        // Return lc first even at a nongeneric prefix; the caller will refine it.
        raw.push(lc.clone());
        if p.degree(variable) > 1 {
            let disc = operations.discriminant(p, variable)?;
            ensure!(!disc.is_zero(), "local cut is not square-free");
            raw.push(disc);
        }
        if prefix_value(&lc, prefix)?.is_zero() {
            continue;
        }
        let started = Instant::now();
        let fiber = algebra.specialize_univariate(p, variable, prefix)?;
        ensure!(
            fiber.degree() <= crate::roots::MAX_ROOT_DEGREE,
            "local root classification exceeds supported univariate degree"
        );
        if !fiber.isolate_real_root_intervals().is_empty() {
            bearing.push(id);
        }
        root_seconds += started.elapsed().as_secs_f64();
    }
    // No successful certificate may use this prefix. Return its existing
    // obstruction before attempting potentially expensive pair resultants;
    // the builder propagates a vanishing factor and the checker rejects it.
    if raw
        .iter()
        .any(|p| prefix_value(p, prefix).is_ok_and(|v| v.is_zero()))
    {
        return Ok((bearing, raw, 0, 0, root_seconds));
    }
    let mut pairs = 0;
    for (j, &id) in bearing.iter().enumerate() {
        let p = &polynomials[id];
        for &other in &bearing[j + 1..] {
            let q = &polynomials[other];
            let r = operations.resultant(p, q, variable)?;
            ensure!(
                !r.is_zero(),
                "distinct irreducible cuts have zero resultant"
            );
            raw.push(r);
            pairs += 1;
        }
    }
    let all_pairs = cuts.len().saturating_mul(cuts.len().saturating_sub(1)) / 2;
    Ok((bearing, raw, pairs, all_pairs - pairs, root_seconds))
}

struct Builder<'a> {
    problem: &'a Problem,
    normalized: &'a NormalizedProblem,
    options: &'a LocalOptions,
    started: Instant,
    polynomials: Vec<Poly>,
    ids: HashMap<Poly, usize>,
    input_factors: Vec<Vec<usize>>,
    input_levels: Vec<BTreeSet<usize>>,
    report: LocalReport,
    operations: CachedOperations<'a>,
    progress_path: Option<PathBuf>,
    last_progress: Instant,
    guard_domains: Vec<GuardDomain>,
    protected: BTreeSet<usize>,
    guard_certificates: Vec<LocalGuardRemoval>,
    /// This memo only skips repeat bounded certification attempts. Successful
    /// entries reference proof records; no cached omission is trusted by replay.
    guard_certificate_index: HashMap<(usize, usize), Option<usize>>,
}
struct Built {
    node: LocalNode,
    required: BTreeSet<usize>,
    cells: usize,
}

impl Builder<'_> {
    fn snapshot(&self) -> LocalReport {
        let mut report = self.report.clone();
        report.cache = self.operations.cache.report.cache.clone();
        report.phases = self.operations.phases.clone();
        report.elapsed_seconds = self.started.elapsed().as_secs_f64();
        report
    }
    fn publish(&mut self, force: bool) -> Result<()> {
        let Some(path) = &self.progress_path else {
            return Ok(());
        };
        if !force && self.last_progress.elapsed().as_secs_f64() < 0.5 {
            return Ok(());
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("local-{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec(&self.snapshot())?)?;
        std::fs::rename(temporary, path)?;
        self.last_progress = Instant::now();
        Ok(())
    }
    fn check_budget(&self) -> Work<()> {
        if self.started.elapsed().as_secs_f64() >= self.problem.limits.wall_time_secs {
            return Err(Stop::Resource("local projection wall time limit".into()));
        }
        if self.report.speculative_nodes > self.options.max_nodes {
            return Err(Stop::Resource("local projection node limit".into()));
        }
        if self.report.refinements > self.options.max_refinements {
            return Err(Stop::Resource("local projection refinement limit".into()));
        }
        Ok(())
    }
    fn intern(&mut self, polynomial: Poly) -> usize {
        if let Some(&id) = self.ids.get(&polynomial) {
            return id;
        }
        let id = self.polynomials.len();
        self.ids.insert(polynomial.clone(), id);
        self.polynomials.push(polynomial);
        id
    }
    fn guard_nonzero(&self, required: &BTreeSet<usize>, prefix: &[Rational]) -> Work<()> {
        for &id in required {
            if prefix_value(&self.polynomials[id], prefix)?.is_zero() {
                return Err(Stop::Guard(id));
            }
        }
        Ok(())
    }
    fn certify_guard(
        &mut self,
        guard: usize,
        level: usize,
        prefix: &[Rational],
    ) -> Work<Option<usize>> {
        if !self.problem.solver.positivity
            || !self.options.domain_pruning
            || self.protected.contains(&guard)
        {
            return Ok(None);
        }
        let context = &self.guard_domains[level];
        if context.domain.empty {
            return Ok(None);
        }
        // Negative or boundary prefixes cannot borrow a requested domain as a
        // global assumption. Usually leaf() has already rejected/refined them.
        for &i in &context.supporting_constraints {
            if prefix_value(&self.normalized.positives[i], prefix)? <= Rational::zero() {
                return Ok(None);
            }
        }
        let key = (level, guard);
        if self.options.cache_enabled {
            if let Some(&known) = self.guard_certificate_index.get(&key) {
                self.report.guard_certificate_hits += 1;
                return Ok(known);
            }
        }
        self.report.guard_certificate_attempts += 1;
        let started = Instant::now();
        let certificate = certify_nonzero(
            &self.normalized.algebra,
            &self.polynomials[guard],
            &context.domain,
            &CertificateOptions::default(),
        )?;
        self.operations.phases.domain_certificate_seconds += started.elapsed().as_secs_f64();
        let index = if let Some(certificate) = certificate {
            let record = LocalGuardRemoval {
                level,
                guard,
                certificate,
                supporting_constraints: context.supporting_constraints.clone(),
                supporting_factors: supporting_factors(context, &self.input_factors),
            };
            // Even uncached ablations intern proof records deterministically;
            // they still recompute their algebraic certificate above.
            if let Some(Some(index)) = self.guard_certificate_index.get(&key) {
                if self.guard_certificates[*index] != record {
                    return Err(Stop::Unsupported(
                        "inconsistent repeated domain certificate".into(),
                    ));
                }
                Some(*index)
            } else {
                let index = self.guard_certificates.len();
                self.guard_certificates.push(record);
                Some(index)
            }
        } else {
            None
        };
        // Successful proof records need an index for deterministic interning.
        // Optional inconclusive-attempt memoization has a conservative bound;
        // zero cache memory retains only the proof registry itself.
        let memo_limit = (self.problem.solver.cache_memory_mib * 1024 * 1024 / 256).min(65_536);
        if index.is_some()
            || (self.options.cache_enabled && self.guard_certificate_index.len() < memo_limit)
        {
            self.guard_certificate_index.insert(key, index);
        }
        Ok(index)
    }
    fn projection_requirements(&self, projection: &LocalProjectionCertificate) -> BTreeSet<usize> {
        let mut required: BTreeSet<_> = projection.guards.iter().copied().collect();
        for &index in &projection.removals {
            required.extend(&self.guard_certificates[index].supporting_factors);
        }
        required
    }
    fn leaf(&self, prefix: &[Rational]) -> Work<Option<Built>> {
        for (i, p) in self.normalized.positives.iter().enumerate() {
            if highest(p).is_none_or(|j| j < prefix.len()) {
                let value = prefix_value(p, prefix)?;
                if value <= Rational::zero() {
                    let required: BTreeSet<_> = self.input_factors[i].iter().copied().collect();
                    self.guard_nonzero(&required, prefix)?;
                    return Ok(Some(Built {
                        node: LocalNode::Rejected { constraint: i },
                        required,
                        cells: 0,
                    }));
                }
            }
        }
        if prefix.len() != self.normalized.algebra.names.len() {
            return Ok(None);
        }
        let required: BTreeSet<_> = self.input_factors.iter().flatten().copied().collect();
        self.guard_nonzero(&required, prefix)?;
        Ok(Some(Built {
            node: LocalNode::Accepted {
                signs: self
                    .normalized
                    .split
                    .iter()
                    .map(|p| sign(&evaluate(p, prefix)))
                    .collect(),
            },
            required,
            cells: 1,
        }))
    }
    fn project(
        &mut self,
        cuts: &[usize],
        level: usize,
        prefix: &[Rational],
    ) -> Work<LocalProjectionCertificate> {
        self.check_budget()?;
        self.report.current_level = Some(level);
        self.report.active_phase = "local_projection".into();
        self.publish(false)?;
        let (root_bearing, raw, pairs, omitted, root_seconds) = local_projection_raw(
            &self.normalized.algebra,
            &self.polynomials,
            cuts,
            level,
            prefix,
            &mut self.operations,
        )?;
        self.operations.phases.root_classification_seconds += root_seconds;
        self.report.projected_pairs += pairs;
        self.report.omitted_rootless_pairs += omitted;
        let factors = self.operations.factors(&raw)?;
        let mut guards: BTreeSet<_> = factors.into_iter().map(|p| self.intern(p)).collect();
        self.guard_nonzero(&guards, prefix)?;
        let mut removals = Vec::new();
        for guard in guards.iter().copied().collect::<Vec<_>>() {
            self.check_budget()?;
            if let Some(index) = self.certify_guard(guard, level, prefix)? {
                guards.remove(&guard);
                removals.push(index);
                self.report.certified_guard_removals += 1;
            }
        }
        removals.sort_unstable();
        self.publish(false)?;
        self.check_budget()?;
        Ok(LocalProjectionCertificate {
            root_bearing,
            guards: guards.into_iter().collect(),
            removals,
        })
    }
    fn visit(&mut self, prefix: &mut Vec<Rational>) -> Work<Built> {
        self.report.speculative_nodes += 1;
        self.check_budget()?;
        if let Some(leaf) = self.leaf(prefix)? {
            return Ok(leaf);
        }
        let level = prefix.len();
        let mut cuts = self.input_levels[level].clone();
        loop {
            self.check_budget()?;
            let cut_list: Vec<_> = cuts.iter().copied().collect();
            let projection = self.project(&cut_list, level, prefix)?;
            self.report.active_phase = "root_isolation".into();
            self.publish(false)?;
            let root_started = Instant::now();
            let fibers = cut_list
                .iter()
                .map(|&id| {
                    Ok((
                        id,
                        self.normalized.algebra.specialize_univariate(
                            &self.polynomials[id],
                            level,
                            prefix,
                        )?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            let roots = isolate_union(fibers)?;
            self.operations.phases.root_isolation_seconds += root_started.elapsed().as_secs_f64();
            self.publish(false)?;
            let mut required = self.projection_requirements(&projection);
            let mut sectors = Vec::new();
            let mut cells = 0usize;
            let mut additions = BTreeSet::new();
            for i in 0..=roots.len() {
                self.check_budget()?;
                let sample =
                    sector_sample(if i > 0 { roots.get(i - 1) } else { None }, roots.get(i));
                prefix.push(sample.clone());
                let child = self.visit(prefix);
                prefix.pop();
                match child {
                    Err(Stop::Guard(id)) if highest(&self.polynomials[id]) == Some(level) => {
                        additions.insert(id);
                        break;
                    }
                    Err(stop) => return Err(stop),
                    Ok(child) => {
                        for id in child.required {
                            match highest(&self.polynomials[id]) {
                                Some(i) if i == level => {
                                    if !cuts.contains(&id) {
                                        additions.insert(id);
                                    }
                                }
                                Some(i) if i < level => {
                                    required.insert(id);
                                }
                                _ => {
                                    return Err(Stop::Unsupported(
                                        "child returned a guard in a later axis".into(),
                                    ));
                                }
                            }
                        }
                        if !additions.is_empty() {
                            break;
                        }
                        cells = cells
                            .checked_add(child.cells)
                            .ok_or_else(|| Stop::Resource("cell count overflow".into()))?;
                        if self.problem.limits.max_cells.is_some_and(|max| cells > max) {
                            return Err(Stop::Resource("local projection cell limit".into()));
                        }
                        sectors.push(LocalSector {
                            sample: sample.to_string(),
                            child: child.node,
                        });
                    }
                }
            }
            if !additions.is_empty() {
                let before = cuts.len();
                cuts.extend(additions);
                if cuts.len() == before {
                    return Err(Stop::Unsupported(
                        "nongeneric sample remained on an existing stack cut".into(),
                    ));
                }
                self.report.refinements += 1;
                continue;
            }
            self.guard_nonzero(&required, prefix)?;
            return Ok(Built {
                node: LocalNode::Stack {
                    cuts: cut_list,
                    projection,
                    roots: roots.iter().map(bound).collect(),
                    sectors,
                    required_below: required.iter().copied().collect(),
                },
                required,
                cells,
            });
        }
    }
}

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    ensure!(
        problem.solver.homogeneous_anchor.is_none(),
        "standalone local projection requires homogeneity to be applied outside it"
    );
    if !problem.solver.order.is_empty() {
        return Ok(problem.solver.order.clone());
    }
    let groups = if problem.variable_groups.is_empty() {
        vec![problem.variables.clone()]
    } else {
        problem.variable_groups.clone()
    };
    if problem.solver.ordering == "given" {
        return Ok(groups.into_iter().flatten().collect());
    }
    let method = match problem.solver.ordering.as_str() {
        "brown" => crate::ordering::OrderingMethod::Brown,
        "sparsity" | "chordal" => crate::ordering::OrderingMethod::Sparsity,
        _ => anyhow::bail!(
            "local projection requires given, explicit, Brown or sparsity ordering; MCTS must supply an explicit trial order"
        ),
    };
    let normalized = normalize(problem, &problem.variables)?;
    let mut all = normalized.positives;
    all.extend(normalized.split);
    let mut eliminated = Vec::new();
    // Static cheap scores deliberately perform no global projection. These
    // are ordering hints only; the local proof establishes correctness later.
    for group in groups.iter().rev() {
        let candidates: Vec<_> = group
            .iter()
            .map(|v| {
                problem
                    .variables
                    .iter()
                    .position(|x| x == v)
                    .expect("validated variable group")
            })
            .collect();
        eliminated.extend(crate::ordering::rank_candidates(&all, &candidates, method));
    }
    eliminated.reverse();
    Ok(eliminated
        .into_iter()
        .map(|i| problem.variables[i].clone())
        .collect())
}

fn collect_output(
    node: &LocalNode,
    order: &[String],
    axes: &mut Vec<Axis>,
    cells: &mut Vec<Cell>,
    used: &mut BTreeSet<usize>,
    statistics: &mut Statistics,
) {
    match node {
        LocalNode::Accepted { signs } => cells.push(Cell {
            axes: axes.clone(),
            signs: signs.clone(),
        }),
        LocalNode::Rejected { .. } => statistics.rejected_branches += 1,
        LocalNode::Stack {
            cuts,
            roots,
            sectors,
            ..
        } => {
            statistics.visited_stacks += 1;
            used.extend(cuts);
            for (i, sector) in sectors.iter().enumerate() {
                axes.push(Axis {
                    variable: order[axes.len()].clone(),
                    sample: sector.sample.clone(),
                    lower: if i > 0 {
                        roots.get(i - 1).cloned()
                    } else {
                        None
                    },
                    upper: roots.get(i).cloned(),
                });
                collect_output(&sector.child, order, axes, cells, used, statistics);
                axes.pop();
            }
        }
    }
}

fn fallback(
    problem: &Problem,
    options: &LocalOptions,
    started: Instant,
    reason: String,
) -> Result<LocalSolveResult> {
    if !options.fallback_to_global {
        return Ok(LocalSolveResult {
            solution: SolveResult::failure(problem.clone(), Status::ComputationFailure, &reason),
            evidence: LocalEvidence::NotApplicable { reason },
        });
    }
    let remaining = problem.limits.wall_time_secs - started.elapsed().as_secs_f64();
    if remaining <= 0.0 {
        return Ok(unfinished(
            problem,
            "wall time exhausted before global fallback".into(),
            started,
            None,
        ));
    }
    let mut global_problem = problem.clone();
    // Preserve the requested order, groups, constraints and transformations.
    global_problem.solver.engine = "optimized".into();
    global_problem.limits.wall_time_secs = remaining;
    let mut solution = crate::solver::solve(&global_problem)?;
    solution.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(LocalSolveResult {
        solution,
        evidence: LocalEvidence::GlobalFallback { reason },
    })
}

fn unfinished(
    problem: &Problem,
    reason: String,
    started: Instant,
    report: Option<LocalReport>,
) -> LocalSolveResult {
    let mut solution = SolveResult::failure(problem.clone(), Status::Incomplete, &reason);
    solution.statistics.total_seconds = started.elapsed().as_secs_f64();
    LocalSolveResult {
        solution,
        evidence: LocalEvidence::Unfinished { reason, report },
    }
}

pub fn solve_local(problem: &Problem, options: &LocalOptions) -> Result<LocalSolveResult> {
    problem.validate()?;
    let started = Instant::now();
    let order = match selected_order(problem) {
        Ok(order) => order,
        Err(error) => return fallback(problem, options, started, error.to_string()),
    };
    let normalized = normalize(problem, &order)?;
    let cache_options = OptimizedProjectionOptions {
        checkpoint_dir: options
            .cache_enabled
            .then(|| problem.solver.checkpoint_dir.clone())
            .flatten(),
        memory_cache_bytes: if options.cache_enabled {
            problem.solver.cache_memory_mib * 1024 * 1024
        } else {
            0
        },
        capture_dir: std::env::var_os("SYMGCAD_CAPTURE_DIR").map(PathBuf::from),
        domain_fingerprint: serde_json::to_string(&(
            "local-generic-mc-v1",
            &order,
            &problem.constraints,
            &problem.split,
            &problem.parameters,
        ))?,
        ..Default::default()
    };
    let mut builder = Builder {
        problem,
        normalized: &normalized,
        options,
        started,
        polynomials: Vec::new(),
        ids: HashMap::new(),
        input_factors: Vec::new(),
        input_levels: vec![BTreeSet::new(); order.len()],
        report: LocalReport::default(),
        operations: CachedOperations {
            cache: ProjectionCache::new(&normalized.algebra, &cache_options),
            phases: LocalPhaseCosts::default(),
        },
        progress_path: std::env::var_os("SYMGCAD_PROGRESS_PATH").map(PathBuf::from),
        last_progress: started,
        guard_domains: (0..order.len())
            .map(|level| guard_domain(&normalized, level))
            .collect::<Result<_>>()?,
        protected: BTreeSet::new(),
        guard_certificates: Vec::new(),
        guard_certificate_index: HashMap::new(),
    };
    builder.report.active_phase = "input_factorization".into();
    builder.publish(true)?;
    for p in normalized.positives.iter().chain(&normalized.split) {
        let mut input = Vec::new();
        for factor in builder.operations.factors(std::slice::from_ref(p))? {
            let level = highest(&factor).expect("nonconstant factor");
            let id = builder.intern(factor);
            builder.input_levels[level].insert(id);
            input.push(id);
        }
        input.sort_unstable();
        builder.input_factors.push(input);
    }
    builder.protected = builder.input_factors.iter().flatten().copied().collect();
    let built = match builder.visit(&mut Vec::new()) {
        Ok(built) => built,
        Err(Stop::Resource(reason)) => {
            builder.report.active_phase = "incomplete".into();
            builder.publish(true)?;
            return Ok(unfinished(
                problem,
                reason,
                started,
                Some(builder.snapshot()),
            ));
        }
        Err(Stop::Unsupported(reason)) => return fallback(problem, options, started, reason),
        Err(Stop::Guard(id)) => {
            return fallback(
                problem,
                options,
                started,
                format!(
                    "unresolved top-level guard {}",
                    normalized.algebra.format(&builder.polynomials[id])
                ),
            );
        }
    };
    ensure!(
        built.required.is_empty(),
        "root retained an unproved projection obligation"
    );
    let mut cells = Vec::new();
    let mut used = BTreeSet::new();
    let mut statistics = Statistics::default();
    collect_output(
        &built.node,
        &order,
        &mut Vec::new(),
        &mut cells,
        &mut used,
        &mut statistics,
    );
    let mut projection_levels = vec![Vec::new(); order.len()];
    for &id in &used {
        projection_levels[highest(&builder.polynomials[id]).unwrap()].push(id);
    }
    statistics.raw_cells = cells.len();
    statistics.total_seconds = started.elapsed().as_secs_f64();
    let merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&cells));
    if started.elapsed().as_secs_f64() >= problem.limits.wall_time_secs {
        return Ok(unfinished(
            problem,
            "wall time limit during local output construction".into(),
            started,
            Some(builder.snapshot()),
        ));
    }
    builder.report.current_level = None;
    builder.report.active_phase = "complete".into();
    builder.publish(true)?;
    let report = builder.snapshot();
    statistics.projection_seconds = report.phases.factor_seconds
        + report.phases.discriminant_seconds
        + report.phases.resultant_seconds
        + report.phases.domain_certificate_seconds;
    statistics.lifting_seconds =
        report.phases.root_classification_seconds + report.phases.root_isolation_seconds;
    let solution = SolveResult {
        schema_version: 1,
        status: Status::CompleteGeneric,
        problem: problem.clone(),
        order,
        normalized_constraints: normalized
            .positives
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect(),
        split_polynomials: normalized
            .split
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect(),
        polynomials: builder
            .polynomials
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect(),
        projection_levels,
        exceptional_polynomials: used.into_iter().collect(),
        cells,
        trace: None,
        statistics,
        diagnostic: None,
        merged_cells,
        homogeneous: None,
        monotone: None,
        local: None,
    };
    Ok(LocalSolveResult {
        solution,
        evidence: LocalEvidence::Local {
            options: options.clone(),
            input_factors: builder.input_factors,
            guard_certificates: builder.guard_certificates,
            root: built.node,
            report,
        },
    })
}

struct Checker<'a> {
    normalized: &'a NormalizedProblem,
    polynomials: &'a [Poly],
    ids: HashMap<Poly, usize>,
    input_factors: &'a [Vec<usize>],
    guard_certificates: &'a [LocalGuardRemoval],
    guard_domains: Vec<GuardDomain>,
    protected: BTreeSet<usize>,
    domain_pruning: bool,
}
impl Checker<'_> {
    fn check_registry(&self) -> Result<()> {
        ensure!(
            self.domain_pruning || self.guard_certificates.is_empty(),
            "domain guard removals require enabled local positivity"
        );
        let mut seen = BTreeSet::new();
        for record in self.guard_certificates {
            let context = self
                .guard_domains
                .get(record.level)
                .ok_or_else(|| anyhow!("guard certificate has invalid level"))?;
            let polynomial = self
                .polynomials
                .get(record.guard)
                .ok_or_else(|| anyhow!("invalid certified guard id"))?;
            ensure!(
                highest(polynomial).is_some_and(|i| i < record.level),
                "certified guard is not in an earlier axis"
            );
            ensure!(
                seen.insert((record.level, record.guard)),
                "duplicate domain guard certificate"
            );
            ensure!(
                !self.protected.contains(&record.guard),
                "an original constraint or split factor was removed"
            );
            ensure!(
                !context.domain.empty,
                "empty domain cannot certify a local guard"
            );
            ensure!(
                record.supporting_constraints == context.supporting_constraints,
                "changed original bound constraint provenance"
            );
            ensure!(
                record.supporting_factors == supporting_factors(context, self.input_factors),
                "incomplete supporting bound factor obligations"
            );
            for &id in &record.supporting_factors {
                ensure!(
                    self.protected.contains(&id)
                        && highest(&self.polynomials[id]).is_some_and(|i| i < record.level),
                    "bound support is not a protected ancestor factor"
                );
            }
            verify_certificate(
                &self.normalized.algebra,
                polynomial,
                &self.normalized.positives,
                &context.excluded,
                &record.certificate,
            )?;
        }
        Ok(())
    }
    fn nonzero(&self, guards: &BTreeSet<usize>, prefix: &[Rational]) -> Result<()> {
        for &id in guards {
            let p = self
                .polynomials
                .get(id)
                .ok_or_else(|| anyhow!("invalid guard id"))?;
            ensure!(
                !prefix_value(p, prefix)?.is_zero(),
                "projection guard vanishes at the certified prefix"
            );
        }
        Ok(())
    }
    fn walk(&self, node: &LocalNode, prefix: &mut Vec<Rational>) -> Result<BTreeSet<usize>> {
        let level = prefix.len();
        match node {
            LocalNode::Rejected { constraint } => {
                let p = self
                    .normalized
                    .positives
                    .get(*constraint)
                    .ok_or_else(|| anyhow!("invalid rejected constraint"))?;
                ensure!(
                    prefix_value(p, prefix)? <= Rational::zero(),
                    "rejected constraint is not nonpositive"
                );
                let required = self.input_factors[*constraint].iter().copied().collect();
                // Nonconstant zero values cannot justify rejecting an open cell.
                self.nonzero(&required, prefix)?;
                Ok(required)
            }
            LocalNode::Accepted { signs } => {
                ensure!(
                    level == self.normalized.algebra.names.len(),
                    "accepted before assigning every coordinate"
                );
                ensure!(
                    self.normalized
                        .positives
                        .iter()
                        .all(|p| evaluate(p, prefix) > Rational::zero()),
                    "accepted constraint is not positive"
                );
                let expected: Vec<_> = self
                    .normalized
                    .split
                    .iter()
                    .map(|p| sign(&evaluate(p, prefix)))
                    .collect();
                ensure!(signs == &expected, "incorrect split signs");
                let required = self.input_factors.iter().flatten().copied().collect();
                self.nonzero(&required, prefix)?;
                Ok(required)
            }
            LocalNode::Stack {
                cuts,
                projection,
                roots,
                sectors,
                required_below,
            } => {
                ensure!(
                    level < self.normalized.algebra.names.len(),
                    "stack after the final axis"
                );
                let cut_set: BTreeSet<_> = cuts.iter().copied().collect();
                ensure!(
                    cuts == &cut_set.iter().copied().collect::<Vec<_>>(),
                    "cuts are not unique and sorted"
                );
                let (bearing, raw, _, _, _) = local_projection_raw(
                    &self.normalized.algebra,
                    self.polynomials,
                    cuts,
                    level,
                    prefix,
                    &mut IndependentOperations,
                )?;
                ensure!(
                    projection.root_bearing == bearing,
                    "incorrect root-bearing cut subset"
                );
                let mut projected = BTreeSet::new();
                for p in factors_checked(&raw)? {
                    projected.insert(
                        *self
                            .ids
                            .get(&p)
                            .ok_or_else(|| anyhow!("missing local projection guard factor"))?,
                    );
                }
                let retained: BTreeSet<_> = projection.guards.iter().copied().collect();
                ensure!(
                    projection.guards == retained.iter().copied().collect::<Vec<_>>(),
                    "retained guards are not unique and sorted"
                );
                let removal_ids: BTreeSet<_> = projection.removals.iter().copied().collect();
                ensure!(
                    projection.removals == removal_ids.iter().copied().collect::<Vec<_>>(),
                    "guard removal references are not unique and sorted"
                );
                let mut covered = retained.clone();
                let mut required = retained;
                for &index in &projection.removals {
                    ensure!(self.domain_pruning, "domain guard removal is disabled");
                    let record = self
                        .guard_certificates
                        .get(index)
                        .ok_or_else(|| anyhow!("invalid guard certificate reference"))?;
                    ensure!(
                        record.level == level,
                        "guard certificate belongs to a different ancestor level"
                    );
                    ensure!(
                        covered.insert(record.guard),
                        "a guard is retained or removed more than once"
                    );
                    for &source in &record.supporting_constraints {
                        ensure!(
                            prefix_value(&self.normalized.positives[source], prefix)?
                                > Rational::zero(),
                            "local prefix does not satisfy a certificate bound premise"
                        );
                    }
                    // These nonzero obligations reach the owning ancestors.
                    // Their constant signs and the positive values above prove
                    // the *whole* current base cell lies in the certified box.
                    required.extend(&record.supporting_factors);
                }
                ensure!(
                    covered == projected,
                    "local projection obligations omitted or changed"
                );
                self.nonzero(&projected, prefix)?;
                let fibers = cuts
                    .iter()
                    .map(|&id| {
                        Ok((
                            id,
                            self.normalized.algebra.specialize_univariate(
                                &self.polynomials[id],
                                level,
                                prefix,
                            )?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let actual_roots = isolate_union(fibers)?;
                let expected_roots: Vec<_> = actual_roots.iter().map(bound).collect();
                ensure!(
                    roots == &expected_roots,
                    "incomplete or changed local root union"
                );
                ensure!(
                    sectors.len() == actual_roots.len() + 1,
                    "local stack omits an open sector"
                );
                for (i, sector) in sectors.iter().enumerate() {
                    let expected = sector_sample(
                        if i > 0 { actual_roots.get(i - 1) } else { None },
                        actual_roots.get(i),
                    );
                    ensure!(
                        sector.sample == expected.to_string(),
                        "local sample is not the certified sector sample"
                    );
                    prefix.push(expected);
                    let child_required = self.walk(&sector.child, prefix)?;
                    prefix.pop();
                    for id in child_required {
                        match highest(&self.polynomials[id]) {
                            Some(j) if j == level => ensure!(
                                cut_set.contains(&id),
                                "child boundary not jointly included among sibling cuts"
                            ),
                            Some(j) if j < level => {
                                required.insert(id);
                            }
                            _ => anyhow::bail!("child guard depends on a later axis"),
                        }
                    }
                }
                ensure!(
                    required_below == &required.iter().copied().collect::<Vec<_>>(),
                    "incomplete lower-level obligations"
                );
                self.nonzero(&required, prefix)?;
                Ok(required)
            }
        }
    }
}

/// Replay the algebraic obligations and exhaustive tree independently of the
/// discovery/refinement algorithm. In particular, valid samples alone cannot
/// pass when a sibling boundary, local projection guard or sector is missing.
pub fn verify_local(result: &LocalSolveResult) -> Result<Verification> {
    verify_local_parts(&result.solution, &result.evidence)
}

pub fn verify_local_parts(
    solution: &SolveResult,
    evidence: &LocalEvidence,
) -> Result<Verification> {
    ensure!(
        solution.status == Status::CompleteGeneric,
        "only complete local results can be certified"
    );
    let (input_factors, guard_certificates, root, options) = match evidence {
        LocalEvidence::GlobalFallback { .. } => return crate::verify::verify_nonlocal(solution),
        LocalEvidence::NotApplicable { .. } => anyhow::bail!("local projection was not applicable"),
        LocalEvidence::Unfinished { .. } => anyhow::bail!("unfinished local projection evidence"),
        LocalEvidence::Local {
            input_factors,
            guard_certificates,
            root,
            options,
            ..
        } => (input_factors, guard_certificates, root, options),
    };
    solution.problem.validate()?;
    ensure!(solution.schema_version == 1, "unsupported output schema");
    ensure!(
        solution.order == selected_order(&solution.problem)?,
        "local output changed the fixed variable order"
    );
    ensure!(
        solution.trace.is_none() && solution.homogeneous.is_none() && solution.monotone.is_none(),
        "incompatible additional proof evidence"
    );
    let normalized = normalize(&solution.problem, &solution.order)?;
    ensure!(
        solution.normalized_constraints
            == normalized
                .positives
                .iter()
                .map(|p| normalized.algebra.format(p))
                .collect::<Vec<_>>(),
        "changed normalized constraints"
    );
    ensure!(
        solution.split_polynomials
            == normalized
                .split
                .iter()
                .map(|p| normalized.algebra.format(p))
                .collect::<Vec<_>>(),
        "changed normalized splits"
    );
    let polynomials = solution
        .polynomials
        .iter()
        .map(|p| normalized.algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let mut ids = HashMap::new();
    for (id, p) in polynomials.iter().enumerate() {
        ensure!(
            !p.is_constant() && canonical(p) == *p,
            "noncanonical or constant local polynomial"
        );
        ensure!(
            ids.insert(p.clone(), id).is_none(),
            "duplicate local polynomial"
        );
        ensure!(
            factors_checked(std::slice::from_ref(p))? == vec![p.clone()],
            "local cut is not a primitive irreducible factor"
        );
    }
    ensure!(
        input_factors.len() == normalized.positives.len() + normalized.split.len(),
        "wrong number of input factor witnesses"
    );
    for (p, factors) in normalized
        .positives
        .iter()
        .chain(&normalized.split)
        .zip(input_factors)
    {
        let expected = factors_checked(std::slice::from_ref(p))?
            .into_iter()
            .map(|q| {
                ids.get(&q)
                    .copied()
                    .ok_or_else(|| anyhow!("input factor missing from local table"))
            })
            .collect::<Result<BTreeSet<_>>>()?;
        ensure!(
            factors == &expected.into_iter().collect::<Vec<_>>(),
            "incomplete input factorization witness"
        );
    }
    let checker = Checker {
        normalized: &normalized,
        polynomials: &polynomials,
        ids,
        input_factors,
        guard_certificates,
        guard_domains: (0..solution.order.len())
            .map(|level| guard_domain(&normalized, level))
            .collect::<Result<_>>()?,
        protected: input_factors.iter().flatten().copied().collect(),
        domain_pruning: options.domain_pruning && solution.problem.solver.positivity,
    };
    checker.check_registry()?;
    ensure!(
        checker.walk(root, &mut Vec::new())?.is_empty(),
        "unproved top-level projection guards"
    );
    let mut cells = Vec::new();
    let mut used = BTreeSet::new();
    let mut statistics = Statistics::default();
    collect_output(
        root,
        &solution.order,
        &mut Vec::new(),
        &mut cells,
        &mut used,
        &mut statistics,
    );
    ensure!(
        solution.cells == cells,
        "output cells do not match the exhaustive local tree"
    );
    let mut levels = vec![Vec::new(); solution.order.len()];
    for &id in &used {
        levels[highest(&polynomials[id]).unwrap()].push(id);
    }
    ensure!(
        solution.projection_levels == levels,
        "incorrect local boundary level inventory"
    );
    ensure!(
        solution.exceptional_polynomials == used.into_iter().collect::<Vec<_>>(),
        "incomplete excluded-seam inventory"
    );
    let merged = (solution.problem.solver.merge == "aggressive").then(|| merge(&cells));
    ensure!(
        solution.merged_cells == merged,
        "incorrect compatibility merge"
    );
    ensure!(
        solution
            .problem
            .limits
            .max_cells
            .is_none_or(|max| cells.len() <= max),
        "complete result exceeds cell limit"
    );
    Ok(Verification {
        verified: true,
        cells: cells.len(),
        stacks: statistics.visited_stacks,
        rejected_branches: statistics.rejected_branches,
    })
}
