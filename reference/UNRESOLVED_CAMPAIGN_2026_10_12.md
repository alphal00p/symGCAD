# Unresolved originals: the October 12 local-recipe campaign

Date: 2026-10-12. Evidence (machine-local):
`artifacts/research/claude-2026-10-12/unresolved/`. It holds:

* the scripts `run_lanes.sh`, `run_all.sh` and `stop_campaign.sh`;
* `inputs/` and `inputs-skipped-round3/`;
* `v1-nopw-retain/<case>/`, with `input.toml`, `result.json`, `status.txt`,
  `time.txt`, `cpu.txt` and `binary.sha256`;
* `v1-pinning-bug-aborted/`.

**Outcome.** No case completed. The strict corpus count stays **303/331**. Every
number below is a measurement of incomplete runs. It is not a lower bound on
work, cells or coverage.

## Configuration

**Binary.** `artifacts/research/claude-2026-10-12/runs/bin/symgcad-integration`,
sha256 `c8f2c62f…1ccd09`. It is integration commit `4dbe440`, which already
contains the algebra fixes, sibling retention, the witness gate options,
`bilinear_general` and `kinematic_portfolio`. It does **not** contain the
projective sections of `9baf91c`.

**Inputs.** 24 of the 28 unresolved originals. Each recipe is the case's
earlier slow-suite recipe ([SLOW_SUITE.md](SLOW_SUITE.md),
`tests/slow_options.json`), changed as follows:

* `engine = "local"`;
* `local_positive_pair_witnesses = false`, because pair witnesses caused the
  measured thrash ([LOCAL_ENGINE_THRASH.md](LOCAL_ENGINE_THRASH.md));
* `local_stable_samples = true` and `local_retain_siblings = true`;
* Descartes witnesses and intervals and adjacent pairs kept as in the recipes;
* `cache_memory_mib = 512`, where the A317 recipe, for example, had 128;
* `[limits]`: `wall_time_secs = 3600`, `memory_mib = 5120`, `workers = 1`.

**Ordering and anchors:**

| Group | Ordering | Anchor |
|---|---|---|
| A317, A318, A377 | `given` | `x4` |
| A446 | `given` | `x5` |
| all other cases (20) | `mcts` (300 s search) | case-specific (`x0`, `x7` or `x8`) |

**Execution.**

* `run_lanes.sh <binary> v1-nopw-retain 12 320` runs 12 lanes. Lane `L` runs
  jobs `L`, `L+12`, … on CPU `320+L` only.
* Each job is `taskset -c <cpu> timeout 4000 <bin> solve`, followed by
  `verify` if the result is `complete_generic`.
* The 24 inputs ran in two rounds of 12.

### Caveat: shared warm checkpoint cache

The 20 `mcts` recipes do not set `checkpoint_dir`. MCTS then enables the
default `artifacts/checkpoints` directory, which is shared:

* by the concurrent runs of this campaign;
* with earlier campaigns on this machine.

These runs may therefore have reused completed operations from other runs. For
example, B_7_471 records 3 projection-prefix disk hits. Their times are warm,
shared-cache continuation measurements, not cold measurements.
The bench-infra runner (`artifacts/research/claude-2026-10-12/bench/`) avoids
this by injecting a fresh per-run `checkpoint_dir`. Do the same for any rerun.

## The aborted first run: CPU pinning bug

* **The bug.** The first launch (`run_all.sh … 12 320`, 09:39:51 UTC) gave
  *each* job the CPU range `320-331` through `taskset`. The solve supervisor
  pins its worker to the **first** CPU of the inherited mask, so all 12
  concurrent workers ran on CPU 320.
* **The stop.** The run was stopped at 10:19:51 UTC, after about 40 min:
  `solve_rc=143` for the 12 running A cases. It is preserved unchanged in
  `v1-pinning-bug-aborted/` and contains no usable measurement.
* **The relaunch.** At 10:20:08 UTC, `run_lanes.sh` relaunched with one CPU per
  concurrent solve.
* **The lesson** (also in the bench runner's README): never give one solve a
  CPU range. Give it exactly one CPU.

## Per-case outcomes (`v1-nopw-retain`)

Round 1 ran from 10:20:08 to 11:20:09 UTC. Round 2 started as each lane
finished round 1.

* `wall` and peak RSS come from GNU `time -v` around the supervisor and include
  the worker.
* Node, refinement and maximum-cut counts are the last
  `interrupted_local_progress` snapshot in `result.json`. It may be stale
  relative to the kill time.

### Round 1

| Case (symbolic) | Ordering, anchor | Status at 3,600 s | Speculative nodes | Refinements | Max refinement cuts | Peak RSS (KiB) |
|---|---|---|---:|---:|---:|---:|
| A_6_317 | given, x4 | incomplete (worker timeout) | 9,452 | 2,854 | 24 | 925,052 |
| A_6_318 | given, x4 | incomplete | 92,587 | 18,914 | 106 | 473,384 |
| A_6_377 | given, x4 | incomplete | 22,644 | 6,618 | 86 | 1,938,296 |
| A_6_414 | mcts, x8 | incomplete; 473 polynomials in the search inventory | 2,601 | 1,329 | 39 | 441,160 |
| A_7_381 | mcts, x8 | incomplete; no local progress recorded after the 300 s search | 0 | 0 | 0 | 1,307,680 |
| A_7_446 | given, x5 | incomplete | 12,555 | 6,773 | 93 | 1,968,136 |
| Ax12_6_317 | mcts, x8 | incomplete; 405 polynomials | 6,382 | 2,541 | 32 | 2,068,060 |
| Ax12_6_318 | mcts, x8 | incomplete; 4,642 polynomials, max degree 688 | 195,033 | 97,419 | 198 | 1,176,864 |
| Ax12_6_377 | mcts, x8 | incomplete; 46,372 polynomials, 1.33M terms | 25,412,537 | 8,850,895 | 237 | 4,181,508 |
| Ax12_6_414 | mcts, x8 | incomplete; 4,644 polynomials, max degree 688 | 190,836 | 95,380 | 198 | 1,167,760 |
| Ax12_7_381 | mcts, x8 | incomplete; no local progress recorded after the 300 s search | 0 | 0 | 0 | 288,824 |
| Ax12_7_446 | mcts, x8 | incomplete; 4,585 polynomials, max degree 688 | 184,690 | 110,153 | 198 | 1,180,364 |

### Round 2

Round 2 ran from about 11:20 to 12:20 UTC. All cases used `mcts` with a 300 s
search.

| Case (symbolic) | Anchor | Status at 3,600 s | Speculative nodes | Refinements | Max refinement cuts | Polynomials in inventory | Peak RSS (KiB) |
|---|---|---|---:|---:|---:|---:|---:|
| B_7_471 | x0 | incomplete; no local progress record. The statistics show 3 projection-prefix disk hits from the shared cache | n/a | n/a | n/a | n/a | 1,122,616 |
| Bx12_7_471 | x0 | incomplete; no local progress after the search | 0 | 0 | 0 | 0 | 715,192 |
| C_6_215 | x7 | incomplete; no local progress after the search | 0 | 0 | 0 | 0 | 263,616 |
| C_6_231 | x7 | incomplete | 227,780 | 93,476 | 3,692 | 4,902 | 1,117,300 |
| C_6_470 | x7 | incomplete | 718 | 280 | 34 | 243 | 488,704 |
| C_6_476 | x7 | incomplete | 29,910 | 11,111 | 43 | 1,147 | 767,808 |
| C_7_247 | x7 | incomplete; no local progress after the search | 0 | 0 | 0 | 0 | 827,752 |
| C_7_478 | x7 | incomplete | 4,207 | 2,461 | 37 | 241 | 733,312 |
| Cx12_6_215 | x7 | incomplete | 1,981 | 946 | 133 | 703 | 1,057,076 |
| Cx12_6_231 | x7 | incomplete | 2,442 | 1,258 | 218 | 736 | 796,632 |
| Cx12_6_476 | x7 | incomplete | 7,851 | 3,386 | 159 | 714 | 709,488 |
| Cx12_7_247 | x7 | incomplete | 30,180 | 18,141 | 160 | 1,277 | 939,676 |

All 24 cases that ran ended incomplete at the 3,600 s worker deadline. None
produced a result to verify.

### Not re-run (round 3 deliberately skipped)

The third round was deliberately skipped, by the orchestrating session's
decision when the session wound down. Its inputs were moved to
`inputs-skipped-round3/` while the lanes were running. When round 3 started at
12:20 UTC, each of its four solves therefore exited at once with `solve_rc=1`
(`No such file or directory`). Those four `v1-nopw-retain/<case>/` directories
contain no measurement.

| Case | Note |
|---|---|
| `finiteCx12_7_478_10_0__0_4_1_1_1_0_1_1_1/symbolic` | not re-run; status unchanged (unresolved) |
| `finiteD_7_251_12_0__6_1_0_1_1_1_1_1_0/symbolic` | not re-run; status unchanged |
| `finiteD_7_375_13_0__7_1_1_0_1_1_1_0_1/symbolic` | not re-run; status unchanged |
| `test_gluza_ex33/solve/3` (positive Gluza33) | not re-run with the local recipe. Complete under the opt-in projective contract ([PROJECTIVE_SECTIONS.md](PROJECTIVE_SECTIONS.md)); open under the strict contract |

## Interpretation

**What the counters show.**

* Retention and witness-free recipes did not change the outcome for any case
  that was run.
* The counters span four orders of magnitude, from 0 to 25M nodes.
* In the cases with many nodes, the search inventory reaches thousands of
  polynomials of degree up to 688. That is the same cut-set and algebra growth
  seen on the Gluza33 pieces.
* Six cases recorded no local progress after their 300 s order search: A381,
  Ax12_381, B471, Bx12_471, C215 and C247. Their remaining time presumably went
  into the first projection of the chosen order.
* C_6_231 reached 3,692 refinement cuts at one stack, more than any other cut
  count recorded in these notes.

**Why ordering is not the fix.** The parametric analysis in
[KINEMATIC_PORTFOLIO.md](KINEMATIC_PORTFOLIO.md) explains why ordering alone
does not resolve these 2-loop symbolic cases. Most kinematic-level factors are
spurious relative to the positive Landau set, and removing them needs exact
relevance certificates.

**Recommended next steps:**

* the relevance-aware local engine and chamber-wise orders listed in the
  HANDOFF's October 12 section;
* for any rerun, a per-run `checkpoint_dir` and exactly one CPU per solve.
