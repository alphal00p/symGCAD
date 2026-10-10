# Gluza30 positive release traversal diagnosis

The original positive Gluza30 input remains unresolved. Two cold direct-worker
trials used release `dda5980`, SHA-256
`f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`,
the 49 verified strict Bézout proposals and fixed order
`[x2,x6,x7,x5,x1,x4,x0]`. Each worker had 600 seconds, 8 GiB, ten million
refinements and one hundred million speculative nodes. The exact original
mathematical fields and order were checked unchanged. Proposal discovery took
0.5100 seconds separately and is not included in worker wall time.

| Measurement | Ordinary samples, CPU 5 | Retained samples, CPU 4 |
| --- | ---: | ---: |
| Outcome | incomplete at wall limit | hard timeout |
| Last speculative nodes | 15,238,869 | 14,145,042 |
| Last refinements | 2,772,327 | 2,808,944 |
| Factor misses | 467 | 491 |
| Resultant misses | 302 | 322 |
| Strict Bézout resultant omissions | 632,290 | 681,556 |
| Retained sample hits | 0 | 3,232,079 |
| Peak RSS, KiB | 626,760 | 645,204 |
| Recorded resultant seconds | 1.9457 | 2.5156 |
| Recorded root-isolation seconds | 58.9348 | 51.2002 |

Counters for the hard timeout are the final published snapshot, not a completed
inventory. Different samples change the traversal; these counts cannot measure
relative coverage or demonstrate a speedup. Neither result is verified or adds
a completed original problem.

A ten-second 99 Hz profile was attached to the ordinary-sample worker, so its
reported timing includes that instrumentation. Of 969 samples, with none lost,
18.19% were in exact integer prefix evaluation and 8.06% in SipHash writes.
Rational cloning and allocation also contributed. The final report records
26.4 million factor cache hits but only 467 misses. Ordinary resultant
arithmetic is not the dominant cost in this configuration; repeated traversal,
evaluation and cached work remain substantial. No backend change is inferred
from this profile.

The retained comparison is
`artifacts/research/gluza-strict-bezout-release-01/comparison.json`, SHA-256
`85ab7c1b8140318a186e5e8cdfbaf82b380f16dc18c8fa6421c4083d838b9a24`.
It links both process reports, exact-input records, progress snapshots, cold
algebra checkpoints and profile data. The two reports preserve all failures;
the ordinary worker managed to export an explicitly incomplete result at the
deadline, while the retained-sample worker did not export a proof.

The separately derived [two-fiber reduction](GLUZA30_POSITIVE_LADDER.md) was also
tested with a longer release solve of its five-variable base. The original
given order completed projection but failed allocation at the 8 GiB limit
after 193.339 seconds. Its last lifting snapshot already contained 2,500,511
accepted cells and 2,271,747 visited stacks, while processing the outer sample
`x2=1/40`. This is a bounded allocation failure, not a completed base. The
report and checkpointed projection are retained under
`artifacts/research/gluza-positive-ladder-base-release-01/`. More suitable
conditional domains and local projection remain necessary before this reduction
can complete the original problem.
