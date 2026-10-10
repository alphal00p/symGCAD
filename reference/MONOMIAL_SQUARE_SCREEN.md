# Bounded monomial-square positivity screen

The bounded screen found **no additional certificates** among 281 authentic
production declines, after 552 candidate square identities. It extends the
*discovery* of square-plus-strict-residual identities to binomials in monomials.
The existing production square methods use
coordinate binomials or affine coordinates. This screen changes no production
certificate, projection rule, or exceptional set. Its inputs are the authentic
declines retained by the [recursive quadratic screen](RECURSIVE_QUADRATIC_SCREEN.md).

## Exact certificate and finite search

For either signed target `p` or `−p`, suppose the sparse polynomial contains
positive diagonal coefficients `A,B` at exponents `2β,2γ`, and a negative
coefficient `−K` at `α=β+γ`. For a positive rational

```
K/(2A) ≤ r ≤ 2B/K,
w = K/(2r),
S = w (x^β − r x^γ)^2,
R = sign*p − S,
```

the square matches the negative cross coefficient exactly and leaves
nonnegative diagonal coefficients in `R`. These observations only propose
an identity: acceptance requires the ordinary production portfolio to prove
**strict `R>0` on the complete target-free domain**. Independent replay checks
that strict certificate and the exact signed identity. The square alone,
nonnegative residual, or sampled positivity is insufficient. In particular,
common zeros such as `xy=z²` remain real interior boundaries.

The finite ratio menu is `1` when admissible, the two interval endpoints, and
their midpoint, with duplicates removed. At most 32 candidates are attempted
per polynomial, shared across both signs. Input bounds are 128 terms, 16 map
coordinates, degree at most 32 in each coordinate, and 512 bits of combined
rational numerator/denominator height. Square weights and ratios have
2,048-bit individual bounds and a conservative 4,096-bit product estimate
before multiplication. Residuals have at most 131 terms and 4,096-bit
coefficient height. The ordinary residual portfolio keeps its existing limits.
The entire worker, including source reading and native algebra, is killable
after 60 seconds at 4 GiB; a soft 85% deadline stops further discovery.

[`profile_monomial_squares.rs`](../examples/profile_monomial_squares.rs) checks
explicit hashes of the prior scan and its input audit, the exact variable map,
normalized original constraints, and the derived domain. Each attempted
polynomial is rebound to one original factor record, its file hash, and its
`(output,multiplicities)` checksum. The earlier input audits cover all source
records. These provenance checks do not grant cached algebra proof authority.
Sparse coefficient/exponent syntax is bounded before conversion; arbitrary
power expressions are rejected. The current production certificate portfolio
is rerun before any new square attempt.

## Results and limitations

Both final scans used the same immutable development-build executable,
sequentially on CPU12 with one worker and the limits above. All prior declines
remained declines when the current production portfolio was rerun.

| Inventory | Declines checked | Square attempts | Additional certificates | Worker internal wall | Process wall | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| A414 | 173 | 443 | 0 | 0.399367 s | 0.425476 s | 10,392 KiB |
| Physical elliptic | 108 | 109 | 0 | 0.284526 s | 0.304822 s | 12,940 KiB |

No candidate, time, or coefficient-height budget was exhausted. Maximum
candidates attempted per polynomial were 14 and 26, respectively. These are
single diagnostic process measurements, not a solver benchmark.

The A414 candidate geometry explains part of that result: 47 polynomials have
no matching diagonal pair, 22 have diagonal pairs but no admissible positive
ratio, and 104 admit at least one pair (200 pairs in total). All 443 residual
proof attempts decline; this does not establish that every residual or target
has a zero. At least 37 targets are nonconstant linear polynomials in the
unrestricted coordinate `t`. Their nonzero coefficient polynomial cannot
vanish throughout the remaining open box, so each takes both signs on the
full domain. Of those 37, 17 reached the square portfolio and 20 had no candidate.

Two further exact examples on `s>4,x1>0` illustrate genuine obstructions:
`s²−4s−4s*x1+4` equals `−11` at `(5,1)` and `24` at `(10,1)`;
`−2x1²+2s+s*x1` equals `13` at `(5,1)` and `−140` at `(5,10)`.
The other positive coordinates may all be one. Their zeros cannot be removed
by any strict nonvanishing certificate on this domain.

The physical dataset retains `s>4,t<0,0<pp4<4`, positive
`x0,x1,x2,x3,x5`, and the positive constant anchor, without the original
target sign as a premise. Its 108 declines comprise 33 polynomials with no
diagonal pair, 52 with pairs but no admissible ratio, and 23 with admissible
pairs (60 pairs in total). All 109 residual proof attempts declined.

The first physical worker failed closed before
algebra because its untagged historical domain fingerprint differs from the
tagged A414 format; that failed record is retained. Support for both explicit
formats also checks the normalized constraint list, rather than weakening
the mathematical input binding.

No result here changes corpus coverage or demonstrates whole-solver speed.
The successful synthetic identity `(2xy−3z²)²+1` shows that the research
rule is broader than the current portfolio, but is not evidence of practical
benefit on the retained workloads. This finite menu operates in the stored
coordinate basis and subtracts one square; its failure does not rule out
other exact certificates. The observed sign-changing targets instead require
their interior zero sets to remain in the decomposition. Neither completed
screen justifies a production extension.

## Reproduction and evidence

Four focused tests pass: exact weighted-monomial identity and independent
strict replay, altered weight/ratio/residual/sign rejection, common-zero
rejection and original-domain binding, and the two explicit audit layouts.
The helper source and full library source hashes used for the build are
recorded in `artifacts/research/monomial-square-build-02/build-provenance.json`.
The binary SHA-256 is
`033ab0a36a3a1ebe84c40cecd9734286c2023d1c2bc600ba3b85d9e035790487`.

Build and tests used the project environment, a 300-second outer deadline,
CPU14–15, two Cargo jobs and a 4 GiB address-space bound:

```sh
source .symgcad-env
timeout 300s nix develop -c taskset -c 14,15 bash -c \
  'ulimit -v 4194304; cargo test -j2 --example profile_monomial_squares && cargo build -j2 --example profile_monomial_squares'
```

Run the frozen helper with a fresh output path and the corresponding prior
scan and audit hashes below. Its Rust supervisor enforces the worker limit:

```sh
artifacts/research/monomial-square-build-02/bin/profile_monomial_squares \
  SOURCE_SCAN --scan-sha256 SCAN_SHA \
  --input-audit SOURCE_AUDIT --audit-sha256 AUDIT_SHA \
  --output FRESH_REPORT --cpu 12 --seconds 60
```

| Evidence | SHA-256 |
|---|---|
| `recursive-quadratic-a414-02/scan.json` | `ba58651d94563cd70e800c4df7583b80b86dc45f6cb933be7543d1637e8b2a1c` |
| `recursive-quadratic-a414-02/input-audit.json` | `ce33f89a740e350cfd502204d4b61388488bf11c0b475140fd3e0894f4542568` |
| `recursive-quadratic-physical-01/scan.json` | `ba5ac907cc48b24456b30dbcca6abe4227516465ba44cd7de42d11253f4c51c9` |
| `recursive-quadratic-physical-01/input-audit.json` | `0f2d4090446896081c4f22cde1a5fdf600d4ea8ee4f58a762dec64096cdda4a7` |
| `monomial-square-a414-02/report.json` | `510e43f486bdd9edbf9fa939ff6935fcc25854b2df09405ad3958a9a7bcb8c2b` |
| `monomial-square-physical-02/report.json` | `116fd33972324e5224204b2df71633cda7c9d4245ddd3e9df5512f55d25935a8` |
| `monomial-square-build-02/build-provenance.json` | `efe2c5fe7b758e8238949e737cbbd596e8825f2ef2998f3f07546a5385e53274` |

All paths in the table are relative to `artifacts/research/`. The earlier
A414 result is retained in `monomial-square-a414-01/report.json`; its exact
candidate results match the final run. The physical metadata-layout failure
remains in `monomial-square-physical-01/report.json` with SHA-256
`a9e11885e149da7dc8fe9255c3ddc2bb1f8de21b461ffe66c0ed103a2e86fc28`.
It provides no completed-scan evidence. The initial compile failure is also
retained in the first build directory; the final source passes all four tests.
