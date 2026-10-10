# Differentiated support-source limits

This is a resource policy for the existing, default-off first-axis
support basis. It changes no zero set, multiplicity, root inventory, selector,
projection obligation or proof contract. Repeated factors can make a source
much larger than the blocks whose full real roots describe its support; those
two objects need different bounded admission envelopes.

| Quantity | Limit |
| --- | ---: |
| Source numerator or denominator height | 65,536 bits |
| Source coefficient storage | 128,000,000 bits |
| Each block height / coefficient storage | unchanged: 20,000 / 64,000,000 bits |
| Complete block inventory per source | unchanged: 64,000,000 bits |
| Cumulative source-plus-block registry | 512,000,000 bits |
| Canonical source expression | unchanged: 32 MiB |
| Sources / blocks per source | unchanged: 128 / 64 |
| Degree / terms / axes | unchanged: 4,096 / 4,096 / 64 |
| Denominator storage before normalization | unchanged: 65,536 bits |
| Product height / degree-weighted work | unchanged: 65,536 bits / 256,000,000 |

These limits apply to mathematical shape, independent of fixture names or
hashes. Source admission must still decline before native SQF when any static
check fails. Once native work begins, resource exhaustion remains incomplete;
map, support and product inconsistencies remain computation failures. Neither
starts an ordinary second backend. Existing worker time and address-space
limits remain necessary: coefficient storage does not bound native temporary
memory, factorization time or root-isolation time.

## Implementation

1. Separate the shared support/degree/term checks from coefficient accounting.
   `source_cost` uses the new source envelope, while the existing block `cost`
   retains its limits. Apply the source envelope both before and after primitive
   normalization. Preserve the full variable map, actual axis zero, degree-two
   source minimum and exact signed normalization identity.
2. Add a bounded canonical-source formatter before native SQF. It accepts only
   integer coefficients, the first axis and increasing sparse exponents, emits
   the exact existing `Algebra::format` spelling, and checks the accumulated
   byte count before admitting each term. Build at most one bounded coefficient
   string at a time, retain at most 32 MiB of term text, and join into an exactly
   sized final string only after the bound passes. Reuse that string in evidence
   instead of formatting again after native work. A text-size failure is a new
   static `source_text` decline, published before ordinary fallback.
3. Keep the source parser's strict sparse integer grammar. Its lexical integer
   ceiling becomes 19,729 decimal digits, a ceiling for a 65,536-bit integer;
   retain the exact bit check after parsing. A 19,729-digit literal need not fit
   the bit bound. Reject functions, products of expressions, fractions, unknown
   variables, duplicate/non-increasing exponents and out-of-bound exponents
   before polynomial expansion. Zero, leading-zero and alternate spellings
   still fail the final canonical spelling check. Canonical sources remain
   primitive integers even when the raw guard carries a rational signed unit.
4. Update the validator's source accounting to `source_cost`, retaining block
   accounting separately. Put the cumulative 64M-bit block check into shared
   product preflight before powers: the checker already imposed it, whereas
   discovery previously checked only individual blocks. Replay the complete
   canonical product and the raw signed scale exactly. Preserve proportional
   lookup against already validated sources without reapplying discovery caps
   to a freshly recomputed raw scalar multiple.
5. Use the same source, text, block and cumulative registry checks in builder
   and independent checker. Keep evidence schema version 1: historical proofs
   satisfy the larger source/registry limits, and block/product limits do not
   weaken. New telemetry fields use serde defaults and remain non-authoritative.

Text construction requires at most about 64 MiB of live term-plus-joined text
for one accepted source, plus bounded coefficient scratch and vector overhead.
The 512M-bit registry is about 64 MB of logical coefficients, **not an RSS
limit**. Text, exponent maps, shared-registry references, proof copies and native
temporaries are additional. The independent 32-MiB-per-source limit and 128
source limit give a coarse 4-GiB total text ceiling; coefficient and term/name
accounting makes ordinary cases substantially smaller. No 4-GiB worker-fit or
one-hour completion claim follows from these limits.

## Focused validation

- Exact source admission at 65,536 bits and decline at 65,537; block rejection
  remains at 20,001 bits. Exercise a multiplicity-heavy exact product whose
  source exceeds the old height cap while every block remains below it.
- Construct a sparse source above the old 64M-bit source storage cap and below
  128M, then check source accounting and bounded serialization without invoking
  SQF. Test 128M and registry512M boundaries with checked metadata, including
  repeated block references and the unchanged 64M complete-block ceiling.
- Test formatter byte boundaries with a small injected test limit, plus one
  real 32-MiB decline, and exact equality to existing formatter output. Confirm
  text decline occurs before SQF and survives the pre-fallback checkpoint.
- Test parser 19,729-digit lexical admission followed by exact bit rejection,
  oversized input before integer construction, exponent/map/support tampering,
  historical evidence, signed rational scale and exact multiplicities.
- Re-run independent checker, missing-root/sector, deferred-LC and nested
  wrapper regressions. No root or interior-boundary omission is permitted.

Validation passed 222 unique tests: 169 library tests, 13 basis integrations,
two reporting integrations, six deferred integrations, 18 root integrations
and 14 research-helper tests. One expensive authenticated replay stayed ignored.
The library pass is in `artifacts/base-univariate-resource-focused-02.log`
(SHA-256 `96feaae8de56eda03778cef1018d8878e038f351700787458aa505d11ec502b7`).
That invocation subsequently stopped on an unrelated root-test ownership
adaptation; the remaining targets and both helper builds passed in
`artifacts/base-univariate-resource-focused-03.log` (SHA-256
`6c54464c6b61e6e669665c271556141cffdfe8bd65beac8165bcdda89a8ef31f`).
Independent read-only review found no remaining blocker.

The authenticated D32 measurements motivate separate limits but are not a
dispatch condition: its canonical source is 102,695,453 coefficient bits with
33,667-bit height; its four blocks total 22,021,775 bits and the largest block
height is 15,417. Its source-plus-block charge is 124,717,228 bits. Complete
signed product replay is recorded in [the SQF audit](A377_SQF_GCD_AUDIT.md),
and its discriminant identity in [the grid audit](A377_D32_GRID_VERIFICATION.md).
An existing registry's actual retention must be added; root costs and original
CAD coverage are separate obligations.

## Cheap segmentation applicability check

A generic research-only `profile_operation --binary-plan-only` mode can validate
a bounded, hash-bound sparse operation capture and call only the production
`resultant_segmentation::plan`. For a discriminant it uses the exact derivative
and one-to-two surviving parameters; pair resultants retain the default two.
It must reject calculation, reversal, factor-policy and expected-result flags.
Report source/capture/input hashes, dimensions, public plan fields and explicit
kernel-preflight-only semantics. This does not bypass the ordinary discriminant
interpolation priority and cannot claim an operation or original solve complete.

The actual production planner admitted the authenticated D32 capture with
4,921 combined terms, a 210,179,055-bit output bound and a 216,696,235-bit total
encoded-input bound. The captured full map is preserved and the exact derivative
has degree 31 against input degree 32. The final helper's entire process took
0.0852 seconds and 9,488 KiB peak RSS; no resultant was computed. The separate
ignored artifact directory is `artifacts/research/a377-degree32-production-plan-02`;
its report SHA-256 is
`49037393580fac7c80e21b52fe9d6af996e2bb4205d85a506cd97eb70c9bd717`.
The development helper was supervised on CPU 10 with a 15-second/1-GiB cap.
The source/capture/helper manifest and completed process record remain beside
the report. This is kernel eligibility evidence, not operation or CAD coverage.
The supervisor also rechecked the capture hash after the worker completed.
The first successful planner run remains preserved separately in `-01`.
