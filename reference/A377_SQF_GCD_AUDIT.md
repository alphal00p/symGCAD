# A377 D32: native square-free/GCD path and retained proposals

The important correction is that the first D32 native square-free call
**returned in 293.53293782 seconds**. The 300.059206-second worker expired
in `factor_reconstruction`, after that native call, without a completed
factor product. The final progress file's unchanged SHA is
`8a15e741571bf773cbf4d5878d3d9ac3bd61ed2c1f25f2af954986ae7c787253`.
A native return is useful cost evidence, but does not make its unrecorded
factors verified. The old failed run remains unchanged.

This audit reads Symbolica revision
`75f8350094b90254ee71dc2a391fde0d14b0204a`, with a clean source checkout.
No native backend, solver, cache, or production limit was changed. The
separate [complete grid](A377_D32_GRID_VERIFICATION.md) now establishes
that the supplied signed D32 polynomial is the captured discriminant;
that identity does not establish its factorization or remove its zeros.

## Source path actually selected

Rational-field square-free decomposition removes rational scalar content
and delegates to integer square-free decomposition
(`src/poly/factor.rs:4928`). The integer implementation strips the common
monomial before Yun's derivative/GCD stage (`:4642`, `:2995`). Thus another
manual removal of `s^798` would duplicate existing preprocessing. The
first nontrivial D32 input is the dense degree-3260 residual, with 3,261
terms and up to 33,667 coefficient bits.

`MultivariatePolynomial::gcd` removes monomial shifts and common exponent
compression before dispatch. For this large one-variable input,
`IntegerRing::heuristic_gcd` chooses the univariate modular route rather
than scalar evaluation (`src/poly/gcd.rs:7453`, especially `:7516`).
The direct scalar route's selection threshold is only 8,192 estimated
bits and large degree also disables its heuristic admission.

The modular route has several optimizations already. They should not be
proposed again as missing features:

- `DenseZp64UnivariateGcdImage` (`gcd.rs:4093`) reduces both integer inputs
  to degree-indexed 64-bit finite-field buffers. The 4,096-coefficient
  bound admits the first residual/derivative pair. Its `remainder` and
  `run` methods (`:4218`, `:4280`) use ordinary dense Euclidean long
  division. This inspected path is **not half-GCD**. The generic field
  fallback also uses Euclid.
- `UnivariateModularGcdContext::new` (`:4785`) removes integer contents,
  computes the GCD of the leading coefficients, and can instead normalize
  images by the GCD of their constant coefficients when that predicts
  fewer initial prime images. It already avoids blindly selecting a much
  larger leading-coefficient scale.
- The CRT loop (`:4928`) keeps the smallest observed modular GCD degree,
  skips primes losing input leading terms or the chosen scale, and
  restarts when a smaller degree appears. It tries exact reconstruction
  starting at the selected normalization's bit length plus two, with
  geometrically spaced retries. It **does not wait for a universal full
  coefficient bound**.
- Reconstruction first checks necessary divisibility after evaluation at
  one, then exact polynomial division into both original inputs
  (`:4872`). If constant normalization is selected, an additional
  leading-normalized reconstruction is tried on its own schedule.
- CRT merges already use a precomputed modular inverse, in-place updates
  for equal supports, and one word correction per coefficient
  (`src/poly/polynomial.rs:7115`). There is no justified proposal here to
  replace a generic whole-polynomial CRT that this route does not use.
- Exact cofactor certification has bounded dense division and a special
  two-ended, fully product-checked method for near-balanced dense products
  (`gcd.rs:4317–4717`). The latter is not half-GCD; it accelerates a
  different operation after reconstruction.

The source hashes are:

| File | SHA-256 |
|---|---|
| `src/poly/gcd.rs` | `35332a1b91b5b970c0b3a86f0b3a03aedad1e8ee9026d380b67a35e0f7a3b357` |
| `src/poly/factor.rs` | `d5eaea5bc76e04aa29a0f481f94ba43580230d314fb0b182faebd9fe469e3fa1` |
| `src/poly/polynomial.rs` | `0b027b7db5ed8de255b75a1ad7d6accb8af93c948ad7100bc46050e0596e0e74` |

These files are under the ignored native Symbolica checkout. Line numbers
refer to that exact revision, not a generic claim about future versions.

## Concrete opportunities, with missing measurements explicit

There is one directly visible duplicate operation. Successful modular
reconstruction returns the GCD **and both exact cofactors**.
`gcd.rs:3758–3767` retains only tuple member zero. Yun's routine immediately
computes `self/c` and `derivative/c` again (`factor.rs:3024–3036`), and its
later loop similarly divides after a GCD. A narrow internal API retaining
cofactors could avoid repeated exact divisions. It must preserve all
monomial, content, variable-map, sign, and exponent-compression corrections;
reusing cofactors from an earlier normalized pair without undoing those
changes would be wrong. No such patch is implemented or timed here.

Before changing this API, a bounded profile should distinguish time spent
in finite-field remainders, input modular reductions, CRT merges,
unsuccessful reconstruction probes, successful exact division, and the
subsequent Yun divisions. Useful counters are GCD input/output degrees,
number of good/bad prime images, chosen normalization and its height,
probe moduli, and whether balanced checked division was admitted. The
existing single SQF timer cannot separate these costs.

Half-GCD is a mathematically relevant alternative to the visible quadratic
Euclidean loop, but it would be a substantive native implementation. It
should be considered only if modular remainders dominate measured time,
with exact comparisons against the existing Euclidean result across
unlucky primes, zero inputs, leading drops, sparse maps, and large-degree
fixtures. The current source already uses cheap Montgomery-style word
arithmetic and dense storage. A faster asymptotic algorithm alone does
not predict its crossover point on these operands.

Likewise, geometric reconstruction backoff can overshoot the first
sufficient modulus, but increasing probe frequency may cost more large
exact divisions. No cap or schedule change is justified without the
per-probe measurements above.

## D24 comparison and known-factor reuse

The earlier degree-2169 D24 discriminant completed native square-free
work in 30.579 seconds in one development helper, followed by 5.246 seconds
of signed product replay. Its separately predeflated residual took 29.001
seconds native plus 5.367 seconds replay and an extra 1.925 seconds of
preparation: that predeflation did not improve total time. A later matched
release pipeline took 16.608 seconds for decomposition and 2.544 seconds
for signed replay. These different builds and operands are not a D32/D24
speed ratio.

D24 includes a degree-881 block with multiplicity two and a degree-111
block with multiplicity one. The 881 block has 35 real roots above four.
Its square must remain in the exact product and its roots remain part of
any required boundary inventory. Details and immutable evidence are in
[the D24 factorization note](A377_UNIVARIATE_FACTORIZATION.md) and
[the matched pipeline](BASE_UNIVARIATE_BASIS_PROPOSAL.md).

The parallel modular screen of known completed cache factors and these
D24 blocks is a useful prerequisite to exact division. A nonzero remainder
at a valid prime rules out the proposed integer divisor; zero modular
remainders only nominate a possible divisor. Any accepted removal requires
exact division and full signed product replay, repeated to the measured
multiplicity, while retaining every extracted factor. The remaining
residual must still be decomposed or isolated with all its zeros present.
A known high-degree repeated divisor could materially reduce the next
GCD; many rejected or small known factors cannot establish that saving.
The screen alone is never a factor certificate or nonvanishing proof.

## Restartable, unverified SQF proposals

The research helper
[`profile_univariate_content`](../examples/profile_univariate_content.rs)
now publishes a bounded factor proposal **before** expensive reconstruction.
Native output is checked for full map, integer coefficients, total storage,
positive multiplicities, weighted source degree and product-work bounds.
A same-directory atomic no-overwrite publication retains full signed sparse
expressions, their hashes, multiplicities, and native elapsed time.

The proposal is explicitly `unverified_native_factor_proposal`, with
`exact_product_verified:false` and `square_freeness_certified:false`.
Its bindings include the original capture/candidate digests, candidate
expression digest, canonical-source expression digest, complete variable
map and signed rational scalar. Merely writing this file does not complete
the worker or authorize a support block.

`--resume-proposal PATH --proposal-sha256 SHA` skips native SQF and loads
only this narrow bounded format. Every expression and the aggregate
coefficient storage are preflighted before proposed integers are parsed;
all source bindings are checked anew. Both fresh and resumed paths require
complete canonical product equality and signed original reconstruction
before setting checked-product flags. A forged same-degree factor can
pass shape admission, but the exact product test rejects it. No SQF or
irreducibility label is trusted by replay.

Only this research stage's maximum was increased from 300 to 3,600 seconds;
content-only remains at most 60 seconds and the worker memory cap remains
4 GiB. Production caps are unchanged. Inputs are hashed before and after,
all outputs must be fresh, and interruption retains the unverified proposal
and the last atomic progress record. A resumed report distinguishes the
previous native time from its current load/reconstruction times.

Ten focused tests pass, including signed proposal roundtrip, source/hash/
coefficient/multiplicity tampering, static caps, deadline bounds, and
atomic no-overwrite publication. A small actual CLI smoke produced a
proposal for signed `-4*s^2` and then successfully reconstructed it in a
fresh process with `native_square_free_called:false`. No native result
from the D32 run was replaced by that fixture.

Build tests and executable compilation used CPUs14–15, two jobs, 4 GiB and
separate 300-second deadlines. The frozen helper SHA is
`ae88b349c1a2580ebe53ff97ad36021d4647e76cb63d3653cdaeda206c99a72a`;
source SHA is
`43b1c48ce8f47b46140957ad9bcae9e2d91a33ab283eba0c7d3dd5eddb492396`.
The build/source manifest is
`artifacts/research/univariate-content-checkpoint-build-01/manifest.json`,
SHA `4e8ec5c3835f6b214af7194c2342b4489b97072effcc39df99403f5f56eaaa79`.
Independent source review found no blocker.

The fresh original D32 SQF-and-reconstruction run under
`artifacts/research/a377-degree32-content-03/` **completed in
330.869262415 seconds**, at **211,512 KiB peak RSS**, on CPU12 with a
4-GiB/3,600-second worker limit. Native SQF took 285.499570642 seconds,
proposal publication 0.300279570 seconds, and complete signed product
reconstruction 41.468249620 seconds. This is one development-helper run,
not a repeated benchmark or an original CAD completion. All original
input files remained byte-identical.

The exact integer primitive source has the following product blocks.
The final report verifies their complete signed product; it does not
independently certify square-freeness or irreducibility.

| Degree | Multiplicity | Terms | Maximum coefficient bits | Coefficient storage bits, including unit denominators |
|---:|---:|---:|---:|---:|
| 1 | 798 | 1 | 1 | 2 |
| 268 | 1 | 269 | 2,783 | 698,891 |
| 1,478 | 2 | 1,479 | 15,417 | 21,322,867 |
| 3 | 12 | 4 | 4 | 15 |

Their weighted degrees sum to 4,058. The cubic is
`s^3-6*s^2+9*s+4`. The coordinate factor and cubic, both high multiplicities,
and the entire degree-1,478 square remain in the reconstruction. No roots
were isolated or omitted.

Current production support-basis limits were inspected without changing
them. The comparison is precise for this one source in an otherwise empty
retention budget:

| Obligation | Observed | Current limit | Result |
|---|---:|---:|---|
| Canonical source coefficient height | 33,667 bits | 20,000 | decline |
| Canonical source total coefficient storage | 102,695,453 bits | 64,000,000 | decline |
| Canonical source text | 30,948,511 bytes | 33,554,432 | fits |
| Source degree / terms | 4,058 / 3,261 | 4,096 / 4,096 | fits |
| Largest block height | 15,417 bits | 20,000 | fits |
| Total block coefficient storage | 22,021,775 bits | 64,000,000 | fits |
| Source plus block retention | 124,717,228 bits | 128,000,000 | fits alone |
| Reconstructed product height bound | 34,518 bits | 65,536 | fits |
| Product work bound | 140,108,562 | 256,000,000 | fits |

Every individual nonconstant block also fits the existing degree, term,
map and coefficient-storage bounds. Only 3,282,772 bits would remain in
the shared retention budget after this isolated source, so earlier sources
in an actual solve can change that result. The source admission failures
remain decisive: checked factors do not bypass them. The signed raw source
is even larger; the recently corrected proportional-source checker lookup
does not make an over-cap canonical source admissible.

Warm04's retained evidence has no completed result or local-progress report;
its final capture records the pending discriminant, not the Builder's source
inventory. The current progress type reports source/block counts and times,
but not retained coefficient bits. Therefore these records do not establish
the aggregate retention needed by that traversal. In particular, the
128-million-bit limit is **not** shown sufficient for adding D32 after all
earlier accepted sources.

The exact product replay is bound to the durable proposal's hash in the
completed report. The proposal file itself stays labeled unverified; it
is not rewritten into a proof or injected into a solver cache.

| Artifact | SHA-256 |
|---|---|
| `report.json` | `bdd0a2eed71f1496f90af78ec9e381a9e7e27ccb19c5bb113bbc36a128dd12fd` |
| Completed `report.progress.json` | `5af81edb29b8d952fb37e59bb96dfed5e0afd1670511b515ed8372ecb7d9d3cf` |
| `report.factors.json` | `3836f35e4e26d1ae51fbe674c963a28782590710e01d404839f54007614994c2` |
| `completed-summary.json` | `1213fd20d74d89a88d2120de392b29756470e00994eac8f160475ea322aa1c18` |
| `launch.sh` | `11e3cc0595f7a2bf50f7efa32aa2be8f19a06e1cb37fb5593863307de294bf52` |
| Independent metadata audit `independent-audit.json` | `1de20c726d90419eb115351df47b925af6e0c5293740b71f2318727761e7c2ef` |

The independent audit rehashed source, executable, native source and original
inputs; checked the successful process and immutable proposal binding; and
matched factor hashes, full map, signed scale and multiplicities. It did not
repeat native SQF or claim a separate square-free, coprimality or root proof.

The failed 300-second run is not overwritten. Reconstruction-only resume
is available using this exact proposal hash, the same authenticated source
files, and a fresh output path. No solver checkpoint, root inventory, or
CAD result is produced by this workflow.

The later sparse five-factor modular screen completed in 7.337 seconds.
Whole D24 blocks of degrees 881 and 111, and `s-4`, are nondivisors modulo
all three checked good primes. Only `s` and `9*s-10` survived that screen;
no characteristic-zero multiplicity or quotient was produced. Partial
shared factors inside the composite D24 blocks remain possible. Therefore
this evidence does not support cheaply removing either entire large block
before D32's GCD. The broader known-cache-factor screen timed out after
120.031 seconds, so it supplies no complete inventory conclusion. The
successful five-factor report is
`artifacts/research/a377-known-factor-screen-02/report.json`, SHA
`5ccc9470c06375addb52595ee7850c8ad9f6d7f031c68264c44b42bc8dd4a0e7`.
