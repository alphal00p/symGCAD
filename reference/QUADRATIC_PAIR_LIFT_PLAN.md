# A conditional quadratic-pair lift

Implementation design, 10 October 2026. This is a generic theorem and a
proposed wrapper, not an implemented solver or a completion of the original
unrestricted kinematics. Its cell counts and signed remainder formulas have
independent static review. The endpoint and rational-map algebra already
replayed is recorded in [positive pair roots](POSITIVE_PAIR_ROOT_STRATIFICATION.md)
and [the kinematic map](RATIONAL_KINEMATIC_PAIR_MAP.md).

## Minimal admission theorem

Retain the original coordinate order and all original constraints. Let
`v` denote the complete prefix before the final two axes `x,z`; in the
authentic example this prefix includes `y`. Require the original fibers
to be the unrestricted positive rays `x,z>0`, an exact positive
normalization of the original split polynomial, and

```text
F=q*z+B,
q=a*x^2+p*x+c,
B=b*x^2+E*x+C,
a>0, b>0, h>0, h*E-C>0.
```

All displayed coefficients and `h` are independent of `x,z`, with exact
whole-domain or whole-cell certificates for the strict premises. Admission
uses these identities and proofs, never graph or fixture names.

The endpoint theorem removes the need for the quartic-in-`y` discriminant
of `B` in every regular branch:

- `C>0` forces `E>0`, so `B>0` for every `x>0`.
- `C<0` gives exactly one simple positive root `beta`, with signs `-,+`.
  Its full-real ordinal is 1, as in existing `QuadraticOpposite` evidence.
- `C=0` also gives `B>0` on the positive ray, but an initial generic
  wrapper may record this nonidentically-zero polynomial as an auxiliary
  seam instead of specializing it.

The quadratic `q` needs its own complete positive-root inventory on each
connected prefix cell. Existing positive-ray cases give this from its
leading coefficient, endpoint `c`, coefficient `p`, and discriminant
`p^2-4*a*c`: opposite endpoint signs give one simple positive root; positive
endpoint with nonnegative `p` gives none; the remaining regular cases use
the discriminant and give zero or two. Their required guards and indices
must be replayed. A pair-order proof does not replace this step.

## Signed remainder and joint root order

Reconstruct

```text
a*B-b*q=J*x+K,
R=Res_x(q,B),
b*R=b*K^2-E*J*K+C*J^2.
```

On the `C<0` branch, `q(beta)=-(J*beta+K)/b`. Consequently, when
`J*K<0`,

```text
sign q(beta)=sign(J)*sign(R).
```

Same weak signs of `J,K`, not both zero, determine the opposite strict
sign of `q(beta)` directly. Production already implements the corresponding
same-sign nonintersection rule through `separate_positive_pair`; that part
is not a missing algorithm. If one coefficient is identically zero, the
other supplies the sign. If both vanish, the generic distinct-root theorem
does not apply; a separate proportional-polynomial case or exact fallback
is required. An identically zero polynomial must never be used as an
exception that removes the entire domain.

For two simple positive roots `alpha1<alpha2` of `q`:

| Exact signs at beta | Joint order |
|---|---|
| `q(beta)<0` | `alpha1<beta<alpha2` |
| `q(beta)>0`, `q_x(beta)<0` | `beta<alpha1<alpha2` |
| `q(beta)>0`, `q_x(beta)>0` | `alpha1<alpha2<beta` |

For one simple positive root `alpha`, the sign of `q(beta)` directly
distinguishes `beta<alpha` from `alpha<beta`, because `q` has signs `-,+`.
Equality is a genuine collision seam.

A signed derivative witness can use
`V=b*p^2-2*a*E*p+4*a^2*C`: when `p<0`,
`sign q_x(beta)=-sign(V)`. But this extra norm need not be projected.
The smaller initial implementation should isolate the complete `q,B`
root union at the exact rational sample and reuse that order after proving
whole-cell positive nonintersection. Continuous simple root branches cannot
exchange order without a collision. Thus an otherwise unnecessary `V`
guard would add work without strengthening the required topology proof.

## Exact lift and generic cell inventory

For each certified prefix cell, arrange every positive `q` root and `beta`
when present. On every resulting `x` sector, retain the signs of both
polynomials and restore `z` as follows:

| sign q | sign B | Positive-z lift |
|---|---|---|
| + | + | One positive F cell |
| - | - | One negative F cell |
| + | - | Cut at original `F=0`: negative below, positive above |
| - | + | Cut at original `F=0`: positive below, negative above |

The cut is the unique positive root `z=-B/q` of the **original** signed
split polynomial. Its bound must be exported, with the positive
normalization accounted for. For distinct simple roots, the exact number
of emitted open `(x,z)` cells is:

| Positive q roots | No positive B root | One positive B root |
|---:|---:|---:|
| 0 | 1 | 3 |
| 1 | 3 | 4 |
| 2 | 4 | 6 |

This replaces a guessed row count with a verifier-checkable exhaustive
inventory. Keep the `q=0` and `B=0` auxiliary sections explicit. At
`q=B=0` the entire positive `z` fiber belongs to the original zero set;
the coincidence is not an absent boundary. A generic wrapper can record
nonidentically-zero auxiliary seams, but cannot erase the original zero
sheets on either open side or count a seam as an open full-dimensional cell.

## The remaining base problem is explicit

The minimal wrapper reduces the original problem by two fiber variables,
leaving the original prefix through `y`. It requires a certified,
exhaustive prefix decomposition with these conditional targets:

1. Classify `C` and the ordinary quadratic-root guards for `q`.
2. On `C<0` where `q` has a positive root, classify the remainder
   coefficients `J,K`. An empty positive-root inventory for `q` needs no
   pair-order guard, since then `q>0` throughout the positive ray.
3. On those branches with `J*K<0`, classify `R` and preserve every positive
   common-root boundary. Same-sign remainder branches do not need `R`.
4. Retain all required lower-dimensional seams and every original base
   constraint, including unrestricted original external parameters through
   the exhaustive kinematic chamber split.

The residual prefix solve must itself be complete and independently
verified. Conditional targets require guarded subproblems or an equivalent
verified local projection, rather than silently adding all inequalities to
the original input. A first fallback may retain ordinary resultant
projection wherever the conditional proof is unavailable.

For the authenticated complement, `C` is quadratic in `y`, the `q`
endpoint/discriminant guards are at most quadratic in `y`, `J` is quadratic
with one positive root, and `K` is a strictly concave cubic. However `R`
remains the six-degree, 394-term polynomial from the preceding research.
An ordinary sign-invariant prefix CAD of `R` can still encounter the
documented irrelevant negative-common-coordinate collision. The signed
remainder formula is not permission to omit its discriminant while using
ordinary resultant root ordinals.

The concrete structural saving is that **no unconditional discriminant of
`B` is needed**; the two final fibers have an exact bounded lift. This may
reduce projection and repeated local discovery, but it is not yet proof
that the remaining prefix solve is cheaper or will finish.

## The stronger gate needed to bypass the known obstruction

To improve beyond ordinary `R` projection, introduce a certified selector
of common solutions with `x,y>0`, or a certified resultant-root index
relative to an explicitly bounded `J,K` sign sector. Existing `Positive`
indices count positive `y` alone and do not provide either selector.
For each whole earlier-base cell, the certificate must prove finite and
ordered selected branches, absence or explicit stratification of positive
interior critical points and endpoint contacts, and the handling of
`J=K=0`. The original coordinate order is unchanged.

The rational-map alternative proposes the `s`-free system `B=D=0` for
stationary points and then orders their `S` images. Its measured resultant
has 5,863 terms and degree eleven in `y`; it removes one external parameter
but is not automatically a smaller elimination problem. The factorization
measurement and a single whole-region positive-selector replay should
precede any production claim.

## Minimal proof interface

Reuse the extraction, fresh positivity replay, base `SolveResult`, root
isolation, original-F bound restoration, and row-count checking patterns
of `src/quadratic_ladder.rs`. A new evidence type would additionally bind
the endpoint multiplier, the residual guarded prefix solve, and each joint
root inventory. The checker reconstructs coefficients and remainder norms
from the original split, verifies all strict certificates and prefix cells,
then independently regenerates the root order and emitted cells.

If integrated into the local engine rather than a wrapper, witnesses must
refer to actual canonical cut identifiers and scalar units, including
parent-factor relations when the registry factors `q` or `B`. Existing
`PositiveRayRule`, full root-union lifting, ancestor guards, and checker
replay should remain authoritative. The narrower interface discussion is
in [positive-root remainder evidence](POSITIVE_ROOT_REMAINDER_INTERFACE.md).
No production implementation is authorized by this design note alone.

## First implementation gate

Start with unrelated rational examples covering all six inventory entries,
then parameterized examples that force changes of order. With `h=2`, use
`q=x^2+1`, `q=x^2-1`, or `q=x^2-3*x+2`, paired with either
`B=x^2+x+1` or `B=x^2-9`; all six pairs meet the endpoint premise and
have distinct positive roots. For the two-root `q`, additionally use
`B=x^2-1/4` and `B=x^2-2` to exercise beta below and between its roots.
Check the exact emitted counts, root ordinals, signs, and original-F bounds.

The family `q=x^2-3*x+2`, `B=x^2-t`, `t>0` must preserve both genuine
collisions `t=1,4`. The existing negative-collision example
`q=x^2+x-2-t`, `B=x^2-2`, `-2<t<0` should need no pair-resultant guard:
its same-sign remainder proves positive nonintersection even at
`t=-sqrt(2)`. This is already a supported local separation, not a new claim.
Proof mutations must reject a missing prefix guard, changed signed unit,
wrong union order, missing original-F bound, and an incomplete cell list.
Only after this small replay gate should an authentic conditional prefix
solve measure whether the wrapper reduces total work.
