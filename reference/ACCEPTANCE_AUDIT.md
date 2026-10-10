# Acceptance obligations beyond mathematical completion

Read-only audit at the 302/331 completion checkpoint. All 331 equivalence groups,
579 sign-expanded baseline inputs, 277 fixture identifiers, 93 extracted core
calls and three disabled core tests are accounted for. This note records
assertion-level obligations separately from the 29 unresolved mathematical
problems. It does not alter either denominator.

## Ordering APIs

`artifacts/bench/core-api-deterministic/report.json` records 16 exact order
matches, one exact projection match and six ordering calls that timed out in
both implementations at 60 seconds:

- `test_massless_planar_elliptic_symb/order/1`
- `test_elliptic2l_physical_symb/order/1`
- `test_gluza_ex33_trivial/order/1`
- `test_gluza_ex33/order/1`
- `test_pentabox_exb_symb/order/1`
- `test_pentabox_exd_symb/order/1`

These remain incomplete API workloads even when the corresponding mathematical
problem has a verified decomposition. Several upstream expected-order assertions
are commented out; the timeouts are not completed mismatches. Original ordering
and projection assertions must retain their separate dispositions at M6.

## Aggressive merging

The deterministic fast-core aggressive replay
`artifacts/bench/core-fast-aggressive-deterministic-release/report.json` matches
all 48 raw and merged counts with three paired repetitions. The full M2
aggregate deliberately uses no merging and does not extend that assertion
coverage to the remaining slow cases. Upstream slow merge assertions include
the physical elliptic, massless elliptic, `test_ex_e5`, and pentabox B/D cases.

The three disabled examples also contain merge assertions: ex38 and ex43 expect
29 cells; ex44 T/C1/B2 expects 28 while its own source comment reports 19.
Projection ex43's stale `[16,4,2,1]` assertion has already been independently
dispositioned: native and Rust both return `[16,2,1,1]`. The separate disabled
aggressive replay under `artifacts/bench/disabled-aggressive-256527b/` completed:
ex38 timed out in Rust, while native returns one cell; ex43 and ex44 match raw
counts but have Rust/native merged counts 20/23 and 20/19. Separate independent
replay verifies both completed Rust proofs. The native merger also has two
reproduced defects and order-dependent successful counts. See
[the upstream merge audit](UPSTREAM_MERGE_AUDIT.md) for exact evidence and explicit
policy dispositions. These are not merged-parity passes.

## Utility assertions

The inventory includes six rational-helper tests and five Python/Cython native
conversion tests. A dedicated Rust mapping/disposition was completed in
`tests/upstream_utilities.rs` and `reference/UPSTREAM_UTILITY_TESTS.md`.
All eleven mapped tests and eighteen root tests passed in the focused checkpoint
bundle; the utility milestone is commit `3c53855`. Backend-specific conversions
have exact Rust analogues or explicit explanations in that disposition.

The upstream reversed-endpoint rational test is an actual defect, independently
confirmed with the native extension: it returns `9/332`, outside the interval
with endpoints `193/7121` and `191/7109`. Ordering those endpoints returns the
valid simplest fraction `1/37`. The Rust correction normalizes endpoint order
and tests interval membership rather than preserving the invalid sample.
Native evidence: `artifacts/upstream-utility-audit-01/native-reversed-interval.json`.

## Search recovery

The existing MCTS tests cover deterministic seed, group legality, clipped worker
deadlines, promotion, heuristic pruning and incomplete status. The 126-solve
cold ordering ablation includes all search costs and finds no MCTS speedup on
that subset; a negative measurement is not a missing test.

Before the new search-journal work, only exact algebra operations and projection
prefixes survived a killed search parent. Scheduler history and the best restart
order were emitted only on normal return. The bounded atomic journal in
`src/search/checkpoint.rs` closes this recovery gap, with exact
context validation, deterministic scheduler replay and fresh solve validation
before an archived completion can become a current winner. This note records
the obligation separately from corpus coverage; [the checkpoint note](SEARCH_CHECKPOINTS.md)
records the mechanism, tests and limits.
