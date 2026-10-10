# Local ordering promotion: paired Gluza30 base experiment

The opt-in `solver.search_local_progress` policy has a measured benefit on the
five-dimensional positive Gluza30 ladder base. It remains disabled by default:
this experiment does not establish a gain over the full corpus or over every
heuristic portfolio. The original Gluza30 completion is recorded separately in
[the release comparison](GLUZA30_BILINEAR_RELEASE_BENCHMARK.md).

## What changed

The existing local engine did not emit the global-projection checkpoints used
by the ordering search's unfinished-trial score. As a result, local timeouts
were usually tied when selecting an order for a longer trial.

The new optional inventory records distinct polynomial count, total terms,
maximum total degree and maximum exact coefficient bit length. Along with
refinement count, these give the sum of `ln(1+x)` as a monotone structural cost.
The search uses this cost only to select timeout promotions within its existing
1-, 10-, and 60-second tiers. UCT rewards, pruning, final completed-winner
selection and incomplete fallback selection are unchanged. A low score is a
prediction, never a coverage or nonvanishing certificate.

The flag and policy version are included in the persisted search context.
Historical records default to the former behavior. Incremental inventory is
disabled when the flag is false; proof verification does not trust these
performance counters.

## Matched fresh-process comparison

Both configurations used the same immutable release binary from `6ef1d40`,
SHA-256 `aac964b6816be58594163552e254c4eb2c7b77a7be4639de6597c67d7618c812`.
The exact reduced problem and all solver settings differed only in the flag.
Three pairs used assigned CPUs 4, 6 and 12, reversing treatment order for the
middle pair. Every invocation had fresh checkpoints, seed 0, at most 256
rollouts, a 60-second search limit, 4 GiB of memory and one worker. The outer
solve limit was 330 seconds; its 95% inner allowance was 313.5 seconds. The
separate verifier had 90 seconds. Each supervisor campaign was capped at
450 seconds.

| Configuration | Verified completions | Cells | Median public solve, including search | Median verification |
| --- | ---: | ---: | ---: | ---: |
| Existing timeout promotion | 0/3 | incomplete | 313.624853 s | — |
| Structural promotion enabled | 3/3 | 15,389 | 65.101299 s | 2.499888 s |

The median search costs were respectively 60.032698 and 60.027343 seconds.
Median enclosing campaign wall times, including hashing, export and independent
verification where applicable, were 314.964805 and 69.944572 seconds.
The disabled runs are timeouts, so no finite solved-time speed ratio is claimed.

All enabled runs found the winning lifting order `[x1,x7,x2,x6,x5]` in trial 17
(zero-based), its first 10-second promotion. The corresponding exact solves
took 5.03–5.35 seconds. The search continued within its budget and the public
command performed and independently verified a final solve. One repeat also
completed a different legal order. Completed solves, rather than predicted
costs, selected the winner.

This is an ablation of promotion scoring inside MCTS. Earlier inexpensive-order
controls and the cold 300-second search are retained in
[the positive ladder research](GLUZA30_POSITIVE_LADDER.md); they are separate
experiments and are not silently combined into this matched comparison.
The previous multi-problem ordering study's lack of an overall MCTS gain is
also unchanged by this single-workload result.

Evidence is under `artifacts/research/local-promotion-ablation-01/`:
`design.json`, both source TOMLs, six independent process/proof/verifier
directories, and `summary.json`. The summary hash is
`7924baf9cd6c97500a75be2be06043b39bdbc34f7068e2d6663dad67953a56c5`.
The frozen Rust supervisor hash is
`45f7c3a08ad798d642c51c8bf5f66a2ebd55bf6e6c76719ba12565accb3f6202`.

Implementation qualification passed 478 library/integration/helper tests.
The separate interrupted-worker reporting fix subsequently passed the expanded
487-test suite. This experiment uses the earlier frozen executable, so timeout
comparisons use its enclosing process measurement; its historical incomplete
`statistics.total_seconds` field omitted final worker time.
