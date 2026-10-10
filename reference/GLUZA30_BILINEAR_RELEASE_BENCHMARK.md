# Positive Gluza30 release controls

The full original strict positive Gluza30 problem completed and passed independent verification in three fresh release processes, with **80,980 cells in every trial**. The original native GCAD implementation timed out in all three corresponding 60-second controls. These are censored measurements; no finite speedup ratio is reported, and the frozen baseline parity denominator is unchanged.

| Measurement | Result |
| --- | ---: |
| Rust public solve median | 22.567567 s |
| Independent verification median | 16.854419 s |
| Maximum solve RSS | 483,428 KiB |
| Maximum verification RSS | 762,300 KiB |
| Native outcomes | 3 timeouts at 60 s |
| Maximum native RSS | 825,808 KiB |

Both engines used the original positive target, all seven positive coordinates, unchanged variable groups, no specialization or extra domain restriction, no merging, and the same explicit legal order `[x1,x7,x2,x6,x5,x4,x0]`. The mathematical TOML prefix and explicit order were checked byte-for-byte between their prepared inputs. Each process was pinned to CPU4, with one worker and an 8 GiB limit. Solve and verification each had a 60-second deadline. Three fresh native trials ran first, followed by three fresh Rust solve/verification trials with cold algebra caches. All unsuccessful and successful results remain in the report.

The opt-in `bilinear_positive` wrapper checks the exact guide premises, solves a five-variable coefficient-sign arrangement, and restores the two final positive fibers through its exhaustive sign table. It does not identify the positive set with a complement of the negative cells or remove the leading-coefficient sign partition. The original-coordinate proof retains the base proof and every restoration obligation; the independent verifier checks both.

Recorded median internal times are 0.000811 s for ordering/guide planning, 4.091663 s for projection, 4.057572 s for lifting/restoration, and 21.642066 s total. The individual phase counters do not exhaust all local traversal and proof-construction costs; the total and public wall measurements include them. Public times also include startup and export. Exact guide checks are included in every solve.

## Discovery cost and retained earlier attempts

The fixed winning order was found by a separate bounded MCTS search on the exact five-variable coefficient-sign base. That search took **300.025654 seconds** and recorded 104 trials. Its winning order was `[x1,x7,x2,x6,x5]`; the wrapper appends `[x4,x0]`. Earlier algebraic guide discovery is research outside these fixed-order controls. Neither the search nor the smaller transformed base is counted as an original-problem completion.

An earlier wrapper attempt reached the default local-node cap and remains under `artifacts/hard-campaigns/bilinear-positive-debug-01/`. Raising the explicit research caps to one million refinements and ten million nodes produced the independently verified original proof under `bilinear-positive-debug-02/`. The release controls use those same caps. The wrapper's base has 59,536 cells; a prior mathematically equivalent base with a different constraint insertion order had 15,389. Those differently ordered runs are not a matched performance comparison.

The separate standard completion campaign in `artifacts/completion-bilinear-positive-01/` also solved and independently verified the original 80,980-cell result, in 25.662615 s and 15.778594 s on CPU6. Its 300-second solve/verification limits are separate from the matched controls above.

## Reproduction and provenance

- Comparison: `artifacts/bench/gluza30-bilinear-positive-release-01/comparison.json`, SHA-256 `ab5d197cebc8385474f122b783d92e8ac410ea150e872665ef48d13bf610b4e3`.
- Frozen release: `artifacts/bin/symgcad-bilinear-positive-ff22294`, SHA-256 `d9a9ba3a6df35fdc7c381014b0a76a0e1805392ca6ac5baadc3a1da4f74b72df`. The adjacent `-build.json` records the source commit, path-only dependency patch, isolated build, and 467 passing implementation tests across 32 targets.
- Native revision: `fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`; exact adapter, native metadata and supervisor hashes are in the retained `sha256.txt` manifest.
- Search: `artifacts/research/gluza-positive-ladder-order-release-02/order.json`, with original base input and provenance alongside it.
- All control inputs, configurations, per-trial result/verification hashes and process outcomes are under `artifacts/bench/gluza30-bilinear-positive-release-01/{native,rust}/`. `summarize.sh` reproduces the comparison and the hash manifest from those retained outputs.

For fresh repetitions, copy the configurations to new output directories before executing them:

```sh
nix develop .#reference --command bash artifacts/bench/gluza30-bilinear-positive-release-01/native/run.sh
nix develop --command taskset -c 4 artifacts/bin/symgcad-bilinear-positive-ff22294 bench artifacts/bench/gluza30-bilinear-positive-release-01/rust/campaign.toml
bash artifacts/bench/gluza30-bilinear-positive-release-01/summarize.sh
```

The positive Gluza33 problem is separate and remains unresolved at this measurement checkpoint.
