# A414 fixed-work release comparison

Three fresh paired release diagnostics measured median solver times of
**7.747562 seconds before and 6.916713 seconds after** combining primitive
retained-root fibers with the affine-prefix shortcut: 10.7% lower for this
fixed-work traversal. The first pair was 5.3% slower, prompting two additional
pairs with alternating execution order; that initial outcome remains saved.
Exact inputs, non-timing result fields, local counters and all 212 completed
algebra-operation records agree across all six runs.

All six runs intentionally stopped at the same node cap and returned
**Incomplete, zero cells**. This is neither a new original-problem completion
nor a full-solve or native-comparison benchmark.

## Fixed work and configuration

The input is the complete original symbolic A414 problem from the
[CPU profile](A414_SUBTREE_CPU_PROFILE.md), preserving `s > 4`, unrestricted
`t`, positive Feynman coordinates, parameters and split. Its mathematical TOML
prefix matches the regenerated original byte for byte. The given order is
`[s,t,x8,x1,x2,x3,x4,x7]`, with homogeneous anchor `x8` and the full positive
Feynman block. The local engine retains deferred projection, stable samples,
adjacent pairs, Descartes witnesses and intervals, and positive pair witnesses.

Each run uses CPU0, one worker, an 8 GiB memory limit, a 60-second wall limit,
100,000 local nodes and 20 million refinements. Each operation cache starts
empty in a separate namespace. Operand captures, lifting diagnostics and perf
sampling are disabled. The saved launcher refuses existing result or checkpoint
paths. Baseline and candidate TOMLs differ only in their checkpoint path.
The parsed exported problem also matches the earlier observed CPU-profile
worker input after removing that path and the deliberately lowered node cap.

The baseline release includes the primitive-linear persistence shortcut.
The candidate additionally contains
[primitive retained-root fibers](PRIMITIVE_ROOT_FIBERS.md) and the
[affine-prefix shortcut](AFFINE_PREFIX_EVALUATION.md); this comparison does
not isolate their individual effects.

| Observation | Baseline | Candidate |
| --- | ---: | ---: |
| Pair 1 solver time, seconds; baseline first | 8.546037874 | 9.001016540 |
| Pair 2 solver time, seconds; candidate first | 7.739950789 | 6.916712885 |
| Pair 3 solver time, seconds; baseline first | 7.747561778 | 6.804010723 |
| Median solver time, seconds | 7.747561778 | 6.916712885 |
| Median external CLI wall time, seconds | 7.80 | 6.94 |
| Median user CPU time, seconds | 7.52 | 6.71 |
| Median system CPU time, seconds | 0.12 | 0.11 |
| Peak RSS range, KiB | 36,872–36,908 | 36,864–36,892 |
| Speculative nodes | 100,001 | 100,001 |
| Refinements | 24,636 | 24,636 |
| Refinement cuts | 57,426 | 57,426 |
| Completed operation records | 212 | 212 |

The final reduced-stack level is five with exact prefix
`[5,0,1/10240,1/3,1/64]` in every run. The 100,001st node triggers the common
`local projection node limit` diagnostic. This evidence is an explicitly
unfinished local result, not a stale progress snapshot at a timeout.

## Exact comparison and limits

The comparison checks the entire saved result after removing only the named
timing fields and checkpoint paths. Solver time limits and other configuration
fields remain compared. It also separately compares every non-timing local
counter, canonical exported fields, and the complete set of operation-record
paths and contents after removing each operation's elapsed time. The 212
records contain the same exact algebra outputs and provenance.

Matching these incomplete exports, counters and operation records does not
establish identity of an unexported complete traversal trace. None has complete
coverage to verify. Each pair uses fresh processes and operation caches, with
execution orders baseline/candidate, candidate/baseline, baseline/candidate.
CPU affinity and cold operation caches are controlled; intervening host load,
filesystem caches and CPU frequency are not. The initial pair's reversal is
part of the evidence, not discarded as an outlier. These diagnostic medians
must not be presented as a speedup for completing A414 or for either component
optimization in isolation.

## Evidence

Ignored artifacts are under `artifacts/profile/a414-fixed-work-release-01/`.
They include all inputs/results, external process observations, exact operation
manifests, normalized comparisons, the original `comparison.json`, the final
`paired-comparison.json`, and saved launch/comparison scripts. Pair one ran at
09:32/09:45 UTC and the two follow-up pairs at 09:48–09:49 UTC on
10 October 2026. The initial pair was not overwritten. Large polynomials and
outputs remain outside Git.

| Item | SHA-256 |
| --- | --- |
| Original TOML | `4feff5d7177f5b1b92d523c808373aa91866099221a860c8d1653ecb9a95049f` |
| Baseline executable, source `af1d942` | `18f117360567d51134ea5c257c7b4c18a7097fe1d6b5f2979ceec26431d3ec90` |
| Candidate executable, source `875f537` | `2887414c30abddd437da4a216f50f6bbddfdd176577dacbec75734df80e64d85` |
| Baseline input | `f0e9d5c483284863449192783850f2ab14d8d44fb826d2cfb4a3f9eafedbdd26` |
| Candidate input | `9361112f8e1ba0befe0f2b074d84ac8128fb7b245dd79d6b7fe18501a9dd49d2` |
| Baseline result | `c6f028e70d8fe5b8158cff7452182094ba7cbfdb40b1a5c83ea5a0cc7ecf89bf` |
| Candidate result | `29c599218a978f0282ed5be2c79ee8dd56f8dd17b3b174355f2f8167481ed3b8` |
| Pair 2 baseline result | `eff835bc461a01bd5bd6afcad721e68869d011cc6f94073cf7bb66d0cbc2e9e2` |
| Pair 2 candidate result | `399f13ffaa7645d825af71fa13407e301ca7725f772139cda4a9c60ae09cef63` |
| Pair 3 baseline result | `be8e008deac2141362c3ef195157131990249e2d6771a50fa0f50161b8534848` |
| Pair 3 candidate result | `b11e9bb76be0824fae21706432a28a32172a82173beafce65c253c1fea6b2e4b` |
| Shared complete non-timing result | `cbf44d2274c4ba1ebe776c6e78bac4732c606f1bfdd234875bd1f28868021321` |
| Shared exact operation manifest | `d83d93ed8870fe8055b345f569552733a9d3c5d3a06f70dfe12273e7bddcc59b` |
| Three-pair report | `b1b3e5f4b2f82a9ebb6bf351b04d0b06977531e659701e67738d761ec1d1f433` |
