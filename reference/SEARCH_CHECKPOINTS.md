# Durable bounded ordering search

`search_checkpointed` and the MCTS CLI retain a trial journal in
`checkpoint_dir/searches/<context-sha256>.json`. The ordinary `search` API
remains available without persistence. These files contain search hints, not
decomposition certificates.

The journal records completed scheduler iteration boundaries, trial requests,
outcomes and earlier revalidation attempts. Deterministic replay from the seed
reconstructs RNG state, tree visits/rewards, duplicate-order visits, promotion
counters and abandoned-prefix predictions. Each replayed request must equal the
request the scheduler would generate. A killed in-flight callback has no journal
entry and is repeated on restart. Timings from earlier invocations remain
historical measurements, not credits against the new wall budget.

The context binds the original problem and proof options, ordered normalized
positive/split polynomials, actual variable groups, heuristic seeds/priorities,
backend version, exact executable SHA-256 and scheduler-policy version. Output
and checkpoint paths and invocation time limits are excluded. Normalization and
executable hashing run in the killable seed worker; if that worker cannot return
a complete context, the CLI reports why journal reuse is disabled. Changing the
executable or mathematical context creates a separate journal.

Archive checks include a canonical checksum, schema/context equality, legal
orders, finite nonnegative metrics, valid tiers/deadlines, and deterministic
request replay. A checksum is not authentication: even a coherently forged
completion claim can only influence the search heuristic. Restored `completed`
statuses become `unverified` in the public report. Each historical candidate
must complete a fresh bounded exact solve before becoming the current winner;
these revalidation calls are reported separately. A timeout, failed worker or
partial result cannot satisfy this check. The selected final solve still emits
its complete proof, which is verified through the ordinary independent kernel.

Persistence is capped at 64 variables, 4,096 scheduler iterations/records,
1 MiB of caller context and an 8 MiB regular file. Diagnostics are truncated to
4 KiB before cloning. Symlinks at the archive file are rejected; bounded reads
also catch file growth. Publication writes a fresh exclusive temporary file,
syncs it, atomically renames it, then syncs the directory. An interrupted write
leaves the previous checkpoint valid. Corrupt, oversized or incompatible data
are ignored with a reported reason. Write failures retain the previous journal
and disable further writes for that invocation; they do not change solver truth.

The current invocation's budget includes loading, replay, I/O and revalidation.
Each worker deadline is clipped to the remaining budget. Scheduler iterations
remain bounded cumulatively by `max_rollouts`; revalidation calls also consume
the current invocation's trial budget. Completed exact operations in the
existing algebra cache remain separately reusable. Increasing a scheduler
rollout budget can extend the journal; lowering it below the saved count rejects
that journal and starts a fresh bounded search. Filesystem operations have
bounded input sizes; a supervising process remains the hard cancellation
boundary for a blocked syscall or an individual algebra call.

`tests/search_checkpoint.rs` includes an actual subprocess kill during the fifth
worker callback and compares all resumed order/prefix/tier decisions with an
uninterrupted scheduler. It also covers recomputed-checksum completion forgery,
context/version/scheduler tampering, truncation and oversized files, symlinks,
callback failure, failed publication, callback limits, malformed caller groups,
and public CLI resume followed by independent verification of a fresh result.

The initial focused bundle passed 51 tests (10 existing search, 10 checkpoint,
2 CLI, 18 root and 11 upstream-utility tests), recorded in
`artifacts/search-checkpoint-focused-tests-01.log`. The combined follow-up passed
151 tests, including all 126 library tests and the public CLI restart regression
(`artifacts/search-checkpoint-focused-tests-03.log`). The final regression also
checks that failed revalidation preserves the fastest historical order only as
a restart hint: all 12 checkpoint, 10 existing search and 2 CLI tests passed in
`artifacts/search-checkpoint-focused-tests-04.log`. The earlier `02` log contains
only subsequently fixed borrowing errors in an unrelated research example.
No performance speedup is inferred from recovery correctness.
