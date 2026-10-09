# A381 root-union profile after the heap change

Immutable release `7629a91`, SHA-256
`f3479621ada2731221775dea3c04f5552fb0c683af06a67262ce160872132805`,
still spends substantial time separating the first-axis root union for fixed
A381. A fresh 60-second run reached no lifting stacks or accepted cells and
retained 12,054 candidate roots. It used one CPU, an 8 GiB address-space cap,
the exact order/anchor from
`artifacts/hard-campaigns/a381-global-root-queue-debug-01`, and the same shared
projection checkpoint directory. All six projection checkpoints hit. This is
a warm-projection diagnostic, not an end-to-end cold benchmark.

A ten-second 199 Hz profile captured 1,983 samples with 16 KiB user stacks.
The inclusive attribution is:

| Frame | Inclusive samples |
|---|---:|
| `roots::same_root` | 57.07% |
| univariate polynomial `gcd` | 56.82% |
| `roots::refine_once` | 29.24% |
| univariate polynomial `evaluate` | 29.19% |
| `roots::sign_at` | 14.08% |

These percentages overlap through call ancestry and must not be added.
Rational interval comparison is below the report's 1% threshold. Allocation
and integer arithmetic dominate self samples: GMP `hgcd2` 13.20%, `malloc`
10.59%, `div2` 8.65%, and `realloc` 6.01%. The worker used 59.38 CPU seconds
and 64,108 KiB peak RSS before its external 60-second timeout.

The first sample was attached before observing a progress marker. A second
fresh run therefore gated recording on `phase=separate_roots`; its retained
start/end snapshots show 49.744–59.751 seconds, level zero, and 12,054 roots
throughout. It captured 1,918 samples and ended at the worker deadline. The
second recording used 4 KiB user stacks, which truncate higher callers; its
inclusive `same_root` percentage is consequently an underestimate and is not
used above. Its flat polynomial-GCD and integer-arithmetic costs corroborate
the first recording. Both sampled endpoint fibers have integer coefficients.

The measurements motivate two independent exact optimizations:

- A bounded number of sign bisections before the first shared-root GCD can
  separate overlapping *intervals* that contain different roots. The bound is
  necessary because truly shared roots cannot be separated this way.
- A bounded per-fiber cache of **coprime polynomial-pair results** can avoid
  repeating the same GCD after heap reinsertion. A nonconstant GCD does not
  mean that two particular roots coincide; exact root counting in the interval
  intersection must remain. Cache identities must be internal input identities
  or validated unique IDs, because the public root-union API accepts external
  polynomial IDs without requiring uniqueness.
- Integer homogeneous Horner evaluation can avoid rational normalization at
  every multiply/add when evaluating integer-coefficient fibers at rational
  bisection points. A positive denominator preserves the sign after clearing
  its degree-th power; other coefficient types need the existing exact path.

The captured degree-24 fiber contains 25 integer coefficients and 6,828 total
coefficient bits. It is available as
`artifacts/profile/a381-root-union-7629/degree24-fiber.json`, SHA-256
`a75a19940cf6a7c7e009cc4de9402a761eb8373fb666d8581df6645d39b35929`.

| Artifact | SHA-256 |
|---|---|
| `artifacts/profile/a381-root-union-7629/pre-lifting-perf.data` | `c47bad60106cef84bb396c1ac1e376181332794e7da31f850fc129a871b31a58` |
| `artifacts/profile/a381-root-union-7629/children-noinline-report.txt` | `4ae316520c192b6fd506482bc0cb6f69f50f361706d954e9df325054b22bdf2f` |
| `artifacts/profile/a381-root-union-7629-02/perf.data` | `beee5eaa8861607c47a61b79c9d4603489a67bc9f99acd7898da88850bcc706e` |
| `artifacts/profile/a381-root-union-7629-02/profile-start.json` | `65b433b815c20e51f006a8b5a94036d6cac3711f2410f7aaf54c302497161a05` |
| `artifacts/profile/a381-root-union-7629-02/profile-end.json` | `6baa0a8fd591e719137d464a499006a148ab28068d11d83ff152412bc02a6cce` |

Each directory retains its worker input, input/binary hashes, external deadline
script, progress, resource log and timeout status. For fast reproducible
attribution, disable costly inline-frame expansion in perf:

```sh
nix develop -c perf report --stdio --children --call-graph none --no-inline \
  --max-stack 24 --sort symbol --percent-limit 1 \
  -i artifacts/profile/a381-root-union-7629/pre-lifting-perf.data
```

These profiles establish a bottleneck, not a completed A381 decomposition or
an optimization speedup. Any later change needs a fresh, separately identified
measurement and independently checked complete outputs.
