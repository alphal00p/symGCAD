# Restartable first-axis support proposals

The optional first-axis support basis can reuse bounded polynomial-product
proposals from a checkpoint directory. Every reuse reconstructs the exact
source polynomial. Root isolation and independent proof verification remain
fresh computations.

This is enabled only when `local_base_univariate_basis` and local caching are
enabled and `checkpoint_dir` is configured. It has no effect on initial input
factorization or the ordinary irreducible-factor cache. Durable local hint
archives remain incompatible with the support-basis option.

## Record and replay contract

Records live separately under `base-support-v1/`. Their schema explicitly says
`unverified_product_proposal`; they are not completed algebra-operation records.
The key binds the trusted request's ordered full variable map, actual first
axis, canonical source digest, coefficient domain, backend version and resource
policy. The source text is already bounded by normal admission and is not
duplicated in the cache.

A record contains sparse integer block coefficients/exponents and positive
multiplicities, with a payload checksum. It contains no runtime polynomial IDs,
root intervals, sign facts or irreducibility claims. Reducible, repeated and
shared-factor blocks are valid only when their exact product reconstructs the
canonical source. Builder separately checks the original raw guard's signed
nonzero rational scale.

Reads use a no-follow regular-file descriptor and a 32-MiB complete-file limit.
Integer grammar, coefficient height, support, degree, term, multiplicity,
collective block-storage and product-work bounds precede powers. The existing
registry quota and source-plus-block accounting also apply to cache hits.
The schema 2 registry's expanded count and cumulative allocation bounds are
part of the policy key; historical proposal namespaces are not silently reused.
See [registry capacity and proof compatibility](BASE_SUPPORT_REGISTRY_CAPACITY.md).
The approximately 22-million-bit D32 block inventory fits this record envelope;
no workload identifier participates in admission.

Missing, malformed, wrong-key or structurally inapplicable records are bounded
misses. An exact product mismatch rejects an untrusted proposal and permits
ordinary native discovery. Cancellation or resource exhaustion during replay
propagates `Incomplete`; it never starts a second backend. Failures after native
discovery enters retain the existing `ComputationFailure` behavior.

Native proposals may be atomically saved after bounded output preflight and
before expensive product reconstruction. A restart must still replay them.
Cache I/O is best effort, and checksums never replace mathematical checks.
Report counters distinguish disk lookups/hits/misses/rejections and proposal
writes from existing in-memory hits and completed-operation counters.

The checker never reads this cache. It validates the proof's source/block
products, regenerates raw projection guards, checks proportionality, isolates
all roots and checks every sector. Deleting the cache must not affect proof
verification. All interior seams and full-real root ordinals remain required.

## Expected scope of savings

The authenticated D32 research run measured 285.4996 seconds in native SQF and
41.4682 seconds in exact product replay; later all-block root isolation took
1187.682 seconds. These identify skipped and retained stages, not a measured
cache speedup or original-problem completion. A warm proposal still pays for
parsing, product replay and every root-isolation obligation.

## Separate future root-certificate experiment

Native exact VAS currently exposes no public restart/certificate interface.
Its returned root intervals omit the rejected zero-root subdivision leaves.
The public `roots_hot_start` uses numerical Aberth iteration and supplies no
exact restart authority. Naively checking many saved intervals with
`roots_in_open_interval` also rebuilds a Sturm sequence for each call.

A possible exact alternative is a complete rational partition under a strict
Cauchy root bound. Each Möbius-transformed open interval must have Descartes
variation zero or one; every finite endpoint is checked separately. Zero
variation excludes roots, and one proves exactly one simple root counting
multiplicity. Exact rational endpoint roots require bounded deflation and
product replay. Both infinite tails, negative roots, shared roots and complete
owner ordinals remain explicit obligations. Cached intervals are partition
hints, never proofs of completeness.

The existing local interval-Descartes implementation uses this theorem but has
small symbolic degree/height/stock caps. Raising those caps is not a restart
implementation. A separate research verifier should first measure the
authenticated degree-268 block, with bounded transform size, subdivision depth,
total nodes, retained coefficient bits and a hard 60-second/2-GiB process cap.
Any unproved interval or gap makes the certificate incomplete. Repeated
irrational roots require another certificate or a conservative decline. No
high-degree root-certificate experiment or production root-cache change is
included here.

## Focused validation

112 tests passed across five targets: 22 base-support library tests, 15 basis
integration tests, six deferred-projection tests, 67 existing local-projection
tests and two report-compatibility tests. One authenticated expensive replay
remained intentionally ignored. New regressions cover signed units,
composite/shared/repeated blocks, rehashed malformed records, bounded regular
reads, pre-replay persistence, cancellation without native retry, registry
charging after a disk hit, cache ablation, exact cold/warm proof equality and
independent verification after deleting the cache.

All three passing commands used CPU 3, one Cargo job, a 180-second supervisor
and a 2-GiB aggregate RSS limit. The largest sampled compiler/test RSS was
2,095,596 KiB, close to the 2,097,152-KiB cap. The integration and report runs
peaked at 1,873,052 and 876,020 KiB. An earlier compilation failed on an owned
test's moved value; adding the required test-only clones fixed it. No algebra
or acceptance rule changed for that correction.

Logs (under ignored `artifacts/`):

- `base-support-cache-focused-02.log`: SHA256
  `92e22fde31cf2ae9b21b0543d4b3b6d9dcf9ba9a68897620a50da96879c8af71`.
- `base-support-cache-integration-01.log`: SHA256
  `0a37b57a538edb4800f0c3700c37ed5064a251fa49c57cc57486fc7d081bad1e`.
- `base-support-cache-report-01.log`: SHA256
  `56230d9b39c5db41a43577580e9895e981c1da646350db3d54ef9a9a10c7551d`.

Independent source review found no blocker. No new full-original campaign,
release speed claim, cache-hit measurement on D32 or root-certificate run is
part of this milestone. Source-count and storage admission limits are unchanged.
