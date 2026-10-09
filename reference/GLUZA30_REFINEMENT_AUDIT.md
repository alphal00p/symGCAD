# Gluza30 local-refinement audit

The measured cap is not explained by an immediate same-stack non-progress
loop or by enabling learned-cut hints. The evidence is consistent with a large
number of discovered conditional regions, with repeated work after ancestor
refinement and bounded-cache eviction still possible. The retained reports do
not count distinct `(prefix, cut set)` events, so they cannot establish the
exact fraction of repeated fibers or the number of final mathematical cells.

Both controlled trials used immutable release `7629a91`, SHA-256
`f3479621ada2731221775dea3c04f5552fb0c683af06a67262ce160872132805`,
the full original Gluza30 F<0 input, order
`[x0,x6,x4,x2,x5,x7,x1]`, one CPU, 8 GiB, a 300-second solve limit,
100,000 permitted refinements and two million permitted nodes. Comparing the
retained worker TOMLs shows exactly one difference:
`local_reuse_learned_cuts = false` in the second trial. These are single-run
diagnostics, not a statistically established timing improvement.

| Measurement | Learned hints on | Learned hints off |
|---|---:|---:|
| Result | refinement cap | refinement cap |
| Process seconds | 44.983 | 43.731 |
| Speculative nodes | 607,411 | 566,492 |
| Refinements | 100,001 | 100,001 |
| New cut additions across stack visits | 254,784 | 270,714 |
| Factor cache misses | 20,962 | 20,889 |
| Resultant cache misses | 8,748 | 8,691 |
| Completed-subtree hits | 103,024 | 110,998 |
| Learned-cut hits | 8,499 | 0 |
| Peak RSS, KiB | 185,908 | 183,768 |

The implementation at commit `7629a91` requires strict growth of the current
stack's cut set before incrementing the refinement counter
(`src/local_projection.rs`, lines 1582–1598). A discovered guard that is already
a cut instead returns `Unsupported` (lines 1521–1527); such a failure would
leave a different fallback/evidence kind from the observed refinement-cap
result. A sample lying on a *new* projection guard is an expected refinement
trigger, but the existing report does not separate that trigger from new
obligations returned by a successfully built child.

Each recursive visit begins with the input cuts and optionally the learned
cuts for its exact rational prefix. Ancestor refinement can revisit a prefix,
and the learned cache is intentionally bounded to 4,096 entries. Therefore
the strict-growth invariant is local to a visit, not a claim that every
reported refinement discovers a globally new polynomial. Turning hints off
produced the same failure and nearly the same algebraic operation counts;
this does not support blaming learned-cut oscillation for the cap.

The earlier 120-second debug on/off experiment also failed in both modes,
with 47,986/46,552 refinements and identical 7,508 factor and 6,258 resultant
misses. Its order and options differ, so it is supporting evidence only.

| Retained artifact | SHA-256 |
|---|---|
| `artifacts/gluza30-local-release-7629/results/ledger.json` | `295e4a24ce1805be6a592f64ddb304c62086f89a755fd345c4d56471784fb48f` |
| `artifacts/gluza30-local-release-7629-learned-off/results/ledger.json` | `61cc8a35aa7884e8903de6f3c341dee72d88d1f67a7bf2b00b21e3dfb87c1ec3` |
| `artifacts/hard-campaigns/gluza30-learned-frontier-debug-01/analysis.json` | `a4f1f5419afa4f258c13d60ba31bf74fd46a8dcf6de49733a0e01a0185230de8` |

Reproduce either release trial with a fresh output directory using its adjacent
`campaign.toml` and the immutable executable. No partial-domain result or
incomplete trial in this audit counts as a corpus completion.
