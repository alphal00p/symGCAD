# Slow original aggressive-merge assertions

The twelve active slow solve/merge assertions beyond the fast core and disabled
examples were replayed with the unchanged native reference and frozen Rust
release `dda5980`. Every case completed in both engines in an initial bounded
trial, then in three fresh paired repetitions. All raw counts agree. Eight
merged counts agree; four differ under the two merge policies. Every case has
an independently verified Rust proof. This is not a merged-parity pass.

## Scope and exact counts

The source is `DO_NOT_PUSH_REFERENCES/gcad/tests/test_gcad.py`, SHA-256
`c430f10b61505a44558ec6103f1ddb0c1d393ebc8134b89e1c35708a36a11bcf`.
Every native raw and merged count below agrees with its active source assertion
in all three repetitions. Every Rust raw count also agrees in all repetitions.

| Original test / solve | Raw cells | Native merged | Rust merged | Disposition |
| --- | ---: | ---: | ---: | --- |
| massless planar elliptic / 2 | 131072 | 103 | 97 | merge-policy difference; proof verified |
| massless planar elliptic / 3 | 138678 | 104 | 103 | merge-policy difference; proof verified |
| physical elliptic / 3 | 135 | 3 | 3 | counts match; proof verified |
| physical elliptic / 4 | 601 | 11 | 11 | counts match; proof verified |
| E5 / 2, below threshold, negative | 0 | 0 | 0 | counts match; proof verified |
| E5 / 3, below threshold, positive | 25160 | 1 | 1 | counts match; proof verified |
| E5 / 4, above threshold, negative | 31457 | 45 | 60 | merge-policy difference; proof verified |
| E5 / 5, above threshold, positive | 105322 | 99 | 118 | merge-policy difference; proof verified |
| pentabox B / 2 | 1845 | 7 | 7 | counts match; proof verified |
| pentabox B / 3 | 881 | 3 | 3 | counts match; proof verified |
| pentabox D / 2 | 139 | 7 | 7 | counts match; proof verified |
| pentabox D / 3 | 70 | 4 | 4 | counts match; proof verified |

The four differing merged covers are not claimed to equal the native covers.
The original native queue and historical Rust pair-first policy are different;
[the separate merger audit](UPSTREAM_MERGE_AUDIT.md) demonstrates policy-only
count differences on identical raw inventories. It also records actual native
defects and the three disabled examples' stale assertions. Those dispositions
must not be confused with the twelve valid native assertions above.

The benchmark deliberately leaves `verified: null` after a merged-count
difference. Four separate bounded verifier invocations therefore replayed the
initial saved massless and above-threshold E5 proofs. They verify all raw
branches and reconstruct the declared Rust aggressive cover, retaining its
exceptional seams. Their wall times were 33.39, 40.43, 23.39 and 32.76 seconds;
all returned `verified: true`. The eight matching cases were verified by the
ordinary repeated benchmark. The consolidated audit records the exact proof
and verification hashes; supplemental verification does not rewrite the
benchmark's false parity result.

## Frozen inputs, limits and timing

Inputs preserve all original constraints, variables, groups and parameters.
Only the already-qualified original order, aggressive merge, and resource
settings were fixed explicitly. The orders are:

- Massless: `[x4,x3,x2,x6,x5,x1,x0]`.
- Physical: `[x6,x3,x5,x1,x0,x4,x2]`.
- E5: `[p1s,x4,x5,x3,x2,x1,x0]`.
- Pentabox B: `[x7,x4,x5,x6,x3,x2,x1,x0]`.
- Pentabox D: `[x7,x6,x5,x4,x3,x2,x1,x0]`.

The Rust binary is `artifacts/bin/symgcad-indexed-merge-dda5980`, SHA-256
`f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`.
Each campaign ran sequentially on CPU 7, one worker, with a 16 GiB memory cap,
120-second per-worker limits, and a one-hour total bound. The initial campaign
took 335.30 seconds; the three-pair campaign took 674.38 seconds. Neither
exhausted its budget and no worker timed out. Native qualification was frozen
before Rust timings in each campaign. All initial workers completed in under
35 seconds, so all twelve were eligible for the fresh three-pair replay.

The descriptive geometric mean of Rust/native projection+lifting+merging
medians is 0.52346; the fresh-process wall-time ratio is 0.26825. The largest
kernel ratio is 1.78246 for empty E5 below threshold. These compare the declared
engine policies, including their differing merge schedules; they are not used
to replace or extend the original no-merge M2 parity gate. `parity_passed` stays
false because four merged counts differ. The mathematical completion count
stays 302/331.

## Commented symbolic expectations

These source comments are retained as dispositions, not promoted to active
assertions:

- Symbolic massless: solve and merge calls and counts 138678/104 are commented
  out at lines 308–311. Its actual ordering call remains an unresolved API
  workload; both engines timed out at the recorded 60-second limit.
- Symbolic physical: the selected-order solve is active, but counts and merge
  call are commented out at lines 355–357. Its original baseline native solve
  qualification timed out at 60 seconds. A later optimized original problem
  has a separate verified completion, which does not reproduce this baseline
  ordering/merge assertion.
- Symbolic pentabox B and D: solves and merges are commented out at lines
  507–512 and 546–551. The copied proposed order contains physical-elliptic
  parameters `t,pp4,s`, omits the actual kinematics, and is not a permutation of
  either problem's variables. Their active ordering calls remain unresolved
  after both engines timed out at 60 seconds. No invalid order was benchmarked.

## Artifacts and reproduction

All paths below are under `artifacts/bench/slow-aggressive-dda5980/`:

| Artifact | SHA-256 |
| --- | --- |
| `inputs.json` | `3c10ac03c4ff4dd99423d4908628255b3655ebc84c9da842479c9c422adea582` |
| `initial.toml` | `4f6d395116747366bb977edb35503a9673bde85d9832c3eb562dcbceb1ca8cc0` |
| `initial/report.json` | `0ca14f42862d2a4dd31cf80be466223386e67e466d4c417c8703e8d5377c3dba` |
| `repeats3.toml` | `ca77a02ca73060c8ee4cdd026b0bbf588972562ce5fa0987e8a79537c5f13b8d` |
| `repeats3/report.json` | `cd201cebdde92b7ae21f8646d75185ce09685d32826c920ead3df7fdfe762ce5` |
| `audit.json` | `78cb5879e85dbf36d9a0a50162d9cefb2c0330d4b1085a35f63289128fab8d3b` |

`prepare.sh` records the deterministic input edits and hashes;
`assertions.json` records the exact source expectations. `verify-mismatches.sh`
runs the four bounded supplemental checks. `aggregate.sh` rebuilds `audit.json`
from the immutable reports and streaming proof hashes. The audit separately
records all twelve verified proofs, native assertion agreement, raw parity,
and false merged parity. The initial invalid TOML-manifest attempt failed
before any worker ran and is preserved in `config-parse-error.log`.

```sh
nix develop --command taskset -c 7 \
  artifacts/bin/symgcad-indexed-merge-dda5980 bench \
  artifacts/bench/slow-aggressive-dda5980/repeats3.toml
nix develop --command bash \
  artifacts/bench/slow-aggressive-dda5980/verify-mismatches.sh
bash artifacts/bench/slow-aggressive-dda5980/aggregate.sh
```

For a fresh run, first copy the TOML to a new campaign location and change its
output directory, preserving the recorded evidence.
