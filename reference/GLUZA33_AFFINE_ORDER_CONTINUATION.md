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
That specific operation lies outside the discriminant segmentation envelope
(one parameter at launch, at most two after `380d014`) and is not a
first-axis univariate basis input. This is a captured pending operation, not a completed timing or a reason to
discard any of its roots. No original completion is credited before a
complete result and independent full proof verification.

## First outcome and higher-memory continuation

The `x3` cold run finished at **13:46:33 UTC** with an explicit memory
allocation failure, after **1,341.544040 seconds**, at **3,314,968 KiB peak
RSS** under its 4 GiB address-space limit. It was not a one-hour timeout.
The public command returned a `computation_failure` result containing no
accepted cells or trace; no verification ran. Its diagnostic records a
failed 1,572,864-byte allocation. The last local progress snapshot was stale
at 19.207449 seconds and cannot time or identify the final native call.
The final exact capture is byte-identical to the earlier `x4` discriminant
snapshot above, and its operation key has no completed cache record.

The final cache contains **4,685 completed native records**, **18,699,827
bytes**: 4,453 factorizations, 192 discriminants and 40 resultants. Every
file hash, native operation key, output/multiplicity checksum, full variable
map and domain fingerprint was checked. Recorded operation times sum to
1.439851 seconds; they are not the full process time. The final audit and
manifest under `gluza33-x5-last-hour-01/` have SHA-256:

- `final-audit.json`: `41e62773469851184acd2c27f0fa237e375af8f81cc0cf92b0bb6f61bd9e7750`.
- `final-cache-manifest.json`: `4cac2ac2c33cf07a6e5bc4f35d228cd779f39f8d892729d902591531a4102f9a`.
- The failed `0-result.json`: `b716821093985a198a78bc6a863e72e11b4b4d146eae5ce0af21afbd7346b3cb`.

A fresh warm continuation launched at **14:30:55 UTC**, CPU13,
**24 GiB**, using the same original mathematics, `x3` anchor, order and
proof options. The only input differences are the memory limit and fresh
checkpoint namespace. An earlier 8 GiB preparation remains immutable and
was never launched; the 24 GiB allowance superseded it before execution.
The new run copies and rechecks all 4,685 completed native records, with no
research algebra injected. Solve and verification still share one hour,
with verification capped at 600 seconds of the remaining time.

The frozen executable is `symgcad-two-parameter-discriminant-380d014-release`,
SHA-256 `9f4230ea1b6a1c5c1090237947d76f432591f2d55a98897c9e40eea2c02d71a3`.
Binary-resultant and base-basis options remain disabled for this Gluza input.
The actual worker audit confirms all original fields, explicit solver
options, order, anchor, shared cache, CPU13, 24 GiB limit and capture
propagation. Its first saved pending operand is the same `x4` discriminant.
This is a measured memory-limit follow-up, with no topology-specific
production dispatch or omitted interior boundary.

New artifacts are under
`artifacts/research/gluza33-x5-last-warm24g-hour-03/`:

| Artifact | SHA-256 |
| --- | --- |
| `input-audit.json` | `64f3a4e4b0c6b847e1769c56d760303578059163ddc61b713c8b334184ffad4c` |
| `initial-cache-manifest.json` | `3eb29ada5b779680cba30ab63f28b5dd627f304f15b3a8ae2d77442d8618c2d2` |
| `launch.json` | `2a8a41afe8cec8be1d5381d7f07cc594b256c59970ee07e307ac4eaf93d8be8d` |
| `actual-worker-audit.json` | `1f87b7e04b9d0746d3e92dfd299284dd6bb9a12596e4f754ace7e8c0ff6f5d2b` |

The earlier 1,341.544-second failed solve and one-second search nomination
remain part of the retained history. This warm launch is not a cold speed
comparison and earns no completion credit before full verification.

## Nonlinear-anchor alternative

A separate cold campaign launched at **13:36:23 UTC**, CPU 0, 4 GiB, with
the same frozen executable, proof options and shared one-hour budget. It
uses anchor `x0` and lifting order `[x0,x2,x1,x7,x4,x6,x3,x5]`.
Only the order, anchor and fresh checkpoint path change from the first
campaign. The positive homogeneous quartic permits this scale choice;
exact `x0=1` substitution leaves 143 terms and retains both affine axes,
including terms `x1*x3` and `x2*x5` with coefficient +1.

Retaining both affine axes is **not itself a new strategy**: earlier
original-input Brown portfolios with anchors `x0`, `x1` and `x6` already
eliminated `x3` then `x5`, and timed out at 20 seconds with selected
60-second promotions. This alternative reverses their affine elimination
order and preserves the first campaign's relative middle order. Its exact
order/anchor pair was absent from the bounded audit of 178 retained TOMLs
(50 matching the original mathematical fields) and the 99-trial
`x3`-anchor search. This is not an exhaustive claim about unrecorded
history or a prediction of lower projection cost. All earlier failures
and all original boundary obligations remain retained.

Files below are under `artifacts/research/gluza33-x0-anchor-hour-01/`:

| Artifact | SHA-256 |
| --- | --- |
| `input.toml` | `f0932cf08e031bdcd15abcc7b9eaf94945ef7534bcc6ee21d6312c49d9656a99` |
| `input-audit.json` | `bd554823c9a178c0d37f632d1d2321e29b0364183d729128a31875f454c3495e` |
| `campaign.toml` | `a5b24d38d36ad511f689ad5407184621135e31307ed6c1691198103e050622e8` |
| `launch.json` | `5aa65baa37376b6b7e249e5fce774187583b163f60907eca6bd8d2f92f05b1ea` |
| `actual-worker-audit.json` | `6528638bfb5f8ad5a63a8107602dfefa9cfc96a05508b5e6048d0ef2f47f4fd1` |

The independent live audit checked worker 3166062's original mathematical
fields, explicit options, order, anchor, CPU affinity, memory/time limits
and capture environment. The worker uses the campaign's fresh cold
checkpoint path. Its first retained pending capture is a 224-term factor
input with multidegrees `[9,9,6,0,0,0,0]` in reduced map
`[x2,x1,x7,x4,x6,x3,x5]`, coefficient height 17 bits. This is an exact
operand snapshot, not a completed operation timing or a proven bottleneck.
Neither launch changes the verified original count, **303/331**.

The `x0` alternative subsequently **timed out at 3,600.025448 seconds**,
with **181,628 KiB peak RSS**, no result and no verification. Its final
cache contains **5,177 completed native records**, **26,872,438 bytes**:
4,998 factorizations, 55 resultants and 124 discriminants. A bounded final
audit rechecked all native keys, output/multiplicity checksums, maps,
domain fingerprints and original input/binary bindings. Completed operation
times sum to 38.225178 seconds, including resultants of 13.759818,
12.179291 and 7.243604 seconds; these are completed-record timings only.

The final pending capture remains byte-identical to the initial 224-term,
three-variable factor operand, input hash
`692447d2758a0dc4af4c3cb0ef38f2a17deb480ab1589f00a1d2e40a9d8421a1`.
Its key has no completed cache record. The saved progress snapshot is stale
at 53.293018 seconds, so no operation duration is inferred from it. This
multivariate factor input is outside both discriminant segmentation and the
first-axis-only support-basis mode; neither recent option is a demonstrated
remedy for this particular pending request. No automatic restart followed.

Final evidence under `gluza33-x0-anchor-hour-01/`:

- `results/ledger.json`: `894a2d96b9d9abfd3452b2b381d435ae6ad4a7d3e94153c980c520f9a115ccb2`.
- `final-audit.json`: `2f6d0280d94e3aa209b0991f815caab4454ffc389f90a708f5aa61fc2aa09ba7`.
- `final-cache-manifest.json`: `fe3dac6ee33672e0c1eb60180257a5274b52edaa0411491f920cd2ec0d68a37d`.
- `final-capture.json`: `162d5b9b98efeed85cadf3f375ad914fa902682a58764d927505927e66100646`.

The two completed cold attempts add no original coverage. The separate
24 GiB `x3` continuation also finished incomplete, as recorded below.

## Existing native CRT diagnostic

A separate replay of the exact saved `x4` discriminant used the existing
Symbolica CRT backend on CPU 2 with a 300-second, 4-GiB limit. It **timed out
at 300.032155 seconds**, peak RSS **111,500 KiB**, without producing a
candidate. The original 24-GiB solve continued untouched. Input, full variable
map, frozen executable and launch-script hashes were rechecked after the run;
no solver cache received research output.

Native `resultant_crt` can stop either at its deterministic coefficient-bound
threshold or after stable reconstruction and two fresh modular checks. The
latter is Monte Carlo, and this profiler does not expose which branch or prime
count it reaches. Even a completed output would therefore require deterministic
validation before reuse. This censored trial establishes neither a deterministic
CRT speedup nor its impossibility.

Evidence is under `artifacts/research/gluza33-x4-discriminant-crt-01/`:
`report.process.json` SHA-256
`d479c267e6711bff5c9442095aa61c019462b4f502eaa57a39178cda50b7f6cf`,
and `final-audit.json`
`f90a485fb37def6fe4ea7703a2b602d3cb2e4ba844e16382b1024fab07212d5e`.

## Terminal 24 GiB continuation and two-hour preparation

The 24 GiB continuation ended at **15:30:56 UTC**, with a
**3,600.037918-second timeout**, **3,461,012 KiB peak RSS**, no result and no
verification. Its 4,685 completed native records (18,699,827 bytes) are all
unchanged from the initial cache; no new completed operation was saved.
The final captured request remains the same 2,209-term `x4` discriminant,
with no completed record for its key. The progress snapshot is stale at
12.542224 seconds, so it supplies no duration for that operation.

Under `artifacts/research/gluza33-x5-last-warm24g-hour-03/`, the final audit is
`67787f8a318e2e9bcef0ff1a22374cb8d1b03ea414c79925b0816e4fef5d8dee`,
the cache manifest is
`0293182a1e5c2ce553cb064eae47caa6bb8d50180d9b664dd770e254ca1d5b73`,
and the captured input is
`aab900ab332f90a1d0c9d32ec73cdd65b88d64b22a45d20e23ef77e33aa70289`.
All original mathematical fields, map, order, native checksums, inherited
files and executable bindings were rechecked without native algebra.

The requested 50 GiB, two-hour continuation first encountered a launcher
limit: frozen release `9773999` rejects campaign and problem deadlines above
3,600 seconds. Attempt `gluza33-x5-last-warm50g-two-hour-04` therefore exited
before creating any worker, result or capture. Its immutable failure audit is
`76feb778e290f720fdfdc9e12a5984cb4b0a93fe6aab5c6399ab8ab89f1a9cf4`.
A fresh `...-05` preparation preserves the original mathematics, order,
anchor and proof options, copies only the 4,685 authenticated native records,
and awaits a tested generic deadline-limit release. Its input audit is
`df708d4cc17e52c105d7e4a705b24dceb80509cb5fb0d3d77bb0b5eef7e0b348`;
its initial manifest is
`48b4f573960486300c37d22904002cf265e5a3b26f9ad93111c4734a326cd936`.
This preparation is not a completed solve or a launched two-hour attempt.
