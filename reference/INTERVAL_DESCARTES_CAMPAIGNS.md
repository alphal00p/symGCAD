# Interval Descartes campaign outcomes

Production `e8e1cd4` passed 321 library/integration tests and independently
replayed historical five-cell, eight-cell fixed-A446, and twelve-cell symbolic
massless proofs. Its immutable debug worker is
`artifacts/hard-campaigns/interval-quadratic-debug-01/bin/symgcad`, SHA-256
`8c0b09b11013c5baf226fdc4832174517fe9136d54f2bca66e4e2b7c31e41dc5`.
This version also includes the quadratic-discriminant cache change; these
campaigns do not isolate the interval rule's runtime effect.

Both full original trials timed out. Each used one CPU, an 8-GiB limit,
300 seconds, the local engine, adjacent pairs, one pair cancellation,
global Descartes witnesses, interval covers and positive-pair witnesses.
Sibling batching and stable sample selection were disabled. Existing exact
algebra checkpoints were reused, so these are warm exploratory campaigns.

| Original problem | Wall seconds | Peak RSS KiB | Last published nodes | Interval omissions |
| --- | ---: | ---: | ---: | ---: |
| Symbolic A446 | 300.045 | 246,296 | 93,892 | 163 |
| Fixed A381 | 300.023 | 81,072 | 462,655 | 1,766 |

The A446 progress snapshot was published at 137.129 seconds, before its final
captured factorization. Its nine admitted interval templates had been reused
154 times. The final captured operand has 41 terms, degrees `(3,3,6)` in
`(s,t,x1)`, and maximum coefficient size 17 bits. No factorization result was
returned before timeout. This is a new operation to investigate, not a
standalone factorization timing inferred from the campaign.

The A381 snapshot was published at 299.988 seconds, with 7,602 refinements,
124 interval templates, 1,642 reuses and 52 finite-stock fallbacks. Its final
capture is a completed factor operation; it does not identify a stalled
algebra call. Neither traversal counters nor omission counts establish
coverage of an original solution region.

The retained records are
`artifacts/hard-campaigns/{a446,a381}-interval-descartes-debug-01/initial-x5-0/record.json`.
Their SHA-256 values are respectively
`59ca8d7247f668a0507d2551958599640762dd81ea4b93e39df06f22f06d673a` and
`decdd0c641cf34426c2a730813bd3ed282887ffbc2074de83baae62585100b2e`.
Each campaign's `summary-hashes.json` records immutable operand snapshots and
the exact last published prefix. The final A446 capture SHA-256 is
`caeb1e6c55cdb75dff0b31bfe989c1ca7f09b25db3ac1651e2905a4b3570baf5`.

The interval rule remains opt-in. These trials produced no new completed
original decomposition.
