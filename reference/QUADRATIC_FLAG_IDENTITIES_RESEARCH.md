# Quadratic identities for actual cylindrical projection obligations

Research continuation, 11 October 2026. The
[quadratic-face experiment](QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md) exposed a cheap
stationary-event inventory, but that inventory cannot replace cylindrical
projection. This note gives an exact counterexample and a narrower, sound
optimization: compute particular **existing projection obligations** through
smaller resultants, preserving their factor sets together with already
required coefficient guards. There are no production changes or new solves.

## The face inventory misses root-ordering cuts

Consider the strictly convex quadratic

```text
Q(a,b;t) = a^2+a+b^2+t*b-1/4,
0<a,b<1, -11/10<t<-9/10,
lifting order t,a,b.
```

All nine face quadratic coefficient matrices are nonsingular, with the
zero-dimensional convention at vertices. Their determinants are 1; the
positive-dimensional Hessians are twice those matrices. The nonconstant
critical-value guards are drawn
from `-2-t^2`, `-1-t^2`, `7-t^2`, `2+4*t`, `3+4*t`, `11+4*t`, and their
stationary membership guards from `-1`, `3`, `-t`, `2+t`. The remaining
critical-value guards are the nonzero constants `-2,-1,7`. Every such guard
has a fixed strict
sign throughout the stated t interval: in particular `t^2<121/100<7`,
`-t>9/10` and `2+t>9/10`.

But the two endpoint polynomials are

```text
P0(a) = a^2+a-1/4,
P1(a) = a^2+a+t+3/4,
Res_a(P0,P1) = (t+1)^2.
```

Each has exactly one root in `(0,1)` because its derivative is `2a+1>0`
there and its endpoint values have opposite signs. Since `P1-P0=t+1`,
their root order reverses at `t=-1`. This is a collision of projections of
two different endpoint sheets, without a face extremum changing sign. An
ordinary cylindrical decomposition in the declared order needs this cut.
The frozen helper independently generated the nine face identities and
ordinary projection; the latter explicitly contains the factor `1+t`.
An additional exact replay evaluates the following complete product at
`t=-1` to `-5292/25 != 0`:

```text
[(-2-t^2)*(-1-t^2)*(7-t^2)*(-2)*(-1)*7*(2+4*t)*(3+4*t)*(11+4*t)]
 * [(-1)*(-t)*3*(2+t)] * (t+11/10)*(-9/10-t).
```

This covers all critical-value and membership guards (the constant `-1`
occurs in both groups) and both prefix constraints; determinant guards are 1.
Hence `1+t` is not hidden as a factor of that face-guard product.

Thus **51 raw face guards versus 71 ordinary prefix factors does not establish
coverage** for the earlier graph chart. They must be compared through exact
factor identities, and even matching zero sets would still leave degree,
delineability, lifting and original-coordinate obligations. The toy example
already rules out a general theorem equating the two inventories.

## Conventions and specialization

Work over the fraction field of an exact rational polynomial coefficient
ring. The coefficient ring includes the complete remaining CAD prefix,
not just external kinematics. For nonzero polynomials `f,g` of actual degrees
`m,n`, use the ordinary Sylvester convention

```text
Res(f,g) = lc(f)^n * product_{f(alpha)=0} g(alpha),
Res(f,g) = (-1)^(m*n) Res(g,f),
Res(p1*b+p0,q1*b+q0) = p1*q0-p0*q1.
```

This is the convention in the
[Sage resultant documentation](https://doc.sagemath.org/html/en/reference/polynomial_rings/sage/rings/polynomial/polynomial_rational_flint.html),
used here only to fix terminology; every executed calculation uses Symbolica.
In this repository use `projection::exact_resultant`, which already repairs
the pinned Symbolica small-case omission of the odd-degree sign in a linear
second operand. Calling that native small-case function directly would not
be a reliable signed replay.

Write `R_(m,n)(f,g)` for the resultant of the **fixed-degree binary forms**
`Y^m*f(X/Y)` and `Y^n*g(X/Y)`. It is a polynomial in their padded coefficients.
It does not automatically become the ordinary actual-degree resultant after
specialization. If the specialized nonzero polynomials have degrees `p,q`
and leading coefficients `alpha,beta`, then

| Specialized degrees | Fixed-degree resultant |
|---|---|
| `p=m, q=n` | `Res(f,g)` |
| `p=m, q<n` | `alpha^(n-q)*Res(f,g)` |
| `p<m, q=n` | `(-1)^(n*(m-p))*beta^(m-p)*Res(f,g)` |
| `p<m, q<n` | `0` |

The last row occurs because both binary forms acquire a root at infinity.
The other rows follow by factoring out powers of Y and using
`Res(F,Y)=lc(F)` and `Res(Y,G)=(-1)^deg(G)*lc(G)`. Zero polynomials need
their own fallback; a degree is not assigned to them. A formally vanishing
resultant on a degree-drop stratum does not say that finite affine roots
collide there.

## Exact two-coordinate flag identities

Let a jointly quadratic block, viewed in its final two coordinates, be

```text
Q(b,c) = A*c^2 + (D*b+E)*c + (F*b^2+G*b+H),
B = D*b+E, C = F*b^2+G*b+H,
Delta = disc_c Q = B^2-4*A*C.
```

All six coefficients are independent of b,c. They can still depend on every
earlier coordinate and on unrestricted external kinematics. For endpoint
`e=0,1`, define `P_e=Q(b,e)` and `K_e=B+2*A*e`. Then

```text
Delta = K_e^2 - 4*A*P_e,
R_(2,2)(P_e,Delta) = R_(2,1)(P_e,K_e)^2,
R_(2,2)(P0,P1) = F * R_(2,1)(P0,B+A).
```

These are fixed-degree polynomial identities, valid even when coefficients
specialize to zero. They follow from reducing the second operand modulo
`P_e`, resultant multiplicativity, and the homogenizing factor Y in the
opposite-endpoint difference. There is **no missing F factor** in the second
identity and there **is one F factor** in the third.

For explicit replay define

```text
T0  = F*E^2-G*D*E+H*D^2,
T1  = F*(E+2*A)^2-(G+D)*D*(E+2*A)+(H+E+A)*D^2,
T01 = F*(E+A)^2-G*D*(E+A)+H*D^2,
J   = D^2-4*A*F,
D0  = G^2-4*F*H,
D1  = (G+D)^2-4*F*(H+E+A),
R   = A*D0+T0.
```

The endpoint/discriminant resultants are `T0^2,T1^2`, the opposite-endpoint
resultant is `F*T01`, and

```text
disc_b Delta = 16*A*R.
```

For the two-variable quadratic form with matrix
`[[F,D/2],[D/2,A]]`, linear vector `(G,E)` and constant H, the stationary
critical-value numerator `4*det(matrix)*H-l^T*adj(matrix)*l` is exactly `-R`.
This relates a particular ordered projection event to a small matrix. It
does not identify all cross-event ordering resultants with stationary values.

For this single-cut family with the stated finite interval boundaries,
first c projection retains `A,Delta,P0,P1` and all inactive bounds and
constraints. For the unfactored, full-degree family, the complete next b
projection is represented by

```text
A, F, J, D0, D1, R, T0, T1, T01,
P0(0), P0(1), P1(0), P1(1), Delta(0), Delta(1),
all inactive prefix constraints and coefficient contents.
```

This list includes leading coefficients, all three discriminants, all three
nontrivial pair resultants, and interactions with `b,1-b`. Constant-in-b
resultants introduce powers of already retained coefficient factors.
If the original block is jointly quadratic also in an earlier coordinate a,
then `A,D,F` are independent of a, `E,G` are affine in a and H is quadratic
in a. Every displayed nontrivial guard remains at most quadratic in a.
The **next pair resultants between these different a-events are still
required**. The counterexample above shows why they cannot be replaced by
the stationary inventory of the original full cube.

This compact raw list is not automatically a drop-in replacement for the
current factor-first pipeline: repeated factors or common factors can make
a parent discriminant or parent resultant identically zero while individual
factor obligations remain nontrivial. Such cases require the admission rules
below or the unchanged baseline fallback.

## Actual degrees and the zero-set-preserving reduction

For nonzero actual operands satisfying `Delta=K^2-U*P`, let
`m=deg(P)>0`, `n=deg(Delta)`, `k=deg(K)` and `p=lc(P)`. Evaluating at the
roots of P gives the exact identity in the fraction field

```text
Res(P,Delta) = p^(n-2*k) * Res(P,K)^2.
```

For `V=P+L`, likewise

```text
Res(P,V) = p^(deg(V)-deg(L)) * Res(P,L).
```

The multiplier U may itself be a polynomial; it vanishes modulo P. Negative
exponents mean **cross-multiply**, then prove the polynomial identity or exact
division. They are not permission to divide on an unguarded `p=0` fiber.
Constant K is important: when P and Delta are quadratic and K=E is constant,
`Res(P,Delta)=p^2*E^4`, not merely `E^4`. If the leading coefficient of Delta
cancels identically, `n=1,k=1` gives exponent -1. When a leading coefficient
only vanishes on a parameter slice, that slice remains in the exception or
specialization obligations and the actual-degree pipeline is replayed there.

For nonzero resultants, unique factorization proves that the union of
irreducible factors of `{p,Res(P,Delta)}` equals that of
`{p,Res(P,K)}`. The analogous statement holds for the endpoint pair. Thus a
projection routine may factor the **smaller** resultant and retain p instead
of constructing, expanding and refactoring its square. It preserves the
same algebraic zero set, including nonphysical cylindrical collisions.
It is an exact reduction, not a domain heuristic. Zero operands or zero
resultants decline this first implementation; they are not certified empty.

## Rational and affine boundary polynomials

For `c=N/D`, with D independent of b,c and nonzero on the chart, define

```text
P_e = D^2*C + N*D*B + N^2*A,
K_e = D*B + 2*N*A.
```

Then `Q(b,N/D)=P_e/D^2` and the exact cleared identity is

```text
D^2*Delta = K_e^2-4*A*P_e.
```

If N is affine in b, `P_e` remains quadratic and `K_e` affine. For two such
endpoints `(N_l,D_l)` and `(N_u,D_u)`,

```text
D_l^2*P_u-D_u^2*P_l = W*V,
W = N_u*D_l-N_l*D_u,
V = B*D_l*D_u + A*(N_u*D_l+N_l*D_u).
```

W and V are at most affine in b. Resultant multiplicativity therefore reduces
the endpoint collision obligation to `Res(P_l,W)*Res(P_l,V)`, with the
actual-degree leading-coefficient powers and `D_l^(2*deg(P_l))` retained in
the cleared identity. For the ordered-simplex interval `b<c<1`, this is
`W=1-b`, `V=B+A*(1+b)`: the first factor is an existing endpoint guard,
and the second is another small linear-resultant obligation. This extends
the idea directly to ordered worldline positions without a nonlinear
simplex-to-cube substitution.

All denominator factors must remain guards unless an existing exact domain
certificate proves them nonzero. Inequality orientation additionally needs
a fixed denominator sign; one cannot multiply a strict inequality by an
arbitrary D. Chart coverage and `u-l>0` must be proved separately. A `D=0`
stratum is a chart failure requiring another chart or the original problem,
not a deleted physical region. The identities themselves do not assume
physical kinematics or identify a physical Landau locus with a CAD boundary.

## Existing implementation and a minimal next change

The native backend already has straight-line linear and quadratic resultants.
`projection::exact_resultant` supplies signed linear evaluation and bounded
interpolation; discriminants through degree four also have exact formulas.
`AlgebraCache::raw_projection` skips constant-degree pairs and caches ordinary
resultants. This proposal would add **cross-operation structural reuse**:
knowledge that a particular discriminant and endpoint came from the same
quadratic fiber, before their squared resultant is expanded and factored.
It must not be advertised as a newly discovered generic quadratic resultant.
Additional source polynomials keep all their cross-resultant obligations.
Only existing domain-boundary polynomials receive endpoint tags; a positive
half-axis input must not acquire an artificial upper-bound constraint.

A minimal typed provenance record should bind:

- source polynomial and complete variable map; eliminated coordinate c;
  exact coefficient polynomials A,B,C and the reconstruction identity;
- boundary numerator/denominator IDs and the exact `P_e,K_e,Delta` identities;
- the next elimination variable b, actual degrees, signed rational scales,
  and all coefficient/denominator guards required by the cleared identity;
- factor reconstruction linking each parent to one primitive positive-b-degree
  factor, its multiplicity, and a product of degree-zero content factors.

Initially admit only a single multiplicity-one positive-degree factor per
parent. If an endpoint or discriminant splits into several b-dependent
factors, if the selected factors coincide, if an operand is zero, or if a
required structural equality fails, use the ordinary pair calculation.
Do not search arbitrary polynomial pairs for ideal membership. Carry the
small lineage record when the source operation constructs the endpoint or
discriminant; lookup then has a cheap, bounded applicability test.

Contents can be handled without an unsafe division. If a recorded identity
has `v*D_primitive=K^2-U*u*P_primitive`, with u,v independent of b, then

```text
v^m * Res(P_primitive,D_primitive)
  = lc(P_primitive)^(n-2*k) * Res(P_primitive,K)^2.
```

Together with retained factors of v and the leading coefficient, this gives
the same factor union. The content/scale reconstruction must be replayed.
The corresponding endpoint-pair formula uses the second endpoint's content
in exactly the same way.

The insertion point is the pair loop in `AlgebraCache::raw_projection`, just
before `self.resultant`. A separate `ProjectionFactorIdentity` result can
return the smaller factors plus a typed witness. **Do not store the smaller
resultant in the existing exact-resultant cache under the larger pair's key**:
it has a different value and multiplicity. Give the factor-identity operation
its own versioned key/evidence and retain the unchanged exact-resultant API.
Verification must replay the source, content, degree and multiplier identities
and check the same canonical factor union; it must not trust a cached lineage
or infer a sign certificate from a zero-set identity.

The first regression matrix should cover full degrees, constant K, an
identically dropped Delta degree, a dropped endpoint degree, both padded
degrees dropping, a zero polynomial, repeated/shared factors, rational and
negative denominator orientations, the signed pair `Res(b,b-1)=-1`, altered
lineage, and a changed variable map. The endpoint-reordering counterexample
must retain `t+1` after the optimization.

## Executed checks and the next bounded experiment

Using only existing frozen executables, 18 Symbolica expression identities
were checked exactly: Sylvester determinants of sizes 4 and 3, both endpoint
identities, opposite endpoints, the double discriminant, the stationary
matrix value, fixed-degree drops, a negative actual-degree exponent, the
linear sign convention, rational endpoint clearing, affine endpoint
differences, and the nonzero counterexample guard product. They check the displayed finite formulas against separately
assembled determinant expressions; they do not exercise a new resultant
dispatcher or implement the proposed provenance verifier. The sign-preserving
normalization wrapper `1+w*P` prevents a scalar mismatch being hidden.

The existing quadratic helper also completed both modes on the nine-face
counterexample. Evidence is in the ignored directory
`artifacts/research/quadratic-flag-identities-01/`. No Cargo build or production
mutation was needed. All workers had one CPU (logical CPU 3), a 2-GiB bound,
and a 20-second deadline.
The individual expression replays took at most 0.01 s and 3,132 KiB peak RSS.
The audited evidence manifest SHA256 is
`65a7f9e4b7b836f6704d2093d0d2e3e7c988b1fe6c26367e96ce06df35a0e5e9`.

The next implementation experiment should compare the **same complete factor
basis** and actual ordinary projection costs with and without these typed
identities, first on generic toy families and then the authenticated graph
chart. Keep every unmatched pair on the ordinary path. The earlier chart's
first two levels took only about 0.435 s out of 8.643 s, so optimizing only
those levels has a small direct ceiling in that experiment; the expensive
last-level cross-event resultants remain. Further gains require additional
proved identities for those pairs, not removal based on a face or threshold
label. A separate exact factor-inclusion audit of the 51 raw face guards and
71 ordinary factors is still proposed, not claimed to have been executed.

For parametric kinematics the sound route is therefore: use graph/CFF data
to propose a verified quadratic chart and legal order, retain all original
kinematic chambers, and reduce the cost of identified projection obligations
through exact algebra. Threshold information can prioritize charts or
ordering trials; it cannot replace the root-ordering guards this note retains.
