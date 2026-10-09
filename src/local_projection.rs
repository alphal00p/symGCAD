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
//!
//! With an original positive coordinate bound, a separate positive-ray theorem
//! retains the endpoint and leading coefficient of each cut. Quadratics use
//! exact coefficient signs to omit discriminants governing only negative roots;
//! other cases retain the ordinary discriminant. Pair resultants are then
//! needed only between positive-root-bearing cuts. One explicit rejected
//! negative half-line and every positive sector prove exhaustive coverage.
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
            positive_pair_cancellations: 2,
            batch_sibling_discovery: true,
            reuse_completed_subtrees: true,
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
    pub speculative_nodes: usize,
    pub refinements: usize,
    /// New stack cuts retained across all speculative refinement rounds.
    pub refinement_cuts: usize,
    /// Refinement rounds which discovered more than one new cut.
    pub multi_cut_refinements: usize,
    /// Tentative siblings visited after another sibling invalidated the stack.
    pub discovery_siblings: usize,
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
    pub pair_separation_candidates: usize,
    pub pair_resultants_omitted: usize,
    pub second_cancellation_pairs_omitted: usize,
    pub subtree_cache_hits: usize,
    pub subtree_cache_misses: usize,
    pub subtree_cache_evictions: usize,
    pub subtree_cache_retained_bytes: usize,
    pub subtree_cache_peak_bytes: usize,
    pub subtree_cache_retained_nodes: usize,
    pub subtree_cache_peak_nodes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalProjectionCertificate {
    /// Cuts with real roots, or strictly positive roots for a positive-ray proof.
    pub root_bearing: Vec<usize>,
    /// Square-free irreducible factors of all required lc, disc and resultants.
    pub guards: Vec<usize>,
    /// References into LocalEvidence::Local.guard_certificates. Their bound
    /// factors replace the removed guards as explicit ancestor obligations.
    #[serde(default)]
    pub removals: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positive_ray: Option<PositiveRayProjection>,
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
    // The complete-support check above is essential: replace_all evaluates
    // the supplied coordinates directly and does not require unused trailing
    // coordinates. Avoid rebuilding a polynomial after each substitution.
    Ok(p.replace_all(prefix))
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

/// Isolate exactly the boundary 0 and the positive roots. Full-real root
/// ordinals are retained by isolating before discarding negative roots.
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

/// Generic positive-ray delineability. All coefficient signs used here are
/// returned as ancestor obligations; a sample alone is never a certificate.
/// Quadratic negative-root collisions are harmless only in the explicitly
/// rootless case. Full-real boundary indices are invariant in every other case.
fn positive_projection_raw(
    algebra: &Algebra,
    polynomials: &[Poly],
    cuts: &[usize],
    variable: usize,
    prefix: &[Rational],
    constraint: usize,
    coordinate: usize,
    pair_separation: bool,
    pair_cancellations: usize,
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
                    // Preserve the signed discriminant: operation caches store
                    // canonical zero-set representatives and discard its sign.
                    let d = &b * &b - (&a * &c).mul_coeff(Rational::from(4));
                    ensure!(!d.is_zero(), "local cut is not square-free");
                    let dv = prefix_value(&d, prefix)?;
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
                let coefficients = p.to_univariate_polynomial_list(variable);
                let values = coefficients
                    .iter()
                    .map(|(coefficient, _)| prefix_value(coefficient, prefix))
                    .collect::<Result<Vec<_>>>()?;
                if values.iter().any(|v| v.is_zero()) {
                    raw.extend(coefficients.into_iter().map(|(coefficient, _)| coefficient));
                    break;
                }
                if values.iter().all(|v| sign(v) == leading_sign) {
                    raw.extend(coefficients.into_iter().map(|(coefficient, _)| coefficient));
                    rules.push(PositiveRayRule {
                        polynomial: id,
                        case: PositiveRayCase::DescartesZero {
                            coefficient_sign: leading_sign,
                        },
                    });
                    continue;
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
    };
    if raw
        .iter()
        .any(|p| prefix_value(p, prefix).is_ok_and(|v| v.is_zero()))
    {
        return Ok((bearing, raw, 0, 0, root_seconds, ray));
    }
    let mut pairs = 0;
    for (j, &id) in bearing.iter().enumerate() {
        for &other in &bearing[j + 1..] {
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
            let r = operations.resultant(&polynomials[id], &polynomials[other], variable)?;
            ensure!(
                !r.is_zero(),
                "distinct irreducible cuts have zero resultant"
            );
            raw.push(r);
            pairs += 1;
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
        report
    }
    fn publish(&mut self, force: bool) -> Result<()> {
        let Some(path) = &self.progress_path else {
            return Ok(());
        };
        if !force && self.last_progress.elapsed().as_secs_f64() < 0.5 {
            return Ok(());
        }
        write_progress(path, &self.snapshot())?;
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
        self.report.current_prefix = prefix.iter().map(ToString::to_string).collect();
        self.report.active_phase = "local_projection".into();
        self.publish(false)?;
        self.operations.progress_context = Some(self.snapshot());
        let mut pair_seconds = 0.0;
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
                    &self.normalized.algebra,
                    &self.polynomials,
                    cuts,
                    level,
                    prefix,
                    constraint,
                    coordinate,
                    self.options.positive_pair_separation,
                    self.options.positive_pair_cancellations,
                    &mut pair_seconds,
                    &mut self.operations,
                )?;
                if self.options.positive_pair_separation {
                    self.report.pair_separation_candidates += bearing
                        .len()
                        .saturating_mul(bearing.len().saturating_sub(1))
                        / 2;
                    self.report.pair_resultants_omitted += ray.separations.len();
                    self.report.second_cancellation_pairs_omitted += ray
                        .separations
                        .iter()
                        .filter(|proof| proof.cancellations == 2)
                        .count();
                }
                self.report.positive_ray_stacks += 1;
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
                (bearing, raw, pairs, omitted, seconds, Some(ray))
            } else {
                let (bearing, raw, pairs, omitted, seconds) = local_projection_raw(
                    &self.normalized.algebra,
                    &self.polynomials,
                    cuts,
                    level,
                    prefix,
                    &mut self.operations,
                )?;
                (bearing, raw, pairs, omitted, seconds, None)
            };
        self.operations.phases.root_classification_seconds += root_seconds;
        self.operations.phases.pair_separation_seconds += pair_seconds;
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
            positive_ray,
        })
    }
    fn visit(&mut self, prefix: &mut Vec<Rational>) -> Work<Built> {
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
        loop {
            self.check_budget()?;
            let cut_list: Vec<_> = cuts.iter().copied().collect();
            let projection = self.project(&cut_list, level, prefix)?;
            self.report.active_phase = "root_isolation".into();
            self.publish(false)?;
            let root_started = Instant::now();
            let roots = stack_roots(
                &self.normalized.algebra,
                &self.polynomials,
                &cut_list,
                level,
                prefix,
                projection.positive_ray.as_ref(),
                &projection.root_bearing,
            )?;
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
                let sample =
                    sector_sample(if i > 0 { roots.get(i - 1) } else { None }, roots.get(i));
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
                match child {
                    Err(Stop::Guard(id)) if highest(&self.polynomials[id]) == Some(level) => {
                        if cuts.contains(&id) {
                            return Err(Stop::Unsupported(
                                "nongeneric sample remained on an existing stack cut".into(),
                            ));
                        }
                        additions.insert(id);
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
                self.report.refinements += 1;
                continue;
            }
            self.guard_nonzero(&required, prefix)?;
            let built = Built {
                node: LocalNode::Stack {
                    cuts: cut_list,
                    projection,
                    roots: roots.iter().map(bound).collect(),
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
    ensure!(
        (1..=2).contains(&options.positive_pair_cancellations),
        "positive pair cancellations must be one or two"
    );
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
                .min(32 * 1024 * 1024),
        ),
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
    builder.report.current_prefix.clear();
    builder.report.active_phase = "complete".into();
    builder.publish(true)?;
    let report = builder.snapshot();
    statistics.projection_seconds = report.phases.factor_seconds
        + report.phases.discriminant_seconds
        + report.phases.resultant_seconds
        + report.phases.domain_certificate_seconds
        + report.phases.pair_separation_seconds;
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
    positive_ray: bool,
    positive_pair_separation: bool,
    positive_pair_cancellations: usize,
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
                let (bearing, raw) = if let Some(ray) = &projection.positive_ray {
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
                        &self.normalized.algebra,
                        self.polynomials,
                        cuts,
                        level,
                        prefix,
                        ray.constraint,
                        ray.coordinate,
                        self.positive_pair_separation,
                        self.positive_pair_cancellations,
                        &mut 0.0,
                        &mut IndependentOperations,
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
                        &mut IndependentOperations,
                    )?;
                    (bearing, raw)
                };
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
                let actual_roots = stack_roots(
                    &self.normalized.algebra,
                    self.polynomials,
                    cuts,
                    level,
                    prefix,
                    projection.positive_ray.as_ref(),
                    &projection.root_bearing,
                )?;
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
                    if i == 0 {
                        if let Some(ray) = &projection.positive_ray {
                            ensure!(
                                matches!(sector.child, LocalNode::Rejected { constraint } if constraint == ray.constraint),
                                "negative half-line must be rejected by its positive coordinate premise"
                            );
                        }
                    }
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
    ensure!(
        (1..=2).contains(&options.positive_pair_cancellations),
        "positive pair cancellations must be one or two"
    );
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
        positive_ray: options.positive_ray,
        positive_pair_separation: options.positive_pair_separation,
        positive_pair_cancellations: options.positive_pair_cancellations,
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
