# Two-point convexity: conservative local sign certificates

This is a research proposal, not a new solver feature or a completed Gluza33
decomposition. It extends the [rational-flow proposal](CFF_KINEMATIC_PROJECTION.md)
to regions above threshold. The original-coordinate examples below follow
algebraically from the already authenticated Gluza33 matrix identity; a fresh
bounded Symbolica replay is the next experimental gate, not a reported run.

## What follows from Thomson's principle

For a connected two-terminal graph, positive edge parameters x and the checked
convention F=U L−sV, U>0, L=Σ m_e²x_e, put R=V/U. Thomson's principle gives
R(x)=min_f Σ x_e f_e² over unit flows. See
[Doyle–Snell, *Random Walks and Electric Networks*](https://arxiv.org/abs/math/0001057)
and [Aldous–Fill, Proposition 3.39](https://www.stat.berkeley.edu/~aldous/RWG/Book_Ralph/Ch3.S7.html).
The following consequences are our deductions from that variational formula:

- R is concave and coordinatewise nondecreasing, since it is an infimum of
  linear functions with nonnegative coefficients.
- For fixed s≥0, g=F/U=L−sR is convex and homogeneous of degree one. Thus
  {F<0} in the positive orthant is a convex cone. The statement concerns F/U;
  it does not assert joint convexity of the expanded polynomial F.
- For any feasible rational flow f, the affine homogeneous function
  H_f=Σ x_e(m_e²−s f_e²) satisfies g≥H_f. At a rational positive point a,
  the minimizing flow is rational and H_f is a supporting hyperplane touching
  g at a. Rational fixed kinematics preserve rational coefficients.

The convexity statement remains valid at zero masses. It requires s≥0 and a
positive definite cycle Gram matrix. A disconnected graph, an uncertified
parameter map, a different signed F convention, or arbitrary multi-leg
Minkowski Gram data does not inherit it automatically. Dependence on external
s is affine, and g is nonincreasing in s because R≥0.

## Relation to existing reductions

The existing CFF note treats uniform subthreshold positivity by requiring all
coefficients of H_f to be nonnegative. The useful extension permits mixed
coefficients and proves H_f>0 on a particular exact domain. Existing Rayleigh
peels exploit monotonicity along massless original coordinates. The general
convex-fiber note instead proves a polynomial endpoint/root theorem. Neither
already implements a joint F/U convexity certificate for a bounded polytope.

For Gluza33, the exact reconstruction and paper-to-parameter mapping are in
[the Rayleigh report](GLUZA33_RAYLEIGH_REDUCTION.md), using equation (10) of
[Dubovyk et al.](https://arxiv.org/html/2201.02576v2#A1.SS3). Here s=10,
L=x0+x1+x2+x4+x6+x7, and

```text
M = [[x0+x2+x5+x6, -x0-x6,        -x2],
     [-x0-x6,        x0+x1+x3+x6,  -x3],
     [-x2,           -x3,           x2+x3+x4+x7]]
j = [x5+x6, -x6, x7]
c = x5+x6+x7
U = det(M),  V = c*U-j^T*adj(M)*j,  F = L*U-10*V.
```

M is a positive Gram sum. Its x5, x1 and x4 terms alone supply positive
diagonal entries in three independent directions, so M is positive definite
throughout the original positive orthant. Equivalently, R is the minimum of
Σ x_e(r_e−b_e·ell)² over the three loop variables ell. This supplies the
variational proof directly from checked routing and polynomial identities;
graph-name metadata is not certificate authority.

## Two explicit original-coordinate candidate certificates

At a=(1,1,1,1,1,1,1,1), exact matrix arithmetic gives
M(a)^{-1}j(a)=(3/4,1/4,1/2), U(a)=36 and R(a)=5/4. In coordinate order
x0,...,x7 the minimizing routing flow is

```text
f = [-1/2, -1/4, -1/4, 1/4, -1/2, 1/4, 1/2, 1/2].
```

Hence the mixed-coefficient supporting form is

```text
H = [3(x1+x2)-12(x0+x4+x6+x7)-5(x3+x5)]/8.
```

Every positive point with H>0 has F>0. For example x1=x2=10 and all other
coordinates one gives H=1/4. This is a nonempty region above threshold; the
old uniform coefficient-slack test cannot certify it because H has negative
coefficients. Write K=4(x0+x4+x6+x7)+(5/3)(x3+x5). With x1 and x2 last,
H>0 is represented using the ordinary linear bounds x1+x2>K and positivity.
The switch at x1=K must be partitioned or handled by a certified max boundary.
The H≤0 complement, including every possible original F=0 point, remains to
be solved; a proposed H=0 seam needs explicit coverage/exception provenance.

A second candidate uses only monotonicity, without enumerating 256 vertices.
If every coordinate is at least one and each of the six massive coordinates
is at most two, then R≥5/4 and L≤12. Consequently g≤12−10(5/4)=−1/2<0.
The two massless coordinates can be unbounded above. Thus this entire
rectangular region is strictly F<0. Open versions of these rational bounds
are ordinary cylindrical cells. This is a domain sign proof, not a statement
that checking one CAD sample suffices. For variable s the same region is
strictly negative whenever s>48/5.

These hand-replayed rational identities are deliberately not recorded as a
new Symbolica diagnostic timing or as original corpus progress. The signed
original F identity has existing independent evidence; new certificate
serialization and containment checks still require their own exact replay.

## Certificate and containment obligations

For a positive supporting-form certificate, discovery may choose a rational
flow at a cell sample. Verification independently checks flow feasibility,
the full variable map, Gram rank, U=det(M), and the signed identity

```text
F = U*H_f + s*b^T*adj(M)*b,  b = C^T*diag(x)*f.
```

Here C is the checked cycle-routing matrix. The quadratic correction is
nonnegative because M is positive definite. It is essential to prove H_f>0
on the whole proposed original-coordinate cell, for example by exact affine
optimization over a rational box or polytope containing that cell. Checking
H_f only at its stored sample is insufficient. This differs from dropping F
globally: only a domain with a full strict sign certificate can omit its roots.

For a negative certificate, convexity gives another sufficient rule: a
nonempty bounded polytope with an exact finite vertex representation is
strictly F<0 if g(v)<0 at every vertex. Vertices must stay in the positive
orthant so U(v)>0. Evaluate the original F and U at those rational vertices
exactly; then Jensen's inequality is the sign theorem. Require exact coverage
of the proposed domain by the polytope. General algebraic cylindrical cells
usually have dependent or unbounded bounds and do not automatically lie in
the box formed from bounds at their sample. Those cells need a separate exact
containment proof or must decline. The simpler lower-corner/upper-mass bound
above also supports unbounded massless directions without a false finite
vertex argument.

For parametric s in a certified nonnegative interval, a positive proof uses
its upper endpoint and a negative proof its lower endpoint. An unbounded
upper interval need not admit a useful positive bound. No physical-kinematics
restriction may be silently added to an unrestricted input domain.

## Why this is not a global projection shortcut

Even the equal-mass bubble has

```text
F = x²+(2-s)*x*y+y²,   U = x+y.
```

F/U is convex for every s≥0, but at s=4 its interior zero x=y is a same-sign
singular seam. For s>4 a positive y fiber has two distinct positive roots;
for 0≤s<4 it has none. Convexity therefore does not license removing the
threshold/discriminant or a zero residual. Empty fibers, two exterior
positive intervals and endpoint switches still require exact partitions.

The additional a,b,c,H guards in the current Gluza33 positive reduction need
not be graph F/U functions; their high-degree projection obstructions do not
inherit this theorem. A radial atlas from a negative interior point also does
not automatically export a CAD in the original coordinates. Neither idea
replaces the existing boundary-ordering and exhaustive-lifting obligations.

## Next bounded gate

1. Replay the two explicit Gluza33 regions and the original signed F identity
   with Symbolica in a small research helper. Also replay a two-point graph
   with a different cycle rank, masses and parameter names. Reject altered
   routing/mass/map data, a missing lower bound, and a region crossing H=0.
2. Compare these domain certificates against existing positivity methods on
   exactly the same rational boxes/polyhedra. Include discovery, independent
   replay and containment costs. Record a gain only if the old portfolio
   actually declines or costs more; the algebraic theorem alone is not a
   measured solver improvement.
3. Only after a gain, try a bounded local-engine proposal: at most four
   rational sample flows and small rational boxes, no topology dispatch.
   Emit a checked sign proof on each accepted region and keep the entire
   uncovered complement. Measure complete conditional-domain solves before
   claiming a global GCAD benefit.

No new expensive Gluza campaign was run for this assessment. The existing
negative Gluza33 decomposition is already complete; priority for these
proposals is reducing work in the unresolved positive complement.
