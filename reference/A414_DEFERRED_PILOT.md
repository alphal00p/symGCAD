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

## Witness growth and finite closure

A read-only source audit found **no global template-admission cap**. The existing limits are permanent stocks attached to one Builder's append-only canonical polynomial registry: two global Descartes separator vectors per polynomial, two interval covers by default (configurable up to eight), and two pair-order templates per sorted polynomial pair and axis. A successful admission consumes its slot before the caller tries its ancestor guards. Failed stacks, ancestor restarts, and proof/hint-cache evictions never replenish these stocks.

This still gives a finite symbolic closure in a fixed number of variables. It does not permit an infinite chain of fresh polynomials at the same variable level:

| Rule at highest axis `z` | Returned polynomial obligations |
|---|---|
| Ordinary individual/pair projection | Leading coefficient, discriminant and resultant, independent of `z` |
| Positive-ray quadratic and zero/one-variation rules | Endpoint and selected coefficient polynomials, independent of `z` |
| Global Descartes separators | Original coefficients and finitely many exact evaluations `p(c)`, with rational `c` replacing `z` |
| Interval Descartes cover | Endpoint, leading coefficient, finite seams and exact rational linear combinations of the coefficient polynomials |
| Pair separators | Two exact polynomial evaluations per cross-owner edge, with rational separators replacing `z` |
| One/two cancellation pair proof | All nonzero coefficients of one of two deterministic polynomial combinations, independent of `z` |
| Strict Bézout and domain removals | Existing original coordinate/bound factors in earlier axes; the current coordinate premise is discharged by the positive-ray rule |

The assertions in both raw projection functions require each cut's highest axis to equal the current axis. Factoring these obligations cannot introduce an eliminated variable. Canonical factor results pass through `Builder::intern`, which returns the existing ID for an equal polynomial and never reassigns an ID. Each fixed polynomial therefore has finitely many coefficient/discriminant/template outputs, and each fixed pair has finitely many resultant/cancellation/template outputs. Starting with the finite highest-axis inputs and descending the variable order proves finite closure. **Highest variable decreases; total polynomial degree need not decrease.** Resultants can have much larger degrees.

This is a closure argument, not a measured runtime bound or a separate proof of every scheduling path's termination. An ordinary visit adds a genuinely new same-axis cut on refinement. Deferred discovery also adds cuts, and its later compaction has a fixed limit and does not discover new children. Nevertheless finite nested arrangements, repeated visits at changed ancestor samples and eviction of reusable proofs can be extremely expensive. The argument is scoped to one Builder: hypothetical cross-run restoration of hints with fresh template stocks needs its own lifetime accounting. The current hint-archive option excludes these sample-template modes.

The saved counters distinguish admission from repeated use:

| Last saved counter | 300-second deferred pilot | One-hour case05 final worker |
|---|---:|---:|
| Global Descartes admissions | 46 | 1,035 |
| Interval-cover admissions | 15 | 118 |
| Pair-template admissions | 371 | 4,070 |
| Total admissions | 432 | 5,223 |
| Registered polynomials / terms | 2,242 / 36,200 | 87,864 / 812,184 |
| Maximum total degree / coefficient bits | 16 / 108 | 62 / 1,058 |
| Speculative nodes / refinements | 2,389,781 / 343,147 | 22,770,824 / 3,555,772 |
| Factor memory hits / uncached attempts | 3,371,787 / 2,262 | 40,445,425 / 86,706 |
| Resultant memory hits / uncached attempts | 146,271 / 48 | 2,745,826 / 64,498 |
| Discriminant memory hits / uncached attempts | 261,087 / 32 | 3,784,903 / 529 |

The hour snapshot is `artifacts/slow-suite-second-wave-diagnostic-01/final/05-progress.json`, at 3,299.928840 seconds of the final worker after the public solve's ordering search. These are different-duration, different-scheduling runs, not a controlled comparison. The reported `*_misses` increment only after disk reuse also fails; successful disk loads are separate counters, and the pilot had none. Its 146,318 projected-pair visits did not mean 146,318 newly computed resultants. Budget-fallback counters likewise count repeated calls, not distinct exhausted stocks or new polynomial families.

The reported roughly 303,000 operation files belong to the hour campaign's shared cold checkpoint directory, which also served ordering-search trials. The final worker itself records 151,733 checkpoint writes. A directory file count is therefore neither a final-worker polynomial count nor a template-admission count. The separate 36-of-10,000 small primitive-factor sample tests a narrow factor shortcut and cannot attribute the other records to witness generation.

The pilot does **not** demonstrate a template-admission explosion: 432 admissions accompanied hundreds of thousands of refinements and millions of repeated factor lookups. The hour does have a much larger finite registry, but present records do not label new factors by producing rule, level or admission. They cannot establish whether separator evaluations, ordinary coefficients, pair resultants or inherited guards caused most of that growth. Original coordinate factors themselves are a fixed finite input set. Zero compactions means the selected completed children retained all currently live cut obligations; it does not establish that another valid proof could not need fewer cuts.

A permanent global admission budget or a fixed rational menu would remain sound if old valid templates were tried first and exhaustion used ordinary discriminant/resultant projection. It would not remove original zero sets or confer proof authority. Such a budget can also replace a cheap witness with a much larger ordinary projection and worsen the computation. No new budget is justified by these measurements. Before that choice, useful missing evidence is newly interned factors attributed to rule and highest axis, versus already-known guard factors, plus admissions and repeated fallbacks by polynomial/pair. No production instrumentation or new solve was added for this audit.

## Completed-subtree materialization

`src/local_projection/subtree_cache.rs` stores bounded JSON payloads, not a `Built` object. Every successful `get` executes `serde_json::from_slice`, reconstructing the complete owned tree and required-obligation set. Each admitted insertion serializes the tree with `serde_json::to_writer`. Keys are exact rational prefixes in the current immutable Builder; a hit reuses all conditional obligations, and the caller still propagates them to its current ancestors.

The pilot made **770,527 hit deserializations**, with 768,701 misses and 752,705 FIFO evictions. The retained-node ceiling reached 32,768 while peak charged bytes were only 13,624,005 under the 33,554,432-byte budget. Per-entry limits remain 128 expanded nodes, depth 16 and 1 MiB. The learned-cut cache separately reached its 4,096-entry ceiling at about 3.8 MB, with 25,918 hits, 742,783 misses and 226,506 evictions. These counts identify repeated materialization and capacity churn; existing timers do not isolate their wall-time share.

An internal immutable proof DAG could potentially share nodes and obligations across exact-prefix hits, while recursively exporting the same ordinary tree certificate and leaving independent replay unchanged. Merely putting the existing byte payload behind an `Arc` would not avoid deserialization; storing `Arc<Built>` and immediately deep-cloning its owned children would not avoid tree copying either. The builder's child representation would need to retain shared immutable nodes through speculative traversal, with expansion only during output collection/serialization. Exact-prefix keys, complete returned obligations, stable registry/removal IDs, logical cell/node limits and final cell order must remain unchanged. Memory accounting must include shared nodes still reachable after cache eviction, not only entries remaining in the cache. Completed output size is still an inherent cost even if speculative trees share storage.

Before implementing that representation change, profile cache serialization/deserialization, rational-prefix hashing, allocations and destruction separately on the existing traversal workload. There is no measured serialization percentage or speedup claim in this audit, and no new worker was launched.
