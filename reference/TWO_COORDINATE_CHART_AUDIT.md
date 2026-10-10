# Two-coordinate positive homogeneous chart: export and verification audit

This is an implementation design, not an original-coordinate completion
certificate. The research helper checks polynomial identities and can solve
the transformed problem; original export still needs the obligations below.

For a strictly positive block with selected coordinates `a,b`, put `L=a+b`,
`r=b/L`, and `z_i=x_i/L` for its remaining coordinates. The chart domain is
`a>0, 0<r<1, z_i>0`, and its triangular inverse in lifting order
`[outer parameters, a, b, remaining block]` is

```text
a = scale,
b = scale*r/(1-r),
x_i = scale*z_i/(1-r).
```

Every original constraint and split polynomial must be homogeneous in the
declared block, with its own degree `D`. Its transformed polynomial is
`q(r,z)=p(1-r,r,z)`. Verify the exact identity
`p = L^D q(b/L,x/L)` by polynomial reconstruction. Since `L>0`, each sign is
preserved. Degree-zero kinematic constraints are unchanged. Transform every
original restriction; adding `z_i>0` requires original positivity premises.
The map covers the entire strict positive block with no additional interior
seam. Existing strict boundary exclusions remain explicit.

Fixed orders and variable groups are obligations. Both `a` and `b` must
precede every remaining block coordinate, with `a` first. The reduced ratio
`r` must therefore be the first reduced block coordinate. Moving `r` last to
make monotone peeling convenient does not give cylindrical original cells.

## Minimal wrapper architecture

Follow `src/homogeneity.rs` for preparation, a nested independently verified
reduced `SolveResult`, original metadata, and one-to-one cell reconstruction.
Use a separate evidence type recording the pair, block, original degrees,
and reduced certificate. Preparation must clear its own chart option before
calling the ordinary solver, and reduce the remaining wall-time budget.

For each reduced boundary polynomial `h`, choose its total degree `e` in all
ratio coordinates and reconstruct `H=L^e h(b/L,x/L)`. Check the identity on
`L=1`, retain its exact polynomial-table map, and assign its actual highest
original variable. Unlike the existing scale wrapper, levels cannot simply
be shifted: `1-r` reconstructs to `a`, an earlier coordinate. Every retained
boundary must be cylindrical in the requested original order.

Insert the positive scale axis with sample `a=1`. A reduced sample `r=rho`
maps to `b=rho/(1-rho)`; every later ratio sample and isolating interval is
multiplied by `1/(1-rho)`. Nonblock samples and bounds are unchanged. Copy
split signs in their original order. Include all transformed exceptional
polynomials and the positive scale boundary; denominator zeros lie outside
the strict domain. Preserve incomplete status and enforce budgets during
reconstruction, including output cell limits.

The verifier should independently rebuild preparation and every exact
polynomial identity, verify the complete reduced certificate, then check each
mapped cell and all metadata without cloning a large proof. Check cell count
and ordering to reject omissions/duplication, boundary tables and levels,
exceptional sets, root transport, original split signs, and optional merges.
As with the scale wrapper, a fresh global projection of the original input is
unnecessary: exhaustive reduced coverage plus the checked bijection proves
original coverage. The new wrapper needs explicit verifier dispatch and
mutual exclusion of unrelated top-level proof variants.

## Exact root transport

At the `b` axis, a reduced root `alpha` inside `(0,1)` maps increasingly to
`a*alpha/(1-alpha)`. Its source isolating interval may need exact refinement
until both rational endpoints lie strictly inside `(0,1)`, before applying
this formula. Endpoint roots must be identified by their certified ordinal
and isolating interval, not merely by testing `h(0)` or `h(1)`: those could be
different roots of the same polynomial.

- `r=0` becomes the explicit `b=0` boundary.
- `r=1` becomes positive infinity, represented by an absent upper bound.
  A factor `r-1` becomes a nonvanishing power of `a`; a degree drop at this
  pole must not manufacture a finite root.
- A finite interior root should export `RootIndexDomain::Positive`. If its
  source interval is `(lo,hi)`, with `0<lo<=hi<1`, its zero-based positive
  ordinal is `N_h(0,lo) + [lo<hi and h(lo)=0]`, where `N` counts distinct
  roots in the open interval. For an exact point, the second term is absent.
  This works with repeated or irrelevant roots and with either source index
  convention. It must count roots of the polynomial itself, not only roots
  that retained that polynomial as owner in a merged root union.
- Source roots above one map to negative `b<-a`; negative source roots map to
  `-a<b<0`. Consequently full real-root indices are not preserved.
- For every later coordinate, multiplication by the positive, already-fixed
  `L` preserves both real and positive ordinals and scales isolating intervals.

A useful regression polynomial is
`h(r)=(r+2)^2*r*(r-1)*(2*r-1)^3*(r-3)`. The interior root `1/2` has source
real index 2, target real index 3, and positive index 0 in both charts. Its
multiplicity is three; the root at one disappears to infinity. This catches
several tempting but incorrect index shortcuts simultaneously.

Exact rational samples and the reduced certificate fix all fiber data needed
for transport. The existing `roots_in_open_interval` provides distinct-root
counts. Cache repeated transport calculations by exact polynomial/prefix;
do not reuse a count across a different prefix merely because names match.

## Affine dependence does not remove the cylindrical-order requirement

For `q=A(z)+r*B(z)` with `B>0`, endpoint signs at zero and one completely
describe each fixed-`z` fiber: positive throughout, negative throughout, or
one separating root `r=-A/B`. This gives a useful decomposition in the order
`[z,r]`. Its pullback is generally not cylindrical in `[a,b,other x_i]`,
because holding original `x_i` fixed while varying `b` changes `z_i=x_i/L`.

An exact counterexample is

```text
F=(L-x)*(L-2*x)*(L-3*x)+b*x^2/100,  L=a+b,
q=1-6*z+11*z^2-6*z^3+r*z^2/100.
```

Here `dq/dr=z^2/100>0`. Nevertheless, at `a=1/10,x=1`, the signs at
`L=1/2,3/2,5/2,7/2` alternate `-,+,-,+`, giving three distinct positive
`b` roots. In general
`dF/db=L^(D-1)*(D*q+(1-r)*q_r-sum(z_i*q_zi))`; positivity of `q_r` alone
does not control this expression. Endpoint decomposition would require an
additional certified reversal/common refinement before original cylindrical
export, which may recreate the projection cost it was intended to avoid.

The affine pairs identified in A381 and A446 remain worthwhile transformed
CAD candidates. Their exact positive slopes can help bounded-domain
certificates and local projection. They do not, by themselves, prove a direct
original-order decomposition.
