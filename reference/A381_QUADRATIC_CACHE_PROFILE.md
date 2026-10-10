# Cached quadratic guards: fixed-work A381 controls

The positive-ray quadratic classifier now evaluates the signed discriminant
from its already computed rational coefficient values and obtains the symbolic
discriminant guard through the existing exact operation cache. Previously it
rebuilt `b²−4ac` symbolically at every visit. The signed numerical value is kept
separate from the canonical guard: canonicalization can reverse a polynomial's
sign while preserving its zero set. The independent checker recomputes the
guard with its uncached exact algebra operations.

Three unit regressions cover rational scaling, a canonical guard whose sign is
opposite to the signed discriminant, rational multivariate coefficients, cache
reuse, independent replay, and rejection of an identically zero discriminant.
The existing quadratic projection comparisons also pass. The coherent
`e8e1cd4` source passed 321 library/integration tests.

## Full original input, bounded equal work

The controls use the original fixed A381 problem
`artifacts/corpus/fixed/0089_finiteA_7_381_10_0__4_0_1_1_1_1_1_0_1_fixed.toml`,
with `mt2=1`, `mz2=5/18`, `s=10`, and `t=-3`. The homogeneous anchor is `x5`;
the order is `[x5,x8,x4,x6,x3,x0,x2]`. Both workers use local projection,
positivity, adjacency and Descartes witnesses. Sibling batching, stable samples,
pair witnesses and interval Descartes are disabled; pair cancellation depth is
one. This prevents the newer interval rule from changing the comparison.

Each process has a 100,000-node cap, one million refinement cap, 120-second
limit and 4 GiB memory cap, on CPU 8 with one worker. Each side begins with an
independent copy of the same 5,223-file, approximately 26 MiB operation-cache
seed. The seed manifest hash is
`de32cdd02673f92a8c216858344010e080a9eb31239203148e5b4279ce392366`.

| Worker | Source | SHA256 |
| --- | --- | --- |
| Old | `a551f36` | `4ab2aecde1bad2a4ff4de2cb8bb6f4b8eac45ba7b679114c0049b85c1ab321ef` |
| New | `e8e1cd4` | `8c0b09b11013c5baf226fdc4832174517fe9136d54f2bca66e4e2b7c31e41dc5` |

Every control stops as **incomplete** at exactly 100,001 speculative nodes,
4,880 refinements and 10,386 refinement cuts, at level 5 and prefix
`[1,16/29,14,1,1]`, with diagnostic `local projection node limit`. Final
structural reports agree after excluding timings, cache counts and new zero
option counters. Inputs agree after removing their checkpoint directory paths.
The 4,731 unique canonical factor-output sets also agree exactly; their sorted
JSON-lines hash is
`0970384174672bc325d3fafbdab917ba5f98c248ed01abaa6534b4ad287a748e`.
An incomplete result does not retain a complete cut-by-cut proof trace, so this
is structural endpoint and factor-output agreement, not verified full-trace
equivalence or a completion of A381.

The new worker records 31,901 discriminant cache hits, 22 misses and 13 disk
hits, versus 1,786 hits, zero misses and 13 disk hits before the formerly
uncounted direct quadratic operations entered this cache. Factor and resultant
counts match: 254,180 factor hits and 481 disk hits; 11,295 resultant hits and
181 disk hits. Neither side reports invalid cache entries or evictions.

## Timing controls and their limits

The first pair used the Rust `hard_probe` harness with operation captures and
workspace progress files. Old/new worker wall times were 61.480/74.449 seconds,
while user plus system CPU times were 60.420/56.152 seconds. This mixed outcome
is preserved. More frequent cached discriminant calls can request diagnostic
context publication when captures are enabled; it is not established that this
accounts for all of the additional wall time.

Two further fresh-seed pairs disable captures and externally supplied progress
paths. They use the public CLI's Rust worker supervisor and its ordinary local
temporary progress file. GNU `time` records frontend-plus-worker resources.

| Pair | Old solver seconds | New solver seconds | Old user + system | New user + system | Old frontend wall | New frontend wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| First | 53.763 | 45.701 | 52.92 + 0.42 | 44.66 + 0.41 | 53.87 | 45.80 |
| Repeat | 55.407 | 47.355 | 52.58 + 0.45 | 45.77 + 0.48 | 55.50 | 63.16 |

The fixed-work solver elapsed times decrease by 15.0% and 14.5%, and CPU time
also decreases. The repeat's new frontend wall time contains approximately
15.8 seconds outside the solver's measured interval, with cause unestablished.
These results support reduced solver work in two debug controls; they do **not**
establish a repeatable end-to-end wall speedup, a release acceptance ratio, or
any additional completed original problem. Peak RSS remains approximately
46 MiB. No additional capture-context gate was added: construction of the
cached operations already disables that diagnostic context without a capture
directory.

Evidence is under `artifacts/profile/a381-quadratic-cache-01/`: `old/`, `new/`,
`no-capture/`, `no-capture-repeat/`, both factor-output inventories and the
independent checkpoint copies. The no-capture folders retain exact TOMLs,
commands, results, stdout, stderr and resource logs. An initial shell `time -v`
keyword error occurred before a worker started; its log is preserved separately
and the measured runs use `command time -v`.

| Result | SHA256 |
| --- | --- |
| First no-capture old | `e87ed08c0dfd024b8c8010816ba2b8b639272c258339707837fb90d2436b7f6f` |
| First no-capture new | `3bfe9ab03825c65d8b4e69bb32cba2569479803b5d94dd1493578d45ad3e8163` |
| Repeat old | `9417a2ecb28992faa118368db70fdc4ec79ac329a425ceb97bec20fa23856f14` |
| Repeat new | `a57717c0b0e96681bb0a070bc0a2a94a3c64251e63e0258d16b3b11f6a7979ca` |
