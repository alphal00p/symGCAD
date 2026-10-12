# Projective sections: an opt-in contract extension for `bilinear_general`

Date: 2026-10-12. Code: commit `9baf91c` (branch `claude/projective-sections`),
`src/bilinear_general.rs` (module documentation and `check_projective`),
`src/output.rs` (`RootIndexDomain::PositiveOrInfinity`, `PROJECTIVE_INFINITY`).
Evidence: `artifacts/research/claude-2026-10-12/projective-sections/`
(`REPORT.txt`, `measurements.json`, run directories; machine-local, not in Git).

**Status in one line.** With `solver.projective_sections = true` (default
false, user-approved), positive Gluza33 completes and verifies with 2,372 cells
in 0.42 s. Under the default strict contract positive Gluza33 is still
unresolved, and the audited corpus count stays **303/331**.

## What changes in the contract

The default contract (HANDOFF, "Central correctness contract") has three
parts: `A ⊆ S`, `S \ A ⊆ Z(E)` for an explicit exceptional set `E`, and
disjoint open cylindrical cells whose sections are continuous real-valued
functions on the base cell. The extension changes only the last part:

* An **upper** section may be the `index`-th distinct strictly positive root of
  its polynomial on the fiber where that root exists, and `+infinity` where it
  does not. `RootIndexDomain::PositiveOrInfinity` (`"positive_or_infinity"` in
  JSON) has exactly this meaning. It is never used as a lower bound.
* Cells stay connected open cylindrical cells in the compactified coordinate
  `u = x/(1+x)`, where every upper section is continuous with values in
  `(0,1]`. In the original coordinates such a section can switch between finite
  and `+infinity` inside one base cell. That is the only relaxation.
* Containment, disjointness, the exceptional set, and verification of the base
  pieces are unchanged.
* If the root is missing at the cell sample, the sample-specific isolation
  strings are `"+inf"`. That text deliberately does not parse as a rational, so
  a consumer that does not know the domain fails instead of reading a wrong
  number.

The extension answers the obstruction recorded in
[GLUZA33_POSITIVE_BILINEAR.md](GLUZA33_POSITIVE_BILINEAR.md) and in the
HANDOFF: a compactified positive fiber whose upper endpoint "switches between
finite and infinity within a parent region".

## The fiber theorem (proved)

Notation is the code's. `x` is the last lifted axis and `y` the one before it.
Write `F = a·x·y + b·x + c·y + d = A(y)·x + B(y)` with `A = a·y + b` and
`B = c·y + d`. All coordinates are positive.

**Premises.** Every premise is replayed by the verifier:

* There are three ratio orders, `a ≼ c`, `b ≼ d` and `c ≼ d`. Each one,
  `lower ≼ upper`, is the identity `upper·u_lower − lower·u_upper = m·s²` with
  `m > 0`. The guide coefficients `u_*` must be certified strictly positive on
  the coordinate box. Pointwise, `lower/u_lower ≤ upper/u_upper`, so
  `lower > 0 ⇒ upper > 0` and `upper < 0 ⇒ lower < 0` at every point,
  including points where an atom vanishes.
* There is a square separator: `Δ = b·c − a·d = m·G²` with `m > 0`, so
  `Δ ≥ 0` everywhere.

**Statement.** At every base point the following hold.

1. **`d < 0`.** Then `b < 0` and `c < 0`, so `a < 0`, so `F < 0` on the whole
   open quadrant. The fiber is empty; this is the *empty claim* `{d<0}`.
2. **`d > 0`.**
   * Let `Y*` be `−d/c` if `c < 0` and `+∞` otherwise. Then `B > 0` exactly on
     `0 < y < Y*`.
   * Let `X(y)` be `−B/A` if `A < 0` and `+∞` otherwise.
   * The positive fiber is then **exactly** `{0 < y < Y*, 0 < x < X(y)}`.
   * For `y > Y*`, which can only happen when `c < 0`: `c < 0 ⇒ a < 0` and
     `Y* − (−b/a) = Δ/(a·c) ≥ 0`. Hence `A < 0` and `B < 0`, so `F < 0`. On
     the line `y = Y*`, `F = A(Y*)·x ≤ 0`.
3. **`d = 0`.** This is a lower-dimensional seam, and `d` is exceptional.

**Continuity.** In compactified form the sections are

* `v* = d/(d + max(−c, 0))`, and
* `u* = B/(B + max(−A, 0))` with `B > 0`.

Both are continuous and positive on `{d>0}`.

**Consequence.** `S` decomposes into one cell per base cell of a decomposition
of `{d>0}`, plus the empty claim `{d<0}`. Here `{d>0}` means the retained base
constraints plus the strict condition `d > 0`, with no split. Each cell has
`y ∈ (0, PositiveOrInfinity root 0 of c·y+d)` and
`x ∈ (0, PositiveOrInfinity root 0 of F)`.

The `F = 0` sheet in the open quadrant is exactly the boundary `x = X(y)`, or
the line `y = Y*` where `A(Y*) = 0`. Interior `F=0` sheets therefore stay cell
boundaries. The `G = 0` seam (H for Gluza33), which the strict contract needs as
a cut, is no longer a boundary in this plan.

**How it was checked.**

* The proof is in the module documentation of `src/bilinear_general.rs`.
* The unit test `projective_fiber_is_exact_under_the_corner_chain` enumerates
  every integer tuple `(a,b,c,d)` in `[−3,3]⁴` that satisfies the premises:
  more than 100 tuples and more than 50,000 points. Cell membership equals
  `F > 0` at every point.

## Implementation

### Option and validation

* The option is `solver.projective_sections`, a bool that defaults to `false`.
* It is accepted only with `engine = "bilinear_general"`.
* Validation refuses it together with `merge = "aggressive"`.
* With the option off, the strict engine runs unchanged. On Gluza33 planning,
  compared with the integration binary, the piece TOMLs are byte-identical and
  `plan.json` differs only in planning timings.

### Planner

* The projective plan is used iff the option is on and planning discovered:
  * the `Square` separator;
  * the ratio orders `(a,c)`, `(b,d)` and `(c,d)`;
  * a non-constant `d`.
* The projective plan is one piece, `{d>0}` with no split, and the empty claim
  `{d<0}`. It is serialized as `"projective": true`, which is omitted when
  false.
* If any premise is missing, the planner silently uses the strict plan.

### Restoration

* Upper bounds: `y` uses `PositiveOrInfinity` root 0 of `c·y+d`, and `x` uses
  `PositiveOrInfinity` root 0 of `F`.
* Samples are `Y*/2` or `1`, and `X/2` or `1`.
* The exceptional set is the piece's set plus `d`, the coordinates, `B` and
  `F`.

### Verifier rules (`check_projective` plus the generic replay)

A projective result is accepted only if all of the following hold:

1. the problem enables the option;
2. the separator is `Δ = m·G²` with `m > 0`, replayed exactly;
3. the three ratio orders replay exactly, with their corner certificates;
4. no atom is identically zero;
5. `d` is not constant;
6. the plan is exactly the piece `{d>0}` plus the empty claim `{d<0}`, and
   `{d<0}` closes to all-negative coefficients;
7. the piece verifies with the ordinary verifier;
8. every cell is rebuilt and compared.

Beyond `check_projective`:

* `verify::verify`, `verify_nonlocal` and the verifiers of the base pieces
  reject any `positive_or_infinity` bound.
* A projective bound on a strict `bilinear_general` result, or on a baseline
  result, is rejected.

### Other code paths

* The compatibility merge never merges a cell that carries a projective bound.
* `comparison::compare_generic` and `examples/sanity_sample.rs` treat a missing
  root as `+∞`.
* Homogeneous rescaling handles the new domain explicitly.

### Tests

* **`tests/bilinear_projective.rs`, 7 tests:**
  * a Gluza-like synthetic chain solved with the `optimized`, `local` and
    `baseline` base engines. Exact membership on an 8⁴ rational grid plus 400
    random rational points agrees with the baseline engine, and
    `compare_generic` agrees too;
  * a one-parameter example with constant square `Δ`;
  * fallback to the strict plan without a guide, or when `Δ` is not a square;
  * option validation and TOML round trip;
  * plan and result serde;
  * a piece solved separately, then assembled and verified;
  * Gluza33 planning with the option on (1 piece) and off (4 pieces).
* **25 single-field forgeries**, each failing for its own reason, plus
  projective bounds smuggled into strict and baseline results.
* **Unit tests:** the brute-force fiber check, comparison with infinite
  sections, the merge refusal, and serde of the new domain.
* **`cargo test --example sanity_sample`:** 9 passed (2 new).
* **Fast suite on `9baf91c`:** 753 passed, 0 failed, 2 ignored, over 71
  targets.

## Measured results

All runs used one pinned CPU and release binary `symgcad-projective`, sha256
`4f212724…6b675`.

### Positive Gluza33

Input:

* constraints, variables and name of
  `artifacts/corpus/symbolic/0338_test_gluza_ex33_solve_3.toml` (sha256
  `63247029…20574`);
* `bilinear_general_guide` = U, the first Symanzik polynomial (`det` of the
  loop matrix);
* `bilinear_general_base_engine = "optimized"`;
* `bilinear_general_base_anchor = "x4"`;
* `positivity = true`.

The `x5 last` column uses order `[x4,x7,x2,x1,x0,x6,x3,x5]`; `x3 last` swaps
the last two axes.

| | x5 last | x3 last |
|---|---|---|
| Plan | 1 piece `{d>0}`, empty `{d<0}` | same |
| Solve | complete, 2,372 cells, 0.42 s wall, 27,648 KiB | complete, 2,372 cells, 0.39 s, 30,772 KiB |
| Internal phases | plan 0.014 s, piece 0.121 s, assembly 0.169 s | plan 0.011 s, piece 0.116 s, assembly 0.167 s |
| `symgcad verify` | verified: 2,559 stacks, 7,274 rejected branches, 0.55 s, 43,056 KiB | verified, same counts, 0.55 s |
| Cells with an infinite section at the sample | 954 (y), 919 (x) | 1,208 (y), 714 (x) |

**Supplemental sanity check** (`examples/sanity_sample.rs`):

* settings: seed `20261010`, two fresh points per cell, evaluated against the
  **original** TOML, outside solve timing;
* x5 last: passed, 4,744/4,744 points, 85,392 original evaluations, 42,696
  constraint checks, 0 failures, 568.5 s wall. 2,245 of the fresh points have
  an infinite y-section and 1,972 an infinite x-section;
* x3 last: passed, 4,744 points, 0 failures, 586.7 s.

This sampling supplements the verifier. It is not a coverage proof.

**Strict comparison.** With the option off the same input still plans 4 pieces
plus 1 empty claim. Those pieces have not been solved: the estimate is about
3·10⁶ cells (see [BILINEAR_GENERAL.md](BILINEAR_GENERAL.md)).

### Positive Gluza30

Input: the constraints of `artifacts/corpus/symbolic/0332_test_gluza_ex30_solve_3.toml`
(the constraint text is identical), the U guide, and the default base engine.
Each order is a single run, `complete_generic` and verified:

| Order | Cells | Wall |
|---|---:|---:|
| `[x1,x7,x2,x6,x5,x4,x0]` | **1** | 0.04 s |
| `[x1,x7,x2,x6,x4,x5,x0]` | 4 | 0.04 s |
| `[x0,x4,x2,x6,x1,x7,x5]` | 7 | 0.05 s |

* **Strict reference:** the published strict result is 80,980 cells, with
  22.57 s solve and 16.85 s check (release, N=3, cold;
  [GLUZA30_BILINEAR_RELEASE_BENCHMARK.md](GLUZA30_BILINEAR_RELEASE_BENCHMARK.md)).
  It used `bilinear_positive` and its own order, so this is not a matched-order
  comparison.
* **No sanity check:** no fresh-point sanity check was recorded for the
  Gluza30 projective runs.

## Scope and limitations

**Proved:**

* the fiber theorem above;
* the verifier's acceptance conditions imply it.

**Measured:**

* the Gluza33 and Gluza30 runs above, and the sanity check;
* nothing else.

**Not done or open:**

* **Engine scope.** Only `bilinear_general` positive targets with all of the
  premises are covered. The target must be a single strict `F > 0` whose last
  two axes enter bilinearly. It also needs a guide whose corner coefficients
  are certified positive, the three ratio orders, and a square separator. No
  other engine emits or accepts the new domain.
* **Sign-split problems.** All symbolic ZZ originals request both signs of `F`.
  These are not covered:
  * The `{d<0}` region is then not an empty claim, because it carries `F < 0`
    cells.
  * Those cells would need a section going to `+∞` as a *lower* bound or a
    `−∞`-type selector. Neither exists, and the verifier rejects both.
* **Minkowski kinematics.** The ratio-order premises are Rayleigh/Dodgson
  identities of two-point (Euclidean-type) graphs. Two facts suggest they fail
  on the ZZ inputs, but neither proves it for the bilinear corner premises:
  * The related one-axis guide premise fails on all 39 linear axes of the 27
    unresolved ZZ originals ([ZZ_RAYLEIGH_GUIDE_AUDIT.md](ZZ_RAYLEIGH_GUIDE_AUDIT.md)).
  * Joint convexity of `F/U` fails for indefinite kinematic Gram matrices
    ([KINEMATIC_PORTFOLIO.md](KINEMATIC_PORTFOLIO.md)).

  The extension was not tried on any ZZ case. **No parametric (symbolic ZZ)
  case closes under it.**
* **Downstream consumers.** Any tool that reads cell bounds must learn
  `positive_or_infinity` and `"+inf"`. Those already updated are the verifier,
  merge, comparison and sanity sampler. FastSecDec export and integration have
  not been touched.
* **Order and engine dependence.** 2,372 cells is the count for these orders
  and this base engine. It is not a minimum.
* **Corpus accounting.** Counting positive Gluza33 under the extension gives
  304/331, but that figure must always carry the qualifier "under the opt-in
  projective-sections contract". The strict count is 303/331.

### Relation to other contract proposals

The "generalized sectors" idea from the convex-theory track is a different
relaxation and is **not implemented**. It allows a sector to be empty on a
nowhere-dense subset of its base cell. The "selected roots" boundary is also
not implemented. Both are recorded in
[GLUZA33_STRUCTURE_2026_10_12.md](GLUZA33_STRUCTURE_2026_10_12.md).

A projective-style simplification for sign-split problems would need a typed
treatment of empty-fiber sectors. The HANDOFF's October 12 section lists it as
an open direction.

## Reproduce

```sh
nix develop -c cargo build --release --locked
target/release/symgcad solve g33_projective_x5last.toml --output g33.json
target/release/symgcad verify g33.json
cargo run --release --example sanity_sample -- \
  --original artifacts/corpus/symbolic/0338_test_gluza_ex33_solve_3.toml \
  --result g33.json --output g33-sanity.json \
  --seed 20261010 --points-per-cell 2 --seconds 1200 --memory-mib 4096
```

The input `g33_projective_x5last.toml` is in
`artifacts/research/claude-2026-10-12/projective-sections/inputs/`. Its
`[solver]` table is:

```toml
engine = "bilinear_general"
ordering = "given"
order = ["x4", "x7", "x2", "x1", "x0", "x6", "x3", "x5"]
merge = "none"
positivity = true
bilinear_general_guide = "<U, the first Symanzik polynomial of the Gluza33 graph>"
bilinear_general_base_engine = "optimized"
bilinear_general_base_anchor = "x4"
projective_sections = true
```
