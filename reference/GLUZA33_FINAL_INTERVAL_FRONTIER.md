# Gluza33: the next interval-witness resource gate

The later [source-specific stock trace](GLUZA33_WITNESS_STOCK_TRACE.md) now
identifies the actual quarter-prefix fallback: both permanent interval slots
are occupied and their covers fail, so fresh discovery is skipped. The
standalone successes and earlier continuations below remain separate evidence.

The isolated Wang continuation completed the previously blocking factor but
stopped again at discriminant `4fa9b805…`, input hash `eba809f2…`. It is the
same 19,942-term degree-24 polynomial diagnosed earlier, with full variable
map `[x2,x1,x7,x4,x6,x3,x5]`. At the newly recorded prefix `[2,2/3]`, an exact
interval certificate exists, but the current discovery exceeds the fixed
65,536-term total guard budget. More template slots alone cannot admit this
freshly discovered certificate.

The final capture SHA-256 is
`5c15531346bf5a4a4e247db0f23fdbb38031d1bdf0485f7e08e7af7b1c2ad3d6`.
The previous `[2,1/2]` diagnostic needed only two intervals and 49,870 guard
terms. At `[2,2/3]`, the unchanged frozen helper obtains four intervals:
`(0,1/4)`, `(1/4,1/2)`, `(1/2,1)`, `(1,infinity)`, with variations `[1,0,0,1]`.
It independently isolates two distinct positive roots, while the global
coefficient sequence has four variations. Every guard is nonzero at the
prefix. The four-piece proof has 97 distinct guards and 109,726 terms;
individual guard size, coefficient height, slots and all other non-source
limits pass. Construction takes 2.041523 s and 49,532 KiB peak RSS.

`WitnessStock::attempt` first replays its permanently stored covers, then
checks its lifetime quota before discovering another cover. The campaign
never serialized this polynomial's stock history. Its 31 aggregate budget
fallbacks therefore cannot establish which rejection happened first for this
polynomial. The fixed total-terms gate is nevertheless independently observed
on a fresh discovery. The frozen solver refreshes the prefix context before
large discriminants, invalidating that context after periodic progress writes;
this strengthens the association with `[2,2/3]`. The 176.85-second progress
snapshot remains stale and does not measure the pending call's duration.

## A bounded compact-cover experiment

The research helper's `--compact-cover-search` builds a finite menu of up to
32 rational knots from the original cover, simple dyadics and ratios around
existing separators. Exact zero/one-variation transforms give edges in a
finite directed graph. Complete paths span the entire positive ray. Ranking
uses interval count and rational height as predictions; up to four candidates
then receive full Symbolica reconstruction of every original transformed
coefficient, leading coefficient, zero endpoint and finite seam. The chosen
result minimizes measured guard terms among these candidates and the baseline.
All existing production guard budgets remain unchanged.

The new prefix produced 24 knots, 325 tested edges and 178 admissible edges.
No two-piece cover exists in that finite menu. Four three-piece candidates
were reconstructed, each with 73 guards and 79,798 terms. The selected cover
is `(0,1/4)`, `(1/4,1/2)`, `(1/2,infinity)`, with variations `[1,0,1]`.
It reduces guard terms by 27.3%, retaining both original interior roots and
all seam/degree obligations, but still exceeds 65,536. Its maximum guard has
1,247 terms, coefficient height 166 bits, and the cover uses 75 coefficient
slots. The complete search takes 7.152745 s and 58,896 KiB peak RSS, including
baseline construction and four symbolic candidates. No guard factorization
was attempted in this default-budget trial, which rejects every candidate.
This bounded negative result does not exclude a better cover outside the menu.

## Explicit total-guard budget and production option

A second frozen research helper accepts `--max-total-guard-terms`, defaulting
to 65,536 with a finite ceiling of 262,144. It preserves the default-policy
failure in its report and separately checks the configured resource policy.
At 131,072, both the original greedy cover and the compact cover pass every
guard constraint. All their guards were factored with exact signed product
reconstruction; no factor was injected into the solver checkpoint.

| Cover | Guards / terms | Checked factor time | Slowest guard | Whole worker | Peak RSS |
|---|---:|---:|---:|---:|---:|
| Original four pieces | 97 / 109,726 | 6.536548 s | 0.126134 s | 33.730334 s | 54,504 KiB |
| Compact three pieces | 73 / 79,798 | 5.092787 s | 0.118882 s | 26.328667 s | 62,020 KiB |

Whole-worker time includes parsing, discovery, symbolic identities and repeated
durable JSON writes; the compact trial also searches four candidate covers.
There are respectively 117 and 93 canonical factor occurrences. The factor
routine checks signed scalar units and multiplicities before returning the
canonical basis. These are operand diagnostics, not solve-time comparisons.

The production option is `solver.local_descartes_interval_max_total_terms`.
It defaults to 65,536, validates values from 1 through 262,144, and records a
nondefault value in solver evidence. Discovery, stored-template reuse and
independent coefficient replay share this immutable per-stock budget. Missing
historical fields retain the original default. All per-guard term limits,
coefficient heights, interval/slot/root bounds and permanent template quotas
remain unchanged. Production retains its original greedy discovery; compact
cover selection remains a research helper only.

The generic production regression constructs a primitive polynomial whose
unique canonical guard sum exceeds 65,536, admits it only under the larger
budget, reuses its template, and independently replays the same guards. A
lower replay cap is rejected, as are invalid budgets and altered signs;
oversized individual guards remain rejected. Three integration tests cover
solve/verification round trips, historical omission/defaults and invalid
direct/configured/saved budgets. Together with the existing interval tests
and helper tests, all 29 focused tests pass. No original complete decomposition
follows from these resource-admission results.

## Original-problem continuation

A frozen development build with the separately optional Wang backend repeated
the original warm Gluza33 solve with the total-guard budget set to 131,072.
The original mathematical input, order, homogeneous anchor, source budgets,
template quota of two and other solver flags were unchanged. Production used
the original greedy cover. All 16,739 inherited records were verified unchanged;
no standalone research factors were injected.

The old discriminant was bypassed: a new captured factor is byte-identical,
including the full variable map, to transformed guard 69 of the independently
reconstructed four-piece cover. This guard factored in 0.056682 s. Downstream
work retained **7,194 new operations**: 7,004 factors, 71 discriminants and
119 resultants, for **23,933 total**. The worker nevertheless timed out at
**600.069031 s**, with **554,520 KiB peak RSS**, and produced no result or
verification. Gluza33 remains unresolved.

The final captured obstruction is factor key `e0d20d43…`, with 1,283 terms,
degrees `[17,13,13,0,0,0,0]` and 50-bit coefficients. Its SHA-256 is
`22f92bf277215836d66a4ef034e38be593eb87e34ec73bb7fbb0254234985cf9`.
The last progress snapshot reports level 3 and prefix `[2,1/4,1/2]` at
337.030948 s; this is stale and is not the pending call's duration. The factor
itself depends on only the first three axes. No later numeric prefix is
inferred from its variable support.

The complete metadata audit checks the original input, all inherited files,
every operation tuple key and output hash, and the missing result/verification.
It is not an independent algebraic replay of all retained operations.
Artifacts are under `artifacts/research/gluza33-total-guard-production-01/`;
`final-audit.json` has SHA-256
`c3fc069fd04a7fe5373d1ba81dd6151636100f7639fe9bd09c1193ba4055ba71`.
The combined tracked fast suite separately passed 646 tests, with zero
failures and two slow tests ignored, across 49 targets.

## Reproduction and evidence

Artifacts are in `artifacts/research/gluza33-final-interval-frontier-01/`.
Both workers use one CPU (8), 4 GiB, and the pinned production Symbolica
backend `75f8350094b90254ee71dc2a391fde0d14b0204a`, without optional root or
Wang patches. The original frozen construction uses a 30-second limit; the
compact search uses 60 seconds. These are single research measurements,
outside benchmark timings. Eleven helper tests pass, including new exact
cover/coalescence, full-map guard and repeated-root/seam rejection tests.

```sh
profile_interval_descartes source-capture.json --prefix 2,2/3 \
  --max-terms 32768 --seconds 30 --cpu 8 --output construction.json
profile_interval_descartes source-capture.json --prefix 2,2/3 \
  --max-terms 32768 --compact-cover-search --seconds 60 --cpu 8 \
  --output compact.json
profile_interval_descartes source-capture.json --prefix 2,2/3 \
  --max-terms 32768 --max-total-guard-terms 131072 --factor-guards \
  --seconds 60 --cpu 8 --output greedy-factorization.json
```

The reported `production_eligible` includes the historical default source cap
of 4,096; `production_non_source_limits_satisfied` separately reports the
fixed guard limits. The tested source override is 32,768. Coefficient signs
refer to the original transformed coefficients, while guard expressions are
primitive canonical representatives and may differ by a scalar sign.

## Release continuation and the four-root prefix

The matched original-coordinate control on the subsequent 1,283-term factor
completed in 0.042986 seconds with exact signed reconstruction; its affine
translation instead stalled on an internal 330-term component. A new frozen
release continuation therefore disabled the optional affine policy, retaining
the same original problem/order, optional Wang backend, proof budgets and
23,933 audited completed operations. This is a configuration control, not
factor injection or a cold benchmark. Cache keys bind mathematical operands
and domains; cached factors replay their exact canonical product on load.

The original worker completed that prior factor in **0.008411 seconds**, then
returned to the 19,942-term discriminant at recorded prefix **[2, 1/4]**. It
exhausted its 600-second limit: **600.100677 seconds, 880,180 KiB peak RSS**,
no result and no independent verification. It retained **140 new operations**
(130 factors, eight discriminants, two resultants), for **24,073 total**.
All inherited files and operation tuple/output hashes passed the metadata
audit; Gluza33 remains unresolved. Artifacts:
`artifacts/research/gluza33-native-release-01/`; audit SHA-256
`b72741bb87be994a4933ec0adb489d9b3ee6824df1dfae43166653d818777b37`.

A separate exact inventory at the new prefix explains a concrete fallback.
There are **four positive roots**, versus two at the previously studied
prefixes. The greedy cover has five pieces with variation counts
`(0,1/2):1`, `(1/2,1):0`, `(1,11):1`, `(11,19):1`, `(19,infinity):1`.
Its **121 guards / 139,654 total terms** exceed 131,072; all other limits
pass (maximum individual guard 1,247 terms, 215 coefficient bits, 125 slots).
Construction took 2.483 seconds. The old `[2,2/3]` cover's tail `(1,infinity)`
now contains three roots, so its at-most-one-root-per-piece proof cannot be
reused. Aggregate counters do not identify the actual source-specific stock
history or establish whether its permanent two-template limit also applied.

The frozen bounded compact-cover research search completed in **9.802
seconds** and independently reconstructed a four-piece cover:
`(0,1/2)`, `(1/2,11)`, `(11,19)`, `(19,infinity)`, each with variation one.
It retains every transformed coefficient, leading/zero endpoint and retained
finite-seam obligation, and matches the independently isolated four roots.
Its **97 guards / 109,726 terms** save 24 guards and 29,928 terms (21.43%);
all fixed limits and explicit source 32,768 / total 131,072 limits pass.
No guards were factored or injected into the original worker by this probe.

Both research probes use the ordinary pinned backend, CPU9 and a hard 2 GiB
address-space limit, with separate 30/60-second deadlines. They invoke the
frozen helper's `--worker` path under external `timeout` because its built-in
parent supervisor fixes 4 GiB. Original source SHA-256 remains
`5c15531346bf5a4a4e247db0f23fdbb38031d1bdf0485f7e08e7af7b1c2ad3d6`.
Artifact `artifacts/research/gluza33-quarter-interval-01/compact/worker.log`
contains the full exact inventory and has SHA-256
`bc2157c0ad903f5841f0a566780daa01e3005d2d6832090bb5e7a70c1f38e30c`.
The search is finite and does not prove optimality. These measurements are
supplemental diagnostics outside benchmark timings.

## Bounded adjacent coalescence in production discovery

Production discovery now makes one left-to-right pass over its existing
certified intervals before constructing symbolic guards. It proposes at most
N−1 adjacent unions. A union is accepted only when the existing variation
counts sum to at most one, the exact rational coefficient transformation has
that same variation, and the finite endpoints and removed seam are root-free.
Any zero specialized union coefficient conservatively declines the merge.
No new separator candidates, approximate root tests or global search enter
this path. The independent symbolic checker reconstructs every coefficient,
endpoint, leading coefficient and retained seam of the resulting cover.

This removes artificial rootless pieces where the exact union theorem
permits it. It does not remove any true F=0 sheet or alter the permanent
finite template stock, root isolation, guard caps or verifier semantics.
The quarter-prefix research measurement motivates the generic operation;
the large DAG search itself remains research-only. A new full original solve
has not yet been measured with this implementation.

All **22 focused tests pass**: 16 interval module tests and six existing
source/aggregate-budget integration tests. The unchanged module regressions
still exercise more than 65,536 actual distinct guard terms and permanent
stock exhaustion. New tests cover 0+1, 1+0 and 0+0 unions, multi-root and
unsaturated-bound refusal, root-valued seams/endpoints, gaps, nullifying
parameter coefficients, full variable maps, exact proof replay and tampering.
Independent read-only review found no soundness blocker. Evidence:
`artifacts/research/interval-coalescence-production-01/focused.log`.

## Direct production certificate measurements

An artifact-only harness imports byte-identical frozen production modules,
including coalescence, and calls both the real producer and its independent
checker. The source remains the authenticated 19,942-term polynomial with
full map `[x2,x1,x7,x4,x6,x3,x5]`. These are fresh-stock conditional witnesses,
not reproductions of the original solve's permanent template history or
certificates of complete original-domain coverage. Every required guard must
still have its sign certified over the ancestor cell in a real decomposition.

| Prefix / proof | Positive roots | Complete required raw guards | Total terms | Construction / replay |
|---|---:|---:|---:|---:|
| `[2,1/4]`, production intervals | 4 | 97 | 109,726 | 1.603566 / 1.538465 s |
| `[2,3/2]`, production intervals | 3 | 73 | 79,798 | 0.902197 / 0.867611 s |
| `[2,3/2]`, production global witness | 3 | 27 | 22,436 | 0.059831 / 0.025894 s |

The first cover is `(0,1/2), (1/2,11), (11,19), (19,infinity)`; the second
is `(0,1/2), (1/2,1), (1,infinity)`. These are the actual bounded production
producer's outputs, not the research DAG. The largest guard has 1,247 terms.
All 25 ordinary coefficients are nonzero at `[2,3/2]`, excluding the caller's
coefficient-nullification branch for this exact diagnostic point.

The global witness uses the same two separators, `1/2` and `1`, and exact
alternating signs `(+,-,+,-)` from zero through infinity. Its complete raw
inventory is **all 25 original coefficient guards, including the endpoint
and leading coefficient, plus both separator guards**. Reporting only the
module's two returned guards would omit the caller's obligations. The existing
source budget 8,192 declines this source; 32,768 admits the witness with the
unchanged permanent two-template stock. No proof rule or default was relaxed.

Each diagnostic ran under a separate 30-second / 4 GiB / single-CPU bound.
Full process times and peak RSS were 3.75 s / 58,368 KiB, 2.32 s / 39,936 KiB,
and 0.49 s / 15,360 KiB respectively. These are local proof measurements,
including independent replay, outside benchmark solve timings. No result was
injected into an operation cache. Evidence and exact proof exports:

- `artifacts/research/production-coalescence-replay-01/result.json`, SHA-256
  `5bd1677c6a38154e3bf14e27849d30cbef2fa0a7f87db5a2965471a1f5a91f66`.
- `artifacts/research/production-coalescence-replay-02/result.json`, SHA-256
  `05b7adbcee6e253493a6826ff2bee05cb82f6baccff4f4c5e3a31747225274fd`.
- `artifacts/research/production-global-witness-replay-01/result.json`, SHA-256
  `cc80bb7137fff4bac802ea0e2b6c87fcef1812ab6a57a975a3719ad6607bd468`.

## Frozen production continuations after coalescence and scaling

The isolated release build used production source commit
`976634997fa2bed894aa3067413c4192db7a574b`, unchanged optional Wang backend and
recorded Cargo/source/backend hashes. Build time was 10m37s; an optional-Wang-
enabled three-cell smoke solve and independent verifier passed. Frozen
`symgcad` SHA-256 is
`e4bb6aed0fcb2cc73ff778c40bb59efafb69967792d466bc3b49caa26ce3c6dd`;
`hard_probe` is
`e49c33048e1a7d9812303007f52c2b8e3eedd2cb57949996fe51169e3b6ec574`.

Both original Gluza33 runs preserve the exact mathematical input, fixed order
and anchor. Both use scaling-first affine factorization, interval source
budget 32,768, total interval guard budget 131,072 and the unchanged permanent
template limits. Each uses one CPU8 worker and 50 GiB. The second changes only
the global source admission, total deadline and fresh cache path. Earlier
build, discovery and cache-generation costs are excluded: these are warm
experimental continuations, not cold benchmark speedups.

| Global source cap | Total limit | Worker time | Peak RSS | Outcome |
|---|---:|---:|---:|---|
| 8,192 | 1,230 s; solve capped 600 s | 600.125674 s | 880,564 KiB | Timeout; no result or verification |
| 32,768 | 300 s, including any verification | 299.434076 s | 653,576 KiB | Timeout; no result or verification |

The first run added **244 operations** (243 factors and one discriminant),
from 24,073 to **24,317** retained records. It reached the same large
uncompleted discriminant at recorded prefix `[2,3/2]`. All inherited files
and every operation tuple/output hash passed the final metadata audit:
`artifacts/research/gluza33-coalesced-scaling-release-01/final-audit.json`,
SHA-256 `84c6ff5d42e06cec270fc7962ccd0178fd927c89d8f5d85992015e69416bf52f`.

The second run inherited that audited cache and returned to the same
uncompleted discriminant at recorded `[2,1/4]`. Its whole campaign took
300.101 seconds; setup consumed the difference from the worker's remaining
299.434-second allocation. The supervisor would have limited independent
verification to remaining campaign time, but no complete result appeared.
It retained **89 new factors**, bringing the cache to **24,406 records**.
All inherited files remained unchanged and every operation tuple/output hash
passed the final audit:
`artifacts/research/gluza33-global-witness-release-01/final-audit.json`,
SHA-256 `c7d1503ca748728c8f4fc7949db01142a66a11475208986890c5bc427e907379`.
The final cache manifest SHA-256 is
`112d8abd2ba84f1cfe03160b78f95ca5f86ba233bce453f7d76bc6f6349d4da6`.
These audits establish artifact and cache integrity; they do not certify a
complete decomposition.

The capture object itself contains no prefix. Frozen `CachedOperations` code
refreshes the matching diagnostic level/prefix before every large
discriminant, while ordinary progress publication invalidates the remembered
context key. This establishes a code-associated diagnostic context, not a
standalone proof certificate; the latest progress time is not the pending
operation's duration. Per-polynomial permanent stock history was not exported.
Fresh witness success therefore does not prove which lifetime-stock or other
call-path condition caused the original fallback. **Gluza33 remains unresolved**;
no further campaign was launched after this final bounded control.
