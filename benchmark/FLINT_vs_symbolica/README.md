# FLINT vs Symbolica: exact algebra kernels on real symGCAD operands

This folder holds a reproducible benchmark of the exact polynomial kernels that
symGCAD's projection runs on Symbolica, timed against the corresponding FLINT
routines on the **same exact integer inputs**. The inputs were harvested from
real symGCAD projections (Gluza30/33, A317/A318/A377 and the parametric
finiteA problems). The benchmark answers two questions. Is Symbolica, as
symGCAD uses it, a bottleneck compared with a state-of-the-art C library? And
which kernels would gain most from upstream work or from a FLINT-style
algorithm?

Everything here is benchmark tooling. Production symGCAD does not link FLINT.
The Symbolica-side hooks sit behind the off-by-default cargo feature
`flint-bench-hooks`.

**Status.** The campaign ran on 2026-10-12. The committed results are a
snapshot taken at 12:57 UTC while the campaign was still running; see
[Results](#results-snapshot).

* **Complete:** every harvested operand (Gluza30, Gluza33, fixed A377 and the
  parametric finiteA problems), plus the A317 and A318 pair resultants.
* **Partly measured:** the A377 pair resultants. Two of eleven have
  measurements on both sides, and a third has only its Symbolica production
  time.
* **Not measured yet:** the four Gluza33 critique discriminants, the
  stalled Gluza33 resultant, the remaining A377 operands, both operations in
  progress at the harvest deadlines, and every large-tier segmentation
  encoding and interpolation image (30 operands, 126 of the 772 jobs).

Unmeasured operands appear as "not run" (`pending`), never as timeouts.
`orchestrate.py` can resume the campaign at any time.

## What is compared

Each operand is one exact operation, `symgcad-flint-bench-op 1` text: integer
coefficients, exponent vectors, variable names and the eliminated variable.
Both programs read the same file. They run one kernel single-threaded and dump
the exact result, and `scripts/canon.py` hashes it.

| Operation (kind) | Symbolica: production path (`sym-production`) | Other Symbolica kernels timed | FLINT 3.6.0 |
|---|---|---|---|
| **resultant** of two multivariate polynomials (projection pairs) | symGCAD dispatch: bounded **binary segmentation** (`resultant_segmentation::try_resultant`: Kronecker-encode two parameters into huge integers, one univariate integer resultant, centred decoding, independent spot check), otherwise `projection::exact_resultant_with_statistics` (bounded **exact interpolation** in one parameter, the linear identity, or the deterministic **integer Lazard–Ducos** fallback) | `ducos` = `projection::exact_resultant_ducos` (the production fallback: exact vanishing test, then Ducos over Z[params]); `brown`, `crt` = Symbolica's own `resultant_brown` / `resultant_crt` (not used by production; CRT is Monte Carlo) | `fmpz_mpoly_resultant` (Ducos subresultants over Z[params], `mpoly_univar_pseudo_gcd_ducos`) |
| **discriminant** | `projection::discriminant_with_binary_statistics`: closed formula for degree ≤ 4, otherwise interpolation, then 1–2-parameter segmentation, then Ducos | `ducos`/`brown`/`crt` on (p, ∂p), divided by the leading coefficient with the discriminant sign | `fmpz_mpoly_discriminant` |
| **factor** | `symgcad::algebra::factors` (the production *factor basis*: distinct irreducible non-constant factors, normalized) | `native` = Symbolica `factor()` with multiplicities | `fmpz_mpoly_factor`; univariate inputs also `fmpz_poly_factor` |
| **gcd** (P, ∂P/∂v) of harvested factor operands: the first square-free step | Symbolica `gcd` (`sym-native`) | – | `fmpz_mpoly_gcd` (dispatcher over Brown / Hensel / Zippel); univariate `fmpz_poly_gcd` |
| **Z[x] resultant (segmentation)**: the exact univariate integer pair that production segmentation hands to its numeric kernel (degree ≤ 12, coefficients of 0.5–4.5 million bits) | `integer_resultant` (linear identity, exact shared-factor test, then Symbolica `UnivariatePolynomial<Z>::resultant`, i.e. Lazard–Ducos) | `ducos` (same pair through the multivariate fallback) | `fmpz_mpoly_resultant`, `fmpz_poly_resultant` (its dispatcher picks the multimodular method here), `fmpz_poly_resultant_euclidean` |
| **Z[x] resultant (interp. image)**: one of the `degree_bound+1` univariate images that production exact interpolation computes | `integer_resultant` | `ducos` | as above |

Correctness is checked on every pair of outputs.
* **Resultants and discriminants:** the exact polynomial must be identical,
  including sign and scale (`exact` hash).
* **Factorizations:** the multiset of primitive associates with
  multiplicities must match (`unit` hash). Production's factor basis is
  compared as a set (`unit_set`).
* **gcds:** must agree up to a unit.

Outputs are compared against FLINT `fmpz_mpoly`. When FLINT produced no
output, they are compared against Symbolica production.

Versions and pins:

* **Symbolica:** 3.0.1 at git rev `75f8350094b90254ee71dc2a391fde0d14b0204a`
  (`[patch.crates-io]` in `Cargo.toml`), `integer-gmp`, the repository's
  `Cargo.lock`.
* **symGCAD:** the campaign binaries were built at `4dbe440` plus the
  benchmark hooks. This folder builds them at `9baf91c` plus the hooks; the
  kernels did not change between the two.
* **FLINT:** 3.6.0, with GMP 6.3.0 and MPFR 4.2.2, from the flake's
  `reference` dev shell (`flake.lock` pins nixpkgs).
* **Machine:** AMD EPYC 9754 (128 cores, 384 threads), 1.1 TiB RAM, NixOS.
  Other jobs were running at the same time.

## Methodology

* **Isolation:** one fresh process per (operand, side, kernel) job. The job
  is pinned to one CPU with `taskset`. `RAYON_NUM_THREADS=1`,
  `OMP_NUM_THREADS=1` and `flint_set_num_threads(1)` are set. Two workers run
  at once, on two distinct cores.
* **Repetitions:** the kernel is repeated until the cumulative time reaches
  1 s or 100 repetitions. The **minimum** is reported (the median is also
  recorded). Parsing, conversion and output dumping are excluded.
* **Caps:** 600 s for Symbolica production and every FLINT kernel. 300 s for
  Symbolica Ducos when production does not choose it. 120 s for the
  exploratory kernels: Symbolica `brown`/`crt`, and `fmpz_poly_resultant` on
  segmentation encodings, whose multimodular choice is hopeless there. Three
  exploratory jobs ran before that cap was lowered and stopped at 300 s or
  600 s. Each job also has a 16 GiB address-space limit.
* **Memory:** GNU `time` records each job's peak RSS. Jobs killed at their
  cap record only about 3 MB, which looks like the `timeout` process's own
  RSS: the killed child's peak is not captured. Ignore RSS on capped jobs.
* **Ratio:** Symbolica production time / best FLINT kernel time. A ratio
  below 1 means Symbolica is faster.
* **Size class:** taken from the slower side. `capped` means a side hit its
  cap or failed. `pending` means the job has not run yet.

### Operands and how they were harvested

1. **Harvest** (`harvest/harvest.sh`). This runs `symgcad project` on six
   projection inputs (`harvest/inputs/*.toml`). The `symgcad` binary is built
   with `flint-bench-hooks`, `SYMGCAD_CAPTURE_DIR` is set and
   `SYMGCAD_CAPTURE_ALL=1` is exported. Every completed algebra operation is
   then written as an `OperationCapture` JSON file: operands over Q, variable,
   input hash and elapsed time. The six inputs are:
   * `g33D`: Gluza33 base {d>0}.
   * `g33S2`: Gluza33 base S2 {a<0, d>0; split b,c}.
   * `g30base`: positive Gluza30 two-fiber ladder base.
   * `fixedA377`: finiteA_6_377 at s=10, t=−3.
   * `symA5_31` and `symA317`: finiteA_5_31 and finiteA_6_317 with symbolic
     s, t.

   g33S2 and symA317 were killed at their deadline (1500 s and 900 s).
2. **Large captures.** The "large" tier is the 18 captures listed in
   `harvest/selection.json`, taken from earlier campaigns: the Gluza33
   critique discriminants, the Gluza33 resultant stalled at the deadline, and
   the A317/A318/A377 pair resultants.
3. **Selection** (`scripts/build_manifest.py`) is deterministic. Per problem
   and operation it takes the slowest half of the quota by recorded capture
   time, plus an even stride through the rest, deduplicated by input hash. It
   also takes the operation in progress at each harvest deadline.
   `convert` clears denominators; no committed operand needed scaling.
4. **Derived operands.** `flint_vs_symbolica derive` writes the operands
   production really hands to its numeric kernels:
   * the Kronecker-encoded univariate pair of binary segmentation (`-segenc`),
     for every operand where production would use segmentation;
   * the middle interpolation image (`-interpimg`);
   * (P, ∂P/∂v) for the 24 largest harvested factor operands (`-gcd`).

`operands/manifest.json` lists **all 195 operands of the reported campaign**:
sha256, byte size, sizes, operation and full provenance. Provenance covers the
capture file and its sha256, the capture's input hash, the selection rule,
the derivation parent and the interpolation point. Of the 195:

* 137 non-derived operands are committed as `operands/sources/<id>.op.xz`
  (2.7 MB).
* 46 derived operands are regenerated bit-exactly from them by
  `scripts/materialize.py` (sha256-checked).
* 6 large Gluza33 S2 factor operands (1.6–2.3 MB each), and the 6 gcd
  operands derived from them, are omitted for size. They need a re-harvest
  of g33S2; see below.

`operands/subset.json` is the reproduction subset. It has 26 timed operands
covering every operation kind and every size class the campaign measured.
It also adds one interpolation image that the campaign has not reached yet,
and two small parametric operands.

## Folder layout

```
README.md                      this file
symbolica/flint_vs_symbolica.rs  Symbolica side: convert | derive | run  ([[example]] in Cargo.toml)
flint/flint_bench.c, Makefile  FLINT side (fmpz_mpoly / fmpz_poly kernels)
harvest/harvest.sh             capture every operation of one projection run
harvest/inputs/*.toml          the six harvested projection inputs (checkpoint_dir is injected)
harvest/selection.json         large captures, quotas, deadlines of the reported campaign
scripts/build_manifest.py      captures -> operand files + manifest (full pipeline)
scripts/materialize.py         committed sources -> operand files (+ derived), sha256-checked
scripts/orchestrate.py         run the job matrix (resumable, N pinned workers)
scripts/run_one.sh             one bounded pinned measurement + output hashing
scripts/canon.py               canonical hashes of an exact output dump
scripts/collect.py             results -> summary.json, summary.md, per_op.csv, per_job.{csv,json}, table.typ
scripts/compare.py             re-run vs committed results, job by job
scripts/reproduce_subset.sh    materialize + run + collect + compare the subset
scripts/import_campaign.py     one-off: campaign manifest -> committed manifest/sources
scripts/symenv.py              Symbolica subprocess environment (license file, 1 thread)
operands/manifest.json         all 195 operands with sha256 + provenance
operands/sources/*.op.xz       137 committed operands
operands/subset.json           reproduction subset
results/                       campaign snapshot + reproduction record (see below)
```

## Build and run

All commands run from the repository root. The Symbolica side needs
`SYMBOLICA_LICENSE`, exported by `.symgcad-env` or by the file that
`SYMGCAD_ENV` points to. Without a license, Symbolica allows only one
restricted thread per user, which is too few for two concurrent workers.

```sh
# Symbolica side (+ the harvest binary), ~11 min from scratch with -j 8
nix develop -c cargo build --release --locked -j 8 --features flint-bench-hooks \
    --example flint_vs_symbolica --bin symgcad
# FLINT side (FLINT 3.6.0 from the flake's reference shell)
nix develop .#reference -c make -C benchmark/FLINT_vs_symbolica/flint

# Reproduce the committed subset (~10 min with two workers on CPUs 2,3)
nix develop .#reference -c benchmark/FLINT_vs_symbolica/scripts/reproduce_subset.sh \
    benchmark/FLINT_vs_symbolica/work/subset 2,3
```

Full campaign, from the committed operands. This reproduces 183 of the 195
operands; the rest need the harvest below.

```sh
F=benchmark/FLINT_vs_symbolica
nix develop .#reference -c python3 -I $F/scripts/materialize.py $F/operands/manifest.json $F/operands/sources $F/work/ops
nix develop .#reference -c python3 -I $F/scripts/orchestrate.py $F/operands/manifest.json $F/work/ops $F/work/results 2,3
nix develop .#reference -c python3 -I $F/scripts/collect.py $F/operands/manifest.json $F/work/results $F/work/out
```

`orchestrate.py` skips jobs that already have a `.status` file. Interrupting
and restarting it therefore resumes the campaign. `--ids`, `--filter` and
`--dry-run` restrict or list jobs. The whole matrix has 772 jobs. The caps
bound it at roughly 50 CPU-hours worst case, and most of that is the
large-tier ceiling.

From scratch, starting with a harvest:

```sh
for p in g33D g30base fixedA377 symA5_31 g33S2 symA317; do
  nix develop .#reference -c benchmark/FLINT_vs_symbolica/harvest/harvest.sh $p 2 benchmark/FLINT_vs_symbolica/work/harvest
done
nix develop .#reference -c python3 -I benchmark/FLINT_vs_symbolica/scripts/build_manifest.py \
    --harvest-dir benchmark/FLINT_vs_symbolica/work/harvest --ops-dir benchmark/FLINT_vs_symbolica/work/ops --cpu 2
```

`build_manifest.py` also takes `--captures-root`, which holds the large-tier
captures from `selection.json`. These live in the ignored `artifacts/` tree of
the original checkout, and all large-tier sources are committed anyway.

Run against the original harvest directory and capture tree,
`build_manifest.py` (at `9baf91c` plus the hooks) regenerated the campaign
manifest exactly. The 195 ids, their order, every sha256 and every
selection count matched. The killed harvests (g33S2, symA317) depend on
machine speed, so a fresh re-harvest may select slightly different
operations. That is why the committed sources and the sha256 manifest are the
reference.

## Results (snapshot)

`results/` holds:

* `summary.json`: aggregates plus one record per operand.
* `summary.md`: the tables below.
* `per_op.csv`: one row per operand.
* `per_job.csv` and `per_job.json`: one row per job, with output hashes.
* `table.typ`: self-contained Typst content for `benchmark/benchmark.typ`
  (`#include "FLINT_vs_symbolica/results/table.typ"`).
* `reproduction/`: the record of the subset re-run.

Snapshot of 2026-10-12, 12:57 UTC (campaign still running):

* 195 operands and 772 jobs.
* 607 jobs ok, 126 not yet run, 39 stopped at their cap. Of the capped
  jobs, 34 are exploratory 120 s jobs. The other 5 are FLINT
  `fmpz_mpoly_resultant` and Symbolica Ducos on `a377-1d3e6e9835cc`, plus
  three early exploratory jobs.
* 441 exact outputs compared, **0 mismatches**.

Ratio = Symbolica (symGCAD production kernel) time / best FLINT time;
< 1 means Symbolica is faster.

| Operation | operands | timed on both sides | geo. mean ratio | total-time ratio | Symbolica total | FLINT total |
|---|---:|---:|---:|---:|---:|---:|
| resultant | 67 | 56 | 0.67 | 0.19 | 320 s | 1649 s |
| discriminant | 34 | 28 | 0.51 | 0.22 | 97.1 s | 451 s |
| factor | 42 | 42 | 2.28 | 5.90 | 45.4 s | 7.7 s |
| gcd | 24 | 24 | 2.31 | 9.32 | 16.4 s | 1.8 s |
| Z[x] resultant (interp. image) | 2 | 0 (see reproduction: 1.70) | – | – | – | – |
| Z[x] resultant (segmentation) | 26 | 14 | 0.96 | 0.97 | 267 s | 277 s |

| Operation | size class | n | geo. mean ratio | range | Symbolica faster | Ducos / fmpz_mpoly |
|---|---|---:|---:|---:|---:|---:|
| resultant | <1 ms | 32 | 1.08 | 0.38–2.74 | 16/32 | 1.08 |
| resultant | 1-100 ms | 5 | 0.84 | 0.37–1.73 | 3/5 | 0.84 |
| resultant | 0.1-10 s | 3 | 0.98 | 0.94–1.02 | 2/3 | 0.98 |
| resultant | >=10 s | 16 | 0.23 | 0.06–0.80 | 16/16 | 0.79 |
| resultant | capped | 1 | (Symbolica 34.1 s, FLINT > 600 s) | – | – | – |
| discriminant | <1 ms | 18 | 0.76 | 0.30–1.71 | 13/18 | 1.13 |
| discriminant | 1-100 ms | 4 | 0.31 | 0.06–1.24 | 3/4 | 0.94 |
| discriminant | 0.1-10 s | 1 | 0.06 | – | 1/1 | 1.54 |
| discriminant | >=10 s | 5 | 0.29 | 0.13–0.83 | 5/5 | 0.78 |
| factor | <1 ms | 17 | 1.03 | 0.17–9.84 | 10/17 | – |
| factor | 1-100 ms | 17 | 2.66 | 0.45–14.2 | 4/17 | – |
| factor | 0.1-10 s | 7 | 8.52 | 2.66–22.1 | 0/7 | – |
| factor | >=10 s | 1 | 11.7 | – | 0/1 | – |
| gcd | <1 ms | 10 | 1.21 | 0.19–3.00 | 3/10 | – |
| gcd | 1-100 ms | 6 | 1.15 | 0.39–3.42 | 3/6 | – |
| gcd | 0.1-10 s | 8 | 8.70 | 5.72–18.4 | 0/8 | – |
| Z[x] resultant (segmentation) | 0.1-10 s | 1 | 0.94 | – | 1/1 | 1.68 |
| Z[x] resultant (segmentation) | >=10 s | 13 | 0.96 | 0.93–1.07 | 12/13 | 1.63 |

`results/summary.md` holds the same tables, including the `pending` rows.

### Key numbers

The ratio is Symbolica production / best FLINT kernel; below 1 means
Symbolica is faster.

**Large symbolic pair resultants (binary segmentation).**

| Operand | Symbolica production | `fmpz_mpoly_resultant` | Ratio |
|---|---:|---:|---:|
| `a318-pair` | 15.7 s | 165 s | 0.095 |
| `a377-1d4048e35d3e` | 14.6 s | 230 s | 0.064 |
| `a377-1d3e6e9835cc` | 34.1 s | > 600 s (capped) | – |
| `a317-pair` | 29.2 s | 101 s | 0.29 |

On `a377-1d3e6e9835cc`, Symbolica's own Ducos also exceeded its 300 s cap.

**Gluza33 S2.**
* Segmentation pair resultants (10 operands): 12–37 s against 47–201 s
  (0.15–0.30).
* Segmentation discriminants (4 operands): 7.6–37 s against 35–278 s
  (0.13–0.32).
* Where production falls back to integer Ducos, the two sides are at parity
  (0.80–1.13, with one 0.37). Symbolica Ducos / FLINT Ducos on the
  ≥ 10 s operands: 0.74–0.88.

**Parametric kinematics (symA317, symA5_31; s, t symbolic).**
* Integer-Ducos resultants (4 active variables): 16.3 s and 34.7 s against
  23.4 s and 47.0 s (0.70–0.74).
* Closed-form discriminants (degree ≤ 4): 6–18× faster than
  `fmpz_mpoly_discriminant` (ratios 0.056–0.155 on the three largest).
* Sparse 4–5-variable factorizations: Symbolica is faster, 0.45–0.69 on
  symA317.
* gcds: 0.19–1.9 over 7 operands.

**Dense bivariate Gluza33 S2 factorizations (13–21 k terms).** FLINT is
faster here.
* Factorization: Symbolica 3.2–10.6 s against FLINT 0.17–2.8 s (2.7–22×).
* gcd(P, ∂P): 1.3–5.0 s against 0.14–0.27 s (5.7–18×).

**Segmentation numeric kernel.** Symbolica `integer_resultant` and FLINT's
best (`fmpz_poly_resultant_euclidean` or `fmpz_mpoly`) are at parity: ratio
0.93–1.07 over 14 operands. FLINT's default `fmpz_poly_resultant` exceeds
its cap on all 14: 120 s, or 600 s for two early jobs.

**Interpolation image** (`g33-disc2`, subset run). Symbolica takes 16.9 ms
and FLINT's multimodular `fmpz_poly_resultant` 10.0 ms: ratio 1.70.

### Caveats

* **Not yet measured.** 30 operands of the large captured tier are still
  `pending`; those rows are not timeouts. They include the operations that
  dominated historical symGCAD runtimes:
  * nine of the eleven A377 pair resultants (`a377-ac6f3048ce62` took
    3,041.8 s in the old ordinary backend);
  * the four Gluza33 critique discriminants and their interpolation images;
  * the stalled Gluza33 main-axis resultant (16,828 terms);
  * the two operations in progress at the harvest deadlines;
  * every large-tier segmentation encoding.

  One of the pending A377 operands, `a377-2084c3ad850e`, already has its
  Symbolica production time (10.7 s). The first A377 operand measured
  already hit FLINT's 600 s cap. Resume the orchestrator to fill these in.
* **One run per job.** Each job ran once in the campaign. The subset was run
  twice more (see the reproduction record); every other number comes from a
  single campaign run.
* **Clock.** Times are single-core wall-clock on a shared machine. Run-to-run
  spread is given in the reproduction record.
* **Small operands.** Below about 50 µs, ratios mostly reflect fixed
  overhead (conversion, allocation, dispatch), not algorithms.
* **The ratio is not a speed-up.** "Production / FLINT" compares symGCAD's
  algorithm choice with a single FLINT routine. For resultants and
  discriminants this choice already includes symGCAD's own segmentation and
  interpolation strategies. The like-for-like kernel comparison is the
  "Ducos / fmpz_mpoly" column: both are Ducos subresultants over
  Z[parameters].

## Interpretation

* **Resultants and discriminants (production beats FLINT on large inputs).**
  For the large pair resultants and discriminants, symGCAD production uses
  **bounded binary segmentation**. Two parameters are packed into huge
  integers, which leaves one univariate integer resultant whose output is
  decoded exactly. This applies to Gluza33 S2 (degree ≈ 12 in three
  variables, 400–1,000 terms) and to the symbolic A317/A318/A377 pairs
  (degree 6–22 in s, t and one variable, up to 2,560 terms). Production is:
  * 3–7.5× faster than FLINT's `fmpz_mpoly_resultant` on Gluza33 S2;
  * 3.5× faster on A317, 10.5× on A318 and 15.7× on `a377-1d4048e35d3e`
    (14.6 s against 230 s);
  * finished in 34 s on `a377-1d3e6e9835cc`, where FLINT did not finish in
    600 s.

  `fmpz_mpoly_resultant` runs Ducos subresultants over the multivariate ring
  and suffers intermediate expression swell. Like-for-like, Symbolica's own
  integer Ducos is about 0.74–0.88× FLINT's Ducos on large operands and about
  1.0–1.1× on small ones. So the Symbolica Ducos kernel is competitive; the
  large gain comes from the algorithm choice. On the parametric symA317
  resultants, where segmentation and interpolation decline and production
  runs integer Ducos, Symbolica is still 0.70–0.74× FLINT. Degree ≤ 4
  discriminants use closed formulas and are 6–18× faster than
  `fmpz_mpoly_discriminant` on the larger parametric ones. On
  sub-millisecond operands the two libraries are at parity (geometric mean
  ≈ 1.0, spread 0.4–2.7).
* **Segmentation numeric kernel (parity).** Segmentation reduces each such
  resultant to Z[x] with degree ≤ 12 and coefficients of 0.5–4.5 million
  bits. There,
  Symbolica's `integer_resultant` and FLINT's `fmpz_poly_resultant_euclidean`
  or `fmpz_mpoly_resultant` are within a few percent (ratio ≈ 0.97). Both are
  bound by GMP-class big-integer multiplication and exact division.
  FLINT's default `fmpz_poly_resultant` dispatcher picks the **multimodular**
  method for these shapes (`len³·bits > 6·10⁶`): it needs a Hadamard bound of
  millions of bits and does not finish in 120 s. Symbolica's multivariate
  Ducos route on the same pair is 1.6–1.7× slower than its univariate
  kernel, which is why production uses the univariate one.
* **Factorization and gcd (FLINT 3–20× faster on large dense inputs).**
  * On the large dense bivariate Gluza33 S2 factor operands (degree ≈ 200 in
    each variable, 13–21 k terms, coefficients of ~400 bits), FLINT's
    `fmpz_mpoly_factor` is 3–22× faster than Symbolica: 3.2–10.6 s against
    0.17–2.8 s.
  * The square-free step alone, gcd(P, ∂P), is 6–18× slower in Symbolica:
    1.3–5.0 s against 0.14–0.27 s. On `g33S2-fac-28` it accounts for about
    half of Symbolica's 10.6 s factorization (4.95 s). FLINT's gcd dispatcher
    can choose Brown's dense modular algorithm, which suits these dense
    bivariate inputs. Symbolica tries a GCDHEU-style heuristic and then a
    Zippel-type modular interpolation.
  * On seven of the fourteen **univariate** factor operands (degree 10–40),
    Symbolica needs a near-constant 1.2–1.6 ms where FLINT's
    `fmpz_poly_factor` needs 0.09–0.56 ms (2.2–14×). On the other seven
    (degree 4–26), Symbolica takes 7–593 µs, between 0.8× and 9.8× FLINT's
    time. The 1.2–1.6 ms plateau points to a fixed cost on one code path
    rather than to asymptotics.
  * The FLINT advantage is specific to dense, few-variable inputs. On the
    sparse 4–5-variable factor and gcd operands of the parametric problems
    (symA317, symA5_31), Symbolica is mostly faster: factorization 0.17–0.77
    except two univariate cases, gcd 0.19–1.9.
* **Symbolica's alternative resultant kernels.** `resultant_brown` hits its
  120 s cap on every large Gluza33 S2 operand. `resultant_crt` is competitive
  on some operands but is Monte Carlo. Production is right not to use either.

## Ideas for optimization

1. **Multivariate integer gcd in Symbolica (highest pay-off).**
   Square-free factorization of every projection factor goes through gcd.
   On dense bivariate inputs Symbolica is 6–18× slower than FLINT. Two
   possible upstream changes:
   * a dense Brown-style modular gcd path, chosen by a density estimate;
   * a better heuristic gcd (evaluation at a large integer, then integer gcd
     and reconstruction) for bivariate inputs with moderate coefficients.

   The harvested `-gcd` operands are ready-made regression benchmarks for
   either change.
2. **Multivariate factorization.** After the gcd fix, the remaining
   factorization gap (FLINT 3–20×) is Hensel lifting and recombination. The
   `g33S2-fac-*` operands show it best. Two options:
   * Inside symGCAD, `algebra::factors` needs only the *distinct*
     irreducible factors, never the multiplicities. A cheaper square-free
     part could replace the full square-free decomposition.
   * Behind a feature, an optional FLINT backend (LGPL, exact, deterministic
     for `fmpz_mpoly_factor`) could be benchmarked for large dense
     factorization.
3. **Univariate factorization overhead.** The ~1.2–1.6 ms floor on some
   degree 10–40 integer polynomials, against 0.09–0.56 ms in FLINT, points
   to a fixed set-up cost on one path. Candidates are prime selection,
   Hensel set-up, or recombination; the cause is not profiled yet. One
   Gluza30 projection performs 3,554 distinct factorizations (g30base
   harvest), so profiling `g30base-fac-10..13` and `fixedA377-fac-11..13`
   in Symbolica is cheap and worthwhile.
4. **Segmentation numeric kernel.** Both libraries are at parity, so the
   remaining levers are on the symGCAD side:
   * smaller encodings: tighter output bounds, since every bit removed from
     `digit_bits` is saved in every big-integer operation;
   * splitting one huge resultant into a few smaller ones (CRT over moduli
     2^k±1, or evaluation at several shifts);
   * a faster big-integer multiplication backend for million-bit operands.
     FLINT 3's `fft_small` is one candidate. It needs an AVX2-enabled
     build, and this campaign did not check whether the generic nixpkgs
     FLINT has it.

   The `-segenc` operands isolate exactly this kernel.
5. **Interpolation images (measured in the subset run).** Production
   computes `degree_bound+1` univariate resultants per interpolated
   discriminant: 1,829 for `g33-disc2`, each of degree 36/35 with
   coefficients of about 480 bits. On the middle image, FLINT's
   multimodular `fmpz_poly_resultant` takes 10.0 ms. Symbolica's
   `integer_resultant` (Ducos) takes 16.9 ms, about 1.7× slower, or roughly
   31 s instead of 18 s per interpolated discriminant. A multimodular
   univariate integer resultant has two possible homes:
   * in symGCAD, for images whose `len³·bits` is large but whose
     coefficients are moderate;
   * upstream in Symbolica, as the default for such shapes.

   Either is small and self-contained. Symbolica already has
   modular machinery; the deterministic Hadamard bound makes the result
   exact.
6. **Large tier.** Finish the pending measurements. So far, production
   segmentation beats `fmpz_mpoly_resultant` by 3.5–16× on A317, A318 and
   A377, and FLINT did not finish one A377 operand in 600 s. The pending
   A377 segmentation encodings have coefficients up to 3.7 million bits,
   twice the Gluza33 S2 ones. They will show whether the numeric-kernel
   parity holds at that size. If it does, idea 4 is where any further
   A377 time must come from.

## Reproduction record

The full record is in `results/reproduction/`. The procedure started from a
clean state on 2026-10-12, on branch `claude/flint-vs-symbolica`, base
`9baf91c`:

1. **Builds.** Both sides were built in a fresh worktree with an empty
   `target/`. The cargo command above took 11 min 10 s; the FLINT `make` used
   FLINT 3.6.0 and gcc 16.2.0. The default `cargo build --release --locked`
   (feature off) was also checked and builds.
2. **All committed operands.** `materialize.py` decompressed 137 operands and
   regenerated 46 derived ones. All 183 match the campaign sha256; the
   remaining 12 are the omitted g33S2 factor operands and their gcd children.
3. **Selection pipeline.** `build_manifest.py` was run against the original
   harvest and capture tree. It reproduced the campaign manifest exactly:
   195 of 195 sha256, the same ids in the same order, and the same
   selection counts. `harvest/harvest.sh` re-harvested `g33D` and
   `fixedA377` with the in-repo build. It captured the same 101 and 284
   operations, with identical input hashes and operations, as the campaign
   harvest.
4. **Subset runs.** `reproduce_subset.sh` ran twice on CPUs 220 and 230,
   while the campaign itself kept running on CPUs 140 and 150. Run 1
   (12:19–12:28 UTC) used the first build. Run 2 (12:47–12:57 UTC) used
   binaries rebuilt from the committed, rustfmt-formatted sources. Its files
   are in `results/reproduction/`.
   * Each run took about 9 min 40 s for 26 operands and 97 jobs.
   * Against the committed snapshot, 88 jobs completed in both the re-run
     and the campaign. All **88 exact outputs were identical**, in both
     runs.
   * No job changed state. The same 4 exploratory jobs hit their 120 s cap:
     `resultant_brown` and `fmpz_poly_resultant` on the two large Gluza33
     operands.
   * There were 0 cross-library mismatches.
   * Re-run time / campaign time had a geometric mean of **0.994** in run 1
     and **0.995** in run 2, with ranges of 0.85–1.04 and 0.85–1.05.
   * 5 jobs were new measurements, consistent to within 0.2% between the two
     runs. These are the `g33-disc2` interpolation image, which the campaign
     had not reached. Symbolica production (`integer_resultant`) took
     16.9 ms. FLINT took 10.0 ms with `fmpz_poly_resultant` (multimodular),
     17.6 ms with `fmpz_poly_resultant_euclidean` and 20.8 ms with
     `fmpz_mpoly`; Symbolica's multivariate Ducos took 31.7 ms.

## Upstream context

Symbolica issues found during this session were reported upstream:

* symbolica-dev/symbolica#75: integer left-shift overflow.
* symbolica-dev/symbolica#76: Lazard–Ducos panic on a vanishing subresultant.

The panic of #76 is why `integer_resultant` performs an exact shared-factor
test before calling Ducos.
