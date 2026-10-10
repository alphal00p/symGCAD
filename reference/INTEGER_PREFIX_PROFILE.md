# Exact integer evaluation of rational prefixes

The warm A446 traversal profile attributes substantial cost to rational GCD,
cloning and multiplication. `examples/profile_integer_prefix.rs` tests a
different evaluation method from the unsuccessful cached `replace_except`
experiment: for integer coefficients, form a common positive denominator once
and normalize the resulting rational only at the end.

For an assigned coordinate `x_i=n_i/d_i`, with `d_i>0`, let `D_i` be its maximum
polynomial degree. The exact numerator is

```text
N = sum_e c_e * product_i n_i^e_i * d_i^(D_i-e_i)
D = product_i d_i^D_i
p(prefix) = N/D
```

Symbolica supplies all integer operations and final rational normalization.
Negative and zero numerators are allowed. The wrapper rejects any polynomial
with unassigned support before evaluation. Rational coefficients and bounded
resource exclusions use the original exact rational evaluator. No sign-only
approximation or floating point is used.

The research bounds are 4,096 terms, 16 active axes, degree 64 per axis,
16,384 estimated power bits and a 1-MiB estimated power-array allocation.
That last quantity is a scratch estimate, **not** a hard process-memory cap;
input/coefficient storage is not included. Both measured processes separately
had a 4-GiB limit and a 75-second external deadline. Powers are rebuilt on every
evaluation; the candidate does not receive a retained-prefix cache.

## Captured-workload measurement

The same executable compares both evaluators, checking equality of every full
rational result before timing. Both timed paths validate complete assigned
support. Five repeats alternate evaluator order, with 1,000 evaluations per
selected guard per repeat and opaque inputs/results. Parsing and serialization
are outside the timers. The helper alone was compiled with `-C opt-level=3`;
dependencies use the repository's development profile. These are isolated
kernel measurements, not release GCAD or corpus-completion claims.

| Captured batch | Guards / terms | Exact fallbacks | Rational median seconds | Integer median seconds | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| Symbolic A446 | 12 / 2,978 | 0 | 0.325044 | 0.085834 | 3.79 |
| Fixed A381 | 64 / 4,535 | 2 | 1.331088 | 0.249901 | 5.33 |

A446 ran on CPU 8 for 2.11 wall seconds, peak RSS 3,072 KiB; A381 ran on CPU 9
for 8.26 wall seconds, peak RSS 9,220 KiB. Three helper tests passed, covering
zero/negative rational points, cancellation, large integers, rational and degree
fallbacks, unassigned support rejection and 100 deterministic polynomial cases.
Independent source review found no arithmetic or timing blocker.

## Retained evidence

Evidence is under `artifacts/profile/integer-prefix-01/`:

| File | SHA256 |
| --- | --- |
| Frozen helper | `49d38344c5d497721b0fe3df519a9e65f0f6802fe15003b3512cb9b480ed0022` |
| Frozen helper source | `6a5a665fb3ab89454706545e2d4293067c5172d018b440379c7625735d88d8ff` |
| `a446.json` | `968b337a8ccc0d831c93a6ed20715eb15a8a30974aea5938ac8a6b2c5276cddc` |
| `a381.json` | `c2a4c4386370afde435eee3ce9db62d7082535985d495de233e98f96a2277ff2` |

Each report retains input/selection/full-value hashes, prefixes, all five raw
timings, fallback count and exact-equality confirmation. Source inputs are the
same `artifacts/profile/prefix-values-01/{a446,a381}.json` inventories used by
the earlier prefix-cache experiment; explicit supported-guard selection and
largest-term ordering are unchanged. Build/test logs are
`artifacts/integer-prefix-optimized-build.log` and
`artifacts/quadratic-prefix-focused-tests-02.log`.

```sh
taskset -c 8 artifacts/profile/integer-prefix-01/profile_integer_prefix \
  artifacts/profile/prefix-values-01/a446.json \
  --output artifacts/profile/integer-prefix-01/replay.json \
  --iterations 1000 --repeats 5
```

Production promotion requires the same exact support check, conservative
fallbacks, additional coefficient-height bounds and whole-solver regression
testing. These measurements alone add zero completed original problems.
