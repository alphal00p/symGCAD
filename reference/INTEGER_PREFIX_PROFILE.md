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

## Production and fixed-work control

Commit `3acd6aa` promotes the evaluator with the same exact support check and
fallbacks, a 4,096-bit coefficient-height cap, and six focused regressions.
All 354 library/integration tests and two campaign-helper tests pass. The
debug and release workers also independently replay the historical 5-cell
capability proof, 8-cell fixed A446 proof, and 12-cell massless proof.

A whole-solver control uses the original fixed A381 input, homogeneous anchor
`x5`, and order `[x5,x8,x4,x6,x3,x0,x2]`. Each process starts from an independent
copy of the same 5,223-file operation cache. The CPU is 8, memory limit 4 GiB,
worker deadline 120 seconds, and speculative-node cap 100,000. Operation
captures are disabled. Positivity, input positivity and operation pruning are
all explicitly disabled, so the intervening quadratic-domain certificate is
dormant. Other local-projection flags are identical. These are debug builds
and warm algebra-cache controls, not release acceptance measurements.

The execution sequence is old, new, new, old:

| Run | Solver seconds | Frontend wall seconds | User + system seconds | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Old 1 | 43.654 | 43.74 | 42.95 + 0.35 | 43,124 |
| New 1 | 43.620 | 43.66 | 42.91 + 0.37 | 43,040 |
| New 2 | 42.978 | 43.03 | 42.37 + 0.25 | 46,164 |
| Old 2 | 43.699 | 43.81 | 43.07 + 0.28 | 43,140 |

The median solver times are 43.677 and 43.299 seconds, a difference below 1%.
This control does **not** demonstrate a meaningful whole-solver speedup.
Every run stops incomplete at exactly 100,001 nodes, 4,881 refinements and
10,462 cuts, with prefix `[1,16/29,1]`. All structural report fields agree
after excluding elapsed time, phase timings and cache statistics. All four
inventories of 4,770 distinct factor-input/output/multiplicity records agree,
with SHA256
`5b9cd1b20c45d8fd914f5881314550fa0b60ba78c9bbb7976b0fe3ef13e24d4f`.
This is endpoint and algebra-output agreement; incomplete runs do not retain
a complete traversal proof. No original problem gains completion credit.

Evidence is under `artifacts/profile/integer-prefix-fixed-work-01/`, including
the identical-except-checkpoint-path inputs, independent caches, resource logs,
results, structural reports, factor inventories and `comparison.json`. The
seed manifest SHA256 is
`4ca65f2a1d7b79825404f41a795bf03b88fd2fcd3d6b6f82fd9008b6c73eefd8`.
The old debug worker (`ab8b9ba`) SHA256 is
`68a9cdc90842868ec9d7c08e2b9ba8f2b6265acbf315437bfae4ec647ca58157`;
the new debug worker (`3acd6aa`) SHA256 is
`463806e5636de61a66ef203d8e67bb6767a0192b56d47b8a92f6ef4f8fb9bed9`.
The new release worker, built in an isolated checkout and retained at
`artifacts/bin/symgcad-quadratic-integer-3acd6aa`, has SHA256
`c22220744258a28e487543d9796e5e113d621f6f05cc32c4f68a7cbe846b60b8`.
Build metadata and historical replay reports are retained beside the frozen
workers. Symbolica source is unchanged.
