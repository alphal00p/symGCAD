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
