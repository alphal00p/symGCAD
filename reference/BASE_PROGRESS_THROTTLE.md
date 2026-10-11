# Bound progress I/O on optional base-support fallbacks

The optional base-univariate support path was rewriting the progress JSON on
each ordinary-factor fallback. Throttling these diagnostic writes substantially
reduces system CPU in a matched short C470 experiment. The experiment remains
incomplete and establishes no time-to-solve improvement or new solved case.

## Diagnosis and correction

`Builder::guard_basis` calls `publish_base_progress` for declined support
proposals. The old helper bypassed the ordinary half-second progress throttle:
each call serialized a snapshot, wrote a temporary file, then renamed it. The
earlier 600-second support-basis run recorded 1,163,056 such static declines and
167.335 seconds of system CPU. Algebra phase timers do not include this I/O.

The correction preserves fresh in-memory operation-capture context but bounds
routine base-progress writes to one per half second. A separate clock prevents
these writes from postponing ordinary observer updates. The first diagnostic
snapshot is immediate, even before half a second has elapsed, and diagnostic
stops force a final best-effort snapshot. Failed diagnostic writes retain their
old error semantics. Mandatory ordinary snapshots and changed-prefix operation
captures remain; the limit does not apply to every progress-file write.

No projection, factorization, guard, root-isolation, or verification obligation
changes. The focused base-univariate library suite passes 29 tests, with one
capture-dependent slow test ignored. It includes immediate-decline publication,
capture refresh, and failed-diagnostic routing. Broader integration validation
is tracked separately by the main development run.

## Matched measurements

Each fresh worker used CPU 10 and a 4 GiB worker memory limit, the same original
symbolic C470 problem, the same given order and homogeneous anchor, and an
independent copy of **58,511 completed ordinary operations** with no initial
support proposals. Input auditing checks all mathematical and solver fields;
the two untraced inputs differ only in their checkpoint directory. Cache-copy
and audit costs are outside these diagnostic worker timings. These are warm
cache measurements from one run per configuration, using frozen debug builds.

| Untraced, 60-second worker limit | Before | Corrected |
|---|---:|---:|
| Wall seconds | 60.032588 | 60.038311 |
| User CPU seconds | 41.539190 | 57.861461 |
| System CPU seconds | 17.504901 | 1.282241 |
| Peak RSS, KiB | 59,888 | 65,652 |
| Recorded speculative nodes | 85,684 | 252,974 |
| Recorded refinements | 14,325 | 50,264 |
| Static support declines | 127,668 | 358,890 |
| Retained support sources | 89 | 100 |
| Outcome | timeout | timeout |

System CPU decreased by **92.7%**. Node and refinement counters show increased
search activity, not certified coverage or a solver-speedup ratio. Both runs
remain incomplete: neither exports a verified decomposition. Both report zero
post-entry support failures, resource stops, rejected proposals, or invalid
operation checkpoints. The support registry did not fill in these short runs;
the longer run's 4,096-source limit remains a possible later obstruction.

Separate matched 30-second `strace 7.1` probes identify the filesystem effect:

| Completed traced syscalls | Before | Corrected |
|---|---:|---:|
| Progress temporary writes and successful renames | 38,967 | 3,410 |
| Progress bytes written | 188,762,750 | 16,662,484 |
| All successful renames | 39,039 | 3,517 |
| New support-proposal renames | 72 | 98 |

An exact parser rejoins per-process unfinished/resumed records. The corrected
probe performs 91.2% fewer progress writes in this fixed time. Work reached
differs, and tracer overhead is substantial; traced timings are not benchmark
times. Exit 124 is the deliberate inner profiling timeout, not an algebra
failure. The first draft trace under `new-traced/` is preserved as preliminary
and excluded from these final-patch comparisons because that draft could delay
the first diagnostic snapshot.

The active bottleneck is again exact search and refinement. A separate
cache-first support policy has not been implemented: admitted support sources
currently enter the support registry before ordinary-factor lookup. A future
policy would need a lookup-only API and existing exact cache validation; a
native-factor call on a cache miss would defeat the intended fallback.

## Reproduction and bindings

The original fixture is
`artifacts/corpus/symbolic/0211_finiteC_6_470_10_0__0_5_1_0_1_0_1_1_1_symbolic.toml`.
It retains `s > 4`, unrestricted `t`, `mt2=1`, `mz2=5/18`, and positive Feynman
parameters. The given order is `s,t,x1,x2,x4,x6,x8,x7`, with homogeneous anchor
`x1`. Both workers enable the existing affine factorization, support basis,
Descartes witnesses/intervals, positive-pair witnesses, and adjacent pairs.
All other flags and resource limits are stored in generated `input.json` files.

Evidence is under ignored `artifacts/research/c470-progress-io-profile-01/`
(old trace) and `c470-progress-io-profile-02/` (old/new untraced controls,
corrected `final-traced/`, input/cache audit and raw traces). Frozen build/test
evidence is under `artifacts/research/base-progress-throttle-01/`.

SHA-256 bindings:

- Old worker: `315cc6393b510e9432f34a81ba55e4b68d950f67f346665fe261474443584b33`.
- Corrected worker: `bf90956cea00c33b78b8be2e0d6a9e53ab5bca068e6d331c935b1bc1c0bec2e7`.
- Corrected `local_projection.rs` at freeze:
  `82a542d4f757be231a03e2b49e7451dde422e86f9a57a8f486c48752e5e02e21`.
- Unchanged frozen Rust supervisor:
  `0372b619858c8b464580a6930fc79258631b9518f1db49cf689dd4e59cbd6ad2`.
- Shared inherited cache manifest:
  `32bf9e223066a469a044c896ac1cf08a419f8fcb22db795322c10705d4d925d2`.
- Old raw trace:
  `9bde7c85b703ba1bd803a7cc5d88d5613713ee6a1c909371cd4abb30194f4d7b`.
- Corrected raw trace:
  `8767a23a20bbbe87e2cddd5d12b701175256ae6be0face584eda8e52c1541b7c`.

The frozen source precedes the independently developed interval aggregate-budget
extension. The paired comparison changes diagnostic throttling only; it must not
be attributed to that later algebra-policy extension.

## Original 600-second continuation

A subsequent original-problem run used the same corrected frozen worker, CPU 10,
8 GiB, and all previous mathematical options and limits. It inherited a fresh,
hash-checked copy of continuation-03's 58,749 ordinary operations and 4,096
support proposals. This newer warm state makes it a continuation, not a matched
timing comparison with the preceding controls or earlier campaigns.

It timed out after **600.050078 seconds**, with 575.660552 user seconds,
15.106857 system seconds, and **272,260 KiB** peak RSS. No result, cell count, or
independent verification was produced. The final snapshot records 2,138,944
speculative nodes and 470,081 refinements; these are activity, not coverage.
Factorization consumed 68.614 seconds, root isolation 44.234 seconds, pair
separation 56.360 seconds, and resultants 11.400 seconds. The final captured
operation is a completed degree-eight factorization taking 0.000771 seconds;
it does not identify a pending native-algebra stall.

All 4,096 inherited support proposals replayed, with 0.126134 seconds of exact
reconstruction and no rejected proposals, post-entry failures, or invalid
operation checkpoints. The registry reached its unchanged source limit and
recorded 25,966 source-count declines. The ordinary cache added 279 completed
operations: 141 factorizations, 125 resultants and 13 discriminants. The final
inventory contains **59,028 ordinary operations and 4,096 support proposals**;
every inherited file is present with unchanged hash.

Remaining pressure is visible in the conditional caches: 939,262 subtree
evictions and 447,136 learned-cut evictions. Their 32,768-node/4,096-entry limits
were full while charged memory was only about 11.6/3.6 MB. This suggests testing
the existing generic 256/64 MiB cache options, which scale these count limits
without changing per-entry proof obligations. Evictions alone do not prove
recomputation, and that configuration has not been tested in this continuation.

Evidence: `artifacts/research/c470-shear-continuation-04/`, including original
input equality, complete inherited/final inventories, phase snapshots and the
separate cache-budget recommendation. The result record SHA-256 is
`d40c224f1c902bd157eeea5fbda0e73fd6905c8252d4f698dfe9702ee4d0c9ce`;
the final cache/input audit SHA-256 is
`7b271c6efca687e48eee0a8ffd14d05814e5c43c16201f1c1965769222ed5678`.
The combined tracked regression suite subsequently passed **646 tests, zero
failures, two slow tests ignored**, across 49 targets (59.23 seconds of test
bodies). That suite also includes the separate default-compatible interval
aggregate-budget change; it is correctness evidence, not part of these timings.
