# Algebra kernels and projection changes of 2026-10-12

Date: 2026-10-12.

**Code** (branch `claude/algebra-fixes`):

| Commit | Change |
|---|---|
| `0e77b99` | `project`/`solve` parity |
| `dd524dd` | domain-free cache keys |
| `f8d2500` | kernel admission; segmentation default |
| `da5575e` | strict-Bezout pair omission |
| `ec518c7` | docs |
| `716dd18` | review fixes: zero images, admission, hint caps, replay |

The backend patch is `patches/symbolica-ducos-zero-subresultant.patch`.

**Evidence** (machine-local): `artifacts/research/claude-2026-10-12/algebra-fixes/`
and `fix/`, with `REPORT.txt` and `measurements.json`. All runs used one pinned
CPU on a shared host with load 33–53. The same run varied by up to 1.5× between
sessions, so same-session pairs are quoted where available.

**Status.**

* **Exactness:** every change keeps exact, deterministic algebra and is replayed
  by the verifier.
* **Speed:** two production gates got faster: the Gluza33 R+− projection, and
  large one-parameter discriminants.
* **Corpus:** none of the 75 corpus inputs that were re-run uses the new
  kernels, so there is no corpus-wide speed claim.

## (a) Proved-degree interpolation admission (`src/resultant_interpolation.rs`)

**Degree bound.**

* One-parameter resultants and discriminants interpolate from exact integer
  images.
* The image count is a *proved* degree bound plus one.
* The bound is the smaller of the old closed-form Sylvester bounds and a new
  exact maximum-weight assignment bound on the Sylvester matrix.
* The assignment bound's LP dual is verified entry by entry, so correctness does
  not depend on the Hungarian search.

**Admission.**

* **Historical envelope:** plans whose closed-form bound is at most 1024 keep
  the historical caps.
* **All other plans,** including one brought under 1024 only by the assignment
  bound, are admitted only if an a-priori Hadamard bit bound on every image, and
  on all images together, fits the image caps. So no dynamic resource stop can
  fire after admission.
* **Hard ceiling:** degree 65,536.

**Images.**

* Each image is an exact integer evaluation followed by a deterministic integer
  resultant; the rational scale is restored exactly.
* **Zero-image guard.** Symbolica 3.0.1's Ducos loop panics when an image
  resultant is zero (see (f)). So the kernel first decides whether the image is
  zero:
  * a degree-zero gcd modulo a fixed prime that keeps both degrees proves the
    operands coprime;
  * otherwise Symbolica's integer gcd decides.

  Ducos only ever runs on coprime operands.
* **Spot check.** Every interpolated or segmented result is rechecked at one
  extra full-degree point through a different route: Symbolica substitution plus
  its Brown resultant over Q. A mismatch is an error. This runs in both the
  solver and the verifier.

**Measured kernels.** All outputs have the same canonical hash; operands are
from the Gluza33 R−− piece.

| Operand | New interpolation | Binary segmentation | Earlier |
|---|---|---|---|
| disc2, bound 1,828 | 27.80 s, 18 MB | 71.88 s, 109 MB | old interpolation 52.31 s; old default > 660 s |
| disc3, bound 2,220 | 58.83 s | 128.35 s | old interpolation 106.79 s |
| disc4, elimination degree 68, bound 7,256 | declines | declines | unchanged (Ducos) |

* The disc2 and disc3 interpolation times are fast sessions. The loaded-session
  times were 42.92 s and 83.71 s.
* The spot check costs nothing measurable on disc2: 27.86 s against 27.87 s in
  the same session.

## (b) Binary segmentation is on by default (`src/resultant_segmentation.rs`)

* `binary_resultants` now defaults to `true`; set `false` to opt out. The
  opt-out survives the hand-off to the worker process.
* Interpolation keeps priority for one-parameter discriminants.
* **Pair applicability:**
  * exactly two surviving parameters;
  * at most 8,192 combined input terms;
  * at most 256 million encoded output bits.
* **Corpus check.** All 75 optimized-engine inputs of `completion-deterministic`
  give identical tables and cells with segmentation on and off. Segmentation was
  never admitted on any of them: 48.64 s against 52.92 s is noise.

## (c) Strict-Bezout pair omission in global projection (`src/strict_bezout_projection.rs`)

**The rule.**

* Option `projection_strict_bezout_hints`, allowed only with `positivity = true`
  and an engine with certified global projection:
  * `optimized`;
  * `linear_negative`;
  * `monotone` with an optimized base.

  Validation refuses it elsewhere.
* A hint gives `p·L + q·M = k·S` with `k > 0`, where either:
  * `S = H²` and H is certified nonzero by a `domain.rs` certificate on the open
    coordinate box; or
  * S itself is certified positive.
* Then `p` and `q` never vanish together in the box, and `Res(p,q)` is omitted
  at every level where both appear.
* What is kept: `p`, `q`, their leading coefficients and discriminants. Pairs
  involving a coordinate-face polynomial are never omitted.

**Verifier replay.** Every omission is recorded in
`optimized_projection.pair_omissions`. The verifier rechecks:

* the identity;
* the certificate against the original domain;
* the level, the axis and the pair's membership;
* duplicates.

Rejected hints are counted, never assumed.

**Hint text is untrusted.**

* A structural bound on Symbolica's unexpanded parse tree must stay within
  16,384 terms, total degree 64 and 65,536 coefficient bits.
* Exponents must be integer literals and `k` a decimal rational literal.
* This runs before expansion, because Symbolica's parser computes numeric powers
  itself: `2^99999999` took 5.38 s and `7^(3^20)` never returned.
* Each such input is now rejected in under 0.5 s.

**With a homogeneous anchor,** hints are written in the reduced coordinates
(anchor = 1).

**Measured on Gluza33 R−−** (`hints_Pmm.toml`, 8 hints, all accepted, 2 pairs
omitted at the `x6` elimination). Cells give polys/terms/max degree:

| Level | Control | With omission |
|---|---|---|
| after `x6` | 18/451/5 | 16/403/5 |
| after `x0` | 59/4,355/11 | 44/3,476/11 |
| after `x1` (level 3) | 1,033/228,998/112, at 164.2 s | 661/112,759/112, at 122.4 s |

* At level 3 that is 36% fewer polynomials and 51% fewer terms, with the same
  maximum degree.
* The last elimination was not run.
* An exploration-CAS projection of the same piece agrees: 1,025 against 655
  polynomials at level 3 (`critique/measurements.json`).

**Soundness argument.** Inside the box, roots can only enter or leave through
the face resultants, and those are never omitted. The review fuzzed the rule
with H vanishing on the box boundary and found 1,287 matching points per run.

## (d) Domain-free operation cache keys (`src/optimized_projection.rs`)

* Factor, resultant and discriminant records no longer include the domain
  fingerprint in their key. They still include:
  * the full ordered variable map;
  * backend and algorithm version;
  * the operation, the eliminated axis and the exact input.
* **Why this is sound:** the only writers store canonical results before any
  certificate, omission or positivity filter.
* **Old caches:** domain-keyed records are read, validated against their
  original header, and migrated.
* **Measured:** running R+− then R−− with one checkpoint directory, R−− got 50
  disk hits out of 127 operations through level 2. The old keys gave 0.
* **Caveat:** ablations of `weighted_square_positivity` or
  `quadratic_component_positivity` now share records with ordinary runs. Clean
  timing comparisons need separate `checkpoint_dir`s.

## (e) `symgcad project` computes what `solve` lifts

* **The bug:** `project` ignored the homogeneous anchor reduction. It projected
  all 6 base variables, so the last elimination kept two parameters and
  interpolation never applied.
* **The fix:** it now uses the same dispatch predicate and optimized-engine path
  as `solve`. Its output names the anchor and the original order.
* **Test:** project tables equal the reduced solve's tables on three engine
  paths.
* **Gluza33 R+− (`Ppm_O1`):**
  * Before: timed out at 1,500 s.
  * After: complete in 354.5 s and 453 MB.
  * Level sizes: 13/269/5, 26/1,079/10, 175/10,131/24, 22,853/350,623/156, with
    6,831 interpolated resultants.

## (f) Upstream fixes

### Ducos panic on a vanishing subresultant

**Upstream:** [symbolica-dev/symbolica#76](https://github.com/symbolica-dev/symbolica/pull/76), open.

**Cause.** `UnivariatePolynomial::mul_exp` turns the zero polynomial into
explicit zero coefficients. The Ducos loop then misses `b.is_zero()`:

* debug builds: subtraction overflow at `resultant.rs:172`;
* release builds: out-of-bounds index at `resultant.rs:375`.

It only happens when the resultant is 0. The minimal case is
`Res(3x^5, −x^3−3x)`.

**Reach in production.** The new admission sent inputs with zero images to the
image path. The review's reproducer is `panic_pair.toml`, a degree-8 pair plus
`s^71`: the old binary solved it; the pre-fix branch crashed. The fix solves it
as `complete_generic` with 8 cells.

**Local patch.**

* `patches/symbolica-ducos-zero-subresultant.patch` makes `mul_exp` return the
  zero polynomial unchanged and truncates the subresultant. It adds regression
  tests.
* `scripts/bootstrap-deps.sh --apply-patches` applies it, but it is **not**
  linked into production builds.
* Production avoids the path through the zero-image guard in (a), and through a
  non-vanishing proof before full Ducos in `projection::exact_resultant_ducos`.

**Review statistics.** The panic occurred in 621 of 40,000 zero-resultant
integer cases and in 4 of 3,000 full bivariate cases. It never produced a wrong
value in 60,000 checks.

### Integer left-shift overflow

**Upstream:** [symbolica-dev/symbolica#75](https://github.com/symbolica-dev/symbolica/pull/75), open.

* The bug: `Integer::shl_usize` treats primitive `checked_shl` as an overflow
  check. `Integer::from(1) << 63usize` gives a negative value.
* Local patch: `patches/numerica-integer-left-shift.patch`, optional.
* No production path in `src/` is affected.

### Earlier: linear-operand resultant sign

**Upstream:** [symbolica-dev/symbolica#69](https://github.com/symbolica-dev/symbolica/pull/69), open.

* Local patch: `patches/symbolica-linear-resultant-sign.patch`.
* Production computes linear operands with its own signed identity.

## Review findings that shape future work

* **Verifier independence is lower than before.**
  * Global replay of one-parameter resultants above bound 1024 now uses the same
    interpolation kernel as the solver.
  * Local replay uses segmentation for discriminants.
  * The spot check through Brown/substitution is the mitigation. The review
    found no wrong values.
* **Coverage gap.** No corpus input exercises the new kernels. They are covered
  by unit and fuzz tests and by the Gluza33 R+−/R−− operands only.
* **Unrelated verifier cost.** `verify` on `panic_pair` takes about 7.4 min, all
  in `roots::roots_in_open_interval`. This predates the branch.
