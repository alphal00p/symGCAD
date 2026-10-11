# Final implementation checkpoint

This records the requested handoff, not completion of the all-cases objective.
The reproducible solver, original-corpus measurements and research diagnostics
have different scopes.

The exhaustive [root handoff](../HANDOFF.md) brings together the implementation,
full investigation history, counterexamples, reproduction instructions and
remaining research directions, including CFF, tropical methods and SOFIA.

## Implemented and usable

- Rust library and TOML CLI for normalization, projection, solving, ordering,
  verification and benchmark campaigns, using Symbolica exact algebra.
- Baseline generic projection/lifting, rational samples, algebraic root
  boundaries, explicit exceptional sets and replayable coverage evidence.
- Exact algebra/checkpoint reuse, certified positivity and homogeneity
  reductions, conditional linear/quadratic reductions, and experimental local
  projection. Interior zero sheets remain required, including same-sign seams.
- Bounded heuristic and MCTS ordering, process-enforced resource limits,
  restart records, and separate fast/slow test execution.
- Pinned Nix/dependency setup, native-GCAD comparison adapter, per-problem
  Typst/PDF benchmarks, and independent supplemental interior-point checks.
- FastSecDec graph-bundle interoperability with native U/F and an explicit
  edge-to-parameter map. Numerical integration and the full no-deformation
  pipeline are not implemented.

Experimental options and optional dependency patches remain explicitly
identified. The baseline phase-time comparison, isolated algebra experiments
and full original-problem completions are not interchangeable measurements.

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
identify modular reconstruction as the current cost. The subsequent
[isolated Wang experiment](GLUZA33_WANG_PREFLIGHT.md) completes the 2,410-term
factorization in 1.039 seconds; a matched disabled control times out at 60.034
seconds. Exported 106- and 821-term factors reconstruct the full signed source
exactly, each with multiplicity one. This optional backend patch is separate
from the unchanged production dependency. A truncated search of 4,096 cached
factor proposals found no divisor. With the optional patch, a further warm
original continuation saved 202 operations before timing out at 600.096 seconds
with 762.5 MiB peak RSS. It reached the earlier 19,942-term discriminant at a
different recorded prefix; no result or verification completed. The original
Gluza33 problem remains unresolved.

The new [exact shear preconditioner](SHEAR_FACTORIZATION.md) extends the existing
default-off affine option without changing any original zero set. Its automatic
policy factors a captured C470 obstruction in 0.087845 seconds, including exact
internal reconstruction. The original symbolic C470 continuation then saves
56,671 additional operations before its 300.028-second timeout, using 136.5 MiB.
A separate warm continuation saves 1,205 more before timing out at 600.061 seconds,
using 221.7 MiB. It remains incomplete. The [local-refinement control](LOCAL_REFINEMENT_PROFILE.md)
separately identifies successful-child obligation discovery as the main source
of refinement in a measured B471 prefix; cache hits alone establish no speedup.

The subsequent [progress-I/O profile](BASE_PROGRESS_THROTTLE.md) found an
unthrottled optional-support fallback report. Correcting it reduced system CPU
from 17.505 to 1.282 seconds in matched 60-second C470 trials, with unchanged
mathematics and preserved first/failure snapshots. Both trials remain incomplete;
activity counts are not certified coverage or solve-time ratios.
The subsequent original C470 continuation timed out at 600.050 seconds with
265.9 MiB peak RSS, retaining 279 additional operations. All 58,749 inherited
operation records and 4,096 support proposals passed their metadata checks;
no complete result or verification was exported.
The subsequent release run with larger conditional caches stopped at its
one-million-refinement limit after 343.614 seconds / 459.5 MiB. A fresh
[alternative-scale trial](LINEAR_PREFIX_RESEARCH.md) stopped at the same limit
after 231.608 seconds / 492.3 MiB, without filling its support registry.
Neither completed. The next proposed diagnostic distinguishes repeated child
obligations from genuinely new fibers before increasing resource caps again.

At Gluza33's next recorded prefix, the [interval-guard diagnostic](GLUZA33_FINAL_INTERVAL_FRONTIER.md)
reconstructs a complete four-piece positive-ray cover with 97 guards. All guards
factor and reconstruct exactly in a 33.730-second worker under an explicit
131,072 aggregate-term allowance. The new production option preserves the
65,536 default and every other guard/cover obligation. A compact-cover research
search reduces the inventory to 73 guards but is not production discovery.
With that explicit budget and the optional Wang backend, the original Gluza33
continuation passed the discriminant and retained **7,194 new operations**.
It timed out at **600.069 seconds / 541.5 MiB** on a different 1,283-term
factorization. All 16,739 inherited records were unchanged. This is measurable
progress through the original problem, but not a complete decomposition.

A [matched native control](AFFINE_COMPONENT_DISPATCH_RESEARCH.md) factors
the next 1,283-term source in 0.043 seconds, while the optional translation
stalls. An original release continuation with that option disabled passes the
factor in 0.0084 seconds and retains 140 further operations. It nevertheless
times out at 600.101 seconds / 859.6 MiB on the large discriminant at prefix
`[2,1/4]`, with no result or verification. Exact isolation now finds four
positive roots, so the earlier two-root cover cannot simply be reused.
A separately reconstructed compact cover fits the existing 131,072-term
guard budget; the lifetime template quota remains an independent limit.
The new production coalescer subsequently reproduces that four-root cover
with 97 guards and 109,726 terms in 1.604 seconds; fresh exact verification
passes in 1.538 seconds. This isolated fresh-stock check does not establish
that the original solve's accumulated templates permit the same step.

The [diagonal-scaling study](DIAGONAL_FACTORIZATION_RESEARCH.md) preserves
monomial support while correcting the same evaluation-fiber degree drops.
It exactly factors both the historical translation success and the new
translation regression. These algebra controls motivate the bounded optional
policy; they are not new original-case completions.

The combined coalescence/scaling release subsequently times out after
**600.126 seconds / 859.9 MiB**, with the same large discriminant at the
different recorded prefix `[2,3/2]`. It retains 244 new completed operations;
all inherited records and the resulting 24,317-operation cache pass the audit.
Fresh production replay at that point
preserves three positive roots. The interval certificate needs 73 guards /
79,798 terms; the existing global saturated-Descartes certificate instead
needs **27 raw guards / 22,436 terms**, including all 25 original coefficients.
Its construction takes 0.060 seconds and exact replay 0.026 seconds. Raising
the existing global source allowance from 8,192 to 32,768 admits this proof;
the original control declines at source admission. Finite template stocks
and every proof obligation remain unchanged. These diagnostics do not
reconstruct the original worker's stock history or prove domain coverage.
The final original configuration trial raises only that source allowance and
uses a fresh copy of the audited cache. It still times out: **300.101 seconds
total campaign, 299.434 seconds worker, 638.3 MiB peak RSS**, with the large
discriminant at the recorded quarter prefix. No result or verification is
produced. It retains 89 further factorizations; the resulting 24,406-operation
cache passes the complete metadata/hash audit. This leaves Gluza33 unresolved and motivates source-specific
template/call-path diagnostics before attributing the fallback to stock
exhaustion or raising further limits.

The subsequent [source-specific trace](GLUZA33_WITNESS_STOCK_TRACE.md)
confirmed that both permanent interval-template slots had been consumed.
Increasing the existing stock limit to four admits the new exact cover and
bypasses that discriminant. A first bounded trial saves 1,094 operations;
the next 600.096-second continuation saves another 2,397, reaching 27,897
audited records. It still produces no complete result or verification and
stops during a different factorization. The earlier pending factor completes
in 0.076 seconds on resumption, illustrating why a deadline capture alone
does not identify a slow algebra operation. The newest captured source has
2,867 terms and three active variables.

The [final main-axis experiment](GLUZA33_FINAL_FACTOR_PUSH.md) clears that
captured factorization. A general bounded lowest-degree-axis heuristic factors
its 1,731-term square-free component in **0.280 seconds**, and the complete
2,867-term source in **1.951 seconds**, with independent signed reconstruction.
A same-binary default-axis component control times out at 30.033 seconds;
the second-leading alternative times out at 60.021 seconds. These are single
diagnostic observations. All optional Wang candidates reject; the successful
route is the changed axis and ordinary native fallback. Generic admitted
controls preserve rational units, multiplicities and inactive map entries.
The separately replayable patch remains optional, leaving production unchanged.

The final warm original continuation with that optional backend times out at
**6,603.858 seconds**, with **1,963.508 MiB peak RSS** under a 50-GiB cap.
It clears the captured factor inside the original solver, then continues
through resultants. The cache retains **32,456 operations, 4,559 new**.
No decomposition is exported and no verification runs. The original problem
remains unresolved; operation counts are not cell counts. The
[terminal record](GLUZA33_FINAL_FACTOR_PUSH.md) and [handoff](../HANDOFF.md)
retain the source bindings, integrity-audit scope and remaining frontier.

## Validation and performance evidence

The final integrated production fast suite passed **662 tracked tests across
51 targets**, with zero failures and two deliberately expensive tests ignored.
It used a clean isolated worktree, public Cargo sources and no reference
checkout. Test bodies took 74.80 seconds; fresh compilation took 17 minutes
49 seconds separately. The unrelated alternate-mass test was absent. The
[validation record](COMBINED_FAST_VALIDATION.md) binds the exact source and logs.
This includes the opt-in affine factorization policy's module and solve/verify
regressions, support-preserving diagonal scaling, exact adjacent-interval
coalescence, configurable source and aggregate-guard budgets, and progress I/O. Research helpers
retain their own focused checks.

The optional completed-proof cache entry limit is now configurable from 128
through 4096 nodes, with the original 128-node default and unchanged total
memory, depth and payload bounds. Exact replay and same-sign interior seams
remain mandatory. The [cache investigation](C470_REFINEMENT_OBSERVER.md)
separates admission diagnostics from original-case completion and timing.
Matched ordinary-release controls reach the same refinement limit in 8.398
and 6.958 seconds with entry limits 128 and 1024. A subsequent 600.084-second
trial still times out, saving 1,295 new operations for a 61,671-record restart
snapshot. Neither activity timings nor retained algebra establish a solve-time
gain; C470 and the original completion count remain unchanged.

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
but the production dispatcher is unimplemented. A final-axis audit additionally
replays three applicable smaller-resultant identities, retaining the exact
71-factor projection basis. Its full applicability scan times out at 60 seconds;
the remaining largest pairs are inconclusive and no solver speedup is measured.
The [SubTropica review](TROPICAL_SUBTROPICA_RESEARCH.md)
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
