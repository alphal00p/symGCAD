# Crossed A318: measured operand improvement and warm continuation

The original one-hour queue left both symbolic Ax12-A318 and Ax12-A414
unresolved. Inspection found five common completed discriminant operands
with identical canonical output hashes in their native operation caches.
The largest had 800 terms and degrees `(s,t)=(35,25)`; its canonical
discriminant has degree 1439 in `s` and 933 terms. Historical native
operation records took 423.214 and 424.153 seconds respectively.

The frozen production segmentation profiler recomputed that operand in
**48.819 seconds**, including 48.784 seconds of algebra, at **98,596 KiB
peak RSS**. Its 18,476,050-bit encoding is within the existing production
cap. An independent audit found the same complete canonical output-array
hash as the native records:
`2857d839806b63421fccc9389d599adddaf72af8bbfbe7cd934f2ef1ecef992a`.

This is a single-operation comparison between two Symbolica strategies,
not an upstream GCAD speed ratio or a completed decomposition benchmark.
The profiler records a canonical hash, which cannot establish signed
rational scale. Exact whole-proof verification remains required. The
captured operand was reconstructed from a completed cache record; it was
not a snapshot of the operation executing when the original timed out.

The operand capture is under
`artifacts/research/slow-followup-selection-01/recovered-9462984a190e/`;
its SHA-256 is
`e3f9f6964148eb6821c41b94882290ba50d7dc9fc035e7540dd43c08645212e4`.
The independently checked production replay audit is
`artifacts/research/slow-followup-segmentation-01/production-replay-audit.json`,
SHA-256 `8c20d05d77d34e44f19041a17b0f86de33d9ffbf31b34f5093be2d5da3ae7d91`.

## Bounded continuation

A symbolic Ax12-A318 continuation started at **12:59:20 UTC** on
10 October 2026, on CPU11 with 8 GiB and a one-hour campaign budget.
It enables segmentation with the frozen `9b7dd56` release solver,
SHA-256 `7bf9db11d4f8636ad09b57b52aaf6b87a3b61ab3a3cb61d70d9cef592c752657`.
The first-axis support-basis experiment remains disabled: no completed
factorization record established a benefit for it on this workload.

The full order is `[s,t,x8,x1,x2,x3,x4,x5]`, with the previously verified
positive-scale anchor `x8`. Original mathematics, domains and groups are
unchanged. The launch copied 38,713 matching completed native records
(216,634,922 bytes), checking their full maps, keys, output checksums and
copied file hashes. Sixteen killed temporary fragments and records from
other maps were excluded. No research result was injected.

The historical ordering search cost remains recorded. This is a warm
continuation with a given order, not a new cold measurement excluding
search costs. It supplies no completion credit until an exported result
passes independent exact verification.

Evidence is in `artifacts/research/ax318-segmentation-warm-hour-01/`.
The launch digest is
`355ce3cfc3e85c6b87a15b9146076893ab3fdc6b4cc6c144d87452c58b811620`;
the input-audit digest is
`e92508cb71332acd6bc073b809cdde95ce976a702bb2476a1f869cf7def0cf34`.
The launch binds the initial cache manifest, frozen build and CLI smoke
verification. All ordinary and repeated interior boundaries remain required.

## Final continuation audit

The continuation ended with a timeout after **3600.023400 seconds**, at
**695,368 KiB peak RSS**, without a result or verification file. The final
cache contains **46,551 completed records** (396,657,035 bytes). All 38,713
inherited files are byte-identical; the **7,838 new records** comprise 7,331
factorizations, 466 resultants and 41 discriminants. Their recorded operation
times sum to 1462.268914 seconds, excluding other solver work. The largest
new completed factorizations took 275.169, 241.462 and 209.298 seconds; a
completed `t`-resultant took 183.280 seconds. These are retained-work
measurements, not solved-case timings or paired speed ratios.

The final incomplete capture identifies `Disc_x1(p)`, with **572 terms**,
degrees **`(s,t,x1)=(9,10,10)`**, and 106-bit coefficients in the full map
`[s,t,x1,x2,x3,x4,x5]`. Its operand hash is
`55cf751856d1a05a1c734293b24d75d6188bb835c0c02c607117824cc9e60caa`.
No completed record exists for that request. The capture identifies the last
published operation; it does not measure how long that operation ran.

This is a concrete next diagnostic target. It has **two surviving parameters**,
so the current one-parameter discriminant segmentation gate declines it.
First-axis support-basis mode cannot replace a discriminant at lifting axis
`x1`. A bounded comparison of exact strategies on this captured operand would
therefore be more informative than another unchanged one-hour solve. No such
comparison or new solve was launched by this audit, and no interior boundary
may be omitted on the strength of these timings.

The final audit rechecked original mathematical fields, legal order and
homogeneous anchor, prepared/trial options, frozen executable hashes, every
complete native tuple key, output/multiplicity checksum and inherited file
hash. This is provenance validation, not independent algebraic replay of the
cache. Evidence under the same campaign directory is:

- `final-audit.json`: `31a1810a214200eb2ab5f5aa2b60bbc5909d54d89b7b83e58584e64422e87c62`.
- `final-cache-manifest.json`: `0508158c0ae0613e60267deeedd1f708d7609a6e791a87608b70676fb822f9b0`.
- `final-capture.json`: `42c98491b8bce9e98028ace6e2947908e828696b8cb3f87627813437cdfa600e`.
- `results/ledger.json`: `cd783c7b49e66d90654267628e0304a2ff1cd44033079913c58770fcb2507224`.

No research output was inserted and no original completion credit was added.
Verified coverage remains **303 / 331**.
