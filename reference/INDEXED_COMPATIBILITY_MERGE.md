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
`artifacts/bench/disabled-aggressive-256527b/`. Fresh indexed release timing
and independent verification are required before claiming that the large
merge timeout has been resolved.
