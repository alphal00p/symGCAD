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
