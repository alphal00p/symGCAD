# Positive Gluza domains: exact one-fiber reduction and bounded screen

The negative Gluza30 and Gluza33 solutions do not automatically solve their
positive counterparts. This note records an exact positive-fiber theorem and
research-only reduced-problem measurements. No result in this note is credited
as a decomposition of either original positive problem.

## Exact positive-fiber rule

Write `q=A*z+B` and a proposed positive guide as `U=C*z+D`, with `z>0`.
Suppose `C,D>0` and `W=B*C-A*D>=0` over the entire remaining positive domain.
The exact graph identities in the [Gluza30](GLUZA30_RAYLEIGH_REDUCTION.md) and
[Gluza33](GLUZA33_RAYLEIGH_REDUCTION.md) reports provide these premises for all
seven Gluza30 coordinates and the massless Gluza33 coordinates `x3,x5`.

If `B<=0`, the comparison identity implies `A<=B*C/D<=0`, so `q<=0` on the
positive ray. Therefore

```text
q>0 and z>0
    iff B>0 and [ A>=0 or (A<0 and 0<z<-B/A) ].
```

A conventional cylindrical implementation can solve the exact reduced
problem `B>0` with split polynomial `A`, then restore one positive fiber over
each accepted base cell. The upper endpoint is infinite on `A>0` and the
unique linear root on `A<0`. The nonconstant seam `A=0` must be explicitly
retained in a generic decomposition's exceptional set. If `A` is identically
zero, the whole positive ray is admissible; it must not be excluded as a zero
exception polynomial. The seam `B=0` contains no points of the requested
strict positive domain under these premises.

The reciprocal-coordinate negative theorem preserves the comparison
identity but does not remove the upper-endpoint switch in the original
coordinates. A single finite-root boundary cannot represent this switch
without a base partition. No reciprocal transport is assumed here.

## Bounded ordinary-base screen

`examples/positive_linear_bases.rs` constructs each exact endpoint `B` and
leading coefficient `A` with Symbolica, independently reconstructs the
factorization of `W`, checks strict coefficient positivity of `C,D`, and
checks the sign of every odd-multiplicity factor of `W`. Mixed-sign factors
are permitted only with even multiplicity. Each prepared base preserves the
other original coordinate constraints and solves `B>0` with split `A`.

The first screen used the immutable debug worker from production `18cd5d0`,
SHA-256 `4da623f39bc7f552fd3dbb716f85162d7000f92a3d0178b5685333362dd173af`.
Each solve had one CPU, 4 GiB, five seconds and a 1,000-cell cap. Both the
optimized global and local engines used Brown ordering and exact positivity
pruning. No operation/result cache was supplied. The outer Rust supervisor
limited each original fixture's entire screen to 180 seconds and 4 GiB.

| Original positive fixture | Eligible first axes | Endpoint / leading terms | Outcome |
|---|---|---|---|
| `0332_test_gluza_ex30_solve_3.toml` | `x0,x4` | 27 / 32 | All four engine trials timed out |
| same | `x1,x2,x6,x7` | 35 / 24 | Local `x1,x2` reached the cell cap; the other six timed out |
| same | `x5` | 33 / 26 | Both engines timed out |
| `0338_test_gluza_ex33_solve_3.toml` | `x3,x5` | 86 / 57 | All four engine trials timed out |

The Gluza30 screen took 65.547 seconds internally; the Gluza33 screen took
21.670 seconds. The two local cell-cap exits occurred in 1.864 and 1.838
seconds and exported no complete cells. They are incomplete search results,
not evidence that the domain is empty. No reduced base completed, so this
screen does not establish a best first axis. The smaller `x0,x4` endpoint
support is only a structural heuristic for a follow-up.

Artifacts are `artifacts/research/positive-linear-bases-01/gluza30.json` and
`gluza33.json`, with fresh per-trial inputs and process records in the adjacent
`.cases` directories. The frozen first helper SHA-256 is
`59e128556f63c90052be8b53ecfaa26759ad99fba5eff440f6f6b2f52c9e2cd3`;
its source SHA-256 is
`58555460da55d14817f522f409944f3843365b86101f43f3a7bd8ca6e4ebe7e8`.

These are single short debug screening measurements. They do not measure
runtime parity or establish that the reduced problems are intractable.

A second screen retained the first peel and exact structural lifting order
from each successful negative problem, with 30 seconds and a 10,000-cell cap
per engine. The Gluza30 `x0` base used
`[x2,x6,x7,x5,x1,x4]`; the Gluza33 `x3` base used
`[x6,x1,x4,x0,x2,x7,x5]`. All four solves timed out. The two complete screening
processes took 61.623 and 61.593 seconds internally. Their reports are under
`artifacts/research/positive-linear-bases-02/`; helper SHA-256 is
`9bf62e8c43e2b87b466304f0368dda487c27c0187fa31384973a96f7a906e96b`,
and its copied source SHA-256 is
`fad217382eaf83c95e1a31992cf209c3af283efeffcf1875d4cc88609e8faec1`.

## Exact endpoint chains

The helper's `--algebra-only` mode checks hereditary endpoint identities,
retaining the exact current target, guide, coefficients and square factors.
For Gluza30, setting `x0=0` and then `x4=0` gives the positive terminal

```text
x1*x2+x1*x5+x1*x6+x2*x5+x2*x7+x5*x6+x5*x7+x6*x7.
```

Both steps have strictly coefficient-positive `C,D`, with comparison square
roots of 18 and 12 terms respectively, all coefficient-positive. Thus the
positive fiber theorem applies twice over a completely positive terminal
base. This would give a two-fiber compactified chart, subject to the original
coordinate export restriction below. The second endpoint comparison is
different from the six-term square in the negative derivative chain.

For Gluza33, endpoint steps `x3,x5` preserve the nonnegative comparison
identities, but leave a 50-term mixed-sign terminal polynomial. The first
comparison's square root has mixed coefficients; the second has six
positive terms and is strictly nonzero. No terminal positivity claim follows.

Full identities are in
`artifacts/research/positive-linear-endpoints-03/{gluza30,gluza33}.json`.
The frozen helper SHA-256 is
`8f15bf3fd24894d63fc9e25f4c68c8099aa62d55229802e423fbd4e4994cfd28`,
with copied source SHA-256
`e4af56a80b0b15d0a5cff6bf82f1dd18a3d55558b627285d0e41c544a2787afb`.
Three helper tests cover exact nonnegative factor reconstruction, rejection
of negative/mixed odd factors, and the distinction between a nonnegative
mixed square and a strictly nonzero square. Runtime identity checks were
successful for both original fixtures. These are algebraic evidence only.

## Compactifying a fiber does not preserve the original cell presentation

The increasing bijection `w=z/(1+z)` maps the positive ray to `(0,1)` and
preserves the sign through the positive multiplier `1-w`:

```text
(1-w)*q(w/(1-w)) = (A-B)*w+B.
```

On `B>0`, the positive fiber in the new coordinate is `0<w<g`, where `g`
is the smallest strictly positive root of
`(w-1)*((A-B)*w+B)`. It equals `B/(B-A)` when `A<0` and `1` when `A>=0`.
Thus `g` is finite, positive and continuous, including the double-root event
`A=0`. The other linear factor becomes the nonzero constant `B` when
`A=B`, so this degree drop does not invalidate the selected root `1`.
These elementary facts are exact; they do not require generic degree
invariance of the complete defining polynomial.

However, the inverse upper bound `g/(1-g)` is infinite on one part of the
base and finite on another. A standard cell uses a finite continuous
real-valued boundary function or a constant infinite endpoint; the
definition does not allow these cases to switch inside one base cell.
See [Strzeboński, §2, p.4](https://arxiv.org/pdf/1405.4925) and
[Nalbach, Definition3.1](https://publications.rwth-aachen.de/record/1022295/files/1022295.pdf).
The independent presentation in
[Davenport, Locatelli and Sankaran, Definition3.1](https://people.bath.ac.uk/masgks/Papers/regularcad.pdf)
also constructs stacks from continuous real-valued sections.

For example, take `x>0`, `A=x-1`, `B=x`, and `C=D=1`. The strict comparison
is `B*C-A*D=1`. The compactified upper boundary is `min(x,1)`, whereas the
original positive fiber is bounded by `x/(1-x)` for `0<x<1` and is the
whole positive ray for `x>=1`. A single original-coordinate strip with a
finite continuous upper function or a constant infinite upper endpoint
cannot describe both parts. Splitting the base at `x=1` resolves this.

Consequently a compactified decomposition is a useful exact mapped-region
description, but it is not an original-coordinate CAD completion under the
saved plan without additional refinement/transport. This obstruction is
different from the implemented negative-domain bound `max(0,-B/A)`:
that lower bound stays finite and continuous everywhere on `A<0`, even at
its kink. No production export change is proposed here.

## Strict comparison identities can still reduce pair projection

There is a separate possible optimization that preserves the original
coordinates. If the exact identity `C*B-D*A=H^2` has a strictly nonzero `H`
throughout the positive base, then `A` and `B` have no common zero there.
Any factor of `A` and any factor of `B` therefore cannot share a positive
fiber root in any remaining coordinate. Under the existing individual
root-delineability obligations, this supplies a sound reason to omit that
pair's resultant. The identity, strict-domain certificate and exact factor
divisibility must all be checked; sample separation alone is insufficient.

The strict coefficient-positive square roots in the negative Gluza30
derivative chain provide candidate identities. A mixed-sign square root,
such as the Gluza33 second-step numerator, only proves nonnegativity and
does not supply this global no-common-zero argument. Applicability counts
and end-to-end benefit remain unmeasured.
