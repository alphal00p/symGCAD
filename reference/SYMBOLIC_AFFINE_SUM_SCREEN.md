# Bounded symbolic ZZ screen after exact sum preconditioning

A fresh completion-oriented screen tried 26 of the 27 remaining original
symbolic ZZ problems with the coherent `b8139e0` debug worker. A446 was excluded
because it had a separate priority campaign. **All 26 attempts timed out at
20 seconds; this screen adds zero verified original completions.** No result
reached verification, so these are resource failures rather than mathematical
verification rejections.

The screen retained the previous adjacent-release campaign's original
mathematical inputs, legal Brown ordering policy, homogeneous blocks and
anchors. Its prior inputs had no explicit fixed order; the variable groups
remain unchanged. The new rules enable Descartes witnesses, interval Descartes
and positive pair witnesses, with adjacent pairs, unbatched siblings, one pair
cancellation and stable samples disabled. The compiled worker includes the
exact affine-square certificate and sum-dependent factor preconditioner, and
passed the coherent 341-test library/integration suite before freezing.

Two disjoint 13-case shards ran concurrently on CPUs 8 and 9. Each child had
one worker, 4 GiB, a 20-second solve limit and a 60-second verification budget
for any completion. Each shard had a 580-second campaign limit, with an outer
590-second guard. The existing Rust campaign/worker supervisor enforced the
limits, saved every attempt, and would have independently verified completed
results. Both shard supervisors exited successfully in approximately 260.370
seconds. Summed solve wall time was 520.701 seconds; maximum observed process
group RSS was 143,596 KiB. The ledgers preserve 95 corpus aliases for the 26
distinct original problems.

This is a completion screen with changed proof rules, warm caches, a debug
worker and shorter limits. It is **not a timing comparison** with the older
release campaign. Nested public solve supervisors may omit killed descendant
CPU time from their `wait4` totals, so those fields are not used to infer
algebra cost.

## Input and cache provenance

`artifacts/symbolic-affine-sum-debug-01/selected.json` records each original
input, its SHA256 and mathematical identity, corpus aliases, and prior prepared
input/hash. Preparation checked both original and prior input hashes before
copying the TOMLs. The two shards started from independent copies of the prior
deterministic operation cache: 15,060 files, approximately 101 MiB per copy.
Every file in both copies was checked against the same sorted seed manifest.
Subsequent writes remain isolated by shard. Cache hits still pass the ordinary
schema, key, map and checksum validation; proof verification never treats a
cache entry as authority.

The retained `prepare.sh` and `run.sh` contain exact preparation and launch
commands. `preparation-hashes.sha256`, `run-provenance.json`, both campaign
TOMLs, input copies, ledgers, stdout/stderr, process outcomes and the aggregate
summary remain in that artifact directory. No production code was added for
the screen.

| Artifact | SHA256 |
| --- | --- |
| Immutable `b8139e0` debug worker | `e24b80cd8a8d69bb361b7ed08737393e8fcd8161eefa69d0bc3e1b3b44bdb6e7` |
| Initial cache manifest | `9cf0c8ab9a76e39c310bdd8057f2edd30c6214fc0a570e3693ce1e2c29be5d10` |
| Shard 0 ledger | `84f978486c081242cc613557d8ceaeea4c5c845f267883c164da5b6be8dafddc` |
| Shard 1 ledger | `d931ace14caa450e8e87924e2e023b47922ae14eeb3503981068d4c573fff255` |
| Run provenance | `05f10a65ab1a36246fa983600c873f796aca6613a0437b74c78bbb6da71042c4` |
| Summary | `469a1c6d079f0930581ba024152b80a43224cdbf45728796d9df18fe258037c8` |
