# CFF-informed kinematic projection and exact graph certificates

Research assessment, 10 October 2026. This note proposes general algorithms;
none is an implemented optimization or a new completed corpus result.

The strongest near-term direction is **conditional projection over an explicit
kinematic chamber**, informed by graph thresholds and certified with the
original parametric polynomial. A second promising direction is an exact
network-flow positivity certificate, which applies to arbitrary two-point
graphs and generalizes the familiar bubble/banana square identity. CFF is
useful input to both directions, but does not replace the real boundary
inventory required by GCAD.

## What the momentum-space information means

For an ordinary CFF cut separating connected vertex sets, write its denominator
as

\[
\eta_S(\ell;p)=\sum_{e\in\delta(S)}
\sqrt{|\mathbf q_e(\ell,p)|^2+m_e^2}-Q_S^0.
\]

The cut supplies a channel, positive on-shell energies, and compatible nested
or disjoint cut families. These are the graph-theoretic structures exposed by
[Capatti, *Exposing the threshold structure of loop integrals*, §§IV–V,
especially (19)](https://arxiv.org/html/2211.09653). For real nonnegative masses,
energy minimization gives the familiar normal-threshold candidate
\(h_S=Q_S^2-(\sum_{e\in\delta(S)}m_e)^2\). The energy-flow orientation matters:
the incoming channel energy must have the appropriate sign. At equality,
pinched configurations need separate treatment. Restricted loop subspaces can
have different effective shifts and feasibility conditions.

Three distinct objects must remain separate:

| Object | What it can inform | What it does not certify |
|---|---|---|
| An E-surface and its opening condition | Active channels, likely difficult parameter chambers, edge groups and candidate sign certificates | The full positive-parameter zero set of F |
| A Landau variety in external invariants | Critical parameter values where the integrand geometry degenerates | Every intermediate cylindrical discriminant or root collision |
| A CAD projection factor | Coordinate-specific delineability, root ordering or endpoint obligations | A physical singularity of the integrated amplitude |

Normal thresholds alone miss anomalous configurations and other boundary
phenomena. [Fevola–Mizera–Telen, *Landau Singularities Revisited*](https://arxiv.org/html/2311.14669)
explain why naive on-shell equations and contraction lists can miss scaling
strata, especially with massless edges and UV/IR divergences; second-type
singularities involve infinite loop momenta. The projective boundary must not
disappear from an exact parametric treatment.
[Helmer–Papathanasiou–Tellander, *Landau Singularities from Whitney
Stratifications*](https://arxiv.org/html/2402.14787), Definition 1, instead
stratify the projection to kinematics, including coordinate boundaries in a
projective compactification. Properness makes their topology statement work.
That statement does not supply root-indexed cylindrical transport between
two numerical samples in the same kinematic chamber.

[Caron-Huot–Correia–Giroux, *Recursive Landau Analysis*, §2](https://arxiv.org/html/2406.05241)
use unitarity cuts to derive kinematic singularities recursively from
subgraphs. This supports bounded subgraph proposals and explains why external
invariants deserve separate treatment. It is not a proof that their resulting
alphabet is a sufficient cylindrical projection set for a signed parametric
integrand. The earlier [SOFIA review](LITERATURE_REGIONS_SOFIA.md) and
[tropical review](LITERATURE_TROPICAL_SAMPLING.md) remain complementary:
asymptotic boundary scalings supply order features, while the present proposal
uses finite-kinematics threshold chambers and exact network identities.

A simple algebraic distinction is instructive: for
\(f=(x-1)^2-y-k+1\), the full hypersurface is smooth because
\(\partial_y f=-1\). Nevertheless, eliminating x requires the discriminant
\(4(y+k-1)\), and the repeated x root is the interior point x=1 when
y=1−k>0. Absence of a singularity of the full hypersurface does not remove
this cylindrical boundary.

Likewise, CFF cross-free compatibility describes denominator products and
their residues. It does not prove that roots belonging to two projected
polynomials can never coincide. Never delete a resultant solely because two
cuts cross or do not co-occur in one CFF term.

## Native implementation and interchange audit

The inspected native revision is GammaLoop/FeynKit
[`259df8790f27b8d3ef32778cd7195942691b4ef0`](https://github.com/alphal00p/gammaloop/tree/259df8790f27b8d3ef32778cd7195942691b4ef0),
the revision pinned by the present FastSecDec Cargo lockfile. The inspected
files are clean at that revision.

| Native path | Reusable information and restriction |
|---|---|
| `crates/feynkit-cff/src/generation.rs` | `FeynmanDiagramCffExt::build_cff`, `CffGenerator`, `CffResult`, and an explicit `with_max_orientations` bound. The default bound is effectively unlimited; a diagnostic importer must set one. |
| `crates/feynkit-cff/src/surface.rs` | `EnergySurface` stores original edge IDs, an integer external shift, and the vertex set. `SurfaceCache` separates E and H surfaces. Do not apply positive-energy E-surface formulas indiscriminately to H surfaces. |
| `crates/gammalooprs/src/cff/esurface.rs`, `classify_existence` and `classify_invariant_margin` | The full-space implementation forms `shift² − spatial_shift² − mass_sum²` and distinguishes `Existing`, `Pinched`, and `NonExisting`. Its floating-point tolerance bands and subtraction-specific rejection rules are not exact polynomial certificates. |
| FastSecDec `docs/NO_DEFORMATION.md`, revision `99f96a5623eb5059f83c340833fafc2858498966` | Native graph U/F, stable denominator-to-edge-to-parameter order, exact kinematics, routing, external Gram data, and explicit `kinematic`/`feynman` roles are already exported. |
| symGCAD `src/fastsecdec.rs` | Checks sparse polynomial/expression agreement, substitutions, roles and companion problem. Graph/routing metadata remains provenance; graph derivation is **not** thereby verified. New graph certificates must reconstruct their identities independently. |

The [HEPKit CffResult interface](https://hepkit.org/docs/hepkit/CffResult.html)
also exposes surfaces, orientations and denominator products. A data-only
sidecar can therefore avoid linking a numerical threshold subtraction engine
into symGCAD. Bind it to the original graph, U/F, mass assignments, exact Gram
rules and full variable map. Validate edge identities and contractions; reject
unmatched metadata. A bounded partial CFF inventory may supply search features,
but must never be described as a complete threshold classification.

For reproducibility, SHA-256 values of the three native files above are,
respectively:

```text
7a0bbb53011868a2c21c39b82d3499f24a52f43fd0f122a119c5424f340b642b
decb14275b1549d8d066e12c9eda946be93b512bdc7958f8206c041d32d51b9b
c326b8ec2f537e38724ff353cb53b2e759189987936ca4596f809e0e07376a27
```

## A projection strategy that distinguishes the two kinds of variables

Let k denote external invariants and x positive Feynman parameters. Keep k
outermost in lifting, so x is projected first; respect any explicit original
fixed order. Merely imposing this grouping is already supported and is not a
new optimization. The proposed change is **using certified chamber assumptions
during x projection**, before large unconditional projection sets are built.

1. Extract a bounded set of exact kinematic polynomials: channel threshold
   candidates, original domain boundaries, coefficient linear forms of F,
   and endpoint/leading coefficients proposed by low-degree reductions.
   Deduplicate by exact algebra. Do not compute a full CFF sum when a bounded
   connected-cut enumeration supplies the same proposed channels more cheaply.
2. Build a small outer CAD for these polynomials, intersected with the original
   domain. Keep every nonempty chamber. Record all inserted seams; an excluded
   seam belongs to the explicit exceptional set, not to an adjacent open cell.
3. Within each chamber, retry exact coefficient-sign, quadratic, Descartes and
   positive-pair certificates. Only a proven sign on the **whole chamber** can
   change an obligation. A physical channel label or its rational sample is
   insufficient. Any unresolved x obligation uses the current exact fallback.
4. Let local projection add missing kinematic guards and refine the affected
   chambers. Rebuild invalidated proof dependencies and root ordinals. Initial
   threshold cuts are proposals, not a claim that refinement is finished.
5. Cache algebra by canonical operands; cache chamber-dependent certificates
   by their actual assumptions, variable order and transformation. Reuse a
   proof in another chamber only by an independently checked implication.

For the current symbolic ZZ corpus, the input is **s>4, t unrestricted**,
with mt²=1 and mZ²=5/18. It is not the physical 2→2 strip. CFF real-energy,
crossing, Gram-sign and angular restrictions cannot narrow this input. Any
physical/nonphysical partition must retain its nonphysical complements, even
if only the physical chamber receives a useful new certificate. The existing
fixed A446 graph provides a source for additional symbolic experiments, not
authorization to change those mathematical problems.

For the authenticated A446 graph, the native convention is
u=2*mz2−s−t. Re-inserting s−4 into a domain that already requires s>4 offers
no new partition. The potential value is conditional information about the
other channels and polynomial coefficient signs. In the physical 2→2 strip,
t and u are negative; outside it they need not be. The earlier exact
[guide audit](ZZ_RAYLEIGH_GUIDE_AUDIT.md) already refutes global positive-guide
hypotheses on all 39 tested linear axes of the 27 unresolved symbolic inputs.
Any chamber proposal should first test whether it actually avoids those
counterexamples, while retaining the chambers containing them.

When masses are square roots of rational squared masses, threshold candidates
may be algebraic. A polynomial obtained by eliminating the square roots also
contains conjugate branches, including pseudo-thresholds. Retain it as an
over-refining proposal or attach exact positive-root selectors; never choose
a branch with floating-point rounding. Rational conservative flow capacities
below provide an initial certificate path needing no algebraic coefficient
extension.

Splitting every possible threshold up front can multiply work. Prefer a
bounded portfolio: no split; coefficient-sign split; threshold split; and
combined adaptive split. Measure total preprocessing, all chambers, exact
verification, and final common-refinement size. An easy chamber is not a
completed original problem. Under a fixed original order, use one legal x
order throughout; chamber-specific permutations require a separately verified
common cylindrical refinement before claiming ordinary CAD output.

### Cheap order priors

For each edge/Feynman parameter, record how many currently possible threshold
cuts contain it, how often it lies in compatible nested cut families, and the
frontier size under contraction. Combine these with actual polynomial degree,
support and coefficient growth as extra MCTS/portfolio seeds. Compare both
elimination directions. A channel proved closed may receive less heuristic
weight, but its associated polynomials stay unless independently certified.
No graph name, integral-family ID, or fixture hash belongs in the policy.

An affine change of kinematic channel basis can also reduce coefficient
complexity because F is affine in independent scalar products and squared
masses before nonlinear replacements. Check that property on the actual input.
Use an invertible rational triangular change when original cylindrical export
is required; retain the exact inverse and domain. An arbitrary crossing or
one-way specialization is not an invertible coordinate change.

### Convexity supplies another exact chamber certificate

This is a direct consequence of verified affine dependence, not a CFF theorem.
If \(F(x;k)=F_0(x)+\sum_j k_jF_j(x)\) and F is strictly positive for every
x>0 at every vertex of a rational kinematic polytope, it is strictly positive
throughout that polytope: write k as a convex combination of its vertices.
For an unbounded polyhedron, also certify each recession-direction polynomial
\(\sum_j d_jF_j(x)\ge0\). Exact vertex/ray decomposition and the per-vertex
positivity proofs are required. Fixed-point sample signs do not suffice.
The finite vertex/basepoint set must be nonempty, and its Minkowski–Weyl
representation must cover the entire proposed polyhedron exactly. A domain
without vertices cannot pass by vacuous quantification: provide explicit
basepoints and coverage, or decline. Represent lineality by both signs of its
direction and check both recession premises, forcing the associated direction
polynomial to vanish on the domain, or decline that extension.

The same reasoning can certify coefficients of a quadratic in one Feynman
axis even when F itself changes sign. This is potentially more useful above
threshold: it can enable a positive-root certificate while preserving F=0.
Generated projection polynomials need not remain affine in k, so the rule
must check its algebraic hypothesis individually.

## A broad exact graph-flow positivity certificate

The following is our proposed certificate construction, derived from standard
electrical-network identities. It is not a performance result or a claim of
new physics. [Bogner–Weinzierl, *Feynman graph polynomials*](https://arxiv.org/html/1002.3458)
give the spanning-tree/forest and determinant conventions. The minimum-energy
characterization is Thomson's principle; see
[Aldous–Fill, Proposition 3.39](https://www.stat.berkeley.edu/~aldous/RWG/Book_Ralph/Ch3.S7.html)
or [Doyle–Snell](https://arxiv.org/abs/math/0001057).

### Arbitrary connected two-point graphs

Assume a connected graph with two distinguished external vertices, real
nonnegative squared masses, x_e>0, and exact reconstructed convention

\[
F=U L-sV,\qquad L=\sum_e m_e^2x_e,\qquad U>0.
\]

The graph has arbitrary loops and internal structure. V/U is its effective
resistance when x_e are edge resistances. For any rational unit flow f from
the first external vertex to the second,

\[
\frac VU=\min_{\text{unit flows }j}\sum_e x_e j_e^2
\le\sum_e x_e f_e^2.
\]

Consequently, for s≥0,

\[
F\ge U\sum_e x_e(m_e^2-sf_e^2).
\]

If every coefficient on the right is nonnegative throughout a kinematic
chamber, and at least one is strictly positive throughout it, this proves
strict F>0 for all positive x. For s<0 the original positive tree/forest
expansions provide a separate simpler certificate. No sampled loop momentum
or sampled Feynman point enters this argument.

A useful proposal generator chooses rational capacities c_e≥0 satisfying
\(c_e^2\le m_e^2\). An exact positive source–sink flow of value C gives a unit
flow with \(|f_e|\le c_e/C\). Thus 0≤s<C² and L>0 imply strict positivity.
If the masses themselves are rational, max-flow/min-cut with capacities m_e
gives C equal to the minimum cut mass. For irrational masses, rational lower
bounds give a conservative chamber entirely within the rational coefficient
engine; no rounded square root is a certificate.

The checker need not trust optimality, or even the discovery algorithm: a
feasible rational flow and exact inequalities suffice. Maximum flow only
improves the proposed chamber. A zero flow value, all-massless L=0, or equality
without a strict witness causes decline. The theorem does not discard zeros
at or above threshold. Bridges are allowed in the network identity; vacuum
components, disconnected inputs, or graph reductions need explicit handling.

### Polynomial replay, without trusting a graph label

Take a full-rank fundamental cycle matrix C_g and X=diag(x). Define

\[
A=C_g^T X C_g,\quad b=C_g^T Xf.
\]

For x>0, A is positive definite; with this integral cycle-basis convention
\(\det A=U\). Completing the weighted square gives the **polynomial identity**

\[
F=U\sum_e x_e(m_e^2-sf_e^2)+s\,b^T\operatorname{adj}(A)b.
\]

The last term is nonnegative for s≥0. A certificate must check, using exact
Symbolica operations:

- incidence, edge-to-parameter bijection, unit-flow conservation, cycle rank,
  and that the proposed cycle columns span the incidence kernel;
- the full signed identities U=det(A) and the displayed F identity in the
  original map, including mass assignments and substitutions;
- x positivity, s≥0 and each coefficient slack on the whole chamber;
- a strict slack witness, or a separately proved nonzero positive term.

An LDLᵀ/principal-minor positivity witness can replay the quadratic form if
the checker does not admit the Gram-matrix theorem directly. Clear only
denominators proved positive. For a tree, use det(empty)=1 and zero quadratic
correction. Current graph-import validation alone establishes none of these
new identities. A source-side proof may be transported through later sector
maps only after verifying their exact positive-domain coverage and F identity.

### The banana identity as the smallest visible example

For n parallel edges between the two external vertices, let M=Σm_i. Direct
expansion gives

\[
F=(M^2-s)\prod_i x_i+
\sum_{i<j}\left(\prod_{k\ne i,j}x_k\right)(m_ix_i-m_jx_j)^2.
\]

For positive masses this is strictly positive below M². At M² it has an
interior zero ray m_i x_i=constant; above M² the ray has negative F. The
equal-mass bubble at s=4 has F=(x1−x2)², so dropping its same-sign seam would
be wrong. For n>2 the geometry need not be described as exactly two sheets.
The rational-flow certificate avoids adjoining irrational m_i merely to use
this square representation.

### A conservative multi-leg extension

This is a separate proposed experiment, not required for the two-point proof.
Let G(k) be the exact external Gram matrix in an independent momentum basis,
and R a rational routing matrix. Define

\[
Q=R^T X R-R^T X C_g A^{-1}C_g^T X R\succeq0,
\qquad F/U=L-\operatorname{tr}(GQ).
\]

Suppose a chamber admits an exact matrix majorant
\(vv^T-G\succeq0\), with v rational (or rational functions with certified
denominators), and a rational flow h with the same vertex divergence as Rv and
\(h_e^2\le m_e^2\). Then
\(\operatorname{tr}(GQ)\le v^TQv\le\sum_e x_e h_e^2\).
A strict coefficient slack proves F>0 as before. Check every matrix identity,
Gram majorant, flow condition and chamber premise exactly. This sufficient
condition can fail on large physical and nonphysical regions; retain them.
Several positive directions permit a higher-rank majorant and corresponding
sum-of-squared-flow capacities, but discovery becomes more expensive.

Unlike a physical-region guess, a verified Gram inequality is an algebraic
domain certificate. Even so, above-threshold ZZ regions may admit no useful
global F-positivity proof. Their gain is more likely to come from conditional
coefficient/root certificates and improved projection order.

## Ranked experiments and acceptance

| Priority | Experiment | Broad target and decisive measurement |
|---|---|---|
| 1 | Export bounded CFF cut metadata and generate role-aware chamber/order candidates | Multi-leg graphs with few external invariants. Measure preprocessing plus all original chambers against the same fixed-order baseline; threshold count and completed proof reuse explain any gain. |
| 2 | Exact two-point rational-flow certificate and banana regression family | Massive self energies, arbitrary two-terminal networks and certified source polynomials before sector maps. Compare old positivity kernels, below/at/above threshold, unequal rational masses, massless bottlenecks, bridges and graph relabellings. |
| 3 | Adaptive kinematic chamber assumptions for existing positive-root certificates | Inputs whose quadratic/Descartes premises change across a few external surfaces. Preserve all complements and compare exact common refinements, not cell counts alone. |
| 4 | Affine kinematic convex-hull/ray certificates | Sign or coefficient-sign regions with rational polyhedral covers. Include an exact negative interior counterexample whenever proposed hypotheses fail. |
| 5 | Multi-leg Gram-majorant/flow proposals | Graphs with a simple positive-energy direction and manageable Gram constraints. Count certificate discovery cost and unsuccessful attempts. |
| 6 | Landau-stratified transport or a smaller conditional projection operator | Potentially large gains, but requires new delineability/transport proofs, boundary strata and full fallback. Do not start by removing guards from a purported complete physical alphabet. |

Use FastSecDec's existing native bubble and sunrise DOT inputs first, extend
them by mass changes and general connected two-point graph families, then
test multi-leg graphs including the existing symbolic A446 graph. Graph
identities, not example names, decide admission. Original ZZ fixtures lacking
an authenticated graph binding remain ordinary polynomial problems.

Keep exact root isolation, coincident-root refinement, all interior F=0
boundaries and original-domain coverage as acceptance criteria. Rootless
chambers need strict nonvanishing evidence. A closed physical channel, a
Landau alphabet, a finite CFF truncation, or a sampled common sign is never
that evidence by itself. Supplemental randomized points remain useful checks
outside benchmark timings, not a substitute for these obligations.
