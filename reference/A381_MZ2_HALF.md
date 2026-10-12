# A381 at the colleague's fixed-kinematics point

The regression is `test_finiteA_7_381_10_0__4_0_1_1_1_1_1_0_1` with
`mt2=1`, **`mz2=1/2`**, `s=10`, `t=-3`, and all seven Feynman coordinates
strictly positive. The original F and constraints are preserved in
[`tests/fixtures/a381_mz2_half.toml`](../tests/fixtures/a381_mz2_half.toml).
The `_10_0__4_...` and `_11_0__5_...` imported A381 fixtures have identical
polynomial/domain data; their names differ. This point is distinct from the
historical `mz2=5/18` benchmark.

## Result and reproduction

All three measured runs completed with independently verified generic coverage:
**836 unmerged cells**, comprising **158 negative-F** and **678 positive-F**
cells. The lifting order, outermost to innermost, is
`[x8,x4,x6,x0,x2,x5,x3]`. The solver uses the homogeneous chart `x8=1`,
certified conditional quadratic reductions in `x3` then `x5`, and exact
reconstruction of the positive scale and root boundaries. The four-variable
base lifting order is `[x4,x6,x0,x2]`.

This is an explicit order arranged around the quadratic reductions, not an
MCTS-selected result. All measured search times are zero. See the
[conditional quadratic theorem](CONDITIONAL_QUADRATIC_PEEL.md) for the
certificate and discovery procedure; in the `x8=1` chart the multipliers are
`1+x0+x2` and `1+x4+x6`.

The focused current-source regression solves, round-trips the export, and
replays its exhaustive certificate. It passed in 0.43 seconds in the debug
test profile:

```sh
nix develop --command cargo test --locked --test a381_mz2_half
nix develop --command cargo run --release -- solve tests/fixtures/a381_mz2_half.toml --output artifacts/a381-mz2-half.json
nix develop --command cargo run --release -- verify artifacts/a381-mz2-half.json
```

## Timing evidence

Three fresh processes, cold algebra caches, one worker pinned to CPU6 on an
AMD EPYC 9754 host, 2 GiB limit, no merging. The machine was shared with other
workloads. The frozen release engine is `0a95bc2`, SHA-256
`0f41d37c9b22bbbf95fe5a0eca38b669dbc877006ccd769e4d2853266b327091`.
Solve times include startup, preprocessing and export; verification is separate.
Earlier order/certificate research is not included. Subsequent native upstream
controls at this point are recorded below; both timed out, so neither provides
a completed native solve time or a finite measured speedup ratio.

| Repetition | Public solve (s) | Verification (s) | Solve peak RSS (KiB) |
| --- | ---: | ---: | ---: |
| 1 | 0.250413 | 0.129748 | 17,804 |
| 2 | 0.142581 | 0.145076 | 19,080 |
| 3 | 0.148037 | 0.144417 | 19,084 |
| Median | **0.148037** | **0.144417** | — |

Maximum observed solve peak RSS was **18.64 MiB**. Full inputs, immutable
workers, proof hashes, verification results and process records are retained
locally under `artifacts/bench/a381-mz2-half-side-01/`; the three-run ledger is
`timings/ledger.json`. An earlier single completion run is kept separately in
`results/ledger.json` and is excluded from these medians.

The three-run ledger SHA-256 is
`4459d6841e3f17e1f8a17afa23aeb9903e70a294682b6f74cf5d7bc8b237287a`.
The original fixture SHA-256 is
`6479a7f510a55cf19b8a57c2125c479de4fb9d732d4d9db5b16a9dd8ce0da72f`.

## Supplemental randomized sanity check

Seed **20261010**, two fresh strictly interior rational points per exported
cell: **836/836 cells**, **1,672 original-F evaluations**, **11,704 original
constraint checks**, **zero failures and zero skipped cells or points**.
The checker recomputes algebraic boundaries at each new coordinate prefix,
checks strict membership with exact arithmetic, and evaluates the original
expressions independently of reconstruction. Points differ from the stored
CAD samples and from each other. This is supplemental evidence, not the
generic coverage proof, and is excluded from the solver timings above.

The unchanged `examples/sanity_sample.rs` was compiled with `rustc -C opt-level=2`
against the existing debug library/dependency artifacts, into this experiment's
own artifact directory. The successful check took 67.345 seconds. Its executable
SHA-256 is `4eb5294c67ad0993be14b588d2168cc15cec8be495f4c0bb4a08a9c9d2a3391c`;
source SHA-256 is
`e21c2c23b2aec2d6e261b788d80ff8b2fca48fdc00285537f9aea8d0dac58d0d`.
The full points and checks are in `sanity-optimized.json` under the experiment
directory, SHA-256
`548da980efe003293d9c37cc9e182b2234b4f1989f4d02de0c5517f453dda55e`.

An earlier unoptimized checker hit its 120-second limit; that separate partial
report is preserved as `sanity.json` and is not counted as a complete check.
The successful run restarted from the beginning with the same seed.

For a fresh build, run the same checker separately from benchmark timing:

```sh
nix develop --command cargo run --release --example sanity_sample -- \
  --original tests/fixtures/a381_mz2_half.toml \
  --result artifacts/a381-mz2-half.json \
  --output artifacts/a381-mz2-half-sanity.json \
  --seed 20261010 --points-per-cell 2 --seconds 300 --memory-mib 2048
```

## Native GCAD forced-order controls

The accelerated native upstream implementation at revision
`fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e` was copied into a temporary scratch
directory and called through its public
`GCAD` API. Both controls used the exact `mz2=1/2` point, an explicit order,
no merging, and a 300-second wall-clock limit. An independent SymPy check
reconstructed F from the original fixture's coefficient/exponent arrays and
confirmed exact equality with the new TOML input. The upstream GCAD module's
hash matched the original source; only the scratch driver added phase timing.

| Configuration | Outcome | Wall (s) | CPU (s) | Peak RSS (MiB) |
| --- | --- | ---: | ---: | ---: |
| Full `[x8,x4,x6,x0,x2,x5,x3]` | timeout | 300.036 | 146.668 | 144.1 |
| Unit-scale `[x4,x6,x0,x2,x5,x3]`, x8=1 | timeout | 300.067 | 146.664 | 401.3 |

Both stopped during projection of the first, positive-F branch. Neither
reached lifting or the negative-F branch. Thus the order alone, and the order
plus homogeneous scale fixing, did not suffice within this elapsed-time
budget. This does not establish failure for every ordering or preprocessing
choice. CPU time was about 147 seconds per process on the shared machine;
the limit was wall time, not 300 CPU-seconds. Runs used one pinned CPU each
(6 and 7), with a generous 64 GiB address-space cap, and ran concurrently.

Native extension SHA-256:
`63a22939953fca668719e7f52037fb960d5900693a656a352f258cbb1b56463b`.
Scratch driver SHA-256:
`81723e950ad76d7d8f487bf65fb2ed445f5e6e3810e08bc1fc77d6b87de7f034`.
Process records, input checks and log hashes are retained locally, outside the
public tree, in `reports/a381-mz2-half/measurements.json`, next to the letter
that reported these results.
