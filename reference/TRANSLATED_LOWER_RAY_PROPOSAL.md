# Constant lower-ray certificates: proposal and A377 evidence

This is a research proposal, not an implemented rule. A narrow extension can
use an original constraint `z > alpha`, with constant rational `alpha`, to
omit cuts proved rootless on that ray. It requires local evidence and checker
changes, but no change of the problem's coordinates or whole-problem proof
wrapper. The already transformed A377 pilot would still need the separately
proposed shear transport before earning original-problem completion credit.

## Where the current implementation stops

`local_projection::positive_coordinate` recognizes only a literal positive
multiple of `z`. `positive_projection_raw`, `stack_roots` and its checker
therefore use endpoint zero. In the double-shear A377 pilot, certified
homogeneity with anchor `x0=1` changes the first wedge to `x3>1`. The current
detector misses even this constant bound. The second wedge remains
`x6>x4+x5` and is outside the proposed first implementation.

The existing domain portfolio already understands constant affine bounds
and shifts polynomials for coefficient-sign certificates. It does not turn
such a bound into the local ray theorem. In particular, `guard_domain(level)`
deliberately excludes constraints whose highest axis is at or beyond that
level. A current-axis rootless certificate needs a distinct proof obligation;
it must not silently widen the assumptions of `certify_guard`.

## Stage one: rootless cuts, ordinary selectors

Require an exact original normalized constraint
`a*(z-alpha)>0`, with rational `a>0`, rational `alpha`, and no other support.
Select one deterministically, preferably the strongest constant lower bound.
The checker reconstructs `a` and `alpha` from the referenced constraint;
they are not trusted proposals. Protect its canonical factor `z-alpha`.

For each other cut `p(b,z)`, where `b` denotes earlier coordinates, form

```text
q(b,w) = p(b,w+alpha) = sum_j c_j(b) w^j
c_j(b) = sum_{i>=j} binomial(i,j) alpha^(i-j) a_i(b).
```

Identically zero coefficients are skipped. Every remaining coefficient must
be nonzero at the rational parent sample and have the same sign. Return all
their exact factors as ordinary ancestor obligations. On the eventual
connected parent cell those signs are fixed, so `q` has that strict sign for
every `w>0`. This proves that `p` has no root above `alpha`; it makes no
assertion about roots below the bound. Require a nonzero endpoint as well;
the irreducible boundary factor itself is handled separately. Sample zeros,
nullification, uncertain arithmetic or exceeded caps use ordinary refinement
or the existing full projection path.

All other cuts retain ordinary leading-coefficient/discriminant obligations
and ordinary pair-resultant obligations. Every nonboundary cut also retains
the endpoint guard `p(b,alpha)`, preventing crossings of the ray boundary.
The complete relevant root union
contains `alpha` and all their roots above it. Their existing full-real
ordinals remain valid because their full-real delineability was retained.
The one open lower sector is rejected by the exact original lower-bound
constraint. Every upper sector still receives a complete ordinary child.
No root-bearing rule or `Positive` selector is widened in this first stage.

Suggested evidence is an optional, mutually exclusive constant-lower-ray
record on `LocalProjectionCertificate`: source constraint, protected boundary
ID, exact endpoint and rootless IDs/signs. The checker must reconstruct the
translations, coefficient factors, relevant union, lower rejection, sector
membership and every propagated obligation independently. Historical
certificates retain the current zero-ray/full-real interpretation.

This is finite: one selected original constant endpoint per axis and one
deterministic translation per interned polynomial. Conservative initial
limits could be degree 16, 128 terms, 4,096 estimated coefficient contributions
and bounded rational/coefficient heights. Constant translation produces at
most `nterms*(degree+1)` monomial contributions. A bounded cache may retain
translations; cache failure has no proof meaning. No adaptive separator stock
is needed for this stage.

An already replayable global nonvanishing certificate could later substitute
for the same-sign test, but only on a box justified by earlier original
constraints plus this current lower bound. All earlier supporting factors
must propagate; the current bound is discharged by the ray split. Using the
existing cheap coefficient test first avoids a new recursive certificate
search.

## Why later constraints cannot supply parent premises

For order `(u,z,y)`, the later constraints `y>0` and `y<u` imply `u>0` on
the final feasible set. They do not prove it on a previously lifted parent
cell containing negative `u`. A proposed root boundary of `z^2-u` is not
even real there. Later rejection of impossible `y` fibers cannot validate an
earlier undefined boundary. Using those constraints would require a separate
exact projection of feasibility and corresponding parent partition. The
current `guard_domain` exclusion is therefore essential.

Likewise, replacing a moving bound `S(b)` by its value at the sample is
unsound: `p(b,w+S(b))`, not `p(b,w+S(b_sample))`, is the relevant identity.
General affine wedges are a later extension with larger expansion and support
obligations. No coordinate-positivity assumption is fabricated from a wedge.

## Stage two: reuse the existing descending selector

If a stronger translated Descartes certificate proves exactly `k` distinct
simple roots above `alpha`, those are the `k` largest distinct real roots.
They can use existing `RealDescending` indices `k-1,...,0` in increasing
geometric order. Collisions below `alpha` do not change these indices. Thus
no new root-index domain is mathematically necessary. This corrects the
initial concern that every translated positive ordinal would need a new
selector. Ordinary `Positive` would still be wrong: it counts from zero.

This stage additionally requires exact endpoint/leading guards, translated
coefficient and separator guards, fixed root counts, all relevant pair-order
obligations, and per-polynomial ordinals even when other cuts own shared union
roots. The checker, comparison and wrapper tests must exercise the existing
descending selector in this new context. A simple adversarial family is
`(w-1)*((w+2)^2-u)` for `-1<u<1`: its sole positive root stays fixed while
negative roots appear or collide. Ascending full-real indices fail; descending
index zero does not. No stage may discard an allowed `F=0` sheet.

## Exact captured A377 evidence

The final shear capture from the
[60-second pilot](A377_BLOCK_SHEAR_PILOT.md) has a quartic and a degree-12
operand in `x3`. For the quartic, put `w=x3-1` and `r=s-4`. Exact Symbolica
expansion gives the following coefficients, low degree first:

```text
c0 =  324 r^3 + 2268 r^2 + 4536 r + 2592
c1 =  648 r^3 + 4122 r^2 + 7326 r + 4464
c2 =  324 r^3 + 1710 r^2 + 1746 r + 720
c3 = -144 r^2 - 1807 r - 3224
c4 = -713 r - 1872.
```

For every `s>4`, the signs are `+,+,+,-,-`: exactly one root, counted with
multiplicity, lies above `x3=1`. It is simple and is the largest real root.
The endpoint also equals `324*s*(s-2)*(s-3)>0`; the leading coefficient is
`980-713*s<0`. This is a concrete stage-two candidate on the full original
kinematic half-domain, with unrestricted `t`.

At the retained prefix `(s,t)=(5,0)`, exact Sturm diagnostics find respectively
one and three roots above one for the two operands. Their shifted sign
variations are also one and three. Neither is rootless, so stage one does not
directly remove this final pair. The translated operands have 16/256 terms
and degrees 4/12. No reduced degree was found.

A bounded numeric pseudo-remainder diagnostic gives a second possible route:
the first two cancellations have mixed coefficient signs and positive roots,
but cancellation eight (degree four) and the complete ninth remainder (degree
three) have uniform signs at that prefix. Only the *first* symbolic
cancellation was constructed: 704 monomial-product estimate, 277 terms,
degree 11. A longer bounded symbolic remainder, its identity and ancestor
guards remain unbuilt and unproved. This is not permission to trust the
numeric remainder or to omit the resultant.

For a small prevalence check, cached factor outputs with highest axis `x3`,
degree at most eight, at most 64 terms and at most 8 KiB were deduplicated.
The first 32 by polynomial SHA were inspected at `(5,0)` in each arm:

| Conditional sample test | Original, endpoint 0 | Shear, endpoint 1 |
|---|---:|---:|
| Structurally small / all distinct x3 outputs | 47 / 53 | 33 / 43 |
| Uniform-sign, nonzero-endpoint candidates | 9 / 32 | 8 / 32 |
| One-variation candidates | 14 / 32 | 13 / 32 |

These are different polynomial sets, not measured avoided work. Coefficient
guards were not factored or proved on parent cells. One shear sample had a
vanishing symbolic coefficient and was excluded. The final counts use the
corrected v2 term-count selection, independently checked by Symbolica parsing.

## Evidence and next decision

Existing frozen helpers only; no source edits, Cargo builds or solver
campaigns ran for this investigation. Each inspection had a five-second hard
cap and at most 2 GiB; the factor sampling used at most 1 GiB. The initial
small pair inspection used CPU10 before its build reservation; later work used
the released CPU8. Artifacts are local under
`artifacts/research/a377-translated-ray-01/`.

| Exact record | SHA-256 |
|---|---|
| Quartic identity check, zero polynomial | `beb3371c16bbe65698a44cf3edb229621bb7365c1a024aa0ad2ede086de2c5fd` |
| Shifted pair inspection | `10eb8192f95a8cc200d24fd51e41be4f24c12b21775e9cbae8c8a06bbd09aa25` |
| Numeric remainder diagnostic | `3e558eb90e8833678eb73dbc0eaf0ea71eb01ad233fb1df9d56a4843dd71f610` |
| Corrected factor selection | `04e41b27a98704a20fefadf82d54fbb82cee956430c8032273d250e81e0db5f5` |
| Corrected candidate counts | `65d5d29d752b419d619ea0e7cabc601bf3996bae67c4c156de7f60cd6b0a9c98` |

The smallest implementation is stage one with unchanged root-bearing
projection. The strongest immediate A377 algebra lead is the exact quartic
stage-two certificate, followed by a bounded symbolic check of the small
remainder candidate. Neither currently establishes a solve-time benefit or
completion.
