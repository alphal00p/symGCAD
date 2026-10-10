# Graph structure, SOFIA and region analysis for GCAD

Research assessment, 10 October 2026. The most actionable transfer is a
graph-informed choice of **legal orders and exact triangular sum coordinates**.
A small existing-helper check below reduces A377's normalized input from 42 to
23 terms. This is a preparation result, not a projection benchmark or a solved
original. No solver or verifier change is proposed without separate validation.

## Identify the package and its scope

The Feynman-integral package matching the name is **SOFIA: Singularities of
Feynman Integrals Automatized**, by **Miguel Correia, Mathieu Giroux and
Sebastian Mizera**. It computes candidate singularities and symbol alphabets;
it is not an expansion-by-regions enumerator. Its
[repository](https://github.com/StrangeQuark007/SOFIA) identifies version 1.0.0,
Mathematica source and the authors' manual. The README describes fetching
source at load time: a reproducible future experiment must pin those fetched
files too, rather than only its notebook.

The [manual, §§2.1–2.2, 3.2–3.3 and 5](https://arxiv.org/html/2503.16601)
optimizes Baikov representations, reuses subtopologies, and intersects Fubini
reductions from different elimination orders. `SolverBound` defaults to 100
monomials and can omit intermediates. Its physical-sheet/pinch reconstruction
is unfinished. Crucially, `Symmetry` can return **one-way kinematic
specializations**, not just invertible symmetries (§3.3.4 and Appendix C).
Therefore neither a finite-bound output nor a returned symmetry rule is a
GCAD coverage or transport certificate.

Related primary sources address different parts of the problem:

| Source and precise location | Useful content and limitation |
| --- | --- |
| A. Pak and A. V. Smirnov, [Geometric approach to asymptotic expansion of Feynman integrals](https://arxiv.org/abs/1011.4863) | The geometric region-finding approach underlying **asy**. Its objects are asymptotic scalings, not finite-kinematics sign cells. |
| B. Jantzen, A. V. Smirnov and V. A. Smirnov, [Revealing potential and Glauber regions automatically](https://arxiv.org/html/1206.0546), §3, Eqs. (9)–(14), and program description | **asy2** adds cancellation-resolving substitutions and domain splits (`PreResolve`). Its outputs include transformations and scaling vectors; keep these two roles separate. |
| G. Heinrich et al., [Expansion by regions with pySecDec](https://arxiv.org/html/2108.10807), §§3.1–3.3 and 5.2 | Newton-facet region construction, vanishing multiple expansions and indefinite-sign limitations. Extended region integrals and regularized overlaps do not constitute disjoint CAD coverage. |
| E. Gardi, F. Herzog, S. Jones and Y. Ma, [Dissecting polytopes](https://arxiv.org/html/2407.13738), §§2.2–2.4, 3.2–3.3 and 4.4 | Opposite-sign cancellations can hide interior pinches from the Newton polytope. Exact dissection can expose them as boundaries. Their massless scattering examples do not prove a corresponding theorem for our massive ZZ inputs. |
| E. Gardi, [Hidden Region Finder](https://arxiv.org/html/2609.29442), §§2.2, 2.4–2.5; [release 1.0.1](https://github.com/einangardi/HiddenRegionFinder-release/releases/tag/v1.0.1) | September 2026 preprint/package: separates candidate cancellation loci, scaling hierarchy and final dissection certification. Localizing by a factor requires its nonvanishing; a candidate locus alone is insufficient. This is a distinct package from SOFIA. |
| M. Borinsky, [Tropical Monte Carlo quadrature](https://arxiv.org/html/2008.12310), Theorems 19 and 32, Remark 35 | Exact sector maps and graph/Newton-polytope structure motivate geometric scoring. The generalized-permutahedron statement has Euclidean, non-exceptional hypotheses; general non-Euclidean cases need not satisfy it. |
| F. Brown, [On the periods of some Feynman integrals](https://arxiv.org/html/0910.0114), §§1.4, 7.1–7.5, Theorem 96 | Vertex-frontier width and Dodgson identities explain favorable graph-polynomial elimination orders. The theorem about the first graph polynomial is not a guarantee of low-degree GCAD projection for our massive second polynomial. |

## What must remain exact

Our task covers the original strict domain and both signs of the requested
split at finite kinematics: `s > 4`, unrestricted `t`, and positive Feynman
coordinates. An ordinary `F=0` sheet is a required boundary even when it is
not a Landau pinch. Interior Landau conditions additionally impose stationarity;
endpoint cases have their own active-coordinate conditions. A critical point
of one coordinate projection need not be a physical Landau singularity.

Thus intersecting projection sets from different orders cannot replace a
fixed-order delineability proof. Discarding subleading monomials, making an
invariant zero, or omitting a scaleless integral does not preserve this GCAD
problem. Adding verified extra cuts is conservative but can be expensive;
removing cuts still needs the existing exact root, sign and coverage obligations.
These are requirements of our output contract, not claims that the cited
integration algorithms should supply CAD certificates.

## Concrete graph structure already available

The existing [exact ZZ guide audit](ZZ_RAYLEIGH_GUIDE_AUDIT.md) reads the
upstream literal Symanzik data and checks each original normalized F. Its
retained U expressions give the same block identity in the three targets:

\[
U=ab+(a+b)z,\qquad z=x_8.
\]

| Original | a | b | U terms | Normalized F terms |
| --- | --- | --- | ---: | ---: |
| A377 | x0+x3 | x4+x5+x6 | 11 | 42 |
| A414 | x1+x2+x3 | x4+x7 | 11 | 45 |
| A446 | x1+x2+x3 | x4+x5+x7 | 15 | 78 |

Each F is linear in x8. However, the same audit gives exact counterexamples to
the global positive-guide condition for both orientations on every screened
linear axis. Positivity of U or an apparent U symmetry must not be promoted
to positivity or symmetry of F. Preserve masses, external momentum labels,
parameter values and the signed split when validating graph transformations.

## Ranked experiments and their proof obligations

**1. Graph-block orders with a cost model that distinguishes the three cases.**
Generate a small, deduplicated set of edge orders minimizing graph frontier
width, keeping the a and b blocks together, and putting x8 first in projection
when legal. Remember that public orders are **lifting** orders, so projection
uses their reverse. Compare homogeneous anchors inside a or b, leaving x8
free, against the existing x8-anchor recipes; reject any violation of fixed
orders or variable groups before spending algebra time.

The current [ordering implementation](../src/ordering.rs) already has Brown,
MODS, T1, SOTD and minimum-fill scores. Its sparsity graph joins all variables
present anywhere in one polynomial, so a single dense F obscures graph-block
structure. Add graph information as an untrusted seed/ranking input, not a
pruning rule. Score current degrees and support, coefficient heights, predicted
pair-resultant size and availability of existing exact positive-ray rules.
For A377 prioritize the [measured resultant growth](A377_BINARY_RESULTANT_DIAGNOSTIC.md);
for A414 also penalize cut/refinement inventory and root/prefix work identified
by the [CPU profile](A414_SUBTREE_CPU_PROFILE.md). For A446 penalize the
high-degree inner cuts, while retaining its known genuine projection-critical
locus. Reuse earlier tested orders rather than repeating blind portfolios.
This experiment changes no polynomial or proof semantics.

**2. Order-triangular block sums, with explicit original-coordinate replay.**
In a chosen lifting order, retain the earlier block members and replace its
last coordinate by

\[
z_j=x_j+S(x_{<j}),\qquad x_j=z_j-S(x_{<j}).
\]

For positive-coordinate sums, the exact new constraint is `z_j > S`, not just
`z_j > 0`. The map is globally invertible, prefix-triangular and increasing in
the current axis. It preserves cylinder structure and dimension. Two block
sums make U a three-term expression while keeping all residual coordinates.

Using the already-built [triangular-shift helper](../examples/triangular_shift_probe.rs),
six bounded preparations checked each constraint and split by exact Symbolica
inverse substitution. They retained all kinematics and selected legal orders
with s,t first and x8 last. Internal names x3/x6/x7 now denote sums:

| Original | First replacement | Second replacement | F terms, original → first → both |
| --- | --- | --- | ---: |
| A377 | x3 := original x0+x3 | x6 := original x4+x5+x6 | 42 → 37 → 23 |
| A414 | x3 := original x1+x2+x3 | x7 := original x4+x7 | 45 → 51 → 41 |
| A446 | x3 := original x1+x2+x3 | x7 := original x4+x5+x7 | 78 → 82 → 68 |

These are exact sparsity observations only; even the first shear can increase
terms. A377 is the best initial candidate by this measure. The chosen orders
are legal for these unfixed original inputs; they are not asserted identical
to every prior campaign order.

For a future wrapper, independently verify the transformed proof, then replay
the inverse on every cell, sample, polynomial and exceptional boundary.
Full-real ascending or descending distinct-root indices survive the translation.
**Positive-only indices do not generally survive:** the cutoff moves from zero
to S, and roots can cross zero without crossing that cutoff. Initially require
full-real selectors on changed axes, or design a separately checked shifted
cutoff selector. Wedge constraints may also disable today's coordinate-only
positivity shortcuts. Preserve that fallback rather than weakening a premise.
Compose any homogeneous reduction with its existing exact reconstruction;
do not silently replace its positive-scale domain by a simplex convention.

This differs from the existing [sum-factor preconditioner](SUM_FACTOR_PRECONDITIONING.md),
which restores factors before returning and never changes CAD coordinates.
The [kinematic shear t → t−s](TRIANGULAR_KINEMATIC_SHIFT.md) already reduced
A446's input but still timed out; its result is not evidence that the proposed
Feynman-coordinate shears will solve the problem. SOFIA's Baikov optimization
changes the representation of an integral and is not such a prefix bijection.

**3. Signed kinematic partitions and cancellation factors from graph data.**
Separate invariant coefficients of F before combining momentum-conservation
relations. Use their exact factors to propose outer s,t cuts and cheap inner
seams. Start with the existing A446 identity `F=A*x8+B`, not a fresh black-box
singularity search. Its [quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md)
proves B>0 on u≤4 and on u>4,C0≥0, where u=5/9−s−t and
`C0=(X+Y)^2−u*X*Y`. C0 depends on Feynman coordinates: this is not a purely
kinematic two-variable partition. The u>4,C0<0 complement has both signs and
a [certified positive common projection-critical point](A446_FILTERED_JACOBIAN_RESEARCH.md).
Do not restart the disproved universal-Jacobian-sign search.

For A377/A414, first extract similarly small invariant-coefficient factors and
test exact sign certificates on the *entire* proposed pieces. Derivative
factors, as suggested by cancellation analysis, are proposals rather than
proofs of a common pinch. Any localization denominator, slope-zero case,
complement and seam must remain explicitly accounted for. A piecewise wrapper
must refine shared outer boundaries consistently before combining original
cells. Current [fixed-parent ratio research](A446_FIXED_PARENT_RATIO.md)
illustrates why a convenient chart root count alone is not this proof.

**4. Symmetry reuse with a deliberately smaller authority than SOFIA.**
Use labeled graph automorphisms and subtopology signatures to deduplicate order
seeds and algebra proposals. Proof transfer additionally requires an exact
bijective map of F, constraints and signed annotations. For A/Ax pairs,
`t → 5/9−s−t` is the already checked candidate; its reversal requires s before t,
endpoint swaps and Real/RealDescending selector exchange, as detailed in the
[transport design](CROSSING_TRANSPORT_DESIGN.md). Reject specializations,
contractions and noninvertible symmetry rules as full-problem transports.
They may still suggest endpoint formulas, which must be rechecked as exact
identities. No counterpart becomes complete merely because its graph matches.

**5. Newton/tropical information as a bounded secondary score.**
Compute exact support widths or a few graph-derived weight directions before
expanding candidate resultants. Use them to rank orders, anchors and potential
cancellation charts. Keep the full polynomial at every finite point. The
usual coefficient-blind polytope cannot certify absence of sign cancellations;
nor does a preferred asymptotic scaling authorize dropping terms. Generic
monomial sector maps need not be triangular in the mandatory original order,
so original-coordinate export requires more than integral equivalence. A
SOFIA alphabet may likewise supply candidate outer cuts, never permission to
omit an ordinary F sheet or a required fixed-order critical cut.

## Bounded preparation evidence and next gate

Only literature/source reads and the six small preparations ran for this
assessment: no SOFIA installation, new build, projection or solver campaign.
The preparations used CPU9, a five-second/2-GiB cap per invocation, and the
existing helper. Inputs, both stages, exact inverse flags and hashes are saved
under ignored `artifacts/research/literature-regions-01/`.

| Item | SHA-256 |
| --- | --- |
| Frozen helper | `e5dff64cd99db7d25e886e21327f96dda7f0774fa57e23b553f7eb8511c46950` |
| Helper source inspected | `f903b6eaa9fd563e9d23aec6fa0003dc0a677b47b0d852838292a6475ea8ed30` |
| Six-stage preparation summary | `9f8e76d312b39afd43540f611a5a519caae93632d2654b0f293d62005225492a` |

The next useful gate is a bounded projection-cost comparison on these unchanged
finite-kinematics problems, including added wedge guards. Only a measured
benefit would justify implementing the transport wrapper. A completed
transformed decomposition would still require independent original-coordinate
replay before earning original-problem coverage.
