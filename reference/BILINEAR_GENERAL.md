# `bilinear_general`: exact two-axis positive fibers over verified base pieces

Date: 2026-10-12.

**Code** (`src/bilinear_general.rs`, `src/identity.rs`):

* `6cabcf5` adds the engine and the pieces CLI.
* `eb1cd8a` addresses the review.
* `0d4e38c` changes integration dispatch.
* `9baf91c` adds the projective plan; see [PROJECTIVE_SECTIONS.md](PROJECTIVE_SECTIONS.md).

**Tests:** `tests/bilinear_general.rs` and `tests/bilinear_projective.rs`.

**Evidence** (machine-local):

* `artifacts/research/claude-2026-10-12/bilinear-general/` for the engine;
* `shadow-prototype/` for cell estimates;
* `order-screen/` for the order ranking;
* `runs/` and `critique/` for piece runs.

**Status.**

* **Strict mode:** the engine plans positive Gluza33 into 4 pieces plus 1 empty
  claim in 0.034 s. None of the 4 strict pieces has been solved. Estimates put
  the strict decomposition at about 3·10⁶ cells.
* **Projective mode:** with the opt-in projective plan the same input completes
  with 2,372 verified cells.

## What the engine does (proved, replayed by the verifier)

**Target.** The engine handles `F = a·x·y + b·x + c·y + d > 0`. The final axes
are `y` then `x`, and both are strictly positive. All 16 strict sign patterns of
`(a,b,c,d)` are handled.

**Zero coefficients.**

* An identically zero `b`, `c` or `d` takes its partner's sign in the open
  quadrant, and carries no premise, condition or split.
* An identically zero `a` or `Δ = b·c − a·d` is declined.

**Fiber table.**

* `fiber_rows(signs, delta)` gives the positive `(y,x)`-fiber for each strict
  sign vector:
  * The breakpoints are `−b/a` (when `a·b < 0`) and `−d/c` (when `c·d < 0`).
  * When both are positive, they are ordered by `sign(Δ)·sign(a·c)`.
  * On each `y`-interval, the signs of `(A, B) = (a·y+b, c·y+d)` give the
    `x`-fiber:

    | `(A, B)` | `x`-fiber |
    |---|---|
    | `(+,+)` | `(0,∞)` |
    | `(−,+)` | `(0,−B/A)` |
    | `(+,−)` | `(−B/A,∞)` |
    | `(−,−)` | empty |
* The table is brute-force checked against exact membership in a unit test of
  more than 100k points.

**Pieces.**

* The base is split into ordinary symGCAD problems ("pieces"). Each consists of
  the retained base constraints, then strict *atom conditions* in canonical
  order, then the leaf atoms as `split`.
* The atoms are `a, b, c, d, G`. `G` is the root-order separator.
* Pieces are independent, so they can run as separate processes.

**Premises.** Each premise is a typed identity that the verifier replays
exactly:

* **`RatioOrder{lower, upper}`.**
  * Definition: `upper·u_lower − lower·u_upper = m·s²` with `m > 0`. Here `u_*`
    are the bilinear coefficients of a guide `U = u_a·x·y + u_b·x + u_c·y + u_d`.
  * Each `u_*` is certified strictly positive on the coordinate box.
  * Pointwise this gives `lower > 0 ⇒ upper > 0` and `upper < 0 ⇒ lower < 0`.
  * An optional `strict` certificate proves `s ≠ 0`. It is recorded but not
    used, because the sign logic works on strict sign vectors.
* **`GlobalSign{atom, certificate}`:** a whole-box strict sign certificate.
* **Separator `G`:** one of
  * `Δ = m·G²`,
  * `Δ = −m·G²`, or
  * `G = Δ` itself.

**Verifier.** It never trusts the planner. It:

1. replays every identity and certificate;
2. checks with a truth table over all premise-consistent strict sign vectors of
   `(a,b,c,d,G)` that exactly one piece or empty claim matches each vector;
3. checks that every leaf decides the fiber table, and that every empty claim
   closes to a contradiction or to all-negative coefficients;
4. re-derives each piece problem (serde equality; wall time may only shrink),
   rejects nested composite evidence, and verifies each piece with
   `verify::verify`;
5. rebuilds and compares every cell.

The atom zero sets, the coordinates, `a·y+b`, `c·y+d` and `F` belong to the
exceptional set.

## Ratio premises from the Symanzik guide

The guide `U` is the first Symanzik polynomial. For a graph with massless edges
`x, y` and fixed `s`, the Rayleigh/Dodgson identity is
`F_z·U − F·U_z = −m·H_z²`. Taking bilinear corners of it gives exactly the
`RatioOrder` identities: write `X ≼ Y` for `X/u_X ≤ Y/u_Y`.

**On Gluza33** (planning output; `bilinear_general/measurements.json`):

* `u_a..u_d` have 6/9/9/12 terms, all with coefficient +1, and
  `u_a = x0+x1+x2+x4+x6+x7`.
* The premises are `a ≼ b`, `a ≼ c`, `b ≼ d` and `c ≼ d`, all with multiplier
  10. Their squares have 5, 5, 6 and 6 terms.
  * These are the Dodgson chain identities `hab`, `hac`, `hbd` and `hcd` of
    [GLUZA33_STRUCTURE_2026_10_12.md](GLUZA33_STRUCTURE_2026_10_12.md), with
    `b` and `c` swapped there; see the naming note below.
  * `b ≼ d` and `c ≼ d` carry strict certificates, because their squares have
    only positive coefficients.
* The separator is `Δ = 1·H²`, where H has 26 terms and degree 3.

**Consequences.**

* `a > 0 ⇒ b, c > 0 ⇒ d > 0`, so `F > 0` on the whole quadrant.
* `d < 0 ⇒ b, c < 0 ⇒ a < 0`, so the fiber is empty.
* The sign pattern `{a>0, d<0}` cannot occur.

**Naming.** The code uses `x` = last axis, so its `b` and `c` are swapped
relative to `artifacts/research/claude-2026-10-12/shared/CONTEXT.md` and the
structure note. The region files `Ppp`, `Ppm`, `Pmp` and `Pmm` use the
CONTEXT naming.

## Planner and objective

`plan_pieces` enumerates every branching order over the free atoms:

* When both children of a branch are non-empty leaves, the branch becomes a
  split.
* An empty child becomes an empty claim.
* Ties are broken deterministically.

`bilinear_general_plan_objective` picks among the valid plans:

| Objective | Rule | Gluza33 plan | Cost (weighted / plain) |
|---|---|---|---|
| `region_weighted` (default) | atom polynomials per piece, weighted by the number of premise-consistent sign regions the piece covers | 4 pieces | 28 / 11 |
| `atoms` | plain atom count | 3 pieces: `{b<0,c>0}` and `{b<0,c<0,d>0}` merge into `{b<0,d>0}` split `c` | 30 / 9 |

**Why `region_weighted` is the default** (measured, not proved):

* With full projection, the merged piece projects `d` over a region where
  another sign already fixes it.
* On a synthetic quartic chain with 3 parameters and the local base engine,
  `region_weighted` was 4–14% faster (6.0–6.2 s, 8,142 cells against 6.3–7.2 s,
  12,015 cells). The optimized engine was equal within noise.
* The comparison has not been run on Gluza33.
* The verifier accepts either plan.

**Strict Gluza33 plan.** Order `[x4,x7,x2,x1,x0,x6,x3,x5]`, base anchor `x4`.
Code naming; region-file names as recorded by the implementer's comparison
script:

| Piece | Conditions | Split | Region file |
|---|---|---|---|
| `piece_000` | `b>0, c>0` | `a` | `Ppp` (R++) |
| `piece_001` | `b>0, c<0` | `G` (= H) | `Pmp` (R−+) |
| `piece_002` | `b<0, c>0` | none | `Ppm` (R+−) |
| `piece_003` | `b<0, c<0, d>0` | none | `Pmm` (R−−) |
| empty claim | `b<0, c<0, d<0` | | |

The generated piece TOMLs match the hand-written `*_O1` region inputs as
polynomial sets up to positive scaling: `all_equal = true` in
`compare_region_inputs.py`.

## Pieces CLI

```sh
symgcad pieces plan input.toml --out-dir pieces          # piece_NNN.toml + plan.json
symgcad solve pieces/piece_000.toml -o r0.json             # one process per piece
symgcad pieces assemble input.toml --plan pieces/plan.json --results r*.json --output result.json
symgcad verify result.json
```

* `pieces plan` and `pieces assemble` run in workers bounded by the input's
  limits.
* `plan` removes stale `piece_NNN.toml` files.
* `assemble` refuses duplicate, missing or foreign results.
* `statistics.bilinear_general_pieces` keeps planning, per-piece and assembly
  timings apart.

The options are documented in [docs/USAGE.md](../docs/USAGE.md):

* `bilinear_general_guide`;
* `_base_engine`: `optimized` (default), `baseline` or `local`;
* `_base_anchor` and `_base_block`, which set an inner homogeneous anchor per
  piece;
* `_plan_objective`;
* `projective_sections`.

## Measured attempts on the strict Gluza33 pieces (all incomplete)

### Local engine

| Source | Configuration | Outcome |
|---|---|---|
| `bilinear-general/` | region plan pieces 002 and 003, plus the atoms-plan piece 002; 1,200 s, 4 GiB | all timed out, about 270 MiB peak |
| `runs/pc-*`, `critique/pc_final_progress.txt` | plain or Descartes-witness options, 31 min | Ppm: 15.2M nodes, level 3 (plain). Ppp: 6.7M nodes, level 3. Pmm: frozen after 774 s, 728 s of it in discriminants. Pmp: frozen after 30 s at level 4 |
| `local-thrash/` | plain + stable samples ± sibling retention, 30 min | none complete; see [LOCAL_ENGINE_THRASH.md](LOCAL_ENGINE_THRASH.md) |

### Optimized engine

Order O1, from `runs/op-*` and `critique/measurements.json`. Each cell gives
polys/terms/max degree after eliminating that axis:

| Piece | `x6` | `x0` | `x1` | `x2` |
|---|---|---|---|---|
| Ppm | 13/269/5 | 26/1,079/10 | 175/10,131/24 | 22,853/350,623/156 |
| Ppp | 16/296/5 | 30/1,706/10 | 268/26,549/52 | not finished |
| Pmm | 18/451/5 | 59/4,355/11 | 1,033/228,998/112 | not finished |
| Pmp | 20/386/5 | 61/4,444/12 | 1,035/279,526/114 | not finished |

* **Ppm:**
  * The solve then failed with `memory allocation … failed` at the 16 GiB
    limit after 1,228 s.
  * With the session's kernel changes, `symgcad project` completes the same
    projection in 354.5 s ([ALGEBRA_KERNELS_2026_10_12.md](ALGEBRA_KERNELS_2026_10_12.md)).
  * Lifting over a 22,853-polynomial bottom level was not attempted.
* **Pmm:** strict-Bezout pair omission reduces level 3 to 661/112,759/112.

## Cell-count estimates (exploration only, not proofs)

### Shadow prototype

Source: `shadow-prototype/`, `wf3-shadow.txt`. A selected-root
("relevance-restricted") projection for order O1, with relevance screened on
600–1,500 random samples, an exact bottom stack, and a Knuth estimator with
1,500 paths:

| Piece | Base cells (95% CI) | 8-d cells |
|---|---|---|
| R++ (a-split) | 1.76·10⁶ [1.71, 1.80]·10⁶ | 2.78·10⁶ |
| R+− | 3.04·10⁴ | 2.29·10⁴ |
| R−+ (H, x5 last) | 2.15·10⁴ | 3.98·10⁴ |
| R−− | 1.20·10⁵ | 1.50·10⁵ |

* **Strict totals (x5 last):** 1.93·10⁶ base and **2.99·10⁶ 8-d cells**. With
  x3 last: 2.05·10⁶ and 3.21·10⁶.
* **"Generalized sectors"** (not implemented): 1.66·10⁶ and 2.39·10⁶.
* **R++ dominates.**
* **Full (non-relevance) CAD:** for R+− in the same order the estimate is
  1.05·10¹⁰ base cells.
* **Bottom sections:** 93–98% are crossings of incomparable members, which
  convexity does not certify.
* **Caveat:** an independent x7 scan suggests the bottom stacks are 30–50% too
  low.
* The track's verdict was **NO-GO for a strict structured solve**.

### Order screen

Source: `order-screen/ranking_proxy.txt`, `ranking_precise.txt`.

* **Coverage:**
  * The proxy screen covered most canonical leaf orders per piece: 83/90
    (R++), 72/180, 98/180 and 45/90.
  * The precise screen covered 3, 4, 3 and 2 orders.
* **Best common order:** O1 `[x4,x7,x2,x1,x0,x6]` (x5 last). The two screens
  disagree by about a factor of 2.7:

  | Screen | Base cells | 8-d cells |
  |---|---|---|
  | proxy | 7.09·10⁵ | 1.0·10⁶ |
  | precise | 1.93·10⁶ | 2.89·10⁶ |
* **Best per-piece orders:** R+− `[x4,x6,x1,x7,x2,x0]`, precise 6.3·10³ base
  cells; R−+ (with the H split) `[x4,x7,x1,x2,x0,x6]`, 1.3·10⁴. In the
  precise pass R++ (1.73·10⁶, 91% of the total) and R−− (1.50·10⁵) are best at
  O1 itself; no screened order brings the strict total below about 2.9·10⁶
  8-d cells.
* **Without convex relevance:** the same estimator in an ordinary
  local-projection mode (every adjacent-pair resultant root is a boundary)
  gives 1.4·10⁸ base cells for R+− at O1 (exact) and roughly 10⁹–10¹⁰ for
  R++ and R−− (extrapolated from sampled pairs), i.e. about 10¹⁰ in total.
  This is why the existing local engine cannot reach a strict solve.
* **Per-piece best precise sum:** 1.9·10⁶ base cells. Choosing a different
  order per piece does not change the order of magnitude: R++ is at least
  1.7·10⁶ in all three orders screened precisely, and its proxy best is
  6.1·10⁵.
* **Avoid:** cross-chain elimination pairs and orders that eliminate `x1` and
  `x2` first ([GLUZA33_STRUCTURE_2026_10_12.md](GLUZA33_STRUCTURE_2026_10_12.md)).

## Open points

* **Local-engine assumption.** As with `bilinear_positive`, the engine assumes
  every base engine returns cells on which the split signs are constant. Split
  factors are protected boundaries in all base engines, and each piece's own
  verifier checks this.
* **Product cap.** A fixed cap of 4M terms on the `b·c` and `a·d` products
  declines larger inputs before planning.
* **Duplication.** Polynomials are duplicated across piece tables.
* **Timings.** Planning and piece timings are reports; the verifier does not
  check them.
* **Strict Gluza33.** A strict solve would need a relevance-aware base engine,
  since about 3·10⁶ cells exceeds every budget tried. The opt-in projective
  plan avoids the strict pieces entirely.
