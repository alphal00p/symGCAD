# A414 completed-subtree CPU profile

A 60-second replay of the original symbolic A414 problem attributes **4.52%**
of sampled CPU cycles to completed-subtree retrieval and **2.38%** to insertion.
The cache serialization cost is measurable, but projection, root isolation and
prefix evaluation account for larger overlapping call paths. This profile does
not establish a benefit from changing cache storage or increasing its node cap.

The replay returned **Incomplete, zero cells**, after 60.020663771 seconds.
No independent verification or original-problem completion is claimed.

## Workload and limits

The original is
`artifacts/corpus/symbolic/0085_finiteA_6_414_10_0__0_5_1_1_1_0_0_1_1_symbolic.toml`.
Its complete mathematical TOML prefix, including parameters, constraints, split
and variable groups, matches the prepared input byte for byte. In particular,
the domain remains `s > 4`, unrestricted `t`, and positive Feynman variables.

The replay preserves the prior pilot's local engine and given order
`[s,t,x8,x1,x2,x3,x4,x7]`, with homogeneous anchor `x8` in the full positive
Feynman block. Deferred projection, stable samples, adjacent pairs, Descartes
witnesses and intervals, positive pair witnesses, positivity, and the existing
cache options remain unchanged. The interval template limit remains two.
The only input changes are the wall limit, from 300 to 60 seconds, and a fresh
empty checkpoint namespace. This is a cold replay, not a warm-cache continuation.

The frozen release executable is
`artifacts/bin/symgcad-local-deferred-dec27e8-release`. It predates the
[primitive-linear persistence bypass](PRIMITIVE_LINEAR_CACHE.md).
The algebra worker ran exclusively on CPU0, with one worker and an observed
8 GiB address-space limit. Operation captures were enabled. The launcher began
at 2026-10-10 09:16:01 UTC and returned at 09:17:04 UTC. Its 90-second outer
watchdog bounded profiling and CLI cleanup; the solver itself enforced 60 seconds.

## Attribution

`perf record -F 99 --call-graph dwarf,8192` recorded **5,640 samples**, with
**zero lost samples**, from `cpu/cycles/Pu`. Reports use `--no-inline` to avoid
unbounded external inline-symbol resolution. The following inclusive rows
overlap and must not be added as independent costs.

| Call path | Inclusive cycles | Self cycles |
| --- | ---: | ---: |
| `Builder::project` | 49.51% | 0.29% |
| `roots::isolate_union` | 27.81% | 0.20% |
| `local_projection::prefix_value` | 16.55% | 0.61% |
| Integer prefix evaluator | 16.10% | 9.32% |
| `CompletedSubtrees::get` | 4.52% | 0.05% |
| `CompletedSubtrees::insert` | 2.38% | 0.19% |

The retrieval call tree attributes 4.09% of all sampled cycles to its
`LocalNode` JSON deserialization path. Insertion includes 1.68% in the bounded
JSON serializer's `LocalNode` field. Thus the counters identify real serialization
work, rather than merely frequent cheap lookup calls. They do not measure its
wall-time share or predict the effects of another representation.

The last progress snapshot, at 59.980390443 seconds, records 324,536 speculative
nodes, 47,736 refinements, 78,169 subtree hits, 135,030 misses and 127,496
evictions. The cache reached its **32,768-node cap** while peak estimated payload
bytes were **13,624,005**, below its 32 MiB byte budget. Increasing the node cap
might change hit rates and memory use; neither outcome was tested here.

The final operation capture is a **completed** factorization taking
0.000214001 seconds. It does not identify a pending backend operation at the
deadline. The result retains the final local progress snapshot and reports the
current prefix `[5,-393217,1,46/51]` in the reduced order.

This is one profiled 60-second prefix of a continuing search. There is no
cache-disabled, alternative-storage or unprofiled matched control. CPU-cycle
sampling omits time blocked on diagnostic or checkpoint I/O; capture overhead
and profiler overhead are present. No end-to-end speedup, native comparison,
or claim about the entire earlier 300-second traversal follows from these data.

## Retained evidence

The ignored directory `artifacts/profile/a414-subtree-cpu-01/` contains the
prepared and source TOMLs, worker input and resource observations, launcher,
raw perf data, inclusive/self reports, compact symbol tables, completed capture,
incomplete result, and a machine-readable `report.json`. Large artifacts are
not committed.

| Item | SHA-256 |
| --- | --- |
| Original TOML | `4feff5d7177f5b1b92d523c808373aa91866099221a860c8d1653ecb9a95049f` |
| Prior pilot input | `3c4158ffa79144832ce838d216c85369ef45dfe7fc34e1a809d609f5e9c787ba` |
| Replay input | `6387bc86b84ffb9a4bf9f6a915c56fab883a67e451b60788dfc265a42633e5c1` |
| Frozen executable | `ab98822a1c0bb02129cfb20b26355ee7e415ded6b1bea00e3997dd087697bb9b` |
| Result | `157f040eebd4d8f1481d2e9a7d3f206617429d48c746931cb739834a06e538bd` |
| Machine-readable report | `9473fc37a2333e435090ed7fddd79eb8c108dc6830adc75b8a133a5ba5e1ba43` |
| Raw perf data | `08b87a1f57756fbc75401f6d29b55f9a4e2a8bda12c9de87e3ddf60ca71c706e` |
| Inclusive report | `b0b771474fda9191980bae6328ed67bc0ca1deeaaf0912029739025d657a13ee` |
| Self report | `6aa7f1575bf2cb708cd3b149e622da1534018a59b3b79256075e8a18c8123b8a` |
| Final completed capture | `6db78bf7bb50d668227436986bece0734999b72fca8b1a79d9c3817ac08e3e6c` |

To regenerate the textual attribution from retained data:

```sh
nix develop -c taskset -c 0 timeout 20s perf report --stdio --no-inline \
  --children --sort symbol --percent-limit 0.5 \
  -i artifacts/profile/a414-subtree-cpu-01/perf.data
```

Use `--no-children` for self attribution. The saved `run.sh` records the original
replay command and refuses to overwrite its checkpoint directory or perf data.
