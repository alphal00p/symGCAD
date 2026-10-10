# Indexed compatibility merging

The previous Rust aggressive merge searched every cell pair and every axis
again after each successful merge, then removed an element from a vector.
This can require cubic work in the raw cell count. In the new disabled-case
compatibility campaign, `ex38` has 131,256 raw cells; all three old release
workers timed out at 120 seconds with merging enabled. The native engine
completed with one merged cell. This exposed a missing large-merge workload
in the earlier 48-case fast merge comparison.

`src/solver/merging.rs` now indexes finite lower and upper faces. A face key
contains the axis, split signs, the other axes' two boundary identities and
the candidate seam. Boundary identity includes the polynomial ID, root
ordinal and root-index domain. Sample-specific isolating intervals are
evidence attached to a root, not its identity.

## Preserving the previous Rust policy

This change preserves the exact previous Rust greedy result, including
samples, retained boundary evidence and output order. It does not switch
historical proofs to a new merge schedule.

Give each raw cell a stable original ID. The old algorithm always kept cell
`i` and removed `j>i`; consequently live vector order is the order of these
IDs after every operation. The smallest live `(i,j,axis)` tuple is therefore
exactly the old nested scan's next merge. A sorted candidate set provides
that tuple. Face lookup enumerates all potentially adjacent pairs. Removing
old faces and reinserting a changed survivor discovers every newly possible
merge; unaffected pairs retain their candidates. Stale candidates are
discarded only after checking their current cells and boundaries. The
forward seam still takes precedence over the backward seam.

Raw cells are retained as the certified decomposition. The optional merged
presentation still carries the raw cells and trace needed to recover
excluded seams, including interior singular sheets. Independent verification
reconstructs the same merge from the verified raw cells.

## Tests and remaining compatibility work

The former scan remains in tests as an independent algorithmic oracle.
Tests compare complete cells, not just counts, for every subset of a 2×2×2
grid under deterministic permutations and mixed split signs; shuffled grids
through four dimensions; infinite endpoints; all root-index domains; and
differing sample-specific intervals. A 4,096-cell grid checks the large
adjacency path. All four tests passed in the 126-test library bundle in
`artifacts/search-checkpoint-focused-tests-03.log`.

The same disabled-case campaign also found order-dependent merge-count
differences for `ex43` and `ex44_T_C1_B2`: Rust returned 20 in each case,
whereas native returned 23 and 19. Raw counts matched at 66 and 224.
Upstream uses a different stack-based greedy schedule. Its disabled source
assertions expect 29 and 28 respectively, so neither assertion can be treated
as an independently established mathematical requirement. The indexed change
does not claim to resolve these discrepancies; their disposition and the
upstream merge bookkeeping are under separate audit. The historical raw-cell
M2 parity cohort is unchanged.

The pre-change comparison and all repetitions are retained in
`artifacts/bench/disabled-aggressive-256527b/`.

## Matched release result

The immutable `dda5980` release executable has SHA-256
`f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`.
Three fresh paired repetitions used the same original inputs, orders, CPU 6,
120-second worker deadlines and 16 GiB limits as the preceding campaign.
`ex38` now completes every repetition, with 131,256 raw cells and one merged
cell. Median Rust/native wall times are 40.7161/26.4997 seconds; matched solver
phases are 38.4195/25.7283 seconds. The Rust merge itself takes a median
4.7724 seconds, versus 7.4507 native. Peak Rust RSS is 1,692,724 KiB. Its full
raw proof and compatibility reconstruction independently verify.

The large-merge timeout is therefore resolved. It is not an overall speedup
over native for this case: Rust's matched total remains 1.4933 times native,
principally because lifting is slower. The two smaller cases still give the
same historical Rust merged counts, 20/20 versus native 23/19, and their fresh
Rust proofs also independently verify. Calling native merge on those exact
Rust raw inventories reproduces 23/19, isolating the policy difference;
see [the independent upstream audit](UPSTREAM_MERGE_AUDIT.md).

The frozen report remains `parity_passed=false` because of those declared
merged-count differences. It does not change the separate successful M2 raw
cohort. Evidence:

- `artifacts/bench/disabled-aggressive-dda5980/results/report.json`, SHA-256
  `6356a213e66659f1db54ce71567048bfdf9ce8f5df06333d2921af1664d30957`.
- The comparison, all per-run merge times and additional verification records
  are in `artifacts/bench/disabled-aggressive-dda5980/comparison.json`, SHA-256
  `3fd656eeb25b46256481e0388ccbfaf1c236f45f0ead772371f2fb3f20822f1f`.
- The complete production library/integration suite passed 456 tests in 30
  targets, with zero failures or ignored tests, in
  `artifacts/checkpoint-indexed-merge-full-tests-02.log`. The initial run's
  checkpoint test setup timeout is preserved; the test now grants the full
  five-second context setup allowance while short-deadline tests stay strict.
