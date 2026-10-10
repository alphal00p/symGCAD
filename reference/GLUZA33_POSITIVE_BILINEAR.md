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
