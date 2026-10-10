# Audited original A414 deferred-projection pilot

This single cold diagnostic timed out after **300.034391 seconds**, with **178,404 KiB** sampled peak RSS. It produced neither a result proof nor an independent verification. It does not change the original-corpus completion count and is not a speed ablation.

The campaign is `artifacts/research/local-projection-followup-03/a414.toml`, using the original `0085_finiteA_6_414_10_0__0_5_1_1_1_0_0_1_1_symbolic.toml`, CPU2, an 8-GiB process-group limit, a 300-second solve deadline and a separate 600-second verification allowance within a 900-second campaign. The frozen release worker has SHA-256 `ab98822a1c0bb02129cfb20b26355ee7e415ded6b1bea00e3997dd087697bb9b`.

## Independent input and option audit

The actual final worker from one-hour case05 was recovered at `/tmp/nix-shell.krfqwo/symgcad-workers/2902147-300-input.json`. Its full order is directly recorded as

```text
[s,t,x8,x1,x2,x3,x4,x7], homogeneous anchor x8.
```

Thus this is an observed final worker order, rather than an order inferred solely from the last reduced projection variable map. The new actual worker input was `/tmp/nix-shell.MYnhJc/symgcad-workers/1002122-0-input.json`.

The original fixture, hour final worker, pilot worker and actual cold prepared TOML have exactly equal parsed `name`, `variables`, `constraints`, `split`, `parameters` and `variable_groups`. No polynomial equivalence approximation is needed: the strings and arrays match exactly. The original has no explicit fixed order; its groups are `[s,t]` followed by `[x1,x2,x3,x4,x7,x8]`, so the chosen order and first physical anchor respect the original constraints. Every homogeneous coordinate has its original strict positive-coordinate constraint.

Compared with the hour's selected final worker, the only solver-option changes are `local_stable_samples=true`, `local_deferred_projection=true` and the new cold checkpoint path. Both selected workers already say `ordering="given"`; the hour's public solve previously spent part of its one-hour budget on MCTS, whereas this pilot supplies the selected order directly. The original witness, interval, pair-witness, adjacency, positivity, cache and resource-cap options are retained. The refinement/node ceilings remain 20 million/200 million. The frozen binaries and total solve budgets differ, so these runs cannot isolate a performance effect of one option.

The immutable input copies and audit are under `artifacts/research/local-projection-followup-03/a414-independent-audit/`. `configuration-audit.json` has SHA-256 `3454e45985b1e4f31b6119e87131b80075ab9f2777dbacde35c1d9a59d6cb71a`; it records all source/copy hashes. The unchanged original source hash is `4feff5d7177f5b1b92d523c808373aa91866099221a860c8d1653ecb9a95049f`.

## Last saved progress

The final progress snapshot was published at 299.995988 seconds:

| Measurement | Value |
|---|---:|
| Speculative nodes | 2,389,781 |
| Refinements | 343,147 |
| Deferred stacks | 768,701 |
| Deferred frontiers/projection calls | 768,696 |
| Deferred fallbacks | 0 |
| Cut/child compactions | 0 |
| Subtree-cache hits / misses / evictions | 770,527 / 768,701 / 752,705 |
| Learned-cut hits / misses / evictions | 25,918 / 742,783 / 226,506 |
| Stable-sample hits / misses | 766,328 / 2,735,297 |
| Certified guard removals | 777,150 |
| Projected pairs | 146,318 |

The current reduced level was5 with prefix `[5,-393217,1,47/67,252831860/643]`. Phase counters report 65.27 seconds of root classification, 60.22 seconds of root isolation, 26.76 seconds of factor work, 24.47 seconds of pair-separation work, 2.70 seconds of resultants and 1.77 seconds of discriminants. These are instrumentation categories, not a complete exclusive attribution of the public solve wall time.

The last capture was a **completed** factor operation on a 2,185-character operand, taking 0.000315 seconds. Nearby snapshots continued increasing the node/refinement counters. The supported diagnosis is continued speculative traversal with frequent cache eviction; these records do not show a single algebra operation stalled at the deadline. No further cache enlargement or algorithm improvement is inferred from this one timeout.

The outer killed public-process outcome reports very little CPU time; its raw fields are retained, but they do not reliably account for the inner worker's CPU. Wall time, process-group RSS and exact progress counters are the relevant evidence here.

Final retained hashes:

- `final-outcome.json`: `0dd713a0f76b7e32114dca6d36709e46ff83fbdaaee5152a4da7c1034fc776a0`.
- `final-ledger.json`: `fb4d3d98173b7709074950713e592afeef9466b1550b39a838fe879853b7b971`.
- `final-progress.json`: `df7b362708ddd4da3feccfc6ac0eb963782a41343107a01b20c4765890363b48`.
- `final-active-capture.json`: `40f743959ef0970605efcdd12c5376aa49dbbb0940d4229e7ec02f764753fcd0`.

The one-hour queue, original evidence, and external benchmark documents were not modified by this audit.
