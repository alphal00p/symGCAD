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
