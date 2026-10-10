# Exact affine preconditioning of the A377 degree-32 operand

The research gate succeeded: an exact integer substitution removes a known
`s^28` coefficient content and reduces the parameter degree from 84 to 64.
The resulting static integer-encoding bound is 172,067,945 bits, versus
210,179,055 bits for the original operand. Both remain above the production
96-million-bit cap. No transformed discriminant, speed improvement, original
CAD completion, or production change is established.

This follows the checked constant `5/18` in
[the linear-ancestor and collision-index research](ITERATED_RESULTANT_RESEARCH.md).
It is internal discriminant preconditioning, not a new chart for the user's
input. The source is the full-map `D_t` capture at
`artifacts/research/a377-warm03-progress-01/capture.json`, SHA-256
`bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75`.
The original polynomial expression SHA is
`59b41f78e9e212a5c4237595ed766b15f91922e6f631696ba9e0c7b3fa870948`.
No active solver cache or original input was modified.

## Integer identities and retained boundary factors

For `N=32`, the helper constructs the following polynomial without rational
substitution. The new coordinate `u` occupies the old `t` slot internally;
all other variable-map entries are preserved.

```
P*(s,u) = 18^32 p(s,(5+s*u)/18)
        = sum_i p_i(s) (5+s*u)^i 18^(32-i).
```

It independently reconstructs the denominator-cleared inverse:

```
sum_j P*_j(s) (18*t-5)^j s^(32-j) = 18^32 s^32 p(s,t).
```

Signed scalar normalization, native coefficient GCDs in `Q[s]`, exact
quotient division, and full product replay give

```
P*(s,u) = K s^28 Q(s,u),
K = 57607655128752665137801420266270621696.
```

The helper verifies this signed identity directly. The exact discriminant
scaling law, with every content and coordinate factor retained, is

```
18^992 s^992 Disc_t(p) = (K s^28)^62 Disc_u(Q).
```

Equivalently, cancelling the common polynomial factor in the integral
domain `Q[s]` gives

```
18^992 Disc_t(p) = K^62 s^744 Disc_u(Q).
```

This cancellation is an identity of polynomials; it is not division by a
numerical value at `s=0`. In particular the factor `s^744` remains in the
original discriminant. Any later reconstruction must retain it and the
signed rational unit `K^62/18^992`. No roots of `Q`, repeated-root collision
factors, leading-coefficient factors, or original interior boundaries have
been discarded. The calculation does not establish the remaining valuation
or factorization of either discriminant.

| Polynomial | Terms | Degree in `s` | Degree in second axis | Maximum integer coefficient bits |
|---|---:|---:|---:|---:|
| Original `p(s,t)` | 2,502 | 84 | 32 | 691 |
| Integer transformed `P*(s,u)` | 2,089 | 92 | 32 | 799 |
| Content-free `Q(s,u)` | 2,089 | 64 | 32 | 673 |

`Q` expression SHA is
`b2c53e1b5f63a6cc3e0ce3eef6fdc28ab1bd204ac33b25889250442646b29320`.
Its derived capture explicitly denotes a proposed discriminant of `Q`;
it is not a completed original operation record.

## Bounded implementation and validation

The research-only implementation is
[`examples/profile_affine_discriminant.rs`](../examples/profile_affine_discriminant.rs).
It uses Symbolica polynomial arithmetic and the existing bounded sparse
integer parser and coefficient-content routine. The original capture,
completed constant-shift report/image, and all seven shift-source files
are hash checked. The supervisor checks all ten input-file hashes again
after the worker exits.

Before products, the helper bounds the active map, degrees, coefficient
heights, dense support, single-product pairs, and cumulative pairs. The
original envelope is at most 5,000 terms, parameter degree 84, second-axis
degree 32 and 1,024 coefficient bits. Forward support is bounded by 3,861
terms and parameter degree 116; the independent inverse can reach degree
148. Intermediate limits are 6,000 terms, parameter degree 160, 4,096 bits,
2 million pairs per product and 20 million pairs overall. The actual
forward/inverse calculation consumed 116,198 product pairs. Native GCD
transients remain subject to the separate 60-second, 4-GiB worker bound.

An initial run performed the algebra but failed while hashing its
2,152,257-byte output with the reused 2-MiB input reader. It is retained as
**failed**, in `artifacts/research/a377-affine-discriminant-01/`. The fixed
helper retains the 2-MiB input cap and uses a separately bounded 16-MiB
streaming output hash. Fresh output paths and exit-zero, untruncated,
hash-consistent worker completion are mandatory. A leftover candidate file
alone never counts as success.

Twelve focused tests pass: four new tests and eight included ancestor-gate
tests. They check the full signed discriminant identity against independent
small native resultants, nonconstant content, loss of degree at `s=0`,
static input declines, and the separate output-hash bound. The degree-loss
test explicitly distinguishes specializing the generic discriminant from
recomputing the discriminant after lowering the polynomial degree.
Independent theorem/source review found no blocker.

The final build used CPUs14–15, two Cargo jobs, a 4-GiB virtual-memory cap,
and separate 300-second test/build deadlines:

```sh
source .symgcad-env
ulimit -v 4194304
nix develop -c taskset -c 14,15 timeout 300 cargo test -j2 --example profile_affine_discriminant
nix develop -c taskset -c 14,15 timeout 300 cargo build -j2 --example profile_affine_discriminant
```

The frozen helper's SHA is
`5f640b433260a72802846c20244efa58eb694452488bddd4290ff194bfd92166`;
source SHA is
`7107ac44825dd1f48ffca286a9305f701ed2e17988253b9bb153913b2e3c0975`.
Build logs and the complete source/library manifest are in
`artifacts/research/affine-discriminant-build-02/`; manifest SHA is
`e78501e0511dc47e6e9e6a32fbb8215c7f9ef8392d9f68b70cae3296cceea8fa`.

## Completed gate and static-layout evidence

The successful fresh run used CPU12, one worker, 4 GiB and 60 seconds.
It completed in **0.466899553 seconds**, with **18,436 KiB** peak RSS.
Forward/inverse replay took 0.196346948 seconds and signed content extraction
0.034666446 seconds. These are research-helper timings, not discriminant
or solver timings.

The directory `artifacts/research/a377-affine-discriminant-02/` contains:

| Artifact | SHA-256 |
|---|---|
| `report.json` — bounded process and unchanged-input audit | `e604a9302d8f50130a804fddb07d90e4c63ae7d9886e9f05a95d7468761ffca5` |
| `report.candidate.json` — signed polynomials and exact replay | `c50b64773cd91b60659a01924d4b00e949a872e12897dbf505d7f92b36f756af` |
| `report.Q.capture.json` — explicitly derived `Q` operation input | `72e639e34898f47bfa7d083a406822dc0cbe1642ae02e789acee7be27cc531ac` |
| `Q-layout.json` | `bb2a9570fc42b0aab4837bad5a3f34d95dcde1c3e1e8eb02515e24c8e72cf094` |
| `original-layout.json` | `b583beab7b90d34b282fce7308f180d4cc51059e2d5586f8778780f999a3d8b9` |

Both static inspections used the same frozen
`profile_kronecker_resultant` executable, SHA
`44428e764331e44d295eb4b8dae17201d39fe7b1a8573a240a499b1fea95ec09`,
with `--inspect-only --max-encoded-bits 96000000`. Each completed in about
0.084 seconds and under 10 MiB peak RSS. Neither allocated an encoded
resultant or called a native discriminant.

| Input | Output degree bound | Digit bits | Digits | Encoded bit bound | Within 96M cap |
|---|---:|---:|---:|---:|---|
| Original | 4,772 | 44,035 | 4,773 | 210,179,055 | no |
| `Q` | 4,032 | 42,665 | 4,033 | 172,067,945 | no |

The encoding bound falls by approximately 18.13%. This is a modest static
reduction, not measured algebra acceleration. A later native or encoded
`Disc(Q)` experiment needs its own bounded authorization, complete signed
reconstruction, and independent validation. The present work neither
promotes a production cap nor relies on the separately computed original
wide-segmentation candidate.
That original candidate was subsequently independently verified by the
[complete signed grid](A377_D32_GRID_VERIFICATION.md); the affine gate
remains an independent polynomial identity, with no transformed D result.

Independent evidence review rehashed all source, binary, candidate, and
capture bindings and checked the signed monomial-content product
`d*Q=P*` coefficient by coefficient. It confirmed both completed layout
processes and both cap declines. The audit is
`artifacts/research/a377-affine-discriminant-02/independent-audit.json`, SHA
`21b354949abac8d0b10f9e8e2c5114470f1b044f9f93c34eaa066d6a83a3a6dc`.
This supplements the source review and exact Symbolica forward/inverse
replay; it does not certify a discriminant that has not been computed.
