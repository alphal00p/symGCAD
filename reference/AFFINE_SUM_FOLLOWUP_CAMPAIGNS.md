# Follow-up campaigns after exact square and sum reductions

These four bounded trials all timed out and add **zero** completed original
problems. Each used one CPU, an 8-GiB process limit and 300 seconds. They are
exploratory campaigns, not matched performance ablations.

| Original problem / configuration | Worker | Wall seconds | Peak RSS KiB | Last nodes | Last refinements |
| --- | --- | ---: | ---: | ---: | ---: |
| Fixed A381, global Descartes witnesses | b81 release | 300.039 | 101,044 | 1,993,159 | 298,738 |
| Fixed A381, interval and pair witnesses | b81 release | 300.061 | 215,172 | 1,027,319 | 25,698 |
| Fixed Ax381, interval and pair witnesses | b81 release | 300.179 | 1,606,244 | 550,824 | 58,038 |
| Symbolic A446, interval and pair witnesses | b813 debug | 300.071 | 514,964 | 103,588 | 15,432 |

All use the local engine, adjacent pairs, one pair cancellation, no sibling
batching, no stable sample selection and homogeneous anchor `x5`. The fixed
order is `x5,x8,x4,x6,x3,x0,x2`; the symbolic order is
`s,t,x5,x1,x2,x3,x4,x7,x8`. Global Descartes witnesses are on in all four.
The two A381 release trials shared an existing atomic operation checkpoint
store while running concurrently; Ax381 started cold. Their timings therefore
must not be used to isolate the interval option's cost or benefit.

The frozen b81 release executable SHA256 is
`34daa0ef54b7b870f95a96f6a6968eecc3caf495d39c39fbb30a4470672dc423`.
The frozen b813 debug executable SHA256 is
`e24b80cd8a8d69bb361b7ed08737393e8fcd8161eefa69d0bc3e1b3b44bdb6e7`.
The latter passed 341 library/integration tests before these campaigns.

Symbolic A446 reused its earlier exact operation checkpoints. The new sum
factor preconditioner gets past the previously captured 41-term factorization;
the last published snapshot now occurs at 298.196 seconds. Reported phase
costs are 98.320 seconds for factor handling, 38.703 for domain certificates,
38.911 for resultants and 26.809 for pair separation. The final captured
operation completed, so it does not identify a stalled algebra call. More
speculative traversal is not a coverage certificate or an end-to-end speedup.

Evidence lives under `artifacts/hard-campaigns/`, with the following campaign
directories and SHA256 values for `initial-x5-0/record.json`:

| Directory | Record SHA256 |
| --- | --- |
| `a381-global-witness-release-b81` | `3e59c30826d43595ae525e376eeb3e7544789e25f298923f11c5617d3bfec6b3` |
| `a381-interval-release-b81` | `ad809a5eafa0a2e52b7572db97baf7e694b1c5a27fb9a2486ca12fc081c4af91` |
| `ax381-interval-release-b81` | `67a34432a34bd90a7a5fd38945c59cad5119513c6a450c4fa6cf9293c144f434` |
| `a446-affine-sum-debug-01` | `a70f2d0f59980a74a7b29c086ec4f9f19405174455bfa7f2048b5e71aade0ea7` |

`affine-sum-followup-summary.json` retains compact process/phase metadata.
Every directory also retains the actual normalized worker input, provenance,
progress, checkpoint reference and failure status.
