# Validation and benchmark record

The all-corpus goal remains unfinished. Results below identify the measured subset; fixed and symbolic ZZ domains are separate obligations.

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

## Remaining acceptance work

The importer preserves 177 collected tests, three disabled core tests, 277 fixture identifiers, and 93 extracted core calls (70 solves, 22 ordering calls, one projection call). Exact equivalence manifests currently contain 124 distinct fixed problems and 207 symbolic/core problems. Completing a solve campaign does not by itself reproduce the original assertions of ordering or projection tests.

The expanded native ZZ benchmarks, cold release reduction ablations, ordering-portfolio/MCTS comparisons, full-corpus completion ledger, and unresolved symbolic/hard cases remain in progress. Timeouts stay in the reports and never count as complete results.
