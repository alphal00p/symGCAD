# Exact resultant factorization research

The captured A446 resultant is now constructed by deterministic interpolation
in about 5.9 seconds, but its subsequent factorization takes about 21 seconds
in the debug research executable. Two captured A381 resultants construct in
about 2.2 seconds each, while factoring either degree-576 output exceeds a
25-second limit. These are isolated algebra measurements, not completions of
the original GCAD problems.

## Reproducible Rust helper

`examples/profile_verified_resultant.rs` owns the bounded subprocess through
`symgcad::benchmark::run_bounded`; its default worker memory limit is 4 GiB.
It supports exact numeric-image checkpoints, deterministic Newton
interpolation, independent candidate verification, and checked factorization.
Untrusted CRT/Brown/primitive candidates are accepted only after equality at
more distinct full-degree exact parameter points than the proved degree bound.
A partial computation never supplies a certificate.

The research-only `--compact-factor-map` option removes inactive coordinates
before factorization, restores every factor and multiplicity to the original
variable map, and reconstructs the entire original polynomial exactly. It
does not change the production factorization path. Four helper tests cover
degree drops, wrong candidates, exact rational scale, zero-image variable
maps, and compact-map restoration with scalar content and multiplicities.

## Captures and measurements

All runs below used CPU 7, one worker and a 4 GiB limit. The original-map
profile used frozen helper SHA256
`9e55193cdb0595692032c502ea3165ad62e905b1eeeda925eea46ac8f9687bae`;
the compact-map helper has SHA256
`78d22473b11f3cf8e61c13a17cf1c025f9921c64a55c61d0d48028dcb06100c8`.

| Input | Variable map | Algebra time | Outcome |
| --- | ---: | ---: | --- |
| A446, degree 461 / 318 terms | 8 axes, 1 active | 21.247 s | Six distinct factors, reconstruction checked |
| Same A446 expression | 1 active axis, restored to 8 | 17.521 s | Identical canonical factors, reconstruction checked |
| A381 x6 ordering, degree 576 / 577 terms | 6 axes, 1 active | 28 s limit | Timeout, no factorization result |
| Same A381 expression | 1 active axis, restoration required | 25 s limit | Timeout, no factorization result |

A prior unprofiled original-map A446 run took 20.940 seconds. The compact-map
trial suggests a modest debug improvement, but these are separate helper
builds without release repetitions. No production speedup is established.
Generic polynomial operations may be instantiated in the unoptimized research
crate even though dependency packages use optimization level 2.

The A446 canonical factor-vector hash agrees in all completed runs:
`13fbabc78d9e079dd03213db9992b5c2d554aeb2f95a3822d2347df638bb7fac`.
Its distinct factor degrees are 1, 4, 12, 61, 81 and 1. Exact integer Sturm
counts separately found six and five roots in `s > 4` for the degree-61 and
degree-81 factors, respectively; neither can be removed by nonvanishing
certification on the full requested domain.

## Sampled bottlenecks and primary source

Linux perf sampled at 99 Hz with 16 KiB DWARF stacks: 2,117 A446 samples and
2,672 A381 samples, with no lost samples. Inclusive percentages overlap and
incomplete unwinding prevents interpreting them as a disjoint phase split.

| Attributed function | A446 | A381 x6 |
| --- | ---: | ---: |
| Hensel lifting, inclusive | 55.12% | 44.07% |
| Finite-field monic remainder, inclusive | 16.83% | 60.92% |
| Finite-field monic remainder, self | 3.94% | 18.23% |
| Modular prime screening, inclusive | below displayed threshold | 35.10% |
| GMP basecase multiplication, self | 28.35% | 13.33% |

The primary source is the clean local Symbolica checkout at commit
`75f83500`, specifically:

- `src/poly/factor.rs:4958`: rational factorization already removes content
  and converts to integer coefficients.
- `src/poly/factor.rs:4641` and `:2995`: integer factorization already removes
  common monomials and performs square-free decomposition using Yun's method.
- `src/poly/factor.rs:9816`: square-free univariate factors are reconstructed
  from finite-field images and exact Hensel lifting/recombination.
- `src/poly/factor.rs:99`, `:293`, `:9294`: the dense modular distinct-degree
  workspace is limited to 256 coefficients; larger blocks use the generic
  modular polynomial path observed in the A381 profile.
- `src/poly/polynomial.rs:3297`: the existing public
  `rearrange_with_growth` operation supports dropping inactive axes and
  restoring the original map, rejecting the removal of an active variable.

Repeated primitive normalization or replacing square-free decomposition is
not supported by these measurements. A controlled release comparison of
compact maps is a small next experiment. A larger dense modular workspace or
different Hensel strategy would need separate backend measurements and tests;
no Symbolica checkout changes or production shortcuts were made here.

## Saved evidence

`artifacts/profile/resultant-factor-hotspots-01/` contains frozen helpers,
Rust-supervised process outcomes, exact factor outputs, perf data/reports and
the compact-map replay script. Its `summary.json` SHA256 is
`a94d79eb9984a6e40656883796e94a074a9c30c277c2d22300e9fca4ae90563a`.
The directory's `provenance.sha256` identifies both perf recordings and the
compact helper. Earlier interpolation, factor-domain and A381 replay evidence
is under `outer-t-interpolation-debug-02/`, `outer-t-factor-domain-01/`, and
`a381-discriminants-interpolation-01/` in the same artifact parent.
