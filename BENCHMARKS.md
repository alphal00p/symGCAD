# Validation and benchmark record

The all-corpus goal remains unfinished. Results below identify the measured subset; fixed and symbolic ZZ domains are separate obligations.

**Certification update:** the historical measurements below used Symbolica's automatic resultant dispatcher. Source review found probabilistic CRT early termination on sufficiently large operands. Production construction and verification now use deterministic integer Ducos and invalidate the old algebra caches. Historical reports retain their compatibility evidence and measured timings; fresh deterministic verification is required before those artifacts count toward exact all-corpus completion.

## Deterministic completion checkpoint

The `9025e71` engine solved and independently verified **122 of 124 distinct fixed-kinematics problems** in a fresh completion campaign, including both A446 and Ax12 and the previously difficult B469/D373 cases. Each solve and verification had a 30-second limit, one CPU and 4 GiB; the campaign took 77.961 seconds. The two timeouts are A381 and its Ax12 crossing and remain unresolved. This is a completion run, not a three-repetition release timing benchmark. The exact corpus aliases, normalization identities, input hashes, result hashes and verification hashes are in `artifacts/completion-deterministic/fixed/results/ledger.json`, SHA-256 `5d7d52dbf936ee78d80cb1a7db1704c3888059cc4059adaf2762fee8f23ee400`.

The debug executable SHA-256 is `a0bd1554c08f16a625efd66ba1efa499519b06640db2bcc3898a584cda772cc1`. The separately frozen release executable for clean baseline replays is `artifacts/bin/symgcad-deterministic-9025e71`, SHA-256 `8b2376bc6f9963350e24e26a7d10bdedd6147da5714e73501deda99d311f5ea9`; its path-only manifest patch is stored beside it.

Certified local guard reduction also completed fixed A446 in a cold debug trial: 35 raw cells, 0.427-second solve, 0.163-second verification, 15 MiB peak RSS. Eight distinct certificates replaced 436 guard occurrences, with 21 new factorizations, 12 resultants and five discriminants. The unreduced earlier local trial timed out at 60 seconds. This exploratory result includes a supplied order and anchor, so it does not establish end-to-end ordering speedup. The artifact `artifacts/hard-campaigns/a446-local-domain-debug-01/initial-x2-0/result.json` has SHA-256 `18283a4edbc5ce950110e15d60cbd22c9547e9553f114a9389dfaea1248a7e97`.

Three fresh release measurements of captured degree-2-by-degree-8 operands gave integer-Ducos/CRT medians of 0.933/2.118 seconds for the homogeneous chart and 2.080/1.985 seconds for the unanchored operand. The speed difference depends on the operands; deterministic certification motivates the production choice. All computed polynomials agreed exactly in these trials. The profile summary SHA-256 is `daa0116abb5fd23d298f3674f7d1f5f2be63ef4c9b3223c1f3d5d7b17907d93d` (`artifacts/profile/resultant-release/summary.json`).

## Native baseline, fast core

The frozen M1 implementation (`3125469`) was compiled in release mode and compared with the accelerated native upstream revision recorded in `PROVENANCE.toml`. The reference first qualified all 48 solve invocations from the 39 active, unmarked core test functions. Three fresh-process measurements per engine used identical explicit orders, one CPU, and no merging. Every Rust result passed the independent exhaustive verifier, and all raw cell counts matched upstream.

| Measure | Rust/reference geometric mean |
|---|---:|
| Projection + lifting + merging | 0.1386517 |
| Fresh-process wall time | 0.2527771 |

The 1.25 target passes for this subset. No case exceeded 2× in the matched algorithm phases. Normalization and process initialization are reported separately; the phase ratio is not an end-to-end ratio. This does not yet establish parity over the entire reference-completing corpus.

Local artifacts (excluded from Git):

| Artifact | SHA-256 |
|---|---|
| `artifacts/bin/symgcad-baseline-3125469` | `52a7b27a80c697791e047b56b9ae765f747040322941b1a4e2ccce9992c0fbd1` |
| `artifacts/bench/core-fast-release/report.json` | `7c4b3d4c72ab99fa33467ae4c019d5dc9b3cf99fc99d734bc8d49819bffcfe58` |
| `artifacts/bench/core-fast-release/frozen-reference-set.json` | `18ed25cde59ace1d06d860a017bb19c50a98c44e9be52cc615d317d9a65fd6aa` |

The build metadata and path-only dependency-manifest adjustment are next to the immutable binary. Detailed phase times, peak RSS, raw counts, verification evidence, hardware and compiler metadata are retained in the report directory.

The fixed-kinematics active ZZ tier subsequently qualified and verified all 162 separate sign branches from 81 inputs. Its matched-phase geometric mean is 0.12308 and its fresh-process ratio is 0.19695; the largest matched-phase ratio is 0.72485. No branch was excluded or timed out. Report SHA-256: `f829d8fec7cce60f95655cf94a9735b8e7075aa87e32d71134ca000167dbdb8e` (`artifacts/bench/zz-fixed-completing-release/report.json`); frozen-set SHA-256: `0a157e753583900ce69fdafe9289466810006f4a0cae5574ea39b791477821a5`. The original symbolic tier is a separate ongoing benchmark.

A separate aggressive-merge fast-core run matched all 48 raw counts, all 48 merged counts, and passed its verifier. Its matched-phase ratio was 0.06757; report SHA-256 `03107122443bd39d0d8f293b8420d2bc6f73305e80d5e036c3df66306c00124d`. The 23 non-solve API comparisons produced 16 exact ordering matches and one exact projection match; six large ordering calls timed out in both engines. API report SHA-256 `e0068fbad60e8f5c8faf33c55141ed7b3ce50242e2a47dafd00727131fa59e50`. The disabled `failing_test_ex43` expects projection level sizes `[16,4,2,1]`, but both the actual native implementation and Rust produce `[16,2,1,1]`; the source assertion is not treated as an oracle.

## A446 and Ax12 at fixed kinematics

Both `finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1` and its Ax12 crossing have verified complete generic decompositions at `mt2=1,mz2=5/18,s=10,t=-3`. The optimized projection engine used exact nonvanishing certificates and homogeneous scale reconstruction, with anchor `x2` and lifting order `[x2,x7,x5,x4,x3,x1,x8]`. Each result contains 35 unmerged cells and covers both signs of F. The verifier checks the reduced projection obligations, certificate domain assumptions, exhaustive root gaps, and the exact inverse scale map.

These initial experiments used a debug executable with optimized algebra dependencies and some shared checkpoints. They establish correctness, not cold release performance or a search-overhead speedup. Their successful orders were found during an anchor/order campaign, so the solve times alone must not be presented as end-to-end campaign costs.

| Result file | SHA-256 |
|---|---|
| `artifacts/hard-campaigns/a446-debug-01/initial-x2-1/result.json` | `e7bd50117371a86d8686714461d51bb16adec16f72436ef4cfb0c0c110214126` |
| `artifacts/hard-campaigns/ax12-446-fixed-debug-02/initial-x2-0/result.json` | `6594efe4d910d9e1d32cbd714b67cace5c8536bf073cc7d6d2444743bff031b4` |

The corresponding `verification.json`, `record.json`, exact input, and campaign provenance are in each directory. Symbolic parameter-outer A446/Ax12 decompositions remain unresolved at this checkpoint.

The subsequent certified monotone engine reduced both fixed topologies to **eight verified raw cells: seven positive and one negative**. With `x2` as scale, the endpoint polynomial after peeling `x1,x3,x5,x7` is `x4*(x4+1)+x8*(x4^2-8*x4+1)`. The engine solves this base exactly and restores every peeled positive coordinate through its certified monotone quadratic root. These are full seven-variable decompositions, not restrictions of the solution to zero endpoint coordinates. The actual lifting order is `[x2,x4,x8,x7,x5,x3,x1]`.

Eight-cell artifact hashes:

| Result file | SHA-256 |
|---|---|
| `artifacts/hard-campaigns/a446-monotone-debug-01/initial-x2-0/result.json` | `68a30bf51d073516d5c54c47eb14617246db920d88e82ee34bbf23f6350a9f7b` |
| `artifacts/hard-campaigns/ax12-446-monotone-debug-01/initial-x2-0/result.json` | `dd2a45cf5b503bb626122fd75efd9d13dd7dae00102304bf13f54e2978bebae4` |

These remain exploratory debug measurements. Unrestricted symbolic trials exhausted both 60-second and 300-second limits; they are recorded as incomplete and are being pursued through further campaigns and algorithmic work.

The subsequent release ablation used immutable `b69ec24`, three fresh processes per successful configuration, one CPU, 8 GiB, and no persistent cache. Both topologies independently verified again. These timings include the public CLI, preprocessing, algebra and export, with the anchor and any explicit order supplied by the configuration; they do not include the earlier research campaign that found those choices.

| Configuration | A446 median seconds / cells | Ax12 median seconds / cells |
|---|---:|---:|
| Optimized, positivity and scale | 1.326 / 35 | 1.445 / 35 |
| Monotone, positivity and scale | 0.825 / 8 | 0.452 / 8 |
| Optimized, positivity, no scale | 2.409 / 35 | 1.032 / 35 |
| Optimized, scale, no positivity | timeout at 60 s | timeout at 60 s |

The no-positivity configurations were stopped after their first timeout and have no median. The ledger is `artifacts/bench/hard-fixed-release-ablation/ledger.json`, SHA-256 `7dd0edeffd85d02106fb0a145c46f5c19effbc769071ad11db8ff0d98985b31f`; the release executable hash is `df6ddd09c806cd1228db72b4cee0c60e85e95527e231d9fafe450f906c4b56ef`. Scale reduction is not a uniform speedup on these already-pruned inputs; positivity is essential to this tested projection configuration.

## Symbolic export regression

The first original-symbolic baseline campaign retained failures caused by unbuffered JSON output. For `finiteA_5_286/signs=-`, a 12-second syscall profile recorded 392,791 writes for 812,205 bytes. The unchanged M1 algebra, writing to local temporary storage, completed 3,308 cells in 1.354 s (projection 0.0353 s, lifting 1.3172 s), versus native matched phases of 1.8149 s. The independent M1 verifier checked 24,382 stacks and 75,721 rejections successfully. This establishes an export defect, not a slow resultant on that case.

Production export now uses a buffered writer with explicit flush. A separately frozen M1 build changes only this output path and dependency locations; its SHA-256 is `2db92b0b6c4d7259a98f622611aa45bad2d1bbdea4ba97c91874a0374a9a6f47`. The exact build patch is `artifacts/bin/symgcad-baseline-buffered-3125469-build.patch` (SHA-256 `9753ed91129079a699a992ab1fe5e9a3bf7c045b53d1f01ada6be6041bbd0a32`). The full original frozen denominator is being rerun, and the failed campaign remains preserved. Worker control/progress files now use the system temporary directory; durable checkpoints and final exports retain their requested paths.

The first buffered replay exposed a second harness issue: scanning all host PIDs every 20 ms consumed roughly half of a pinned CPU. The supervisor now follows the worker's own process tree and records `wait4` user/system CPU time. Historical pinned-supervisor wall times and deadline failures remain in their original reports; clean paired replays use the corrected supervisor. Profiling the 64,537-cell `finiteA_5_376/signs=-` case also identified rational root gcds and repeated substitution as real lifting costs. Exact linear isolation now bypasses unnecessary gcd/factorization, and lifting discards satisfied constant constraints while preserving original rejection IDs.

## Remaining acceptance work

The importer preserves 177 collected tests, three disabled core tests, 277 fixture identifiers, and 93 extracted core calls (70 solves, 22 ordering calls, one projection call). Exact equivalence manifests currently contain 124 distinct fixed problems and 207 symbolic/core problems. Completing a solve campaign does not by itself reproduce the original assertions of ordering or projection tests.

The expanded native ZZ benchmarks, cold release reduction ablations, ordering-portfolio/MCTS comparisons, full-corpus completion ledger, and unresolved symbolic/hard cases remain in progress. Timeouts stay in the reports and never count as complete results.
