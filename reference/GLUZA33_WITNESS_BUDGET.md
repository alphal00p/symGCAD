# Gluza33: the saturated-witness source budget

Research-only result, 11 October 2026. Increasing the diagnostic helper's source
budget from 4,096 to 8,192 terms admits an exact positive-root witness for the
new Gluza33 obstruction. Discovery and all eighteen checked guard
factorizations finish in **2.623493 seconds**, using **16,324 KiB** peak RSS.
Production eligibility is unchanged; no lower-dimensional guard projection
or new original-problem decomposition is claimed.

## Bound source and prefix

The [affine continuation](AFFINE_GLUZA33_CONTINUATION.md) reached a
6,938-term polynomial of degree 16 in `x7`, with remaining degrees
`[30,32]` in `[x2,x1]`. Its complete map is
`[x2,x1,x7,x4,x6,x3,x5]`. Its discriminant request has key
`5aac94020b04dd127c4abce7b7f24d4910a9fd8dee377abcb9db81a02e31cc3d`.

A separate short diagnostic records `current_level = 2` and the exact prefix
`[1/2,2]` in `statistics.interrupted_local_progress`. Its captured operand is
byte-identical to the original pending discriminant capture:

- `artifacts/research/affine-gluza33-continuation-01/first-discriminant-capture.json`
- `artifacts/research/affine-gluza33-continuation-01/prefix-diagnostic/captures/active-217033.json`

Both have SHA-256
`2c5be2b92053d1c85fba321eb5a67e5204c3954c926387d6a57d678a00926d6a`.
The progress result has SHA-256
`d2d98b4d2dcee99f49d3cd24c2ed9cf099e3ff31a227321fa73a1bc02c5c0e33`.
This is a newly recorded prefix for this operand, not the earlier degree-ten
polynomial's saved prefix. Independent exact Sturm counting and root isolation
agree on two positive roots here, with no degree drop and seventeen nonzero
coefficients.

The production saturated-witness admission test requires at most 4,096 source
terms. Thus this particular operand fails that arithmetic budget independently
of the template-stock counters. The new helper option `--max-terms` defaults
to 4,096 and permits research values from 1 through 65,536. No fixture, hash,
graph identifier or production dispatch rule selects the override.

## Exact witness and every retained guard

For `p(x2,x1,x7)`, the nonzero coefficient signs from constant to leading are

```text
[+, +, +, +, +, +, +, +, +, -, -, -, +, +, +, +, +]
```

Equivalently, powers `0..8` and `12..16` are positive, and powers `9..11`
are negative: exactly two Descartes variations. The exact separator is
`x7=2`, and the signs of `p(0)`, `p(2)`, and the leading coefficient are
`(+,-,+)` at `(x2,x1)=(1/2,2)`.

On a connected region where all these guard signs remain strict and `x7>0`
is the proven domain, the intermediate value theorem supplies a root in each
of `(0,2)` and `(2,infinity)`. Descartes' upper bound counts multiplicities;
therefore these are the only positive roots and both are simple. Their
continuous separated sections remain exact interior boundaries. A guard
vanishing, a degree drop, or failure of the sign pattern requires fallback
or further subdivision.

The helper isolated twelve distinct real roots at the recorded prefix, ten
negative and two positive. It leaves the source polynomial unchanged. A
collision confined to negative or nonreal roots need not invalidate this
positive-domain certificate, while the coefficient, endpoint and separator
guards prohibit losing a positive root through zero, infinity or collision.
No negative-domain problem is certified by this positive-domain argument.

All seventeen coefficient guards and the separator evaluation are retained.
Degrees below refer to `[x2,x1]`; all seven map entries are preserved.

| Guard | Terms | Degrees | Factor time (ms) | Terms in exported boundary factors |
| --- | ---: | --- | ---: | --- |
| Coefficient 0 | 441 | `[30,32]` | 23.109 | 2, 1, 3, 137 |
| Coefficient 1 | 440 | `[30,31]` | 18.576 | 2, 1, 3, 181 |
| Coefficient 2 | 453 | `[30,30]` | 34.681 | 2, 1, 3, 239 |
| Coefficient 3 | 459 | `[30,30]` | 42.680 | 2, 1, 3, 295 |
| Coefficient 4 | 471 | `[30,29]` | 29.166 | 2, 1, 367 |
| Coefficient 5 | 467 | `[29,28]` | 33.700 | 2, 1, 389 |
| Coefficient 6 | 480 | `[29,28]` | 38.557 | 2, 1, 426 |
| Coefficient 7 | 475 | `[28,27]` | 23.593 | 2, 1, 448 |
| Coefficient 8 | 481 | `[28,26]` | 31.467 | 481 |
| Coefficient 9 | 451 | `[27,26]` | 33.875 | 451 |
| Coefficient 10 | 419 | `[26,25]` | 263.245 | 419 |
| Coefficient 11 | 388 | `[25,24]` | 17.760 | 388 |
| Coefficient 12 | 359 | `[24,24]` | 497.130 | 359 |
| Coefficient 13 | 329 | `[23,22]` | 261.536 | 329 |
| Coefficient 14 | 302 | `[22,22]` | 23.098 | 302 |
| Coefficient 15 | 274 | `[21,20]` | 17.625 | 274 |
| Coefficient 16 | 249 | `[20,20]` | 15.901 | 249 |
| Separator `p(x2,x1,2)` | 725 | `[30,32]` | 46.160 | 725 |

The eighteen guard factorizations total **1.451859 seconds**. Discovery takes
0.013510 seconds, including 0.001962 seconds of root isolation and 0.001947
seconds of guard construction. The full process also includes parsing,
formatting, checkpoint writes and independent signed factor reconstruction.

Every factorization uses `factors_checked`: the full signed product,
including scalar content and multiplicities, is reconstructed before
canonical boundary-factor export. The report retains every raw signed guard
and the complete canonical factor inventory: 38 occurrences and 21 distinct
boundary factors. These canonical factors alone are not a serialized signed
product certificate; their checked scalar and multiplicity data are internal
to `factors_checked`.

## Reproduction and limits

The frozen helper used the ordinary production dependency checkout, with no
optional positive-root or Taylor-shift research patches. Its command was:

```sh
profile_descartes_witness \
  artifacts/research/affine-gluza33-continuation-01/first-discriminant-capture.json \
  --prefix 1/2,2 --max-terms 8192 --factor-guards --seconds 60 --cpu 8 \
  --output artifacts/research/gluza33-witness-budget-01/report.json
```

The worker was capped at one CPU, 4 GiB and 60 seconds. It completed with
exit code zero and no truncated output. This is one fresh development-build
measurement, outside corpus benchmark timings. Seven helper tests passed,
including a generic 4,101-term source that the default rejects and the
override admits, exact reconstruction from emitted coefficient guards, and
invalid budget, axis, prefix, map and degree rejection.

Artifacts are under `artifacts/research/gluza33-witness-budget-01/`:

| Evidence | SHA-256 |
| --- | --- |
| Frozen helper | `4491bd6b72869b833d8f82d7658f2fd821aa141af3c3f7841bc9916e64b8d6f6` |
| Helper source | `0c8893756f0bbd5848ea71a9f0c46e1cf3371f33644224fe7fd98ba2498183e3` |
| `audit.json` | `e0739fd59f0eb13add6008bfedf96e9ec1fced3b637a39a76ff36c55e1a4511c` |

The audit binds the captured prefix, both identical source captures, the
independent fiber diagnostic, every raw guard and factor inventory, process
limits, binary and source hashes. `report.json` and `report.progress.json`
preserve the exact guard expressions and measurements.

This result motivates a general, bounded production budget policy based on
source and guard costs, with matching verifier limits. It does not establish
that the subsequent two-parameter guard projection is cheap: eighteen guards
still introduce substantial ordinary and pairwise obligations. Those costs
and original-solve verification remain future work. No production cap was
changed and no further original campaign was launched for this experiment.
