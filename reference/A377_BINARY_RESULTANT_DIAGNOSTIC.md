# A377 captured resultant follow-up

The second one-hour wave left the original symbolic A377 problem incomplete.
Its completed operation cache identifies two expensive resultants in `x0`
with only `s,t` remaining. The exact ordered operand strings were recovered
from saved canonical factors by matching the original input SHA-256; no
polynomial was guessed from a topology name or a numerical sample.

| Input SHA-256 prefix | Degrees | Input terms | Saved operation seconds | New release algebra seconds | Process seconds | Peak RSS MiB |
|---|---:|---:|---:|---:|---:|---:|
| `1d3e6e9835cc` | 12 × 8 | 1,009 + 575 | 859.856 | 34.511 | 34.626 | 176.1 |
| `87adaef31346` | 8 × 8 | 885 + 915 | 325.592 | 21.619 | 21.706 | 139.4 |

Both new canonical polynomial digests exactly match the corresponding saved
canonical outputs. This comparison does **not** recover the rational scalar
or sign removed by the operation cache. The cache's own `output_hash` hashes
`[output,multiplicities]`; the profiler hashes only the canonical output list.
The comparison recomputes the latter digest from the saved output strings.

The old and new timings use different builds and CPUs. They are operation
diagnostics, not a matched speedup measurement or an original-problem
completion. Both fresh profiles used CPU 3, one worker, an 8 GiB address-space
limit, and a 60-second process deadline. The production release profiler has
SHA-256 `cf4148ef6ea10d14f99055586026a0a5313802ba8f8e0c3f98305622426158f8`.

Before execution, exact layout inspection found output bounds of 54,210,420
and 44,584,722 bits, below the production 64,000,000-bit cap. Conservative
encoded-input bounds from the coefficient degree boxes were 59,762,724 and
47,399,040 bits, below 512,000,000. The production function independently
repeated its complete applicability checks before constructing encoded
integers. No applicability cap was enlarged.

Local provenance, exact operands and retained process records are under
`artifacts/slow-suite-second-wave-diagnostic-01/a377-captures/`; the recovery
manifest, each capture, cache record and comparison are hashed in the local
summary. The original reference polynomial and large cached outputs are not
vendored in this document.

An original-problem follow-up uses the hour run's exact mathematical input,
legal order and solver settings, changing only `binary_resultants=true` and
the fresh checkpoint path. It keeps the whole split polynomial and uses a
300-second solve deadline followed, only on completion, by independent
verification with a 600-second deadline. Its input audit and campaign ledger
are under `artifacts/research/segmentation-hour-followup-01/a377/`.
This cold follow-up timed out after 300.007477 seconds at 410.2 MiB peak RSS.
It produced no result or verification file. Nine of the ten tracked historical
pairs completed with matching canonical outputs, but that is not a complete
decomposition. Its final ledger SHA-256 is
`5b6a42ddaa7481fc6df74ab40b947c1a51e5b2f0f91af0eacade86378e242c73`.

A separate one-hour follow-up started from a copy of those 1,210 completed
cache files (78,372,155 bytes), retaining the original pilot unchanged. The
initial per-file hash manifest has SHA-256
`6730d5b0ae046c328ee1270bcbd57bd2c7bc3b82ee42aeebdc110013553fe7ab`.
The new campaign is explicitly warm and must account for the preceding
300.007477 seconds of preprocessing. Its input, bounds and provenance are in
`artifacts/research/a377-segmentation-warm-hour-01/`; the exact original
mathematics, order and solver options remain unchanged. This follow-up ended
at its hour limit and contributes no completion credit; final evidence is
recorded below.

The warm run subsequently completed the largest tracked pair in 34.480010
seconds with the same canonical output digest. All ten recovered historical
pairs have therefore matched across the cold pilot and warm continuation.
The subsequent live-worker profile sampled 1,452 events over 15 seconds at
99 Hz, with no lost samples. Complete sampled callchains include
`positive_projection_raw → ProjectionCache::resultant →
exact_resultant_with_statistics → exact_resultant_ducos`; they identify an
ordinary pair resultant after a static segmentation applicability decline.
They do not identify its exact operands or which applicability cap declined.

GMP basecase multiplication accounts for 81.43% of sampled self cost; dense
integer exact division has 14.84% inclusive cost, and the ordinary resultant
chain has 14.70% inclusive cost. Some stacks unwind incompletely, so the GMP
self samples cannot all be assigned to that caller. Recording and bounded
report analysis were pinned to the worker's allocated CPU 3. Their overhead
is included in this research campaign's elapsed budget; this is not a matched
benchmark. The first inline-symbol report encountered `addr2line` errors;
the bounded `--no-inline` report succeeded. Neither the worker nor its limits
were restarted or changed.

The raw profile, readable report and analysis are retained beneath
`artifacts/research/a377-segmentation-warm-hour-01/perf-0909/`:

- `perf.data`: SHA-256 `e6f83bab51e6a623ea8fafef3a966a114cfd354f1d14827398a098685266076a`.
- `report-no-inline.txt`: SHA-256 `b5c8f9e167df4cd263ab53e90fa26e048f35eb3b63c18c234bde1fe9ce5ed0bb`.
- `analysis.json`: SHA-256 `67c6508afb50dc3e3d184fc020db052cb2d0f2d6865964de71627ff638584cca`.

This run did not enable `SYMGCAD_CAPTURE_DIR`. Its stale progress record is
not an operand capture, and the profile provides no additional proof coverage.

## Final warm-hour outcome

The warm continuation timed out after 3,600.004030 seconds at 573,196 KiB
peak process-group RSS. Neither a result nor a verification file exists.
The original constraints, splits, parameters, variable groups and selected
order were rechecked against the original input. Together with its preceding
cold pilot, this consumed at least 3,900.011507 seconds of solve wall time,
including the live profiling overhead described above. It is not a cold run
or a matched speed ratio.

The operation cache grew from 1,210 to 1,880 completed records, adding 670
records and reaching 153,045,011 bytes. The last retained progress snapshot,
at worker elapsed 3,559.578 seconds, has 1,272,153 speculative nodes and
288,625 refinements, with level 5 and prefix `[5,0,5/4,1/2,1/2]`. This shows
substantial later progress; it does not identify the exact operation pending
at termination.

The largest newly completed operation was a resultant in `x0`, input SHA-256
`ac6f3048ce62636fa9eb95a6df3995fb92d2a7b62c9eb5f4b25945acd5ce3741`,
with recorded algebra time 3,041.788607 seconds. Its file was persisted at
09:51:44.480 UTC; subtracting that duration estimates a start near
09:01:02.691 UTC. That interval contains the 09:09 profile, making this a
timing-consistent candidate for the sampled ordinary resultant. This is an
inference from elapsed time and file metadata, not a direct operand binding
from the profiler. The next most expensive new operation was a 72.839-second
factorization.

Its exact ordered operand strings were recovered by a unique SHA-256 match
among 141 retained factors with the required variable support. The operands
contain 38,110 and 19,319 characters in the map
`[s,t,x0,x3,x5,x6,x8]`. The saved canonical result hashes to
`49e3495b80ed9a6cb4c987e552497231619476808c798214347eba6b9a11c7d6`.
As above, the completed cache does not retain the removed scalar/sign.
No new algebra was executed for this recovery.

The immutable final evidence is retained under
`artifacts/research/a377-segmentation-warm-hour-01/`:

| File | SHA-256 |
|---|---|
| `results/ledger.json` | `d759a2a30048237bc0a0b6dcd099e6db7e10a0a4dc2442edf77bcba82288ae16` |
| `final-audit.json` | `54c50dbebc618aa42d73e99a62d5e5c155174487d8972bc911d2c0bdeb2b12f5` |
| `final-cache-manifest.json` | `50a1d461a673047a69d961c5c6bffc034168ccd01b8d70c4f385264f958d8c4a` |
| `final-progress.json` | `8a7205e126a9d2766dd8f79143a808c1d89a25ff1281e4968763a36e2a4ce975` |
| `new-operation-timings.json` | `cc8d23267e1a9c10a19e8bab72bbca294414288b7501cc7a8b8f21055f4bc108` |
| `ac6f3048ce62-capture.json` | `efcbc63dec5f454ada5744254397d9729ba9492742597652ca9be6efe6476f3b` |
| `operand-recovery.json` | `7fc86953fb126162a9a4e752dde12850c07a57c2037f0b56fb029b19686d7d02` |

The final cache manifest binds the source factor records and all completed
outputs. The original warm worker was not restarted; subsequent experiments
must use separate outputs and explicitly describe any reuse of this cache.
