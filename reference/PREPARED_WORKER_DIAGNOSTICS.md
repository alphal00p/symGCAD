# Direct prepared-problem worker diagnostics

`examples/probe_prepared.rs` runs an arbitrary prepared TOML problem through
the hidden solve worker under the existing Rust process-group supervisor.
It preserves every mathematical field, the variable groups, the selected
order and all proof/discovery options. It changes only the requested worker
limits, optional explicit local node/refinement caps and a fresh cold
checkpoint directory. It does not select or force a homogeneous anchor.

Each fresh output directory retains the original TOML, effective TOML and
JSON, input/worker/helper hashes, direct `ProcessOutcome`, external local
and lifting progress paths, operation captures and checkpoints. Compact
progress snapshots are appended once per second and once at worker exit.
A killed worker therefore leaves useful progress even if it cannot export
a complete or partial result. The solve deadline is a hard worker deadline;
the last snapshot can precede that deadline by the normal publication
interval. A stale completed operation capture is not labeled a current
algebraic stall.

A successful complete stdout summary triggers a separate bounded verifier.
Proof hashes are streamed before and after verification, and the verifier's
cell count must match the successful solve summary. Timeout, memory-limit,
failed or truncated worker output never receives completion credit. The
supervisor does not deserialize proof geometry or future proof variants.
An optional preceding discovery report is hashed and its cost recorded
separately from the solve. The whole diagnostic campaign is capped at one
hour, at most 64 GiB and one pinned worker.

Three non-algebra tests check exact mathematical/order/option preservation,
resource bounds/fresh output rejection, and rejection of a completion
string from a killed worker. A direct local CLI smoke test solved and
independently verified two cells for `x>0` split by `x-1`.

## Positive Gluza30: persistent traversal rather than one hard resultant

The prepared original positive Gluza30 input comes from
`artifacts/research/gluza-bezout-hints-01/gluza30/problem.toml`. It retains
the full original mathematical problem, order
`[x2,x6,x7,x5,x1,x4,x0]`, and all 49 exact strict Bézout proposals.
Their discovery took 0.509998024 seconds internally and is not included
in the solve figures below. Both diagnostic runs used the same frozen
debug worker, CPU8, an 8-GiB hard worker limit, a 120-second solve limit,
one million refinements, ten million nodes and fresh cold operation caches.

| Measurement | Default caches, 32/8 MiB | Larger caches, 256/64 MiB |
| --- | ---: | ---: |
| Worker result | Timeout | Timeout |
| Supervised wall seconds | 120.035 | 120.025 |
| Last local snapshot seconds | 120.000 | 119.513 |
| Peak RSS KiB | 64,504 | 146,956 |
| Speculative nodes | 387,691 | 377,611 |
| Refinements | 70,484 | 68,056 |
| Strict Bézout resultant omissions | 17,356 | 16,862 |
| Remaining pair projections | 18,454 | 20,629 |
| Factor hits / misses | 677,181 / 422 | 659,797 / 422 |
| Resultant hits / misses | 18,188 / 266 | 20,363 / 266 |
| Discriminant misses | 30 | 30 |
| Subtree cache hits / evictions | 58,391 / 168,673 | 59,425 / 133,849 |
| Learned-cut cache hits / evictions | 1,289 / 64,211 | 935 / 33,395 |
| Retained subtree bytes | 12,497,303 | 98,370,511 |
| Retained learned-cut bytes | 3,568,295 | 28,480,445 |

The two effective input JSON documents match exactly after removing only
the two cache-budget fields and their necessarily distinct checkpoint
paths. Their 718 completed operation-checkpoint keys also match exactly.
Thus these runs reach the same recorded operation inventory; repeated
cheap work and speculative traversal remain substantial. This does not
establish how much of the requested domain either partial run covers.

In both runs the subtree cache reaches its node cap before its byte cap:
32,768 versus 262,144 retained nodes. The learned-cut cache reaches its
entry cap: 4,096 versus 32,768 entries. Larger limits reduce evictions and
increase memory, but neither run completes. Different retained hints can
change the speculative trajectory, so smaller node/refinement totals do
not by themselves establish better progress or a solver speedup.

The final phase measurements are similar: factor work 14.32/13.68 seconds,
root isolation 7.51/7.56 seconds and resultants 0.815/0.826 seconds, with
roughly 26 seconds across all recorded algebra phases. The operation cache
holds about 2.49 MB and has no evictions. The final saved capture is an
already completed small factor operation, not a pending long resultant.
Prefixes keep changing throughout the history.

An eight-second attached `perf` sample in the first run recorded 784
samples at 99 Hz. Leading individual symbols included memory copying
(6.11%), the integer prefix evaluator (3.89%) and polynomial degree queries
(3.63%); debug runtime checks, hashing and bounded subtree serialization
also appeared. No single resultant kernel dominated. This sample adds
profiling overhead to the first run, and both runs use a debug executable.
The measurements support a traversal/cache-churn diagnosis, not a release
speed ratio or a new completion. They do not justify another memory-cap
increase on their own.

## Retained artifacts

The supervisor and its smoke test are under
`artifacts/research/prepared-probe-01/`:

- Helper SHA-256:
  `97db7507b26181634be58d943dcc08aa931129be29faab98f8b9cd10abc257f4`.
- Helper source SHA-256:
  `dad7c5cd560e41031a539646702ba3a229b809e0598dc5d95629324f2d530278`.
- Frozen worker SHA-256:
  `607dd85fd0aad160da25f6f947d90d450033f34405bf59d7e8069a0b21a774cc`.
- Default-cache report, `gluza30-strict/report.json`:
  `92f584837fc1a6b6876298a6e13796bb27a0c74f2baba8904e5c30d893c9e85d`.
- Larger-cache report, `gluza30-large-cache/report.json`:
  `e04f9f8dc232af3b249648f159dcef492bf95fa0603b71209713a1cdc57de473`.
- `comparison.json`:
  `a8df4a7c4985c438d2a0852529f88fa95d7b932939df85bf2074c3583cfeffb3`.

The per-run progress JSON, JSONL history, captures, original/effective
inputs and checkpoint key lists are retained alongside these reports.
Neither run produced an independently verified decomposition, and neither
changes the original-corpus completion count.
