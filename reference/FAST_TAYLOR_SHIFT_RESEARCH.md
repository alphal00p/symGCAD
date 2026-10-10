# Exact factorial-convolution Taylor shifts (research)

This is a general, bounded experiment in `examples/profile_taylor_convolution.rs`.
It changes neither Symbolica nor the production solver. A successful run must
compare every output coefficient with native `UnivariatePolynomial::shift_var`;
it gives no root-isolation or CAD completion credit.

## Why measure this operation

The authenticated degree-1478 A377 support block previously needed 1138.558 s
in native isolation. A separate 150 s development-build replay stopped before
that block's isolation finished. Its first 49.963 s reconstructed the complete
signed source product and isolated three smaller blocks. A 60 s, 99 Hz profile
then sampled the known degree-1478 isolation phase: 5806 samples, none lost.

GMP `addmul_1` accounted for 90.35% self samples and `submul_1` for 4.22%.
Most GMP call chains were single-frame (5499 of 5806); these percentages cannot
all be attributed to Taylor shifts. Recovered `shift_var` ancestry appeared in
295 samples, beginning 0.325 s into the sampled window. Native square-free
decomposition precedes VAS, so it had already returned by that recovered VAS
call. This establishes a useful experiment, not the phase proportions of the
earlier complete 1138 s isolation. The frozen profiler predates shared root
fibers, so it is not a memory comparison against the current release.

Evidence: `artifacts/research/a377-d32-isolation-perf-01/summary.json`, SHA256
`80e9ec5c595bdfaefaed4315636d875a5d6838fb5073d93f7cc080aec03701aa`;
the exact call-chain summary is
`ca4a946a7fea327bddc09763deeaca15e19f18b40dc2b7b13e47194ba847799b`.
The worker timed out at 150.021 s with 492240 KiB peak RSS. Its source, native
backend, input, proposal and frozen executable hashes are retained beside it.

The pinned native shift uses the triangular update
`p[j] += p[j+1] * a`, with `k=0..n-1`, `j=n-1..k`.
At degree 1478 this is 1,092,981 coefficient updates. The proposed alternative
reduces the transform to one large integer product plus exact packing and
division. Whether that is faster depends on the actual coefficient heights.

## Exact identity

For integer `p(x)=sum p_j x^j`, degree `n`, and integer shift `a`, set

```
A_i = p_(n-i) (n-i)!             (0 <= i <= n)
B_k = n! a^k / k!               (0 <= k <= n)
C = A * B
q_i = C_(n-i) / (n! i!)
```

Each `B_k` is integral and each final division is exact. Expanding gives
`q_i = sum_(j>=i) p_j binom(j,i) a^(j-i)`, hence `q(x)=p(x+a)`.
Negative coefficients and negative shifts preserve their signs. Constant and
zero polynomials are covered, as are zero coefficients and translated roots
at zero.

The coefficient bound is `H=||A||_1 ||B||_1`. A power-of-two radix `R>2H`
uniquely encodes signed convolution coefficients. Balanced recursive packing
uses `low + R^k high`; centered recursive decoding recovers **all** `2n+1`
coefficients, including the unused upper convolution half. The helper checks
each decoded coefficient against `H`, exactly re-encodes the entire product,
and checks all `n! i!` division remainders. It does not truncate the integer
product before this check. Explicit arbitrary-precision shifts avoid the
pinned backend's optional small-integer shift fix.

Only Symbolica integer arithmetic is used. The helper does not implement
another polynomial multiplication backend or use floating-point recovery.
Its native comparison is a complete coefficient-vector equality, including
the variable identity and leading coefficient.

## Resource and input contract

The generic input is a SHA256-bound sparse integer polynomial with a complete
ordered variable map and a selected axis. All other exponents must be zero.
The parser checks literal grammar, ordered unique exponents, bounded names,
map lengths and byte/term limits before registering symbols or constructing
large integers. No fixture identifier selects an algorithm.

The current experimental limits are degree 4096, input height 20000 bits,
integer shift height 256 bits, input coefficient sum 64M bits and factorial
height 65536 bits. Conservative bounds precede factorial construction and
packing: at most 256M bits each for the packed output and aggregate packed
inputs; at most 2B bits of logical buffer/scratch reservations; at most 32 MiB
of coefficient JSON. Twelve output-sized buffer reservations plus factorial
and input scratch deliberately account for balanced recursion, decoded
vectors and re-encoding. These are mathematical payload estimates, **not** a
bound on allocator/GMP transient RSS. The worker is independently limited to
2 GiB and 60 s. Static declines and timed-out comparisons are inconclusive.

Stages report factorial/scaling, packing, integer multiplication, complete
decoding/re-encoding, exact division and native comparison separately. The
parent saves a small terminal process summary and stream hashes before
fallible postchecks. Outputs require a fresh directory; before/after input
and executable hashes must agree. A plan-only run performs no shift.

## Authentic input and comparison limits

The degree-1478 block is copied literally, with every coefficient string and
exponent checked against proposal block 2. Its original complete ordered map
is retained. The proposal SHA is
`3836f35e4e26d1ae51fbe674c963a28782590710e01d404839f54007614994c2`;
the complete signed-product report SHA is
`bdd0a2eed71f1496f90af78ec9e381a9e7e27ccb19c5bb113bbc36a128dd12fd`.
Preparation performs no algebra and the shift worker does not repeat that
source-product reconstruction. This is an ordinary coefficient input to the
shift experiment, not a new claim about factorization or discriminants.

The input and preparation record are under
`artifacts/research/taylor-convolution-01/`; input SHA256 is
`4fa804c83098bf885ff26f11a301b3645f4aa38d4c2eb901ee341e80a0d7a058`.
The first intended comparison is `a=1`. This raw block is not necessarily a
large-height intermediate transformed polynomial encountered later by VAS.
One microbenchmark cannot establish an isolator or solver speedup.

The worker currently accepts integer coefficients and integer shifts.
Signed rational coefficient clearing is tested: for positive common
denominator `E`, shifting `E*p` and dividing coefficients by `E` is exact.
A test-only rational-shift identity also checks
`P(X)=E*D^n*p(X/D)` for shift `a=A/D`, `D>0`, shift the integer polynomial by
`A`, and restore coefficient `i` by dividing by `E*D^(n-i)`. It would require
its own denominator/power resource preflight before worker admission.

## Measured result: no gain on the raw block

Seven tests passed, covering signed shifts, zero/constant polynomials, endpoint
roots, complete signed decode, nonintegral quotient rejection, full-map and
parser/cap failures, rational coefficient/shift restoration and bounded
terminal reporting. The focused test/build took 12 s and 794312 KiB aggregate
sampled RSS. An initial compile-only field/method typo was corrected; that
failed log is retained separately. The successful build used the development
profile, command-only symGCAD debug-info reduction, and the repository's
dependency `opt-level=2`. This is not a release benchmark.

The fresh CPU3, 2 GiB, 60 s comparison completed in **1.645 s**, peak
**124580 KiB**, with every coefficient exactly equal to native `shift_var`.
The experimental transform took **0.671435 s**; native took **0.266245 s**.
The experiment therefore provides **no speedup** on this raw degree-1478,
15417-bit block at shift 1.

| Experimental stage | Seconds |
|---|---:|
| Factorials and weights | 0.020058 |
| Balanced packing | 0.029957 |
| One integer multiplication | 0.424353 |
| Complete decoding and re-encoding | 0.131275 |
| Exact factorial divisions | 0.065687 |

The exact radix width was 40998 bits and the product had 121198534 bits.
The conservative preflight allowed 129708805 output bits, 129752670 aggregate
input bits, and 1728226434 logical buffer/scratch bits. It did not predict
process RSS. The parent rehashed the unchanged input and executable and saved
the checked coefficient vector. Fast-then-native order and one operand pair
do not establish general performance or the cost of later VAS operands.

Provenance under `artifacts/research/taylor-convolution-01/`:

| File | SHA256 |
|---|---|
| Frozen executable | `fbd967b028b832d7b46c0c3e19b28988687b964e3742f6a0f753b5e49f5f629a` |
| Helper source | `1afcd4569425143bf4150237d97c4e51dca43b25d56d5b038a7b9d0828312039` |
| `tests-build-02.log` | `21a31549cfdd3968180f7764fefa3cf17eb3866b93fe21c3770cc66f372e1705` |
| `shift-one/report.json` | `dcde77cc9d2e696a5f34a17f032d7c708baf8dfa4d9dd53867aa26f9d5155cd0` |
| `shift-one/shifted.json` | `70731f516cad62852ba065cb4a9ff794cd95e2624889fa2f4181b39a2370075b` |

`build.sh` and `run.sh` preserve the exact commands. `source-before-02.sha256`
and `source-after-check.txt` bind the complete library and manifest used by
the helper. The sparse coefficient output is an exact shift result only.

No production patch is justified by this result. A later experiment would
need representative transformed operands and a measured benefit. Native root
isolation, its complete root inventory, all interior seams and independent
verification remain unchanged.

## Isolated native VAS observation

A separate research-only Symbolica worktree at
`75f8350094b90254ee71dc2a391fde0d14b0204a` now provides a bounded observer at
its four integer VAS shift sites. The active dependency checkout and production
code are unchanged. Every site calls the original `shift_var` exactly once;
observer failures disable diagnostics and do not change the returned polynomial.
The global observer lock is released across arithmetic. Native shift timing
excludes operand serialization.

The replayable patch is retained under
`artifacts/research/vas-shift-observer-01/symbolica-vas-observer.patch`, SHA256
`9bcdeb7ff3b7fe0de7b313024799498360205a1a65900526155ad1c484942e54`.
The isolated symGCAD source is `0adcf97a03ebcdabb4788927227db5a8d3f959b7`.
The preparation, source manifests, build scripts and uniquely named frozen
executables are in the same directory. Shared Cargo dependency-cache reuse
avoids overwriting ordinary helper executables; rebuilding the isolated
numerica/graphica artifacts nevertheless made ordinary debug Symbolica stale,
so subsequent debug work requires a normal bounded dependency rebuild.
Release-profile artifacts were untouched.

The observer admits at most 256 calls and one million coefficient positions
for metadata, with at most 1 MiB of event records. It saves at most one raw
operand after a completed shift taking at least 100 ms, degree 512–4096,
coefficient height 20001–65536 bits, total coefficient storage at most 128M
bits, and a 256-bit shift. A conservative decimal-size check precedes bounded
streaming to a fresh file capped at 32 MiB. The single capture adds bounded
coefficient passes beyond metadata accounting. Native variable formatting is
also bounded. No source polynomial, root inventory, or proof is authorized by
an observer record.

Five isolated tests passed, including signed/zero/repeated inputs, capture
I/O failure, call exhaustion, oversized coefficients, and fresh-directory
validation. A separate four-fixture native root comparison produced
byte-identical off/on inventories, observing 17 shifts and exactly one
capture. The development build took 737 s with a sampled aggregate peak of
4990224 KiB; the tiny comparison took 1 s and 164928 KiB. All source manifests
and active-dependency hashes remained unchanged. The combined summary SHA256
is `f733362d70b0eaf3c66d60f1eced42312bb217af9c2428d1bcc2331ac0dfef8c`.

The authentic CPU2 diagnostic used the original D32 capture, signed candidate
and checked factor proposal. It replayed the complete canonical and signed
raw products afresh in 45.456557 s, then entered the degree-1478 block at
47.339708 s. No previous root inventory was supplied. A 4 GiB descendant-tree
RSS supervisor bounded the whole run to 150 s, with an inner 145 s worker
limit to retain its terminal report. The worker timed out after
145.035902 s, reporting 394084 KiB peak RSS. The separately sampled outer
aggregate recorded 160776 KiB and 148 s; these are different sampling/rusage
statistics and must not be treated as interchangeable exact peaks.

Exactly 256 native shifts completed before the observer disabled itself at
its call limit. Their heights ranged from 21607 to 34160 bits; shift values
were 1, 2, 4 and 8. The observed native times totalled 83.725200 s, with median
0.339381 s and maximum 0.490629 s. These are a bounded prefix of the isolation
run, not a timing breakdown of the earlier complete 1138-second inventory.

The one captured operand is the initial VAS shift input: degree 1478, height
21607 bits, 26786293 coefficient bits, shift 1. Its completed native shift
took **0.229108 s** and capture I/O took **0.084663 s**. The exact literal is
8115501 bytes, SHA256
`40bcadeae426d5d15f0ed72ad2ee8c63f061aa046690374ca46c07efcfbd48de`.
The frozen observer helper SHA256 is
`18431539e64c34d95c5a40cd2eaee4c03e3f42e0a7aaa72c87547c9c4aaa777c`.
The terminal summary, input/output hashes and reproduction script are under
`artifacts/research/vas-shift-observer-01/authentic-01/`; summary SHA256
`e95160788ea442f4a32538f7904e84d1880f4410f227a82e7df7eb37a5d857ca`.
Independent review rehashed the complete binding set and recounted the literal
operand's degree, support and exact coefficient bits; its audit SHA256 is
`ddef6f085a456efec0294dcc35f91f9ccfa75d6b349b1a6c33c1e73596f6d5b1`.

This observer run's degree-1478 root inventory is incomplete. No root or CAD completion, cache
injection, or production performance improvement follows. The captured height
exceeds the Taylor helper's unchanged 20000-bit default. Comparing it requires
a separately explicit research-policy extension with all existing encoding,
storage, exact-division and coefficient-equality checks retained.

## Explicit height policy and the authentic shift comparison

The second helper version keeps the default input-height limit at 20000 bits.
Its explicit research policies additionally allow 32768 or 65536 bits, with
the same degree, coefficient-storage, encoded-integer, output-text and worker
limits. The authentic VAS operand uses the 32768-bit policy. Its literal
coefficient strings and exponents are copied without arithmetic; the native
variable `symgcad::s` is bound to axis zero of the original full alias map
`[s,t,x0,x3,x5,x6,x8]`. Pinned native control flow identifies this as an
internal **negative-root VAS operand**, not the original block polynomial on
the positive axis.

Factorial convolution remains the default. The recursive method splits
`p(x)=p_low(x)+x^m p_high(x)` and forms
`p_low(x+a)+(x+a)^m p_high(x+a)`. It caches exact signed binomial powers, uses
native shifts below the chosen leaf length, and checks every packed product
by complete centered decoding and exact re-encoding. A third comparator
admits only `a=+1` or `a=-1` and performs the triangular recurrence with direct
integer additions or subtractions. It leaves the recursive method's native
leaves unchanged.

Every mode retains the conservative factorial admission envelope; recursive
and unit modes additionally apply their own bounded preflight. This can
decline inputs that a less conservative method-specific envelope would admit.
The selected input passed a 1,966,182,414-bit logical reservation bound;
this is not process RAM. The unchanged default correctly declined it before
integer construction, while the explicit 32768-bit plan passed.

Fifteen focused tests passed, including signed and rational identities,
zero/constant inputs, uneven recursive splits, 700-bit and 24000-bit
coefficients, unchanged storage limits, exact height boundaries and exclusive
CLI modes. The conventional release tests/build completed in 547 s with
3,875,008 KiB sampled aggregate peak RSS under an 8 GiB supervisor. All
production and helper source hashes remained unchanged during the build.
Only the research examples and this note changed; no production dispatch did.

All six fresh processes used the same standard release helper, the exact
captured literal, CPU2, a 60 s worker deadline and 2 GiB memory limit. Every
coefficient agreed with native `shift_var`, and all six serialized outputs
had the same hash. Kernel timings exclude input parsing and output writing;
each native comparison ran afterward in the same process.

| Method | Method kernel (s) | Native comparison (s) | Process wall (s) |
|---|---:|---:|---:|
| Factorial convolution | 0.657005 | 0.228648 | 1.116229 |
| Direct unit addition | **0.137993** | **0.227206** | 0.587678 |
| Recursive, leaf 32 | 0.470921 | 0.227367 | 0.913056 |
| Recursive, leaf 64 | 0.439413 | 0.226318 | 0.891871 |
| Recursive, leaf 128 | 0.372769 | 0.226544 | 0.832046 |
| Recursive, leaf 256 | 0.324307 | 0.226221 | 0.770503 |

The convolution variants showed no gain on this operand. Direct addition
used about 39% less kernel time in this single comparison. This motivated the
separate isolated native patch and complete root-inventory comparison below;
the shift comparison alone establishes no root-isolation or solver speedup. Earlier development
and observer timings are operand-selection evidence, not controls for this
release comparison. No native patch was applied to the active dependency.

The input SHA256 is
`e7545e4d5e0a651e4e843b7880297380bec42540757fd9f6147ec6b1f8fe0fb4`;
the common output SHA256 is
`a7a34434ae082b382274aed934b7566106df16608b0850f1e0c2a4a1929a4c17`.
The frozen release helper SHA256 is
`3c31be0bbbaf02a98e118b27f4784c6752cc16b7205debc7224b276424135a08`.
Bound inputs, commands, source manifests, individual process reports and the
checked summary are under
`artifacts/research/taylor-convolution-authentic-02/`; summary SHA256 is
`327462691b7e371f5782123d63ce3e453e3a3f51919c99c367582207d76b18ff`.
The test/build log SHA256 is
`413929f5cff2b5884c5fc7f7db653f7e86391179d4f0d34846ac119c646039b6`.
Independent review checked 49 bound files and the complete statuses and common
outputs; its audit SHA256 is
`17874e69e886705f9b0f321f6b94a7a636b2337d8eb3c11150b945aa0b85a936`.

For another bounded literal in the helper's strict full-map input schema:

```sh
cargo test --release --locked --example profile_taylor_convolution
cargo run --release --locked --example profile_taylor_convolution -- \
  INPUT.json --input-sha256 SHA256 --output FRESH_DIRECTORY \
  --input-height-bits 32768 --shift 1 --cpu 2 --seconds 60 --unit-add
```

Omit `--unit-add` for factorial convolution, or replace it with
`--recursive-leaf 32`, `64`, `128` or `256`. All completed comparisons require
exact native coefficient equality. No root inventory or original CAD
completion is claimed by this helper.

## Complete isolation follow-up in an isolated dependency copy

The measured unit-addition idea was applied only in an isolated Symbolica
checkout. For a shift of exactly `+1`, `shift_var` uses the same triangular
recurrence with ring addition; all other shifts, including `-1`, retain the
original fused multiply-add path. This is a generic arithmetic identity,
with no polynomial or graph identifiers in the dispatch. Two focused Z/Q
regressions compare the old fused recurrence with the patched method on large
signed coefficients, zero/constant polynomials and nonunit shifts. Eleven
isolation/comparison tests also passed. The active dependency and production
solver were unchanged.

The follow-up uses the original degree-1478 block, rather than the internal
VAS operand used in the shift microcomparison. Its full map remains
`[s,t,x0,x3,x5,x6,x8]`, with axis zero and literal expression SHA256
`90fbf50d36da8ef1841eef81fbbf6973a127ac01d21650927f8250b3004a0028`.
The exact input file SHA256 is
`4fa804c83098bf885ff26f11a301b3645f4aa38d4c2eb901ee341e80a0d7a058`.

All three fresh processes used CPU2 and a 4 GiB/1800 s worker limit. The old
full-real control replayed the complete signed D32 product before isolating
this block afresh; only the other three small-block inventories were
retained. Product replay took 45.500059 s and is excluded below. The two
patched runs used the same frozen helper, rational API and build profile,
sequentially on that CPU. The positive-only API is additive and retains the
native square-free and cross-factor separation steps.

| Query | Distinct roots | Isolation time (s) | Worker peak RSS (KiB) |
|---|---:|---:|---:|
| Old full-real control, symGCAD wrapper | 294 | 1138.549871 | 821288 |
| Isolated `+1` patch, native full-real API | 294 | 765.807915 | 832592 |
| Same patch/helper, native strictly-positive API | 145 | 557.982649 | 431304 |

The old control's timing includes symGCAD's retained-fiber and interval
preparation around the native call; the new helper times the native API only.
Both use development helpers with optimized native dependencies, rather than
the release shift-microcomparison profile. Thus the full-real control shows
a shorter measured complete-inventory path, but its exact timing difference
cannot be attributed solely to the `+1` branch. The two patched API timings
have matching scope: the strictly-positive query used 27.14% less isolation
time and 48.20% less peak RSS in this one comparison. It answers a smaller
query; these measurements are neither repeated benchmarks nor CAD timings.

The fresh old control's 294 intervals exactly matched the historical inventory.
A separate exact comparison then matched all 294 old/new roots through
sign-changing rational intersections. The positive result similarly matched
all 145 positive roots of the patched full inventory, including native
multiplicities. The legacy control did not export internal root multiplicities,
so the first comparison explicitly does not claim to verify them. Both
comparisons completed without an additional GCD, ambiguous-endpoint correction
or interval refinement. Their process times, 34.577002 s and 20.528488 s, are
separate from the isolation table.

The comparison helper's small tests exposed endpoint ownership assumptions:
a native interval can place its own root at an endpoint or border a different
root there. The helper now resolves ambiguous zero endpoints using exact open
root counts and source ordinals before matching. Positive ordinals exclude
zero. No native interval-refinement modification was needed or applied.

The frozen patched helper SHA256 is
`25b4872636fbdcc8e078642983ac29daa7cb1453fd0e84bc42051481b33e6e86`.
The independently applicable unit-shift patch SHA256 is
`245d220fb0db7a4bfd6283b04f2315af355781454778511f85812f6057d0a5e1`.
The reports and source/build manifests are under
`artifacts/research/positive-axis-api-01/`. Principal result hashes are:

| Evidence | SHA256 |
|---|---|
| Fresh old full-real block | `b9e11d22bceb2b4f4bdd6208f6db5a24db8306f86e6555fc7c54fcc9fe65946a` |
| Patched full-real inventory | `aaa79aebbdc4eb8aa4d048789373a165acaaae37189064cfdff3267904139319` |
| Patched positive inventory | `b5889499d50d240ad35ca3a63cb78075d9fde8ec0ef6c563f99c210a19674cc0` |
| Exact full/full comparison | `6b69261278cf8bcf443169542cd1b68b37a061f891971be454ebb96b4f31c6bc` |
| Exact full/positive comparison | `5c30b6cdb7243af4004e0ee497d22177448078b32df887c0f59d79a18140a900` |

The metadata audit separately rechecked file hashes, process completion,
ordered source bounds, witness intersections and timing scope. Positive-mode
metadata and comparison audit hashes are
`c6aa12fbe0c0eea5c7377274aa7f7a925c3d7c403d7507cb5156555520e3d7f1`
and `d71a8108d04215a37ea0c7f78feba2c1fa599bbeae9c3e634e82dff5b03567e9`.
This establishes exact isolated root-query results for the saved block. It
does not complete an original CAD or authorize removing negative-axis seams
from existing proofs. Production use of the positive-only API still requires
the domain, selector and coverage obligations in
[the positive-axis design](POSITIVE_AXIS_ISOLATION_RESEARCH.md).
