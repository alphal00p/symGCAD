# symGCAD algorithm research

The optimization target is an exact generic decomposition into open cells,
with complete coverage away from explicitly recorded exceptional algebraic
sets. Finding valid sample points is not sufficient. Search heuristics may
discard candidate *orders*, never mathematical solution regions.

## Workloads and reference method

The supplied local paper is [Jones, Olsson and Stone, 2024](https://arxiv.org/abs/2407.06973).
The [2025 paper](https://arxiv.org/abs/2506.24073) and
[2026 proceedings](https://arxiv.org/abs/2603.05444) explain subsequent progress:
positive and negative regions of the second Symanzik polynomial can be mapped
to integration boundaries. Algebraic boundaries can still require further
work in downstream sector decomposition, so a completed GCAD is a distinct
milestone from completed numerical integration.

The [HP2 talk, slide 27](https://indico.kit.edu/event/5167/contributions/23879/attachments/11093/19138/talk_hp2.pdf)
describes 263 master integrals in 115 sectors, with fixed values
`mt2=1`, `mz2=5/18`, `s=10`, `t=-3`. The local ZZ test harness instead keeps
`s>4` and, when present, `t` symbolic. These workloads must remain separately
identified; solving a fixed point does not solve its symbolic parent problem.

The reference projection operator comes from
[Strzebonski, 2000](https://doi.org/10.1006/jsco.1999.0327): normalize and factor,
project leading coefficients, discriminants and pairwise resultants, and lift
through all intervals between roots. The reference's aggressive cell merging
may include removed seams where strict inequalities fail. Integration output
must retain singular seams even when the signs on both sides agree.

## Optimization hypotheses, in priority order

1. **Equivalent exact algebra.** Canonical factor caches, reuse between `F>0`
   and `F<0`, low-degree resultant formulas, coordinate-boundary evaluation,
   and avoiding redundant degree-zero resultants can reduce work without
   changing the projection theorem. Cache keys must include the actual
   polynomial set, variable map, domain and transformations. Eliminated
   variable subsets alone are insufficient because projection is path dependent.

2. **Certified domain pruning.** Strictly positive or negative projection
   factors need no zero-set decomposition on the certified domain. Start with
   coefficient-sign proofs on positive orthants. Bounded exact
   [Polya certificates](https://experts.illinois.edu/en/publications/a-new-bound-for-p%C3%B3lyas-theorem-with-applications-to-polynomials-p/)
   and [Bernstein certificates](https://rileroy.perso.math.cnrs.fr/pdf/leroy_SAGA_08.pdf)
   are stronger candidates. Keep factors when tests are inconclusive. A
   nonnegative factor can have interior zeros and must not be deleted merely
   because no sampled point is negative. Domain boundaries remain explicit.

3. **Homogeneity.** When every relevant constraint is scale invariant on the
   positive orthant, fixing a positive anchor to one removes a scale variable.
   Return an exact inverse transformation and positive scale. The actual
   cylindrical order must place the scale before ratio variables. This does
   not automatically apply to unit-box domains or nonhomogeneous constraints.

4. **Order selection.** The implemented MODS, SOTD and T1 methods reproduce
   the original greedy metrics and tie breaks. Brown and minimum-fill
   correlative-sparsity scores provide additional candidates. The
   [complexity-based ordering study](https://arxiv.org/abs/2206.13480) emphasizes
   accounting for heuristic overhead; the
   [chordal ordering study](https://arxiv.org/abs/2102.00823) motivates preserving
   sparse variable interactions. Minimum fill is a heuristic, not a theorem
   that permits omitting projection factors.

5. **Bounded MCTS.** The scheduler searches legal elimination prefixes using
   UCT with inexpensive input-feature priorities and seeded rollouts. Fresh
   trials receive one second; promising unfinished candidates are promoted to
   ten and sixty seconds. Total search defaults to the smaller of 300 seconds
   and 20% of the campaign budget, with at most 256 iterations. The callback
   must run killable workers because a single algebra operation may not offer
   cancellation. Only completed exact solves determine the best solved order;
   censored trials can provide an explicitly unfinished recommendation. Record
   search overhead separately and include it in end-to-end comparisons.
   [Kuipers et al.](https://arxiv.org/abs/1207.7079) demonstrates MCTS for Horner
   variable ordering, not GCAD; benefits for this corpus require measurement.

## Further experiments and correctness restrictions

[Local projection](https://arxiv.org/abs/1405.4925) can avoid unnecessary global
subdivision, but requires its own checked applicability conditions and
exhaustive cell-covering algorithm. [Open weak CAD](https://arxiv.org/abs/1507.03834)
can support positivity checks, but its representative samples do not by
themselves provide the integration-cell boundaries needed here.

Symanzik structure suggests low-degree variables, simple endpoint polynomials,
derivative factors, symmetry and sparsity as ordering features.
[Brown's polynomial reduction](https://arxiv.org/abs/0910.0114) and
[Panzer's compatibility graphs](https://arxiv.org/abs/1403.3385) motivate
additional experiments, but do not automatically justify a smaller GCAD
projection operator. Physical Landau singularities do not describe every
regular `F=0` threshold sheet requiring decomposition; the
[Whitney-stratification analysis](https://arxiv.org/abs/2402.14787) provides
relevant context. Physics-based expectations remain ordering heuristics until
their pruning consequences have exact certificates.

For cases the reference cannot finish, coverage requires a checked trace:
input-factor reconstruction; required projection-factor inclusion or checked
nonvanishing certificates; exhaustive exact fiber-root isolation; and a child
or rejection witness for every open sector. Accepted samples establish signs
only after projection/delineability establishes sign invariance. Random
sampling is supplemental and cannot rule out a missed narrow component.

## Implemented monotone quadratic reduction

For `q(z)=a*z^2+b*z+c` on `z>0`, exact certificates establishing `a>0,b>=0` on the whole relevant base domain prove strict increase and positive unbounded growth. If `c<0`, exactly one positive simple root exists (index 1 among all distinct real roots), and its two sectors have opposite signs. If `c>=0`, the entire positive axis has positive sign. The linear variant requires a strictly positive leading coefficient. A nonzero endpoint's zero sheet may be recorded in the generic exceptional set; an identically zero endpoint is handled as a positive cylinder, never discarded as an exceptional polynomial.

The implemented engine peels only variables with positive-axis domain constraints and no additional constraints involving them. It solves the endpoint problem exactly, restores variables in reverse elimination order, and independently checks every coefficient certificate and reconstructed sector. The resulting cells remain cylindrical in the requested coordinates. Both fixed A446 topologies admit four such peels and verified eight-cell decompositions after positive-scale reduction.

This argument has a real applicability boundary for unrestricted symbolic kinematics: a negative linear coefficient can produce two positive roots even with a positive endpoint. `examples/symbolic_structure_audit.rs` verifies an exact counterexample and an alternative positive-definite quadratic identity in additive graph coordinates. That additive identity is research evidence; it does not authorize relabeling transformed cells as an original-coordinate CAD or restricting the symbolic domain to its physical region.

## Local positive-ray projection and conditional proof reuse

### Adjacent-root chain projection

The optional `solver.local_adjacent_pairs=true` uses an indexed-root chain,
an established local projection technique: [Nalbach's 2025 dissertation,
Definition 5.6 and Rule 4.36](https://publications.rwth-aachen.de/record/1022295/files/1022295.pdf)
describes consecutive sample-root comparisons and the delineability/resultant
premises needed to preserve them. Strzeboński's Algorithm 8 instead retains
every pair of root-bearing polynomials.

Our implementation keeps all individual leading-coefficient, discriminant,
endpoint and conditional coefficient obligations. On the final connected
parent cell, each relevant simple root branch is continuous. A selected pair
resultant, or the existing certified positive-pair separation, keeps each
adjacent difference nonzero and thus preserves its sample sign. Transitivity
then fixes the complete root order. Same-polynomial neighbors need no extra
pair guard. Every sibling refinement reconstructs the chain from the enlarged
cut inventory.

This adapts the chain to generic open cells and to the separately certified
positive ray. In positive mode only strictly positive branches participate;
negative-root coincidences and negative collisions under a Descartes-one
witness do not invalidate the positive chain. Before selecting any pair, exact
isolation compares the sum of individual distinct relevant root counts with
the merged union count. A mismatch triggers all-pair fallback, preventing root
deduplication from concealing coincident owners. A generated nonzero guard
vanishing at the prefix immediately requests ancestor refinement; a partial
projection is never accepted as a finished certificate.

Evidence records individual counts, the exact sample branch order, selected
pairs and fallback mode. The checker independently reconstructs all of them
and still requires every open sector and inherited guard. Tests compare exact
common refinements with full-pair local and ordinary global CAD, exercise
multiple branches per polynomial and coincident roots, and reject altered
pair, ordering, scope and sector evidence. The option remains off by default
pending hard-workload measurement. With fewer than three bearing polynomials
no pair can be omitted, so the implementation avoids redundant chain isolation.

On an originally constrained positive axis, quadratic coefficient signs and
Descartes' zero- and one-variation cases can certify root counts without a
discriminant. One variation counts exactly one positive root with multiplicity,
so that root is simple. Every nonzero coefficient-sign premise is propagated
to the ancestor; its nonzero leading and constant coefficients prevent escape
through infinity or zero. Negative-root collisions do not affect this positive
root function. Its exported ordinal therefore counts positive roots explicitly;
an ordinal among all real roots would be unsound here. Other cases retain full
real-root ordinals and their required projection obligations. Tests cross a
negative-root collision, compare exact common refinements, and reject altered
ordinal domains or coefficient-sign evidence.

For two cuts `p,q`, an exact polynomial combination `R=A*p+B*q` that is nonzero
for every positive axis value rules out a shared positive root. The implemented
bounded shortcut uses at most two leading-term cancellations, and certifies R
through same-sign coefficient guards on the entire ancestor cell. No division
by a parameter expression is needed. The second cancellation is selected by an
exact rational diagnostic, then constructed and checked symbolically; the
diagnostic alone cannot remove a resultant. On a captured stalled quartic pair,
the first remainder has a positive root, but a second cubic combination has four
positive coefficients at the current prefix. Its 1,225-term symbolic expansion
took 2.75 ms in a debug diagnostic and its coefficient factorizations about
77 ms. These operand measurements justify an experiment, not a completed CAD.

Local saturation can revisit the same exact rational prefix after refining an
ancestor. A completed subtree is a proof conditional on its full returned
ancestor guards. Reusing it at the same prefix remains valid when all those
guards are propagated again; reusing its samples at a different prefix would
not. The in-process cache therefore stores only completed, bounded-size proof
trees and preserves every obligation. Independent replay still checks the
resulting full decomposition without relying on the cache.

Unfinished stacks may separately retain bounded learned-cut hints at the same
exact prefix. Ancestor refinement previously discarded these hints and repeated
their discovery. Reusing a cut set entails no sign or coverage assertion: the
entire seeded set is projected again and every sector regenerated. FIFO bounds
apply to entry count, polynomial IDs, prefix bit size, and retained bytes.

The default retained budgets remain 32 MiB for completed subtrees and 8 MiB
for learned cuts. `solver.local_subtree_cache_mib` and
`solver.local_learned_cut_cache_mib` permit larger controlled experiments,
clipped respectively to 1/16 and 1/128 of the process memory limit. Node,
entry and ID ceilings scale upward with the effective byte budget; per-entry
size/depth and prefix-bit bounds remain fixed. Reports expose the effective
budgets and count ceilings. Changing these exploration settings does not
change proof replay or allow reuse at a different rational prefix.

Some large symbolic inputs are linear in a kinematic parameter with a monomial
leading coefficient. For `A*y+B`, degree one over the fraction field and
`gcd(A,B)=1` prove irreducibility over the rational polynomial ring by Gauss's
lemma. When A is a monomial, the gcd condition is an exact exponent-map check.
The shared factoring path uses this bounded sufficient test and otherwise
retains Symbolica factorization. Captured input calls finish in milliseconds;
their stale progress markers did not establish that factorization caused the
larger campaign timeouts. The shortcut improves those isolated exact calls,
without a demonstrated end-to-end hard-case speedup.

The kinematic shear `u=s+t+13/9` has positive unit slope and is triangular when
s precedes t. Exact substitutions reduce the symbolic A446 polynomial from 78
to 58 terms and roughly halve some captured projection operands. Bounded
transformed-coordinate experiments still timed out; no original-coordinate
decomposition is claimed. Any future use must reconstruct cylindrical root
boundaries and certify the inverse map over the entire symbolic domain.

Measured fixed-case ordering results are negative for MCTS so far: all seven
policies solve the six-case cold ablation, while search overhead makes MCTS
slower than the inexpensive portfolio. Detailed limits, medians and hashes are
in `BENCHMARKS.md`. A search heuristic's usefulness on the hard symbolic cases
remains an open empirical question.

## Exact interpolation of bivariate resultants

For inputs of eliminated-axis degrees `m,n` and total degrees `P,Q`, the
Sylvester determinant has parameter degree at most `n*P+m*Q-m*n`. Every
determinant monomial chooses `n` coefficients from the first input and `m`
from the second, and their eliminated-axis exponents sum to `m*n`.
Cancellation can only lower the degree. The rectangular degree bound provides
another upper bound; the smaller of the two suffices.

For one surviving parameter, the bounded implementation evaluates at distinct
exact integers, skips points where a leading coefficient vanishes, computes
each numeric resultant with deterministic Symbolica Ducos, and reconstructs
the polynomial with Symbolica Newton interpolation. It uses the complete
degree bound plus one valid image, preserving rational scale and sign. This
is a polynomial identity proof, not an early-termination heuristic. Numeric
zero images retain the original variable map. Support, degree, image sizes,
and cancellation are checked; a stopped interpolation cannot trigger a costly
fallback or become a complete result. Existing deterministic operation caches
remain compatible; older automatic-resultant caches remain invalid.

The captured symbolic A446 outer-`t` pair has degrees 23 and 17, giving the
bound 871. Its 872-image computation completes in about six seconds and yields
a degree-461 polynomial with 318 terms. An independent fresh 872-image replay
proves the same identity. This removes one measured algebra obstacle; it does
not establish a complete symbolic A446 decomposition.

## Separating large exact root unions

The global A381 chart produces 12,054 candidate roots at its first lifting
stack. Re-sorting and scanning that complete list after every collision adds
substantial work. An ordered queue instead exposes the lowest interval. If it
ends strictly before the next lower endpoint, it lies before every pending
root and can be finalized. Otherwise exact shared-root checks or bisection
resolve the collision and the affected intervals return to the queue.
Refinement only shrinks intervals, so finalized roots cannot be crossed later.
Shared roots retain the smallest source-polynomial identifier. No tolerance or
floating-point comparison participates in root equality or ordering.

A matched 90-second release diagnostic on the fixed A381 `x8=1` chart does
not isolate the cost of this root-union phase. Both `7629a91` and `e43a2c4`
loaded the same six completed projection checkpoints and timed out on one CPU
with a 4 GiB address-space cap. The newer release published 2,004 domain
nonvanishing certificates and reached lifting, recording 79,138 visited stacks
and 66,657 partial cells at 89.07 seconds. The older release published neither
the completed certificate list nor a lifting diagnostic. Its empty removals
array is stale: that list is accumulated privately until certified projection
finishes. An additional bounded eight-second replay sampled the old release
inside domain certification, univariate Sturm counting, and rational
evaluation/GMP gcd. Root sign routines are also used by these pre-lifting
proofs. Thus the evidence shows different progress under a fixed deadline,
without establishing a root-separation speedup, a completed solve ratio, or
certified coverage from the partial cells. Detailed hashes and observations
are in `artifacts/profile/a381-root-reuse-release-comparison/summary.json`.

## Two-coordinate simplex chart experiment

The projective normalization is established in
[Panzer, *Feynman integrals and hyperlogarithms*, §2.1.3,
eqs. (2.1.19)–(2.1.21)](https://arxiv.org/pdf/1506.07243): a nonzero
linear form with nonnegative coefficients separates the positive scale
from the positive projective coordinates.
[Artico and Magnea, §4, Theorem 2 and its Cheng–Wu discussion](https://arxiv.org/pdf/2310.03939)
explain why a subset of Feynman parameters can define the normalization
hyperplane. Choosing `H=a+b` motivates the chart below. Their integral
identities do not establish cylindrical CAD reconstruction; its triangular
map and root-ordinal obligations are separate requirements here.

For a positive homogeneous block, set `a=1-r`, `b=r` and leave its other
coordinates as `z` to construct a reduced polynomial `q`. If the original
polynomial has block degree D and `L=a+b`, the exact identity is
`L^D*q(b/L,x_other/L)=p`. The research helper `examples/simplex_probe.rs`
checks this identity separately for every normalized constraint and split by
termwise polynomial rehomogenization. It requires the entire final variable
group as its explicit block and original strict positive coordinate
constraints; kinematic parameters stay outside the block.

The inverse map is triangular for original lifting order
`[outer parameters,a,b,other block axes]`: with positive scale A, it sends
`a=A`, `b=A*r/(1-r)`, and `x_other=A*z/(1-r)`. The reduced domain is
`0<r<1`, `z>0`. Every original strict constraint is transformed, and its
positive homogeneity multiplier preserves its sign. The helper rejects
incompatible explicit orders rather than silently changing them.

This identity does not yet supply an original-coordinate cell certificate.
In particular, reconstructing the b axis needs root ordinals restricted to
`0<r<1`; a source boundary at r=1 maps to infinity, and real roots outside
that interval do not retain their full-real ordinal under the rational map.
Other block axes undergo positive scaling once the preceding b axis is fixed.
Any future production wrapper must replay these boundary and coverage facts.

Exact tests on fixed A381 pairs `(a,b)=(x4,x5)` and `(x6,x5)` reduce its
57-term polynomial to 42 terms and make it affine in r. Its r derivative has
strictly positive coefficients on positive spectators at the fixed
kinematics. Symbolic A446 pairs `(x4,x5)` and `(x2,x1)` reduce 78 terms to 61
and are also affine in r; the unrestricted t parameter prevents the same
global derivative-sign assertion. Four cold 60-second release trials still
timed out, so no hard transformed decomposition or speedup is established.
The helper's small workflow smoke completed and independently verified.
Inputs, exact identities, operation captures and binary hashes are retained
in `artifacts/hard-campaigns/simplex-release-01`; every record explicitly
sets `original_completion=false`.

## Bounded stock of rational root witnesses

A saturated Descartes certificate combines k coefficient sign variations
with alternating strict signs at zero, k−1 ordered positive rational
separators, and positive infinity (the leading-coefficient sign). The
intermediate value theorem supplies at least one positive root in each of
the k intervals; Descartes' bound counts multiplicities, so each interval
contains exactly one simple root. Keeping all nonzero coefficient and
separator-evaluation signs fixed establishes continuous, locally analytic
positive root branches without excluding collisions of negative roots.
Exported root ordinals must therefore count positive roots.

Unbounded regeneration of separator constants can nevertheless prevent
termination. For `p(z,x)=product_j(z-x-j)` on x>0, ordinary leading-coefficient
and discriminant projection is constant, while separators chosen afresh at
successive samples can introduce endlessly translated cuts `p(c,x)=0`.
Valid completed certificates alone do not rule out this search failure.

The proposed construction limit is two distinct separator vectors per
canonical polynomial in the Builder's fixed variable map and axis. Reuse
an existing vector when its exact signs establish a witness; after its
stock is exhausted, use ordinary full-real projection. Admission consumes a
slot immediately, including when the tentative stack later fails. Slots
persist for the Builder's entire lifetime: ancestor restarts and subtree or
learned-cut cache eviction must not reset them, and eviction must never
permit fresh replacement vectors. Stable append-only canonical polynomial
IDs are suitable keys.

This yields a finite symbolic closure by downward induction on the highest
variable. The top-axis inputs are finite. Every polynomial supplies finitely
many coefficients, leading coefficients, discriminants and at most two
fixed evaluation sets; each pair supplies finitely many resultants and
bounded algebraic separation candidates. All resulting guards have strictly
lower highest variable. Factoring and canonical deduplication therefore
produce a finite input inventory at the next axis, and the induction repeats.
Learned hints reuse only guards already generated. With restarts requiring
a newly discovered canonical cut, this removes the unbounded artificial-cut
mechanism; it does not make the finite computation cheap or guarantee
completion within resource limits.

Independent verification must check the supplied witness, not reproduce the
history-dependent stock choice. It recomputes the polynomial's coefficient
sign variation count, ordered positive rational separators, exact evaluations,
strict alternating signs and the full lower-axis guard inventory. It rejects
missing obligations and zero evaluations. A supplied full-real certificate
remains valid when its ordinary leading-coefficient/discriminant obligations
hold, even if the checker could have found a cheaper rational witness.

Future section templates may avoid translated or scaled artificial cuts.
Examples are `c_j=q_j*L`, for a positive earlier-coordinate or Feynman-parameter
sum L, and `c_j=-a_(m-1)/(m*a_m)+q_j*L`, using the Vieta center of a
degree-m polynomial. These are proof sections only; they need not change the
output coordinates. For common positive denominator D and numerators N_j,
the exact additional obligations are `N_1>0`, `N_(j+1)-N_j>0`, `D>0`, and
the signs of `D^m*p(N_j/D)`, expanded polynomially. Existing coefficient
obligations remain. Templates also need finite admission stock and strict
expansion budgets; no performance benefit has yet been measured.

A related prospective pair rule can keep two adjacent, individually
delineable branches apart with a continuous separator c and nonvanishing
`p(c)` and `q(c)`. Every cross-polynomial adjacency edge must be covered;
one separator for a deduplicated polynomial pair is insufficient when its
roots interleave. Exact sample placement, denominator/order guards, stable
positive ordinals and finite template stock would all require replay. This
fixed-rational pair rule is now implemented behind the default-false
`positive_pair_witnesses` option; polynomial/rational sections remain research.

The implementation certifies the complete two-polynomial positive root chain.
Its supplied inventory must contain every pair `(owner, positive ordinal)`
exactly once, with each owner's ordinals increasing from zero. Exact Sturm
counts first check both full positive root counts. For every adjacent edge
`(p,i)->(q,j)` with distinct owners, its strictly positive rational separator c
must satisfy `N_p(0,c)=i+1`, `N_q(0,c)=j`, `p(c)!=0`, and `q(c)!=0`.
Every cross-owner edge is present and separators increase strictly. These
checks establish sample placement even when a template is reused at a new
prefix; signs alone would not suffice. On the connected ancestor cell,
individual positive-root delineability and the nonzero evaluation guards
preserve each edge inequality. Same-owner edges follow from ordinals;
transitivity preserves the whole chain, excluding common positive roots.

Each sorted canonical polynomial pair and main axis admits at most two
templates over the entire Builder lifetime. Admission occurs before any later
stack or ancestor failure, and neither proof-cache nor hint-cache eviction
resets the stock. Each admitted template adds only finitely many evaluations
in strictly earlier axes, so the downward finite-closure argument above
extends to pairs. The current limits are degree 32 and 4,096 terms per
polynomial, 16 total distinct positive roots, eight cross-owner edges, and
256 combined numerator/denominator bits per separator. Shared positive roots,
ineligible inputs, exhausted stock and inconclusive placement use ordinary
resultants. Shared negative roots do not invalidate positive separation.

Independent replay validates the supplied inventory and every exact Sturm
placement, recomputes evaluation signs and polynomials, and propagates their
complete factor obligations through ancestor cells. It does not reconstruct
stock history. A missing pair witness selects the stronger ordinary resultant;
the entire regenerated ray certificate and guard inventory must still match,
so deleting witness metadata cannot silently delete its obligation.

## Sum-of-squares screen for positive repeated roots

For fixed rational w>0, `S(z)=p(z)^2+w*p'(z)^2` vanishes at a real point
exactly when p and its derivative both vanish. Strict positivity of S on
z>0 therefore excludes positive multiple roots. Together with nonzero
leading coefficient and endpoint p(0), this can establish positive-root
delineability while allowing negative-root collisions. A prospective output
rule would use positive ordinals and retain those leading/endpoint guards.

The standalone research helper `examples/profile_sos_discriminant.rs` screens
coefficient nonnegativity of `(1+u)^k*S(lambda*u)` with positive rational
lambda and k<=64. A nonzero polynomial with nonnegative coefficients is
strictly positive on the open ray. The derivative is taken in the original
z coordinate before substitution. Numeric specialization only chooses a
candidate: any symbolic certificate must propagate strict positive signs of
every nonzero symbolic coefficient through ancestor cells. A coefficient
that vanishes only at the sample cannot be discarded. The helper checks
the reconstructed specialization identity and caps estimated polynomial
products before expansion; it does not change production projection.

Its exact weight mode writes each coefficient as `A_i+w*B_i` and intersects
all rational half-lines `A_i+w*B_i>=0` with w>0. This tests all positive
weights, not just rational squares or a finite weight grid. The bounded
scale/degree portfolio remains finite; any future adaptive symbolic rule
also needs a lifetime stock limit to avoid generating unbounded new guards.

For the captured 1,232-term, degree-eight A446 discriminant operand at
`(s,t,x1,x2)=(13/3,-1505,1,1)`, all 81 initial scale/weight combinations
failed through k=64 in 0.57 seconds. The stronger exact weight screen also
failed for every one of 585 systems: nine scales
`1,1/4,4,1/16,16,1/64,64,1/256,256` and k=0..64. It took 0.15 seconds
wall time and 3,108 KiB peak RSS on CPU5. No symbolic square was expanded.
This excludes this bounded certificate family for that fiber, not general
sum-of-squares or higher-order positivity methods. Immutable binaries,
source, input hashes, coefficients and complete trial inventories are in
`artifacts/profile/a446-sos-discriminant-01` and
`artifacts/profile/a446-sos-weight-interval-01`. Six helper tests cover exact
weight intersections, positive versus negative repeated roots, and the
symbolic scaling identity. No CAD completion follows from these profiles.

## Finite coordinate charts for the Gluza30 core example

The Gluza30 polynomial is multiaffine in seven positive coordinates. A finite
atlas splits each original axis at x=1 and substitutes either `x=1+y` or
`x=1/(1+y)`, with y>0. The 128 charts cover the original orthant away from
the seven explicit seams `x_i=1`. After multiplication by the positive lower
chart denominators, every numerator remains multiaffine. The research helper
`examples/core_chart_probe.rs` verifies all cleared inverse identities using
Symbolica; it does not infer coverage from samples.

Exact coefficient screening finds eight sign-definite charts, thirteen mixed
charts with four or five successive increasing linear peels, 58 with one or
two, and 49 without an initial increasing peel in either global sign
orientation. For a multiaffine polynomial on a positive orthant, coefficient
nonnegativity is necessary for global nonnegativity: a negative monomial is
exposed by setting its active variables to T and all others to `1/T`, then
taking T large. Thus the mixed derivative coefficients in those 49 charts
prove a limitation of global monotonicity, not merely a weak positivity test.

The all-lower chart `0<x_i<1` completed with four peels and five full-sign
cells; chart 61 completed with five peels and seven cells. Both transformed
arrangements independently verified. Their release solver times were about
eight milliseconds. A zero-peel control chart timed out at 30 seconds, as did
two additional 30-second trials restricting it to the originally requested
sign using local and global optimized engines. These results establish two
transformed pieces, not an additional complete corpus problem. Exact inputs,
identities, hashes, successful proofs and retained failures are under
`artifacts/research/gluza30-atlas-01`, summarized in `analysis.json`.

Each coordinate map is monotone and independent, so it transports a fixed
cylindrical order; reciprocal coordinates reverse sector order. Original
boundary export needs ordinals restricted to `0<x<1` on reciprocal charts,
or additional certification of any stronger full-real ordinal. The current
research helper chooses per-chart peel orders, which need not agree. A union
of these outputs therefore also requires one common original lifting order
or a certified common refinement, plus every chart seam. No production
transport wrapper has been implemented.

## Finite interval Descartes covers

The opt-in interval rule extends the bounded rational-witness portfolio to
fibers whose positive root count does not saturate their global Descartes
variation. A supplied template partitions the entire positive ray into
contiguous open intervals with rational seams. On `(a,b)`, the substitution
`z=(a+b*u)/(1+u)` and positive multiplier `(1+u)^degree` give an exact
polynomial in u>0; the final tail uses `z=a+u`. Zero or one coefficient-sign
variation gives exactly zero or one positive root, counted with multiplicity.
Thus every accepted root is simple. All nonzero transformed coefficient
polynomials, the original leading coefficient, the value at zero, and the
values at every finite seam become ancestor nonvanishing obligations.

The checker reconstructs the coefficient identities and their signs from the
supplied intervals. It verifies strict seam ordering and a complete contiguous
cover from zero to infinity; it does not repeat discovery or trust a sampled
root inventory. These obligations keep each root inside its own interval on
the connected parent cell. Export uses the resulting positive-root ordinals,
so negative-root collisions do not invalidate the certificate.

Each canonical polynomial has a permanent stock of at most two interval
templates per Builder, in addition to its separate stock of at most two global
rational-separator templates. Admission consumes a slot before later stack
construction can fail; neither eviction nor ancestor refinement resets it.
Failed discovery adds no guards and uses the ordinary discriminant fallback.
The existing downward-variable induction therefore remains finite: every
polynomial contributes only finitely many coefficient/evaluation guards,
and every pair only finitely many ordinary or separator obligations. Degree,
interval count, rational height, transformed coefficient count and total guard
size are independently bounded. A completed certificate remains valid without
reconstructing this resource history; the stock restriction governs search
termination rather than the mathematical meaning of an individual proof.

## A bounded square plus strict residual certificate

Some physically positive coefficients have negative cross terms that defeat
direct coefficient signs, while their parameter dependence prevents rational
LDL factorization. The new domain member tries one rational-weight binomial
square between two positive constant diagonal coefficients. It records the
chosen sign, bound orientations, coordinate indices, positive rational weight
and residual; it performs no unrestricted sum-of-squares search.

The verified original Domain is retained. A coordinate may use its original
upper bound through `x=upper-u`, even when a lower bound also exists. Every
original-domain point maps to u>0, so ignoring the additional lower bound in
the coefficient argument only enlarges the proof domain. After these exact
affine substitutions, replay checks
`sign*p = weight*(u_i-u_j)^2 + residual` and proves the residual strictly
positive with the existing exact coefficient rule. An identically zero or
merely nonnegative residual does not suffice. The checker validates the
supplied identity independently of the discovery weight or orientation order.

Discovery is capped at 16 active variables, 128 polynomial terms, 16 bound
orientations and at most 64 candidate attempts; the fresh default is 16
attempts. `CertificateOptions.square_residual_candidates=0` disables this
member, and omitted historical option fields deserialize to zero. The
physical elliptic x4 coefficient supplies a concrete exact example in
[the positivity note](reference/PHYSICAL_ELLIPTIC_POSITIVITY.md). Its full
original decomposition remains a separate obligation.

The same bounded portfolio now includes an explicit affine-square proof
`sign*p = weight*(u_i-center)^2 + residual`. It is useful when a homogeneous
anchor turns a variable difference into a variable minus a constant. Discovery
tries the exact rational completing-square center from constant diagonal and
linear coefficients, sharing the existing candidate budget; the new method
caps weight/center heights at 256 bits and coefficient heights at 512 bits.
Replay validates the original bound orientations, valid variable index,
positive weight, exact identity and strict residual independently of discovery.
The earlier binomial certificate representation and replay remain unchanged.

## Bounded parameterized quadratic nonvanishing

A polynomial `p=a*z^2+b*z+c` has the sign of a on the entire real z axis if a
is nonzero with known sign and `4*a*c-b^2` is strictly positive. The domain
portfolio now records those two independent existing DomainCertificates in a
typed `QuadraticNoRealRoots` proof. Replay extracts all three coefficients,
reconstructs p exactly, recomputes the signed discriminant, and verifies both
children against the same independently derived original box. A zero or merely
nonnegative discriminant bound is insufficient.

This is a single proof layer. Discovery disables the quadratic member in child
options; replay rejects either child if it is another quadratic proof before
recursing. All older methods run first. The fresh candidate limit is four, the
hard limit is sixteen, and omitted historical options decode to zero. Input
caps are 128 terms, sixteen active variables, degree 32 per axis and 512-bit
rational coefficients. The product estimate and discriminant term count are
capped at 4096; discriminant coefficients are capped at 2048 bits.

The research scan of actual physical elliptic endpoint checkpoints found 1,904
distinct factors. Of 58 eligible multivariate factors, the previous portfolio
certified seven; this nonrecursive rule certified seven more, including four
purely kinematic factors before pp4 projection. The additional 68 candidate
axis attempts cost 0.0233 seconds in the retained debug scan, whose total wall
time was 0.164 seconds. Full independently checked child certificates and source
checkpoint paths are in `artifacts/research/physical-quadratic-guards-01/report.json`.
These are prospective nonvanishing removals, not a completed original CAD or a
runtime benchmark.

## Persistent local search state: research proposal, not implemented

The durable operation cache does not preserve local cut discovery or completed
conditional subtrees across worker deadlines. This distinction is visible in
two recent unfinished runs. The physical x6 local trial
`physical-quadratic-x6-local-09` recorded 74,842 completed-subtree hits and
241,155 evictions, plus 37,884 learned-cut hits and 33,807 evictions. The
300-second `a446-quadratic-integer-release-01` trial recorded 31,971 subtree
hits and 11,357 evictions; its larger hint budget had 624 hits, no evictions
and about 13.9 MB retained. These counters establish repeated work within a
run; they do not yet measure the benefit of persistence across runs.

The first experiment should persist **hints only**. Store exact rational
prefixes and canonical polynomial expressions, never process-local numeric
IDs. Key an archive by the ordered variable map, exact normalized constraints
and splits, parameter substitution, projection/proof options and an explicit
format/algorithm version. Bound archive bytes, entries, prefix bit sizes,
polynomial terms/heights and load time before admitting data. Re-intern the
polynomials into the fresh registry, verify their axis support, and factor them
through the ordinary exact path. Every seeded stack must still recompute its
projection, all sectors and every ancestor obligation. Imported hints may add
cuts but cannot remove original cuts or supply a proof. A truncated or invalid
archive is a cache miss, never a successful or incomplete CAD certificate.

Snapshot shallow active ancestor cut inventories as well as retained hints:
FIFO caches can have already evicted the root or other early prefixes while
their current recursive stack still holds those cuts. Persisting only the
final FIFO tail may therefore miss the discoveries needed to reach the saved
frontier on a restart. Use bounded atomic replacement, coarse time/byte
thresholds and a final best-effort soft-stop snapshot; hard worker termination
must leave the previous completed snapshot usable. Include serialization and
loading in campaign time. A dictionary's actual polynomial bytes must count
toward the disk budget; the in-memory hint cache currently charges IDs because
its registry is already resident.

Completed-subtree persistence requires more work. Existing `Built` payloads
are trusted only within one immutable `Builder`. A disk entry needs a
self-contained polynomial dictionary and guard-removal registry, remapping
both polynomial IDs and removal-record IDs throughout cuts, roots, pair
witnesses, domain sources and returned obligations. Before reuse, independently
replay the conditional subtree at its exact prefix, verify the complete input
factor inventory and domain premises, recompute its cell count and require its
returned obligation set to agree exactly. Return all those obligations to the
new ancestors, whose intervals may differ. A content hash prevents accidental
mixups; it does not replace mathematical replay. Current per-entry limits
(128 nodes, depth 16, at most 1 MiB) should remain.

There is also a termination consideration separate from proof validity.
Descartes and pair-separator template stocks admit at most two templates per
canonical polynomial or pair during one builder lifetime. Restarting with old
hints but an empty template stock can admit another pair of templates each
campaign. Each completed proof remains valid, but the existing finite-stock
argument then applies per run rather than to the entire resumed search. A
persistent search epoch should retain the admission ledger without resetting
slots on eviction, or first benchmark hints with these optional discoveries
disabled. If a bounded persistent ledger fills, disable further admissions
instead of silently granting fresh slots.

Before implementing disk subtree proofs, measure a controlled interrupted
restart with hints disabled/enabled, identical order/options and equal total
wall budget. Report loaded and actually used prefixes/cuts, preprocessing and
I/O cost, newly reached prefixes, algebra cache hits, refinements, and complete
independent verification if achieved. The current evidence favors this small
hint experiment before a trusted-looking but unverified serialized tree cache.
