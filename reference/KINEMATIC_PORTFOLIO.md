# Kinematic portfolio ordering and the parametric (symbolic ZZ) analysis

Date: 2026-10-12.

**Code** (`src/kinematic_portfolio.rs`, plus the CLI resolution and the
`_worker kinematic_probe` and `_worker kinematic_candidates` workers in
`src/main.rs`):

| Commit | Change |
|---|---|
| `d04747a` | the ordering mode |
| `b696002` | fully supervised in workers |
| `74440c8` | documentation |

**Tests:** `tests/kinematic_portfolio.rs`, 16 tests.

**Evidence** (machine-local), under `artifacts/research/claude-2026-10-12/`:

* `param-profile/` for the profiling and Landau analysis;
* `kinematic-order/` and `kinematic-order-fix/` for the measurements;
* `convex-theory/` and `param-lit/` for convexity and the literature.

**Status.**

* **Solved symbolic cases:** a net gain over the 26 cases.
* **Box-type graphs:** large gains.
* **Regressions:** real ones exist, on Cx12_438, C438 and tiny cases.
* **Unresolved symbolic cases:** none closes, because ordering is not the
  limiting factor for the unresolved 2-loop graphs.

## The ordering mode

`ordering = "kinematic_portfolio"` applies to parametric Feynman inputs under
three conditions:

* the last variable group is exactly `homogeneous_block`;
* earlier groups hold the kinematic parameters;
* the engine is `optimized`, `baseline`, or `monotone` with an optimized base.

Otherwise the mode falls back to MODS and records why. An explicit
`solver.order` is never replaced.

**Candidates.**

* The candidates are, in this order:
  1. the default MODS candidate;
  2. every anchor that passes the positive-scale certificate;
  3. the input anchor's variants last.

  Each anchor is combined with every within-group order of the kinematic
  variables. MODS, run with the kinematic groups merged, picks the Feynman
  order.
* Orders that lift a decoupled kinematic variable before a coupled one are not
  probed. They give the same product decomposition plus constant resultants.

**Probe.** Each candidate runs in a killable worker. The probe:

* follows the solver's own path (sign filter, homogeneous reduction, monotone
  peel, `prepare_projection` with the same positivity options);
* counts the exact open cells of the kinematic prefix with the solver's root
  isolation.

**Selection.**

* Ranking: fewest kinematic cells, then fewest projection terms, then
  enumeration order.
* The winner is written back as `ordering = "given"` with its order and anchor,
  so the unchanged solver and verifier certify it.
* The verifier refuses a result whose problem still asks for the portfolio.
* Every candidate, budget and score is in `statistics.ordering_portfolio`.

**Budgets.** All options are documented in [docs/USAGE.md](../docs/USAGE.md):

| Option | Default |
|---|---|
| `portfolio_default_budget_secs` | wall/10, clamped to 1–60 s |
| `portfolio_min_budget_secs` | 0.25 s |
| `portfolio_budget_factor` | 2 |
| `portfolio_total_budget_secs` | wall/4 |
| `portfolio_fiber_samples` | 8 |
| `portfolio_lifting_budget_factor` | 1 |
| `portfolio_kinematic_cell_cap` | 1,000,000 |
| `portfolio_fixed_budgets` | false |

**Policy refinements** made after measurement:

* The task's first rule (1 s floor) made the 26 solved cases go from
  107.4 s to 585.3 s. The floor is now 0.25 s.
* Budgets tighten once a candidate beats the default.
* The alternatives together are bounded by an estimate of the default's lifting
  time. The estimate is 0.41–0.73 of the real `lifting_seconds` (median 0.65),
  so the bound is conservative.
* `portfolio_fixed_budgets = true` makes the choice reproducible but costly:
  1,317 s on the 26 cases in one run.

## Measured gains and regressions

Measurement setup:

* binary sha256 `79a2b8a3…7ebd`;
* 3 runs per case, medians reported;
* 5,120 MiB per run, one pinned core each, 3 jobs at a time;
* host load about 45–55;
* every complete result verified.

| Set | Default MODS | Portfolio | Notes |
|---|---:|---:|---|
| 26 solved symbolic cases, sum of medians | 105.88 s | 92.97 s | 156/156 runs verified |
| C531 | 21.22 s | 2.27 s | 11,413 → 36 kinematic cells |
| C95 | 22.03 s | 3.00 s | |
| A159 | 4.36 s | 0.86 s | 1,240 → 9 kinematic cells |
| A531 | 3.55 s | 0.61 s | |
| Cx12_438 | 32.30 s | 59.82 s | default kept; its 23.6 s probe projection is computed twice |
| C438 | 8.37 s | 10.56 s | default kept |
| A376 | 0.89 s | 1.41 s | |
| 4 held-out cases where default MODS exceeds 5 GiB | out of memory | A286 5.73 s, Ax286 12.51 s, C158 45.99 s, Cx158 47.57 s | newly solved and verified |
| 36 tiny held-out cases | 2.07 s | 7.73 s | overhead about 0.16 s per case |

## Effect on the unresolved symbolic cases (measured)

| Case | Setup | Outcome |
|---|---|---|
| A317 | 600 s | out of memory in both modes; the portfolio spent 150 s |
| A318 | 600 s | incomplete in both modes |
| C231 | 600 s | four 5 s alternatives scored; the chosen order ran out of memory at 203 s; the default times out |

**No unresolved symbolic case closes with the portfolio.**

## Parametric analysis

Source: `param-profile/`, `convex-theory/`, `param-lit/`.

### Kinematic cells drive cost

The number of open kinematic cells drives solve cost monotonically. A predictor
of the `(s,t)` CAD cell count matched the lifting trace exactly in 8 of 8 runs.

A159, all s-first:

| Kinematic cells | Solve |
|---:|---|
| 11 | 0.082 s |
| 1,232 | 6.03 s |
| 6,665 | memory failure at 5.4 GiB |

### Landau versus spurious factors

**The physical singularities.** The leading singularities of every coordinate
face, saturated by U, give the physical (Landau) curves. The degenerate faces
whose contracted loop makes F vanish need their initial forms; without those,
A317's physical branch is missed. The positive Landau set has only 2–3 chambers
per graph.

**The CAD factors are mostly spurious:**

| Case | Landau-type factors | Physical (positive) factors | Spurious |
|---|---|---|---|
| A159, default order | 1 of 33 s-factors, 6 of 17 t-factors | 2 | the rest |
| C438, best order | 3 of 223 t-factors | 1 (`t = 4`) | 220 of 223 |
| A317 (projection-only order, anchor x3) | about 2 of 2,414 s-factors | | about 2,412 |

* Projection-only (A317 order `[s,t,x3,x2,x0,x5,x4,x8]`): the s-level reaches
  degree 127, about 3,252 s-roots above 4, and about 10⁶ kinematic cells.
* Landau-type factors appear in essentially every order. Only the spurious ones
  depend on the order.

**The theorem behind a sound use (proved).** By stratified Morse theory, the
fiber topology of `{F>0}` is constant on each connected component of the domain
minus the positive Landau set. This justifies the Landau set as a
*completeness target* and a diagnostic. It does **not** justify reusing one
cylindrical decomposition across a chamber: the order's spurious factors are
still CAD boundaries.

### Minkowski convexity failure

The positive Gluza33 structure (convex `F/U`, ratio orders) does not carry over
to the ZZ problems:

* **Gluza33 convexity.** For two-point graphs, `F/U = L − Σ λ_a R_a` is convex
  exactly when the external kinematic Gram matrix is PSD. That gives the convex
  cone `{F<0}` used for Gluza33.
* **ZZ kinematics.** The ZZ problems have a massless leg (`p1² = 0` with
  `p1·p2 = s/2 ≠ 0`), so the Gram matrix is indefinite for every `(s,t)`.
  There is an exact counterexample at `s = 10`, `t = −3`:
  `∂²(F/U)/∂x1² = −120400545/254527054` at `x = (3/2, 5/2, 1, 8/7, 2, 8, 1)`
  (`convex-theory/`).
* **What survives per edge.** `W_y = U_{\y}·F_{/y} − U_{/y}·F_{\y}` is affine in
  `(s,t)`, so each convexity chamber is convex. On `u = 5/9 − s − t < 0`, the
  coefficient signs of `W_x1` and `W_x5` certify `disc F > 0`. But `W_x8` takes
  both signs, so monotonicity in that massless edge fails.
* **Related failure.** The related one-axis guide premise fails on all 39
  linear axes of the 27 unresolved ZZ originals
  ([ZZ_RAYLEIGH_GUIDE_AUDIT.md](ZZ_RAYLEIGH_GUIDE_AUDIT.md)). This is why
  `bilinear_general` and its projective plan do not apply there; see
  [PROJECTIVE_SECTIONS.md](PROJECTIVE_SECTIONS.md).

### Why the unresolved 2-loop cases need relevance, not ordering

**Ordering helps only for factorizable graphs.**

* For box-plus-bubble or tadpole graphs (A159, C531, C95), anchoring in the box
  removes most spurious factors. The `param-profile` study measured 26–312× for
  the best order, excluding search cost; the portfolio, which includes search
  cost, gives the smaller gains in the table above.
* For the genuine 2-loop theta graphs (A317, C231), every order screened keeps
  a huge spurious kinematic alphabet. The best gain is about 2×.

**What remains requires relevance filtering.** Spurious kinematic factors must
be removed with exact certificates, with the positive Landau set as the
completeness target. Two routes:

* positive-root delineability over kinematic cells;
* local projection over kinematic cells.

A chamber-wise order would need a contract extension:

* a per-chamber order;
* chamber constraints in the verifier;
* a checked exhaustive cover of the kinematic domain.

None of this is implemented.

## Open risks

* **Tuning set.** Budgets were tuned on the 26 solved cases. The 40-case
  held-out set is mostly tiny.
* **Wall-clock budgets.** In adaptive mode the choice changed across 3 runs in
  1 of 26 cases.
* **Duplicated projection.** When the default wins, its projection is computed
  twice. Reusing the probe's projection in the final solve would remove the
  Cx12_438 regression.
* **Shared caches.** Sharing the on-disk algebra cache between probes and the
  final solve made things worse (Cx12_438 went to 534 s), so probes ignore
  `checkpoint_dir`.
