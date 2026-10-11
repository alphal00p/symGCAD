# Tropical follow-up: conditional circuits on proved monomial chambers

Research audit, 11 October 2026. **A new route after the negative global-circuit
screen is to include independently proved domain inequalities in the exponent
balance.** This can certify a polynomial on its actual chamber even when a
negative Newton vertex rules out positivity on the full positive orthant.
This is a proposed exact certificate, not an implemented optimization or a new
corpus completion. No native computation, build, or production change was made.

## What the additional primary-source audit changes

The earlier notes already cover joint gauge/order search, global AM–GM circuits,
and the Gluza33 opposite-sign-vertex obstruction. Their conclusions stand.
Borinsky's tropical sampling replaces a polynomial by a max-monomial envelope;
the useful uniform lower bound assumes complete nonvanishing. It does not prove
the hypothesis needed to delete a GCAD cut. [Theorem 8.B and Corollary 9](https://arxiv.org/html/2008.12310#S3.SS1)

SubTropica performs tropical subtraction and symbolic integration. Its
positivity discussion identifies hidden interior singularities as an additional
problem. [Paper, §3.1](https://arxiv.org/html/2604.20954v1#S3.SS1)
The [online interface](https://subtropi.ca/) exposes graph, gauge and ordering
choices; it supplies no additional exact cell certificate. Re-reading the
[pinned source](https://github.com/SubTropica/SubTropica/blob/cbb4d91fdf5c261e8687ba01dfb94d65498e8b73/SubTropica.wl#L17550-L17580)
confirms that `stDeriveGaugeFromHomogeneousLR` uses the final homogeneous LR
variable as gauge. The dispatch checks projectivity; its default HyperFLINT
route can derive the integration order again, so the returned order is not
universally enforced. None of this licenses Fubini-set intersections in CAD.

The additional mathematics is **conditional SAGE**. Murray–Chandrasekaran–Wierman
give a support-function certificate over a convex logarithmic domain and allow
a certified upper bound on that support function. [Theorem 6 and the conic-domain discussion](https://arxiv.org/html/1907.00814#S3.SS1)
Murray–Naumann–Theobald replace ordinary support circuits by domain-dependent
circuits; their weighted-power formulation makes the domain contribution
explicit. [Proposition 2.3, Definition 4.3 and Theorem 4.4](https://arxiv.org/html/2006.06811v3#S4)
We need only the following elementary rational sufficient rule, derived here;
no floating-point entropy optimization or completeness theorem is required.

## Exact proposed rule

Let `x>0`, and suppose the relevant domain is contained in the independently
certified set

```text
D = { x : x^u_j >= L_j > 0 for j=1..r }.
P = sum_i c_i*x^alpha_i - b*x^beta,  c_i>0, b>0.
```

The integer vectors `u_j` may have negative entries. Clear their Laurent
denominators using only proved positive monomials. Coefficients and `L_j` may
depend on external parameters, provided their signs and all inequalities below
are proved over the entire parameter chamber.

Find integers `m_i>0`, `n_j>=0`, `N=sum_i m_i` satisfying

```text
sum_i m_i*alpha_i = N*beta + sum_j n_j*u_j,
Delta = N^N * product_i c_i^m_i * product_j L_j^n_j
        - b^N * product_i m_i^m_i > 0.
```

Weighted AM–GM with weights `m_i/N` gives

```text
sum_i c_i*x^alpha_i
 >= [N^N*product_i c_i^m_i/product_i m_i^m_i]^(1/N)
    * x^beta * product_j (x^u_j)^(n_j/N)
 >= [N^N*product_i c_i^m_i*product_j L_j^n_j
     /product_i m_i^m_i]^(1/N) * x^beta
 > b*x^beta.
```

The implementation and replay use only the integer exponent identity and the
strict rational/polynomial margin; radicals appear only in this proof.
Rational coefficient denominators require their own positive-sign evidence.
Several negative terms need an exact allocation of the positive coefficients
and reconstruction of their sum, with no double spending. Require a strict
component or strict residual everywhere; decline equality in an initial gate.

The decisive difference from the previous global proposal is the nonzero
`sum_j n_j*u_j`: the negative exponent need not belong to the convex hull of
positive exponents. Conversely, a negative vertex remains an obstruction on
this chamber if there is a rational recession direction `v` with

```text
u_j.v >= 0 for every j,
beta.v > max_i alpha_i.v.
```

For any feasible `x0`, the ray `x=x0*t^v`, `t>=1`, stays in `D` and the negative
term eventually dominates. This supplies a small exact rejection test before
trying coefficient margins. A global negative-vertex ray that violates a
domain inequality proves nothing about that chamber.

## Worked example and counterexamples

For `x,y,z>0`, `q>0`, `k>0`, consider

```text
P = x^3+y^3-k*z^3,       xy >= q*z^2.
u=(1,1,-2),  L=q,
alpha_1=(3,0,0), alpha_2=(0,3,0), beta=(0,0,3),
m_1=m_2=1, N=2, n=3,  Delta=4*q^3-k^2.
```

The exponent identity is `(3,3,0)=2*(0,0,3)+3*(1,1,-2)`.
Thus `q=1, 0<k<2` is a full parameter chamber with `P>0`, although the negative
exponent is an exposed vertex and `P(1,1,2)=-6` at `k=1` outside that chamber.
At `q=1,k=2`, `x=y=z` is an interior positive-orthant zero: an equality margin
must not erase it. Even with strict `xy>z^2`, where a stronger proof could use
strictness of the premise, this conservative first gate declines `Delta=0`.
Removing the independent `xy>=qz^2` premise makes the argument invalid.
These are hand-derived exact identities; no executed Symbolica check is claimed.

## Projective gauge and original-cylinder obligations

Ratio premises with `sum(u_j)=0` survive a verified homogeneous positive scale
map. An optional future atlas is especially simple: on the chamber where
`x_h` is the unique largest Feynman parameter, set `rho=x_h` and `y_i=x_i/rho`.
Then `rho>0`, `0<y_i<1`, the inverse is exact and the absolute Jacobian is
`rho^(n-1)`. There are `n` maximum chambers, not `n!` full ordering sectors.
Their complement lies on the explicit tie polynomials `x_i-x_j`; retain every
tie's provenance. Existing sector-decomposition boxes can supply bounded
domains without creating this atlas at all.

This is a new *domain* experiment, separate from the already assigned
anchor/short-prefix search on the whole orthant. Every original constraint must
have its scaling identity checked and kinematics remain outside the scale
block. If the anchor is first in the legal Feynman block, each scale map is
triangular. However, CADs from different anchors/orders cannot be concatenated
and called a single original-coordinate CAD: a common cylindrical refinement
is still needed. Fixed orders and variable groups remain binding. No proposed
atlas is an accepted replacement for the original corpus decomposition.

## Bounded next experiment and realistic priority

The existing `src/domain.rs::derive_domain` uses original coordinate-affine
bounds only. It does not yet authenticate coupled monomial premises. Most
original symbolic ZZ fixtures supply positivity and `s>4`, not parameter-ratio
chambers. Thus this is not an immediate explanation for their unresolved cost.

1. **First, a certificate-only feasibility pilot:** one core, 30 seconds, 1 GiB;
   at most 32 targets, 256 source terms, 64 candidate components, four positive
   exponents and two domain normals per component, common denominator `N<=8`.
   Use the example above, unrelated unit-box/Laurent examples, and captured
   factors only where independent whole-domain premises are available. Record
   zero eligible corpus factors honestly. Bound all products and leaf proofs;
   no auxiliary parameter CAD and no fresh global chamber fan.
2. Compare global circuits, conditional circuits, and existing Bernstein/
   weighted-square certificates on identical domains. New certificates, exact
   replay cost and avoided algebra obligations are the useful metrics; a
   certificate count alone is not a solver speedup. The known 137-term Gluza33
   factor is a sensible rejection/control input, but a box around its stored
   sample is not an admissible domain.
3. Only after an authentic hit, investigate maximum-coordinate charts or
   graph-derived ratio premises. Count their extra boundary, reconstruction,
   ordering-search and common-refinement costs. Keep all complementary regions.

Every accepted witness needs original polynomial/map, premise provenance and
coverage evidence, integer weights, coefficient allocation, exact reconstruction
and strict-margin evidence. Premise dependencies must be acyclic and cannot use
the target's desired sign. A local certificate must stay local in cache/replay.
Inconclusive targets retain the existing exact projection/lifting path,
including every interior `F=0`, degree-drop and root-ordering obligation.

**Priority:** this conditional gate is more plausible than retrying impossible
global circuits, but ranks behind the ongoing exact two-stage ordering probe
for the current unrestricted ZZ corpus. Its strongest prospective application
is the later graph/sector workflow with authenticated bounded domains. No
performance gain is established by this audit.

Independent mathematical review by `parametric_cad_research` confirmed the
integer-weight rule and example; it emphasized Laurent denominator positivity,
independent domain provenance and exact chart/tie transport.
