# Slow original ordering APIs

The six ordering calls that timed out in both implementations at the historical
60-second limit are tagged `slow` in `tests/api_classification.json`. The other
16 ordering calls and one projection call are tagged `fast`. This inventory is
separate from the 331 mathematical solve groups: an ordering call remains
unresolved even when another engine has decomposed its inequalities.

`examples/api_compare.rs` imports the original Rust corpus entries, checks that
the classification covers exactly those 23 API identifiers, and supports
`--suite all|fast|slow` and repeatable exact `--case` selection. Unknown,
duplicate, or out-of-suite identifiers fail before a worker starts. It never
uses a proposed order from a source comment as a completed API result.

Build the helper, then give each slow identifier its own bounded campaign:

```sh
cargo build --example api_compare
taskset -c 0 target/debug/examples/api_compare \
  --solver /absolute/path/to/frozen-release-symgcad \
  --suite slow --case test_massless_planar_elliptic_symb/order/1 \
  --timeout 3600 --campaign-time-secs 3600 --memory-mib 8192 \
  --output artifacts/api-hour-massless-01
```

Use a fresh output directory for each attempt. The helper snapshots and hashes
the solver executable and records the input, adapter, corpus, classification,
and helper hashes. It writes an atomic progress report before each worker and
retains process output, peak RSS, elapsed time, and returned order/projection.
Native and Rust workers run sequentially within a campaign. Any concurrently
scheduled campaigns must stay within the shared 16-core/64-GiB allocation.

Rust runs first. Each native call is bounded by the smaller of the explicit
timeout, remaining one-hour campaign budget, 3600 seconds, and
`max(300 seconds, 3 × completed Rust API wall time)`. Failed, truncated,
incomplete, or malformed Rust results cannot raise that cap. A Rust timeout
at the full hour leaves the reference pending; it cannot become a match.
Complete Rust orders must respect the original variables, groups, and fixed
orders. Comparison checks the exact native order and, for projection, the
canonical exact per-level polynomial sets.

These are single compatibility attempts, not three-repetition benchmark
medians or solve-time ratios. The helper exits nonzero if any selected check
is unfinished or mismatched. A timeout stays `slow`; changing its deadline
does not change its status. Historical reports are preserved unchanged.

## Validation and current campaign

The two helper regressions pass, including rejection of empty heuristic
orders, invalid groups, incomplete projection shapes, failed/truncated
processes, and invalid case selection. A fresh native smoke replay of
`test_input_unmodified/order/1` matched the exact order: Rust 0.0216 seconds,
native 0.9570 seconds, with an explicitly recorded five-second native cap.
Trying to reuse its output directory failed without changing the report hash.
This is a compatibility smoke check, not a performance ratio.

A subsequent fresh `--suite fast` replay matched **all 17 APIs** (16 orders
and one exact projection) in 18.252 seconds. Every native call had an explicit
15-second cap; the longest completed native call took 1.792 seconds. The
report is `artifacts/bench/api-fast-capped-01/report.json`, SHA-256
`2bc0abc2f588f2214c188cd4b196e29a1751e8ec41126d679691a29502668896`.

The first two one-hour API attempts, symbolic massless and symbolic physical
elliptic ordering on CPUs 0 and 1 at 8 GiB each, **both timed out without an
order**. Their native calls did not start: the Rust attempt exhausted the
one-hour campaign. This is `rust_incomplete_reference_pending`, with
`reference_status="campaign_exhausted"`, not a native timeout or a match.

| Original API | Original heuristic | Rust wall seconds | Peak RSS, KiB | Campaign wall seconds |
| --- | --- | ---: | ---: | ---: |
| `test_massless_planar_elliptic_symb/order/1` | `mods` | 3600.094002363 | 613,476 | 3600.095273268 |
| `test_elliptic2l_physical_symb/order/1` | `t1` | 3600.030009300 | 94,504 | 3600.031283764 |

The worker deadlines were 3599.999666199 and 3599.999666349 seconds,
respectively; the small excess in observed wall time includes supervisor
polling and termination. Each saved process has `status="timeout"`, no exit
code because it was killed, empty stdout/stderr, untruncated output and
`result=null`. Neither directory contains a Rust result file or a native
process record. The outer comparison helpers exited 1, as required for an
unfinished API check. These attempts supply no completed Rust timing for an
adaptive native cap and no compatibility or speed-ratio evidence. Both API
identifiers remain `slow`; the separate mathematical solve completion count
is unchanged.

An independent read-only audit checked exact equality of the input names,
variables, groups, constraints, splits and parameters with regenerated
original corpus entries0312 and0317. The original ordering heuristic, empty
fixed order, merge mode and seed also match. Input bytes match the report
hashes; the snapshotted executable hashes match the report provenance. Saved
process status, wall time and peak RSS agree with the final report rows. The
core-input, classification and both adapter hashes were checked against the
current files, and the frozen helper hash was independently checked.

The immutable evidence is under
`artifacts/bench/api-hour-01/{massless,physical}/`:

| Case | File | SHA-256 |
| --- | --- | --- |
| Massless | `report.json` | `68d0a1864d483dd27c1edb5a4baf297c39fdc87fb1a7b1ea02d988d3d408b7a9` |
| Massless | `0004-input.json` | `ca7bb9e2fd7764231917c9824dce5c374ada7d5ac28d825f9e90146ce2c7d32f` |
| Massless | `0004-rust.json` | `2ca4e0a3e99047acd9109c7dc345db9270fc2fa5d21b07c87c5d897c8c9ddf52` |
| Physical | `report.json` | `3ba473b31baf862f2c408ef23d78e9c4c137c37655df8607e463dfbf72beeda0` |
| Physical | `0007-input.json` | `306f5296db86feba3cc1717b91f837a55201d5187fbd7ed03e24336c2a1981ff` |
| Physical | `0007-rust.json` | `fc7ff282bd1b479a6c00278234e36a0dae01ab50cec459fbaa004fb2325b7520` |

The third one-hour attempt, `test_gluza_ex33_trivial/order/1`, also timed out
without an order: 3600.046124255 seconds, 414,016 KiB peak RSS. Its original
`t1` heuristic, mathematical input and frozen executable were checked against
the imported source and provenance. The native call remains pending because
the campaign was exhausted. Its final report SHA-256 is
`013f137db31c34b60649dc4fed16bfc4135cecfcb24dc124634c07509e4b4ca4`;
the process record is
`3440fe542d802a4f5c385c962d0dc12e6b6ae51f7f895d11c456299045ef0581`.

The fourth attempt, `test_gluza_ex33/order/1`, timed out on CPU1 after
3600.101920466 seconds, with 512,764 KiB peak RSS. Its input, original `t1`
heuristic and frozen executable passed the same independent provenance audit.
The native call remains pending because the campaign was exhausted; neither
an order nor a compatibility result was obtained. Its final report SHA-256 is
`61964679e1fbd7eb2ca036fabbc3dfadc7f155b6432ac318dca269441f69b607`;
the process record is
`6fd92dfc2813c10afbf15ea43f20076fb39c6392fc92581a7c42e793550751d3`.
The third and fourth evidence directories are
`artifacts/bench/api-hour-01/{gluza33-trivial,gluza33}/`.
The fifth attempt, `test_pentabox_exb_symb/order/1`, timed out on CPU0 after
3600.255444751 seconds, with 2,628,308 KiB peak RSS. Its native call remains
pending because the campaign was exhausted.

The sixth, `test_pentabox_exd_symb/order/1`, failed on CPU3 after
395.231326809 seconds with an allocation failure under the 8-GiB limit;
peak RSS was 8,174,724 KiB. It produced no order. The subsequent native call
used the required 300-second cap, since failed Rust work supplies no completed
timing, and timed out after 300.345387732 seconds at 2,943,216 KiB peak RSS.
The combined campaign took 695.593972095 seconds. The report's aggregate
`reference_timeout` label does not conceal its separate `rust_status="failed"`.

All six slow API attempts are now finished: five Rust timeouts and one memory
failure, with no completed compatibility result. They remain `slow`. These
ordering APIs are separate from the 331 mathematical decomposition groups.
The final two inputs passed the same original-math, heuristic, executable,
adapter and process-record audit; the evidence is under
`artifacts/bench/api-hour-01/{pentabox-b,pentabox-d}/`.

| Case | File | SHA-256 |
| --- | --- | --- |
| Pentabox B | `report.json` | `9820c59dff5a5a34ffa25870ee15db3ae8a91c3a0b014ea4ab6f1f41b3b05994` |
| Pentabox B | `0015-rust.json` | `7810547ebacf432ab997c9a83bc57dc6acbd2cbeaa56a90b0a13057f4e9d5594` |
| Pentabox B | `0015-input.json` | `120e230d5d07bdd6e54a086d4a541b3c3644e6ce1b628d86be85fede672a8fa5` |
| Pentabox D | `report.json` | `74eacd2214decbf06534f933444dfeb52097cedf6e0402569c33d05567a4c6a6` |
| Pentabox D | `0017-rust.json` | `f0239481a734bcdc13e6ff970c2bf08c9d0f0e0c64a390853e106c259918035a` |
| Pentabox D | `0017-reference.json` | `67c14e849f41cd299ea139f36cfe56f6549390337f203a21a2de735c643de4f1` |
| Pentabox D | `0017-input.json` | `f90600fcfc51f02799c1028ec1827f204ded31a37c364ef74b097feacc0b2bbb` |

| Evidence | SHA-256 |
| --- | --- |
| Frozen API helper | `986e7630954790240741eeb9f91d8b42318885977c9bb4dc991d26c4c7380fd8` |
| Frozen release solver | `a8ddd56489c83950c409e6e7dbcfb51f6f7fefffdf29484aba7519b8097dc911` |
| Focused test log | `8ed585ae139d471add712ee8cba2563e81605ff4994d001eb54079a5ac48700f` |
| Smoke `report.json` | `a043df964bee079b1b8ad90576cb2666c42613b78328171843a0509b05947d20` |
