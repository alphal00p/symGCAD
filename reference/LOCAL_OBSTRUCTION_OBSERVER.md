# Bounded observation of repeated local guard failures

This opt-in diagnostic measures whether repeated failed visits could justify a
future obstruction-hint cache. It does **not** implement that cache or change
projection, samples, accepted/rejected cells, returned guards, learned cuts or
completed-subtree reuse. Its data is not proof evidence and the verifier does
not consult it.

## Enabling and retaining evidence

Library callers can set `LocalOptions.observe_obstructions=true`. Historical
options and the default retain `false`, omitted from serialized options.
For a supervised CLI worker, set `SYMGCAD_OBSTRUCTION_REPORT` to a dedicated
output JSON path. This enables the same observer without adding a solver option
or changing the prepared mathematical problem. Each worker needs a separate
path; use a fixed-order single solve for the first measurement.

When enabled, ordinary `LocalReport` snapshots contain an
`obstruction_observer` object. The dedicated file is written with temporary-file
replacement at the existing progress checkpoints, at most twice per second
unless a checkpoint is forced. A normal exit also attempts a final snapshot
from the observer's destructor, including unsupported/fallback exits. A killed
worker can leave only its last complete checkpoint. `final_snapshot=true`
means that an observer finalized; it never means that a solve completed or was
verified. The JSON's process ID and Builder scope identify a single registry.
Subsequent Builders using the same path replace that report; their counters
must not be mistaken for one accumulated traversal.

Diagnostic I/O failures are counted and never propagated into the mathematical
solve result. No path means no diagnostic file or destructor I/O. All overhead
still consumes ordinary worker time and memory; a timed run may therefore
reach its resource limit earlier. The disabled visit path does not allocate or
hash keys.

## Exact event and retention semantics

The wrapper observes an existing `Stop::Guard(id)` after a visit has restored
its incoming rational prefix. The ID references a nonzero canonical polynomial
in that Builder's append-only registry, whose highest variable is strictly
earlier than the failed prefix. The observer records the already returned
failure; it neither reevaluates the polynomial nor derives a new guard.

The key is **the full exact rational prefix and polynomial ID**. Hash-table
lookup checks full rational equality, so a hash collision cannot equate keys.
Different polynomial IDs at the same prefix remain separate. The owner axis
is the guard's highest variable. Numeric axis indices use the local normalized
lifting map, which must be recovered from the saved worker input/proof context.
IDs are never reusable across Builder scopes or solver invocations.

Every returning visit is counted, including propagation of one deeper failure
through several ancestors. Thus `failed_visits` does not count independent
originating failures. For each owner axis and overall:

- `repeated_while_retained` counts exact matches in the current FIFO window.
- `first_seen_or_evicted` counts admissions, including a previously evicted key
  returning later. It is **not** a lifetime distinct-key count.
- `retained_keys`, `evictions` and charged retained/peak bytes describe that
  window. Fixed first-admission samples record exact prefixes and local IDs;
  further matching events update only their counters.
- Limit declines and invalid-owner diagnostics are explicit. The observer
  never interprets a missing record as a new mathematical obstruction.

The fixed limits are 4,096 retained keys, 1 MiB of conservative key/entry
charges, 64 prefix coordinates and 16,384 total numerator/denominator bits per
prefix. At most 32 samples use a separate 64 KiB charge budget; their prefixes
must have at most 1,024 bits before rational text is formed. Owner counters have
at most 64 entries. Container capacity and fixed report metadata also have
finite structural bounds; charged bytes are not a measurement of process RSS.
FIFO updates never append access records on a repeat.

## What the first measurement can establish

Use the existing A414 fixed-work input, exact build/order/options, cold algebra
cache and node/refinement caps. Compare observer on/off proof content and the
completed operation set, excluding only named diagnostic fields and timings.
Then report repeat frequency by owner, repeat samples, retention/eviction rates
and instrumentation cost. Resource-stopped results remain incomplete.

Many exact repeated failed visits would establish an opportunity for further
study, not a predicted speedup. Existing learned cuts may already prevent most
useful revisits. A remembered guard could also preserve an unnecessary seam
after a different projection choice. The proposed hint would need a fresh
exact vanishing check and ordinary ancestor refinement before it could ever be
used. No such hint is returned here.

Discarded descendant-work accounting is intentionally absent: summing work
over propagated failures double counts nested unwinds. This observer also does
not measure whether a historical guard would become unnecessary on a later
successful attempt, nor the cost of a hypothetical cache hit. Those questions
require additional evidence before implementing any failure-hint cache.

Focused regressions cover exact keys, FIFO re-admission, byte/prefix/sample
bounds, atomic final snapshots, nonfatal I/O errors, default-off compatibility,
incomplete-result preservation, and eager/deferred observer-on/off exact proof
and operation-record equality. No completion or performance gain is claimed.

On 10 October 2026, all four module tests and both integration tests passed.
The latter compare complete independently verified proofs, non-timing counters,
operation records and cold hint archive contents. The diagnostic flag is
explicitly removed from the archive's context input, so enabling it does not
select another hint namespace. The tests mutate neither process environment
nor another worker's diagnostic paths.

Commands ran with one Cargo job on CPU2 and a 4 GiB address-space cap, inside
the existing Nix development environment:

```sh
cargo test -j1 --lib obstruction_observer -- --test-threads=1
cargo test -j1 --test local_obstruction_observer -- --test-threads=1
```

Retained logs are `artifacts/obstruction-observer-unit-tests.log` and
`artifacts/obstruction-observer-integration-tests.log`. The destructor can add
one last successful-write count after the returned report was copied, so the
final sidecar and returned snapshot need not be byte-identical. Neither grants
completion credit.

## A414 fixed-work observation, 10 October 2026

One observer-off/on pair used the original A414 mathematical problem and the
existing fixed order `[s,t,x8,x1,x2,x3,x4,x7]`, with homogeneous anchor `x8`,
stable samples and deferred projection. Both arms used fresh algebra caches,
CPU9, a 4 GiB worker limit, 90 seconds internally and a 120-second outer
deadline. The node limit was 100,000 and the refinement limit 20 million.
Relative to `artifacts/profile/a414-fixed-work-release-01/baseline/input.toml`,
only the two resource limits and fresh checkpoint path changed. The observer
environment variable was the sole algorithm-run difference between arms.

The current debug CLI was copied without rebuilding to
`artifacts/bin/symgcad-obstruction-observer-59257d0-debug`, SHA-256
`4e672059c151d86f267659ef548b22ff7e6f302665d94dc9acc9e72791684787`.
The source commit, complete production-source hash manifest and then-current
corpus-parser patch are retained in
`artifacts/profile/a414-obstruction-observer-debug-01/provenance.json`.
The run script and exact comparison script are beside it.

Both solves returned **incomplete: local projection node limit**, with exactly
100,001 visited nodes, 24,636 refinements and 57,426 added cuts. Their entire
saved results agree after removing only named timing fields, checkpoint paths
and the observer report. All 212 completed operation records also agree after
removing elapsed time. The common non-timing result SHA-256 is
`26efc546db2a4db3964132ed668cf19f2906b8554a88061058af1f392161bbb2`.
No complete proof or new original-problem completion resulted.

The final observer report recorded **zero failed `Stop::Guard` visits**:
zero admissions, repeats, retained keys, evictions or owner samples. It wrote
88 snapshots successfully with no I/O failures. Thus the 24,636 refinements in
this measured traversal prefix occurred without failed guard-return visits;
the visitor instead accumulates obligations from successful conditional child
proofs. This is negative evidence for the proposed obstruction-hint cache on
this prefix. It does not characterize later, unvisited parts of the problem.

Process wall times were 44.00 seconds off and 43.66 seconds on; peak RSS was
46,140 and 49,224 KiB respectively. These are one sequential debug pair, with
fresh algebra caches but no filesystem-page-cache reset, and support no speed
claim. `comparison.json` retains both input/result hashes, exact comparison
results, all diagnostic counters and the final observer-file hash.
