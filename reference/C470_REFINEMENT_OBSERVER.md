# C470: exact observation of local refinement and cache admission

An isolated observational run identifies a concrete source of repeated work:
**148 cache misses followed an observed rejection of a completed proof by the
per-entry node limit.** All occur at reduced level 3. This supports testing
larger individual cached proofs; it does not establish their share of runtime,
an exact solver cycle, or a new decomposition.

The original symbolic C470 input uses the existing x1 scale and order, its
unchanged constraints and kinematics, 256 MiB of completed-proof cache and
64 MiB of learned-cut cache. The fresh warm-cache copy contains 60,376 native
operations and 4,096 support proposals. All remain unchanged after the run;
there are no new records. The ordinary Symbolica backend is unchanged.

## What the observer measures

The isolated observer binds requests to the full exact rational prefix and
sorted polynomial-ID cut set within one append-only registry. It observes
actual cache lookups, admission decisions and evictions without a second
lookup or changing mathematical choices. Parent invocation IDs distinguish
requests across visits. Equal request keys do not imply equal complete solver
states: the permanent witness stocks can differ.

A shared window retains at most 4,096 keys and 16 MiB of charged metadata.
Missing history is explicitly first-seen-or-forgotten, never globally new.
Best-effort, throttled sidecar writes cannot change acceptance or stopping.
The original input uses eager projection; enabled deferred projection would
be counted as outside the detailed request observation.

Five focused tests pass. The same frozen executable also produces identical
non-timing outputs, proof evidence and ordinary counters with observation off
and on for a refinement case and a same-sign interior seam. All four fresh
independent verifications pass. A fixed-refinement stop remains incomplete;
zero-key saturation and an unwritable sidecar preserve exact solve output.

## Bounded original run

The one-CPU (10), 8-GiB worker times out after **60.049951 s**, with
**222,216 KiB** peak RSS and no complete result or verification. The final
retained observer snapshot is at **59.822547 s**, before termination; it is
not marked final. These instrumented development-build timings are outside
the benchmark and establish no solve-time ratio.

| Observation in that snapshot | Count |
| --- | ---: |
| Projection requests | 65,968 |
| Identical exact prefix/cut requests across invocations | 150 |
| Completed-proof admissions | 42,789 |
| Completed-proof cache hits | 32,410 |
| Rejections by per-entry nodes/depth gate | 1,593 |
| Later misses after an observed rejection | 148 |
| Actual completed-proof cache evictions | 5,586 |
| Observed eviction-then-miss chains | 0 |
| Observation-window evictions | 176,327 |

All 150 repeated requests and all 148 post-rejection misses are at level 3.
There are 1,574 size/depth rejections there; seven remaining coordinate levels
cannot exceed the cache's depth limit of 16, so these are node-limit declines.
The current per-entry ceiling is 128 nodes, independent of the total RAM
allowance. No observed key/ID/height/byte admission skips occurred. Peak
charged observer memory is 6,072,410 bytes.

The zero observed eviction/revisit count is weak negative evidence because
the bounded observer forgot substantial history. It does not justify claiming
that eviction never causes repeated work. Likewise, 65,818 requests marked
first-seen-or-forgotten are not proved globally distinct.

Most refinement rounds occur deeper: level 4 records 4,576 rounds / 9,152
requests, and level 5 records 15,453 / 30,906. Those aggregate ratios do not
prove that every individual stack takes exactly two rounds. The observer
records 23,224 discarded direct child proofs; these are neither algebraic
roots nor a count of all discarded descendants. Child-requirement summaries
inspect at most 4,097 requirements per child and are bounded observations,
not exhaustive unique-obligation totals.

## Next controlled intervention

The narrow proposed control raises only the individual proof's node allowance
from 128 to 1,024, retaining the 1-MiB payload limit, total byte/node budgets,
exact prefix keys and every returned obligation. Ordinary verification remains
unchanged. It must measure admission, later hits, payload rejection, eviction
and memory together: admitting larger proofs can displace useful smaller ones.
Increasing aggregate memory alone does not remove the observed entry gate.
No performance benefit follows from this diagnostic alone.

## Executed 1,024-node entry control

A sibling isolated build changes only `MAX_ENTRY_NODES` from 128 to 1,024.
It retains the 16-level depth bound, 1-MiB serialized payload limit, aggregate
byte/node budgets, FIFO policy, exact keys and complete returned obligations.
Four cache tests and five observer tests pass, including a 201-node payload
round trip and rejection above 1,024 nodes. The small exact CLI results and
proofs agree across both variants after removing timings and diagnostic
reports; observation off/on and all four independent verifications also pass.

The variant starts from the identical 60,376-operation / 4,096-proposal warm
snapshot. Its original worker still times out: **60.036595 s**,
**220,780 KiB** peak RSS, no result or verification. The last retained
snapshot is at **59.719498 s**, and is not final.

| Retained snapshot count | 128-node entry | 1,024-node entry |
| --- | ---: | ---: |
| Projection requests | 65,968 | 75,487 |
| Entry nodes/depth rejections | 1,593 | 18 |
| Observed later misses after rejection | 148 | 1 |
| Identical requests across invocations | 150 | 1 |
| Cache admissions | 42,789 | 50,221 |
| Cache hits | 32,410 | 26,487 |
| Actual cache evictions | 5,586 | 20,526 |
| Observer-window evictions | 176,327 | 194,775 |

The admission gate and the corresponding observed repeats largely disappear,
but larger retained proofs increase eviction pressure and change traversal.
Fewer hits are not automatically a regression: one larger hit can replace
many descendant lookups. Conversely, more projection requests within a
deadline are not completion progress or a solve-time speedup. Both snapshots
have bounded, forgotten history, so their zero/unseen categories cannot
establish global absence. No production default changes on this evidence.

Variant evidence is under `artifacts/research/c470-refinement-entry1024-01/`;
its executable hash is
`7567930e5c72d691d8926c233de4c63206c852ea954bc0ac9c9536f794bb38c2`.
The trace hash is
`9bba76cca87fbb0c23f9eca3275c085f92d8b866886e56dae506785c12956c83`;
the final audit is
`2e2f1de888c8e9bacc1e13a1ec2d023bae0b8ed70f36d68a4ea99e921802e416`.
Both cache inventories remain unchanged. All workers have stopped.

## Configurable production admission policy

The optional `solver.local_subtree_cache_entry_nodes` setting now exposes this
resource policy without changing its default of 128. The accepted range is
128 through 4096. It changes only the admission limit of an individual complete
conditional proof. The total byte/node budgets, 16-level depth bound, 1-MiB
payload bound, exact prefix keys, FIFO eviction and full returned obligations
remain unchanged. It has no mathematical role in verification or durable hint
compatibility; valid changes to the policy cannot excuse missing boundaries.

Historical input and saved evidence omissions restore 128. Configuration is
forwarded through homogeneous reduction, recorded in local reports and checked
for malformed values during both solve and replay. Larger entries can displace
smaller useful proofs, so the short diagnostic does not justify changing the
default. Matched ordinary-release measurements are recorded separately below.

## Ordinary-release control at a fixed refinement limit

Both fresh processes use the same release solver, original x1 gauge and order,
unchanged mathematics, and separately audited copies of the 60,376-operation /
4,096-proposal snapshot. Only the per-entry allowance differs. The worker has
one CPU, 8 GiB, a 120-second wall ceiling, 256/64 MiB proof/hint budgets and
an explicit 20,000-refinement limit. Neither uses the diagnostic observer.

| Final recorded quantity | 128 nodes | 1,024 nodes |
| --- | ---: | ---: |
| Process wall seconds | 8.398262 | 6.958375 |
| User / system CPU seconds | 7.196034 / 1.079023 | 6.030100 / 0.828429 |
| Peak RSS, KiB | 129,284 | 128,252 |
| Refinements at the stop | 20,001 | 20,001 |
| Recorded cut occurrences | 28,011 | 28,011 |
| Speculative nodes | 100,422 | 89,680 |
| Completed-proof cache hits | 30,734 | 21,216 |
| Completed-proof cache misses | 41,451 | 40,227 |
| Completed-proof cache evictions | 2,464 | 10,789 |

Both save an **incomplete** result with the exact reason `local projection
refinement limit`, zero final cells, and the same reported frontier
`[6,-24,73/26,1]`. Neither qualifies for independent complete-result verification.
All inherited cache files remain byte-identical, with no new native operation
misses or writes. The source, report and selected entry policy are checked.

The larger entry policy saves time in this one matched activity control while
increasing eviction pressure. Equal refinement counts and the same frontier
are not an exact equality-of-coverage proof. These are warm, single-pair
diagnostics, excluding build and earlier cache-generation costs; they establish
no completed-solve speedup or original/native runtime ratio. The default stays
128. Artifacts are
`artifacts/research/subtree-entry-policy-production-01/controls/`; the release
and test provenance are in [the combined validation record](COMBINED_FAST_VALIDATION.md).

| Record | SHA-256 |
| --- | --- |
| Paired summary | `19b9187e31f801648d763a7c1311a4733a9e7b39564b5f4b5950db361b247450` |
| 128-node process record | `6c3a650ef98c6a7d3c16bade21fb74674ef1cf987956bf815fa867a44ceabb3b` |
| 1,024-node process record | `be14cff2498d6bec630d5b867cd394f0dff640e8845aa045c2f324dc2de62132` |

## Ten-minute ordinary-release continuation

A fresh copy of the same baseline, with the 1024-node entry policy, a
ten-million-refinement limit and a hundred-million-node limit, reaches the
600-second wall deadline. Process time is **600.083640 s**, user/system CPU
535.940393/54.728836 s, and peak RSS **684,980 KiB (669.0 MiB)**. There is no
result or complete-decomposition verification. This remains the original
symbolic input, x1 gauge and order, with one CPU, 8 GiB and no observer.

It saves **1,295 new operations**: 679 factors, 103 discriminants and 513
resultants. The restart snapshot now has **61,671 operation files** and
4,096 support proposals. All inherited files remain byte-identical; new
operation metadata, keys and output hashes pass the integrity audit. This
audit does not independently replay each operation or certify a decomposition.

The last periodic snapshot, at 599.987569 s, records 2,687,069 refinements,
11,888,174 speculative nodes and prefix `[13/3,-13108]`. These are activity
measurements, not covered cells. The completed-proof cache reaches its
262,144-node aggregate cap while using at most 87,112,035 estimated bytes;
it records 5,391,317 entry evictions. Increasing its byte budget alone would
not remove that node limit. The native algebra cache separately records one
**whole-map reset**, not one entry eviction, at its 128-MiB budget; final
estimated retention is 77,479,202 bytes. Its many disk hits can include both
first loads and post-reset reloads. These counters do not establish that more
RAM would finish the problem, and no further cache policy is changed.

The control's shorter activity time has not translated into a completed
original solve. No new completion, native-GCAD ratio or benchmark-table
replacement follows. All workers have stopped; artifacts are under
`artifacts/research/c470-subtree-entry1024-continuation-01/`.

| Record | SHA-256 |
| --- | --- |
| Terminal process record | `1a6ed9411a6da516f7985d9fd1299b873fa361f875d231f4d48f8402a7c31dbb` |
| Final cache integrity audit | `a3a8276c8d056a6e5c1960fa21c8120750dadc39ecca9721298a30b6f3ca8269` |
| Source and binary manifest | `df8476da99513279ac463e50cd8c115e9032fcbddc3a95d8c532c27dd481a96a` |

## Original observer provenance

Evidence is under `artifacts/research/c470-refinement-observer-01/`:

| Artifact | SHA-256 |
| --- | --- |
| Frozen executable | `09087737a88417e6651132a95680162f341e7ce5cb42f310d77925df2df7a276` |
| Observer patch | `c2579b67b7ccee7c94523f7b9ffcfc6c8d02de020d248a723aedcd38d5bfadfa` |
| Retained trace | `0bf7136d18b9f06289cd0d18edcac4cc3bba8e4bea0eba78b92ae14b99f6cccf` |
| Final metadata/cache audit | `a374cc349cd3d0011ef030b663382ce5327a2e8ddccbee50975b115bf555e27f` |
| Artifact report | `e3a116d61e4719714e39f5a178e985919c60f19c5f3bc33b121f56eb0012ad75` |
