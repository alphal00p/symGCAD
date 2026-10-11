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
//! The opt-in adjacent-pair rule strengthens Algorithm 8, which uses all
//! root-bearing pairs. On a connected parent cell the individual relevant root
//! branches are continuous and simple. A nonvanishing guard for each adjacent
//! pair preserves its strict sample ordering; transitivity preserves the whole
//! order. Exact coincidences use all pairs instead. Every sibling refinement
//! rebuilds this ordering, and the checker reconstructs it independently.
//!
//! Optional original-domain certificates replace a projection guard only by
//! explicit obligations for the earlier coordinate bounds proving it nonzero.
//! Protected original factors make those premises sign-invariant on the whole
//! ancestor cell. No certificate borrows an unassigned coordinate's bounds,
//! and no input split factor or coordinate boundary is removed.
//!
//! With an original positive coordinate bound, a separate positive-ray theorem
//! retains the endpoint and leading coefficient of each cut. Quadratics use
//! exact coefficient signs to omit discriminants governing only negative roots;
//! other cases retain the ordinary discriminant. Pair resultants are then
//! needed only between positive-root-bearing cuts. One explicit rejected
//! negative half-line and every positive sector prove exhaustive coverage.
//!
//! At the first lifting axis the base consists of a single rational point.
//! Exact univariate root isolation then suffices: constant projection guards
//! cannot refine that base, and no pair resultants or discriminants are needed.
use crate::{
    Problem,
    algebra::{Algebra, Poly, canonical},
    domain::{Domain, DomainCertificate, certify_nonzero, derive_domain, verify_certificate},
    optimized_projection::{CacheMetrics, OptimizedProjectionOptions, ProjectionCache},
    output::{Axis, Cell, RootBound, SolveResult, Statistics, Status},
    problem::{NormalizedProblem, normalize},
    projection::{discriminant, exact_resultant},
    roots::{FiberRoot, isolate_union, sector_sample},
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

mod descartes;
mod interval_descartes;
pub use interval_descartes::{PositiveInterval, PositiveIntervalEndpoint, PositiveIntervalWitness};
mod base_univariate;
#[cfg(test)]
mod base_univariate_replay;
#[cfg(test)]
mod base_univariate_report_tests;
mod hint_archive;
pub use base_univariate::{
    BaseUnivariateAttempt, BaseUnivariateBasis, BaseUnivariateBlock, BaseUnivariateCacheReport,
    BaseUnivariateDecline, BaseUnivariateDeclines, BaseUnivariateReport, BaseUnivariateSource,
};
mod lazy_projection;
mod learned_cuts;
mod obstruction_observer;
pub use hint_archive::HintArchiveReport;
pub use obstruction_observer::ObstructionReport;
mod pair_witnesses;
mod strict_bezout;
pub use strict_bezout::{PositiveStrictBezout, StrictBezoutHint};
mod stack_samples;
mod subtree_cache;

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
    /// Use an exact positive-ray theorem only with an original k*x>0 bound.
    /// False retains the full-real local projection for controlled ablations.
    pub positive_ray: bool,
    /// Try bounded pseudo-remainders to separate positive roots.
    /// Older evidence omitted this field and must replay the original pairs.
    #[serde(default)]
    pub positive_pair_separation: bool,
    /// Separate every adjacent cross-owner positive branch by guarded rational
    /// evaluations. A finite stock per pair prevents unbounded new templates.
    #[serde(default)]
    pub positive_pair_witnesses: bool,
    /// Use only explicitly proposed and independently checked strict polynomial
    /// combination identities; empty historical evidence retains resultants.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strict_bezout: bool,
    /// Maximum leading-term cancellations per pair, either one or two.
    /// Missing historical fields retain the original one-cancellation proof.
    #[serde(default = "one_cancellation")]
    pub positive_pair_cancellations: usize,
    /// Collect a bounded set of sibling discoveries before restarting a stack.
    /// False restarts after the first discovery, for controlled ablations.
    /// The final proof and its independent replay do not depend on this choice.
    #[serde(default)]
    pub batch_sibling_discovery: bool,
    /// Reuse exact completed local subtrees at the same rational prefix.
    #[serde(default)]
    pub reuse_completed_subtrees: bool,
    /// Reproject previously discovered cuts when an unfinished stack is revisited.
    /// The hints contain no accepted cells or unverified proof obligations.
    #[serde(default)]
    pub reuse_learned_cuts: bool,
    /// Reuse visited rationals inside refined sectors at the same exact prefix.
    /// This only changes sample selection; children are visited and checked.
    #[serde(default)]
    pub stable_sector_samples: bool,
    /// Defer stack projection until the child frontier is complete.
    /// Requires stable samples; old evidence preserves eager construction.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub defer_projection: bool,
    /// Read-only bounded failed-visit observations; never proof authority.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub observe_obstructions: bool,
    /// Use one Descartes sign variation to certify a simple positive root.
    /// Historical evidence retains its full discriminant obligations.
    #[serde(default)]
    pub descartes_one: bool,
    /// Saturate a bounded Descartes upper bound with exact separator signs.
    /// Disabled by default; historical evidence retains discriminants.
    #[serde(default)]
    pub descartes_witnesses: bool,
    /// Source-term admission limit shared by discovery, reuse and proof replay.
    /// Omitted historical evidence retains the original 4096-term limit.
    #[serde(
        default = "crate::config::default_descartes_witness_max_terms",
        skip_serializing_if = "crate::config::is_default_descartes_witness_max_terms"
    )]
    pub descartes_witness_max_terms: usize,
    /// Cover the positive ray with bounded rational Descartes intervals.
    /// A separate permanent stock bounds covers per polynomial.
    #[serde(default)]
    pub descartes_intervals: bool,
    /// Permanent interval templates per polynomial, from two through eight.
    #[serde(
        default = "two_interval_templates",
        skip_serializing_if = "is_two_interval_templates"
    )]
    pub descartes_interval_template_limit: usize,
    /// At level zero isolate the complete univariate root union directly.
    /// Missing historical evidence retains ordinary local projection replay.
    #[serde(default)]
    pub base_point_projection: bool,
    /// Experimental exact support blocks for generated first-axis guards only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub base_univariate_basis: bool,
    /// Experimental: project only consecutive relevant root branches after
    /// their individual delineability guards have been established.
    #[serde(default)]
    pub adjacent_pairs: bool,
    /// Retained conditional-proof payload budget, clipped to 1/16 of the
    /// process memory limit. Count caps grow with budgets above the default.
    pub subtree_cache_mib: usize,
    /// Retained cut-hint budget, clipped to 1/128 of process memory.
    pub learned_cut_cache_mib: usize,
}
fn two_interval_templates() -> usize {
    2
}

fn is_zero_count(value: &usize) -> bool {
    *value == 0
}
fn is_two_interval_templates(value: &usize) -> bool {
    *value == 2
}
fn one_cancellation() -> usize {
    1
}
impl Default for LocalOptions {
    fn default() -> Self {
        Self {
            max_nodes: 100_000,
            max_refinements: 10_000,
            fallback_to_global: true,
            cache_enabled: true,
            domain_pruning: true,
            positive_ray: true,
            positive_pair_separation: true,
            positive_pair_witnesses: false,
            strict_bezout: false,
            positive_pair_cancellations: 2,
            batch_sibling_discovery: true,
            reuse_completed_subtrees: true,
            reuse_learned_cuts: true,
            stable_sector_samples: false,
            defer_projection: false,
            observe_obstructions: false,
            descartes_one: true,
            descartes_witnesses: false,
            descartes_witness_max_terms: crate::config::DEFAULT_DESCARTES_WITNESS_MAX_TERMS,
            descartes_intervals: false,
            descartes_interval_template_limit: 2,
            base_point_projection: true,
            base_univariate_basis: false,
            adjacent_pairs: false,
            subtree_cache_mib: 32,
            learned_cut_cache_mib: 8,
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
    pub pair_separation_seconds: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_univariate_basis: Option<BaseUnivariateReport>,
    /// Diagnostic failed visits, including propagation; no coverage evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obstruction_observer: Option<ObstructionReport>,
    /// Optional append-only registry observations for ordering search. These
    /// diagnostic counters are never inputs to certificate verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_inventory: Option<crate::search::local_progress::LocalInventory>,
    pub speculative_nodes: usize,
    pub refinements: usize,
    /// New stack cuts retained across all speculative refinement rounds.
    pub refinement_cuts: usize,
    /// Refinement rounds which discovered more than one new cut.
    pub multi_cut_refinements: usize,
    /// Tentative siblings visited after another sibling invalidated the stack.
    pub discovery_siblings: usize,
    /// Opt-in complete-child discovery before the ordinary final projection.
    pub deferred_stacks: usize,
    pub deferred_frontiers: usize,
    pub deferred_projection_calls: usize,
    /// Bounded scheduling declines which resumed the ordinary project-first loop.
    pub deferred_fallbacks: usize,
    /// Strictly decreasing cut-set passes; no new children run in these passes.
    pub compaction_rounds: usize,
    pub compacted_cuts: usize,
    pub compacted_children: usize,
    /// Largest number of new cuts collected in one refinement round.
    pub max_refinement_cuts: usize,
    pub projected_pairs: usize,
    pub omitted_rootless_pairs: usize,
    pub cache: CacheMetrics,
    pub phases: LocalPhaseCosts,
    pub elapsed_seconds: f64,
    pub current_level: Option<usize>,
    /// Rational coordinates already assigned at the current stack. In a
    /// parameter-outer problem these identify the exact kinematic branch.
    pub current_prefix: Vec<String>,
    pub active_phase: String,
    pub guard_certificate_attempts: usize,
    pub guard_certificate_hits: usize,
    pub certified_guard_removals: usize,
    pub positive_ray_stacks: usize,
    pub quadratic_discriminants_omitted: usize,
    pub descartes_discriminants_omitted: usize,
    pub descartes_one_discriminants_omitted: usize,
    pub descartes_witness_discriminants_omitted: usize,
    pub descartes_witness_templates: usize,
    pub descartes_witness_template_reuses: usize,
    pub descartes_witness_budget_fallbacks: usize,
    /// Visits declined by the source-term cap, separate from template stock.
    #[serde(skip_serializing_if = "is_zero_count")]
    pub descartes_witness_source_term_fallbacks: usize,
    pub descartes_interval_discriminants_omitted: usize,
    pub descartes_interval_templates: usize,
    pub descartes_interval_template_limit: usize,
    pub descartes_interval_template_reuses: usize,
    pub descartes_interval_budget_fallbacks: usize,
    pub pair_separation_candidates: usize,
    pub pair_resultants_omitted: usize,
    pub pair_witness_resultants_omitted: usize,
    pub pair_witness_templates: usize,
    pub pair_witness_template_reuses: usize,
    pub pair_witness_budget_fallbacks: usize,
    pub strict_bezout_hints_accepted: usize,
    pub strict_bezout_hints_rejected: usize,
    pub strict_bezout_resultants_omitted: usize,
    pub strict_bezout_resultant_fallbacks: usize,
    pub strict_bezout_preparation_seconds: f64,
    pub strict_bezout_rejections: Vec<String>,
    pub stable_sample_hits: usize,
    pub stable_sample_misses: usize,
    pub stable_sample_recorded: usize,
    pub stable_sample_evictions: usize,
    pub stable_sample_peak_bytes: usize,
    pub second_cancellation_pairs_omitted: usize,
    pub subtree_cache_hits: usize,
    pub subtree_cache_misses: usize,
    pub subtree_cache_evictions: usize,
    pub subtree_cache_retained_bytes: usize,
    pub subtree_cache_peak_bytes: usize,
    pub subtree_cache_retained_nodes: usize,
    pub subtree_cache_peak_nodes: usize,
    pub learned_cut_cache_hits: usize,
    pub learned_cut_cache_misses: usize,
    pub learned_cut_seeded_cuts: usize,
    pub learned_cut_cache_evictions: usize,
    pub learned_cut_cache_retained_bytes: usize,
    pub learned_cut_cache_peak_bytes: usize,
    pub learned_cut_cache_retained_ids: usize,
    pub learned_cut_cache_retained_entries: usize,
    pub base_point_stacks: usize,
    /// All potential pairs avoided, including pairs with rootless cuts.
    pub base_point_candidate_pairs_skipped: usize,
    pub base_point_max_cuts: usize,
    pub adjacent_pair_stacks: usize,
    pub nonadjacent_pairs_omitted: usize,
    pub coincident_pair_fallbacks: usize,
    pub subtree_cache_budget_bytes: usize,
    pub subtree_cache_node_limit: usize,
    pub learned_cut_cache_budget_bytes: usize,
    pub learned_cut_cache_entry_limit: usize,
    pub learned_cut_cache_id_limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint_archive: Option<HintArchiveReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalProjectionCertificate {
    /// The zero-dimensional-base theorem uses the complete univariate root
    /// union directly. No root-bearing subset or projection guards are used.
    #[serde(default)]
    pub base_point: bool,
    /// Cuts with real roots, or strictly positive roots for a positive-ray proof.
    /// Empty for the base-point theorem, which isolates every cut instead.
    pub root_bearing: Vec<usize>,
    /// Square-free irreducible factors of all required lc, disc and resultants.
    pub guards: Vec<usize>,
    /// References into LocalEvidence::Local.guard_certificates. Their bound
    /// factors replace the removed guards as explicit ancestor obligations.
    #[serde(default)]
    pub removals: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positive_ray: Option<PositiveRayProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjacent_pairs: Option<AdjacentPairProjection>,
}

/// Sample ordering is used only after every individual relevant root function
/// is delineable on the eventual connected parent cell. Nonvanishing selected
/// pair guards preserve every adjacent inequality, hence the complete order.
/// Shared roots lose owner information in the union, so they use all pairs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjacentPairProjection {
    pub positive: bool,
    pub distinct_root_counts: Vec<(usize, usize)>,
    /// Polynomial and full-real index at this exact sample. For a positive
    /// Descartes-one branch the index is descriptive, not a global ordinal.
    pub branch_order: Vec<(usize, usize)>,
    pub coincident_fallback: bool,
    /// Sorted unique polynomial pairs; empty when fallback selects all pairs.
    pub selected_pairs: Vec<(usize, usize)>,
}

/// A separate delineability theorem on x>0. The omitted negative half-line
/// must have one child rejected by this exact original coordinate constraint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositiveRayProjection {
    pub constraint: usize,
    pub coordinate: usize,
    pub rules: Vec<PositiveRayRule>,
    #[serde(default)]
    pub separations: Vec<PositivePairSeparation>,
    #[serde(default)]
    pub pair_witnesses: Vec<PositivePairWitness>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strict_bezout: Vec<PositiveStrictBezout>,
}

/// The complete positive branch ordering of two individually delineable cuts.
/// Every cross-owner edge has one strictly positive rational separator, with
/// both polynomial evaluations retained as nonvanishing ancestor guards.
/// Ordinals count each polynomial's distinct POSITIVE roots independently.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositivePairWitness {
    pub polynomials: [usize; 2],
    pub positive_counts: [usize; 2],
    pub branch_order: Vec<(usize, usize)>,
    pub separators: Vec<String>,
    pub evaluation_signs: Vec<[i8; 2]>,
}

/// For deg(p)>=deg(q), the exact identity
/// R1=lc(q)*p-lc(p)*z^shift*q. Either R1 or one further leading-term
/// cancellation of q and R1 has coefficients of one strict sign on the base.
/// Each remainder is a polynomial combination of p,q, so its nonvanishing
/// on z>0 excludes every common positive root of p,q without any division.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositivePairSeparation {
    pub higher: usize,
    pub lower: usize,
    pub shift: u32,
    pub coefficient_sign: i8,
    #[serde(default = "one_cancellation")]
    pub cancellations: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositiveRayRule {
    pub polynomial: usize,
    pub case: PositiveRayCase,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PositiveRayCase {
    Coordinate,
    Linear {
        leading_sign: i8,
        endpoint_sign: i8,
    },
    /// After orienting the leading coefficient positively, the endpoint is
    /// negative: one simple positive root, with full-real root index one.
    QuadraticOpposite {
        leading_sign: i8,
    },
    /// Oriented a,c>0 and b>0 (or identically zero): no positive roots.
    QuadraticPositive {
        leading_sign: i8,
        linear_zero: bool,
    },
    /// The discriminant is retained. For a*c>0, D>0 prevents b crossing zero,
    /// so the root count and full-real indices follow from the sample sign.
    QuadraticDiscriminant {
        leading_sign: i8,
        discriminant_sign: i8,
        positive_roots: u8,
    },
    /// Every nonzero coefficient has one sign, so Descartes' rule excludes
    /// positive roots without any claim about the number of negative roots.
    DescartesZero {
        coefficient_sign: i8,
    },
    /// Exactly one sign variation counts one positive root with multiplicity,
    /// hence a simple root. Negative collisions do not affect this graph.
    /// Its exported index counts positive roots, not all real roots.
    DescartesOne {
        coefficient_signs: Vec<i8>,
    },
    /// K alternating intervals saturate the Descartes multiplicity bound K.
    /// Thus exactly K simple positive branches persist, regardless of negative
    /// collisions. All coefficient and separator signs are ancestor guards.
    DescartesWitnesses {
        coefficient_signs: Vec<i8>,
        separators: Vec<String>,
        /// Includes p(0), every p(separator), and the leading coefficient.
        evaluation_signs: Vec<i8>,
    },
    /// A complete rational partition of the positive ray has zero or one
    /// Descartes variation in every interval after an exact Möbius transform.
    /// Nonzero endpoint and coefficient guards certify all simple branches.
    DescartesIntervals {
        intervals: Vec<PositiveIntervalWitness>,
    },
    /// Higher degrees retain ordinary lc/discriminant guards plus p(0).
    FullReal,
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
        /// Exact canonical products, not irreducible factorization witnesses.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_univariate_basis: Option<BaseUnivariateBasis>,
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
    /// An entered optional algebra stage failed: never start another backend.
    Failed(String),
    Unsupported(String),
}
type Work<T> = std::result::Result<T, Stop>;
impl From<anyhow::Error> for Stop {
    fn from(error: anyhow::Error) -> Self {
        if error
            .downcast_ref::<crate::resultant_interpolation::Stopped>()
            .is_some()
        {
            Self::Resource(error.to_string())
        } else {
            Self::Unsupported(error.to_string())
        }
    }
}

fn base_stage_stop(error: anyhow::Error) -> Stop {
    if error
        .downcast_ref::<crate::resultant_interpolation::Stopped>()
        .is_some()
    {
        Stop::Resource(error.to_string())
    } else {
        Stop::Failed(format!("base-univariate stage failed: {error}"))
    }
}

#[cfg(test)]
mod stop_tests {
    use super::Stop;

    #[test]
    fn interpolation_resource_stop_never_requests_global_fallback() {
        let error = anyhow::Error::new(crate::resultant_interpolation::Stopped("budget".into()))
            .context("local resultant");
        assert!(matches!(Stop::from(error), Stop::Resource(_)));
        assert!(matches!(
            Stop::from(anyhow::anyhow!("unsupported algebra")),
            Stop::Unsupported(_)
        ));
    }
}

fn highest(p: &Poly) -> Option<usize> {
    (0..p.nvars()).rfind(|&i| p.degree(i) != 0)
}

/// Evaluate only a polynomial whose complete support is in the prefix. A
/// specialization that happens to become constant is not sufficient evidence.
fn prefix_value(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
    crate::algebra::prefix_value::evaluate(p, prefix)
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

/// Classification needs the signed value, while a projection guard needs only
/// the zero set. Cached polynomials may have been multiplied by a negative
/// scalar during canonicalization, so their sign cannot classify the roots.
fn quadratic_discriminant_guard(
    p: &Poly,
    variable: usize,
    leading_value: &Rational,
    linear_value: &Rational,
    constant_value: &Rational,
    operations: &mut impl ProjectionOperations,
) -> Result<(Poly, Rational)> {
    let value = linear_value * linear_value - (leading_value * constant_value) * Rational::from(4);
    let guard = operations.discriminant(p, variable)?;
    ensure!(!guard.is_zero(), "local cut is not square-free");
    Ok((guard, value))
}

#[cfg(test)]
mod quadratic_tests;

/// The verifier never reads discovery or persistent operation caches.
#[derive(Default)]
struct IndependentOperations {
    binary_resultants: bool,
}
impl ProjectionOperations for IndependentOperations {
    fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly> {
        if self.binary_resultants {
            Ok(crate::projection::discriminant_with_binary_statistics(p, variable)?.0)
        } else {
            discriminant(p, variable)
        }
    }
    fn resultant(&mut self, p: &Poly, q: &Poly, variable: usize) -> Result<Poly> {
        exact_resultant(p, q, variable)
    }
}

struct CachedOperations<'a> {
    cache: ProjectionCache<'a>,
    phases: LocalPhaseCosts,
    progress_path: Option<PathBuf>,
    progress_context: Option<LocalReport>,
    last_captured_context: Option<(Option<usize>, Vec<String>)>,
    started: Instant,
}
impl CachedOperations<'_> {
    /// Captured large operations must have a matching prefix even when the
    /// ordinary progress timer has not fired. This is diagnostic metadata;
    /// it changes neither the projection set nor the proof tree. Repeat cache
    /// calls from the same stack need no additional filesystem write.
    fn capture_context(&mut self) -> Result<()> {
        let (Some(path), Some(context)) = (&self.progress_path, &self.progress_context) else {
            return Ok(());
        };
        let key = (context.current_level, context.current_prefix.clone());
        if self.last_captured_context.as_ref() == Some(&key) {
            return Ok(());
        }
        let mut report = context.clone();
        report.cache = self.cache.report.cache.clone();
        report.phases = self.phases.clone();
        report.elapsed_seconds = self.started.elapsed().as_secs_f64();
        write_progress(path, &report)?;
        self.last_captured_context = Some(key);
        Ok(())
    }
    fn factors(&mut self, polynomials: &[Poly]) -> Result<Vec<Poly>> {
        if polynomials.iter().any(|p| p.nterms() >= 64) {
            self.capture_context()?;
        }
        let started = Instant::now();
        let result = self.cache.factor_basis(polynomials);
        self.phases.factor_seconds += started.elapsed().as_secs_f64();
        result
    }
}
impl ProjectionOperations for CachedOperations<'_> {
    fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly> {
        if p.degree(variable) >= 3 || p.nterms() >= 64 {
            self.capture_context()?;
        }
        let started = Instant::now();
        let result = self.cache.discriminant(p, variable);
        self.phases.discriminant_seconds += started.elapsed().as_secs_f64();
        result
    }
    fn resultant(&mut self, p: &Poly, q: &Poly, variable: usize) -> Result<Poly> {
        if p.degree(variable).saturating_add(q.degree(variable)) >= 5
            || p.nterms().saturating_add(q.nterms()) >= 64
        {
            self.capture_context()?;
        }
        let started = Instant::now();
        let result = self.cache.resultant(p, q, variable);
        self.phases.resultant_seconds += started.elapsed().as_secs_f64();
        result
    }
}

fn relevant_roots(mut roots: Vec<FiberRoot>, positive: bool) -> Result<Vec<FiberRoot>> {
    if positive {
        for root in &mut roots {
            ensure!(
                root.fiber.evaluate(&Rational::zero()) != Rational::zero(),
                "positive pair selection has an unguarded zero endpoint"
            );
            while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
                crate::roots::refine_once(root);
            }
        }
        roots.retain(|root| root.interval.0 > Rational::zero());
    }
    Ok(roots)
}

/// Recomputed independently by the checker, including multiplicity-free root
/// counts BEFORE the merged union discards coincident owners. Negative roots
/// are excluded on both sides for a positive-ray proof.
fn adjacent_pair_projection(
    algebra: &Algebra,
    polynomials: &[Poly],
    bearing: &[usize],
    variable: usize,
    prefix: &[Rational],
    positive: bool,
) -> Result<AdjacentPairProjection> {
    let mut fibers = Vec::with_capacity(bearing.len());
    let mut counts = Vec::with_capacity(bearing.len());
    for &id in bearing {
        let fiber = algebra.specialize_univariate(&polynomials[id], variable, prefix)?;
        let roots = relevant_roots(isolate_union(vec![(id, fiber.clone())])?, positive)?;
        ensure!(!roots.is_empty(), "root-bearing cut has no relevant roots");
        counts.push((id, roots.len()));
        fibers.push((id, fiber));
    }
    let union = relevant_roots(isolate_union(fibers)?, positive)?;
    let individual_count: usize = counts.iter().map(|(_, count)| count).sum();
    ensure!(
        union.len() <= individual_count,
        "root union count exceeds its inputs"
    );
    let coincident_fallback = union.len() != individual_count;
    let selected_pairs = if coincident_fallback {
        Vec::new()
    } else {
        union
            .windows(2)
            .filter_map(|roots| {
                let (a, b) = (roots[0].polynomial, roots[1].polynomial);
                (a != b).then_some((a.min(b), a.max(b)))
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    Ok(AdjacentPairProjection {
        positive,
        distinct_root_counts: counts,
        branch_order: union
            .iter()
            .map(|root| (root.polynomial, root.index))
            .collect(),
        coincident_fallback,
        selected_pairs,
    })
}

fn projection_pairs<'a>(
    bearing: &'a [usize],
    selection: Option<&'a AdjacentPairProjection>,
) -> Box<dyn Iterator<Item = (usize, usize)> + 'a> {
    if let Some(selection) = selection.filter(|proof| !proof.coincident_fallback) {
        Box::new(selection.selected_pairs.iter().copied())
    } else {
        Box::new(bearing.iter().enumerate().flat_map(move |(index, &id)| {
            bearing[index + 1..].iter().map(move |&other| (id, other))
        }))
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
    adjacent_pairs: bool,
    selection: &mut Option<AdjacentPairProjection>,
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
        if prefix_value(&lc, prefix)?.is_zero() {
            return Ok((bearing, raw, 0, 0, root_seconds));
        }
        if p.degree(variable) > 1 {
            let disc = operations.discriminant(p, variable)?;
            ensure!(!disc.is_zero(), "local cut is not square-free");
            let vanishing = prefix_value(&disc, prefix)?.is_zero();
            raw.push(disc);
            if vanishing {
                return Ok((bearing, raw, 0, 0, root_seconds));
            }
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
    // With fewer than three bearing polynomials every possible pair is
    // necessary; avoid repeating root isolation when no omission can result.
    if adjacent_pairs && bearing.len() >= 3 {
        let started = Instant::now();
        *selection = Some(adjacent_pair_projection(
            algebra,
            polynomials,
            &bearing,
            variable,
            prefix,
            false,
        )?);
        root_seconds += started.elapsed().as_secs_f64();
    }
    let mut pairs = 0;
    for (id, other) in projection_pairs(&bearing, selection.as_ref()) {
        let p = &polynomials[id];
        let q = &polynomials[other];
        let r = operations.resultant(p, q, variable)?;
        ensure!(
            !r.is_zero(),
            "distinct irreducible cuts have zero resultant"
        );
        let vanishing = prefix_value(&r, prefix)?.is_zero();
        raw.push(r);
        pairs += 1;
        if vanishing {
            break;
        }
    }
    let all_pairs = cuts.len().saturating_mul(cuts.len().saturating_sub(1)) / 2;
    let bearing_pairs = bearing
        .len()
        .saturating_mul(bearing.len().saturating_sub(1))
        / 2;
    Ok((bearing, raw, pairs, all_pairs - bearing_pairs, root_seconds))
}

/// No specialization may hide parameter dependence here: every polynomial
/// must already lie in Q[x_0], and the base prefix must be empty.
fn check_base_point_cuts(
    polynomials: &[Poly],
    cuts: &[usize],
    level: usize,
    prefix: &[Rational],
) -> Result<()> {
    ensure!(
        level == 0 && prefix.is_empty(),
        "base-point projection has an ancestor"
    );
    for &id in cuts {
        let p = polynomials
            .get(id)
            .ok_or_else(|| anyhow!("invalid base-point cut"))?;
        ensure!(
            highest(p) == Some(0),
            "base-point cut is not an exact nonconstant univariate polynomial"
        );
    }
    Ok(())
}

fn positive_coordinate(normalized: &NormalizedProblem, variable: usize) -> Option<usize> {
    normalized.positives.iter().position(|p| {
        p.nterms() == 1
            && p.degree(variable) == 1
            && p.exponents_iter().all(|e| {
                e.iter()
                    .enumerate()
                    .all(|(i, &degree)| degree == u32::from(i == variable))
            })
            && p.coefficients[0] > Rational::zero()
    })
}

/// Isolate exactly the boundary 0 and the positive roots. Internal full-real
/// ordinals are retained; `local_bounds` exports positive ordinals when needed.
fn stack_roots(
    algebra: &Algebra,
    polynomials: &[Poly],
    cuts: &[usize],
    level: usize,
    prefix: &[Rational],
    ray: Option<&PositiveRayProjection>,
    bearing: &[usize],
) -> Result<Vec<FiberRoot>> {
    let fibers = cuts
        .iter()
        .copied()
        .filter(|id| ray.is_none_or(|r| *id == r.coordinate || bearing.contains(id)))
        .map(|id| {
            Ok((
                id,
                algebra.specialize_univariate(&polynomials[id], level, prefix)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut roots = isolate_union(fibers)?;
    if let Some(ray) = ray {
        let zero = roots
            .iter()
            .position(|r| r.polynomial == ray.coordinate)
            .ok_or_else(|| anyhow!("positive-ray stack omitted its zero boundary"))?;
        ensure!(
            roots[zero].interval == (Rational::zero(), Rational::zero()),
            "coordinate root is not zero"
        );
        roots.drain(..zero);
        ensure!(
            roots
                .iter()
                .skip(1)
                .all(|r| r.interval.0 > Rational::zero()),
            "positive-ray roots were not separated from zero"
        );
    }
    Ok(roots)
}

fn local_bounds(
    roots: &[FiberRoot],
    ray: Option<&PositiveRayProjection>,
) -> Result<Vec<RootBound>> {
    let positive_ids: HashMap<_, _> = ray
        .into_iter()
        .flat_map(|r| &r.rules)
        .filter_map(|r| match &r.case {
            PositiveRayCase::DescartesOne { .. } => Some((r.polynomial, 1)),
            PositiveRayCase::DescartesWitnesses { separators, .. } => {
                Some((r.polynomial, separators.len() + 1))
            }
            PositiveRayCase::DescartesIntervals { intervals } => {
                Some((r.polynomial, interval_descartes::positive_count(intervals)))
            }
            _ => None,
        })
        .collect();
    roots
        .iter()
        .map(|root| {
            let mut output = bound(root);
            if let Some(&count) = positive_ids.get(&root.polynomial) {
                ensure!(
                    root.interval.0 > Rational::zero(),
                    "positive root index requires a strictly positive root"
                );
                output.index_domain = crate::output::RootIndexDomain::Positive;
                // Other polynomials can own coincident union roots. Counting
                // only this polynomial's surviving representatives would then
                // give the wrong ordinal for its later roots.
                output.index = if count == 1 {
                    0
                } else {
                    crate::roots::roots_in_open_interval(
                        &root.fiber,
                        &Rational::zero(),
                        &root.interval.0,
                    ) + usize::from(
                        root.interval.0 != root.interval.1
                            && root.fiber.evaluate(&root.interval.0).is_zero(),
                    )
                };
                ensure!(
                    output.index < count,
                    "positive ordinal exceeds Descartes witness"
                );
            }
            Ok(output)
        })
        .collect()
}

/// Generic positive-ray delineability. All coefficient signs used here are
/// returned as ancestor obligations; a sample alone is never a certificate.
/// Quadratic negative-root collisions are harmless only in the explicitly
/// rootless case. The one-variation case exports a positive-root ordinal;
/// full-real boundary indices remain invariant in the remaining cases.
fn positive_projection_raw(
    normalized: &NormalizedProblem,
    polynomials: &[Poly],
    cuts: &[usize],
    variable: usize,
    prefix: &[Rational],
    constraint: usize,
    coordinate: usize,
    pair_separation: bool,
    pair_cancellations: usize,
    pair_witnesses: bool,
    mut pair_witness_stock: Option<&mut pair_witnesses::WitnessStock>,
    strict_bezout: bool,
    bezout_registry: &mut strict_bezout::Registry,
    descartes_one: bool,
    descartes_witnesses: bool,
    descartes_witness_max_terms: usize,
    mut witness_stock: Option<&mut descartes::WitnessStock>,
    descartes_intervals: bool,
    mut interval_stock: Option<&mut interval_descartes::WitnessStock>,
    replay: Option<&PositiveRayProjection>,
    adjacent_pairs: bool,
    selection: &mut Option<AdjacentPairProjection>,
    pair_seconds: &mut f64,
    operations: &mut impl ProjectionOperations,
) -> Result<(
    Vec<usize>,
    Vec<Poly>,
    usize,
    usize,
    f64,
    PositiveRayProjection,
)> {
    let algebra = &normalized.algebra;
    ensure!(
        cuts.contains(&coordinate),
        "positive coordinate factor is not protected"
    );
    let mut raw = Vec::new();
    let mut bearing = Vec::new();
    let mut rules = Vec::new();
    let mut root_seconds = 0.0;
    for &id in cuts {
        let p = polynomials
            .get(id)
            .ok_or_else(|| anyhow!("invalid cut id"))?;
        ensure!(
            highest(p) == Some(variable),
            "cut has the wrong highest variable"
        );
        if id == coordinate {
            rules.push(PositiveRayRule {
                polynomial: id,
                case: PositiveRayCase::Coordinate,
            });
            continue;
        }
        let a = p.univariate_lcoeff(variable);
        let c = p.replace(variable, &Rational::zero());
        ensure!(
            !c.is_zero(),
            "a non-coordinate irreducible cut has a zero endpoint polynomial"
        );
        raw.extend([a.clone(), c.clone()]);
        let av = prefix_value(&a, prefix)?;
        let cv = prefix_value(&c, prefix)?;
        // Return the endpoint/degree-drop obstruction without expensive work.
        if av.is_zero() || cv.is_zero() {
            break;
        }
        let leading_sign = sign(&av);
        let endpoint_sign = sign(&cv);
        let same_sign = leading_sign == endpoint_sign;
        let case = match p.degree(variable) {
            1 => {
                if !same_sign {
                    bearing.push(id);
                }
                PositiveRayCase::Linear {
                    leading_sign,
                    endpoint_sign,
                }
            }
            2 if !same_sign => {
                bearing.push(id);
                PositiveRayCase::QuadraticOpposite { leading_sign }
            }
            2 => {
                let b = p.derivative(variable).replace(variable, &Rational::zero());
                let bv = prefix_value(&b, prefix)?;
                if b.is_zero() || sign(&bv) == leading_sign {
                    if !b.is_zero() {
                        raw.push(b);
                    }
                    PositiveRayCase::QuadraticPositive {
                        leading_sign,
                        linear_zero: bv.is_zero(),
                    }
                } else {
                    let (d, dv) =
                        quadratic_discriminant_guard(p, variable, &av, &bv, &cv, operations)?;
                    raw.push(d);
                    if dv.is_zero() {
                        break;
                    }
                    let positive_roots = if dv > Rational::zero() { 2 } else { 0 };
                    if positive_roots != 0 {
                        bearing.push(id);
                    }
                    PositiveRayCase::QuadraticDiscriminant {
                        leading_sign,
                        discriminant_sign: sign(&dv),
                        positive_roots,
                    }
                }
            }
            _ => {
                let supplied = replay
                    .and_then(|ray| ray.rules.iter().find(|rule| rule.polynomial == id))
                    .map(|rule| &rule.case);
                let force_full = (descartes_witnesses || descartes_intervals)
                    && matches!(supplied, Some(PositiveRayCase::FullReal));
                let coefficients = p.to_univariate_polynomial_list(variable);
                let values = coefficients
                    .iter()
                    .map(|(coefficient, _)| prefix_value(coefficient, prefix))
                    .collect::<Result<Vec<_>>>()?;
                if values.iter().any(|v| v.is_zero()) {
                    raw.extend(coefficients.into_iter().map(|(coefficient, _)| coefficient));
                    break;
                }
                if !force_full && values.iter().all(|v| sign(v) == leading_sign) {
                    raw.extend(coefficients.into_iter().map(|(coefficient, _)| coefficient));
                    rules.push(PositiveRayRule {
                        polynomial: id,
                        case: PositiveRayCase::DescartesZero {
                            coefficient_sign: leading_sign,
                        },
                    });
                    continue;
                }
                let signs: Vec<_> = values.iter().map(sign).collect();
                if !force_full
                    && descartes_one
                    && signs.windows(2).filter(|w| w[0] != w[1]).count() == 1
                {
                    raw.extend(coefficients.into_iter().map(|(coefficient, _)| coefficient));
                    bearing.push(id);
                    rules.push(PositiveRayRule {
                        polynomial: id,
                        case: PositiveRayCase::DescartesOne {
                            coefficient_signs: signs,
                        },
                    });
                    continue;
                }
                if descartes_witnesses {
                    let started = Instant::now();
                    let attempt = if replay.is_some() {
                        if let Some(case @ PositiveRayCase::DescartesWitnesses { .. }) = supplied {
                            Some((
                                case.clone(),
                                descartes::verify_witness(
                                    p,
                                    variable,
                                    prefix,
                                    &signs,
                                    case,
                                    descartes_witness_max_terms,
                                )?,
                            ))
                        } else {
                            // A stronger full-real certificate is allowed even
                            // when a new separator could have been discovered.
                            None
                        }
                    } else {
                        witness_stock
                            .as_deref_mut()
                            .ok_or_else(|| anyhow!("missing finite Descartes witness stock"))?
                            .attempt(
                                id,
                                algebra,
                                p,
                                variable,
                                prefix,
                                &signs,
                                descartes_witness_max_terms,
                            )?
                    };
                    root_seconds += started.elapsed().as_secs_f64();
                    if let Some((case, guards)) = attempt {
                        raw.extend(coefficients.into_iter().map(|(coefficient, _)| coefficient));
                        raw.extend(guards);
                        bearing.push(id);
                        rules.push(PositiveRayRule {
                            polynomial: id,
                            case,
                        });
                        continue;
                    }
                }
                if descartes_intervals {
                    let started = Instant::now();
                    let attempt = if replay.is_some() {
                        if let Some(case @ PositiveRayCase::DescartesIntervals { .. }) = supplied {
                            Some((
                                case.clone(),
                                interval_descartes::verify_witness(p, variable, prefix, case)?,
                            ))
                        } else {
                            None
                        }
                    } else {
                        interval_stock
                            .as_deref_mut()
                            .ok_or_else(|| anyhow!("missing finite interval Descartes stock"))?
                            .attempt(id, algebra, p, variable, prefix)?
                    };
                    root_seconds += started.elapsed().as_secs_f64();
                    if let Some((case, guards)) = attempt {
                        let PositiveRayCase::DescartesIntervals { intervals } = &case else {
                            unreachable!()
                        };
                        if interval_descartes::positive_count(intervals) != 0 {
                            bearing.push(id);
                        }
                        raw.extend(guards);
                        rules.push(PositiveRayRule {
                            polynomial: id,
                            case,
                        });
                        continue;
                    }
                }
                let d = operations.discriminant(p, variable)?;
                ensure!(!d.is_zero(), "local cut is not square-free");
                raw.push(d);
                if raw
                    .last()
                    .is_some_and(|d| prefix_value(d, prefix).is_ok_and(|v| v.is_zero()))
                {
                    break;
                }
                let started = Instant::now();
                let fiber = algebra.specialize_univariate(p, variable, prefix)?;
                let mut roots = isolate_union(vec![(id, fiber)])?;
                let mut positive = false;
                for root in &mut roots {
                    while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero()
                    {
                        // p(0) is nonzero, so exact refinement eventually separates.
                        crate::roots::refine_once(root);
                    }
                    positive |= root.interval.0 > Rational::zero();
                }
                if positive {
                    bearing.push(id);
                }
                root_seconds += started.elapsed().as_secs_f64();
                PositiveRayCase::FullReal
            }
        };
        rules.push(PositiveRayRule {
            polynomial: id,
            case,
        });
    }
    let mut ray = PositiveRayProjection {
        constraint,
        coordinate,
        rules,
        separations: Vec::new(),
        pair_witnesses: Vec::new(),
        strict_bezout: Vec::new(),
    };
    if raw
        .iter()
        .any(|p| prefix_value(p, prefix).is_ok_and(|v| v.is_zero()))
    {
        return Ok((bearing, raw, 0, 0, root_seconds, ray));
    }
    if adjacent_pairs && bearing.len() >= 3 {
        let started = Instant::now();
        *selection = Some(adjacent_pair_projection(
            algebra,
            polynomials,
            &bearing,
            variable,
            prefix,
            true,
        )?);
        root_seconds += started.elapsed().as_secs_f64();
    }
    let mut pairs = 0;
    for (id, other) in projection_pairs(&bearing, selection.as_ref()) {
        if pair_separation {
            let started = Instant::now();
            let attempt = separate_positive_pair(
                polynomials,
                id,
                other,
                variable,
                prefix,
                pair_cancellations,
            );
            *pair_seconds += started.elapsed().as_secs_f64();
            if let Some((proof, coefficients)) = attempt? {
                raw.extend(coefficients);
                ray.separations.push(proof);
                continue;
            }
        }
        if strict_bezout {
            let started = Instant::now();
            let pair = [id.min(other), id.max(other)];
            let attempt = if let Some(stored) = replay {
                if let Some(proof) = stored.strict_bezout.iter().find(|p| p.polynomials == pair) {
                    Some((
                        proof.clone(),
                        bezout_registry.verify(
                            normalized,
                            polynomials,
                            pair,
                            variable,
                            prefix,
                            proof,
                        )?,
                    ))
                } else {
                    // Ordinary resultants are always an admissible strengthening.
                    None
                }
            } else {
                bezout_registry.attempt(polynomials, pair, variable, prefix)?
            };
            *pair_seconds += started.elapsed().as_secs_f64();
            if let Some((proof, guards)) = attempt {
                raw.extend(guards);
                ray.strict_bezout.push(proof);
                continue;
            }
        }
        if pair_witnesses {
            let started = Instant::now();
            let pair = [id.min(other), id.max(other)];
            let attempt = if let Some(stored) = replay {
                if let Some(proof) = stored
                    .pair_witnesses
                    .iter()
                    .find(|proof| proof.polynomials == pair)
                {
                    Some((
                        proof.clone(),
                        pair_witnesses::verify_witness(
                            algebra,
                            polynomials,
                            pair,
                            variable,
                            prefix,
                            proof,
                        )?,
                    ))
                } else {
                    // A stored ordinary resultant is an admissible stronger
                    // certificate even if a new rational template is available.
                    None
                }
            } else {
                pair_witness_stock
                    .as_deref_mut()
                    .ok_or_else(|| anyhow!("missing finite pair witness stock"))?
                    .attempt(algebra, polynomials, pair, variable, prefix)?
            };
            *pair_seconds += started.elapsed().as_secs_f64();
            if let Some((proof, guards)) = attempt {
                raw.extend(guards);
                ray.pair_witnesses.push(proof);
                continue;
            }
        }
        let r = operations.resultant(&polynomials[id], &polynomials[other], variable)?;
        ensure!(
            !r.is_zero(),
            "distinct irreducible cuts have zero resultant"
        );
        let vanishing = prefix_value(&r, prefix)?.is_zero();
        raw.push(r);
        pairs += 1;
        if vanishing {
            break;
        }
    }
    let all_pairs = cuts.len().saturating_mul(cuts.len().saturating_sub(1)) / 2;
    let rootless = all_pairs
        - bearing
            .len()
            .saturating_mul(bearing.len().saturating_sub(1))
            / 2;
    Ok((bearing, raw, pairs, rootless, root_seconds, ray))
}

/// Bounded sufficient condition, never a completeness assumption. Inconclusive
/// coefficient signs (including a nonconstant zero at the prefix) use the
/// ordinary resultant without introducing an unnecessary coefficient seam.
fn separate_positive_pair(
    polynomials: &[Poly],
    first: usize,
    second: usize,
    variable: usize,
    prefix: &[Rational],
    max_cancellations: usize,
) -> Result<Option<(PositivePairSeparation, Vec<Poly>)>> {
    ensure!(
        (1..=2).contains(&max_cancellations),
        "positive pair cancellations must be one or two"
    );
    let (higher, lower) =
        if polynomials[first].degree(variable) >= polynomials[second].degree(variable) {
            (first, second)
        } else {
            (second, first)
        };
    let p = &polynomials[higher];
    let q = &polynomials[lower];
    // Linear pairs already have an inexpensive exact resultant. Limit both
    // temporary products by their monomial-pair count before multiplying.
    if p.degree(variable) <= 1 {
        return Ok(None);
    }
    let Some(remainder) = pair_cancellation(p, q, variable, 4096) else {
        return Ok(None);
    };
    let shift = p.degree(variable) - q.degree(variable);
    if remainder.is_zero() {
        return Ok(None);
    }
    ensure!(
        remainder.degree(variable) < p.degree(variable),
        "pseudo-remainder failed to lower degree"
    );
    if let Some((coefficient_sign, coefficients)) =
        same_sign_coefficients(&remainder, variable, prefix)?
    {
        return Ok(Some((
            PositivePairSeparation {
                higher,
                lower,
                shift,
                coefficient_sign,
                cancellations: 1,
            },
            coefficients,
        )));
    }
    if max_cancellations == 1 || remainder.degree(variable) == 0 {
        return Ok(None);
    }
    // The first remainder may itself have positive roots. One further
    // polynomial combination can nevertheless have uniformly signed
    // coefficients. Gate that expansion with exact rational specialization;
    // this only selects an experiment, and never replaces symbolic evidence.
    let (next_high, next_low) = if q.degree(variable) >= remainder.degree(variable) {
        (q, &remainder)
    } else {
        (&remainder, q)
    };
    if !numeric_cancellation_has_one_sign(next_high, next_low, variable, prefix)? {
        return Ok(None);
    }
    let Some(second_remainder) = pair_cancellation(next_high, next_low, variable, 16_384) else {
        return Ok(None);
    };
    // Do not use this bounded shortcut to compute a full constant PRS endpoint.
    if second_remainder.is_zero() || second_remainder.degree(variable) == 0 {
        return Ok(None);
    }
    ensure!(
        second_remainder.degree(variable) < next_high.degree(variable),
        "second pair cancellation failed to lower degree"
    );
    let Some((coefficient_sign, coefficients)) =
        same_sign_coefficients(&second_remainder, variable, prefix)?
    else {
        return Ok(None);
    };
    Ok(Some((
        PositivePairSeparation {
            higher,
            lower,
            shift,
            coefficient_sign,
            cancellations: 2,
        },
        coefficients,
    )))
}

/// Leading-term cancellation without division or primitive normalization.
/// Both multiplication work and exponent sums are bounded before expansion.
fn pair_cancellation(
    high: &Poly,
    low: &Poly,
    variable: usize,
    max_products: usize,
) -> Option<Poly> {
    debug_assert!(high.degree(variable) >= low.degree(variable));
    let a = high.univariate_lcoeff(variable);
    let b = low.univariate_lcoeff(variable);
    let estimate = high
        .nterms()
        .saturating_mul(b.nterms())
        .saturating_add(low.nterms().saturating_mul(a.nterms()));
    if estimate > max_products
        || (0..high.nvars())
            .any(|i| high.degree(i) as u64 + low.degree(i) as u64 > crate::algebra::MAX_DEGREE)
    {
        return None;
    }
    let mut exponents = vec![0; high.nvars()];
    exponents[variable] = high.degree(variable) - low.degree(variable);
    Some(&b * high - &a * &low.clone().mul_monomial(&Rational::one(), &exponents))
}

fn same_sign_coefficients(
    polynomial: &Poly,
    variable: usize,
    prefix: &[Rational],
) -> Result<Option<(i8, Vec<Poly>)>> {
    let coefficients = polynomial
        .to_univariate_polynomial_list(variable)
        .into_iter()
        .map(|(coefficient, _)| coefficient)
        .collect::<Vec<_>>();
    let values = coefficients
        .iter()
        .map(|c| prefix_value(c, prefix))
        .collect::<Result<Vec<_>>>()?;
    let coefficient_sign = sign(&values[0]);
    if coefficient_sign == 0 || values.iter().any(|v| sign(v) != coefficient_sign) {
        return Ok(None);
    }
    Ok(Some((coefficient_sign, coefficients)))
}

fn numeric_cancellation_has_one_sign(
    high: &Poly,
    low: &Poly,
    variable: usize,
    prefix: &[Rational],
) -> Result<bool> {
    let dense_values = |p: &Poly| -> Result<Vec<Rational>> {
        let mut values = vec![Rational::zero(); p.degree(variable) as usize + 1];
        for (coefficient, power) in p.to_univariate_polynomial_list(variable) {
            values[power as usize] = prefix_value(&coefficient, prefix)?;
        }
        Ok(values)
    };
    let mut a = dense_values(high)?;
    let b = dense_values(low)?;
    let high_leading = a.last().unwrap().clone();
    let low_leading = b.last().unwrap();
    let shift = a.len() - b.len();
    for value in &mut a {
        *value = &*value * low_leading;
    }
    for (power, value) in b.iter().enumerate() {
        a[power + shift] = &a[power + shift] - &(&high_leading * value);
    }
    let mut signs = a.iter().filter(|v| !v.is_zero()).map(sign);
    let Some(first) = signs.next() else {
        return Ok(false);
    };
    Ok(a.iter().skip(1).any(|v| !v.is_zero()) && signs.all(|next| next == first))
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
    /// Successful conditional proofs, scoped to this immutable problem/map.
    completed_subtrees: subtree_cache::CompletedSubtrees,
    /// Incomplete discoveries are only hints; every seeded stack is rebuilt.
    learned_cuts: learned_cuts::LearnedCuts,
    hint_archive: Option<hint_archive::HintArchive>,
    descartes_stock: descartes::WitnessStock,
    interval_stock: interval_descartes::WitnessStock,
    pair_witness_stock: pair_witnesses::WitnessStock,
    bezout_registry: strict_bezout::Registry,
    obstruction_observer: Option<obstruction_observer::Observer>,
    base_sources: base_univariate::SourceIndex,
    base_records: Vec<BaseUnivariateSource>,
    base_typed: BTreeSet<usize>,
    base_retained_bits: u64,
}
#[derive(Serialize, Deserialize)]
struct Built {
    node: LocalNode,
    required: BTreeSet<usize>,
    cells: usize,
}

fn write_progress(path: &std::path::Path, report: &LocalReport) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("local-{}.tmp", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec(report)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

impl Builder<'_> {
    fn snapshot(&self) -> LocalReport {
        let mut report = self.report.clone();
        report.cache = self.operations.cache.report.cache.clone();
        report.phases = self.operations.phases.clone();
        report.elapsed_seconds = self.started.elapsed().as_secs_f64();
        self.completed_subtrees.report(&mut report);
        self.learned_cuts.report(&mut report);
        report.hint_archive = self
            .hint_archive
            .as_ref()
            .map(|archive| archive.report.clone());
        self.descartes_stock.report(&mut report);
        self.interval_stock.report(&mut report);
        self.pair_witness_stock.report(&mut report);
        report.obstruction_observer = self
            .obstruction_observer
            .as_ref()
            .map(obstruction_observer::Observer::snapshot);
        report
    }
    fn publish(&mut self, force: bool) -> Result<()> {
        if let Some(archive) = &mut self.hint_archive {
            let terminal = matches!(
                self.report.active_phase.as_str(),
                "incomplete" | "complete" | "computation_failure"
            );
            archive.snapshot(
                self.learned_cuts.snapshots(),
                &self.polynomials,
                &self.normalized.algebra,
                terminal,
            );
        }
        if self.progress_path.is_none() && self.obstruction_observer.is_none() {
            return Ok(());
        }
        if !force && self.last_progress.elapsed().as_secs_f64() < 0.5 {
            return Ok(());
        }
        if let Some(observer) = &mut self.obstruction_observer {
            observer.publish(matches!(
                self.report.active_phase.as_str(),
                "incomplete" | "complete" | "computation_failure"
            ));
        }
        if let Some(path) = &self.progress_path {
            write_progress(path, &self.snapshot())?;
        }
        // A periodic snapshot may describe a different small stack than the
        // last captured operation, so the next large operation must refresh.
        self.operations.last_captured_context = None;
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
        if let Some(inventory) = &mut self.report.search_inventory {
            inventory.observe(&polynomial);
        }
        self.ids.insert(polynomial.clone(), id);
        self.polynomials.push(polynomial);
        id
    }
    /// Metadata-only snapshots before potentially blocking optional algebra or
    /// its ordinary fallback. A failed diagnostic write cannot alter the solve.
    fn publish_base_progress(&mut self) {
        if self.progress_path.is_none() && self.operations.progress_path.is_none() {
            return;
        }
        let snapshot = self.snapshot();
        if self.operations.progress_path.is_some() {
            // A subsequent factor capture must not replace this snapshot with
            // the older context from the start of the projection call.
            self.operations.progress_context = Some(snapshot.clone());
            // Keep the original same-prefix write/skip decision. Forcing a new
            // capture could turn a best-effort write error into an algebra stop.
        }
        if let Some(path) = &self.progress_path {
            let _ = write_progress(path, &snapshot);
        }
    }
    fn observed_base_stop(&mut self, stop: Stop) -> Stop {
        let report = self.report.base_univariate_basis.as_mut().unwrap();
        match &stop {
            Stop::Resource(_) => {
                report.post_entry_resource_stops += 1;
                report.phase("resource_stop");
            }
            _ => {
                report.post_entry_failures += 1;
                report.phase("failure");
            }
        }
        self.publish_base_progress();
        stop
    }
    fn prepare_base_text(&mut self, source: &Poly) -> Work<Option<String>> {
        let text = base_univariate::source_text(&self.normalized.algebra, source)
            .map_err(base_stage_stop)?;
        let report = self.report.base_univariate_basis.as_mut().unwrap();
        if let Some(text) = &text {
            report.peak_source_text_bytes = report.peak_source_text_bytes.max(text.len());
        } else {
            report.decline(BaseUnivariateDecline::SourceText);
            self.report.active_phase = "base_univariate_fallback".into();
            self.publish_base_progress();
        }
        Ok(text)
    }
    /// Only projection-generated guards enter this separate support registry.
    /// Initial inputs and the ordinary factor cache keep their old contracts.
    fn guard_basis(&mut self, raw: &[Poly]) -> Work<BTreeSet<usize>> {
        if !self.options.base_univariate_basis {
            let factors = self.operations.factors(raw)?;
            return Ok(factors.into_iter().map(|p| self.intern(p)).collect());
        }
        let mut result = BTreeSet::new();
        for p in raw {
            self.check_budget()?;
            self.report
                .base_univariate_basis
                .as_mut()
                .unwrap()
                .begin(p, self.base_retained_bits);
            let map_ok = base_univariate::map_admissible(&self.normalized.algebra)
                && p.variables() == &self.normalized.algebra.variables;
            let eligible = map_ok && base_univariate::source_cost(p).is_some();
            let source = eligible.then(|| canonical(p));
            if let Some(source) = &source {
                if let Some(ids) = self.base_sources.get(source) {
                    result.extend(ids);
                    let report = self.report.base_univariate_basis.as_mut().unwrap();
                    report.hits += 1;
                    report.phase("hit");
                    continue;
                }
            }
            let cost = source.as_ref().and_then(base_univariate::source_cost);
            if let (Some(source), Some(bits)) = (&source, cost) {
                self.report
                    .base_univariate_basis
                    .as_mut()
                    .unwrap()
                    .source(source, bits);
            }
            if cost.is_none()
                || self.base_records.len() >= base_univariate::MAX_SOURCES
                || cost.is_some_and(|bits| {
                    self.base_retained_bits.saturating_add(bits)
                        > base_univariate::MAX_RETAINED_BITS
                })
            {
                let reason = if !map_ok {
                    BaseUnivariateDecline::MapOrSupport
                } else if cost.is_none() {
                    base_univariate::source_decline(source.as_ref().unwrap_or(p))
                } else if self.base_records.len() >= base_univariate::MAX_SOURCES {
                    BaseUnivariateDecline::SourceCount
                } else {
                    BaseUnivariateDecline::RegistryStorage
                };
                self.report
                    .base_univariate_basis
                    .as_mut()
                    .unwrap()
                    .decline(reason);
                self.report.active_phase = "base_univariate_fallback".into();
                self.publish_base_progress();
                for factor in self.operations.factors(std::slice::from_ref(p))? {
                    result.insert(self.intern(factor));
                }
                continue;
            }
            let source = source.unwrap();
            let Some(text) = self.prepare_base_text(&source)? else {
                for factor in self.operations.factors(std::slice::from_ref(p))? {
                    result.insert(self.intern(factor));
                }
                continue;
            };
            let mut proposed_storage = self
                .report
                .base_univariate_basis
                .as_ref()
                .unwrap()
                .retained_storage;
            proposed_storage.add_source(&source, text.len());
            if let Some(reason) = proposed_storage.decline() {
                self.report
                    .base_univariate_basis
                    .as_mut()
                    .unwrap()
                    .decline(reason);
                self.report.active_phase = "base_univariate_fallback".into();
                self.publish_base_progress();
                for factor in self.operations.factors(std::slice::from_ref(p))? {
                    result.insert(self.intern(factor));
                }
                continue;
            }
            self.report.active_phase = "base_univariate_decomposition".into();
            self.report
                .base_univariate_basis
                .as_mut()
                .unwrap()
                .phase("decomposition");
            self.publish(true)?;
            let mut timings = BaseUnivariateReport::default();
            let started = self.started;
            let limit = self.problem.limits.wall_time_secs;
            let previous = self.report.base_univariate_basis.as_ref().unwrap();
            let old_decomposition = previous.decomposition_seconds;
            let old_reconstruction = previous.reconstruction_seconds;
            let old_cache = previous.proposal_cache.clone();
            let cache_directory = self
                .options
                .cache_enabled
                .then(|| self.problem.solver.checkpoint_dir.clone())
                .flatten();
            let normalized = self.normalized;
            let blocks = base_univariate::discover_cached_observed(
                &source,
                cache_directory
                    .as_deref()
                    .map(|directory| (&normalized.algebra, text.as_str(), directory)),
                &mut timings,
                || {
                    if started.elapsed().as_secs_f64() >= limit {
                        return Err(crate::resultant_interpolation::Stopped(
                            "base support wall time limit".into(),
                        )
                        .into());
                    }
                    Ok(())
                },
                |timings, block_bits, phase| {
                    let report = self.report.base_univariate_basis.as_mut().unwrap();
                    report.decomposition_seconds =
                        old_decomposition + timings.decomposition_seconds;
                    report.reconstruction_seconds =
                        old_reconstruction + timings.reconstruction_seconds;
                    report.proposal_cache = old_cache.clone();
                    report.proposal_cache.accumulate(&timings.proposal_cache);
                    report.phase(phase);
                    if let Some(bits) = block_bits {
                        report.proposal(bits);
                    }
                    self.report.active_phase = format!("base_univariate_{phase}");
                    self.publish_base_progress();
                },
            );
            let report = self.report.base_univariate_basis.as_mut().unwrap();
            report.decomposition_seconds = old_decomposition + timings.decomposition_seconds;
            report.reconstruction_seconds = old_reconstruction + timings.reconstruction_seconds;
            report.proposal_cache = old_cache;
            report.proposal_cache.accumulate(&timings.proposal_cache);
            let blocks = blocks.map_err(|error| self.observed_base_stop(base_stage_stop(error)))?;
            for (block, _) in &blocks {
                proposed_storage.add_block(block, self.normalized.algebra.names[0].len());
            }
            proposed_storage
                .check()
                .map_err(|error| self.observed_base_stop(base_stage_stop(error)))?;
            let retained_cost = blocks
                .iter()
                .try_fold(cost.unwrap(), |sum, (p, _)| {
                    sum.checked_add(base_univariate::cost(p)?)
                })
                .ok_or_else(|| {
                    self.observed_base_stop(Stop::Resource(
                        "base support storage bound overflow".into(),
                    ))
                })?;
            if self.base_retained_bits.saturating_add(retained_cost)
                > base_univariate::MAX_RETAINED_BITS
            {
                return Err(self.observed_base_stop(Stop::Resource(
                    "base support retained source/block cap".into(),
                )));
            }
            // Primitive normalization keeps a signed nonzero rational unit.
            let scale = p.lcoeff() / source.lcoeff();
            if &source * &source.constant(scale) != *p {
                return Err(self.observed_base_stop(Stop::Failed(
                    "base support signed source normalization failed".into(),
                )));
            }
            self.check_budget()
                .map_err(|stop| self.observed_base_stop(stop))?;
            let mut references = Vec::new();
            for (block, multiplicity) in blocks {
                let id = self.intern(block);
                self.base_typed.insert(id);
                result.insert(id);
                references.push(BaseUnivariateBlock {
                    polynomial: id,
                    multiplicity,
                });
            }
            references.sort_by_key(|b| b.polynomial);
            let ids = references.iter().map(|b| b.polynomial).collect();
            self.base_sources.insert(source.clone(), ids);
            self.base_retained_bits += retained_cost;
            self.base_records.push(BaseUnivariateSource {
                polynomial: text,
                blocks: references,
            });
            let report = self.report.base_univariate_basis.as_mut().unwrap();
            report.sources = self.base_records.len();
            report.blocks = self.base_typed.len();
            report.retained_source_bits += cost.unwrap();
            report.retained_block_bits += retained_cost - cost.unwrap();
            report.retained_total_bits = self.base_retained_bits;
            report.retained_storage = proposed_storage;
            report.phase("admitted");
            self.report.active_phase = "base_univariate_admitted".into();
            self.publish_base_progress();
            self.check_budget()
                .map_err(|stop| self.observed_base_stop(stop))?;
        }
        Ok(result)
    }
    fn seed_archive(&mut self, prefix: &[Rational], cuts: &mut BTreeSet<usize>) {
        let Some(mut archive) = self.hint_archive.take() else {
            return;
        };
        let before = cuts.len();
        for candidate in archive.candidates(prefix) {
            if !archive.budget_available() {
                break;
            }
            match candidate {
                hint_archive::Candidate::Known(id) => {
                    if highest(&self.polynomials[id]) == Some(prefix.len()) {
                        cuts.insert(id);
                    }
                }
                hint_archive::Candidate::New(hash, portable) => {
                    let started = Instant::now();
                    let polynomial = portable.reconstruct(&self.normalized.algebra, prefix.len());
                    archive.report.construction_seconds += started.elapsed().as_secs_f64();
                    let id = match polynomial {
                        Ok(polynomial) if archive.budget_available() => {
                            let started = Instant::now();
                            let factors =
                                self.operations.factors(std::slice::from_ref(&polynomial));
                            archive.report.factor_seconds += started.elapsed().as_secs_f64();
                            match factors {
                                Ok(factors)
                                    if factors.as_slice() == std::slice::from_ref(&polynomial) =>
                                {
                                    Some(self.intern(polynomial))
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    if let Some(id) = id {
                        cuts.insert(id);
                    }
                    archive.validated(hash, id);
                }
            }
        }
        archive.report.seeded_cuts += cuts.len() - before;
        self.hint_archive = Some(archive);
        if cuts.len() > before {
            self.learned_cuts.remember(prefix, cuts);
        }
    }
    fn observe_archive(&mut self, prefix: &[Rational], cuts: &BTreeSet<usize>) {
        if let Some(archive) = &mut self.hint_archive {
            archive.observe(prefix, cuts);
        }
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
            &self.problem.solver.certificate_options(),
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
        self.report.current_prefix = prefix.iter().map(ToString::to_string).collect();
        self.report.active_phase = "local_projection".into();
        self.publish(false)?;
        self.operations.progress_context = Some(self.snapshot());
        if level == 0 && self.options.base_point_projection {
            check_base_point_cuts(&self.polynomials, cuts, level, prefix)?;
            self.report.base_point_stacks += 1;
            self.report.base_point_max_cuts = self.report.base_point_max_cuts.max(cuts.len());
            self.report.base_point_candidate_pairs_skipped +=
                cuts.len().saturating_mul(cuts.len().saturating_sub(1)) / 2;
            // stack_roots with no ray proof uses every cut and full-real
            // ordinals. Ordinary constraint checks reject negative sectors.
            return Ok(LocalProjectionCertificate {
                base_point: true,
                root_bearing: vec![],
                guards: vec![],
                removals: vec![],
                positive_ray: None,
                adjacent_pairs: None,
            });
        }
        let mut pair_seconds = 0.0;
        let mut adjacent_pairs = None;
        let ray_constraint = self
            .options
            .positive_ray
            .then(|| positive_coordinate(self.normalized, level))
            .flatten();
        let (root_bearing, raw, pairs, omitted, root_seconds, positive_ray) =
            if let Some(constraint) = ray_constraint {
                let coordinate = *self.input_factors[constraint]
                    .first()
                    .ok_or_else(|| anyhow!("missing coordinate factor"))?;
                let (bearing, raw, pairs, omitted, seconds, ray) = positive_projection_raw(
                    self.normalized,
                    &self.polynomials,
                    cuts,
                    level,
                    prefix,
                    constraint,
                    coordinate,
                    self.options.positive_pair_separation,
                    self.options.positive_pair_cancellations,
                    self.options.positive_pair_witnesses,
                    Some(&mut self.pair_witness_stock),
                    self.options.strict_bezout,
                    &mut self.bezout_registry,
                    self.options.descartes_one,
                    self.options.descartes_witnesses,
                    self.options.descartes_witness_max_terms,
                    Some(&mut self.descartes_stock),
                    self.options.descartes_intervals,
                    Some(&mut self.interval_stock),
                    None,
                    self.options.adjacent_pairs,
                    &mut adjacent_pairs,
                    &mut pair_seconds,
                    &mut self.operations,
                )?;
                if self.options.positive_pair_separation {
                    self.report.pair_separation_candidates += adjacent_pairs
                        .as_ref()
                        .filter(|proof| !proof.coincident_fallback)
                        .map_or_else(
                            || {
                                bearing
                                    .len()
                                    .saturating_mul(bearing.len().saturating_sub(1))
                                    / 2
                            },
                            |proof| proof.selected_pairs.len(),
                        );
                    self.report.pair_resultants_omitted += ray.separations.len();
                    self.report.second_cancellation_pairs_omitted += ray
                        .separations
                        .iter()
                        .filter(|proof| proof.cancellations == 2)
                        .count();
                }
                self.report.positive_ray_stacks += 1;
                self.report.pair_witness_resultants_omitted += ray.pair_witnesses.len();
                self.report.strict_bezout_resultants_omitted += ray.strict_bezout.len();
                if self.options.strict_bezout {
                    self.report.strict_bezout_resultant_fallbacks += pairs;
                }
                self.report.quadratic_discriminants_omitted += ray
                    .rules
                    .iter()
                    .filter(|r| {
                        matches!(
                            r.case,
                            PositiveRayCase::QuadraticOpposite { .. }
                                | PositiveRayCase::QuadraticPositive { .. }
                        )
                    })
                    .count();
                self.report.descartes_discriminants_omitted += ray
                    .rules
                    .iter()
                    .filter(|r| matches!(r.case, PositiveRayCase::DescartesZero { .. }))
                    .count();
                self.report.descartes_one_discriminants_omitted += ray
                    .rules
                    .iter()
                    .filter(|r| matches!(r.case, PositiveRayCase::DescartesOne { .. }))
                    .count();
                self.report.descartes_witness_discriminants_omitted += ray
                    .rules
                    .iter()
                    .filter(|r| matches!(r.case, PositiveRayCase::DescartesWitnesses { .. }))
                    .count();
                self.report.descartes_interval_discriminants_omitted += ray
                    .rules
                    .iter()
                    .filter(|r| matches!(r.case, PositiveRayCase::DescartesIntervals { .. }))
                    .count();
                (bearing, raw, pairs, omitted, seconds, Some(ray))
            } else {
                let (bearing, raw, pairs, omitted, seconds) = local_projection_raw(
                    &self.normalized.algebra,
                    &self.polynomials,
                    cuts,
                    level,
                    prefix,
                    self.options.adjacent_pairs,
                    &mut adjacent_pairs,
                    &mut self.operations,
                )?;
                (bearing, raw, pairs, omitted, seconds, None)
            };
        self.operations.phases.root_classification_seconds += root_seconds;
        self.operations.phases.pair_separation_seconds += pair_seconds;
        self.report.projected_pairs += pairs;
        self.report.omitted_rootless_pairs += omitted;
        if let Some(selection) = &adjacent_pairs {
            self.report.adjacent_pair_stacks += 1;
            if selection.coincident_fallback {
                self.report.coincident_pair_fallbacks += 1;
            } else {
                let all = root_bearing
                    .len()
                    .saturating_mul(root_bearing.len().saturating_sub(1))
                    / 2;
                self.report.nonadjacent_pairs_omitted += all - selection.selected_pairs.len();
            }
        }
        let mut guards = self.guard_basis(&raw)?;
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
            base_point: false,
            root_bearing,
            guards: guards.into_iter().collect(),
            removals,
            positive_ray,
            adjacent_pairs,
        })
    }
    fn visit(&mut self, prefix: &mut Vec<Rational>) -> Work<Built> {
        // No key creation, hashing or diagnostic work on the disabled path.
        if self.obstruction_observer.is_none() {
            return self.visit_inner(prefix);
        }
        let result = self.visit_inner(prefix);
        // Children restore the caller's exact prefix before returning. Observe
        // each failed visit, including propagation, without consuming/changing
        // the guard or attributing duplicated descendant work to that unwind.
        if let Err(Stop::Guard(id)) = &result {
            if let Some(polynomial) = self.polynomials.get(*id).filter(|p| !p.is_zero()) {
                if let Some(axis) = highest(polynomial) {
                    self.obstruction_observer
                        .as_mut()
                        .expect("enabled observer")
                        .observe(prefix, *id, axis);
                }
            }
        }
        result
    }
    fn visit_inner(&mut self, prefix: &mut Vec<Rational>) -> Work<Built> {
        self.report.speculative_nodes += 1;
        self.check_budget()?;
        if let Some(leaf) = self.leaf(prefix)? {
            return Ok(leaf);
        }
        if self.options.reuse_completed_subtrees {
            if let Some(built) = self.completed_subtrees.get(prefix)? {
                // The key includes every exact rational coordinate. The tree
                // remains a conditional proof: return all its obligations so
                // changed ancestor cuts must still discharge them. No proof
                // is reused at a different sample or across Builder instances.
                self.check_budget()?;
                return Ok(built);
            }
        }
        let level = prefix.len();
        let mut cuts = self.input_levels[level].clone();
        if self.options.reuse_learned_cuts {
            self.learned_cuts.seed(prefix, &mut cuts);
        }
        self.seed_archive(prefix, &mut cuts);
        self.observe_archive(prefix, &cuts);
        if self.options.defer_projection {
            if let Some(built) = self.visit_deferred(prefix, &mut cuts)? {
                if self.options.reuse_completed_subtrees {
                    self.completed_subtrees.insert(prefix, &built);
                }
                return Ok(built);
            }
        }
        // The ancestor prefix is restored after every child and remains fixed
        // throughout this loop. Pool hints never escape this visit or carry a
        // child proof. Every selected sample goes through the ordinary visitor.
        let mut sample_pool = self
            .options
            .stable_sector_samples
            .then(stack_samples::StackSamples::new);
        loop {
            self.check_budget()?;
            let cut_list: Vec<_> = cuts.iter().copied().collect();
            let projection = self.project(&cut_list, level, prefix)?;
            self.report.active_phase = "root_isolation".into();
            self.publish(level == 0 && cut_list.len() > 64)?;
            let root_started = Instant::now();
            let typed_base = cut_list.iter().any(|id| self.base_typed.contains(id));
            let roots = stack_roots(
                &self.normalized.algebra,
                &self.polynomials,
                &cut_list,
                level,
                prefix,
                projection.positive_ray.as_ref(),
                &projection.root_bearing,
            )
            .map_err(|e| {
                if typed_base {
                    self.observed_base_stop(base_stage_stop(e))
                } else {
                    Stop::from(e)
                }
            })?;
            if typed_base {
                self.report
                    .base_univariate_basis
                    .as_mut()
                    .unwrap()
                    .root_union_seconds += root_started.elapsed().as_secs_f64();
            }
            self.operations.phases.root_isolation_seconds += root_started.elapsed().as_secs_f64();
            self.publish(false)?;
            let mut required = self.projection_requirements(&projection);
            let mut sectors = Vec::new();
            let mut cells = 0usize;
            let mut additions = BTreeSet::new();
            let mut discovery_siblings = 0usize;
            for i in 0..=roots.len() {
                self.check_budget()?;
                if !additions.is_empty() {
                    discovery_siblings += 1;
                    self.report.discovery_siblings += 1;
                }
                let lower = if i > 0 { roots.get(i - 1) } else { None };
                let upper = roots.get(i);
                let retained = sample_pool
                    .as_ref()
                    .and_then(|pool| pool.choose(lower, upper));
                if sample_pool.is_some() {
                    self.report.stable_sample_hits += usize::from(retained.is_some());
                    self.report.stable_sample_misses += usize::from(retained.is_none());
                }
                let sample = retained.unwrap_or_else(|| sector_sample(lower, upper));
                prefix.push(sample.clone());
                let child = if i == 0 {
                    if let Some(ray) = &projection.positive_ray {
                        Ok(Built {
                            node: LocalNode::Rejected {
                                constraint: ray.constraint,
                            },
                            required: self.input_factors[ray.constraint].iter().copied().collect(),
                            cells: 0,
                        })
                    } else {
                        self.visit(prefix)
                    }
                } else {
                    self.visit(prefix)
                };
                prefix.pop();
                if !(i == 0 && projection.positive_ray.is_some()) {
                    if let Some(pool) = &mut sample_pool {
                        let (recorded, evictions) = pool.remember(&sample);
                        self.report.stable_sample_recorded += usize::from(recorded);
                        self.report.stable_sample_evictions += evictions;
                        self.report.stable_sample_peak_bytes =
                            self.report.stable_sample_peak_bytes.max(pool.bytes());
                    }
                }
                match child {
                    Err(Stop::Guard(id)) if highest(&self.polynomials[id]) == Some(level) => {
                        if cuts.contains(&id) {
                            return Err(Stop::Unsupported(
                                "nongeneric sample remained on an existing stack cut".into(),
                            ));
                        }
                        additions.insert(id);
                    }
                    Err(stop) => {
                        if self.options.reuse_learned_cuts && !additions.is_empty() {
                            cuts.extend(additions);
                            self.learned_cuts.remember(prefix, &cuts);
                            self.observe_archive(prefix, &cuts);
                        }
                        return Err(stop);
                    }
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
                        if additions.is_empty() {
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
                    // These sectors cannot be emitted: their boundary set has
                    // changed. Discard them now while collecting a bounded
                    // number of additional same-axis obligations. Earlier-axis
                    // guards and all failures still return immediately above.
                    sectors.clear();
                    if !self.options.batch_sibling_discovery
                        || additions.len() >= 8
                        || discovery_siblings >= 32
                    {
                        break;
                    }
                }
            }
            if !additions.is_empty() {
                self.report.refinement_cuts += additions.len();
                self.report.multi_cut_refinements += usize::from(additions.len() > 1);
                self.report.max_refinement_cuts =
                    self.report.max_refinement_cuts.max(additions.len());
                let before = cuts.len();
                cuts.extend(additions);
                if cuts.len() == before {
                    return Err(Stop::Unsupported(
                        "nongeneric sample remained on an existing stack cut".into(),
                    ));
                }
                if self.options.reuse_learned_cuts {
                    self.learned_cuts.remember(prefix, &cuts);
                }
                self.observe_archive(prefix, &cuts);
                self.report.refinements += 1;
                continue;
            }
            self.guard_nonzero(&required, prefix)?;
            let root_bounds = local_bounds(&roots, projection.positive_ray.as_ref())?;
            let built = Built {
                node: LocalNode::Stack {
                    cuts: cut_list,
                    projection,
                    roots: root_bounds,
                    sectors,
                    required_below: required.iter().copied().collect(),
                },
                required,
                cells,
            };
            if self.options.reuse_completed_subtrees {
                self.completed_subtrees.insert(prefix, &built);
            }
            return Ok(built);
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
    crate::config::validate_descartes_witness_max_terms(options.descartes_witness_max_terms)?;
    ensure!(
        (1..=2).contains(&options.positive_pair_cancellations),
        "positive pair cancellations must be one or two"
    );
    ensure!(
        (2..=8).contains(&options.descartes_interval_template_limit),
        "interval template limit must be between two and eight"
    );
    ensure!(
        options.subtree_cache_mib <= 65536 && options.learned_cut_cache_mib <= 65536,
        "local cache budget exceeds memory ceiling"
    );
    ensure!(
        !options.defer_projection || options.stable_sector_samples,
        "deferred projection requires stable sector samples"
    );
    ensure!(
        !options.base_univariate_basis || options.base_point_projection,
        "base-univariate support requires base-point projection"
    );
    ensure!(
        !options.base_univariate_basis || !problem.solver.local_hint_archive,
        "base-univariate support does not permit durable hint archives in v1"
    );
    let started = Instant::now();
    let order = match selected_order(problem) {
        Ok(order) => order,
        Err(error) => return fallback(problem, options, started, error.to_string()),
    };
    let normalized = normalize(problem, &order)?;
    let cache_options = OptimizedProjectionOptions {
        binary_resultants: problem.solver.binary_resultants,
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
    let mut report = LocalReport::default();
    if options.base_univariate_basis {
        report.base_univariate_basis = Some(BaseUnivariateReport::default());
    }
    if problem.solver.search_local_progress {
        report.search_inventory = Some(crate::search::local_progress::LocalInventory::new());
    }
    let bezout_registry = if options.strict_bezout {
        strict_bezout::Registry::from_hints(
            &normalized,
            &problem.solver.local_strict_bezout_hints,
            &mut report,
            started,
            problem.limits.wall_time_secs,
        )
    } else {
        strict_bezout::Registry::default()
    };
    if started.elapsed().as_secs_f64() >= problem.limits.wall_time_secs {
        return Ok(unfinished(
            problem,
            "wall time limit during local preparation".into(),
            started,
            Some(report),
        ));
    }
    let obstruction_path = std::env::var_os("SYMGCAD_OBSTRUCTION_REPORT").map(PathBuf::from);
    let obstruction_observer = (options.observe_obstructions || obstruction_path.is_some())
        .then(|| obstruction_observer::Observer::new(obstruction_path));
    let mut builder = Builder {
        problem,
        normalized: &normalized,
        options,
        started,
        polynomials: Vec::new(),
        ids: HashMap::new(),
        input_factors: Vec::new(),
        input_levels: vec![BTreeSet::new(); order.len()],
        report,
        operations: CachedOperations {
            cache: ProjectionCache::new(&normalized.algebra, &cache_options),
            phases: LocalPhaseCosts::default(),
            progress_path: cache_options
                .capture_dir
                .as_ref()
                .and_then(|_| std::env::var_os("SYMGCAD_PROGRESS_PATH"))
                .map(PathBuf::from),
            progress_context: None,
            last_captured_context: None,
            started,
        },
        progress_path: std::env::var_os("SYMGCAD_PROGRESS_PATH").map(PathBuf::from),
        last_progress: started,
        guard_domains: (0..order.len())
            .map(|level| guard_domain(&normalized, level))
            .collect::<Result<_>>()?,
        protected: BTreeSet::new(),
        guard_certificates: Vec::new(),
        guard_certificate_index: HashMap::new(),
        completed_subtrees: subtree_cache::CompletedSubtrees::new(
            ((problem.limits.memory_mib as usize).saturating_mul(1024 * 1024) / 16)
                .min(options.subtree_cache_mib.saturating_mul(1024 * 1024)),
        ),
        learned_cuts: learned_cuts::LearnedCuts::new(
            ((problem.limits.memory_mib as usize).saturating_mul(1024 * 1024) / 128)
                .min(options.learned_cut_cache_mib.saturating_mul(1024 * 1024)),
        ),
        hint_archive: hint_archive::HintArchive::new(
            problem,
            &normalized,
            // A diagnostic flag must not alter durable hint context or reuse.
            &LocalOptions {
                observe_obstructions: false,
                ..options.clone()
            },
            started,
        ),
        descartes_stock: descartes::WitnessStock::default(),
        interval_stock: interval_descartes::WitnessStock::new(
            options.descartes_interval_template_limit,
        )?,
        pair_witness_stock: pair_witnesses::WitnessStock::default(),
        bezout_registry,
        obstruction_observer,
        base_sources: base_univariate::SourceIndex::default(),
        base_records: Vec::new(),
        base_typed: BTreeSet::new(),
        base_retained_bits: 0,
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
        Err(Stop::Failed(reason)) => {
            builder.report.active_phase = "computation_failure".into();
            builder.publish(true)?;
            let mut result = unfinished(problem, reason, started, Some(builder.snapshot()));
            result.solution.status = Status::ComputationFailure;
            return Ok(result);
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
    builder.report.current_prefix.clear();
    builder.report.active_phase = "complete".into();
    builder.publish(true)?;
    if started.elapsed().as_secs_f64() >= problem.limits.wall_time_secs {
        return Ok(unfinished(
            problem,
            "wall time limit during local checkpoint publication".into(),
            started,
            Some(builder.snapshot()),
        ));
    }
    let report = builder.snapshot();
    statistics.projection_seconds = report.phases.factor_seconds
        + report.phases.discriminant_seconds
        + report.phases.resultant_seconds
        + report.phases.domain_certificate_seconds
        + report.phases.pair_separation_seconds
        + report.strict_bezout_preparation_seconds;
    if let Some(base) = &report.base_univariate_basis {
        statistics.projection_seconds += base.decomposition_seconds + base.reconstruction_seconds;
    }
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
        sign_filter: None,
        linear_negative: None,
        bilinear_positive: None,
        quadratic_ladder: None,
    };
    Ok(LocalSolveResult {
        solution,
        evidence: LocalEvidence::Local {
            options: options.clone(),
            input_factors: builder.input_factors,
            guard_certificates: builder.guard_certificates,
            base_univariate_basis: (!builder.base_records.is_empty()).then_some(
                BaseUnivariateBasis {
                    schema_version: base_univariate::SCHEMA,
                    sources: builder.base_records,
                },
            ),
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
    positive_ray: bool,
    positive_pair_separation: bool,
    positive_pair_cancellations: usize,
    positive_pair_witnesses: bool,
    strict_bezout: bool,
    bezout_registry: std::cell::RefCell<strict_bezout::Registry>,
    stable_sector_samples: bool,
    descartes_one: bool,
    descartes_witnesses: bool,
    descartes_witness_max_terms: usize,
    descartes_intervals: bool,
    base_point_projection: bool,
    adjacent_pairs: bool,
    binary_discriminants: bool,
    base_basis: base_univariate::Validated,
}
impl Checker<'_> {
    fn guard_basis(&self, raw: &[Poly]) -> Result<BTreeSet<usize>> {
        let mut result = BTreeSet::new();
        if self.base_basis.sources.is_empty() {
            for q in self.normalized.algebra.factors_checked(raw)? {
                result.insert(
                    *self
                        .ids
                        .get(&q)
                        .ok_or_else(|| anyhow!("missing local projection guard factor"))?,
                );
            }
            return Ok(result);
        }
        for p in raw {
            if let Some(ids) = self.base_basis.proportional_ids(p) {
                result.extend(ids);
                continue;
            }
            for q in self
                .normalized
                .algebra
                .factors_checked(std::slice::from_ref(p))?
            {
                result.insert(
                    *self
                        .ids
                        .get(&q)
                        .ok_or_else(|| anyhow!("missing local projection guard factor"))?,
                );
            }
        }
        Ok(result)
    }
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
                ensure!(
                    !cuts.iter().any(|id| self.base_basis.typed.contains(id))
                        || (level == 0 && prefix.is_empty() && projection.base_point),
                    "typed base support used outside the empty-prefix base-point stack"
                );
                let mut adjacent_pairs = None;
                let (bearing, raw) = if projection.base_point {
                    ensure!(
                        self.base_point_projection,
                        "base-point projection is disabled"
                    );
                    check_base_point_cuts(self.polynomials, cuts, level, prefix)?;
                    ensure!(
                        projection.positive_ray.is_none()
                            && projection.root_bearing.is_empty()
                            && projection.guards.is_empty()
                            && projection.removals.is_empty(),
                        "base-point projection carries incompatible conditional evidence"
                    );
                    (vec![], vec![])
                } else if let Some(ray) = &projection.positive_ray {
                    ensure!(self.positive_ray, "positive-ray projection is disabled");
                    ensure!(
                        positive_coordinate(self.normalized, level) == Some(ray.constraint),
                        "positive-ray premise is not the original positive coordinate constraint"
                    );
                    ensure!(
                        self.input_factors[ray.constraint] == vec![ray.coordinate],
                        "positive-ray coordinate factor is incorrect"
                    );
                    let (bearing, raw, _, _, _, expected) = positive_projection_raw(
                        self.normalized,
                        self.polynomials,
                        cuts,
                        level,
                        prefix,
                        ray.constraint,
                        ray.coordinate,
                        self.positive_pair_separation,
                        self.positive_pair_cancellations,
                        self.positive_pair_witnesses,
                        None,
                        self.strict_bezout,
                        &mut self.bezout_registry.borrow_mut(),
                        self.descartes_one,
                        self.descartes_witnesses,
                        self.descartes_witness_max_terms,
                        None,
                        self.descartes_intervals,
                        None,
                        Some(ray),
                        self.adjacent_pairs,
                        &mut adjacent_pairs,
                        &mut 0.0,
                        &mut IndependentOperations {
                            binary_resultants: self.binary_discriminants,
                        },
                    )?;
                    ensure!(
                        *ray == expected,
                        "positive-ray delineability witness is incorrect"
                    );
                    (bearing, raw)
                } else {
                    let (bearing, raw, _, _, _) = local_projection_raw(
                        &self.normalized.algebra,
                        self.polynomials,
                        cuts,
                        level,
                        prefix,
                        self.adjacent_pairs,
                        &mut adjacent_pairs,
                        &mut IndependentOperations {
                            binary_resultants: self.binary_discriminants,
                        },
                    )?;
                    (bearing, raw)
                };
                ensure!(
                    projection.adjacent_pairs == adjacent_pairs,
                    "incorrect adjacent-root pair projection witness"
                );
                ensure!(
                    projection.root_bearing == bearing,
                    "incorrect root-bearing cut subset"
                );
                let projected = self.guard_basis(&raw)?;
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
                let actual_roots = stack_roots(
                    &self.normalized.algebra,
                    self.polynomials,
                    cuts,
                    level,
                    prefix,
                    projection.positive_ray.as_ref(),
                    &projection.root_bearing,
                )?;
                let expected_roots = local_bounds(&actual_roots, projection.positive_ray.as_ref())?;
                ensure!(
                    roots == &expected_roots,
                    "incomplete or changed local root union"
                );
                ensure!(
                    sectors.len() == actual_roots.len() + 1,
                    "local stack omits an open sector"
                );
                for (i, sector) in sectors.iter().enumerate() {
                    if i == 0 {
                        if let Some(ray) = &projection.positive_ray {
                            ensure!(
                                matches!(sector.child, LocalNode::Rejected { constraint } if constraint == ray.constraint),
                                "negative half-line must be rejected by its positive coordinate premise"
                            );
                        }
                    }
                    let lower = if i > 0 { actual_roots.get(i - 1) } else { None };
                    let upper = actual_roots.get(i);
                    let expected = if self.stable_sector_samples {
                        let sample = crate::algebra::rational(&sector.sample)?;
                        ensure!(
                            stack_samples::strictly_inside(&sample, lower, upper),
                            "local sample is not strictly inside its certified sector"
                        );
                        sample
                    } else {
                        let expected = sector_sample(lower, upper);
                        ensure!(
                            sector.sample == expected.to_string(),
                            "local sample is not the certified sector sample"
                        );
                        expected
                    };
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
    ensure!(
        solution.linear_negative.is_none() && solution.bilinear_positive.is_none(),
        "incompatible additional fiber reduction evidence"
    );
    let (input_factors, guard_certificates, base_evidence, root, options) = match evidence {
        LocalEvidence::GlobalFallback { .. } => return crate::verify::verify_nonlocal(solution),
        LocalEvidence::NotApplicable { .. } => anyhow::bail!("local projection was not applicable"),
        LocalEvidence::Unfinished { .. } => anyhow::bail!("unfinished local projection evidence"),
        LocalEvidence::Local {
            input_factors,
            guard_certificates,
            base_univariate_basis,
            root,
            options,
            ..
        } => (
            input_factors,
            guard_certificates,
            base_univariate_basis.as_ref(),
            root,
            options,
        ),
    };
    solution.problem.validate()?;
    crate::config::validate_descartes_witness_max_terms(options.descartes_witness_max_terms)?;
    ensure!(
        (1..=2).contains(&options.positive_pair_cancellations),
        "positive pair cancellations must be one or two"
    );
    ensure!(
        (2..=8).contains(&options.descartes_interval_template_limit),
        "interval template limit must be between two and eight"
    );
    ensure!(solution.schema_version == 1, "unsupported output schema");
    ensure!(
        !options.base_univariate_basis || options.base_point_projection,
        "base support requires base-point projection"
    );
    ensure!(
        !options.base_univariate_basis || !solution.problem.solver.local_hint_archive,
        "base support does not permit durable hint archives in v1"
    );
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
    let base_basis = base_univariate::validate(
        &normalized.algebra,
        &polynomials,
        base_evidence,
        options.base_univariate_basis,
    )?;
    let protected: BTreeSet<_> = input_factors.iter().flatten().copied().collect();
    for (id, p) in polynomials.iter().enumerate() {
        ensure!(
            !p.is_constant() && canonical(p) == *p,
            "noncanonical or constant local polynomial"
        );
        ensure!(
            ids.insert(p.clone(), id).is_none(),
            "duplicate local polynomial"
        );
        if !base_basis.typed.contains(&id) || protected.contains(&id) {
            ensure!(
                normalized
                    .algebra
                    .factors_checked(std::slice::from_ref(p))?
                    == vec![p.clone()],
                "local cut is not a primitive irreducible factor"
            );
        }
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
        let expected = normalized
            .algebra
            .factors_checked(std::slice::from_ref(p))?
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
        base_basis,
        binary_discriminants: solution.problem.solver.binary_resultants,
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
        positive_ray: options.positive_ray,
        positive_pair_separation: options.positive_pair_separation,
        positive_pair_cancellations: options.positive_pair_cancellations,
        positive_pair_witnesses: options.positive_pair_witnesses,
        strict_bezout: options.strict_bezout,
        bezout_registry: std::cell::RefCell::new(strict_bezout::Registry::default()),
        stable_sector_samples: options.stable_sector_samples,
        descartes_one: options.descartes_one,
        descartes_witnesses: options.descartes_witnesses,
        descartes_witness_max_terms: options.descartes_witness_max_terms,
        descartes_intervals: options.descartes_intervals,
        base_point_projection: options.base_point_projection,
        adjacent_pairs: options.adjacent_pairs,
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

#[cfg(test)]
mod pair_separation_tests {
    use super::*;

    #[test]
    fn zero_remainder_mixed_signs_and_zero_prefix_coefficients_fall_back() {
        let algebra = Algebra::new(&["x".into(), "z".into()]).unwrap();
        for (left, right, x) in [
            ("(x+1)*(z^2+1)", "z^2+1", 1),
            ("z^2-x", "z^2+z-x-1", 2),
            ("z^2-1", "z^2-x*z-2", 0),
        ] {
            let polynomials = vec![algebra.parse(left).unwrap(), algebra.parse(right).unwrap()];
            assert!(
                separate_positive_pair(&polynomials, 0, 1, 1, &[Rational::from(x)], 1)
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn excessive_monomial_product_estimate_skips_separation_before_multiplication() {
        let mut names = (0..70).map(|i| format!("x{i}")).collect::<Vec<_>>();
        names.push("z".into());
        let algebra = Algebra::new(&names).unwrap();
        let a = names[..70].join("+");
        let b = names[..70]
            .iter()
            .map(|name| format!("{name}^2"))
            .collect::<Vec<_>>()
            .join("+");
        let polynomials = vec![
            algebra.parse(&format!("({a})*z^2-1")).unwrap(),
            algebra.parse(&format!("({b})*z^2-2")).unwrap(),
        ];
        assert!(
            separate_positive_pair(&polynomials, 0, 1, 70, &vec![Rational::one(); 70], 2)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn second_cancellation_can_separate_when_the_first_remainder_has_a_positive_root() {
        let algebra = Algebra::new(&["x".into(), "z".into()]).unwrap();
        for (p, q) in [
            ("z^2-x", "z^2-2*z-1"),
            ("-z^2+x", "z^2-2*z-1"),
            ("z^2-x", "-z^2+2*z+1"),
            ("z^3-x*z-1", "z^2-2*z-1"),
        ] {
            let polynomials = vec![algebra.parse(p).unwrap(), algebra.parse(q).unwrap()];
            assert!(
                separate_positive_pair(&polynomials, 0, 1, 1, &[Rational::from(2)], 1)
                    .unwrap()
                    .is_none()
            );
            let (proof, coefficients) =
                separate_positive_pair(&polynomials, 0, 1, 1, &[Rational::from(2)], 2)
                    .unwrap()
                    .unwrap();
            assert_eq!(proof.cancellations, 2);
            assert!(coefficients.iter().all(|p| {
                sign(&prefix_value(p, &[Rational::from(2)]).unwrap()) == proof.coefficient_sign
            }));
            assert!(coefficients.iter().any(|p| p.degree(0) > 0));
        }
    }

    #[test]
    fn second_cancellation_rejects_zero_or_mixed_coefficients_and_bounds_products() {
        let algebra = Algebra::new(&["x".into(), "z".into()]).unwrap();
        let polynomials = vec![
            algebra.parse("z^2-x").unwrap(),
            algebra.parse("z^2-2*z-1").unwrap(),
        ];
        // At x=5 the z coefficient is zero; at x=6 it has the opposite
        // sign from the constant. Neither may be certified from the sample.
        for x in [5, 6] {
            assert!(
                separate_positive_pair(&polynomials, 0, 1, 1, &[Rational::from(x)], 2)
                    .unwrap()
                    .is_none()
            );
        }
        assert!(pair_cancellation(&polynomials[0], &polynomials[1], 1, 1).is_none());
        assert!(separate_positive_pair(&polynomials, 0, 1, 1, &[Rational::from(2)], 3).is_err());
    }
}

#[cfg(test)]
mod adjacent_pair_tests {
    use super::*;

    #[test]
    fn every_branch_participates_in_the_exact_adjacency_order() {
        let algebra = Algebra::new(&["s".into(), "t".into(), "z".into()]).unwrap();
        let polys = ["z^2+s*z+t", "z+1", "z-1"].map(|p| algebra.parse(p).unwrap());
        let proof = adjacent_pair_projection(
            &algebra,
            &polys,
            &[0, 1, 2],
            2,
            &[Rational::zero(), Rational::from(-4)],
            false,
        )
        .unwrap();
        assert_eq!(proof.branch_order, vec![(0, 0), (1, 0), (2, 0), (0, 1)]);
        assert_eq!(proof.selected_pairs, vec![(0, 1), (0, 2), (1, 2)]);
        assert!(!proof.coincident_fallback);
    }

    #[test]
    fn coincident_relevant_roots_fall_back_but_negative_coincidences_do_not() {
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let polys = ["z-a", "z+a", "z-2"].map(|p| algebra.parse(p).unwrap());
        let proof =
            adjacent_pair_projection(&algebra, &polys, &[0, 1, 2], 1, &[Rational::zero()], false)
                .unwrap();
        assert!(proof.coincident_fallback);
        assert_eq!(
            proof
                .distinct_root_counts
                .iter()
                .map(|(_, n)| n)
                .sum::<usize>(),
            3
        );
        assert_eq!(proof.branch_order.len(), 2);
        assert_eq!(projection_pairs(&[0, 1, 2], Some(&proof)).count(), 3);
        let polys = ["z^2-a", "z^2-z-a-1"].map(|p| algebra.parse(p).unwrap());
        let positive =
            adjacent_pair_projection(&algebra, &polys, &[0, 1], 1, &[Rational::one()], true)
                .unwrap();
        assert!(!positive.coincident_fallback);
        assert_eq!(positive.distinct_root_counts, vec![(0, 1), (1, 1)]);
        assert_eq!(positive.branch_order, vec![(0, 1), (1, 1)]);
        let full =
            adjacent_pair_projection(&algebra, &polys, &[0, 1], 1, &[Rational::one()], false)
                .unwrap();
        assert!(full.coincident_fallback);
    }

    #[test]
    fn vanishing_guards_abort_before_unusable_pair_work() {
        struct Counting {
            pairs: usize,
            discriminants: usize,
        }
        impl ProjectionOperations for Counting {
            fn discriminant(&mut self, p: &Poly, variable: usize) -> Result<Poly> {
                self.discriminants += 1;
                discriminant(p, variable)
            }
            fn resultant(&mut self, p: &Poly, q: &Poly, variable: usize) -> Result<Poly> {
                self.pairs += 1;
                exact_resultant(p, q, variable)
            }
        }
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let mut operations = Counting {
            pairs: 0,
            discriminants: 0,
        };
        for text in ["a*z^2+1", "z^2-a"] {
            let polys = [text, "z^3+a*z+1"].map(|p| algebra.parse(p).unwrap());
            let mut selection = None;
            let (_, raw, _, _, _) = local_projection_raw(
                &algebra,
                &polys,
                &[0, 1],
                1,
                &[Rational::zero()],
                true,
                &mut selection,
                &mut operations,
            )
            .unwrap();
            assert!(selection.is_none());
            assert!(
                raw.iter()
                    .any(|p| prefix_value(p, &[Rational::zero()]).unwrap().is_zero())
            );
        }
        assert_eq!(operations.pairs, 0);
        assert_eq!(operations.discriminants, 1);
        let polys = ["z-a", "z+a", "z-2"].map(|p| algebra.parse(p).unwrap());
        let mut selection = None;
        let (_, raw, _, _, _) = local_projection_raw(
            &algebra,
            &polys,
            &[0, 1, 2],
            1,
            &[Rational::zero()],
            true,
            &mut selection,
            &mut operations,
        )
        .unwrap();
        assert!(selection.unwrap().coincident_fallback);
        assert_eq!(operations.pairs, 1);
        assert!(
            raw.iter()
                .any(|p| prefix_value(p, &[Rational::zero()]).unwrap().is_zero())
        );
    }
}
