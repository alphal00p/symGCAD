# Gluza33: the next interval-witness resource gate

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
