# CFF-guided quadratic blocks for parametric projection

Research continuation, 11 October 2026. **A new exact structural reduction has
been checked on both original A446/Ax12 symbolic polynomials:** a positive-sum
and unit-cube change of Feynman coordinates makes the complete F jointly
quadratic in three coordinates. Its Hessian has small, factored determinant
identities. This is an algebraic discovery and proposal for the next solver
experiment, not a completed decomposition, speedup, or permission to omit
interior boundaries.

## What the earlier research established

The [original CFF study](CFF_KINEMATIC_PROJECTION.md) and
[connected-cut replay](CFF_ENDPOINT_COMPOSITION_DIAGNOSTIC.md) already found
the normal channels of the authenticated graph. Its 15 connected cuts yield
only 0, m, s and u; at m=5/18 the additional normal threshold is u=4, since
the problem already requires s>4. Existing tests leave t unrestricted, so
restricting to the physical scattering strip would change the problem.

Below u=4, graph-block composition led to the successful weighted-square and
quadratic-component positivity experiments. Above u=4, exact positive
parameter families give both signs of the endpoint B for every u. Therefore
further normal-threshold splitting alone cannot make that endpoint positive.
The [rational kinematic map](RATIONAL_KINEMATIC_PAIR_MAP.md) removed s from
one critical system but produced a 5,863-term degree-eleven resultant without
a smaller factor. Actual positive critical points rule out global Jacobian
nonvanishing. The [selected-root theorem](SELECTED_POSITIVE_ROOT_DELINEABILITY.md)
and [critical certificate gate](SELECTED_CRITICAL_CERTIFICATE_GATE.md) identify
the remaining proof obligations; they have not solved the complement.

This investigation changes the representation of the Feynman-coordinate
block before elimination, instead of asking the same expanded critical
resultant to become positive.

## What CFF and the literature can supply

[Capatti, *Exposing the threshold structure of loop integrals*, §§IV–V](https://arxiv.org/html/2211.09653)
organizes the energy-integrated representation by connected cuts and their
cross-free families. This makes cut channels, edge groupings and contraction
orders useful proposals. A missing simultaneous CFF denominator does not
prove that two real parametric roots cannot meet. Its threshold-intersection
results concern the represented momentum-space integrand, not arbitrary
cylindrical discriminants of F.

There is a complementary, more directly algebraic route in
[Dai–Siegel, *Worldline Green Functions for Arbitrary Feynman Diagrams*,
§§VI–VII, equations (16)–(17)](https://arxiv.org/pdf/hep-th/0608062).
Their Green function uses a skeleton period matrix Ω and
`-r/2 + vᵀ Ω⁻¹ v/2`. Ω depends on total skeleton-edge lengths, not on insertion
positions. On a fixed ordering sector, r and v are affine in those positions.
This suggests separating lengths from ordered insertion coordinates and
checking for a quadratic position block after multiplying by U. Different
ordering sectors, mass assignments and signs still require exact original
U/F replay. This construction suppresses vertices of internal degree two
and retains their external insertions; it is not a rule for arbitrary graph
contractions. Fixed unequal segment masses contribute an affine length sum,
so they do not by themselves destroy the quadratic position degree. Nonlinear
sector substitutions can destroy it and require a fresh degree check. The earlier
[Schmidt–Schubert construction, §§2–3](https://arxiv.org/pdf/hep-th/9403158)
explicitly relates one- and two-loop proper-time coordinates to ordinary
Feynman parameters. These sources motivate candidate coordinates; the
specific three-cube identities below are our checked algebraic deductions.

For example, a natural skeleton chart for the present graph fixes
`a=x1+x2+x3`, `b=x5+x4+x7`, and z=x8. Ordered positions

```text
x1=a*p, x2=a*(q-p), x3=a*(1-q),     0<p<q<1,
x5=b*r, x4=b*(v-r), x7=b*(1-v),     0<r<v<1
```

make `U=a*b+(a+b)*z` independent of all four positions. Substitution into the
displayed original formula below shows joint degree at most two in p,q,r,v.
This four-position alternative now has its own executed replay: both signed
original A446/Ax12 expressions agree exactly with that quadratic formula,
and the native graph export's U agrees with the position-independent
`a*b+(a+b)*z`. The normalized expanded F expressions have total position
degree exactly two. The map covers the original positive orthant, with
inverse `a=x1+x2+x3`, `p=x1/a`, `q=(x1+x2)/a`, and the analogous lower-path
formulas. Its absolute Jacobian `a^2*b^2>0` was also checked by expanding
the two block determinants. This chart trades fewer positive length
variables for a larger quadratic block. It remains distinct from the
three-fraction chart and does not provide original cylindrical restoration.

[Sturmfels–Telek, *Copositive geometry of Feynman integrals*, Corollary 5.9
and Theorem 6.1](https://arxiv.org/html/2504.01628) connect copositive-cone
boundaries with Landau discriminants and give a Pólya characterization under
their stated positive-mass hypotheses. They do not certify a sign-changing,
mixed-mass chamber by its physical label. Here we use exact matrix identities
and explicit domain bounds, not an assumption of copositivity.

The inspected local interfaces remain the pinned GammaLoop/FeynKit revision
`259df8790f27b8d3ef32778cd7195942691b4ef0` and FastSecDec's existing
`no_deformation` graph export. `feynkit-cff::CffOptions` supports an explicit
orientation cap; `EnergySurface` retains original edge IDs, vertex set and
external shift. FastSecDec supplies the edge-to-parameter map, exact Gram
rules and U/F. A bounded cut/coordinate sidecar should consume these fields
without generating an unbounded full CFF expression. The current importer
checks wire polynomial agreement, not a new graph-derived certificate.

## Executed positive-sum chart

Write m=5/18 and retain s>4 with u unrestricted. For A446 use
`u=5/9-s-t`; for the crossed Ax12 fixture use `u=t`. With

```text
a=x1+x2+x3, b=x5+x4+x7, z=x8,
U=a*b+(a+b)*z,
F=(a+b)*U-s*x2*x4*z
  -u*(b*x1*x3+a*x5*x7+z*(x1+x5)*(x3+x7))
  -m*(b*x2*x3+a*x4*x7+z*(x2+x4)*(x3+x7)),
```

introduce

```text
X=x1+x5,  Y=x3+x7,  R=x2+x4,
x1=X*alpha, x5=X*(1-alpha),
x3=Y*beta,  x7=Y*(1-beta),
x2=R*gamma,x4=R*(1-gamma).
```

This is a bijection from the six original positive coordinates to
`X,Y,R>0` and `0<alpha,beta,gamma<1`. The inverse fractions are
`x1/(x1+x5)`, `x3/(x3+x7)`, `x2/(x2+x4)`. Their denominators are strictly
positive. In paired coordinate order the absolute Jacobian is `X*Y*R>0`.
No coordinate boundary or F=0 sheet is deleted. These domain and Jacobian
claims follow directly from the formulas; the executed helper checks the
polynomial identities, not a general transformation-verifier interface.

Set

```text
T=X+Y+R, A=X*alpha+Y*beta+R*gamma,
C=T^2-u*X*Y-m*R*Y.
```

The complete transformed polynomial is exactly

```text
F=T*A*(T-A)
  -u*X*Y*(T*alpha*beta+A*(1-alpha-beta))
  -m*R*Y*(T*gamma*beta+A*(1-gamma-beta))
  +z*(C-s*R^2*gamma*(1-gamma)).
```

Every term has total fraction degree at most two. In particular, put
`w=(X*alpha,Y*beta,R*gamma)` and define

```text
K11=u*Y-T,        K22=u*X+m*R-T, K33=m*Y-T+s*z,
K12=-T-(u-m)*R/2, K13=-T+(u+m)*Y/2,
K23=-T+(u-m)*X/2.
```

Then

```text
F=w^T*K*w + C*(X*alpha+Y*beta+R*gamma+z)
  -s*R^2*z*gamma.
```

The fraction Hessian is `2*diag(X,Y,R)*K*diag(X,Y,R)`.
Thus its inertia is exactly that of the 3-by-3 symmetric K. The entries
and all its minors are independent of the fractions.

Both original signed fixture expressions, with their stated substitutions,
were independently expanded against the compact formula, and that formula
was expanded against the quadratic coefficient expression. The proof does
not rely on a family name or a graph-label match. The replay TOMLs declare
the investigative chamber u>4; exact zero polynomial differences also hold
outside it. No claim that this chamber covers the original unrestricted u
domain follows from those comparisons.

## Small curvature guards, including the difficult u>4 chamber

Let D denote K's leading 2-by-2 determinant. Exact replay gives

```text
D = -u*C-(u-m)^2*R^2/4,
det(K) = (u-m)^2*T*C/4 + s*z*D.
```

The full 3-by-3 determinant is affine in z. These identities are much smaller
than repeated expanded resultants, although a projection saving has not yet
been measured. Curvature alone is not the zero topology of a translated
quadratic on a cube.

There is also a useful exact monotonicity theorem. Set
`C0=(X+Y)^2-u*X*Y`. Then

```text
D=-u*C0-u*(2*X+(2-m)*Y)*R
  -(u+(u-m)^2/4)*R^2.
```

For u>4, 0<m<1, X,Y>0, D is strictly decreasing on R>0. If C0>=0,
then D<0 there. If C0<0, D has exactly one simple positive root R*.
The 2-by-2 principal block is positive definite for 0<R<R* and indefinite
for R>R*. To see positive definiteness, its first diagonal entry at R=0
is positive because `uXY>(X+Y)^2`; it cannot cross zero while D>0,
since the off-diagonal entry is strictly negative. At R=R*, that block
has one positive and one zero eigenvalue. These are whole-domain algebraic
consequences, not sampled curvature assertions.

On D>0, C<0. Consequently det(K) starts negative at z=0 and increases
strictly with z, crossing zero once at

```text
z*=-(u-m)^2*T*C/(4*s*D)>0.
```

K then changes from two positive and one negative eigenvalues to positive
definite. On D<0 its 2-by-2 principal block already has both signs, so K
remains indefinite even if its determinant crosses zero. Equality guards
and every F=0 boundary remain obligations.

The guard cannot be reduced to external kinematics alone. At the same
u=10, X=Y=1, m=5/18, exact replay gives

```text
D(R=1/5)=331843/6480 >0,
D(R=2)=-48265/324 <0.
```

The first two-fraction diagnostic used x=x2,y=x4 before R grouping; its
two witnesses were x=y=1/10 and x=y=1 respectively. They are exact
counterexamples to a kinematics-only Hessian sign claim.

## What to implement and measure next

| Priority | Proposed experiment | Evidence and difficulty |
|---|---|---|
| 1 | Bounded graph/channel coordinate discovery, exact U/F and quadratic-block replay | The authentic three-cube identity is already checked. Graph skeleton lengths/ordered insertions and pairs with verified common coefficient directions are general proposal sources. Moderate implementation; measurable before a new hard campaign. |
| 2 | Quadratic-block projection and face certificates over complete prefix cells | Small matrices replace discovery of some expanded discriminant identities. Must retain all cube faces, internal folds and pair ordering. Higher proof/export effort; strongest potential for the difficult complementary chambers. |
| 3 | Whole-cell sign certificates in threshold guards without changing original coordinates | Extend the existing strict sum/product proof language to verified affine chamber predicates such as 4-u. Useful on many graphs and compatible with original order, but the current above-threshold obstruction remains. |
| 4 | CFF-informed ordering seeds and active-channel priorities | Low implementation risk; preserve all projection obligations. Measure against Brown/support heuristics, including proposal overhead. Expected benefit is uncertain. |

For priority 1, rank candidate groupings by actual total fraction degree,
coefficient support and Hessian-minor size. Admit a grouping only after exact
original signed reconstruction, inverse/domain coverage and denominator
signs. Equal masses or shared cut incidence can suggest candidates; neither
is an acceptance theorem. Cap candidate count, block dimension, support and
native workers. A one-loop polynomial already has a quadratic block; this
test should also be exercised on unrelated one-loop inputs and on multi-loop
graphs where the proposed degree reduction fails. No topology ID should
appear in the admission or ranking logic.

The next controlled ablation should compare: the original legal order;
ordinary projection of the transformed expression; and the same transformed
order with exact matrix identities and bounded face certificates. Measure
all preprocessing, projection sizes, full solves and separate verification.
The first useful endpoint is a reduction in verified obligations at the same
transformed order, not a timeout on a different easier domain.

For a three-variable quadratic on the closed unit cube there are 27 choices
of free/zero/one coordinates, including its interior. At a nonsingular face
Hessian, a stationary candidate is a rational linear-system solution and
its value has a determinant numerator. Exact tests decide whether the point
belongs to that face and whether the value is zero. Singular Hessians require
separate rank/consistency branches, not division by their determinants.
Such a complete face inventory can certify extrema and propose critical
guards. It does **not** by itself certify a cylindrical decomposition: one
must also handle the projection flag inside the quadratic block, all
leading/endpoint drops, repeated roots and root-order collisions. Keep the
original F in every lift and fall back to ordinary exact projection whenever
an obligation is unresolved. The
[parallel selected-system study](PARAMETRIC_PROJECTION_NEXT_RESEARCH.md)
offers a complementary exact query mechanism for these guards.

The distinction from the requested original corpus is material. Its
`variable_groups` keep kinematics outside Feynman parameters. Eliminating
an unrestricted external invariant last can make an affine F easy, but
violates that grouping and does not supply the intended kinematic fibers.
The channel change between (s,t) and (s,u) is triangular and invertible,
with reversed second-coordinate orientation for A446. The positive-sum
fraction map is not triangular in the original Feynman order. A CAD in its
new coordinates therefore gives exact mapped cells, **not automatically an
ordinary CAD in the original coordinates**. Original completion credit
requires a proved cylindrical restoration/common refinement. A future
FastSecDec workflow may consume a verified map, its Jacobian and the
transformed CAD directly, but that is a separate output contract.

## Reproducible evidence and scope

All current calculations are in the ignored directory
`artifacts/research/cff-projection-next-01/`. `prepare.pl` and
`prepare-cube.pl` generate expressions and metadata only; all polynomial
expansion and equality checks run in the existing Rust/Symbolica
`endpoint_identity` executable. Signed identities are packed as `1+w*P`
(or distinct powers of w for multiple identities), so primitive
normalization cannot hide a different scalar. The checker independently
parses both expressions, binds identical variable maps and strict domains,
and requires exact zero difference. This is not an independent implementation
of the underlying algebra.

Eleven final comparisons pass: two original-to-two-fraction identities;
two-fraction coefficient, Hessian, monotonicity and witness identities;
two original-to-cube identities; cube coefficient reconstruction; and two
3-by-3 determinant identities. Each used CPU 3, 2 GiB and a 20-second
killable limit, returned exit zero, and used at most 6,164 KiB reported RSS.
Individual wall times rounded to 0.00–0.02 seconds. They are tiny algebra
diagnostics outside benchmark timings, not solver performance measurements.
No production source, dependency checkout, corpus classification or benchmark
result changed.

| Retained artifact | SHA-256 |
|---|---|
| Frozen `endpoint_identity` | `addc74a94abc1762c02822c4a73ab619fba97603ec9e70e502d5831492d22381` |
| Original-to-cube A446 replay | `3c9ea02937319da68aae15542b689b8a246ef1f7cd61ea8c529f72a109f8df53` |
| Original-to-cube Ax12 replay | `da7e6de8728ca60a4b6725816d02859a6817ae021a71b317d29742ad180b450e` |
| Cube coefficient replay | `b2c7a4cbeee194222ed8243980a001e36001e8a7dcb8aeb09721b3f4a24e3a1c` |
| Factored 3-by-3 determinant replay | `7b8a0bd06cbe256d2e1a568c7698a00e05e7277773feac621fa0f9f230037b61` |
| Curvature monotonicity identity | `29638ea489623e83e59e61b9ac7513381ca3e32e770170b83845c7a5177cd568` |
| Opposite-curvature exact witnesses | `17b28d95204f4833978f5e44375d7b7557f32ef2e67b9b658e634b456dd5d7c3` |
| Final input/report/process manifest | `7031e0009bd8f15ea3e45b08e7f884eeb444728f70a52539faec35abeba752d9` |

The mathematical consequences of those identities were derived above;
the current executable does not claim to verify inertia theorems, inverse
maps, face completeness, or root transport. Those are explicit next gates.

The separate four-position worldline adjunct is retained in
`artifacts/research/cff-worldline-next-01/`. Its four comparisons check both
original F expressions, native graph U and the two Jacobian blocks/product.
All pass with the same frozen Symbolica helper and CPU 3 / 2 GiB / 20-second
envelope; maximum recorded wall time is 0.01 seconds and peak RSS 6,168 KiB.
Its domain keeps u unrestricted. Its metadata parser only checks total
position degrees of Symbolica's expanded output; it performs no independent
polynomial algebra. The final manifest has SHA-256
`3c9a77be81cda3f42f6edf5643aa8cc38acea6b2d8c287206b99a789c3d593d3`.
Native-U replay is `8811f577a37a7aba0b8e91dc9f4d5bb1df311634a0f796af0cdc02205e75621f`;
A446 replay is `2708b4a1fa5aa339dc2c6ab2b819be47582f238a2abfa512783ceb8f2d92c677`;
Ax12 replay is `2e8677fe935f472d1f2034b8ca22d95efb061708d6459b1f95456b0cb1ec07dc`;
Jacobian replay is `53f22816a78c24697c986f8abde1f4785f22d704627ef452447c49eaab674b2d`.
