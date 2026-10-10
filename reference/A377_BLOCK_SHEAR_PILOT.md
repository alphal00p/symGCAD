# A377 triangular block-sum pilot

On 2026-10-10, both the original and the exact double-shear A377 problems
exhausted their 60-second solve limits, returning `incomplete` with zero
exported cells. The smaller transformed polynomial did not produce a
completion in this pilot. The trials followed different algebraic paths and
their last progress snapshots are stale; they establish no speedup or
original-problem coverage.

## Exact inputs and controls

The original is symbolic
`finiteA_6_377_10_0__5_0_0_1_1_1_1_0_1`, imported as
`artifacts/corpus/symbolic/0081_finiteA_6_377_10_0__5_0_0_1_1_1_1_0_1_symbolic.toml`.
The full domain remains `s > 4`, unrestricted `t`, and positive Feynman
coordinates. There is no fixed-kinematics substitution. Both inputs retain
the legal lifting order

```text
s, t, x0, x3, x4, x5, x6, x8
```

In the transformed input the two changed coordinate names denote

```text
new x3 = old x0 + old x3
new x6 = old x4 + old x5 + old x6.
```

Their positivity constraints are the exact wedges `x3 > x0` and
`x6 > x4+x5`. The unchanged coordinates remain positive. Inverse substitution
was checked exactly with Symbolica for every constraint and the signed split
at both stages. The normalized split has 42, then 37, then 23 terms;
normalization clears the original rational parameters by a positive scale.
The [preparation report](LITERATURE_REGIONS_SOFIA.md) records the identities
and helper provenance. Both public `order` calls also accepted the same
homogeneous anchor `x0` and full six-coordinate block; their output hashes
agree. The shear commutes with this positive scaling.

Both runs used the unchanged release built from `875f537`, with SHA-256
`2887414c30abddd437da4a216f50f6bbddfdd176577dacbec75734df80e64d85`.
They ran sequentially on CPU9, with one worker, 2 GiB memory, 60 seconds
inside the public solver and a 75-second outer watchdog. Separate empty
checkpoint namespaces were enforced before launch. Both enabled operation
capture. The limits were 100,000 local nodes and 20,000,000 refinements;
neither run reported reaching these work caps.

Matched options were local/given ordering, no merging, seed zero, 128 MiB
operation cache, binary resultants, domain positivity, adjacent pairs,
first-axis projection shortcut, completed-subtree and learned-cut reuse,
Descartes one-root/multiple-root/interval certificates with interval stock
two, and one pair cancellation. Sibling batching, stable samples, deferred
projection, hint archives, input-positivity factoring, operation pruning and
pair-separator witnesses were disabled. Parsed solver options agree except
for the fresh checkpoint path; all mathematical fields agree with their
respective exactly checked source inputs.

## Observations

| Measurement | Original | Double shear |
|---|---:|---:|
| Reported total solve seconds | 60.035567 | 60.032588 |
| External command wall seconds | 60.04 | 60.03 |
| User / system CPU seconds | 58.24 / 0.92 | 58.16 / 0.94 |
| Peak RSS, KiB | 52,752 | 80,120 |
| Exported cells | 0 | 0 |
| Final persisted operation files | 600 | 444 |
| Last progress timestamp, seconds | 7.664523 | 45.407661 |
| Nodes / refinements in that snapshot | 13,029 / 2,478 | 12,810 / 1,607 |
| Current prefix in that snapshot | `[9/2]` | `[5,0]` |
| Completed segmented resultants in snapshot | 3 | 18 |
| Multiple-root / interval discriminants omitted in snapshot | 118 / 102 | 0 / 0 |

Both public commands exited normally with `status = incomplete` and the
diagnostic `worker timeout: `. The outer watchdog therefore allowed the
interrupted diagnostic export to finish. No verification command was run
against these incomplete exports.

The last captured unfinished operation for the original was a discriminant
in `t` (18,352-byte operand). For the shear it was a resultant in the
transformed `x3` (166-byte and 4,560-byte operands). Both captures retain
their exact operands, have `complete = false`, and have no corresponding
completed operation checkpoint. These identify the final captured
obstructions, not a measured duration spent in either operation. Progress
timestamps and counters must not be treated as final traversal totals or
equal-work comparisons; additional operation files appeared after each
snapshot.

The changed axes no longer satisfy the solver's *literal coordinate*
positive-ray premise: their supplied constraints are wedges. The observed
loss of higher-degree Descartes omissions is consistent with this change,
but this experiment does not isolate its causal cost from the other changes
in projection. No artificial coordinate-positivity assumption was supplied.
The next useful question is whether an exactly justified translated-ray
certificate can recover those rules while retaining the wedge, before
allocating a longer shear trial. Original-coordinate proof transport remains
a separate prerequisite for any completion credit.

## Retained evidence

All inputs, scripts, process logs, captures, checkpoint manifests and the
structured comparison are retained locally under
`artifacts/research/a377-block-shear-pilot-01/`. The preparation work is
separate from the timed solve commands; no end-to-end timing advantage is
claimed. The original mathematics and exact shear recipes are reproducible
from the corpus and the unchanged research preparation helper. Large
operands and local artifacts are not committed.

| Record | SHA-256 |
|---|---|
| Original imported TOML | `d00c3ebceebe641501c20a6dc28335f7502ee2c0e3c1e8118b388e31ba581e24` |
| Original pilot input | `fc5938138a2801a538805b377b40bcb61eb268e760f5e1236e60094faba293f1` |
| Shear pilot input | `3dbd6d8adf6f4a2778b503bf041038d4f13a99ab32387296cad4c6362c493b6d` |
| Original incomplete result | `9015d635f3f34710d029edd112317698d854bb46b3cc8eeae37bcaccb2c1b811` |
| Shear incomplete result | `39bd3cbe6a068e563e62967c73411a8bc962f3dfcb7dfe0b6227038c39b6fd81` |
| Original final capture | `284c814cb2fe59c07edcc42ff7f027945ea7fd7dc536cb2478eeaa6c1b1ecee8` |
| Shear final capture | `ca7e9f2a86263720d12cb561292a1f5c620b9453fd5df975de42d8df8c5f51fe` |
| Structured report | `5dd4d034e44dff0b61370ffece162665f923e9d27af2a2f526257a2233c3456a` |
| Evidence manifest | `873009549e06e06bbed0cbd22acc652979acdb00c59e39422e2c9e2876a2f9f8` |
