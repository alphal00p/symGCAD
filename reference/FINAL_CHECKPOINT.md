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

The subsequent [8,192-term witness-budget continuation](DESCARTES_SOURCE_BUDGET_PRODUCTION.md)
passed that discriminant and saved **4,554** further exact operations. Its
development worker timed out at 300.058 seconds with 402.5 MiB peak RSS on a
new 19,942-term degree-24 discriminant. A separate exact probe found two
positive roots against four Descartes variations there; merely increasing
the same budget cannot certify that next fiber. A separate
[interval proof](GLUZA33_INTERVAL_BUDGET.md) constructed and factored 49 guards
in 9.271 seconds, all within current non-source resource limits. The ensuing
[interval-source production option](INTERVAL_SOURCE_BUDGET_PRODUCTION.md)
passed that discriminant and saved **6,422** additional exact operations in
a warm original continuation. It timed out at **600.055 seconds / 364.2 MiB**
on a new 2,410-term factorization, with no result or verification. Both
discriminant improvements therefore establish progress, not completion.

The [next-factor diagnostics](GLUZA33_AFFINE_BUDGET.md) find regular all-ones
evaluation fibers, so a larger affine source cap would not activate the current
heuristic. Ten exact univariate factorizations instead show consistent small
factor degrees, including a degree-two plus degree-six split along the
lowest-degree axis. These specialized factors do not establish a factorization
of the original source or a completed Gluza33 decomposition.
Matched automatic, univariate and bivariate Symbolica starts each timed out
at 60 seconds on that operand; no simple strategy switch was successful.

The [factor profile and leading-coefficient preflight](GLUZA33_FACTOR_PROFILE.md)
identify modular reconstruction as the current cost. A bounded exact probe
finds a suitable candidate for the necessary Wang image gates; full assignment
and source reconstruction remain unproved. A separate truncated search of
4,096 cached factor proposals found no divisor. Neither experiment solves Gluza33.

The new [exact shear preconditioner](SHEAR_FACTORIZATION.md) extends the existing
default-off affine option without changing any original zero set. Its automatic
policy factors a captured C470 obstruction in 0.087845 seconds, including exact
internal reconstruction. The original symbolic C470 continuation then saves
56,671 additional operations before its 300.028-second timeout, using 136.5 MiB.
A separate warm continuation saves 1,205 more before timing out at 600.061 seconds,
using 221.7 MiB. It remains incomplete. The [local-refinement control](LOCAL_REFINEMENT_PROFILE.md)
separately identifies successful-child obligation discovery as the main source
of refinement in a measured B471 prefix; cache hits alone establish no speedup.

## Validation and performance evidence

The latest complete production fast suite passed **642 tracked tests**, with
zero failures and two deliberately expensive tests ignored. A separate
workstream's uncommitted alternate-mass test was excluded from this run,
the tracked count and the original corpus. The
[validation record](COMBINED_FAST_VALIDATION.md) binds the exact source and logs.
This includes the opt-in affine factorization policy's module and solve/verify
regressions and both configurable witness-source budgets. Research helpers
retain their own focused checks.

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
are unresolved. The [ordered quadratic identities](QUADRATIC_FLAG_IDENTITIES_RESEARCH.md)
give a counterexample to stationary-face sufficiency and a narrower exact
route through smaller resultants; eighteen Symbolica identity checks pass,
but the production dispatcher is unimplemented. The [SubTropica review](TROPICAL_SUBTROPICA_RESEARCH.md)
recommends joint scale-variable and short linear-prefix ordering proposals.
The [implemented diagnostic](LINEAR_PREFIX_RESEARCH.md) completed all 24
two-stage candidates across three symbolic families in 3.945 seconds. It
distinguishes some elimination pairs by exact projection growth, but the new
anchor pairs tie on their best costs; no gauge advantage is established.
The three promoted original-problem orders each exhausted 60 seconds with no
complete result. Their mathematical inputs were independently checked unchanged;
completed operations and unfinished factor operands remain available for reuse.
Tropical support and integration reductions do not justify deleting interior
threshold boundaries; no GCAD performance gain is claimed from that review.
The [tropical follow-up](TROPICAL_FOLLOWUP_RESEARCH.md) proposes sparse exact
AM–GM certificates and graph-cut coefficient allocation. Strict coefficient
margins, not support geometry alone, would justify nonvanishing removal.
The first [captured-guard support screen](TROPICAL_CIRCUIT_SCREEN.md) found
only three factors within its proposed 128-term cap, all already
coefficient-positive. Its eighteen mixed-sign factors have opposite-sign
exposed vertices in parsed metadata; whole-orthant positivity is therefore
an unpromising target for that snapshot. No new Symbolica certificate was
constructed. Further candidates need independently justified subdomains.
The [conditional follow-up](TROPICAL_CONDITIONAL_CERTIFICATES.md) gives an exact
AM–GM rule incorporating proved monomial domain inequalities. It can overcome
the global Newton-vertex obstruction on suitable chambers, but is unimplemented
and has no measured corpus benefit. Its equality cases retain interior zeros.

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
