# Positive-root remainder signs: existing support and a narrow extension

This is a read-only design review, not an implemented certificate or a new
corpus result. It concerns actual ordered polynomial cuts, not a graph or
fixture identifier. The related research identities are in
[RATIONAL_KINEMATIC_PAIR_MAP.md](RATIONAL_KINEMATIC_PAIR_MAP.md).

## The same-sign omission already exists

`src/local_projection.rs::separate_positive_pair` forms a division-free
leading cancellation, optionally followed by one more cancellation. If all
nonzero coefficients of the resulting polynomial have the same strict sign,
it records `PositivePairSeparation` and returns those coefficients as
ancestor nonvanishing obligations. For equal-degree cuts satisfying
`B=b*q+J*x+K`, the first cancellation is a nonzero scalar orientation of
`J*x+K`. The current path therefore already handles the proposed same-sign
linear remainder within its bounded multiplication envelope.

This proves no common positive root of the two cuts; **uniqueness of a
positive root is unnecessary**. An identically zero coefficient is absent
from the sparse remainder. A nonconstant coefficient that vanishes at the
sample makes the shortcut decline. Sample signs only choose the attempt:
the returned guards must persist over the entire connected base cell.

`positive_projection_raw` first retains each cut's individual root,
leading-coefficient and endpoint obligations. `Checker::walk` recomputes
the pair identity and raw guards, checks the complete retained/removed
guard inventory, and independently isolates the full relevant root union.
The original positive-coordinate constraint rejects the negative half-line.
No original cut or positive interior boundary is removed by this shortcut.
Deferred discovery still passes through the same final projection/replay.

Existing tests in `tests/local_projection.rs` cover negative-only
intersections, signed/unequal-degree inputs, mixed or zero sample
coefficients, changed identity metadata, missing guards and legacy options.
No additional production implementation is warranted for this case alone.

## What a signed certificate would add

On a connected base cell, suppose

```text
B = a*x^2 + E*x + C,       a>0, C<0,
d*P = A*B + J_P*x + K_P,  d>0.
```

All coefficients of `B`, `d`, `J_P` and `K_P` depend only on earlier axes.
The identity is exact in the complete variable map. The hypotheses give
exactly one simple positive root `beta`, with `B<0` below it and `B>0`
above it. Same weak signs of `J_P,K_P`, not both zero, give that strict
sign for `P(beta)`. If `J_P*K_P<0`, set

```text
R_P = a*K_P^2 - E*J_P*K_P + C*J_P^2.
sign P(beta) = -sign(J_P)*sign(R_P).
```

Indeed `w=-K_P/J_P>0`, `R_P=J_P^2*B(w)`, and
`sign(beta-w)=-sign(B(w))`. Zero `R_P` means a genuine common positive
root in this branch. If both remainder coefficients vanish, `P(beta)=0`.
The authentic identity `B=b*q+J*x+K` instead gives `d=b`, `P=q`,
`J_P=-J`, `K_P=-K`; losing that minus sign would reverse the answer.

This can encode an ordering statement, rather than merely nonintersection.
For a positively oriented quadratic `q` with two already certified simple
positive roots, `q(beta)<0` places beta between them; when `q(beta)>0`,
the sign of `q_x(beta)` distinguishes below both from above both. The
individual inventory of `q` is a prerequisite, not a consequence.

The opposite-sign rule does **not** automatically reduce projection work.
For fixed `n=degree_x(P)>=1`, the exact identities imply

```text
d^2 * Res_x(B,P) = a^(n-1) * R_P.
```

This formula uses the displayed homogeneous expression for `R_P`, even
when `J_P` specializes to zero. On a cell where `a,d` are nonzero, its
zero set is the ordinary pair-resultant zero set. Constructing the small
norm may be cheaper than a general resultant, but all its relevant zeros
remain. A performance claim needs captured-operation evidence. Simply
adding its guard reproduces the ordinary projection obligation.

For monic `q=x^2+b2*x+c2` and `B=b*q+J*x+K`, `b>0`, the smaller norm
`Rq=K^2-b2*J*K+c2*J^2` is already `Res(q,B)`. The norm using B is
`b*Rq`; on `J*K<0`, `sign q(beta)=sign(J)*sign(Rq)`. Retaining the
extra positive leading factor provides no new boundary information.

## Minimal replay/interface contract

Start, if measurements justify it, with a bounded certificate for a
quadratic `B` already classified as `QuadraticOpposite` by
`positive_projection_raw`. Bind actual immutable canonical registry IDs,
the current axis and the signed reduction. Derived targets such as `q_x`
need an explicit derivation from an existing ID. Declared parent
polynomials that have been factored into several cuts require an exact
parent/factor binding; an expression-level identity alone is insufficient.

Discovery may use the rational prefix to select signs. Fresh replay must
reconstruct the reduction and `R_P`, validate the full maps and axis
support, and return every nonzero coefficient/multiplier/branch guard to
the ordinary ancestor mechanism. Domain certificates may discharge guards
only through the existing original-bound provenance and supporting-factor
obligations. Neither the target inequality nor a sampled root ordering is
a premise. Keep all individual cut obligations and the full root union.

A small optional witness list on `PositiveRayProjection`, absent in legacy
proofs, would be a narrower interface than a new CAD engine. Store bounded
derivation metadata and claimed signs; do not trust serialized polynomial
text or discovery caches. Static inapplicability falls back to ordinary
projection. Entered arithmetic/resource failures retain their typed stop;
invalid supplied proof evidence must fail verification. Any new discovery
templates need a finite lifetime inventory, including across failed
branches, and appropriate archive/context separation.

For pair omission alone, a bounded extension of leading cancellation to a
linear pseudo-remainder is simpler and still needs no unique-root theorem.
For a signed claim the multiplier's sign is essential: nonzero alone does
not justify `sign P(beta)=sign(J_P*beta+K_P)`.

## Selectors, seams and counterexamples

* `QuadraticOpposite` already has full-real ordinal 1. More general unique
  positive roots require the existing `Positive(0)` selector unless a
  stronger real-root theorem is retained. For example,
  `B=x^3+3*x^2-t`, `t>0`, has one simple positive root throughout, but its
  negative roots collide at `t=4`; that positive root's full-real ordinal
  changes from 2 to 0. A sign witness does not repair a wrong ordinal.
* The quadratic sign formula needs the certified unique-positive inventory.
  With `B=(x-1)*(x-3)`, `J_P=1,K_P=-2`, it gives the wrong sign at beta=1.
  Knowing a sign at just one of several positive roots cannot omit the pair.
* Same-sign guards can legitimately avoid negative collisions: for
  `B=x^2-2`, `P=x^2+x-2-t`, the remainder is `x-t`. On `t<0` it is
  positive on the whole positive ray, although the ordinary resultant
  `t^2-2` vanishes at the irrelevant negative intersection `t=-sqrt(2)`.
* A positive sample is insufficient: with `B=x^2-2` and
  `P=x^2+t*x-2`, the remainder is `t*x`; a shared positive root appears
  at `t=0`. Omitting the ancestor guard loses a real collision.
* Weak sign across a nonconstant zero needs an independently checked
  nonnegativity proof, not the current strict guard mechanism. Two weak
  coefficients may vanish together. Keep `J_P=K_P=0`, endpoint contacts,
  leading drops, repeated-root events and `R_P=0` obligations; equality at
  one sample is not an identically zero polynomial.

No new selector is needed while existing cut roots and their complete
union remain the exported boundaries. Selecting only some roots of a
resultant according to an existential positive common root is a larger
certificate problem; ordinary `Real`, `Positive` or `RealDescending`
indices of that resultant do not express that filter.

Focused acceptance tests should include the examples above, both signs of
canonical units, zero remainder coefficients, denominator-sign mutations,
full-map/inactive-axis mutations, removed ancestor guards, changed norm or
reduction data, missing root bounds, cap declines and legacy proof replay.
Only then should a bounded captured pair compare cost and exact output
against the existing cancellation/resultant path.
