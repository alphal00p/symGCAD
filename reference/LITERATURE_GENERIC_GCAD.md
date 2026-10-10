# Generic CAD literature: priorities for the unresolved corpus

Research date: **10 October 2026**. This is a proposal and source audit, not a
new algorithm, benchmark result or completion claim. Primary papers were read
online; the supplied `DO_NOT_PUSH_REFERENCES/LITERATURE/no_contour_deformation.pdf`
was also read locally (SHA-256
`77dad837197fca429740a4cf8919dd387b3152038a58097814718b0407c4b70e`).

The strongest structural direction is **interval-scoped projection obligations
with certified coverage**, rather than further enlargement of sample-template
stocks. A smaller, independently testable opportunity is omitting selected
leading-coefficient obligations using projective delineability. Neither should
be enabled merely because a sampled fiber looks harmless.

## Scope and observed bottlenecks

The local Jones–Olsson–Stone proceedings, dated 9 July 2024, describe positive
rescalings and exhaustive hierarchy splits that move interior `F=0` surfaces to
integration boundaries (§1.1, Eq. 3). Their massive example explains why quadratic
coordinate dependence complicates this construction (§3, Eq. 18). This requires
whole sign regions and valid boundaries, not merely one sample per component.
[Primary paper](https://arxiv.org/abs/2407.06973).

Our completion contract additionally requires the prescribed original-coordinate
cylindrical arrangement, explicit exceptional seams, and independent replay.
Integration charts, overlapping covers and nonuniform decompositions need a
separate transport/refinement proof before they satisfy that contract.

The [A414 fixed-work comparison](A414_FIXED_WORK_RELEASE.md) reaches 100,001
nodes and 24,636 refinements in both versions, with identical non-timing
exports and 212 completed operation records across all six runs. The first pair
was 5.3% slower; three-pair median times were 7.747562 and 6.916713 seconds,
10.7% lower after the arithmetic changes. These incomplete fixed-work runs show
no structural node-count reduction or original-case completion. The earlier
[A414 pilot](A414_DEFERRED_PILOT.md) had 2.39
million speculative nodes, a 2,242-polynomial registry and zero deferred
compactions. Conversely, [A377](A377_BINARY_RESULTANT_DIAGNOSTIC.md) contains
expensive genuine pair-resultant calls; replaying ten recovered operations
with matching outputs has not established a completed original decomposition.
These are different targets for research.

McCallum/Strzeboński local projection, adjacent-root chains, positive-ray root
certificates, finite separator stocks, certified domain pruning, homogeneity,
min-fill ordering, bounded MCTS and deferred cut compaction already exist.
The proposals below identify what is missing beyond those mechanisms.

## Ranked proposals

| Rank | Mechanism | Best target | First bounded decision |
|---|---|---|---|
| 1 | Sector-local root barriers and interval-specific obligations | A414 repeated refinement; A377 unnecessary cross-sector resultants | Compare required pair graphs on retained small frontiers before computing new algebra |
| 2 | Reuse certified regions; first measure repeated exact-prefix obstructions | A414 rebuilding conditional work | Count repeated failures and discarded work; preserve the current traversal during measurement |
| 3 | Projective delineability for full-real-rootless fibers | Leading-coefficient-induced ancestor cuts | Count distinct removable LC factors whose discriminant is already required |
| 4 | Choose among independently valid proof supports | Repeated large unions of coefficient/pair guards | Exact subset-dominance reduction on recorded alternative supports |
| 5 | Chordal elimination-tree height after exact structural reduction | Sparse reduced systems, not automatically dense Symanzik input | Compute the actual factor-support graph and constrained legal order candidates |

### 1. Replace sibling-wide requirements with certified interval scope

Nalbach et al., *Levelwise construction of a single cylindrical algebraic cell*
(20 July 2023 version), separate proof rules from cell-selection heuristics.
Their §§4.7–4.10 and §5.2 distinguish preserving the complete root chain from
preserving only the relations needed around one sector. The biggest-cell and
lowest-degree-barrier representations can require fewer or cheaper pair
resultants than the chain representation; §6.1 also warns that resultant zeros
can be irrelevant to the selected root branches.
[Primary paper, Definitions 5.5–5.8 and Rules 4.9–4.13](https://arxiv.org/html/2212.09309v2).

**Proposed application.** A child should return a symbolic validity interval,
its defining indexed roots, and the obligations proving its signs throughout
that interval. A guard needed only on that interval must not automatically
become a globally sign-invariant cut throughout every sibling. This is the
stronger extension already identified, but not implemented, in
[LOCAL_PROJECTION_LIVENESS.md](LOCAL_PROJECTION_LIVENESS.md).

For example, roots of a guard wholly below a child's lower boundary may cross
each other without changing that child's sign. They cannot simply be ignored:
the proof must keep them outside, prohibit interior root births, and protect
the finite boundary functions. Initially retain ordinary individual LC/D
obligations and change only the necessary pair relations. This isolates the
benefit from the much harder problem of omitting discriminants.

Required evidence is a region-labelled graph: polynomial/axis/ordinal vertices,
exact sample root inventory including coincident owners, child interval,
selected ordering edges, and the independent certificate supporting each edge.
An arbitrary spanning tree cannot replace our full adjacent-root chain: the
weaker edge set is justified only for the specified interval.

Nalbach–Kremer's covering extension (5 November 2024 preprint, revised version
read) gives a useful assembly model: Algorithms 8 and 19 characterize an
ordered interval cover and protect the relations between its boundaries;
§8 expresses this as proof obligations. The paper explicitly leaves some joint
root-order optimization unused.
[Primary paper, §§4.3, 8; Lemma 8.4](https://arxiv.org/html/2411.03070v2).

**Validation gate.** First build and replay one small exhaustive stack with
interval-specific supports. Then prove parent coverage and original cylindrical
compatibility recursively. Overlapping interval covers are intermediate objects;
exported cells must be disjoint and have equal-or-disjoint lower projections.
Measure common-refinement cost too: final refinement could erase the savings.
Test a pair of roots born inside an interval, endpoint crossing, coincident
owners, and guards live in only one sibling. Preserve every relevant `F=0`
boundary and refuse completion after a missing interval or resource stop.

### 2. Reuse validity regions, not merely sample answers

Brown–Košta's *Constructing a single cell in cylindrical algebraic decomposition*
(JSC 70, 2015, pp. 14–48) constructs and incrementally refines a cell around a
sample, with a proved merge operation and well-orientedness assumptions.
[Primary article](https://doi.org/10.1016/j.jsc.2014.09.024).
Our proposed adaptation is to retain a completed conditional proof together with
its proved validity region. Reuse at another sample would require exact region
membership and re-specialization of dependent root data; the present exact-prefix
`Built` cache cannot simply be keyed by signs instead. This is a mathematical
reuse proposal, distinct from an internal `Arc`/DAG memory optimization.

The smaller preceding proposal remains the appropriate first measurement:
**exact-prefix obstruction hints**. A failed visit can record an existing
canonical nonzero polynomial `g` with `highest(g) < prefix.len()` and
`g(prefix)=0`. After ordinary leaf and completed-proof lookup, a freshly checked
hit may return the normal `Stop::Guard(g)`. It cannot return a rejected cell or
a proof. The owning ancestor still adds/projects the cut and rebuilds coverage.
Existing-cut contradictions must retain the current diagnostic behavior.

This producer-only cache must be bounded, scoped to one Builder, use exact key
equality and never persist dangling IDs. It partly duplicates surviving learned
cuts and can preserve an unnecessary historical seam. Measure repeated failed
prefixes, owner-level hint misses, later successful alternatives and discarded
descendant work before implementation. Attribute work once per unwind to avoid
double-counting nested failures. No measured saving is claimed.

### 3. A narrow leading-coefficient omission theorem

Nalbach et al., *Projective Delineability for Single Cell Construction*
(1 August 2025), Theorem 3.1 requires a simply connected analytic base,
non-nullification and a nonzero order-invariant discriminant. Lemma 3.2 then
shows that a full-degree specialization with **no real roots at all** remains
rootless without separately imposing leading-coefficient sign invariance.
Their general Algorithm 2 still protects actual finite cell boundaries and
uses cyclic root relations to handle infinity.
[Primary paper, §§3–4](https://arxiv.org/html/2508.00512v1).

**Specific missing case.** Take an unrestricted fiber variable `z` and
`p(a,z)=a*z^2-2*z+2` at `a=1`. The sample polynomial is `(z-1)^2+1`.
Current `local_projection_raw` emits both `a` and `D=4-8*a` before detecting
the empty real-root inventory. On the parent cell `a>1/2`, the constant
coefficient `2` proves non-nullification and `D<0` proves order-invariance.
The cited theorem then needs no separate LC obligation. Even the current
positive-ray `QuadraticDiscriminant` branch retains LC, endpoint and D. This
example demonstrates avoiding a redundant explicit obligation, not enlarging
that rootless cell or a measured corpus saving: other branches may still need
the same factor `a`.

**Proposed minimal experiment.** Restrict to ordinary full-real branches already
computing D. A new typed witness would identify the exact polynomial/axis and
one coefficient certifying non-nullification. Independent replay must recompute
the degree and nonzero sample LC, an empty **all-real** root inventory, D and
its nonvanishing factor obligations, and the selected coefficient's nonzero
ancestor obligation. D nonvanishing establishes order zero throughout the
parent. Its usual open CAD cell supplies the simply connected analytic base;
arbitrary unions need a separate argument. Omit only LC factors not required
by another premise, and emit no root bound for the rootless polynomial.

**Priority relative to existing rules.** Prefer successful Descartes-zero and
interval certificates, which can avoid D entirely. This proposal is a narrow
fallback for full-real-rootless fibers that still require D, especially on an
unrestricted axis. It cannot help the captured reversed A446 octic with two
positive and four negative roots. Positive-rootlessness alone is insufficient:
negative real roots can cross infinity and appear on the positive side. Retain
ordinary LC and ordinal protection for every root-bearing cut. Count useful
distinct LC omissions before writing a production rule. Required regressions
include `a*z^2+1` across `a=0`, negative-root escape through infinity,
nullification, a forged rootless count and ordinary boundary ordinal preservation.
General projective boundaries are lower priority because they would require
explicit finite-root selectors and must not introduce variable infinity into
ordinary exported cells.

### 4. Minimize sufficient supports, never arbitrary guard sets

Babatunde–England–Sadeghimanesh (20 January 2026) formulate a set-covering problem
whose covering sets have associated reason sets, and study reduction and
optimization of those reasons for CDCAC conflicts (§§2.6–4). This is primarily
a conflict-clause optimization study, not evidence of a whole-GCAD speedup.
[Primary preprint](https://arxiv.org/html/2601.14424v1).

**Proposed application.** When two independently replayable witnesses establish
the same regional property, retain a bounded collection of their support sets.
For identical conclusions and scopes, a support subset dominates a superset.
Choose a collection covering every required child/edge, scored by estimated
projection cost rather than only number of factors. This could choose among
ordinary resultants, Bézout witnesses and separator certificates without
changing their mathematics. First inspect whether actual frontiers contain
such alternatives; our current finite stocks are not themselves proof-support
alternatives for an identical region.

An optimizer may be heuristic: the independent checker verifies every selected
identity, premise and covered obligation. No optimization result authorizes
deleting an unmatched guard. Start with exact subset dominance and tiny bounded
enumeration; a general MILP dependency is unwarranted without useful instances.

### 5. Chordal structure: use the correct graph

Li–Xia–Zhang–Zheng (1 February 2021 preprint) define an edge when two variables
occur in the **same polynomial**, not necessarily the same monomial. Their
Propositions 3.1–3.3 and 4.1–4.2 explain preservation under elimination along
a perfect elimination ordering; §§5–6 relate projection growth to elimination
tree height and recommend low-height choices.
[Primary paper, Definition 2.1 and §6.1](https://arxiv.org/html/2102.00823v2).

Our `sparsity`/`chordal` ordering already uses min-fill. The possible addition is
group-constrained elimination-tree-height ranking **after** exact factorization
or a verified structural reduction. A full-support irreducible `F` makes the
correlative graph complete, regardless of a sparse physical diagram; on a clique
the proposed graph advantage disappears. Check the actual factor inventory.

Required data are canonical factor hashes and variable supports, parameters
and coordinate roles, fixed-order/group constraints, fill edges, elimination
tree and height. Physical-graph priors additionally need edge-to-variable maps,
external momenta, masses and exact reconstruction of `U,F`. They remain ordering
heuristics until that reconstruction and a relevant algebraic identity are
checked. Physical graph treewidth, monomial sparsity and chordal CAD sparsity
must not be substituted for one another. Introducing auxiliary circuit
variables also needs an exact elimination/transport proof; it is not free
dimension reduction.

## Useful methods that do not directly satisfy this task

| Literature | What transfers | Limitation here |
|---|---|---|
| Han–Dai–Hong–Xia, *Open Weak CAD and Its Applications*, 2015 preprint/2017 JSC; Definition 27, Theorems 37–39 ([primary](https://arxiv.org/html/1507.03834v3)) | GCD/intersection of projection orders for component sampling and positivity subproblems | The guaranteed output is an open sample, not all required cylindrical boundaries. Intersecting our projection factor sets is unjustified. |
| Brown, *Model-based construction of Open NuCAD*, 24 March 2014 ([primary](https://arxiv.org/abs/1403.6487)); Nalbach–Ábrahám's 1 August 2025 variant ([primary](https://arxiv.org/abs/2508.00505)) | Incremental regional discovery and nonuniform reuse | Individual cylindrical cells need not form the globally cylindrical arrangement required here. Conversion cost and coverage need proof. |
| Bradford et al., *Truth Table Invariant CAD*, JSC 2016; §§2–4 ([primary](https://arxiv.org/html/1401.0645v3)) | Formula-aware projection when actual equational constraints are logically implied | `F>0` or `split F` does not imply `F=0`. Turning a boundary into an equational constraint changes the problem. A414's only inequalities are simple bounds, limiting Boolean pruning. |
| Wilson et al., *Cylindrical Algebraic Sub-Decompositions*, 2014; §§2–3 ([primary](https://arxiv.org/html/1401.0647v2)); Bär et al., *Exploiting Strict Constraints*, 29 June 2023, §3 ([primary](https://arxiv.org/html/2306.16757v1)) | Full-dimensional layers and omission of unnecessary lower-dimensional satisfiability cases | We already work generically with rational open-sector samples. This does not authorize filling strict-zero seams during merging or dropping an entire sign region. |
| Le–Safey El Din, *Solving parametric systems … through Hermite matrices*, 28 November 2020 preprint ([primary](https://arxiv.org/abs/2011.14136)) | Exact parameter-dependent real-root counts from matrix signatures, under generic finite-fiber/radical hypotheses | Candidate for a bounded root-geometry certificate, not an automatic CAD exporter. Dense minors can recreate resultant growth; parameter count regions still need cylindrical decomposition and branch/endpoint transport. |

For the last route, first use an already captured univariate-in-the-fiber system,
prove its specialization assumptions and compare exact signature-certificate
size with existing D/interval obligations. Do not start with a new large
Gröbner computation for all of `F=0`. Constant root counts alone do not specify
ordered branches, exclude endpoint crossings, or prove finite-bound continuity.

## Recommended next evidence

1. Reuse A414's existing fixed-work configuration for a bounded observer:
   repeated obstruction prefixes, per-child guard scope, distinct LC-only
   requirements and alternative support sets. This is diagnostic work, with
   no acceptance or rejection changes.
2. Replay retained small A414/A377 stacks offline. Compare current chain edges
   with valid sector-barrier edges and record required premise unions, maximum
   pair degree products and output-degree estimates. A smaller edge graph is
   not a completion claim and may still require expensive discriminants.
3. Implement only the first measured applicable rule, with independent replay,
   exact common-refinement regression tests, historical-proof compatibility and
   tamper tests for omitted roots, intervals, owners, premises and seams.
4. Run same-input/order/build/cache/resource ablations. Include discovery and
   final cylindrical-refinement costs, and require independent original-problem
   verification before changing completion totals. Preserve every incomplete
   outcome; operation-level gains and smaller samples are separate evidence.

No production option, solver, source file or heavy algebra campaign was changed
for this literature review. The bounded obstruction-hint proposal remains a
measurement proposal, not an implemented optimization.
