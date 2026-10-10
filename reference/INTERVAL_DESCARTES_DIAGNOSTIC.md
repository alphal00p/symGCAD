# Rational-interval Descartes diagnostic

The opt-in global Descartes witness cannot certify a fiber when its positive
roots do not saturate the global sign-variation bound. Two later discriminant
captures exhibit this obstruction: symbolic A446 has two positive roots and
four variations; fixed A381 has three positive roots and five variations.

The research-only `examples/profile_interval_descartes.rs` splits the positive
ray into bounded rational intervals and a final tail. For a degree-d polynomial
`p`, it constructs exactly

* `(1+u)^d p((a+b*u)/(1+u))` for the interval `(a,b)`;
* `p(a+u)` for the tail `(a,+infinity)`.

Both maps are increasing bijections from `u>0` onto their original intervals,
and the denominator-clearing factor is strictly positive. The helper accepts
an interval only when its transformed coefficient list has zero or one sign
variation. Descartes' rule and its parity condition then prove exactly zero
or one positive root, counting multiplicity. A one-root interval therefore
contains a simple root. Nonzero rational endpoints prevent a root escaping
through the fixed interval boundaries.

If every nonzero symbolic transformed coefficient has its recorded strict
sign on a connected parent cell, these statements hold throughout that cell.
Together with a complete adjacent interval cover of the positive ray, they
prove its entire simple positive-root inventory and delineability, even when
negative roots collide. A sample-zero coefficient that is not identically
zero is an obstruction, not an omitted guard. Structural zero coefficients
are permitted. Exact sample isolation is used for discovery and a consistency
check; the zero/one interval theorem supplies the lower bounds independently.

This rule is now implemented in production under the opt-in
`solver.local_descartes_intervals` option. The producer and independent checker
carry complete interval/sign evidence, reconstruct all coefficient guards, and
export positive ordinals for original polynomial boundaries. The separate
interval-template stock admits at most two covers per canonical polynomial by
default. `solver.local_descartes_interval_template_limit` can opt into a fixed
limit from two through eight; the chosen limit is recorded in local evidence
and progress reports. Missing historical options retain two. Admissions are
permanent even when a speculative stack later fails;
cache eviction never replenishes them. The ordinary discriminant remains the
fallback when no stored template works and the stock is exhausted. The checker
validates supplied templates directly instead of rediscovering them at a new
sample. More ancestor guards may increase cell counts and refinement work even
when their algebra is cheap. The measurements below describe the original
research diagnostic, not a whole production solve.

The larger fixed limit preserves the same finiteness argument. At the highest
axis, the input inventory is finite and each polynomial can introduce only a
fixed finite number of coefficient/evaluation guard families. Ordinary
fallback contributes finitely many discriminants and pair resultants. Repeat
this argument down the variable order. No eviction, failed stack, cache hit,
or ancestor restart resets the stock. The checker needs no admission history:
it reconstructs the supplied cover and guards, and permits the stronger
ordinary-discriminant proof. Durable learned-cut archives remain disabled
when interval templates are enabled because those archives do not persist
the lifetime admission ledger. A larger stock can increase ancestor cuts;
it is an explicit performance experiment, not a claim of faster completion.

## Measured captures

Both bounded one-process diagnostics completed, including exact checked
factorizations of all unique guards. These are individual debug-helper runs,
not whole-solver timings or GCAD completions.

| Capture | A446 | A381 |
| --- | ---: | ---: |
| Polynomial degree / terms | 8 / 1500 | 24 / 568 |
| Global variations / distinct positive roots | 4 / 2 | 5 / 3 |
| Accepted intervals | 3 | 6 |
| Unique coefficient guards | 25 | 145 |
| Largest guard terms / coefficient bits | 454 / 63 | 31 / 271 |
| Exact discovery, seconds | 0.00146 | 0.01169 |
| Symbolic guard construction, seconds | 0.03086 | 0.11746 |
| All checked guard factorizations, seconds | 0.15392 | 1.28346 |

For A446 at `[s,t,x1,x2]=[17/4,-22,1,1/2]`, projecting `x3`, the intervals are
`(0,8)`, `(8,36)`, `(36,+infinity)`, with variations `[1,1,0]`.

For A381 at `x8=1`, projecting `x4`, the intervals are
`(0,1)`, `(1,5/2)`, `(5/2,23/8)`, `(23/8,13/4)`, `(13/4,4)`,
`(4,+infinity)`, with variations `[1,1,1,0,0,0]`.

The helper bounds degree at 32, input terms at 4096, the interval cover at 64,
subdivision depth at 32, and rational separator numerator-plus-denominator
height at 256 bits. It uses exact Symbolica rational/polynomial arithmetic,
deduplicates canonical guard polynomials, verifies symbolic specialization
against every numeric transformed coefficient, and persists progress before
each checked factorization. Its three tests cover exact cleared Möbius/tail
identities, globally unsaturated examples, and rejection of repeated positive
roots as simple-root certificates.

## Reproduction and hashes

```sh
nix develop -c cargo test --example profile_interval_descartes
nix develop -c cargo run --example profile_interval_descartes -- \
  artifacts/profile/interval-descartes-01/a446-capture.json \
  --prefix 17/4,-22,1,1/2 --cpu 0 --seconds 60 --factor-guards \
  --output artifacts/profile/interval-descartes-replay/a446-report.json
nix develop -c cargo run --example profile_interval_descartes -- \
  artifacts/profile/interval-descartes-01/a381-capture.json \
  --prefix 1 --cpu 5 --seconds 60 --factor-guards \
  --output artifacts/profile/interval-descartes-replay/a381-report.json
```

Each supervisor used one CPU and a 4 GiB address-space limit. The immutable
captures and frozen helper/source are retained in
`artifacts/profile/interval-descartes-01/`.

| Artifact | SHA-256 |
| --- | --- |
| `a446-capture.json` | `8c14d3344554c5ed91ac572395d3c1a1768d7e9626429193885892dfd585f861` |
| `a381-capture.json` | `bbd0b682866e56a95590398109bf15767e8358b6626494a0fd0deccffa8d3975` |
| `profile_interval_descartes` | `1894661ac96af8e096a1d53261a9278a8d75adf59487866e9fb0603743928fc7` |
| `profile_interval_descartes.rs` | `39d55aa2c70463430597ae86ba3854657ecc29de5200b111a1721a1759157034` |
| `a446-report.json` | `9ee69d5f228dbcdfcd28c30f6de0ba8b015b94cc96aa2f138b2e2780f24b0a74` |
| `a381-report.json` | `8e6f52c9bfb478b8df42f03aa0f868dea017a5c8c62581bc7ea4d0ddcf22165d` |

No complete original decomposition or corpus completion is claimed here.

## Reversed-parameter A446 follow-up

The later capture `f09d4745806e20ec3490808ee1ebbc84cb4c86e34a41f8c82fa8af5a5aa820ce`
has degree 8 and 1,127 terms. At `[t,s,x1,x2]=[-9,5,1/2,1/16]`, it has
two distinct positive roots and four positive coefficient variations. The
research adapter `examples/profile_interval_stock.rs` compiles the unmodified
production interval producer and checker into a standalone bounded worker.
With a fresh stock it accepted this exact polynomial, and independent replay
returned the identical 41 guards (17,159 terms) in 0.1562 seconds. Its five
intervals have variations `[0,0,0,1,1]`. Separately, checked factorization of all
guards took 0.3116 seconds in a single diagnostic run.

This establishes current applicability, not the interrupted worker's template
history. That history was not exported. Its zero recorded interval-budget
fallback count came from an earlier progress snapshot; it cannot establish
that the final attempt still had a free slot. Exhaustion of the two-template
stock is consistent with these measurements, but is not directly observed.

The same adapter screened `(1+c*x)^k p(x)` with exact Symbolica arithmetic for
all 13 dyadic values `c=2^j`, `-6<=j<=6`. None saturated the positive-root count
through `k=24` (the degree-32 bound); a separate frozen diagnostic extended the
same menu to `k=32` with no success. For `c=1`, the first numeric saturation
was `k=52`. No degree-60 symbolic certificate was expanded or integrated.
Positive multipliers preserve all positive roots and their ordinals, but
their coefficient guards would still have to hold on the whole ancestor
cell. These screens therefore provide no original decomposition.

The exact report is retained as
`artifacts/profile/a446-reversed-certificates-01/fresh-stock.json`
(SHA-256 `4ff38130c81855277fada8ee57cc5a336a0e0d49de2473b884a9dd4fc24efc44`).
The frozen adapter SHA-256 is
`6aba79072270b88e529473cf7766fba507b2d893d6b53493c681297981e0b3a5`.
Whole supervised diagnostic wall time was 0.2242 seconds, with 14,296 KiB
peak RSS; these are single debug-helper measurements.
