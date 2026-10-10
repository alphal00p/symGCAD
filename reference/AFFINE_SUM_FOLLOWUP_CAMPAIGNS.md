# Follow-up campaigns after exact square and sum reductions

These four bounded trials all timed out and add **zero** completed original
problems. Each used one CPU, an 8-GiB process limit and 300 seconds. They are
exploratory campaigns, not matched performance ablations.

| Original problem / configuration | Worker | Wall seconds | Peak RSS KiB | Last nodes | Last refinements |
| --- | --- | ---: | ---: | ---: | ---: |
| Fixed A381, global Descartes witnesses | b81 release | 300.039 | 101,044 | 1,993,159 | 298,738 |
| Fixed A381, interval and pair witnesses | b81 release | 300.061 | 215,172 | 1,027,319 | 25,698 |
| Fixed Ax381, interval and pair witnesses | b81 release | 300.179 | 1,606,244 | 550,824 | 58,038 |
| Symbolic A446, interval and pair witnesses | b813 debug | 300.071 | 514,964 | 103,588 | 15,432 |

All use the local engine, adjacent pairs, one pair cancellation, no sibling
batching, no stable sample selection and homogeneous anchor `x5`. The fixed
order is `x5,x8,x4,x6,x3,x0,x2`; the symbolic order is
`s,t,x5,x1,x2,x3,x4,x7,x8`. Global Descartes witnesses are on in all four.
The two A381 release trials shared an existing atomic operation checkpoint
store while running concurrently; Ax381 started cold. Their timings therefore
must not be used to isolate the interval option's cost or benefit.

The frozen b81 release executable SHA256 is
`34daa0ef54b7b870f95a96f6a6968eecc3caf495d39c39fbb30a4470672dc423`.
The frozen b813 debug executable SHA256 is
`e24b80cd8a8d69bb361b7ed08737393e8fcd8161eefa69d0bc3e1b3b44bdb6e7`.
The latter passed 341 library/integration tests before these campaigns.

Symbolic A446 reused its earlier exact operation checkpoints. The new sum
factor preconditioner gets past the previously captured 41-term factorization;
the last published snapshot now occurs at 298.196 seconds. Reported phase
costs are 98.320 seconds for factor handling, 38.703 for domain certificates,
38.911 for resultants and 26.809 for pair separation. The final captured
operation completed, so it does not identify a stalled algebra call. More
speculative traversal is not a coverage certificate or an end-to-end speedup.

Evidence lives under `artifacts/hard-campaigns/`, with the following campaign
directories and SHA256 values for `initial-x5-0/record.json`:

| Directory | Record SHA256 |
| --- | --- |
| `a381-global-witness-release-b81` | `3e59c30826d43595ae525e376eeb3e7544789e25f298923f11c5617d3bfec6b3` |
| `a381-interval-release-b81` | `ad809a5eafa0a2e52b7572db97baf7e694b1c5a27fb9a2486ca12fc081c4af91` |
| `ax381-interval-release-b81` | `67a34432a34bd90a7a5fd38945c59cad5119513c6a450c4fa6cf9293c144f434` |
| `a446-affine-sum-debug-01` | `a70f2d0f59980a74a7b29c086ec4f9f19405174455bfa7f2048b5e71aade0ea7` |

`affine-sum-followup-summary.json` retains compact process/phase metadata.
Every directory also retains the actual normalized worker input, provenance,
progress, checkpoint reference and failure status.

## Quadratic/integer release retry

The unchanged unrestricted symbolic A446 input was retried with release
`3acd6aa`, SHA256
`c22220744258a28e487543d9796e5e113d621f6f05cc32c4f68a7cbe846b60b8`.
The original order and anchor above are retained, with interval/pair witnesses,
adjacency, one pair cancellation and no sibling batching. Diagnostic operation
captures are disabled. The exact pre-existing A446 checkpoint directory is
reused; subtree and learned-cut memory budgets are 128 and 32 MiB. This is an
exploratory warm-cache retry, not an isolated comparison with the debug run.

The CPU-3 worker times out at 300.074 seconds, with 292.214 user CPU seconds,
3.582 system CPU seconds and peak RSS 920,760 KiB, under its 8-GiB limit.
No result or proof is produced. Its last progress snapshot, at 97.017 seconds,
records 78,836 nodes and 16,544 refinements at level 4 and prefix
`[17/4,-22,1/2,1]`; the published phase is `root_isolation`. Because snapshots
can precede a subsequent algebra operation, this does not identify the exact
operation responsible for the remaining time. No operand capture was enabled.
The recorded phase totals at that snapshot are 53.445 seconds in resultants,
14.034 in factor handling and 4.193 in domain certificates. The retry adds no
completed original problem.

Artifacts: `artifacts/hard-campaigns/a446-quadratic-integer-release-01/`.
The `initial-x5-0/record.json` SHA256 is
`d2d3b51a2c281ec40ddafce20bc30d8470aba96cc59cb9e410e829eaa8c154a8`.

## Longer fixed A381 and remaining symbolic screening

The same frozen `3acd6aa` release also failed to complete fixed A381 within a
30-minute solve limit. It retained the `x5` anchor and order above, all three
witness modes, adjacency, one pair cancellation, no sibling batching, and the
existing exact operation checkpoint directory. It used CPU 4, an 8-GiB memory
limit, 128/32-MiB subtree/learned-cut budgets, and no operand capture. The worker
timed out at **1800.066 seconds**, with 1756.873 user and 7.965 system CPU
seconds and peak RSS **541,164 KiB**. The last snapshot at 1799.560 seconds
contains 17,330,200 speculative nodes, 2,352,212 refinements, and 5,129,504 cut
additions, at level 2 and prefix `[1/7,2731/4096]`. These are traversal counts,
not a completed cell inventory. The bounded campaign preserved its operation
checkpoints but produced no proof. This failure motivates the separately
certified [conditional quadratic reductions](CONDITIONAL_QUADRATIC_PEEL.md).

The record is
`artifacts/hard-campaigns/a381-quadratic-integer-long-01/initial-x5-0/record.json`,
SHA256 `437792fa32a192cffb3e13919bdb00cf236912ea360a9f3a06655c5c75218fea`.

A separate screen retried all **26 unresolved symbolic ZZ originals other
than A446** with the same release, their unchanged mathematical inputs and
group/order policies, all three witness modes, adjacency, one pair
cancellation, and no sibling batching. Every case timed out at 60 seconds;
the full campaign took **1560.780 seconds**, on CPU 3 with one worker and a
4-GiB limit. No additional original was solved or verified. The initial cache
was a private copy of 32,161 exact checkpoint files from the previous symbolic
screen; its manifest SHA256 is
`8ec6100a97b2d9fe197041e7355637a1042009c1ceafbae812b0cee2153c278e`.
This is a completion retry, not a matched timing comparison.

The inputs, preparation metadata, seed manifest, individual outcomes and
progress records remain under `artifacts/symbolic-quadratic-integer-release-01/`.
Its `results/ledger.json` SHA256 is
`d099bf109c8568ff7c36c1afa0a013d2ba845079f3de774ef4cb64df8bc81f5a`.
