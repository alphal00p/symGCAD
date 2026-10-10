# Exact generalizations of the quadratic positivity rules

Research assessment, 2026-10-10. This proposes bounded sufficient certificates,
not a change to production or an additional completed original problem.

The first bounded experiment extends the existing quadratic certificate to a
small, acyclic proof tree. The A414 and physical elliptic scans found no
additional certificates, so it has not earned production promotion. The next
distinct opportunity is a higher-degree
convex-fiber rule, with an explicit positive-root selector. A bounded
monomial-square certificate is also justified by an already checked A446
coefficient. General SDP discovery or a replacement of CAD by Hermite matrices
is a later experiment, not the immediate recommendation.

Every recommendation retains exact analysis of interior boundaries. In
particular, a polynomial being nonnegative, having the same sign at samples,
or being a square does **not** permit deleting its zero set. A proposed
nonvanishing removal must prove a strict sign throughout its applicable
domain. A fiber reduction must export every original positive root, including
an even-multiplicity touching root unless its hypotheses exclude that root
by an exact proof.

## What the earlier work actually yielded

| Existing result | Exact gain | Limit that remains |
|---|---|---|
| Rational quadratic LDL | Strict definiteness of a rational quadratic form, including its augmented constant/linear representation | Already implemented in `src/domain.rs`; another constant-matrix LDL implementation would duplicate it. Semidefinite forms need separate strictness evidence. |
| Binomial/affine square plus strict residual | The physical elliptic coefficient contains `(x3-x6)^2` plus a strictly positive remainder; the affine version survives `x6=1` | This helped enable the verified physical original, but a square without a strict remainder is insufficient. |
| `QuadraticNoRealRoots` | For `p=a*z²+b*z+c`, known nonzero sign of `a` and `4ac-b²>0` prove that same strict sign for every real `z` | Production permits one quadratic proof layer only. |
| Conditional quadratic endpoint peel | `a>0`, `h>0`, `h*b-c>0` replace the unnecessarily strong global requirement `b>=0` | Both fixed A381/Ax381 originals completed and independently verified with 836 cells; unrestricted symbolic versions do not satisfy this rule everywhere. |
| Graph/Rayleigh identities | Exact guide identities produced the one-cell negative Gluza30 and three-cell negative Gluza33 decompositions; a separate positive bilinear construction solved positive Gluza30 | A square minor is nonnegative, not automatically nonzero. Positive complements require nonempty fibers and their own coverage proof. |

The physical scan examined 1,904 distinct retained factors. Of 58 eligible
multivariate candidates, the old portfolio certified seven and the one-layer
quadratic rule certified seven more; the additional 68 axis attempts took
0.0233 seconds in that diagnostic. In an actual projection, two such removals
reduced the final retained inventory from 1,159 factors/8,366 terms to
178 factors/857 terms. The first reduced-projection trial still failed during
large output construction; later negative-target restoration and verification
completed the original. See [physical evidence](PHYSICAL_ELLIPTIC_POSITIVITY.md)
and [the implemented rule](../ALGORITHM_RESEARCH.md#bounded-parameterized-quadratic-nonvanishing).

The fixed A381 control is stronger whole-case evidence: three old 60-second
timeouts per original versus three new independently verified completions,
with identical cold input/order/anchor. New median solve times were 0.185704
and 0.101736 seconds. The old measurements are censored, so no finite speedup
ratio follows. See [conditional quadratic evidence](CONDITIONAL_QUADRATIC_PEEL.md).

Several negative results should prevent repeated unproductive searches:

* On the symbolic A381/Ax381 negative-gamma piece, each quadratic axis has an
  exact original-domain fiber with `c>0,b<0` and two positive roots. No choice
  of positive `h` can make `hb-c>0` there. See
  [the exact counterfibers](SYMBOLIC_CONDITIONAL_PARTITION.md).
* The captured A446 octic at `(13/3,-1505,1,1)` defeated all 585 exact
  coefficient-feasibility systems for `(1+u)^k[p(lambda*u)^2+w*p'(lambda*u)^2]`,
  with nine prescribed positive scales, every `k=0..64`, and **all** `w>0`
  tested by rational interval intersection. This refutes that finite family
  at that fiber, not all SOS certificates. See
  [the retained screen](../ALGORITHM_RESEARCH.md#sum-of-squares-screen-for-positive-repeated-roots).
* Fourteen saved A446 conditional-Jacobian residuals have explicit negative
  points, and a later construction proves a genuine interior positive-common
  critical point. The proposed universal Jacobian nonvanishing is false.
  Increasing an SOS budget cannot prove it. The useful surviving result is a
  conditional cubic stratification, with an unresolved original-coordinate
  transport obligation. See [the superseding evidence](A446_FILTERED_JACOBIAN_RESEARCH.md).
* The squared degree-881 block of the large A377 discriminant has 35 real
  roots strictly above `s=4`. Its square multiplicity does not authorize its
  removal. See [exact factor/isolation evidence](A377_UNIVARIATE_FACTORIZATION.md).

## Ranked next experiments

| Rank | Proposed extension | Why it is concrete | Promotion gate |
|---|---|---|---|
| 1 | Depth-two/three quadratic proof trees | Reuses the existing exact identity and certificate leaves; a real implementation restriction currently forbids nesting | At least one new authentic retained factor, cheap full replay, then matched whole-input ablation |
| 2 | Higher-degree convex endpoint theorem | Can certify a whole 0/1 positive-root inventory without a full discriminant | Authentic original-order operand, checked derivative certificate, new positive-root export/replay |
| 3 | Bounded monomial squares plus strict residual, optionally after a positive multiplier | Existing A446 `H3` coefficient already has an exact degree-four example | Show a candidate the current portfolio actually declines; measure reconstruction and guard growth |
| 4 | Constraint-weighted graph identities | The A446 `B>0` proof on `C0>=0` is already exact but needs a coupled premise | Explicit original-domain partition and common cylindrical refinement, including its complement |
| 5 | Small critical-value/Hermite certificate | Can distinguish actual positive stationary zeros from irrelevant negative critical points | Degree-four or similarly small example; fewer/cheaper obligations than ordinary projection |

These ranks distinguish prospective leverage from measured success. The
research-only recursive scanner completed both authentic scans:

| Retained source | Unique outputs | Eligible / attempted | Already certified | Additional at depths 0–3 | Internal worker time |
|---|---:|---:|---:|---:|---:|
| A414, 212 completed records | 198 | 175 / 175 | 2 | 0 | 0.556432079 s |
| Physical elliptic, 2,655 factor records | 1,904 | 184 / 184 | 76 | 0 | 0.597247610 s |

The scanner binds exact source/output hashes and target-free domains. The
A414 domain has `s>4`, unrestricted `t` and five positive ratios. Every
eligible candidate was processed within the envelope: at most 128 terms,
depth three, the first four candidate quadratic axes per node, 128 nodes and
one million work units per trial. Neither scan found a new certificate.
Exhaustiveness applies to this bounded portfolio, not all quadratic proof
trees. This weighs against immediate production promotion for these inputs;
it does not refute other axis choices, deeper proofs or other domains. The
synthetic nested example succeeds at depth two/three and fails at depth one,
so the extension is algebraically broader without a measured corpus gain.

Retained reports:

* `artifacts/research/recursive-quadratic-a414-02/scan.json`, SHA-256
  `ba58651d94563cd70e800c4df7583b80b86dc45f6cb933be7543d1637e8b2a1c`.
* `artifacts/research/recursive-quadratic-physical-01/scan.json`, SHA-256
  `ba5ac907cc48b24456b30dbcca6abe4227516465ba44cd7de42d11253f4c51c9`.

These times cover the diagnostic worker, not whole-case solving; process
overheads and RSS remain in its separate supervisor records. The earlier
failed A414 checksum-format attempt is preserved and supplies no result.
The physical snapshot uses its checked `x6=1` endpoint domain and excludes
the original target inequality. Its eligibility rules and current baseline
portfolio differ from the historical seven-additional-certificates scan
above; their raw success counts are not a matched regression comparison.

## 1. Nested quadratic certificates and parameter-dependent Schur complements

The exact identity behind the existing rule is

```text
4*a*p = (2*a*z+b)^2 + (4*a*c-b^2).
```

If both children themselves admit quadratic certificates in earlier or other
remaining variables, the same proof can be repeated. There is no new
positivity theorem to assume. Each chosen axis disappears from both child
polynomials. Discovery should retain all current degree, term, coefficient
height and multiplication bounds, plus a **shared** total budget across
failed attempts as well as accepted nodes. A depth-three accepted binary
tree has at most 15 quadratic nodes, but that alone does not bound failed
candidate search. A separate attempt/work counter is necessary.

Replay must recompute `a,b,c,4ac-b²`, check exact signed reconstruction,
verify both children over the same independently derived target-free box,
and enforce tree depth/node/size limits independently of discovery. No
assumption may be introduced by the parent polynomial's requested sign.
Zero leading coefficient, zero discriminant margin and a merely
nonnegative child remain failures. Historical one-layer proofs retain
their current meaning. A parent-local memoization may share identical child
obligations, but its keys need the exact polynomial, requested sign, domain
and remaining proof budget; it is not proof authority.

A related two-coordinate extension is a parameter-dependent quadratic block

```text
p(x;u) = x^T A(u) x + 2*b(u)^T x + c(u).
```

Exact symmetric positive-definite `A(u)` and
`c*det(A)-b^T adj(A)b>0` imply `p>0` on the whole real
block. This follows by completing the square; denominators can be avoided
using `delta=det(A)`:

```text
delta^2*p = (delta*x+adj(A)*b)^T A (delta*x+adj(A)*b)
            + delta*(c*delta-b^T adj(A)*b).
```

For a two-by-two block, strict Sylvester pivots have small explicit forms.
Positive determinant alone is insufficient, and a semidefinite square can
vanish away from the origin: `(x-y)^2=0` on `x=y>0`. Any pivot division
requires a checked nonzero denominator; the displayed fraction-free
identity avoids an implicit division but still requires strict definiteness
and residual signs.
Unlike the existing rational LDL rule, its matrix entries depend on other
coordinates. However, nested scalar quadratic proofs should be measured
first: expanding adjugates or all principal minors may introduce more work
than they remove. These identities are direct algebraic proposals, not a
claim of a new complete decision method for positivity.

## 2. An endpoint theorem beyond degree two

Let `u` range over the complete target-free parent domain and let

```text
q(u,z) = c(u) + b(u)*z + ... + a_d(u)*z^d,   z>0.
```

Assume a fixed degree `d>=2`, `a_d>0`, and `q_zz>=0` on the **entire** stated
positive ray. Suppose a polynomial or denominator-cleared rational multiplier
independent of `z` has exact certificates `h>0` and `h*b-c>0` on the parent.
Then the following proof is elementary and does not depend on a sampled root
count:

1. If `c>=0`, then `b>0`. Convexity gives `q(z)>=c+b*z>0` for all `z>0`.
2. If `c<0`, positive leading coefficient gives `q(z)->+infinity`; hence a
   positive root exists. At any positive root `r`, convexity gives
   `q_z(r)>=(q(r)-q(0))/r=-c/r>0`. After that crossing the derivative cannot
   decrease, so a second positive root is impossible. The root is simple.

Thus the negative fiber is precisely `(0,r)` when `c<0`, and empty otherwise;
the positive fiber is `(r,infinity)` or the whole positive ray respectively.
The implicit function theorem and uniqueness give a continuous, locally
analytic root on each `c<0` parent cell. The weak derivative inequality is
used only to prove convexity; it is **not** used as a nonvanishing proof for
`q` or `q_zz`. All original positive zeros are represented.

An especially cheap sufficient subclass certifies every coefficient from
`z²` upward nonnegative, with a strictly positive leading coefficient. For
an initial implementation, each such coefficient can be either identically
zero or proved strictly positive by existing certificates. This avoids
inventing a weak-sign certificate API. More general `q_zz>=0` evidence can be
an explicitly reconstructed sum of nonnegative weighted squares; existing
strict `DomainCertificate` evidence must not be relabeled as weak evidence.

The mathematical use of a nonnegative second derivative is conventional;
SOS representations are a tractable sufficient convexity test, not a
necessary one in general. Ahmadi and Parrilo give exact counterexamples to
the converse in [*A convex polynomial that is not sos-convex* (2009 preprint,
2012 publication), introduction and construction](https://arxiv.org/abs/0903.1287).
Here only convexity along the selected fiber is needed, not joint convexity
in all parameters and coordinates.

A small synthetic regression showing strictly more scope than positive
upper coefficients is

```text
q = z^4-z^3+2*z^2+(c+1)*z+c,
q_zz = 12*z^2-6*z+4,    disc(q_zz)=-156,
h=1,                   h*b-c=1.
```

The existing quadratic nonvanishing method proves the derivative positive,
although `q` has a negative cubic coefficient. This is a proposed exact
regression, not a measured unresolved corpus reduction.

### Root labels and strictness are essential

The current quadratic restorer uses full-real ordinal 1. Merely allowing
higher degree in that code would be wrong. Consider the independently
reviewed symbolic counterexample

```text
q_t(z)=(z-1)*((z+1)^2+t),   -1/2<t<1/2,
h=1/4,   q_zz=6*z+2>0,    h*b-c=(3+5*t)/4>0.
```

Its unique positive root is always `1`. Its full-real **distinct** index is
2 for `t<0`, 1 at `t=0`, and 0 for `t>0`. A typed positive ordinal 0 remains
valid. A new restorer/checker must use that selector, independently count the
original polynomial's positive roots, and preserve the original variable
order and all base obligations. Negative-root collisions do not change the
positive inventory proved by this theorem.

Adversarial cases for independent replay include:

* `(z-1)(z-2)`: convex with positive leading coefficient, but `c>0` and two
  positive roots. The endpoint-slope implication is indispensable.
* `z²-z`: endpoint zero does not imply positivity on the whole ray or the
  absence of an interior positive root.
* `t²*z²-z-2`: weak leading positivity allows a root to escape to infinity
  at `t=0`, despite convexity and `hb-c=1` for `h=1`.
* `(z-1)²+t²`: nonnegative everywhere, but a double **interior** positive
  zero exists at `t=0`. Neither an SOS decomposition nor `D<=0` permits
  omission of that seam.

Authentic higher-degree opportunity already exists in the A446 research:
the 52-term cubic `H3(r)` has strictly positive coefficients of `r,r²,r³`,
with the linear coefficient proved by
`q1=5*w²*(v-z)²+R1`, where `R1` has 22 positive monomials. It has exactly
the endpoint-based 0/1 positive-root behavior above. However, it is expressed
in a nontriangular chart; its proof does not yet yield an original-order
peel. In contrast, the symbolic A381 counterfibers already rule out a global
endpoint-only reduction on their full domain, even with this extension.

## 3. Generalize the square, while keeping a strict residual

A small typed certificate can reconstruct

```text
D(u)*sign*p = R(u) + sum_i w_i(u)*g_i(u)^2,
```

with `D>0`, `R>0`, and each `w_i>=0` certified on the original domain.
The first finite portfolio should use positive rational weights times
monomials in verified positive coordinates, a few binomial monomials `g_i`,
and existing strict residual certificates. It need not introduce an SDP
solver. The exact A446 `q1` identity above is an authentic higher-degree
example. Before counting it as a new gain, compare it against the complete
current certificate portfolio, which may already prove it through another
axis or a positive multiplier.

This extends the present single coordinate-binomial/affine square, whose
weight is rational. Independent replay needs exact full reconstruction,
all weight/denominator signs, coordinate-domain binding and strictness of
the residual. An omitted residual or one that vanishes in the domain must
not certify nonvanishing.

One structured quartic proposal is especially simple. For
`p=a*z^4+b*z^3+c*z²+d*z+e`, verify

```text
4*a*p = (2*a*z²+b*z)^2
        + (4*a*c-b²)*z² + 4*a*d*z + 4*a*e.
```

If `a>0` and the remaining quadratic is strictly positive on the required
domain, existing quadratic leaves suffice. A finite menu may include a
constant in the squared quadratic, but there should be no unrestricted
coefficient search before this cheap form finds an authentic success.

For a later discovery engine, a bounded monomial Gram matrix provides a
systematic language: `p-R=m^T Q m`, exact rational `Q=LDL^T`, and nonnegative
diagonal pivots. Diagonally dominant and scaled diagonally dominant Gram
matrices correspond to restricted monomial/binomial-square decompositions;
they admit LP/SOCP searches. These are sufficient cones, not all
nonnegative polynomials. See Ahmadi–Majumdar, [*DSOS and SDSOS Optimization*
(2019), definitions 3.1–3.3 and theorems 3.4, 3.6, 3.9](https://arxiv.org/pdf/1706.02586).
An orthant-specific `Q=S+N`, with `S` positive semidefinite and `N` entrywise
nonnegative, is another sufficient copositivity certificate; strictness
still needs an explicit positive contribution.

Any floating-point discovery would be only a proposal generator. Peyrl and
Parrilo describe rational recovery with exact Gram identities, and its
strict-feasibility limitations, in [*Computing sum of squares decompositions
with rational coefficients* (2008), §§2.2 and 3](https://www.mit.edu/~parrilo/pubs/files/PeyrlParrilo-ComputingSumOfSquaresDecompositionsWithRationalCoefficients.pdf).
Numerical eigenvalues, approximate residuals and failed recovery cannot
authorize a projection removal. Bounded exact Symbolica discovery remains
the preferred first experiment here.

## 4. Use graph identities and explicit domain premises

The successful graph reductions suggest generating candidate multipliers
and squares from minors, rather than guessing many dense polynomials.
Dodgson identities are exact relations among graph minors; their spanning
forest versions extend beyond three marked vertices. See Vlasev–Yeats,
[*A four-vertex, quadratic, spanning forest polynomial identity* (2011),
equation (1) and theorem 7](https://arxiv.org/pdf/1106.2869), and
Brown–Doryn, [*Framings for graph hypersurfaces* (2013), §2.1,
equation (8), and §4.1, equation (63) and its following square identity](https://www.ihes.fr/~brown/Framings.pdf).

For this project, graph provenance and signs must still be checked against
the exact original `U,F`, masses, Gram invariants and coordinate map. A
positive Kirchhoff polynomial does not make a Minkowski `F` positive.
Massive-edge quadratic dependence also does not inherit every massless
multiaffine Rayleigh inequality automatically.

A concrete next certificate language is a bounded constraint-weighted
identity `D*p=R+sum sigma_i*g_i`, where `D,R>0`, `sigma_i>=0`, and each
`g_i>=0` is an explicit verified premise of the applicable cell/domain.
For A446, the already checked identity

```text
(l+r)*B0 = l*r*C0 + u*(x1*x7-x3*x5)^2
```

together with the checked positive remainders proves `B>0` on
`u>4,C0>=0`. This is stronger than deriving only a coordinate box and is
documented in [the A446 quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md).
But `C0>=0` is not a premise on the full original problem. The `C0<0`
complement and `C0=0` boundary cannot disappear. A useful implementation
needs a finite exhaustive original-domain partition and common ancestor
refinement, or it must remain an explicitly partial certificate experiment.

## 5. Critical-value certificates for small higher-degree fibers

For a fixed-degree polynomial with positive leading coefficient, strict
positivity of `p(0)` and of `p(r)` at **every positive real root** `r` of
`p'` proves `p>0` on the positive ray: a nonpositive interior value would
force a nonpositive attained interior minimum. This includes repeated or
stationary touching zeros, which must make the certificate fail.

A research certificate could establish the critical values by an exact
identity `D*p=R+sum w_i*g_i²+T*p'`, with the same strictness/sign rules as
above. Only the equality locus `p'=0` cancels the last term. Alternatively,
weighted Hermite signatures/Tarski queries can count the realized signs of
`(z,p(z))` on `p'=0`; every `z>0,p<=0` count must be zero. Endpoint and
degree-drop conditions remain separate. Start with quartics, whose critical
polynomial is cubic, rather than the captured high-degree A377 factors.

The precise algebraic basis is Gaillard–Safey El Din,
[*Solving parameter-dependent semi-algebraic systems* (2025 version),
theorem 2.4, corollary 3.2 and proposition 4.1](https://arxiv.org/pdf/2402.07782).
The signature equals a Tarski query; several queries recover complete sign
conditions. Parametric specialization needs its leading/denominator locus
excluded or separately handled. This proposal cannot use only nonsingular
leading-principal-minor formulas on a singular matrix: rank changes and
zero minors require valid exact case handling or fallback. The paper's
dense-open parameter classification is not automatically a complete
original-coordinate CAD or a license to discard interior boundary fibers.

Univariate half-line SOS forms provide related certificate templates:
`S0(z)+z*S1(z)` is nonnegative when both terms are sums of squares. Powers
and Reznick discuss the classical representation and interval variants in
[*Polynomials that are positive on an interval* (2000), §1](https://citeseerx.ist.psu.edu/document?doi=f516505c54e760398444313088e864c9b523baac&repid=rep1&type=pdf).
Existence for each specialized univariate polynomial does not give one
small rational formula valid over an entire parameter cell. Here a strict
remainder and all symbolic sign/denominator obligations would still be
required before nonvanishing could be accepted.

## Measurement and validation gate

The first bounded scan should use hash-bound **actual** factor outputs from
A414 and the saved physical endpoint projection, retaining the exact
original target-free domains. Record existing successes, new successes,
failed/over-budget candidates, shared search work, proof size and independent
replay time at depths one, two and three. A node/candidate count without the
final signed identity is not a success. The separate root-owned recursive
scanner supplies the negative results above; no new production
option is justified by this note alone.

For higher-degree candidates, scan original legal endpoint chains first and
small already captured projection operands second. Record which object is
being proved: strict sign, convexity, simple-positive-root inventory, or
conditional critical-value exclusion. These are different proof types.
Preserve the current fallback whenever any hypothesis, resource cap or exact
reconstruction fails. A future whole-case trial must compare the same
original constraints, coordinate order, anchor and cold/warm policy, and
must independently verify the full exported proof.

Required regressions include pure squares with interior zeros, vanishing
leading coefficients, the negative-root-collision family above, forged
child/domain/multiplier evidence, excessive depth/size, missing sign branches,
and wrong positive-root ordinals. Graph-generated proposals must also be
rejected after any mass, Gram-sign, propagator map or polynomial identity
tampering. No positive or negative `F=0` sheet is removed merely because its
factor is squared or its neighboring chambers have the same sign.

Source inspection and primary literature reading only were used for this
note. The convex theorem and its selector counterexample received a separate
read-only mathematical/implementation audit. No production code, build,
solver campaign or certificate cache was changed by this research.
