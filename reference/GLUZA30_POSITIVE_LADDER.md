# A two-fiber positive ladder for Gluza 30

This is an exact reduction for the original positive problem
`0332_test_gluza_ex30_solve_3.toml`. It preserves ordinary cylindrical cells in
the original coordinates. A completed reduced problem alone is not yet an
independently verified decomposition of the original problem; the opt-in
production wrapper below also reconstructs and checks the original fibers.

The original positive Gluza30 problem has now completed and independently
verified with **80,980 cells**, using the legal lifting order
`[x1,x7,x2,x6,x5,x4,x0]`. The bounded search and retained unsuccessful trials
are documented below; symbolic A446 and positive Gluza33 remain separate
unresolved problems.

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

## Opt-in production certificate

The `bilinear_positive` engine implements this table for a generic supplied
guide, with the last two axes of an explicit/given legal order playing `y,x`.
It accepts one strict target `F>0`, literal positive-coordinate constraints
for every variable, and optional positive constants. Extra domain restrictions,
homogeneous transport, and identically zero `a,b,c` are currently inapplicable.
In particular, the inference `a>0 => c>0` uses arbitrarily large positive `y`;
it must not be reused unchanged on a bounded `y` domain.

Set `solver.bilinear_positive_guide` to the guide expression and
`solver.bilinear_positive_base_engine` to `baseline`, `optimized` (default),
or `local`. The wrapper's seven strict certificates are for
`d,C0,D0,W0,C4,D4,W4`, on the full coordinate orthant with the original target
explicitly excluded. It never assumes `F>0` when proving its own exhaustive
fiber classification. Baseline delegated solves clear optimized pruning
flags; this does not weaken the wrapper's mandatory certificates.

The production proof does **not** need the extra Rayleigh-square identity.
When `a<0,b>0,c<0`, evaluate `W0` at the positive root `rA` of `A0`:
`W0(rA)=B0(rA)*C0(rA)>0`, so `B0(rA)>0` and `rA<rB`. Similarly, `a>0`
forces `b>0` via `W4`; if `c<0`, sufficiently large positive `y` would give
`A0>0,B0<0`, contradicting `W0>0`. This is a strictly smaller set of
obligations than the initial research helper's independent checks.

Evidence stores the seven exact domain certificates and the full delegated
base proof. Verification reconstructs the expected base problem/options,
verifies its exhaustive coverage, and streams the expected original cells
against the supplied inventory. It independently checks the complete
polynomial/boundary table, exceptional seams, samples, linear root selectors,
raw-cell count, and any compatibility merge. A failed or partial base cannot
yield original completion; restoration also enforces wall and cell limits.
No second complete restored-cell vector is allocated during verification.

Integration regressions cover all five coefficient-sign patterns through
exact common refinement against the baseline, all three delegated engines,
original group/order preservation, malformed guides and domain premises,
zero-width and zero-coefficient exclusions, cell/exception/sample/certificate
tampering, incomplete bases, late restoration limits, and serialized replay.
This establishes the general wrapper on the tested problems. The subsequent
original Gluza30 proof also replays its complete delegated base and all
80,980 restored cells independently.

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
above remain incomplete; they did not receive completion credit. The later
successful search below retains all these unsuccessful observations.

## Verified original problem after bounded ordering search

A longer cold release search used seed zero, at most 256 rollouts, a
300-second search cap, one CPU and 4 GiB. Its enclosing problem budget was
1,500 seconds, so the 20% rule permitted the full search cap. It performed
104 trials in 299.392 seconds (300.026 seconds including setup/export).
Exactly one trial completed: order `[x1,x7,x2,x6,x5]`, with 15,389 base cells
in 5.046 seconds. The trial was promoted to the ten-second tier; its actual
remaining limit was 9.492 seconds. A fresh solve of that order took 4.974
solver seconds and independently verified all 15,389 cells.

The production wrapper preserves the original coordinate-constraint order
when deriving its base. That order differs from the research preparation.
An independent Symbolica check confirms identical ordered split polynomials,
strict constraint multisets, coordinates and ordered groups. Nevertheless,
local discovery is sensitive to input factor insertion order: the wrapper's
equivalent base produced 59,536 cells, 459,310 speculative nodes and 34,830
refinements, versus 15,389 cells, 127,187 nodes and 5,080 refinements for the
research preparation. A fresh release solve of the actual wrapper base took
18.924 seconds; its 59,536 cells independently verified. No smaller research
proof was silently substituted into the original certificate.

The original wrapper's first debug attempt hit its default local node cap
and remains explicitly incomplete. A second attempt used the research caps
of ten million nodes and one million refinements, with a 180-second solve
budget and 4 GiB. It completed in 137.470 solver seconds and exported 80,980
original cells. Independent verification checked all 34,251 base stacks,
34,321 rejected branches, seven guide certificates, the exact original
problem, every restored interval and the exceptional-set inventory.

The production source is `ff22294`; its frozen debug worker has SHA-256
`b11cec1d40d7fc45595b0f6ddbfe86017b3836f29d1862ad1f6e5c0f4e16ded5`.
The full library/integration suite plus the two critical-geometry helper
tests passed 467 tests across 32 targets. The public two-cell TOML example
also independently verified.

Artifact summary
`artifacts/research/gluza-positive-ladder-original-verification-01.json`
(SHA-256 `46bfb0df777f8ec67f2a245a09055f5956a4b8a963190d5640c4ecf4139a0cfc`)
records original/input/proof/verification hashes and the full search report.
These exploratory timings mix release base runs and a debug original run;
they do not establish a matched speed ratio or a completed ordering
ablation. The roughly 300-second discovery cost must remain included in an
end-to-end comparison. Fresh release measurements and the original-problem
completion ledger are recorded separately.

A cold public-CLI repeat of the same reduced search independently verified
15,389 cells in 310.276 seconds including final verification (305.227 seconds
for search, solve and export). Its 104 trials again contained one completed
candidate. Separate 100-second controls for the three inexpensive seed
orders all timed out: the declared order, Brown's reversed order and the
sparsity order `[x5,x1,x7,x6,x2]`. Their final observed refinement counts were
116,546, 97,286 and 91,150. All used the same immutable release and 4 GiB;
the seed controls ran concurrently on three individually pinned CPUs, while
the search used one CPU. This is exploratory evidence for searching beyond
the seed orders, not a matched three-repeat runtime ablation or a finite
speedup ratio. Summary
`artifacts/research/gluza-positive-order-controls-01.json` has SHA-256
`34ce9b5129f86b96a910fbdd09df72eac757b0810b84b440da5926bd2bb3331e`.

The successful order first appeared at trial 9 and was promoted only at
trial 98. Every local trial had an absent intermediate cost score: the CLI's
historical predictor reads completed global projection checkpoints, which
local refinement does not produce. An optional local-inventory promotion
policy is being tested separately; this observation does not establish that
any proposed predictor improves completion or total runtime.

## Coordinate threshold chambers

A separate research preparation splits the five-coordinate base at `x6=1`
and `x7=1`. Put `p=x5`, `q=x6`, `r=x7`, `u=x1`, `v=x2` and
`T=p+q+r-1`. The exact coefficient identities are

\[
a=1-(1+p)(q+r)-T(u+v),
\]
\[
b=(1-q)((1+v)(p+r)+pr)
  +u(1-(1+r)(p+q)-vT),
\]
\[
c=(1-r)(q+p(1+q))+v(1-(1+q)(p+r))
  +u((1-r)(p+q)-vT).
\]

Consequently `q>1` forces `a,b<0`, and `r>1` forces `a,c<0`.
The helper checks the identities, obtains exact domain certificates for each
known negative sign, and independently replays those certificates against
the chamber's strict coordinate bounds. The remaining arrangements are:

| Chamber | Known negative coefficients | Remaining split polynomials |
| --- | --- | --- |
| `x6>1, x7>1` | `a,b,c` | none |
| `x6>1, 0<x7<1` | `a,b` | `c` |
| `0<x6<1, x7>1` | `a,c` | `b` |
| `0<x6<1, 0<x7<1` | none | `a,b,c` |

The four open chambers cover the base away from the two explicitly excluded
threshold hyperplanes. They are prepared in the common order
`[x6,x7,x5,x2,x1]`. Separately completed chamber CADs may still introduce
different lower-dimensional cuts, so their union is not automatically a
globally cylindrical decomposition. A common ancestor refinement and the
original-fiber replay remain necessary before claiming an original
completion. This preparation adds no new production pruning rule.
