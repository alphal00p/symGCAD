# Exact checks of the bounded production resultant kernel

The opt-in production kernel reproduced both captured A317/A318 pair
resultants exactly, including rational scale and sign, before canonicalization.
Each expected polynomial had already passed an independent complete grid of
ordinary exact numeric Ducos resultants. This checks individual algebra
operations; the original symbolic decompositions remain unresolved.

| Capture | Build | Algebra, s | Whole process, s | Peak RSS, MiB | Terms | Certified grid points |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| A317 | release | 38.16 | 38.29 | 165.5 | 9,835 | 34,945 |
| A318 | dev, dependencies optimized | 20.03 | 20.93 | 164.1 | 18,939 | 42,813 |

These are single fresh processes on CPU 8, each capped at 60 seconds and
8 GiB. The builds differ, so the rows are not a performance comparison.
Exact comparison took 0.0011 and 0.0062 seconds respectively, separately
from the algebra timer. Input parsing and candidate loading are included in
the whole-process wall time. No decomposition speedup or coverage is claimed.

The first A318 production attempt declined the static envelope in 0.286
seconds: its elimination-degree product is 10 × 6 = 60, below the initial
minimum of 64. That failed attempt remains retained. The minimum is now 60;
the other size, coefficient, denominator and encoding caps are unchanged.
A focused regression admits a 10 × 6 pair and still declines a 9 × 6 pair.
All eight kernel and seven profiling-helper tests pass. The option remains
disabled by default, and independent proof verification still uses the
ordinary exact backend.

## Evidence and reproduction

The [compact audit manifest](binary-segmentation-production-check.json)
binds each original capture, expected polynomial, complete-grid verification,
worker outcome, source snapshot, binary and test log by SHA-256. It contains
metadata only; the private polynomial captures and generated results remain
under ignored `artifacts/` directories. Both workers exited successfully,
without truncated output, and their reported equality is exact polynomial
equality rather than equality of canonical hashes.

The A317 release helper has SHA-256
`ab4bc8893bc1f2b527c8e0d2a6de00e8a88f94a6b05a5440705c0b1e90a86be3`.
The A318 helper, rebuilt after the cutoff change, has SHA-256
`20e1b700fc8e7597c2648b9965ececadd86054928cb5fbc0f4bbdc1ba5c0435c`.
The latter's exact module snapshot and replayable cutoff patch are also
bound in the manifest. The earlier research method and complete-grid
verification are described in [the original note](BINARY_SEGMENTATION_RESULTANTS.md).

With the retained local captures and independently certified candidates:

```sh
nix develop --command cargo build --example profile_operation
nix develop --command taskset -c 8 target/debug/examples/profile_operation \
  artifacts/research/symbolic-witnesses-05/a318-last-operation.json \
  --strategy binary-segmentation \
  --expected-result artifacts/profile/symbolic-mcts-03-discriminants/kronecker/a318-witness-resultant-trial.json \
  --seconds 60 --memory-mib 8192 \
  --output artifacts/profile/a318-production-new-attempt.json
```

Use a fresh output path and inspect both the operation report's
`expected_comparison.exactly_equal` and the accompanying process status.
The supervisor preserves failures as evidence; its own successful return
does not imply that the child completed.
