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
No production change or performance benefit follows from this diagnostic.

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

Evidence is under `artifacts/research/c470-refinement-observer-01/`:

| Artifact | SHA-256 |
| --- | --- |
| Frozen executable | `09087737a88417e6651132a95680162f341e7ce5cb42f310d77925df2df7a276` |
| Observer patch | `c2579b67b7ccee7c94523f7b9ffcfc6c8d02de020d248a723aedcd38d5bfadfa` |
| Retained trace | `0bf7136d18b9f06289cd0d18edcac4cc3bba8e4bea0eba78b92ae14b99f6cccf` |
| Final metadata/cache audit | `a374cc349cd3d0011ef030b663382ce5327a2e8ddccbee50975b115bf555e27f` |
| Artifact report | `e3a116d61e4719714e39f5a178e985919c60f19c5f3bc33b121f56eb0012ad75` |
