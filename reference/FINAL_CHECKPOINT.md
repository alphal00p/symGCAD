# Final implementation checkpoint

This records the requested handoff, not completion of the all-cases objective.
The reproducible solver, original-corpus measurements and research diagnostics
have different scopes.

## Verified original problems

| Corpus | Verified | Unresolved |
| --- | ---: | ---: |
| Fixed-kinematics ZZ | 124 | 0 |
| Symbolic ZZ | 97 | 27 |
| Core and disabled examples | 82 | 1 |
| Total distinct original problems | **303** | **28** |

The remaining core problem is positive Gluza33 at fixed kinematics. Every
remaining ZZ problem has both external invariants s and t symbolic. Thus the
remaining problems are **not all parametric in their external kinematics**.
Fixed A446 and its crossing are solved; the complete symbolic domains are not.
Original identifiers, aliases, timings, cells and provenance remain in
[`benchmark/data.json`](../benchmark/data.json) and the
[benchmark PDF](../benchmark/benchmark.pdf).

All 28 unresolved original problems exhausted their first one-hour campaigns.
The subsequent original positive Gluza33 continuation exhausted two hours
with a 50 GiB allowance, reporting 3,995,344 KiB peak RSS. It produced no
ground-truth decomposition. The separate follow-up algebra experiment was
interrupted at the user's wrap-up request; it was neither a completed solve
nor a two-hour timeout. Completed exact operation caches remain outside Git.
See the [original attempts](GLUZA33_AFFINE_ORDER_CONTINUATION.md) and the
[algebra experiment](THREE_PARAMETER_SEGMENTATION_RESEARCH.md).

The subsequent [affine-factor continuation](AFFINE_GLUZA33_CONTINUATION.md)
passed the captured 224-term factor obstruction in 0.152 seconds and saved
384 additional exact operations. It timed out after 300.021 seconds on a
6,938-term degree-16 discriminant, with 452 MiB peak RSS under a 50 GiB
allowance. This warm diagnostic adds no completion credit.

## Validation and performance evidence

The latest complete production fast suite passed **614 tracked tests**, with
zero failures and two deliberately expensive tests ignored. A separate
workstream's uncommitted alternate-mass test also passed; it is excluded from
the tracked count and the original corpus. The
[validation record](COMBINED_FAST_VALIDATION.md) binds the exact source and logs.
The subsequent opt-in affine factorization policy has separate module and
solve/verify regressions; its full-suite follow-up is recorded with that
milestone. Research helpers retain their own focused checks.

All **66 release solve-and-verify trials** for the quadratic-certificate
ablations and quadratic-ladder examples completed. The crossed A446 chamber
t < 4 has five verified cells and passed ten fresh exact interior-point
checks. This chamber leaves its original unrestricted symbolic input
unresolved. See [release measurements](QUADRATIC_RELEASE_ABLATION.md) and
[conditional validation](QUADRATIC_LADDER_VALIDATION.md).

The frozen matched-baseline study reproduced and independently verified all
483 sign-expanded cases completed by native GCAD. Across three paired runs,
the geometric mean ratio of measured algebra phases was **0.1466**. It is not
an end-to-end ratio for all 331 groups, nor a speed ratio for censored cases.
New reference runs obey the cap `max(300 s, 3 * completed symGCAD solve time)`.
Search, preprocessing, verification and warm-cache reuse remain identified
in the retained evidence.

## What remains research

The [CFF assessment](CFF_KINEMATIC_PROJECTION.md) supports conditional
kinematic projection and graph-derived sign certificates. Physical threshold
information alone does not remove cylindrical boundaries. The
[selected-positive-root design](SELECTED_POSITIVE_ROOT_DELINEABILITY.md) and
[exact critical-exclusion gate](SELECTED_CRITICAL_CERTIFICATE_GATE.md) give
explicit proof obligations for avoiding irrelevant resultant collisions.
They have not become a general production solver.

The [quadratic-face diagnostic](QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md) replays
all 27 stationary-face identities in a graph-derived three-coordinate cube
and measures completed ordinary projection through that block. The remaining
prefix decomposition, boundary ordering and original-coordinate restoration
are unresolved. The [SubTropica review](TROPICAL_SUBTROPICA_RESEARCH.md)
recommends joint scale-variable and short linear-prefix ordering proposals.
Tropical support and integration reductions do not justify deleting interior
threshold boundaries; no GCAD performance gain is claimed from that review.

The [Taylor-shift experiments](FAST_TAYLOR_SHIFT_RESEARCH.md) and
[positive-axis isolation investigation](POSITIVE_AXIS_ISOLATION_RESEARCH.md)
record the final bounded root-isolation work. Their measurements do not add
original-problem coverage. Exact addition for a unit Taylor shift reduced
the captured operand's shift time by 39% with coefficient equality. In the
combined isolated backend, positive-only isolation took **557.983 s versus
765.808 s** for all roots, with **421.2 versus 813.1 MiB** peak worker RSS.
All 145 positive roots and native multiplicities matched exactly. The
patched full inventory also matched all 294 roots of the fresh unmodified
control; its wider wrapper timer prevents clean attribution of that timing
change. These are single-run research measurements, not production CAD
speedups. The replay package passed eleven helper/shared-sign tests and two
unit-shift adapter tests; eighteen production root tests also passed against
the isolated backend. All interior F=0 boundaries remain required.

Dependencies, graph producers, generated corpora and large proofs are excluded
from the public repository. The [setup instructions](../docs/SETUP.md) fetch
pinned sources, explain optional replayable patches and reproduce the CLI,
native comparison, FastSecDec interchange and benchmark documents.
