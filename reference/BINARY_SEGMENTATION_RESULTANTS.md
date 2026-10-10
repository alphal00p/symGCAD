# Exact binary segmentation of captured resultants

`examples/profile_kronecker_resultant.rs` is the original research helper. It
completed two captured symbolic A317/A318 pair resultants
that the direct exact backend did not finish in matched 60-second trials.
The original decompositions remain unresolved.

| Captured operation | Encoded solve, s | Peak RSS, MiB | Direct integer Ducos, s | Result terms | Independent exact checks |
| --- | ---: | ---: | --- | ---: | ---: |
| A317 pair resultant | 37.39 | 163.7 | timeout at 60.03 | 9,835 | 34,945 / 34,945 |
| A318 pair resultant | 19.06 | 158.3 | timeout at 60.03 | 18,939 | 42,813 / 42,813 |

These are single fresh-process diagnostic measurements, with dependencies at
optimization level 2, one assigned CPU, and 8 GiB per worker. They are not
release parity ratios. Each candidate was independently checked on a complete
nested grid of exact rational specializations, using ordinary numeric Ducos
resultants rather than binary encoding. Verification took 37.99 and 32.05
seconds respectively. Degree-dropping specializations were skipped with exact
checks and degree bounds; a partial grid cannot certify equality.

The same strategy timed out on the earlier four-active-variable, degree-eight
discriminants: 60.08/60.12 seconds and 730.4/845.7 MiB peak RSS. Their output
bit bounds were 533,306,592 and 704,431,992, compared with 45,113,995 and
49,020,885 for the successful pair resultants. Applicability and cost caps
would therefore be essential before considering a production path.

## Opt-in solver path

`src/resultant_segmentation.rs` implements this exact method with bounded
preflight before denominator clearing and integer encoding. Set
`solver.binary_resultants = true` to try it for pair resultants in optimized
or local projection. The default is false. Pair limits admit exactly two
surviving active parameters, elimination-degree product at least 60,
elimination degrees at most 64, at most 4,096
combined input terms, 65,536 output digits, and 96 million encoded output bits.
Separate coefficient, common-denominator and total encoded-input limits apply.
The two captured successful pairs fit these limits; the earlier four-active-
variable discriminants do not. This size bound does not bound intermediate native memory or runtime,
so the usual killable worker limits remain necessary.

The output limit was initially 64 million bits. A subsequently recovered
A377 pair needs a certified 83,605,434-bit bound: the research encoder completed
it in 95.189 seconds at 327.5 MiB peak RSS, followed by a complete independent
43,681-point signed identity check. This motivates the current 96-million-bit
limit; all other applicability limits are unchanged. The
[A377 diagnostic](A377_BINARY_RESULTANT_DIAGNOSTIC.md) records the exact input,
output and verification evidence. This is an operation-level result, not
an original-problem completion or matched end-to-end ratio.

A static decline keeps the ordinary exact backend. Cancellation, runtime
resource stops and failed exact reconstruction propagate without launching
another expensive attempt. Completed results share the existing canonical
operation caches; reports separately count segmented resultants and their
largest output-bit bound. Baseline projection and low-degree discriminant
identities retain their existing backend.

The same explicit flag also enables discriminants with one or two surviving parameters:
try existing exact interpolation first, and only after a static decline try
segmentation for `Res(p,dp/dx)` with the unchanged size caps. The signed result
is divided exactly by the leading coefficient and receives the conventional
discriminant sign. A separate `segmented_discriminants` counter records use.
Local proof replay independently recomputes these opted-in discriminants using
the same pure exact helper, without consulting discovery caches or supplied
operation results. Global proof replay and pair-resultant replay are unchanged.
The [A377 discriminant diagnostic](A377_BINARY_DISCRIMINANT_DIAGNOSTIC.md)
records the independently grid-verified input, downstream factor timeout and
scope limitations.

The two-parameter extension addresses the captured Ax318 degree-10
discriminant described in [the continuation audit](CROSSED_DISCRIMINANT_CONTINUATION.md).
Its 69,383,424-bit output bound fits the existing 96-million-bit ceiling;
only the former one-parameter gate excluded it. No encoding, degree, memory
or coefficient limit is enlarged, and all interior boundary polynomials
retain their exact signed values. Three-parameter discriminants still use
the ordinary backend. Actual native memory and time remain subject to
killable worker limits rather than being inferred from the output bound.

Eight focused module regressions compare exact signed rational results with
ordinary Ducos, exercise variable maps, repeated/shared factors, centered
decoding, static limits and cancellation. The full fast suite passes 489
tests, including opt-in cache dispatch, static fallback and historical proof
metadata. The profiling helper accepts `--strategy binary-segmentation` and
`--expected-result FILE`: it binds the original input and compares the full
signed polynomial with a saved, independently certified candidate before
canonicalization. Its seven tests include rejection of altered signs/scales.
The [production comparison note](BINARY_SEGMENTATION_PRODUCTION_CHECK.md)
records exact equality with both independently certified captured results,
including the retained initial A318 static decline and its cutoff adjustment.

## Proposed verifier backend, not implemented

Independent local and global replay currently reconstructs pair resultants
through `projection::exact_resultant`: bounded one-parameter interpolation or
ordinary Ducos. The two captured two-parameter operations therefore expose a
possible replay bottleneck, but no completed hard-problem proof has yet shown
that bottleneck end to end. Pair-resultant verification remains unchanged until
such evidence exists. The narrow opted-in discriminant recomputation above is
separate and introduces no operation witness or proof-schema change.

The narrowest future option requires no proof-schema change: after independently
reconstructing the operands and variable map, replay could freshly run the
bounded deterministic segmentation kernel. It would not read discovery caches
or accept the supplied factor basis as authority. Exact sign, rational scale,
degree bounds, decoding and reconstruction would retain the kernel's existing
checks. This is another exact recomputation backend; algorithm diversity is
provided by the existing captured full-grid checks, not assumed from running
the same kernel twice.

A separate candidate-witness design would need additional self-contained
operation records containing the exact signed raw resultant, its ordered
operands and eliminated axis. Current canonical cache values lose scalar/sign,
and final factor tables also lose multiplicities, so neither is that witness.
The verifier could instead validate each supplied raw candidate using the
existing research helper's complete nested degree grid. At every recursion
level it needs `D_i+1` distinct accepted values for an independently derived
resultant degree bound `D_i`; adaptive node sets are valid by polynomial
identity induction. Exact leading-degree drops are skipped, with the number
of possible skips bounded by the two leading-coefficient degrees. Every leaf
must compare against an ordinary exact numeric resultant with both original
elimination degrees preserved. A total point count or partial successful grid
alone cannot certify equality.

Any extraction of that verifier would add input/degree/height and grid-size
caps before denominator clearing, retain signed rational scaling, reject
unknown support and perturbed candidates, and propagate interrupted grids as
unverified. It would not trust stored sample values or a producer's claimed
degree bounds. Witness plumbing is a larger change than fresh recomputation
and is not justified solely by the current operation timings.

## Why reconstruction is exact

Clear rational denominators to obtain integer polynomials P and Q of degrees
m and n in the eliminated variable x. Every Sylvester determinant term uses
n coefficients of P and m of Q, with total x-index weight mn. For each
remaining variable y, two valid degree bounds are

```
n deg_y(P) + m deg_y(Q)
n max(e_x+e_y in P) + m max(e_x+e_y in Q) - mn.
```

Take their minimum, enlarged if necessary to contain the input support too.
Mixed-radix weights then inject this entire rectangular exponent box into
nonnegative integers. The determinant's coefficient L1 norm is bounded by
`C = ||P||_1^n ||Q||_1^m`: expanding all independent row-entry choices bounds
the permutation sum. No probabilistic termination is involved.

Choose a power-of-two base B strictly larger than twice C and twice every
input coefficient magnitude. Encode each parameter as the corresponding
power of B and compute one exact univariate integer resultant in Symbolica.
Balanced signed decoding recovers the unique bounded coefficient vector.
Recursive centered blocks avoid a quadratic sequence of large shifts. The
helper checks the decoded bounds, exact re-encoding, preserved leading
degrees, denominator homogeneity, and the discriminant's exact leading
coefficient division when applicable. Zero and constant elimination inputs
are explicitly outside this prototype's interface.

Reversible Kronecker substitution and polynomial determinant coefficient
bounds are established methods; see [Law and Monagan's structured polynomial
matrix paper](https://www.cecm.sfu.ca/~monaganm/papers/MarshCASC2016.pdf).
The paper is background for the method, not evidence for these GCAD timings.

The pinned numerica `Integer` left-shift fast path overflows at primitive
representation boundaries. This helper explicitly shifts through Symbolica's
arbitrary-precision representation. The separately replayable fix and its
boundary regressions are in [the patch documentation](../patches/README.md).
Neither the reference checkout nor production algebra was modified here.

## Reproduction and provenance

```sh
cargo build --example profile_kronecker_resultant
target/debug/examples/profile_kronecker_resultant capture.json \
  --seconds 60 --memory-mib 8192 --output candidate.json
target/debug/examples/profile_kronecker_resultant capture.json \
  --verify-candidate candidate.json --seconds 600 \
  --memory-mib 8192 --output verified.json
```

The supervisor refuses to replace existing evidence and records input and
binary hashes, process status, elapsed time and peak RSS. Static degree,
coefficient and output-bit caps precede large allocations. Four helper tests
cover signed decoding, rational scaling, shared roots, odd linear signs,
invalid/capped inputs, complete grids, and rejected perturbed candidates.

Retained measurements and exact polynomials are under
`artifacts/profile/symbolic-mcts-03-discriminants/kronecker/` and remain
untracked. The generation binary hash is
`83a1b0ef5527c8d68b504083c489132bf9a88c5ce6529f9fc007a0e216d708cb`;
the independent-verification helper hash is
`44428e764331e44d295eb4b8dae17201d39fe7b1a8573a240a499b1fea95ec09`.
The captures come from the original fixed-order witness-enabled attempts in
`artifacts/research/symbolic-witnesses-05`, not reduced research domains.
