# Positive Gluza33: strict-premise obstruction and a generic two-fiber proposal

The original input is
`artifacts/corpus/symbolic/0338_test_gluza_ex33_solve_3.toml`. This research
preserves its eight positive coordinates, polynomial, parameters and variable
group. It does not yet provide an original decomposition.

The bounded Symbolica helper `examples/bilinear_premises.rs` checks all
56 ordered pairs of distinct axes. Only the two orientations of `(x3,x5)`
are affine in both axes; the other 54 pairs fail that degree condition.
For either orientation, write

\[
F=a x y+b x+c y+d.
\]

The strict seven-premise `bilinear_positive` rule that succeeds for Gluza30
does **not** apply here. Exact evaluation gives `d=-48` when the six
remaining coordinates are all one. Moreover, the derivative guide comparison
`W4` has a zero at a strictly positive rational point. For final axis `x3`,
the point is all coordinates one except `x6=x7=2`; for final axis `x5`, it is
all coordinates one except `x0=x4=2`. The helper independently checks every
original coordinate inequality at these points. Membership in `F>0` is not
a premise of the proposed guide proof, so it is not required for these
counterexamples. Failure to certify `W0` is separately labeled inconclusive.

The exact screen and its certificates are retained at
`artifacts/research/bilinear-premises-01/gluza33/report.json`; it took
0.066 seconds in a bounded debug helper. Thus neither a failed heuristic
nor a sampled positive value is being substituted for a domain proof.

## Generic bilinear reconstruction

A guide is unnecessary if a complete base CAD already fixes the nonzero
signs of `a,b,c,d` and `Delta=b*c-a*d`. On each such connected base cell,
split `y>0` at whichever of

\[
r_A=-b/a,\qquad r_B=-d/c
\]

are positive. Their positivity is fixed by the coefficient signs. When both
are positive, their order is fixed exactly by

\[
r_B-r_A=\frac{\Delta}{a c}.
\]

The nonzero `Delta` premise excludes coincident boundaries. On each resulting
open `y` interval, let `A=a*y+b` and `B=c*y+d`. Their signs are constant and
the complete positive fiber table is:

| Signs `(A,B)` | Admissible `x>0` |
| --- | --- |
| `(+,+)` | `(0,infinity)` |
| `(+,-)` | `(-B/A,infinity)` |
| `(-,+)` | `(0,-B/A)` |
| `(-,-)` | empty |

There are at most three `y` intervals and one nonempty `x` interval per row.
Every finite boundary is an ordinary simple linear root, with full-real
root index zero. The coefficients, `A`, `B`, and `Delta` seams must be
explicitly retained. An identically zero coefficient or `Delta` needs a
separate degenerate rule; it cannot be treated as an excluded nonzero seam.
This is an exhaustive original-coordinate cylindrical construction, without
variable finite/infinite bounds or a projective interpretation of a cell.
It remains a proposal until a typed production replay and complete base
proof are available.

For Gluza33, Symbolica exactly factors the 190-term polynomial `Delta` as
`H^2`, where `H` has 26 terms and degree three. Reconstruction of the full
factorization is checked exactly. The research base uses the five split
polynomials `a,b,c,d,H`; it **does not remove** the `H=0` seam merely because
`Delta` is a square. Such a seam can collapse a used fiber interval and must
be excluded from every relevant open parent cell by the base decomposition.

The six base coordinates are `[x0,x1,x2,x4,x6,x7]`. The split degrees are
`[2,3,3,4,3]`, each homogeneous in those coordinates. The helper therefore
also prepares an explicitly anchored `x7` variant using the already verified
homogeneous solver. Its result, if complete, would still be a decomposition
in all six original base coordinates. The helper declines the anchored
variant whenever any split fails the exact homogeneity check.

Preparation took 0.124 seconds, including 0.018/0.012 seconds for the two
exact `Delta` factorizations. Full expressions, factor multiplicities,
strict-premise counterexamples, and proposed TOML files are retained under
`artifacts/research/bilinear-premises-02/gluza33/`. The immutable helper hash
is `1bde58b88fa4372699630be34f2ac1e81480c82ee55d4c870dcccdcdeb77be10`.
Its four tests cover strict versus zero premises, missing domain bounds,
exact base-permutation comparison, and retaining a repeated root-order seam
while rejecting an invalid homogeneous proposal.

## Ordering experiment

Two cold ordering searches are allocated 150 seconds each, with a common
300-second search ceiling, seed zero, at most 256 rollouts, CPU 8 and an
8-GiB worker limit. The first uses all six base axes; the second uses the
checked `x7` homogeneous reduction. The release worker is
`symgcad-indexed-merge-dda5980`, SHA-256
`f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`.
Inputs, durable journals and outcomes are retained under
`artifacts/research/gluza33-generic-order-01/`. An ordering trial's completion
alone is not an independently verified base proof or an original completion.

Both searches finished within their declared bounds, with **no completed
trial**. The six-axis search recorded 49 timeouts in 150.029 seconds; the
`x7`-anchor search recorded 53 timeouts in 150.034 seconds. The total measured
order-command time was 300.063 seconds, excluding the separately reported
preparation. Inputs and a compact final summary are retained in that directory.

The largest retained six-axis progress snapshot had only 770 speculative
nodes and 13 refinements. It had already spent 13.59 seconds on discriminants
versus 0.65 seconds on factoring, at 15.63 seconds elapsed, before a promoted
60-second attempt timed out. The anchored snapshots likewise showed small
trees, with none newer than 0.87 seconds in the final records. These are stale
snapshots before expensive operations, not measurements of the final active
operation. They do not support blaming initial factorization merely because
an earlier snapshot retained that phase name. This experiment establishes
no completion and no general impossibility; it gives no reason yet to add a
new production wrapper around this particular full base arrangement.

## Exact positive endpoint projection

A stronger endpoint-guide calculation eliminates the entire `d<=0` region.
For a positive coordinate `z`, write `q=A*z+B` and `g=C*z+D`, with
`C,D>0` and `B*C-A*D>=0` over the other positive coordinates. If `B<=0`,
then `A<=B*C/D<=0`, and consequently `q<=0` for every `z>0`. Conversely,
`B>0` gives `q>0` for sufficiently small positive `z`. Therefore

\[
\exists z>0:q>0\quad\Longleftrightarrow\quad B>0.
\]

Apply this rule first to `F,U` in `x3`, then to their **endpoints**
`F|x3=0,U|x3=0` in `x5`. These are different from the leading-derivative
guides in the negative-domain chain. Exact Symbolica factor reconstruction
proves both comparisons are `10` times a square, and all `C,D` coefficients
are strictly positive. The second square has a six-term, strictly positive
coefficient polynomial; the first square can vanish. The reverse endpoint
order passes the same exact checks. Thus, on all six remaining positive
coordinates,

\[
\exists x3,x5>0:F>0\quad\Longleftrightarrow\quad d>0.
\]

This supersedes the earlier, weaker conditional proposal to solve a
`d<0,c>0` branch: the second endpoint comparison proves that branch empty.
No original positive point is lost by adding `d>0` to the reduced base.
The new research inputs use four splits `a,b,c,H` and the strict constraint
`d>0`. Both the constraints and splits are checked for exact homogeneity
before an anchored variant is emitted.

Neither `c` nor `d` has a global sign on the original positive orthant.
For final coordinate `x3`, all coordinates one give `c=-66,d=-48`;
changing only `x0` to `32` gives `c=1174`, while changing it to `15` gives
`d=400`. The reverse orientation also has checked witnesses of both signs.
These are exact rational counterexamples, not failed positivity searches.

The `H=0` seam is still essential. At `x0=15` and all other coordinates
one, for last coordinate `x3` the exact coefficients are

\[
(a,b,c,d)=(-110,200,-220,400),\qquad
r_A=r_B=20/11,\qquad F=270>0.
\]

Hence a middle fiber can pinch inside `d>0`, even above an admissible
original positive point. Reversing the massless order has the same
counterexample with `x6=15` instead. The middle interval cannot be exported
as a nonempty cylindrical cell over a parent containing that seam.

Evidence is retained at
`artifacts/research/bilinear-premises-03/gluza33/report.json` (0.173 seconds
for exact preparation). The immutable helper SHA-256 is
`b4e5463df522fa351b117a6090b5a72b52cf219068cc7a1cd174158c01d835e6`.
Its six tests include the original endpoint identities, the admissible
pinch, wrong guide orientation, exact source comparison and degenerate
root-order safeguards. This is a proved reduction and a research input,
not yet an original decomposition or a new production rule.

Two cold local release trials of that constrained base also timed out:
60.024 seconds for order `[x6,x1,x4,x0,x2,x7]`, and 60.030 seconds for its
`x7`-anchor counterpart. Both used CPU 8, 4 GiB and the same immutable
`dda5980` release as above. The first final active operation was a degree-12
discriminant in `x0` of a 1,205-term polynomial; the second was a degree-17
discriminant in `x4` of a 1,163-term polynomial. These are exact captured
operands, independently parsed with Symbolica. Last progress snapshots had
only 438/1,709 speculative nodes and 9/41 refinements, respectively. Thus
these orders still suffer projection growth despite the smaller admissible
base; increasing traversal caches would not address this measured obstacle.
Inputs, operation captures, progress and process limits remain under
`artifacts/research/gluza33-positive-endpoint-base-01/summary.json`.

## Checked endpoint discriminant ladder

`examples/gluza33_endpoint_structure.rs` independently reconstructs the
endpoint from the exact original input. Set

\[
\begin{aligned}
A&=x0+x6,&D&=x4+x7,&S&=A+D,&B&=x1+x2,\\
K&=x6\,x4-x0\,x7,\\
P&=ADS-10\bigl(x0\,x6\,D+x4\,x7\,A\bigr),\\
R&=(x0+x4)^2-8(x0+x4)(x6+x7)+(x6+x7)^2.
\end{aligned}
\]

Exact Symbolica identities give

\[
SP-10K^2=ADR,\qquad
d=AD B^2+PB+x1\,x2(SB+R).
\]

For `alpha=x1`, this is the quadratic `d=L*x2^2+M*x2+N`, where

\[
L=AD+S\alpha>0,\quad
M=2AD\alpha+P+S\alpha^2+R\alpha,\quad
N=\alpha(AD\alpha+P).
\]

The endpoint topology and both discriminants satisfy the checked identities

\[
\begin{aligned}
AD M-\alpha\bigl((AD)^2-10K^2\bigr)
  &=(AD+S\alpha)(P+AD\alpha),\\
\operatorname{Disc}_{x2}(d)
  &=(S\alpha^2+R\alpha+P)^2-40K^2\alpha^2,\\
\operatorname{Disc}_{\alpha}\bigl(\operatorname{Disc}_{x2}(d)\bigr)
  &=25600K^4S^2P^2R^2\bigl((R-4AD)^2-160K^2\bigr).
\end{aligned}
\]

The first relation avoids substituting a rational expression at `N=0`:
since `AD>0`, it directly fixes the sign of `M` there from
`(AD)^2-10K^2`. The final quartic factor has only 35 terms in the four
coordinates `[x0,x4,x6,x7]`. The original endpoint has 50 terms, its first
discriminant 155, and its expanded second discriminant 1,755. Native
factorization checks the first discriminant is irreducible over the rational
coefficient field in this calculation. The whole exact helper run took
0.081 seconds, including 0.011 seconds for that factorization.

These identities suggest placing `x1,x2` last in the base order and using
an outer homogeneous anchor such as `x7`. They do not yet establish all
pairwise boundary orderings for the required `a,b,c,H` cuts. The prepared
structural trial retains those cuts, the original positive coordinates and
the exact `d>0` constraint. It is a candidate ordering, not a completed CAD.

The helper also checks the substitution `x1=rho*B`, `x2=(1-rho)*B` and
its quadratic discriminant exactly, but no claim of cylindrical transport
for that chart is made. Original-coordinate identities above suffice for
the present ordering proposal. Evidence is under
`artifacts/research/gluza33-endpoint-structure-02/x2/`; immutable helper
SHA-256 is
`644fb1bf481dcd3096f23d98419ff8a57d5829b63cb5c80a9a70865d1c17baa6`.
Two tests check all identities against the original endpoint and reject
changed target/domain assumptions.
