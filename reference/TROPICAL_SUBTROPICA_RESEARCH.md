# SubTropica and tropical structure for exact GCAD

Research follow-up, 11 October 2026. **The most useful new borrowing is a
bounded search that chooses a positive scale and an elimination order together,
favoring long linear projection prefixes.** Tropical geometry can guide this
search and suggest coordinates; it does not certify away the interior zero
sets of the remaining problems. No production change, new corpus completion,
or measured GCAD speedup follows from this review.

## What was inspected, and what is new

[SubTropica's site](https://subtropi.ca/) is the diagram interface to a
tropical-subtraction and hyperlogarithmic-integration package. It is distinct
from Borinsky's tropical Monte Carlo sampling. The primary package paper is
[Giroux–Mizera–Salvatori, arXiv:2604.20954v1](https://arxiv.org/html/2604.20954v1).
The inspected repository revision, obtained with `git ls-remote`, is
`cbb4d91fdf5c261e8687ba01dfb94d65498e8b73`; the source was read without
installing or executing it.

The earlier [tropical review](LITERATURE_TROPICAL_SAMPLING.md) already covers
Borinsky's subset recurrences, mass–momentum spanning conditions, graph/support
ordering seeds, matroid caveats and toric chart coverage. The
[SOFIA/regions review](LITERATURE_REGIONS_SOFIA.md) already warns about hidden
interior cancellations. Those findings are not new results of this follow-up.
The additions here are the SubTropica source audit, its order/gauge connection,
a precise proposed ablation against our existing orderers, and a small exact
threshold example linking support information to our quadratic-block work.

Primary results and their scope:

| Source | What it supplies |
|---|---|
| [SubTropica, §§3.1, 3.4.1–3.4.2](https://arxiv.org/html/2604.20954v1#S3.SS4) | Dynamic programming over Fubini reductions and order scores based on expression size. Its reported correlation concerns hyperlogarithmic integration, not CAD. Section 3.1 explicitly excludes a generic treatment of hidden interior singularities from its sign-coherent tropical framework. |
| [Pinned `SubTropica.wl`](https://github.com/SubTropica/SubTropica/blob/cbb4d91fdf5c261e8687ba01dfb94d65498e8b73/SubTropica.wl) | `STFasterFubini2` intersects proportional reduction sets across paths; `STLinearCrawlWeight` offers expression-size metrics. `stDeriveGaugeFromHomogeneousLR` chooses the last variable of a homogeneous LR order as gauge. Its `GaugeStrategy -> "Derive"` implementation has explicit projectivity/admission checks. Budget/pruning failures are distinguished from established non-reducibility. |
| [Salvatori, arXiv:2406.14606v2, §5.1 Theorems 3–4 and §6 Theorem 6](https://arxiv.org/html/2406.14606v2) | Tropical local-finiteness tests and a subtraction formula under its geometric and integrand hypotheses. The common fan refinement matters. The result is an identity between integrals/renormalized terms, not a disjoint sign decomposition of the original polynomial. |
| [Borinsky, Definition 6, Theorem 8, Algorithm 4](https://arxiv.org/html/2008.12310) | The tropical polynomial is the maximum of supported monomials. A lower comparison with the original absolute polynomial requires complete nonvanishing. Generalized-permutahedron sampling uses a subset table instead of enumerating every permutation; this does not bound CAD cost. |
| [Borinsky–Munch–Tellander, §3.3, Theorems 3.4–3.6, Conjecture 3.7, Theorem 3.9](https://arxiv.org/html/2302.08955v2#S3.SS3) | The first Symanzik support has graph-matroid structure. The analogous generic second-Symanzik support statement extends beyond Euclidean kinematics, without implying positivity. Exceptional specializations need separate treatment; the paper explicitly separates equality results, a conjecture and a containing-polytope result. |

The code mechanisms worth reading are approximately lines 17073–17190
(`STLinearCrawlWeight`), 17430–17547 (`STFasterFubini2`), 17550–17580
(joint gauge/order proposal), and 26541–26579 (projectivity gate) at the pinned
revision. Source comments are implementation evidence, not an independent
mathematical theorem. The package's current optional HyperFLINT backend is
not proposed as a symGCAD dependency: our algebra remains in Symbolica.

## Why tropical support cannot replace exact threshold analysis

Here is an exact, hand-derived regression example; no numerical evidence is
needed. On `x,y>0`, let

```text
P_s(x,y) = x^2 + (2-s)*x*y + y^2
         = (x-y)^2 + (4-s)*x*y.
```

For `s<4`, it is strictly positive. At `s=4`, it has the interior double
zero `x=y`. For `s>4`, it has two positive ratio roots

```text
x/y = (s-2 +/- sqrt(s*(s-4)))/2.
```

At `s=3,4,5` its support, coefficient-sign pattern, Newton polytope and
coefficient-free tropical approximation are all identical. The latter is
`max(x^2,x*y,y^2)=max(x^2,y^2)`. The coefficient disappears at `s=2`, while
the positive-root threshold is at `s=4`: coefficient-zero chambers alone
also miss the transition. Even recording coefficient signs on every Newton
face does not recover the relative magnitudes responsible for it.

The scale/fraction coordinates `r=x+y>0`, `a=x/(x+y) in (0,1)` give

```text
P_s = r^2 * (1-s*a*(1-a)).
```

The threshold now appears as the exact stationary value `1-s/4` at `a=1/2`.
This is the useful connection to the
[quadratic-face diagnostic](QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md): keep the
full polynomial and determine coefficient-dependent critical values. A
Newton polytope suggests scales; it does not supply these values. The same
example requires retaining an interior same-sign seam at `s=4` whenever
that parameter slice is specialized and solved.

Consequently, neither a tropical initial form, an integrated symbol alphabet,
nor a sampled Euclidean measure may substitute for the original signed F.
Subtraction terms may cancel singularities after summation or integration;
that gives no permission to merge original cells across F=0. Conversely,
failure of a linearly reducible integration order does not imply failure of
real CAD, which can use algebraic root boundaries.

## Prioritized experiments

### 1. Joint positive-scale and linear-prefix order proposals

**Recommended first; heuristic search using existing exact operations.**
Our `homogeneity.rs` already verifies scaling and restores original cells,
but requires an explicit anchor. Our Brown/T1 orderers rank one next axis;
the inexpensive MCTS seeds use original-basis features. Add an optional
bounded proposal stage over legal homogeneous anchors and short exact
projection prefixes. This is a new portfolio arm, not a new projection rule.

For each admitted anchor, use the existing homogeneity certificate to form
the reduced problem. Explore at most a small beam of legal Feynman-axis
eliminations, initially only two levels deep. Prefer the largest number of
consecutive stages where every active factor is linear in the eliminated
axis; break ties by projected term count, coefficient bits, and estimated
remaining degree growth. Preserve a small number of non-linear alternatives
so failure to find an LR prefix does not terminate GCAD search. Complete
the proposed order with an existing heuristic, then promote through the
existing killable workers. Record anchor, legal order, exact basis hashes,
predicted cost and completed-solve outcome separately.

In an all-linear stage, `a*x+b` and `c*x+d` have resultant `a*d-b*c`, while
individual leading coefficients and endpoint values are cheap. Nonetheless
include every required leading coefficient, boundary polynomial and pair
resultant, plus independent factors. This stage may still grow rapidly in
the number of factors; stop its worker at the allotted budget.

Do **not** transplant SubTropica's intersection of Fubini sets into the CAD
basis. Nor may two CAD states be merged just because the eliminated subset
is equal: their exact projected factors, domain, transformation and policy
must agree. The search cache key remains the existing mathematical state.

The scale coordinate must precede its ratios in the original lifting order,
and external kinematics remain outside the scale block. Respect fixed orders
and variable groups. A fixed anchor/order yields no competing proposal.
Unlike integration's degree-minus-dimension condition, preserving polynomial
signs under positive scaling needs the existing per-constraint homogeneity
identities; importing the integrator's admission test would be inappropriate.

Proposed first diagnostic budget: at most eight anchors/proposals, beam width
four, two exact levels, and 30 seconds total within the established search
budget. These are engineering starting points, not complexity theorems.

### 2. Use matroid and cut information to propose sum/fraction blocks

**Second priority; coordinate discovery only until restoration is proved.**
From FastSecDec's authenticated edge-to-parameter map, enumerate bounded
degree-two skeleton paths and simple graph separators. Use U's dependence
on path-length sums as a cheap filter, then substitute candidate sum and
fraction variables into the complete original U/F and every constraint.
Test joint fraction degree, coefficient support, and sizes of Hessian minors.
Prefer a block that actually yields a small quadratic or linear system.

The [existing CFF/worldline investigation](CFF_PROJECTION_NEXT_INVESTIGATION.md)
already found and exactly replayed two such charts for A446/Ax12. Tropical
and matroid data can rank candidate blocks on other graphs; the block theorem
and successful A446 replay are not new findings here. Full signed U/F replay
is necessary: a matroid alone forgets external momentum placement and masses.
Current `src/fastsecdec.rs` checks interchange identities and bindings and
reports `graph_derivation_verified=false`; a topology name is not a proof.

Rank candidate charts by the *complete* quadratic-face guard inventory,
including determinant-zero/rank obligations, or by bounded ordinary block
projection. Never rank only Hessian inertia: translating a quadratic changes
its zero set without changing its curvature. Sum/fraction maps are generally
not triangular in the original axes. Their diagnostic success requires a
mapped-cell contract or exact cylindrical restoration before original-corpus
completion can be claimed.

### 3. Parameter-aware support diagnostics, not automatic chamber splitting

**Useful supplement; do not create a global coefficient-sign CAD first.**
Write `F=sum_alpha c_alpha(k)*x^alpha` with exact kinematic coefficient
polynomials. Preserve the coefficient table, graph cut channel, and original
support. On an already certified parameter chamber, annotate coefficients
whose zero/nonzero/sign status is proved. This avoids using generic graph
support assertions after a cancellation specialization and gives ordering
features for which channels mix signs.

Use the older bounded subset valuations as a separate MCTS seed arm, not as
integration probabilities unless dimension, propagator powers and convergence
hypotheses are supplied. A tropical order prior needs no positivity claim.
Coefficient signs may suggest applying existing positivity kernels; only an
accepted exact strict-nonvanishing certificate may remove a factor.

Physical thresholds from CFF can be candidate chamber cuts. Retain all
original chambers, anomalous/ordinary interior sheets, parameter-dependent
critical values and root-order guards. The example above shows why mass
threshold data, support changes and real-root transitions are different
objects. Generating the full principal discriminant or every coefficient
chamber before projection could simply move the expensive calculation.

## Applicability and measurements

For the 27 unresolved symbolic ZZ problems, the strongest lead remains
verified coordinate structure followed by exact parameter-dependent quadratic
or linear analysis. The joint-anchor/order arm is the cheapest broad test.
For the fixed-kinematics positive Gluza33 problem there are no external
parameter chambers to organize; tropical support can only suggest a better
order/anchor or graph identity. Its captured factorization bottleneck should
continue to be measured directly. No tropical theorem predicts that it will
disappear.

Compare existing search, joint-anchor/linear-prefix proposals, and the earlier
support/matroid seed portfolio as separate arms, with solved holdouts and the
same legal orders, workers, caches and limits. Include proposal time and
memory. Report complete verified original-cell coverage, time to completion,
first non-linear stage, projection growth, coefficient bits and peak RSS.
Partial projection timings are diagnostics; they are not solve-time ratios.

Regressions should include the `P_s` example, exceptional coefficient
cancellation, permuted edge labels, mismatched mass/Gram metadata, an
inadmissible anchor, a fixed incompatible order, and the saved A446 interior
critical point. A failed/aborted proposal must return control to ordinary
GCAD and must never authorize skipping a solution region. SubTropica's
numerical verification and any tropical sampling remain supplemental evidence.
