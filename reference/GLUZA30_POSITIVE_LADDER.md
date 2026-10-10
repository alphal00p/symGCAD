# A two-fiber positive ladder for Gluza 30

This is an exact reduction proposal for the original positive problem
`0332_test_gluza_ex30_solve_3.toml`. It preserves ordinary cylindrical cells in
the original coordinates. A completed reduced problem alone is not yet an
independently verified decomposition of the original problem.

The legal lifting order is
`[x2,x6,x7,x5,x1,x4,x0]`. Write, without changing any variable,

\[
 F=a x_0x_4+b x_0+c x_4+d,
 \qquad A_0=a x_4+b,\quad B_0=c x_4+d.
\]

Here `a,b,c,d` depend only on the five preceding coordinates. The final endpoint
`d` is the sum of eight positive monomials:

\[
d=x_1x_2+x_1x_5+x_1x_6+x_2x_5+x_2x_7+x_5x_6+x_5x_7+x_6x_7>0.
\]

The existing graph guide `U` supplies the exact comparisons

\[
 U=C_0x_0+D_0,\qquad
 W_0=B_0C_0-A_0D_0>0,
\]
\[
 C_0=C_4x_4+D_4,\qquad W_4=bC_4-aD_4>0.
\]

Every displayed `C`, `D`, and `W` is a nonzero polynomial with strictly positive
coefficients. Thus these inequalities hold throughout the original positive
orthant, not just at a sample. The determinantal identity independently gives

\[
 bc-ad=H_{04}^2\geq0.
\]

The helper recomputes all identities in exact Symbolica arithmetic, treating
both the guide JSON and the proposed square from the earlier research report
as untrusted expressions. `H04` itself changes sign; no strict nonvanishing
claim is made for it.

## Exhaustive fiber cases

On a connected base cell where `a,b,c` are nonzero and have fixed signs, define
`rA=-b/a` when `a<0,b>0` and `rB=-d/c` when `c<0`. Only these five sign patterns
are possible:

| Signs `(a,b,c)` | Positive `x4` interval | Positive `x0` interval |
| --- | --- | --- |
| `(+,+,+)` | `(0,infinity)` | `(0,infinity)` |
| `(-,-,+)` | `(0,infinity)` | `(0,-B0/A0)` |
| `(-,-,-)` | `(0,rB)` | `(0,-B0/A0)` |
| `(-,+,+)` | `(0,rA)` | `(0,infinity)` |
| `(-,+,+)` | `(rA,infinity)` | `(0,-B0/A0)` |
| `(-,+,-)` | `(0,rA)` | `(0,infinity)` |
| `(-,+,-)` | `(rA,rB)` | `(0,-B0/A0)` |

The first comparison explains the important restriction: if `A0>=0`, then
`B0>0`, since `C0,D0,W0>0`. The derivative comparison similarly proves
`a>0 => b>0`. Then `bc>=ad>0` forces `c>0`.

When `a<0,b>0,c<0`, the square identity gives `rA<=rB`. Equality would produce
a positive `x4` with `A0=B0=0`, contradicting `W0>0`. Therefore `rA<rB`
strictly, and the final interval is never empty. This strict guide premise
cannot be replaced by the nonnegative Rayleigh square alone: the polynomial
`-(x0+1)*(x4-1)` has `d>0` and `bc-ad=0` but coincident endpoints.

All upper bounds `-B0/A0` in the table are strictly positive with `A0<0`.
They are simple linear roots in the current axis and continuous on the stated
base cell. Each interval has either a finite continuous bound throughout or
the constant bound `infinity` throughout. There is no projective transport,
variable finite/infinite bound, or relaxation of the cell definition.

The table exhausts `F>0` away from explicitly retained seams. At most two
positive cells are produced per five-dimensional base cell. A production
certificate would need to replay the base decomposition and every table row,
preserve all base exceptional factors, and retain the nonzero polynomial
seams `a,b,c,A0,B0,F` where relevant. Identically zero polynomials must never be
introduced as exceptional nonzero obligations.

## Why complementation proves the sign but does not remove these cuts

A nonzero multiaffine polynomial cannot have a local nonnegative minimum of
zero inside the positive orthant. On a sufficiently small rectangular box,
multilinear interpolation expresses its center value as a strictly positive
convex combination of all vertex values. Nonnegativity and a zero center
therefore force every vertex value to zero, hence force the polynomial to
vanish on the whole box. This contradicts its being a nonzero polynomial.
Applying the same argument to `-F` shows that every interior zero has both
signs arbitrarily nearby.

Consequently, after an independently verified generic negative decomposition
with nonzero algebraic exceptional factors, the positive region is exactly
the positive orthant minus the closure of the negative cells, with closure
taken relative to the orthant. Density supplies any negative points omitted
by exceptional seams. No numerical membership oracle or analytic maximum
principle is needed for this set equality.

The equality does not by itself produce a cylindrical decomposition. For
`F=x-1-y` on `x,y>0`, the negative region is the single valid cell
`x>0, y>max(0,x-1)`. Reversing the final bound gives an empty fiber whenever
`x<=1`. The positive cell requires the new ancestor restriction
`x>1, 0<y<x-1`. Gluza 30 has the same issue already at its first restored
negative axis: `x5>max(0,1-x6-x7)`. Its complementary branch requires
`x6+x7<1` before the bound can be reversed. The table above is a finite,
explicit treatment of this nonempty-fiber obligation for the last two axes.

## A further exact base identity

With `r=x6+x7`,

\[
 T=x_5+r-1,\qquad
 K=(1-r)(1+x_2)-x_5(r+x_2),\qquad a=K-Tx_1.
\]

For `r>1`, both `T>0` and `K<0`, so `a<0`. For `r<1`, the two positive
thresholds in `x5` are

\[
 z_T=1-r,\qquad z_K=\frac{(1-r)(1+x_2)}{r+x_2},\qquad z_T<z_K.
\]

Below `zT`, `a>0` for every positive `x1`. Between `zT` and `zK`, `a` changes
sign at `x1=K/T`; above `zK`, `a<0`. This identity is recorded for further
analysis; the first experiment uses an ordinary complete sign arrangement of
`a,b,c` and does not assume this specialized base schedule.

## Reproduction and scope

`examples/gluza_positive_ladder.rs` checks the original target, all seven
positive-coordinate premises, group/order compatibility, both strict guide
identities, the square identity, and the base identity. It then writes two
research problems on `[x2,x6,x7,x5,x1]>0`, split by `[a,b,c]`, for the optimized
and local engines. Generated constraints are parsed before export.

Two exact regressions check the real identities, malformed square/order/group
proposals, and the coincident-bound counterexample. Trial metadata and any
independent reduced-proof verification are stored under
`artifacts/research/gluza-positive-ladder-02`. This report does not claim an
original completion until an original-problem reconstruction certificate has
been implemented and independently replayed.

The first two valid trials used the same frozen debug worker
`607dd85fd0aad160da25f6f947d90d450033f34405bf59d7e8069a0b21a774cc`,
CPU 8, 2 GiB, cold operation checkpoints, and 60 seconds each. Exact proposal
preparation took about 11 ms and is recorded separately.

| Reduced engine | Outcome | Final observed work | Peak RSS |
| --- | --- | --- | --- |
| local | timeout, 60.023 s | 151,629 speculative nodes; 14,834 refinements | 41,976 KiB |
| optimized | timeout, 60.105 s | complete projection with 3,326 final factors, degree at most 32 | 773,944 KiB |

The local run again spends substantial time revisiting stacks: its recorded
factor, resultant, discriminant, root, pair, and domain phases total about
19.8 seconds, versus the 60-second wall limit. The global run's projection
counts were `8 -> 14 -> 29 -> 120 -> 3326`; the final elimination itself took
18.0 seconds. There was no completed base proof or independent verification
from either run. These are exploratory debug observations, not a matched
speedup claim or evidence against other legal variable orders.

The earlier directory `gluza-positive-ladder-01` preserves an invalid-input
attempt: its first helper version emitted a bare coordinate expression in
the strict-constraint field. The worker rejected this before solving. Version
02 emits `x>0` and parses the generated problem before export; the rejected
attempt is not counted as an algorithmic timeout.

A subsequent release MCTS experiment used worker
`f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`,
CPU 8 and a 4-GiB process limit. The outer 120-second budget included search
and solve; the public supervisor reserved 5% for export. Although the input
requested 90 search seconds, the solver's existing 20%-of-campaign bound
limited search to 22.825 seconds. Fourteen trials covered thirteen distinct
orders, without a completed candidate. The suggested order
`[x5,x1,x6,x2,x7]` reached an allocation failure in the final bounded solve;
the enclosing process ended after 96.561 seconds with peak RSS 4,153,884 KiB.
The result is unverified and labeled `computation_failure`, with the exact
allocation diagnostic retained. This is a resource failure, not an inferred
mathematical obstruction. The durable search journal and all trial progress
are preserved under `base-mcts-release-valid`.

Two explicit release orders were also tested for 30 seconds each:

| Order | Last projection factor count / degree | Wall / peak RSS |
| --- | --- | --- |
| `[x6,x7,x5,x2,x1]` | 5,062 / 36 | 30.096 s / 710,004 KiB |
| `[x7,x6,x5,x2,x1]` | 5,093 / 36 | 30.068 s / 650,884 KiB |

Both timed out; their final elimination took 17.0/17.4 seconds. These short
runs do not establish that the resulting finite lifting could not complete
with a larger bound. They also do not show that merely putting coordinate
thresholds first reduces global projection size. All five reduced problems
above remain incomplete, and the original positive problem receives no new
completion credit.
