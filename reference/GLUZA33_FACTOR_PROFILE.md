# Gluza33: factorization profile and leading-coefficient preflight

**Follow-up:** the [isolated Wang-preflight experiment](GLUZA33_WANG_PREFLIGHT.md)
now reconstructs this source in 1.039 seconds. The measurements below describe
the unmodified-backend diagnosis that motivated that optional patch; the original
Gluza33 decomposition remains unresolved.

The remaining 2,410-term factor call spends most observed execution in
bivariate reconstruction and Hensel lifting. Its leading coefficient factors
cheaply, and one bounded specialization passes the necessary Wang image gates.
Neither result solves the source factorization or the original CAD problem.
All measurements below are diagnostics outside benchmark timings; no source
factor was inserted into a cache and no dependency code was changed.

The source is the final factor capture from the 600-second original-positive
Gluza33 continuation described in [the affine-budget note](GLUZA33_AFFINE_BUDGET.md).
It has full variable map `[x2,x1,x7,x4,x6,x3,x5]` and degrees
`[20,18,8,0,0,0,0]`. The backend is Symbolica
`75f8350094b90254ee71dc2a391fde0d14b0204a`, without optional root-isolation
patches. The capture SHA-256 is
`aa44f4fb3da0145236f42254989933015ad6d6eda25cada5b49e65d83f60be43`.

## Execution profile

One fresh debug worker used CPU 8, a 4-GiB address-space limit and a 60-second
deadline. It timed out after **60.0334 s**, with **14,468 KiB** peak RSS.
`perf record -e cpu-clock:u -F 99 --call-graph dwarf,16384` collected 5,400
samples, zero lost; 5,338 belong to the worker and 62 to its supervisor.

| Function present in sampled stack | Samples | Fraction of all samples |
|---|---:|---:|
| `bivariate_factor_reconstruct` | 2,380 | 44.07% |
| `hensel_lift_with_strategy` | 2,332 | 43.19% |
| `find_sample` | 1,788 | 33.11% |
| `bivariate_hensel_lift_bernardin` | 1,742 | 32.26% |
| `lift_diophantine_univariate` | 613 | 11.35% |

The first two functions cover 87.26% of samples together, with no observed
intersection. Other rows overlap. A directly observed chain is
`find_sample → bivariate_factor_reconstruct → bivariate_hensel_lift_bernardin
→ lift_diophantine_univariate`, followed by dense integer/composite-modulus
arithmetic. Bivariate reconstruction appears from about one second until the
deadline; the initial univariate scout is observed only during the first second.

The 16-KiB stack snapshots frequently truncate outer frames. These counts are
lower bounds for inclusive stack presence, not exact phase timings. They do
not count retries or identify each recursive call's polynomial. Only 64 samples
retain a profiler entry frame. The first inline-source report was stopped
during expensive `addr2line` processing; `inclusive.txt` is incomplete. The
authoritative `--no-inline` reports and exported stacks reuse the same recording.

## Exact leading-coefficient result

The new research helper [profile_wang_leading.rs](../examples/profile_wang_leading.rs)
primitiveizes the rational source exactly, factors only a bounded leading
coefficient, and replays its signed product with full maps and multiplicities.
Its supervisor enforces one worker, 4 GiB and at most 60 seconds. Source
admission is at most 8,192 terms, three active variables, degree 32 and 8,192-bit
coefficients; the leading coefficient is capped at 128 terms and two active
variables. Reconstruction also has explicit factor-count and work bounds.
Twelve helper/parser tests pass, including map, content, multiplicity and
unacceptable-specialization cases.

At axis `x2`, source rational content is 1. The leading coefficient has 25
terms, degrees `(11,2)` in `(x1,x7)`, and 43-bit coefficients. Its factorization
and exact signed product replay take **0.005646 s**; the complete supervised
preflight takes **0.043731 s / 7,912 KiB**:

```text
LC_x2 = 6250*x1^2*(69+50*x1)*(6+5*x1)^2*(7+5*x1)^3*Q
Q = -49*x7^2 -98*x1*x7 -70*x1*x7^2 -42*x1^2
    -70*x1^2*x7 -25*x1^2*x7^2 +5*x1^3
```

Eight of the nine fixed menu entries fail the pinned backend's necessary Wang
image tests. At **`(x1,x7)=(11,13)`**, the five nonconstant factor-image
magnitudes are `[11,619,61,62,772187]`: all exceed one and are pairwise coprime;
the primitive-variable image 11 is also coprime to leading content 6250.
This certifies **only these necessary image gates**. No bivariate image,
leading-factor assignment, or subsequent Hensel lift was certified by this
probe. In particular, scalar units, all multiplicities and the signed leading
product must still be reconstructed after assignment and scaling.

## Independent cached-divisor control

The bounded cache search read 16,533 files / 128,044,690 bytes and tested the
shortest 4,096 candidate expressions. Three exact univariate filters rejected
499; exact full-polynomial division rejected the remaining 3,597. No divisor
was accepted, and the residual remains the original 2,410-term polynomial.
The inventory was **truncated to 4,096 proposals**; this does not exclude a
divisor elsewhere in the cache. Cached strings were proposals only, with exact
division and signed reconstruction required for acceptance.

This run completed in **29.854535 s** helper time / **29.86 s** wall time,
with **15,400 KiB** peak RSS, on CPU 13 under 60 seconds / 4 GiB. It provides a
measured negative for this bounded reuse strategy.

The independently measured general affine/shear policy also declines this
operand and reaches native fallback, which times out at **30.033260 s /
13,608 KiB**. Stage snapshots under
`artifacts/research/shear-factor-policy-01/profiler/gluza33/` bind that result;
it provides no reason to repeat those coordinate trials.

## Smallest justified follow-up

Pinned `factor.rs::find_sample` (lines 11200–11520) normally accumulates three
suitable bivariate images. Its existing Wang first-image shortcut admits only
a single-monomial, nonconstant leading coefficient. The 25-term coefficient
therefore misses that shortcut. Moreover, merely widening the term gate would
not suffice: content 6250 causes the current prime sampler to skip 2 and 5,
then rotate only `(3,7)`. Either value for `x1` gives shared image primes.

A separately replayable, opt-in backend experiment should first add narrow
sample/rejection counters, bounded small-leading-coefficient admission and a
bounded generic sequence of prime tuples. Before any costly image factorization,
apply the existing image-coprimality gates. Keep every subsequent degree,
squarefreeness, primitive-content and leading-assignment check unchanged;
accept only when all factor multiplicities are assigned and the exact signed
leading product reconstructs. On any failure or exhausted candidate schedule,
retain the existing fallback. The successful point above must not be hardcoded.

The next 60-second matched probe should determine whether a valid first image
actually reaches accepted assignment and lifting, and whether it avoids repeated
bivariate work. The current profile does not establish the exact rejection
cause or justify relaxing any proof requirement. No such backend experiment
has been run in this checkpoint.

## Reproduction and artifact hashes

Ignored artifacts are under `artifacts/research/gluza33-factor-profile-01/`:
`record.sh`, `perf.data`, `stack-summary.json`, `FINDINGS.md`, the frozen helper,
`leading/probe/{worker,process}.json`, build log and `SHA256SUMS`.
The cache control is `artifacts/research/cached-divisors-01/v2/`.
Replay the leading probe from the repository root after loading `.symgcad-env`:

```sh
artifacts/research/gluza33-factor-profile-01/leading/bin/profile_wang_leading \
  artifacts/research/gluza33-factor-profile-01/source-capture.json \
  --capture-sha256 aa44f4fb3da0145236f42254989933015ad6d6eda25cada5b49e65d83f60be43 \
  --axis 0 --output /tmp/gluza-leading-fresh --seconds 60 --cpu 8
```

SHA-256 bindings:

| Artifact | SHA-256 |
|---|---|
| Native profile executable | `d482dbaefcec1dcbc2d69df3fd7a232a14e61d55c612b9a33dca8b6b70367061` |
| Native profile source | `eee465d57baff7256be44ad9dc9caf55d6eae7d528b5b89e0dbcdcd3b27d99a5` |
| Pinned Symbolica `factor.rs` | `d5eaea5bc76e04aa29a0f481f94ba43580230d314fb0b182faebd9fe469e3fa1` |
| Leading probe executable | `14b02b63b8c230868661edccde8a9bdd71dc35d1d5baed8a919da1387fc0945b` |
| Leading probe source | `960a7e27dca627f20d616484b3a5ef62a0d82ab8a6297d041f310cce6cff0186` |
| Leading result | `5ef085122ce6796daf305a92e5a5943218cf31bba89a1215af19ac401f98971a` |
| Cache-control executable | `5dc0a16ffd61459dc33d5d08a5d143277a18b6d2f3b520ea7de6dc06770481c9` |
| Cache-control source | `9c04bdaa8990cf46f426f75e8ba280d772b81e7b668d10a467f664e4787cd195` |
| Cache-control result | `c51a5e7e140825e6ec5e449d73a2315194ba889cb6bc010014531751fdcfc2c7` |
