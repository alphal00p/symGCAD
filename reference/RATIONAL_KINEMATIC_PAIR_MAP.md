# A rational kinematic map for positive pair roots

Research continuation, 10 October 2026. The map and derivative identities in
this note are hand-derived, with independent static review; they have not
been added to the Symbolica replay gate. The preceding endpoint/resultant
gate is recorded in [positive pair root stratification](POSITIVE_PAIR_ROOT_STRATIFICATION.md).
No new algebra run, root-count theorem for the authentic complement, or
production change is claimed.

## Extract the affine parameters without changing the variable order

Suppose exact coefficient extraction gives

```text
F=q*z+B,
q=Q-s*H-u*C,  B=M-u*T,
H>0, T>0.
```

Here `Q,H,C,M,T` are independent of `s,u`. Retain the original external
parameter and Feynman-coordinate order. The following rational functions
describe common solutions; they are not a replacement integration domain:

```text
U=M/T,
S=(Q*T-C*M)/(H*T).
```

For positive denominators, `q=B=0` is equivalent to `s=S,u=U`. Thus the
kinematic values admitting positive common roots are images of the positive
coordinate domain under `(x,y) -> (S,U)`, with the other coordinates fixed.
The coefficient determinant in `(s,u)` is `H*T>0`.

For the documented operand, put `x=x2,y=x4`, and use

```text
l=x1+x3, r=x5+x7, a=l+x, b=r+y,
X=x1+x5, Y=x3+x7, m=5/18,
W=2*X+(2-m)*Y,
H=x*y, C=X*Y,
Q=x^2+2*x*y+y^2+W*(x+y)+(X+Y)^2,
T=b*x1*x3+a*x5*x7,
M=a*b*(a+b)-m*(b*x*x3+a*y*x7).
```

These definitions retain `u=t` for Ax446 or `u=5/9-s-t` for A446. Using the
second relation on an original problem still requires certified triangular
kinematic transport. It does not restrict the original free parameter `t`.

The positive identity

```text
M=a*b*((a+b)-m*Y)+m*(b*l*x3+a*r*x7)>0
```

holds when all Feynman coordinates are positive and `0<m<1`. Consequently
`U>0`, and the earlier endpoint witnesses imply `U_x=B_x/T>0` and
`U_y=B_y/T>0`, with each `B` derivative evaluated at `u=U`. The positive
`U=u` level exists exactly above its coordinate-corner value

```text
u_corner=l*r*(l+r)/(r*x1*x3+l*x5*x7) >= 4.
```

The inequality follows from `4*x1*x3<=l^2` and `4*x5*x7<=r^2`.
For `u>u_corner`, this level is the finite decreasing arc between the two
positive axis roots of `B`. This restates the `H0<0` endpoint chamber in
kinematic-map language, including its dependence on the outer coordinates.

## A turning-point polynomial independent of s

Set `Delta=q_y*B_x-q_x*B_y`. Implicit differentiation on `q=B=0` gives

```text
det d(S,U)/d(x,y) = -Delta/(H*T).
```

To study the entire `U=u` arc, use `N=Q-u*C` and form the polynomial

```text
D=(H*N_y-H_y*N)*B_x-(H*N_x-H_x*N)*B_y.
D=H*Delta-q*(H_y*B_x-H_x*B_y).
```

This is an exact polynomial identity, and `D` is independent of `s`. It
also applies when `C` depends on `x,y`, because its derivatives are included
in `N_x,N_y`. On a positive `B=0` arc parametrized by increasing `y`,

```text
x'(y)=-B_y/B_x,
S'(y)=D/(H^2*B_x).
```

For `H=x*y`, all denominator factors are positive. At a common root
`D=H*Delta`; hence the true interior pair critical points can be studied as
positive solutions of `B=D=0`, independent of `s`, and then mapped to their
critical `S` values. A statement using `Delta` along the whole arc must
evaluate it at `s=S`; an arbitrary fixed `s` does not satisfy `q=0` there.

For the quadratic operand, a compact expression before expansion is

```text
C0=(X+Y)^2-u*X*Y,
D=x*(y^2-x^2-W*x-C0)*B_x
  -y*(x^2-y^2-W*y-C0)*B_y.
```

This is a candidate for a smaller proof problem, not a claim that its
elimination or sign classification is cheap. At a turning point the
additional condition `B_x*D_y-B_y*D_x != 0` certifies a simple stationary
point along the arc. Degenerate stationary points remain separate strata.

## Conditional branch inventory and verifier obligations

For `H=x*y`, away from endpoint contact, `S` tends to signed infinity at
each end of the arc. The signs are those of `q(alpha,0)` and `q(0,eta)`;
these endpoint values are independent of `s`. Equal endpoint signs force
an interior stationary point. If `D` has no zeros along the whole arc,
`S` is strictly monotone, its endpoint signs are opposite, and each real
`s` has exactly one positive common pair.

More generally, an exact finite inventory of stationary points divides
the arc into intervals on which the sign of `D` determines monotonicity.
Each such interval contributes one common pair precisely when `s` lies
strictly between its two limiting `S` values. Critical levels and endpoint
contacts need their own sections or explicit auxiliary exceptions. This
gives an algorithm conditional on the following certificates:

1. Exact affine extraction and positive denominators, endpoint witnesses,
   and finite arc endpoints over the complete base cell.
2. Complete positive solutions of `B=D=0`, including multiplicities,
   their global `y` ordering, and all degeneracy/endpoint events.
3. Certified ordering or coincidence of their critical `S` values and the
   current `s`, including possible positive-dimensional critical fibers.
4. An exported positive-common-solution selector, or whole-cell constancy
   of an existing resultant root ordinal. A positive `y` selector alone
   cannot reject a resultant root whose common `x` is negative.
5. Independent projection for the roots of `q` itself, followed by the
   joint `q,B` lift and every original positive `z=-B/q` boundary.

The map can guide certificate discovery on existing ordered cells. Building
`s` intervals after `u` and the outer coordinates would generally violate
the original order; those critical-value conditions must instead be
projected in that order or supported by a separate transport theorem.
Neither the map nor a physical interpretation supplies that theorem.
Ordinary exact projection remains the fallback when a certificate fails.

At `q=B=Delta=0`, positive critical `z` requires `q_x<0`, since
`z=-B_x/q_x`. This is stationarity only in `x,y,z`. Events with `q_x>0`
may still change the cylindrical order of `q` and `B`; they remain required
for this classification even when they are not positive-`z` critical points.

## What the physics literature contributes

[Hannesdottir, McLeod, Schwartz and Vergu, *Constraints on Sequential
Discontinuities from the Geometry of On-shell Spaces*](https://arxiv.org/abs/2211.07633),
Section 5, develops Landau singularities as critical values of projection
maps on on-shell spaces. This supports the geometric viewpoint, but our
two-coordinate map is only a partial stationarity problem. Their Section
7.5 also distinguishes the box double-spectral support, reached outside
the physical region by continuation, from sequential monodromies about
normal thresholds. Their generic-mass and physical-region hypotheses
cannot silently be transferred to the present mixed-mass operand.

[Correia, Sever and Zhiboedov, *An Analytical Toolkit for the S-matrix
Bootstrap*](https://arxiv.org/abs/2006.08221), Sections 3.2 and 3.5, relates
double-spectral support to Landau curves. Its displayed leading curve is
derived for identical massive scalar scattering in an elastic strip,
using the stated analyticity and unitarity assumptions. It supplies no
whole-domain positive-pair count for this graph. In particular, vanishing
spectral density is not a certificate that an original real polynomial
boundary or a required CAD ordering event is absent.

[Sturmfels and Telek, *Copositive geometry of Feynman integrals*](https://arxiv.org/abs/2504.01628),
Corollary 5.9, connects the boundary of the kinematic copositive cone to
Landau discriminants. Its Pólya completeness theorem requires all internal
masses positive. That theorem cannot certify away the genuinely
sign-changing mixed-mass complement here. A successfully replayed positive
coefficient identity would still be a valid local sufficient certificate.

The recommended next gate remains small: replay the affine-map and `D`
identities on unrelated inputs and the authenticated operand, then certify
the positive pair inventory across one documented irrelevant resultant
collision. Do not start with a full unrestricted sextic discriminant, and
do not infer a global two-intersection bound from the existing samples.
