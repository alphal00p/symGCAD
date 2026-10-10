# Fixed A381 traversal profile and stable sample experiment

The current fixed A381 obstruction is not always a large algebra operation.
The 300-second Descartes-witness run under
`artifacts/hard-campaigns/a381-descartes-witness-debug-01/initial-x5-0/` visited
413,951 speculative nodes and performed 46,255 refinements. Its last report
contains 1,000,481 factor-cache hits versus 879 misses, 3.68 seconds in resultants
and 0.29 seconds in discriminants. It did not finish.

## Full-input profile

A separate immutable debug replay used that complete input, with the time limit
reduced to 60 seconds and memory to 4 GiB. It kept the original Descartes option
and disabled pair witnesses, used the existing warm operation checkpoints, and
did not enable operation captures. CPU 8 ran the worker under an external
70-second deadline. `perf` sampled at 99 Hz with 16 KiB user call chains, capturing
5,799 samples with no lost samples.

| Inclusive frame | Samples |
| --- | ---: |
| `prefix_value` | 26.66% |
| Symbolica `replace_all` | 26.35% |
| `ProjectionCache::factor_basis` | 13.84% |
| `guard_nonzero` | 12.10% |
| Polynomial hashing | 11.56% |
| Local subtree JSON serialization | 6.40% |
| `stack_roots` | 6.32% |

These frames overlap and must not be added. Finite call-chain depth also limits
attribution to high callers. The run returned `incomplete` at 60.015 seconds,
after 113,122 speculative nodes and 5,793 refinements. Reported phase costs were
8.59 seconds for factors, 4.11 for root isolation, 3.87 for pair separation,
1.51 for root classification, 0.75 for resultants and 0.05 for discriminants.
It had 7,550 completed-subtree cache hits and 47,801 misses.

The profile also exposed an unmeasured algebra cost inside quadratic positive-ray
classification. That branch directly rebuilds the symbolic expression `b²−4ac`
on every visit, bypassing the operation cache and its discriminant timer. Of
914 sampled call chains containing Symbolica `mul_dense`, 729 contain
`positive_projection_raw` without `pair_cancellation`; 184 contain the pair
cancellation helper and one is otherwise attributed. The direct classifier
chains therefore account for about 12.6% of all samples. The bounded next
experiment will use the exact numeric coefficient values to classify the
signed discriminant and the existing cache to supply its symbolic zero-set
guard. `multiplication-stacks.txt` retains this attribution; no improvement
from that separate change is claimed yet.

The supervised `perf` command used 61.37 wall seconds, 59.18 user and 1.36 system
CPU seconds, and 111,512 KiB peak RSS. Those resource figures include profiling;
this is a diagnostic, not a timing acceptance benchmark.

Evidence resides in `artifacts/profile/a381-descartes-traversal-01/`:

| Artifact | SHA256 |
| --- | --- |
| Frozen debug executable | `2983d283fac1dd5109e5381409c2552402645114fadc0c7ba0eeb9c495478b0c` |
| `input.json` | `a512b1e589cab91c09c2bb36b4b70397966307ba59e7cb10cba7823c843f1ae4` |
| `perf.data` | `4352c99e52df2b7dec55d1cccb091892c59bfdc9b0a0bdb62dc9fc2860e8c0b7` |
| `progress.json` | `fb47060e019817d0884d82c190220f6a615e959e96e1daa8f918c84cb88d8c26` |

`report.txt`, `result.json`, `resources.txt`, and the command recorded in the
resource log provide the complete retained diagnostic.

## Stable samples without reusing incomplete proofs

The local visitor rebuilds a root union after discovering new cuts. Even an
unchanged mathematical sector can acquire a different rational sample when
its root isolation intervals tighten. That loses exact-prefix subtree-cache
and learned-cut reuse. A tightened subsector can likewise still contain an
already visited sample whose conditional child proof is useful.

The experimental `solver.local_stable_samples` option retains rationals only
after they have actually been visited, and only during one stack invocation at
one fixed ancestor prefix. It checks strict membership against the new exact
root intervals before choosing a retained rational. Point roots are compared
directly. For an open isolating interval, a polynomial zero strictly inside it
identifies the target root; a zero on an endpoint can belong to another root and
must be interpreted using the open endpoint first. Bounded exact bisection
settles ambiguous comparisons; exhaustion falls back to a fresh sample.

There is no stale child-proof bypass: every selected sample passes through the
ordinary visitor. Existing completed-proof reuse still returns all obligations,
and learned cuts still undergo projection. Every final sector is present, and
the independent checker verifies the supplied rational's strict membership in
the reconstructed sector. Default-off and historical evidence retain their
previous canonical-sample check.

The pool is FIFO-bounded to 256 rationals, 32 KiB charged storage, and 1,024
numerator-plus-denominator bits per sample. A choice uses at most 128 bisections.
Counters report hits, misses, admissions, evictions and peak charged bytes.
These bounds do not relax solver time, memory, node or refinement limits.

Five module tests and five integration tests passed. The latter independently
verify both sides of three ablations and compare their generic decompositions
through exact common refinement. In the first small fixture, visited nodes fell
from **570 to 540**, with **44 refinements on both sides**; the other two fixtures
had unchanged node and refinement counts. This demonstrates child-work reuse,
not fewer mathematical cuts or an A381 completion. Boundary/outside sample
tampering, omitted sectors and resource exhaustion are rejected. TOML,
homogeneous reduction and historical default-off replay are covered.

Test evidence is in `artifacts/profile/a381-stable-samples-01/`:

- `unit-tests.log`: SHA256 `e94b93b9e4fd442f66bc248338b43373648a573890f8de00d95322a834ba5c6a`.
- `integration-tests.log`: SHA256 `dcb1ec42f9c1d6114ec5dd47eeca62f7945e0f4464bac14d41d8c018a9229c0f`.
- `integration-initial-failure.log` retains the initial overly strong assertion
  that refinements must decrease. Coverage checks passed; the corrected test
  measures saved child visits and makes no universal performance claim.

A matched full A381 on/off campaign is still required. The option remains off
by default, and no new original-problem completion is claimed here.
