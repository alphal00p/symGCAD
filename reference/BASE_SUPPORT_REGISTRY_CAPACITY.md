# Bounded first-axis support registry capacity

The optional first-axis support basis now generates schema 2 evidence with at
most 4,096 source records. This is a resource-policy extension for generated
guards in `Q[x0]`; it changes neither the support-product theorem nor root
isolation, ordinals, exceptional sets, or the default solver path.

The motivating A377 snapshot reached all 128 historical source slots while
retaining only 71,975 source coefficient bits and 68,659 block coefficient bits
(140,634 total). It recorded 1,335 source-count declines; all admitted native
square-free calls took 0.003184 seconds and their product checks 0.015187 seconds.
There were 125 distinct block IDs. This establishes quota pressure, not 1,335
distinct missed sources or a measured solver speedup. The incomplete worker
did not export its registry, and the snapshot does not establish that it had
encountered D32. Evidence: ignored
`artifacts/research/a377-segmentation-warm-hour-06/quota-inventory-availability-01.json`,
SHA-256 `3ae83d454bd365df6c66a7ca97c08d7657311506130723d4439334b093fcc9c6`.

## Storage policy

Symbolica's `MultivariatePolynomial<Q,u32>` retains a `Vec<Rational>` and a flat
`Vec<u32>` containing one full exponent vector per term. Its polynomial context
is shared through `Arc`. Coefficient-bit limits alone therefore do not bound
the exponent arrays or sparse expression strings.

Schema 2 adds these cumulative limits across source records and their blocks:

| Quantity | Limit |
| --- | ---: |
| Source records | 4,096 |
| Source plus block terms | 2^21 |
| Source plus block exponent entries | 2^25 |
| Source text bytes plus block text upper bound | 128 MiB |

The exponent limit represents 128 MiB of logical `u32` entries, not an RSS
ceiling. Existing coefficient limits remain unchanged: 512,000,000 cumulative
source-plus-block bits, 128,000,000 bits and 65,536-bit coefficients per source,
64,000,000 collective block bits and 20,000-bit block coefficients per call.
Degree 4,096, 4,096 terms per polynomial, 64 blocks per source, 32 MiB source
text, and the 256,000,000 product-work preflight also remain unchanged.

Accounting charges each source record's complete block inventory even when
other records reuse the same block ID. Multiplicity does not create extra
retained block copies, but remains fully charged by the existing product-work
bound. Source text uses its already-required exact byte length. Block text uses
a conservative integer-digit, sign, separator, axis-name and exponent bound;
this introduces no large coefficient formatting merely for measurement. The
bound covers each polynomial's sparse expression spelling, not JSON escaping
or all surrounding serialization overhead. These are logical costs, not an
allocator, transient native-operation, or process-memory guarantee.

A source-only aggregate violation declines before native discovery and uses
the ordinary path. An aggregate violation discovered after obtaining blocks
returns the existing typed resource stop; it cannot start another backend.
Saturating arithmetic rejects overflow. Only an accepted complete signed
product updates retained counters. Progress reports expose source/block term,
exponent-entry and text costs separately, with distinct static decline reasons.

## Compatibility and lookup

Historical schema 1 proofs retain their original 128-source limit and every
existing algebra/resource check. They are not subjected to schema 2's new
aggregate allocation gates. Schema 2 accepts at most 4,096 sources and replays
the new limits independently of reported counters. Unknown schemas are rejected.
The old no-basis format and omitted diagnostic fields remain accepted. New
schema 2 proofs require a verifier that understands that schema; the new
verifier continues to check old proofs.

Canonical source lookup is hashed. An additional support-shape hash narrows
proportional lookup to candidate indices, without duplicating polynomial
payloads: canonical keys and indexed entries share immutable `Arc<Poly>` values.
Every candidate still requires the exact full ordered variable map, complete
exponent array and signed rational coefficient relation. Hash collisions cannot
authorize a match. A bucket can still contain all 4,096 sources with identical
support; no constant-time bound or measured lookup speedup is claimed.

Generation still applies the existing raw admission caps before normalization.
Fresh checker replay preserves the separate large-raw-unit proportional lookup
for already validated canonical sources. Duplicate source records are rejected;
duplicate in-memory requests retain the first checked ID mapping and consume no
new quota or storage.

The untrusted proposal-cache key includes the new schema and allocation policy.
Historical proposal files are cache misses (or rejected if copied under the new
key); no migration turns them into checked products. Every admitted hit still
reconstructs the exact product and pays current registry costs. Independent
verification continues to ignore all disk proposals. See
[the proposal-cache contract](BASE_SUPPORT_PROPOSAL_CACHE.md).

## Validation

Focused regressions cover 160 actual small source admissions, negative rational
and duplicate hits without additional accounting, independent schema 2 product
replay, schema 1 count compatibility, both serialized proof schemas, unknown
schemas, exact shape-collision rejection, aggregate exponent exhaustion from
full 64-axis representations while coefficient bits remain small, pre-entry
fallback versus post-entry resource stops, and proposal-cache policy mismatch.
The focused gate passed **120 tests across five targets**, with the existing
authenticated expensive replay intentionally ignored: 29 library tests, 16
basis integrations, two report integrations, six deferred-projection tests and
67 existing local-projection tests. Independent source review found no blocker.

Commands used `cargo --config profile.test.package.symgcad.debug=0 test
--locked -j1`, CPU 3, and the existing 180-second/2-GiB aggregate-RSS supervisor.
The library filter was `--lib base_univariate`; integration targets were
`local_base_univariate`, `local_base_univariate_report`,
`local_deferred_projection`, and `local_projection`. All ran serial test threads.
The library build/test process used 58 seconds with sampled peak 1,766,224 KiB;
the integration process used 118 seconds and 1,247,520 KiB. A preceding build
with normal test debug information was stopped before tests at 2,097,932 KiB
(80 seconds); its failure record is preserved. The command-only debug setting
does not change the manifest or production build settings.

Ignored log/provenance records:

| Record | SHA-256 |
| --- | --- |
| `artifacts/base-registry-capacity-focused-02.log` | `27e4c9b847aa4cb1d187b43dd47b0dbbfb8c5250c8066777df877e11b189377e` |
| `artifacts/base-registry-capacity-integration-01.log` | `b4deedf02bb0ec1f52e1425cc4a1c90110cfeeda3642160f2cbc9ec21b1076d2` |
| `artifacts/base-registry-capacity-sources-before.sha256` | `81f68f9eda5e1833e439628b60d6c9dca6831f228be71f29aa80509ca573a2eb` |

The source manifest, including the simultaneous weighted-certificate work,
matched after both gates. This evidence concerns correctness and bounded
resource routing; no original workload completion or speedup is established.
