# Original positive Gluza33: an affine-last ordering trial

Research launch, 2026-10-10, **not a completed decomposition**. The input is
`artifacts/corpus/symbolic/0338_test_gluza_ex33_solve_3.toml`.
Its eight positive-coordinate inequalities, strict `F>0` target, parameters,
variables and single variable group are preserved exactly. There is no
original fixed order. This trial uses the existing local engine with given
lifting order

```text
x3, x2, x1, x7, x4, x6, x0, x5
```

and the existing homogeneous reduction anchored at `x3`. The original
homogeneous quartic is affine in `x3` and `x5`; the other axes have degree
two. Setting `x3=1` leaves `x5` affine, with nonzero coefficient evidenced
by the surviving term `x0²*x2*x5`. The reduced polynomial has 143 terms.
This is an ordinary legal ordering choice. It neither replaces `F>0` by
an endpoint condition nor omits any interior `F=0`, coefficient, root-order
or other boundary required by the local proof and its independent verifier.

## Why this candidate

The preceding original one-hour attempt used a 300-second MCTS search with
99 timed-out trials and no completed winner. Its eventual fallback was
`[x3,x0,x1,x2,x4,x5,x6,x7]`, ending in a quadratic axis. Fourteen distinct
orders ending in `x5` each received only one second in that search.

The selected order was trial 85 (zero based). It returned an incomplete
result with a fresh 1.000643-second snapshot: 3,525 speculative nodes,
351 refinements, 758 registered polynomials, 21,756 terms, maximum total
degree 11 and coefficient height 52 bits. Its existing log-inventory
promotion score, 28.938522, was the smallest among the seven `x5`-last
trials with an explicit cell count and at least 0.9 seconds of recorded
progress. This is a heuristic nomination, not a predicted speedup or
completion. The failed one-second measurement is retained; that search
gave this order no longer promotion.

The original hour's final progress was stale at 1.006 seconds, and no
operation capture was enabled. A bounded metadata scan found 634 completed
records for its final map; the largest recorded operation was a 0.529-second
resultant. These records do not identify the subsequent pending operation.
Earlier endpoint-base and box-chart experiments concern different research
problems and do not constitute completed original-order trials; see
[the bilinear investigation](GLUZA33_POSITIVE_BILINEAR.md).

## Limits, provenance and live admission check

The new cold campaign launched at **13:24:11 UTC**, CPU 13, one worker,
4 GiB. Solve and independent verification share a 3,600-second campaign;
verification receives the remaining time, capped at 600 seconds. MCTS is
disabled. All other explicit solver flags are preserved from the preceding
hour recipe. Binary segmentation, typed first-axis bases and deferred
projection remain at their default disabled settings. No research output
or previous checkpoint was injected. Exact operation captures are enabled.

The frozen release is `symgcad-base-univariate-9b7dd56-release`, source
`9b7dd5649d2baa380c467ce05246ea21680b4b04`, executable SHA-256
`7bf9db11d4f8636ad09b57b52aaf6b87a3b61ab3a3cb61d70d9cef592c752657`.
Files below are under `artifacts/research/gluza33-x5-last-hour-01/`:

| Artifact | SHA-256 |
| --- | --- |
| `input.toml` | `0b8cef4fe0f420194be1f5be1136b9a34ca1ecf9fcb35b7f25dc9f1d7ffbbda1` |
| `input-audit.json` | `f67aad78fe38404d9ee238b45e29f8bff26d9c9f474c90db90c39af77a4be866` |
| `campaign.toml` | `85f32cac8bf675adbe23016a495aaeb4158761cfce46ce7da80f0935795704f0` |
| `launch.json` | `d132d1c7f95e253dd916e1069682777cea15ddf23662887dc6d44b94505e047b` |
| `actual-worker-audit.json` | `836d6c0171e03973270fe53cd852efdff794f51b2f84f48f2bb006773e1b3d1c` |

The independent live check bound the actual worker's original mathematical
fields, given order, anchor and explicit solver options to the prepared
input. Only the campaign's fresh checkpoint path differs. It also checked
capture propagation to that worker. At 13:26:12 UTC its saved pending capture
was a degree-10 `x4` discriminant of a 2,209-term polynomial, coefficient
height 28 bits, with three surviving axes `[x2,x1,x7]`. The immutable capture
snapshot SHA is
`aab900ab332f90a1d0c9d32ec73cdd65b88d64b22a45d20e23ef77e33aa70289`.
That specific operation lies outside the current one-parameter discriminant
segmentation envelope and is not a first-axis univariate basis input. This
is a captured pending operation, not a completed timing or a reason to
discard any of its roots. No original completion is credited before a
complete result and independent full proof verification.
