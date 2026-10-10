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

The first two one-hour API attempts cover symbolic massless and symbolic
physical elliptic ordering on CPUs 0 and 1, at 8 GiB each. They are stored in
`artifacts/bench/api-hour-01/{massless,physical}/`; their results remain pending
at this checkpoint. The four remaining slow API cases are still pending.
These workers are independent of the ongoing original-decomposition queue.

| Evidence | SHA-256 |
| --- | --- |
| Frozen API helper | `986e7630954790240741eeb9f91d8b42318885977c9bb4dc991d26c4c7380fd8` |
| Frozen release solver | `a8ddd56489c83950c409e6e7dbcfb51f6f7fefffdf29484aba7519b8097dc911` |
| Focused test log | `8ed585ae139d471add712ee8cba2563e81605ff4994d001eb54079a5ac48700f` |
| Smoke `report.json` | `a043df964bee079b1b8ad90576cb2666c42613b78328171843a0509b05947d20` |
