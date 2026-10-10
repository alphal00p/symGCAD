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

### Static check of the pending operand

A subsequent bounded inspection evaluated only the encoding layout, with no
resultant computation. The generic exact kernel would use **32,852 digits**
of **2,112 bits**, for a **69,383,424-bit output bound**, below its existing
96-million-bit cap. Its total encoded input bound is **74,904,192 bits**,
below 512 million. The derivative pair has degrees `10 × 9`, 1,134 combined
terms, integer coefficients of at most 106/108 bits, and two surviving
parameters, `s,t`. All other checked generic-pair admission limits fit.

The frozen helper's native derivative inspection and a separately constructed,
coefficient-checked `p,p′` capture produced identical layouts. Exact integer
norm bounds were rechecked independently. Each inspection completed in about
0.02 seconds on CPU11 with a 4 GiB/60-second cap. The current discriminant
caller's one-parameter restriction is therefore the only checked static
blocker. This motivates reviewing a bounded two-parameter applicability
extension; it is not evidence of a computed discriminant or a speedup.

The metadata audit is
`artifacts/research/ax318-pending-discriminant-layout-01/audit.json`, SHA-256
`7646b4d75dced2c12c457b333f58058e83d52d29f0ba33f084e310f54ac0c439`.
The explicit derivative-pair capture has SHA-256
`359f8e40c97be6b4a6709843d6ecb635f0d20a23ddabb27520e20e40af0c10be`
and pair input hash
`510e1aa47dafabf31f7c54cf0c9b7198a6d8208d8f6a4c83b192084ed7300544`.
No production option, cache or proof was changed.

### Signed discriminant computation and independent grid

The unchanged frozen research helper then completed that exact captured
`x1`-discriminant in **48.177916 seconds**, at **256,888 KiB peak RSS**.
The signed result has **13,704 terms** and degrees `(s,t)=(162,90)`.
A separate exact numerical Ducos grid completed **all 32,852 valid points**
in **30.125722 seconds**, at **49,244 KiB peak RSS**. The complete campaign,
including the intervening Rust Symbolica adapter and binding checks, took
84.138443 seconds on CPU11. Its limits were 4 GiB, at most 1,200 seconds for
computation, and 3,600 seconds shared by all stages.

The unknown resultant's bounds are `(171,190)`, giving `(171+1)(190+1)`
points. Verification does not substitute the smaller candidate degrees for
these bounds. The nested grid skips specializations where either input
loses its eliminated degree. For degree ten, the exact adapter retains
`Res(p,p′) = -LC_x1(p) * Disc_x1(p)`, including the negative sign and full
rational scale, and replays the exact quotient. Its derivative matches the
separately coefficient-checked derivative capture. Consequently the full
grid proves the signed discriminant identity after cancellation of the
nonzero polynomial leading coefficient; no pointwise nonvanishing assumption
or removal of leading-coefficient-zero boundaries is made.

All input, executable, adapter and candidate bindings were checked again
after completion. Evidence under
`artifacts/research/ax318-pending-discriminant-compute-01/` is:

- `independent-final-audit.json`: `5e4fa155b4287ca7c8719ee999878833453fefd3642952b5bb0ec2e6a9ebebe5`.
- `discriminant.json`: `5e5c0ef77705c3cc740b81c63a96b6b2c4b9442849e7918880e842a92a852a2e`.
- `grid-verification.json`: `9fd05c7c1baf8b64693465ca5633179b87fa1f3521b6744cd3256fe2a1bffc47`.

This is completed algebraic identity evidence for one authentic pending
operation. No research value was inserted into production, and it adds no
original CAD coverage or solved-case performance ratio.

### Production two-parameter continuation

The opt-in production discriminant envelope was extended to at most two
surviving parameters in `380d0144206ff8bc02cbbe8a682de943f5564c5b`, retaining
the existing exact kernel and all size caps. The default remains unchanged.
Its 201 focused tests passed. Release CLI smoke tests independently verified
7, 4 and 4 cells; the dense squared-polynomial case computed one segmented
discriminant and retained both same-sign interior zero sheets, with the
`t=s` leading-coefficient seam still exceptional. The smaller sparse smoke
case declined segmentation and tested the ordinary fallback.

A fresh **warm** original Ax318 continuation launched at **14:29:44 UTC**,
CPU11, 8 GiB, with 3,600 seconds shared by solve and verification (verification
at most 600 seconds of the remainder). Original mathematical fields,
`[s,t,x8,x1,x2,x3,x4,x5]` order, `x8` anchor and solver options are unchanged
except for the new checkpoint path. `binary_resultants=true` remains enabled;
base-univariate mode remains disabled. All **46,551** final native records,
**396,657,035 bytes**, were copied and rechecked. The independently verified
research discriminant above was not inserted into this cache.

The release build took 487 seconds. Its executable SHA-256 is
`9f4230ea1b6a1c5c1090237947d76f432591f2d55a98897c9e40eea2c02d71a3`;
build manifest `artifacts/two-parameter-discriminant-release-build-01.json`
has SHA-256 `79c75caf160926e21a0b28f1c45365d80f1ffe9c6b5c05615de0c24fda20b6ad`.
The source-manifest check and smoke audit are bound by that manifest.

New evidence under `artifacts/research/ax318-segmentation-warm-hour-02/`:

- `input-audit.json`: `5439226522432c3186941d7b213bcf732ee20b16cd13cf636854054430bff3db`.
- `initial-cache-manifest.json`: `32d99d08982789cbf0313bdcacc26ba1a712b9c793d5d19a3528b2f37e69bcd5`.
- `launch.json`: `6844d8e407cb0de1ece8549ad9b36cd0daf10d257d61a686e58a93f80bc7d31d`.
- `actual-worker-audit.json`: `6cb2ad47347f19ad8e4b1a6f5b0b18444baf5361557ca0c269e68736c617fa70`.

The actual worker's original fields, explicit options, order, anchor, shared
cache, memory/CPU limits and capture environment were independently checked.
Earlier search and warm-attempt costs remain retained separately. This launch
has no solved-case speed ratio or original completion credit.
