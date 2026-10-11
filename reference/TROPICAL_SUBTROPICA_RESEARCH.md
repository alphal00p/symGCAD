# SubTropica and tropical structure for exact GCAD

Research follow-up, 11 October 2026. **This review led to bounded joint scale
and ordering experiments; a further lead is exact tropical magnitude gaps
before positive-root isolation.** Tropical geometry can guide searches and
suggest coordinates, but does not certify away the interior zero sets of the
remaining problems. The ordering trials remain incomplete; a bounded exact
root-gap screen found only outer gaps on the saved degree-1478 operand. No new
corpus completion or measured GCAD speedup follows.

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

The [first exact prefix experiment](LINEAR_PREFIX_RESEARCH.md) now implements
this bounded proposal stage. Across three symbolic ZZ families, all 24
two-stage candidates completed in 3.945 seconds of combined controller time.
Different elimination pairs produced substantially different factor growth;
the two tested anchors in each family tied on the best measured prefix costs.
These results rank candidates, not completed decompositions. The experiment
retains every ordinary projection obligation and does not import Fubini-set
intersection into CAD.

A subsequent three-stage probe completes the remaining linear block for
B471 and D375: eight candidates take 2.638 seconds of combined controller
time and yield one exactly checked 11-factor, 112-term basis per family.
Full original solves using the three-axis prefix and a static ranking of the
remaining axes still time out at 60 seconds. Thus this connection to
SubTropica has produced executable ordering proposals, but no new complete
decomposition or established solve-time gain.

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

After the negative global-orthant circuit screen, a
[conditional-certificate follow-up](TROPICAL_CONDITIONAL_CERTIFICATES.md)
derives an exact AM–GM rule that incorporates independently proved monomial
domain inequalities. Its strongest prospective use is in bounded sector
domains; no benefit on the unrestricted symbolic corpus is established.

Regressions should include the `P_s` example, exceptional coefficient
cancellation, permuted edge labels, mismatched mass/Gram metadata, an
inadmissible anchor, a fixed incompatible order, and the saved A446 interior
critical point. A failed/aborted proposal must return control to ordinary
GCAD and must never authorize skipping a solution region. SubTropica's
numerical verification and any tropical sampling remain supplemental evidence.

## Follow-up: sparse structure and certified tropical root gaps

A further primary-source review found no CAD-safe rule for omitting pairs
from SubTropica's compatibility intersections. Those sets track integration
singularities, not all real cylindrical boundary intersections. Likewise,
[treewidth-based CAD](https://arxiv.org/html/2601.00312v1) requires sparsity
in the polynomial variable-dependency graph; a small separator in the original
Feynman graph does not imply that property after expansion and projection.

The complete saved three-stage B471/x6 and D375/x5 bases each have 11 factors
and 112 terms. In each basis, one polynomial involves all five remaining
active variables, so its dependency graph is already K5. Each basis has only
one binomial, `s-4`, with no new coupled monomial premise. This is a support
inventory of the authenticated arrays, not new symbolic algebra. The basis
hashes and exact projection provenance are in
[the linear-prefix experiment](LINEAR_PREFIX_RESEARCH.md). Thus neither a
sparse-variable orderer nor conditional binomial certificates has an immediate
new opportunity on these particular reduced bases.

Conditional circuits also need an architectural extension: `GuardDomain`
currently authenticates original coordinate-affine bounds and keys removals
by `(level, guard)`. Signs of projected binomials would need explicit
ancestor-cell evidence, premise dependencies, domain-sensitive cache keys
and independent replay. A positive sample cannot supply that evidence.
[Borinsky–Fraaije's newer sampling paper](https://arxiv.org/html/2504.09613v2)
uses an effectively Euclidean positive measure, with a separate residual
kernel in its loop-tree-duality applications. It supplies no absence
certificate for the signed Minkowski F=0 sheets.

**A distinct research lead is exact magnitude gaps before positive-root
isolation.** In the pinned Symbolica source, Newton-polygon radii already seed
approximate complex Aberth iteration. The certified real VAS path instead
uses local-max bounds, reciprocal lower bounds, rescaling and continued
fractions; the inspected implementation has no preprocessing for multiple
internal magnitude gaps. [Tropical/Pellet localization](https://repository.essex.ac.uk/15324/1/14096637x.pdf)
provides precedent for locating root moduli, without identifying real signs.

The following conservative certificate is our proposed exact rule, not a
production optimization. For `p(x)=sum_i a_i*x^i`, accept `0<r<R` only when
the same index k satisfies both strict rational inequalities

```text
|a_k|*r^k > sum_(i != k) |a_i|*r^i,
|a_k|*R^k > sum_(i != k) |a_i|*R^i.
```

After division by `|a_k|*x^k`, the sum of the other absolute terms is convex
in `log(x)`. It stays below one throughout the closed interval, proving the
whole gap has no zero. Approximate hulls or coefficient bit lengths may
propose dyadic endpoints; only these exact inequalities may accept them.

For integer coefficients and `x=2^e`, all comparisons reduce to integer
shifts. Preflight the intermediate bit bound
`max_i bits(a_i) + |e|*degree + ceil(log2(degree+1)) + 1`, then bound candidate
count, exponents and cumulative shifted-bit work. Stream the sums and decline
over-budget proposals. Any denominator clearing must also be bounded.

Every complementary positive interval still requires certified isolation,
with exact endpoint checks, square-free/multiplicity handling and complete
coverage of `(0,infinity)`. Rational Möbius band transforms need checked
signed reconstruction. Tropical edge lengths are not positive-root counts.
Positive ordinals follow the exact sorted positive inventory; full-real
indices additionally need the exact nonpositive-root offset, including zero
when present, under the existing multiplicity/index convention. Existing
shared-root comparison obligations remain unchanged.

For example, `1024*x^2-1048577*x+1024` has positive roots `2^-10` and `2^10`,
and its middle term certifies the gap `[1/2,2]`. Reversing that coefficient's
sign preserves tropical magnitudes but removes positive roots, showing why
root counts remain essential. These are elementary identities, not executed
performance tests.

### Executed bounded feasibility screen

The saved degree-1478 A377 block has 1,479 integer coefficients totaling
21,321,388 bits. An artifact-only Rust helper uses ordinary Symbolica exact
integers and rationals, with no factorization, root isolation or band
expansion. Its two-root and reversed-sign toy controls pass. An initial
1-GiB compiler limit cannot map the dependencies; a 4-GiB build retry succeeds.
Execution retains the original one-CPU, 1-GiB, 30-second ceiling.

The first screen exhausts its conservative four-billion-bit work allowance
after four of 44 proposed endpoints. Only the two outer endpoints near zero
pass; the other 40 are explicitly unexplored. This is not a negative result
for the whole proposal set.

A sibling retry changes only that work allowance to 64 billion bits and
checks **all 44 endpoints, `2^e` for e=-28 through 15**, without declines.
It takes **0.034301 s** for the screen, **0.101959 s** including parsing and
controls, and **15,360 KiB** peak RSS. Exactly four endpoints pass: e=-28,-27
with the constant term dominant, and e=14,15 with the leading term dominant.
Every other proposed endpoint fails strict dominance. The two accepted
segments are outer root-free regions. Comparisons against the source-bound
145-root positive inventory find no accepted segment with proved positive
roots on both sides; those comparisons are not new isolation or root counts.

Thus this finite dyadic grid finds **no useful internal gap**. It does not
exclude narrower gaps or other certificate families. No band-isolation
implementation follows: VAS already skips outer empty regions. These timings
measure a feasibility screen, not an isolation speedup. The experiment also
does not address Gluza33's factorization frontier or add original coverage.
The two frozen screens are under
`artifacts/research/tropical-root-gap-screen-{01,02}/`; both bind the input
hash `4fa804c83098bf885ff26f11a301b3645f4aa38d4c2eb901ee341e80a0d7a058`
and the independently checked saved positive inventory.
The complete retry's result hash is
`04749592aed2dc28b255698c549bae7d494619bc42769e14630e7387c936a8a1`.
