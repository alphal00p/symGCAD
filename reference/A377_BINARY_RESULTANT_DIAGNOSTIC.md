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

## The recovered large pair and the 96-million-bit limit

The recovered pair has degrees 12 and 12 in x0, 1,009 and 590 input terms,
and only s,t remaining. Its coefficient heights are at most 86 and 65 bits.
Exact layout inspection gives output degree bounds 360 and 120, 43,681
mixed-radix digits of 1,914 bits, and an encoded-output bound of **83,605,434
bits**. This exceeds the former 64-million-bit production cap and explains
its static fallback to the ordinary method.

The frozen research helper recomputed the pair with a 96-million-bit cap on
CPU8, under 120 seconds and 4 GiB. It completed in **95.188976 seconds** at
335,356 KiB peak RSS: 90.080414 seconds for the integer resultant, 0.020201
for encoding and 4.315338 for decoding. The full signed rational polynomial
contains 30,095 terms. This is a single operation diagnostic; its build and
CPU differ from the saved 3,041.788607-second ordinary operation.

An independent verification process compared the signed polynomial against
ordinary exact numeric Ducos resultants on a **complete 43,681-point nested
degree grid**. It checks every accepted specialization's leading degrees,
skips degree drops exactly, and checks candidate support against independently
derived degree bounds. Every required point passed. Verification took
70.768360 seconds at 82,964 KiB peak RSS, outside the operation timing.
This proves the full polynomial identity, including scalar and sign;
it does not depend on the old canonical cache's lost scalar.

The opt-in production output cap is consequently raised to 96 million bits.
The active-parameter, degree, term, coefficient, digit and encoded-input bounds
are unchanged, as are the baseline and independent decomposition verifier.
The existing worker deadline still bounds the indivisible integer operation.
An encoded-output bound is not a bound on intermediate native memory or time.

The first recovered JSON used an `input` field instead of the profiler's
`operands` field and lacked capture flags. Two saved inspection attempts
rejected it before algebra. A separate `capture-v2.json` fixes only that
serialization, with the ordered operand hash rechecked; the original recovery
evidence remains intact. The frozen research helper hash is
`44428e764331e44d295eb4b8dae17201d39fe7b1a8573a240a499b1fea95ec09`.

| File | SHA-256 |
| --- | --- |
| `ac6f3048ce62-capture-v2.json` | `9f52f643343aef0c9f7c445902f8a88e7ee7f5f7749b72193b987cba08e8f147` |
| `ac6f3048ce62-layout-v2.json` | `4fc54f8a5b59c327db0a0d9c1a0eb367a897c15d9c4388d2a660352300a91711` |
| `ac6f3048ce62-segmentation-96m.json` | `9d96d5dc96f3c7abf210cf7d634b588bce52687e271e5de93aa84c4023b95ac9` |
| `ac6f3048ce62-segmentation-96m.process.json` | `606b855c0222e9a8c4ba2637001b7ef47e72ee9c58bb4ef6bc9292e237c69213` |
| `ac6f3048ce62-grid-96m.json` | `e5f777c7ea4ab9ab654bd2689fb2fe964ce7ba1e3186a76ec62d03c1693c3896` |
| `ac6f3048ce62-grid-96m.process.json` | `f96d348c47cdab5d1666879f468c53dcb0bd4c89876193e2c6349f1a9c0dc9eb` |

A separate 60-second replay from a copy of all 1,880 warm records timed out
after 60.023441 seconds at 352,560 KiB, without a new capture, result or proof.
Its namespace is `artifacts/research/a377-warm-capture-replay-01/`.
It did not identify the final hour's pending operation. The recovered completed
pair and its independent identity check, rather than that replay, supply the
evidence above. A377's original symbolic decomposition remains unresolved.

All eight existing segmentation regressions pass with the new limit, covering
exact signed rational identities, bounds, fallback and cancellation. The log
`artifacts/support-seeds-segmentation-tests-01.log` has SHA-256
`57e3b5f2db60df6a891ce81ebbd615e99a6bc3463c99e3493f277d210d3ec7bd`.
Independent review also checked every unchanged production gate, including a
conservative 87,121,452-bit total encoded-input bound below the 512-million-bit
cap. The frozen production path will be measured separately from the research
helper; the full signed candidate is available for that comparison.
