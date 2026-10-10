# Exact binary segmentation of captured resultants

`examples/profile_kronecker_resultant.rs` is a research helper, not a production
dispatch rule. It completed two captured symbolic A317/A318 pair resultants
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
