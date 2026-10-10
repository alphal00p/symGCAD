# Local projection follow-up pilots

Audited 2026-10-10. These three cold, single-worker development-build trials
all reached their 300-second limit without exporting a result. No verification
ran and no additional original problem is complete. They are exploratory
completion attempts, not isolated performance comparisons.

All used the immutable solver SHA-256
`29e2824b665f8631a86d3ef1873bab1831f92f68461bb56a181a08cf9b41c098`,
qualified by the 503-test fast suite and the independently verified deferred
projection smoke case described in [the implementation report](LOCAL_PROJECTION_LIVENESS.md).
Each allowed 8 GiB, one CPU, and a separate 600-second verification allowance.
The prepared TOML mathematical prefix, including parameters and variable groups,
was byte-identical to its original corpus input; only solver/resource settings
changed. The saved worker input JSON independently identifies those settings.

## Symbolic A446: larger interval stock and deferred projection

Original: `0093_finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml`.
Both retain `s > 4`, unrestricted `t`, all original positive Feynman coordinates,
and the original split. The fixed legal order is
`[t,s,x5,x1,x2,x3,x4,x7,x8]`, with the complete Feynman block homogeneous and
anchor `x5`. The reduced order is `[t,s,x1,x2,x3,x4,x7,x8]`.

Both use local projection, adjacency, individual Descartes/interval witnesses,
positive pair witnesses, one pair cancellation, unbatched discovery, positivity,
and limits of 20 million refinements / 200 million nodes. Stock-eight uses
ordinary projection and canonical samples; the deferred variant uses stock two
and stable samples. Both checkpoints are cold. Three settings therefore differ.

| Recorded quantity | Stock eight (CPU 2) | Deferred + stable, stock two (CPU 3) |
|---|---:|---:|
| Outer wall time | 300.014940 s | 300.022015 s |
| Peak process-tree RSS | 475,484 KiB | 275,296 KiB |
| Last progress elapsed | 298.285255 s | 285.154740 s |
| Speculative nodes at snapshot | 97,130 | 207,557 |
| Refinements at snapshot | 7,187 | 9,212 |
| Factor cache misses | 20,354 | 3,563 |
| Resultant / discriminant misses | 97 / 240 | 69 / 217 |
| Interval templates / reuses | 102 / 802 | 41 / 18 |
| Interval-stock exhaustion fallbacks | 0 | 0 |
| Deferred stacks / completed frontiers | 0 / 0 | 86,935 / 86,931 |
| Compaction rounds / removed cuts / discarded children | 0 / 0 / 0 | 1 / 4 / 10 |
| Deferred scheduling fallbacks | 0 | 1 |

The stock-eight snapshot is at level four, prefix `[-9,5,1,1/30720]`;
the deferred snapshot is at level three, prefix `[-9,5,1]`. These are different
partial traversals at different times. Node/refinement counts do not measure
fraction of the original problem completed. The one recorded compaction proves
the implementation exercised that path; it does not establish that the earlier
captured octic was dispensable or removed.

The final **published** captures are both completed factor operations:
0.011608173 s and 0.001474866 s respectively. They were updated at
08:35:39.341 and 08:35:39.807 UTC, near the deadline. The deferred directory also
contains an unrenamed atomic-write temporary with an incomplete factor-start
record. `ProjectionCache::factor_basis` publishes that record before entering
the backend; publication writes/fsyncs a temporary, renames it, and fsyncs the
directory. The unrenamed file therefore does **not** prove that factorization
started. No terminal long algebra operation can be identified from these files.

Inclusive factor-phase totals at the two snapshots are 112.769 s and 58.901 s;
root-isolation totals are 11.226 s and 37.407 s. These include the instrumented
pipeline's cache/bookkeeping/capture costs and are not standalone kernel
measurements. The differing options and traversal histories rule out a causal
speedup claim. Neither snapshot records exhaustion of the interval stock.

## Symbolic A318: opt-in binary pair resultants

Original: `0079_finiteA_6_318_7_0__0_2_1_1_1_1_0_0_1_symbolic.toml`.
The fixed order is `[s,t,x4,x1,x2,x3,x5,x8]`, with the full Feynman block and
anchor `x4`. This CPU-2 trial enables `binary_resultants`, local projection,
input/domain positivity, adjacency and the individual/pair witness portfolios.
It timed out after 300.006217 s, with peak RSS 273,736 KiB.

The recovered 273.787232-second snapshot records 561,402 speculative nodes,
65,057 refinements, level two (`x1`) and prefix `[s,t]=[13/3,-18]`.
It records **zero segmented resultants**, 1,342 ordinary resultant misses and
413 discriminant misses. Thus this pilot provides no observation of successful
segmentation use, despite the enabled option.

Unlike the A446 pilots, the final published capture is incomplete: an `x1`
discriminant, published at 08:45:34.459 UTC, approximately 26 seconds before
the outer timeout. Its canonical operand has 12,879 terms, degrees
`[s,t,x1]=[39,38,20]`, and 1,349,127 characters. These are static text
measurements, not a new algebra run. The capture identifies a pending
discriminant at the tail, not the cost of the full 300-second traversal.
Discriminants are outside the opt-in pair-only dispatch, and this operand also
exceeds the current 4,096-term segmentation input cap. No override was tried.

## Recovery and provenance

The public CLI's temporary paths follow `std::env::temp_dir()`. Here Nix set
distinct temporary directories: stock-eight used
`/tmp/nix-shell.FHEr9g/symgcad-workers/535133-*`, deferred used
`/tmp/nix-shell.xEnQYv/symgcad-workers/535134-*`, and A318 used
`/tmp/nix-shell.9CHjis/symgcad-workers/763245-*`. Their algebra-worker PIDs were
535142, 535143 and 763250. Looking only under `/tmp/symgcad-workers` or only for
the child PID misses these files. All recovered inputs/progress and captures
were copied with timestamps into the audit directories below before analysis.

The outer campaign and inner worker both have a 300-second limit. The outer
timeout prevented a normal public-CLI result export in these runs. No result
means no independently checked partial CAD. Outer `wait4` user/system fields
measure the public process and must not be treated as the full algebra-worker
CPU consumption.

| Audit binding | SHA-256 |
|---|---|
| A446 stock-eight prepared input | `bc491c1f2fca92f2bb0c9cb9d02b0f3de3993d1fdb6b0229b91c2d5bbe403656` |
| A446 deferred prepared input | `ef5b57ffe80a381ba247db98bcbf36140c2dbb258ab7ab57c03d1562ca7cfbd1` |
| A446 stock-eight progress | `25d21e12ad2cb74e626dfaafbba6b86377809228c31d8ce61b2f2c982d2fdcb5` |
| A446 deferred progress | `a7371bc751c691cef1a71c1a4ad10ee396af7f868d2e0fcb2eeee74888433a15` |
| A318 prepared input | `99a7ccf00afd9d2db0dfaccf507a277b79075c7794d65dc5906ad259a0cd6ec6` |
| A318 progress | `9f1832afb53be6dda4eee03087646285efd53e67086dfb94ef514b906e695c4c` |
| A318 pending-discriminant capture | `821a97374934ce9847522b7d2bf0dc44c5579305cc633d5cb6a0c232ea143084` |

The ignored local artifact directories are
`artifacts/research/local-projection-followup-02/audit` and
`artifacts/research/segmentation-hour-followup-01/audit`. Each contains a
`report.json`, complete copied ledger, original/prepared/worker inputs, final
progress, captures, and a `files.json` manifest with byte counts, timestamps and
hashes. Published operand hashes were independently recomputed from their exact
JSON arrays. No new worker, polynomial computation or verification was launched
for this audit; no benchmark table or PDF was changed.
