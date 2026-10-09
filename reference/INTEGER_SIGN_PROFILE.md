# Exact integer sign evaluation

The release microprofile on the captured A381 degree-24 fiber finds lower
evaluation time after clearing the evaluation denominator with integer Horner
arithmetic. This compares two exact sign evaluators in one executable; it is
not a whole-root-isolation or GCAD speedup measurement.

For an integer polynomial `p(x) = sum(a_i*x^i)` of degree `m` and a normalized
rational point `x=n/d`, `d>0`, the evaluator starts with `H=a_m` and descends
through `H <- n*H + a_i*d^(m-i)`. The final integer is `d^m*p(n/d)`, which has
the same sign as `p(n/d)`. Symbolica supplies all integer arithmetic, including
the fused multiply-add. Only the current accumulator and denominator power
are retained. Noninteger coefficients use Symbolica's ordinary rational
evaluation. Zero, constant, zero-point, and integer-point cases have exact
shortcuts.

Production source: [integer_sign.rs](../src/roots/integer_sign.rs), integrated
in root refinement, endpoint checks, collision checks, and Sturm sign
variation by root-kernel checkpoint `e43a2c4`. The coordinated
library/integration suite passed 287 tests, including five evaluator tests:
deterministic integer polynomials and rational points, general rational
coefficients, zero and constant cases, large coefficients and points near
exact roots, and the full captured degree-24 fiber. The separately compiled
profile helper also passed these five tests through its shared-source module.

## Release measurement

The Rust helper includes the production evaluator by source path. It first
compares signs with `UnivariatePolynomial<Q>::evaluate` at every sample, then
uses opaque inputs and result checksums inside each timed loop. It alternates
the two evaluator orders across five repeats. Each repeat contains 18,000
calls at positive and negative points close to integers, with denominator
`2^k`; `k=0` gives integer controls. Input preparation, validation, and report
serialization are outside the timed loops.

| `k` | Rational median, ms | Integer median, ms | Rational / integer |
| ---: | ---: | ---: | ---: |
| 0 | 46.680 | 8.525 | 5.48 |
| 8 | 229.122 | 20.803 | 11.01 |
| 32 | 315.835 | 34.017 | 9.28 |
| 128 | 627.437 | 112.960 | 5.55 |
| 512 | 1863.116 | 704.813 | 2.64 |

All signs and timed checksums agreed. The complete bounded worker took
20.579 seconds wall time, 20.438 seconds user CPU, 0.021 seconds system CPU,
and 5,112 KiB peak RSS. It ran on CPU 7 with a 60-second deadline and 4 GiB
memory limit. All five raw times per evaluator/group are retained; the two
smallest-denominator rational groups each contain one slower outlier, so the
table uses medians. These controlled points do not claim to reproduce the
frequency of precisions encountered in a whole solve. General rational
coefficient fallback performance is not measured here.

## Reproduction and provenance

```sh
nix develop --command artifacts/bin/profile-integer-sign-e43a2c4 \
  artifacts/profile/a381-root-union-7629/degree24-fiber.json \
  --output artifacts/profile/a381-integer-sign-01/release.json \
  --iterations 1000 --repeats 5 --seconds 60 --cpu 7
```

The helper's parent uses the Rust `benchmark::run_bounded` supervisor. No
Python orchestration is involved. Its source is
[profile_integer_sign.rs](../examples/profile_integer_sign.rs), committed as
`db3d44a`. Local ignored artifacts hold the full evidence:

- Input: `artifacts/profile/a381-root-union-7629/degree24-fiber.json`, SHA-256
  `a75a19940cf6a7c7e009cc4de9402a761eb8373fb666d8581df6645d39b35929`.
  It has 25 integer coefficients and 6,803 numerator bits in total; the
  original capture's 6,828 count also includes 25 unit-denominator bits.
- Release helper: `artifacts/bin/profile-integer-sign-e43a2c4`, SHA-256
  `d79bd0b71ae2f5202edb10d2f39bf98d3c4c8d8c51692a7ca5d1a337f246282c`.
- Included evaluator source SHA-256:
  `9d6cf3741f15b384382798f9b250b43dff250204d6f5f19c61332da97ff3490b`.
- Report: `artifacts/profile/a381-integer-sign-01/release.json`, SHA-256
  `33981a183506ee6efcbd1e95a63885bd42a51ee6bdf6a70a7b0c0c02e9f2fde4`.

An earlier debug smoke report is adjacent as `debug.json`. It establishes
that the supervised harness works but is not used for the release table.
