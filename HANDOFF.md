# symGCAD — exhaustive implementation and research handoff

This is the long-form handoff for the implementation and investigations requested in October 2026. It is intended to let another developer or researcher continue without reconstructing the chat history. Read the status and correctness contract before interpreting any performance number. The shorter entry points are [README.md](README.md), the [implementation checkpoint](reference/FINAL_CHECKPOINT.md), and the [benchmark PDF](benchmark/benchmark.pdf).

**The all-corpus objective is unfinished.** The last audited original-problem aggregate contains **303 verified groups out of 331**, including all **124 fixed-kinematics ZZ groups**. There are **27 unresolved symbolic ZZ groups and positive Gluza33 at fixed kinematics**. Fixed A446 and its crossed counterpart have complete, independently verified decompositions. Their full symbolic domains remain unresolved. The final Gluza33 attempt is recorded in the campaign section below; an isolated factorization success is not an additional solved group.

This handoff is a requested stopping point. It does not declare that all valuable ideas have been exhausted, that Gluza33 is impossible, or that the original goal has been achieved. No continuation should silently turn a timed-out original, a solved specialization, or a solved kinematic chamber into an all-domain completion.

## Contents

- [Objective, scope and execution constraints](#original-objective-scope-and-changing-execution-constraints)
- [How to interpret the evidence](#how-to-read-the-evidence-without-overstating-it)
- [Correctness invariants](#central-correctness-contract-and-non-negotiable-invariants)
- [Architecture, corpus, verification, benchmarks and setup](#architecture-exactness-and-validation)
- [Gluza33 and exact algebra: complete investigation history](#gluza33-and-exact-algebra)
- [Physics, CFF, graph structure, tropical sampling and SOFIA](#physics-graph-structure-cff-and-tropical-research)
- [Other algebra, root and proof-maintenance bottlenecks](#other-algebra-root-and-proof-maintenance-bottlenecks)
- [Resumption priorities, reproduction and publication](#practical-resumption-publication-and-stopping-record)
- [Complete reference-note index](#reference-index)

## Original objective, scope, and changing execution constraints

The exact objective saved in [SYMGCAD_PLAN.MD](SYMGCAD_PLAN.MD) is:

> Build symGCAD entirely in Rust, using Symbolica for symbolic and algebraic operations, with a reusable library and TOML-capable CLI. Reproduce the original GCAD strategy with a baseline runtime geometric mean no greater than 1.25 times the native upstream implementation. Develop and benchmark mathematically sound performance improvements, including bounded heuristic-guided MCTS for variable ordering. Produce validated, complete generic decompositions for every distinct problem in the reference corpus, including current intractables, prioritizing the pictured A446 topology at fixed kinematics before its symbolic counterpart.

The motivation is the no-contour-deformation approach for sector-decomposed Feynman-parametric integrals. The reference PDF is intentionally external, with its hash in [PROVENANCE.toml](PROVENANCE.toml). symGCAD's deliverable is exact generic decomposition, algebraic boundaries, replayable validation and benchmarking. It is not yet a numerical integrator or a complete sector-decomposition/no-deformation pipeline.

The requested source of future graph-driven problems is FastSecDec's dressed HEPKit DOT graph input. A separate `no_deformation` branch prepares graph/U/F exchange. FastSecDec remains an external repository, is never vendored into symGCAD, and its full downstream no-deformation algorithm has not been implemented.

The user subsequently specified several constraints that remain part of the handoff:

- Preserve exact analysis of **interior boundaries**, including ordinary `F=0` sheets and same-sign singular seams. Physics intuition, support geometry, integration measure-zero arguments and sampled signs cannot erase these obligations.
- Improvements must be principled and reusable across classes of graph-derived problems. They must not dispatch on one fixture's name, polynomial fingerprint or a memorized successful coordinate. Hashes are appropriate for authenticated cache/evidence identities, not ad hoc mathematical shortcuts.
- Keep a fast regression suite and classify unresolved original workloads separately as slow. The initial slow suite grants up to an hour per original. A timeout stays incomplete and remains in the reports.
- Permit reasonable larger memory budgets. The latest specific Gluza33 authorization permits **50 GiB and up to two hours**. The ordinary campaign envelope remains at most 16 CPU workers and 64 GiB aggregate; one symbolic worker is normally pinned to one CPU. A large configured limit is not evidence of large actual RAM use.
- New native-reference measurements are capped at `max(300 seconds, 3 * completed symGCAD solve time)`, respecting any shorter explicit/campaign limit. A censored native result does not define an exact finite ratio.
- Keep `benchmark/benchmark.typ` and its PDF published. List every original group, sorted by verified solve time followed by incomplete entries. Put at most two pages of explanatory material after the main table. The independently authored ordering/MCTS companion is separate and must be preserved.
- Keep commits/pushes under **ValentinHirschi <valentin.hirschi@gmail.com>**. The public destination is `alphal00p/symGCAD`, branch `codex/symgcad`. Do not assign BenRuijl as a PR reviewer. This handoff does not require creating a PR.
- Keep external source trees, literature, generated corpora and large results out of commits. Reproduce them with pinned setup commands and retain hashes in public reports.
- Perform the independent seeded, two-fresh-interior-points-per-cell sanity check starting with fixed A446, outside benchmark timings. It supplements rather than replaces exhaustive verification.
- Finish the specifically authorized final investigations, update/push the benchmark and this handoff, then stop. Further campaigns require a new instruction.

### What each milestone means now

| Planned milestone | Handoff status |
| --- | --- |
| M0: foundation | Saved plan, Rust library/CLI, Nix locks, provenance, importer and corpus inventory exist. |
| M1: faithful correct baseline | Exact generic projection/lifting, root export, verifier, regression tests and independent upstream bridge exist. |
| M2: runtime parity | Achieved on the frozen native-completing denominator: all 483 qualifying sign-expanded solves reproduced and independently verified; matched-phase geometric mean 0.146556677, below 1.25. |
| M3: certified reductions | Multiple implemented, independently checked reductions have measured benefits. This does not imply every proposed reduction was implemented or benefits every workload. |
| M4: bounded ordering search | UCT/MCTS, legal groups, worker deadlines, checkpoints and ablations exist. The main easy-case ablation shows no end-to-end advantage over cheap heuristics; a narrower local-promotion experiment is positive. |
| M5: hard topology | Fixed A446 and its crossing are solved. Their unrestricted symbolic domains are not; the whole milestone is therefore not complete. |
| M6: all cases | Incomplete: 303/331 verified at the audited checkpoint. |

## How to read the evidence without overstating it

There are several different counts and timing scopes in this repository. They must not be combined casually.

**Original groups.** The corpus is deduplicated by checked polynomial/domain/order equivalence while retaining source aliases. There are 331 distinct original groups. A full split problem is a different obligation from one of its sign branches. A conditional result with an extra inequality is a different domain. A fixed kinematic substitution is a different problem from its symbolic parent.

**Sign-expanded baseline cases.** The deterministic baseline study contains 579 sign-expanded solves across the groups. The native engine completed 483 under its frozen qualification protocol; symGCAD reproduced and verified all of them. The 96 native timeouts remain censored. Thus 483 successful baseline comparisons and 303 complete original groups are compatible counts, not contradictory totals.

**Regression tests.** The last complete tracked fast gate is **662 passed, zero failed, two deliberately expensive tests ignored**, across 51 test targets. This is a source-regression count, not a corpus-completion count. Later optional backend controls and research helpers have their own focused records. An unrelated alternate-mass A381 test from another workstream is excluded from this task's recorded count and benchmark denominator.

**Cold, warm and single-run measurements.** Fresh processes do not necessarily mean cold disk caches. Table tags disclose cold, shared/possibly warm, or unrecorded cache policy. A campaign may reuse completed exact operations from earlier failed attempts. Its time is valuable as a continuation measurement but is not the time to discover a result from scratch. Earlier order/anchor search is recorded separately unless it occurs inside the measured public solve.

**Compatibility versus proof.** Native GCAD is an independent comparison implementation, not an infallible mathematical oracle. Its raw-cell counts are useful compatibility evidence for matched orders. Its merging policy and assertions have defects or order dependence in some tests. The Rust result earns completion through its exact verifier and original-input audit, not solely by matching upstream counts.

**Subproblem success.** Factoring one stalled operand, replaying a resultant identity, finding a low-cost prefix, or counting roots on one rational fiber is not generic coverage of the original problem. Likewise, a cache with tens of thousands of records says that algebra was completed, not that that many cells or a corresponding fraction of the domain was solved. A last-operation capture identifies what was in flight; the whole campaign timeout is not automatically its isolated runtime.

**Certificates versus guides.** A guide can rank variables, propose a chart, predict a threshold or prioritize a worker. Only a verified exact certificate can discharge a projection or nonvanishing obligation. This distinction applies to CFF E-surfaces, Landau loci, Newton polytopes, tropical weights, graph cuts, specialized factors and floating-point intuition.

The top-level numerical record is [BENCHMARKS.md](BENCHMARKS.md). The table snapshot is [benchmark/data.json](benchmark/data.json), with [benchmark/followups.json](benchmark/followups.json) for audited incomplete continuation bindings. Individual reference notes retain experiment limitations and hashes. Generated proof/capture trees under `artifacts/` are not in Git; their absence from a fresh public clone is expected.

## Central correctness contract and non-negotiable invariants

For strict-input solution set `S`, a certified raw generic output `A` must satisfy `A ⊆ S` and `S \ A ⊆ Z(E)` for an explicit finite collection `E` of nonzero exceptional polynomials. Cell interiors must be disjoint. This certifies full-dimensional generic coverage, not lower-dimensional algebraic sections. Exceptional parameter specializations require a separate specialized solve.

The intended result contains canonical exact polynomials with explicit variable maps, the actual lifting order, rational interior samples, root-index boundary definitions, sample-specific isolating intervals, exceptional-set data and evidence for exhaustive lifting. Isolating intervals at a sample are not substitutes for algebraic boundary functions over the whole cell. Recomputing boundaries at a new prefix is essential.

The baseline eliminates from last to first using leading coefficients, discriminants and pairwise resultants, factoring/normalizing/deduplicating at each level, then lifts from first to last with exact root isolation. Strict constraints cannot be accepted merely because a selected rational sample works: every open interval has to be explored or rejected through the corresponding exact rule.

Every future optimization should be reviewed against the following failure modes:

1. **Changed input domain.** Clearing a rational denominator must preserve the sign. An additional chamber inequality, fixed scale or parameter substitution must be explicit, and a claimed original completion must audit the exact original constraints and groups.
2. **Missing interior zeros.** Nonnegative is weaker than strictly nonzero. Repeated roots, root collisions and even-multiplicity zero sheets can matter despite unchanged sign on either side. Aggressive merging is opt-in and must retain seam provenance.
3. **Wrong variable map or order.** Sparse polynomial coordinates, inactive variables and factor maps must survive conversion and preconditioning. A legal elimination heuristic may not violate a supplied fixed order or variable groups. Kinematic parameters remain outside a declared Feynman scale block.
4. **Probabilistic algebra accepted as exact.** Historical automatic-resultant measurements exposed a probabilistic CRT termination path. Production exact work uses deterministic dispatch and invalidates incompatible old cache evidence. Modular proposals must reconstruct under a sufficient exact bound or replay an exact identity.
5. **Specialization mistaken for a factorization.** A univariate or finite-field factor at one point can be useful evidence for a heuristic, but only lifted factors whose complete signed product reconstructs the original source establish the needed exact algebra.
6. **Incomplete guard sets.** A positive-root count, interval cover or separation certificate has coefficient, endpoint, leading-degree, sign, membership and stock/admission obligations. A budget refusal is a fallback trigger, never permission to drop a guard.
7. **Unsound transformation restoration.** Homogeneous scale fixing, monotone peeling and chart changes must cover the original positive domain and reconstruct its original cylindrical bounds. A convenient nontriangular graph chart can make algebra smaller while making this restoration difficult.
8. **Incomplete search promoted to a proof.** Intermediate scores, a timed-out worker's counters or a partially built proof tree cannot select a certified winning solve. Only a completed solve followed by the required verification qualifies.
9. **Cache scope leakage.** Reuse requires compatible canonical inputs, variable maps, domain assumptions, algorithm/version and transformations. Verify hash/tuple integrity, preserve finished entries and never inject unfinished profiler output as a completed operation.
10. **Misleading reporting.** Keep solve, verification, earlier discovery, process startup, preprocessing, export, RSS and timeout censoring distinct. Do not manufacture a ratio from a missing native denominator.

The sections below describe the concrete implementation, investigations and evidence under that contract.


## Architecture, exactness and validation

This section describes the tracked production implementation and audited original corpus at the 662-test integration checkpoint. Optional backend experiments, conditional kinematic subproblems and the separate alternate-mass A381 workstream do not change these counts.

### What the software implements

symGCAD is a Rust library and supervised CLI over exact Symbolica polynomials. The public modules are exported from [src/lib.rs](src/lib.rs); [src/main.rs](src/main.rs) implements `solve`, `project`, `order`, `verify`, `bench`, `corpus`, and `import-fastsecdec`. The hidden `_worker` command is a process boundary for operations that may block inside native algebra. Library callers of `solver::solve` do not automatically acquire the CLI's hard process deadline.

[config::Problem](src/config.rs) holds declared variables, strict constraints, sign-split expressions, exact string-valued parameter substitutions, variable groups, solver options and resource limits. It denies unknown TOML fields. [problem::normalize](src/problem.rs) binds an explicit complete variable permutation, substitutes exact parameter tokens, and turns strict inequalities into signed positive-polynomial conditions. In particular, rational content is cleared without reversing a sign, `false` and a strict zero constraint represent the empty set, and an identically zero *split annotation* is legitimate. These are different cases. Approximate kinematic values are not silently imported as exact rationals.

[algebra::Algebra](src/algebra.rs) retains canonical sparse polynomial objects and full variable maps, including inactive variables. Production coefficient arithmetic, differentiation, substitution, GCDs, factorization, resultants and root algebra use Symbolica. Exponent overflow and unsupported expressions are rejected. The full map is part of algebraic context: a sparse polynomial whose active support happens to match another operand cannot be transported between different orderings merely by copying a cache entry. Signed input normalization is also distinct from canonicalization for a zero set.

The faithful [projection implementation](src/projection.rs) factors and deduplicates first, eliminates variables from last to first using the required coefficients, discriminants and pairwise resultants, and factors each resulting level. Discriminants compose exact operations; direct low-degree identities are checked alternatives. The production general resultant path uses deterministic integer Ducos algebra, not an unchecked probabilistic CRT answer. [roots.rs](src/roots.rs) specializes fibers at rational prefixes, isolates real roots exactly, and resolves coincident roots by algebra and refinement. [solver.rs](src/solver.rs) lifts in the opposite direction through all required open intervals.

The baseline remains available. The optimized engine adds canonical operation and projection-prefix reuse and certified domain reductions. The experimental local engine in [src/local_projection.rs](src/local_projection.rs) constructs conditional projection obligations around sampled stacks, learns missing ancestor cuts and rebuilds affected work. Its positive-root rules, adjacent-pair rules and support registries are explicit proof-producing alternatives with fallback, not blanket permission to ignore negative or complex collisions.

Higher-level engines have separate evidence and replay paths: [homogeneity](src/homogeneity.rs), [monotone](src/monotone.rs), [sign filtering](src/sign_filter.rs), [linear-negative](src/linear_negative.rs), [bilinear-positive](src/bilinear_positive.rs), and [quadratic-ladder](src/quadratic_ladder.rs). They admit only their checked algebraic/domain structures and reconstruct bounds in the original variables. A successful lower-dimensional solve is not automatically a decomposition of the original problem; the corresponding transformation theorem and export reconstruction must also pass.

### Mathematical contract and evidence

For the original strict solution set S and the union A of exported raw cell interiors, completion means A is contained in S and S minus A is contained in the zero set of an explicit finite collection E of nonzero exceptional polynomials. Interiors are disjoint. This is generic coverage, not coverage of every exceptional parameter slice. To study such a slice, substitute the exact parameters and solve the specialized problem separately.

[output::SolveResult](src/output.rs) carries the original problem, actual order, normalized constraints and splits, polynomial table, projection levels, exceptional-set identifiers, cells, timings and typed evidence. Status is one of `complete_generic`, `incomplete`, `invalid_input`, or `computation_failure`. A completed process, a large count of visited nodes, a solved restricted chamber, or a partial cell export does not establish `complete_generic`.

Each cell contains rational samples and cylindrical root-index boundaries. `RootBound.index` is zero-based among **distinct** roots of the specialized boundary polynomial. `RootIndexDomain::Real` counts all real roots in ascending order; `Positive` counts strictly positive roots and excludes zero; `RealDescending` starts with the largest real root and includes zero. Sample-specific rational isolating intervals are witnesses at that sample, not globally valid numerical bounds. At a new coordinate prefix the algebraic boundary must be specialized and isolated again. A root selector by itself is not a delineability certificate.

Global lifting evidence is an exhaustive tree of `Stack`, `Rejected`, and `Accepted` nodes. An `Unfinished` node prevents completion. [verify::verify](src/verify.rs) re-normalizes the original problem, reconstructs projection and reduction obligations, checks roots and their order with exact arithmetic, and traverses every open interval. Rejection must have an exact constraint reason, and accepted cells must agree with the exported table and sign labels. It rejects missing cells, changed premises, invalid selectors, altered transformations and incomplete branches. Typed local and reduced-engine evidence has its own independent replay rather than being accepted on the producer's assertion.

“Independent verification” here means independent reconstruction/checking paths within Rust using the same trusted exact Symbolica backend. It is not a formal proof assistant or an independent computer algebra system. Upstream GCAD is compatibility evidence, not the ultimate correctness oracle; the disabled `failing_test_ex43` assertion, for example, disagrees with both actual native output and Rust projection sizes. See [BENCHMARKS.md](BENCHMARKS.md).

Default `merge="none"` preserves raw cells. The opt-in upstream-compatible aggressive merging has a separate output and retains excluded-seam provenance. The ordinary F=0 sheet must survive even where F has the same sign on both sides; a perfect square is not permission to erase its interior zero locus.

### Certified reductions and algebra-policy boundaries

[domain.rs](src/domain.rs) implements bounded exact certificates, including coefficient-sign arguments, suitable positive-coordinate shifts, Pólya/Bernstein methods, univariate Sturm reasoning, quadratic rules and optional weighted-square/PSD-component certificates. Projection-factor removal requires **strict nonvanishing** on a suitable superset of the relevant domain. Nonnegativity alone is insufficient. Original constraints and requested split boundaries remain protected.

The local `GuardDomain` currently derives safe prefix-domain information from the original constraints. It does not generally turn every newly observed projected sign condition into a reusable assumption. Doing so would require explicit chamber provenance, proof replay and corresponding cache-key changes. Physical thresholds, graph cuts, support geometry and tropical asymptotics can propose orders or certificates, but cannot independently authorize dropping an interior boundary.

`solver.affine_factorization` remains default-off. Its immutable policy is carried by `Algebra` through production factorization and independent checking. The degree-drop admission searches finitely for a regular evaluation point. Support-preserving nonzero diagonal scaling is tried first; admissible translation and bounded pairwise shear are further strategies. [affine_factorization.rs](src/algebra/affine_factorization.rs) and [shear_factorization.rs](src/algebra/shear_factorization.rs) check full maps, forward/inverse identities, signed products, units and multiplicities under finite budgets. There is one native factorization call for a selected transform; a blocked call does not receive a magical in-process timed fallback. Measurements include genuine wins and an approximately 1.42-times operand overhead case, so this is not a universal-speedup claim.

Optional Wang/small-leading-coefficient and main-axis scheduling patches live under [patches/experimental](patches/experimental). They are separate backend experiments with separate binaries, source pins and replay records. They are not silently installed by normal setup, not part of baseline parity, and not evidence that the original Gluza33 decomposition completed. Their operand successes must retain signed reconstruction and must not be confused with irreducibility inferred from a convenient sample.

### Corpus identity and unresolved originals

[PROVENANCE.toml](PROVENANCE.toml) pins upstream GCAD to `fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`, Symbolica 3.0.1 to `75f8350094b90254ee71dc2a391fde0d14b0204a`, and the literature and source-test hashes. The inventory preserves 177 collected tests, three disabled core examples, 265 literal two-loop fixtures and 12 generated one-loop fixtures. Rust [src/corpus.rs](src/corpus.rs) owns literal parsing, generation and exact equivalence grouping; Python is confined to the independent upstream build/adapter and reference extraction.

The 331 distinct mathematical groups consist of 124 fixed ZZ, 124 symbolic ZZ and 83 core problems. The last category includes 70 extracted solve calls and 13 distinct ordering-only inputs also treated as solve problems. Solving those inputs does not reproduce their ordering API assertions. Those are separately audited: 22 ordering calls plus one projection call, with 17 completed matches and six slow calls; utility tests have dispositions in [UPSTREAM_UTILITY_TESTS.md](reference/UPSTREAM_UTILITY_TESTS.md). Aliases, disabled/commented provenance and generated-input details are retained rather than discarded by filename heuristics.

Fixed ZZ means exact `mt2=1`, `mz2=5/18`, `s=10`, `t=-3` substitutions **before algebra**. Symbolic ZZ keeps the appropriate s and t, with s>4 and positive Feynman parameters. The completed ledger is **303/331**: fixed ZZ **124/124**, symbolic ZZ **97/124**, core **82/83**. Positive Gluza33 is the unresolved core problem at fixed kinematics, so the remaining problems are not all parametric.

The following list was checked directly against rows with `verified=false` in [benchmark/data.json](benchmark/data.json). The JSON's `scope="symbolic_core"` combines symbolic ZZ and core; the identifier and input are needed to distinguish them. `last_status` alone is not a completion classifier because later unsuccessful attempts can coexist with an earlier verified result.

| Unresolved original identifier | Kind |
|---|---|
| `finiteA_6_317_7_0__2_0_1_1_1_1_0_0_1/symbolic` | Symbolic ZZ |
| `finiteA_6_318_7_0__0_2_1_1_1_1_0_0_1/symbolic` | Symbolic ZZ |
| `finiteA_6_377_10_0__5_0_0_1_1_1_1_0_1/symbolic` | Symbolic ZZ |
| `finiteA_6_414_10_0__0_5_1_1_1_0_0_1_1/symbolic` | Symbolic ZZ |
| `finiteA_7_381_10_0__4_0_1_1_1_1_1_0_1/symbolic` | Symbolic ZZ |
| `finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1/symbolic` | Symbolic ZZ |
| `finiteAx12_6_317_7_0__2_0_1_1_1_1_0_0_1/symbolic` | Symbolic ZZ |
| `finiteAx12_6_318_7_0__0_2_1_1_1_1_0_0_1/symbolic` | Symbolic ZZ |
| `finiteAx12_6_377_10_0__5_0_0_1_1_1_1_0_1/symbolic` | Symbolic ZZ |
| `finiteAx12_6_414_10_0__0_5_1_1_1_0_0_1_1/symbolic` | Symbolic ZZ |
| `finiteAx12_7_381_10_0__4_0_1_1_1_1_1_0_1/symbolic` | Symbolic ZZ |
| `finiteAx12_7_446_11_0__0_5_1_1_1_1_0_1_1/symbolic` | Symbolic ZZ |
| `finiteB_7_471_13_0__7_1_1_0_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteBx12_7_471_13_0__7_1_1_0_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteC_6_215_11_0__6_1_1_0_1_0_1_1_0/symbolic` | Symbolic ZZ |
| `finiteC_6_231_7_0__2_1_1_0_0_1_1_1_0/symbolic` | Symbolic ZZ |
| `finiteC_6_470_10_0__0_5_1_0_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteC_6_476_10_0__0_0_5_1_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteC_7_247_10_0__4_1_1_0_1_1_1_1_0/symbolic` | Symbolic ZZ |
| `finiteC_7_478_10_0__0_4_1_1_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteCx12_6_215_11_0__6_1_1_0_1_0_1_1_0/symbolic` | Symbolic ZZ |
| `finiteCx12_6_231_7_0__2_1_1_0_0_1_1_1_0/symbolic` | Symbolic ZZ |
| `finiteCx12_6_476_10_0__0_0_5_1_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteCx12_7_247_10_0__4_1_1_0_1_1_1_1_0/symbolic` | Symbolic ZZ |
| `finiteCx12_7_478_10_0__0_4_1_1_1_0_1_1_1/symbolic` | Symbolic ZZ |
| `finiteD_7_251_12_0__6_1_0_1_1_1_1_1_0/symbolic` | Symbolic ZZ |
| `finiteD_7_375_13_0__7_1_1_0_1_1_1_0_1/symbolic` | Symbolic ZZ |
| `test_gluza_ex33/solve/3` | Positive Gluza33, fixed kinematics |

All 28 exhausted the first one-hour slow campaign without completion. The later two-hour/50-GiB original Gluza33 attempt likewise supplied no ground-truth decomposition. Subsequent targeted progress belongs to separate campaign records, not retroactive completion credit. [SLOW_SUITE.md](reference/SLOW_SUITE.md), [tests/corpus_classification.json](tests/corpus_classification.json) and [tests/slow_options.json](tests/slow_options.json) retain these cases and recipes.

### Representative completed original results

The following selected records come directly from the published benchmark snapshot. They illustrate what the project already solves, rather than forming a matched comparison between builds. N is the number of fresh process measurements; where N=3 the solve/check figures are medians. RAM is the maximum recorded resident memory across solve/check. Earlier discovery is separate: the positive Gluza30 base-order search alone took 300.026 seconds before the fixed-order controls below. The A446 single-run entries have an unrecorded cache policy and must not be presented as cold release medians.

| Original problem | Cells | Solve / check, seconds | RAM MiB | Measurement |
| --- | ---: | ---: | ---: | --- |
| Fixed A446, both F signs | 8 | 0.061978 / 0.042387 | 28.6 | Debug, N=1, cache unrecorded |
| Fixed crossed A446, both F signs | 8 | 0.142413 / 0.041581 | 28.5 | Debug, N=1, cache unrecorded |
| Fixed A381, both F signs | 836 | 0.185704 / 0.061275 | 16.2 | Release, N=3, cold |
| Fixed crossed A381, both F signs | 836 | 0.101736 / 0.061425 | 18.6 | Release, N=3, cold |
| Negative Gluza30 | 1 | 0.185212 / 0.020968 | 6.0 | Release, N=3, cold |
| Negative Gluza33 | 3 | 0.164014 / 0.020934 | 11.4 | Release, N=3, cold |
| Positive Gluza30 | 80,980 | 22.567567 / 16.854419 | 744.4 | Release, N=3, cold |

Fixed A446 initially completed with 35 unmerged cells using certified positivity and homogeneous reconstruction; the monotone rule reduced that to eight (seven positive and one negative). These are full original-variable decompositions. The later five-cell crossed-A446 result has the extra symbolic condition `t<4` and is deliberately absent from this original-result table. The corresponding full symbolic original remains unresolved. See [the complete benchmark snapshot](benchmark/data.json), [A446 measurements](BENCHMARKS.md), [conditional fixed controls](reference/CONDITIONAL_QUADRATIC_PEEL.md), and [positive Gluza30](reference/GLUZA30_BILINEAR_RELEASE_BENCHMARK.md).

### Benchmarks and test interpretation

The frozen baseline audit covers all 331 groups / 579 sign-expanded problems. Native GCAD completed **483** under qualification limits; Rust matched all 483 raw cell counts and independently verified every result. Three fresh paired measurements gave a matched algebra-phase geometric mean of **0.146556677 times native**, satisfying the 1.25 target. The 96 native timeouts remain explicit. This is not an end-to-end ratio for all 331 groups, nor a finite speedup assigned to a censored timeout. See [BENCHMARKS.md](BENCHMARKS.md) and [reference/README.md](reference/README.md).

New native comparisons use the user's adaptive ceiling `max(300 seconds, 3 * completed Rust median solve wall time)`, further bounded by configured/remaining campaign limits and the one-hour reference ceiling. Without a complete Rust measurement, the ceiling is 300 seconds. Reference qualification precedes Rust timing and freezes the cohort, so a later larger allowable measurement budget does not silently add formerly censored cases. Verification, truncated output and unsuccessful repetitions cannot supply the Rust median. [REFERENCE_BUDGETS.md](reference/REFERENCE_BUDGETS.md) documents this precisely.

The final integrated fast gate passed **662 tests across 51 targets**, zero failed, two expensive tests ignored, with 74.80 seconds summed test-body time. A fresh public-dependency build took 17 minutes 49 seconds separately. [COMBINED_FAST_VALIDATION.md](reference/COMBINED_FAST_VALIDATION.md) is append-only, so earlier paragraphs report earlier counts. The final gate used a clean isolated integration worktree with no reference checkout or unrelated alternate-mass test; the preceding 659-test gate used explicit tracked targets. `scripts/test.sh fast --locked` runs ordinary library/binary/integration tests; `scripts/test.sh slow` invokes the bounded Rust slow-suite supervisor and does not turn unresolved cases into passes. Research examples also have focused tests outside the tracked fast count.

Tests exercise exact rationals and signs, false/zero constraints, repeated/shared/nearby roots, disconnected regions, inactive variables, degree drops and nullification, root-selector mutations, missing branches, same-sign seams, transformations, serialization and worker termination/checkpointing. Small mathematical comparisons and proof mutations establish stronger evidence than random samples alone.

The requested supplemental A446 test is implemented separately in [examples/sanity_sample.rs](examples/sanity_sample.rs). Seed **20261010** generated two fresh rational interior points in each of eight fixed-kinematics cells, all distinct from stored CAD samples: **16 original-F evaluations and 112 original strict-constraint evaluations**, zero failures and zero skipped cells. Boundaries are recomputed sequentially at each fresh prefix; independent exact Sturm membership checks certify strict interiors. A separate sparse Symbolica evaluator evaluates the original TOML expressions and exact parameters, not reconstructed F or normalized cell labels. Counterexamples and partial failures are preserved. The approximately 0.41-second original check is outside solver timing and is supplemental, not a coverage proof. Reproduction and committed reports are in [SUPPLEMENTAL_SANITY.md](reference/SUPPLEMENTAL_SANITY.md).

### Ordering, caches, deadlines and checkpoint semantics

[ordering.rs](src/ordering.rs) retains upstream MODS, SOTD and T1 conventions and provides Brown/sparsity candidates. Groups and mandatory full orders constrain every candidate. [search.rs](src/search.rs) uses bounded UCT over legal elimination prefixes with seed 0, at most 256 rollouts, default search time `min(300 seconds, 20% of campaign)`, and 1/10/60-second promotion tiers. Degree/term/coefficient growth and local progress are predictions. Only a complete exact solve can select a completed winner; abandoning a search order never rejects a solution region.

The initial cold MCTS ablation comprised seven policies, six fixed cases and three repetitions: **all 126 solves verified**, but MCTS overhead showed no end-to-end benefit over the inexpensive portfolio. A narrower subsequent experiment with `search_local_progress` on the reduced positive Gluza30 base gave **three verified completions**, median **65.101 seconds** including a 60-second search, versus three incomplete controls at median **313.625 seconds**. That isolates promotion ranking on one reduced workload; it neither reverses the broad ablation nor proves an overall corpus benefit. See [LOCAL_SEARCH_PROMOTION_BENCHMARK.md](reference/LOCAL_SEARCH_PROMOTION_BENCHMARK.md).

[optimized_projection::ProjectionCache](src/optimized_projection.rs) shares exact factorizations, discriminants, resultants and completed projection prefixes; completed operations can survive a later interrupted stage. [search/checkpoint.rs](src/search/checkpoint.rs) journals bounded deterministic search state and validates context/version/order constraints on replay. A restored log's completion claim is not automatically a freshly verified solve. An unfinished native call contributes no completed operation. Source, backend, executable, input and proof hashes distinguish warm continuation from a new implementation.

Local completed-subtree caching stores the complete conditional proof and its required guards. Reuse must satisfy current obligations; equality of a rational prefix or prefix-plus-cuts is not equality of all solver state, because permanent witness stocks and support registries can change. Learned-cut and durable hint archives are proposals that must be checked, not persisted mathematical conclusions.

Important defaults and valid research controls are:

| Option | Default / limits / meaning |
|---|---|
| `cache_memory_mib` | 128 MiB algebra cache budget |
| `local_subtree_cache_mib` | 32 MiB completed-proof storage |
| `local_subtree_cache_entry_nodes` | 128; allowed 128–4096, admission only |
| `local_learned_cut_cache_mib` | 8 MiB learned hints |
| `local_stable_samples`, `local_hint_archive`, `local_deferred_projection` | Off; compatibility requirements checked |
| `local_descartes_witness_max_terms` | 4096 source terms; allowed 1–65536 |
| `local_descartes_interval_max_terms` | 4096 source terms; allowed 1–65536 |
| `local_descartes_interval_max_total_terms` | 65536 unique guard terms; allowed 1–262144 |
| `local_descartes_interval_template_limit` | Two permanent templates; allowed 2–8 |
| `local_base_univariate_basis`, `local_adjacent_pairs` | Off; explicit proof-producing alternatives |
| `local_max_refinements`, `local_max_nodes` | Optional work caps, separate from wall time |

Global saturated Descartes stock remains two templates per canonical source and axis. Raising a source-term cap does not raise that stock. Interval guard/source/stock budgets also do not relax root counts, endpoint/leading-coefficient guards, seam guards or verifier checks. Coalescing neighboring intervals may reduce one witness's guards while worsening reuse at another prefix. Completed-proof entry limits leave depth 16, serialized payload 1 MiB, total-node and total-byte bounds intact; allowing larger entries can increase evictions under the same total budget. This storage policy is stripped from nonsemantic hint-context fields, whereas mathematical witness options remain explicitly bound and checked.

[benchmark.rs](src/benchmark.rs) and [campaign.rs](src/campaign.rs) supervise killable process groups, collect wall/CPU/RSS outcomes and enforce deadlines/memory bounds around native calls. Defaults in `Limits` are one hour, 65536 MiB and at most 16 workers; validation permits an explicitly requested two-hour deadline. Each symbolic worker is single-threaded. Aggregate reservations, compile processes and backend licensing still matter. Fast local loop checks alone cannot bound a single blocking resultant or factorization. Progress files are snapshots: an active-operation capture proves an operand was entered, not how long that call ultimately ran.

### Reproducible setup and the graph boundary

[Cargo.toml](Cargo.toml) requires Rust at least 1.96; [flake.lock](flake.lock) fixes the Nix environment with Rust 1.98.1, native build tools, GMP/MPFR and profiling tools. The separate reference shell supplies Python/Cython/FLINT dependencies. Symbolica default features are disabled; production enables integer-GMP, float-MPFR and tracing, without Python/native-code-generation benchmark dependencies. Root Cargo patches resolve Numerica and Graphica from the same pinned Symbolica monorepo.

Normal setup is `nix develop`, then `cargo build --release --locked`. The final integration includes [PR #1](https://github.com/alphal00p/symGCAD/pull/1): Symbolica is a registry-facing dependency and all three standalone root patches select the same public, pinned Symbolica Git revision. A normal clone needs no `DO_NOT_PUSH_REFERENCES` directory. A downstream workspace owns its root patches and can share one compatible backend identity with symGCAD; arbitrary future backend compatibility is not promised. The optional bootstrap fetches public pinned research sources into ignored directories, checks revisions, and refuses unrelated modifications rather than resetting user work. `--reference` prepares native GCAD; `--fastsecdec` fetches the separate graph producer. `--apply-patches` installs the explicitly documented optional upstream linear-resultant-sign and primitive-integer-shift fixes idempotently in that research checkout; it does **not** redirect the default Cargo build to the patched sources. Select all three local root patches explicitly in an isolated worktree when testing a modified backend. Normal production already avoids those affected paths, and historical measured releases used pristine sources. License configuration belongs in the environment or ignored `.symgcad-env`, not in committed documentation. [docs/SETUP.md](docs/SETUP.md), [patches/README.md](patches/README.md) and [SETUP_VALIDATION.md](reference/SETUP_VALIDATION.md) give exact commands and tested scope.

The final integration also includes [PR #2](https://github.com/alphal00p/symGCAD/pull/2), allowing explicit library certificate limits of 262,144 Bernstein coefficients and 1,000,000,000 work units. Defaults remain 4,096 and 1,000,000. The chosen limits are serialized, validated and replayed; larger allowances do not establish a sign, remove a seam or replace caller-owned time/memory supervision. A nine-variable strictly positive polynomial actually exercises a `4^9`-coefficient tensor, with old-cap and one-work-unit refusals, signed discovery, independent/serialized replay and reduced-limit rejection. That regression validates the larger work option without claiming it consumes more than the old ten-million work ceiling. Checked tensor-cardinality addition fixes that specific overflow path; it is not a blanket claim of 32-bit overflow safety throughout the library. These two PRs were authored by ValentinHirschi in separate workstreams and integrated at the user's explicit request; they were not generated by the final Gluza campaign.

FastSecDec remains a separate repository on branch `no_deformation`, pinned to `99f96a5623eb5059f83c340833fafc2858498966`. It owns the dressed HEPKit DOT graph, routing, masses and external Gram products and exports native signed sparse U/F, exact point substitutions, edge/denominator-to-parameter maps, symbol aliases and provenance. symGCAD's [fastsecdec.rs](src/fastsecdec.rs) checks sparse reconstruction, signed scale, specialization, companion hashes, homogeneity, groups and maps. It explicitly reports `graph_derivation_verified=false` and `coverage_claimed=false`: it does not independently rederive U/F from DOT or certify a CAD merely by importing it.

The regular solver/verifier then proves generic coverage for the imported polynomial problem. Fixed and symbolic bubbles and fixed A446 passed actual export/import/solve/verify workflows; A446 yields eight cells with both F signs and also passes the supplemental 16-point check. These interface checks add no new original corpus problems. [docs/FASTSECDEC_INTEROP.md](docs/FASTSECDEC_INTEROP.md) documents optional native tests and their shared execution deadlines. Projective-simplex transport, sector maps, integration measures and the full no-deformation numerical pipeline remain downstream.

The benchmark JSON, Typst and PDF are committed; `benchmark/render.sh` renders the main report and separately preserved ordering/MCTS companion. Large generated corpora, proofs, dependency checkouts and literature remain outside Git. A fresh clone can rebuild inputs and regenerate evidence, but rendering the published snapshot does not require private historical artifacts. [PUBLICATION_HISTORY.md](reference/PUBLICATION_HISTORY.md) maps historical measured source identities to public commits.

### Correctness rules to retain during further work

Preserve all original parameter chambers and ordinary interior F=0 sheets. Never replace exact nullification, leading-coefficient, endpoint or root-order obligations with physical threshold intuition. Keep signed identities, multiplicities and full maps in every algebraic transform. Re-isolate boundaries at new prefixes and never identify roots solely by matching interval representations. Treat unsupported certificates, exhausted budgets and unexplored branches as fallback or incomplete, not a proof of absence. Distinguish canonical support from signed coefficients, and domain positivity from strict nonvanishing.

Cache proposals and saved result metadata are untrusted until their identities and obligations replay. A change in ordering, chart, domain or backend must be audited before reuse; a change only in storage admission does not authorize changing proof semantics. Do not infer completion from operation throughput, refinement counts, a hand-picked specialization, or a successful captured operand. Keep full-solve, operand-only, warm-cache, debug/instrumented and cold release measurements separate. These boundaries are what allow aggressive future optimizations to remain compatible with exact interior-boundary analysis.


## Gluza33 and exact algebra

Prepared from the retained reference notes, source-bound diagnostic reports and final control ledger. These algebra investigations must be read alongside the final original campaign audit and overall corpus ledger. The original **positive** Gluza33 problem remains unresolved in the last completed checkpoint. The original **negative** problem is solved and independently verified. The final positive campaign has now timed out without an exported decomposition or verification. Its terminal record and retained work are described below.

### Mathematical target and non-negotiable correctness contract

Gluza33 is an important exception to the statement that all remaining problems have parametric external kinematics: **its remaining positive problem has fixed kinematics**. The original positive fixture is `artifacts/corpus/symbolic/0338_test_gluza_ex33_solve_3.toml`, with eight strictly positive Feynman parameters. The negative fixture is `0336_test_gluza_ex33_order_1.toml`. These are distinct strict solution sets, so solving the negative branch gives no positive completion credit.

There is **no certified nontrivial lower bound on the positive decomposition's cell count** in the retained evidence. The exact original point `x0=15`, all other coordinates one, has `F=270>0`, proving a nonempty open positive region and the trivial lower bound of one cell; see [the bilinear seam counterexample](reference/GLUZA33_POSITIVE_BILINEAR.md). The final campaign's 879.838-second snapshot has 124,218 speculative nodes, 17,355 refinements and 32,766 retained subtree nodes, but none is a final cell count. Local refinement clears tentative sectors and restarts their count. Cached proofs are conditional on unresolved ancestor obligations and can contain overlapping nested subtrees. Export occurs only after the root has discharged all required guards, so those counters cannot be summed into a lower bound. The three verified negative cells likewise do not bound the positive count. Any future useful partial-count report must distinguish an order/engine-specific retained decomposition from a mathematical minimum over all decompositions.

More practically, this frozen build also cannot report **how many tentative cells the chosen ordering has reached so far**. `Built.cells` exists inside the process but is absent from `LocalReport`, progress files, algebra checkpoints and the durable hint archive; no partial cell tree was exported. This is a telemetry gap, not a reason to reject an informal progress count. Before another long campaign, expose cumulative accepted-leaf visits (including repeats), the largest completed conditional subtree, and ideally currently retained sibling-sector cells across the live recursion stack. The last counter must avoid nested double counting and subtract sectors discarded on refinement. Keep these separate from final exported cells, and do not restart a valuable active run merely to backfill an unavailable historical count.

Every investigation retains exact original interior `F=0` boundaries. A factor's even multiplicity does not make its zero set removable. An algebraic internal translation, scaling, shear or factorization ordering is permissible because the original polynomial is reconstructed exactly afterward; it does not authorize a change of CAD coordinates. A positive-ray root certificate is permissible only where the original domain proves that coordinate positive, with all coefficient, endpoint, degree, separator and pair-order obligations retained. A successful witness at a rational prefix is conditional: its guards still need certified signs over the parent cell. An isolated factorization or root inventory is never an original decomposition.

Operation-cache audits check immutable inherited bytes, mathematical tuple keys and output hashes. Such audits establish artifact integrity, **not** independent replay of every algebraic operation or complete CAD verification. Completed solver results require a separate verifier. Fresh experimental factors were never manually injected into the solver cache. Campaigns inherited only operations completed by prior original-problem runs.

### Graph identities: what is solved and what fails for the positive branch

[GLUZA33_RAYLEIGH_REDUCTION.md](reference/GLUZA33_RAYLEIGH_REDUCTION.md) records an exact graph reconstruction using a three-loop matrix, `U=det(M)`, `V=cU-j^T adj(M)j`, and `F=L U-10 V`. The paper's propagator order maps to `[x5,x0,x6,x1,x2,x3,x4,x7]`; `x3,x5` are massless. For either massless direction, Symbolica verifies

`F_i U-F U_i=-10 H_i^2`.

For the negative problem, two hereditary linear peels leave `A^2-8AB+B^2`, where `A=x0+x2+x7`, `B=x1+x4+x6`, both positive. The generic negative-fiber theorem, exact guide checks, ordinary base CAD and original-coordinate restoration complete the original negative Gluza33 problem in **three cells**. Three fresh release measurements give public solve median **0.164014 s**, verification median **0.020934 s**; the matched historical native controls all timed out at 60 s. These old censored controls do not supply a finite speed ratio, and their 60-second limit should not be confused with the later user policy for new native comparisons. See [GLUZA_RAYLEIGH_RELEASE_BENCHMARK.md](reference/GLUZA_RAYLEIGH_RELEASE_BENCHMARK.md) and its bound ledgers.

The positive branch requires a different argument. The simple strict bilinear rule that solved positive Gluza30 fails here: only `(x3,x5)` and its reverse are affine in both axes; `d=-48` at the all-one remaining point, and another proposed strict guide has a zero inside the positive orthant. Exact endpoint-guide identities still prove

`exists x3,x5>0: F>0  iff  d>0`.

This is a useful reduction, but the six-coordinate endpoint polynomial is mixed-sign. A generic two-fiber reconstruction would require a complete base decomposition for coefficient/root-order conditions. For `F=a xy+b x+c y+d`, the discriminant-like ordering quantity is `bc-ad=H^2`; the **H=0 seam remains essential**. A checked positive point has `(a,b,c,d)=(-110,200,-220,400)`, coincident fiber boundaries 20/11 and `F=270>0`. Deleting that same-sign seam would invalidate cylindrical cells.

The positive linear/bilinear base searches did not complete. Two 150-second ordering screens produced 49 and 53 timeouts without a completed trial; other fixed-order base trials also timed out. A rational compactification `w=z/(1+z)` simplifies an individual positive fiber, but inverse transport makes its upper endpoint switch between finite and infinity within a parent region. That is not an original-coordinate CAD cell without further splitting. These are exact structural insights, not completed positive solutions. Authoritative notes: [GLUZA33_POSITIVE_BILINEAR.md](reference/GLUZA33_POSITIVE_BILINEAR.md), [GLUZA_POSITIVE_LINEAR_BASES.md](reference/GLUZA_POSITIVE_LINEAR_BASES.md).

### Earlier expensive attempts and the first factorization breakthrough

A two-hour/50 GiB original positive attempt with scale anchor `x3` stopped at a degree-ten, 2,209-term discriminant with three surviving parameters and no new completed operations beyond its inherited cache. A distinct packed-resultant trial was **user-interrupted at 2,580.9 s**; it was neither a completed experiment nor a two-hour timeout. See [GLUZA33_AFFINE_ORDER_CONTINUATION.md](reference/GLUZA33_AFFINE_ORDER_CONTINUATION.md) and [THREE_PARAMETER_SEGMENTATION_RESEARCH.md](reference/THREE_PARAMETER_SEGMENTATION_RESEARCH.md). More RAM alone did not resolve the measured gate.

A legal `x0`-anchor order instead reached a 224-term factor source in active `[x2,x1,x7]`, degrees [9,9,6]. Exact square-free decomposition cheaply exposed `(1+x2+x7)` times a 158-term quintic block. Three native start strategies each timed out at 30 s; six variable-map permutations each timed out at 10 s. Map permutations do not necessarily alter the backend's internally chosen main variable, an important later lesson. The quintic's `x7` discriminant completed in 0.149522 s. Its constant rational gradient rank is three, excluding a lower-dimensional rational-linear reduction, but not useful invertible shears.

At all ones the quintic fiber loses degree; at suitable small integer points it retains degree. The generic internal translation `(1,2,3)` factors the original 224-term source in 1.632485 s, with a 1.698483-second process and full inverse signed reconstruction. A same-backend native control times out 30.023801 s. Restored factors have 3, 10 and 50 terms, all multiplicity one. A separate complete two-level projection of the captured block subset, retaining every content, LC, discriminant and pair resultant, takes 3.692678 s. That subset lacks the contemporaneous stack's companion cuts and hence does not certify that stack or the original problem.

This motivated the opt-in production `solver.affine_factorization=true` policy. Initial translation admission used operand features: at most 2,048 terms, bounded degree/height/maps, a finite shift menu and a strict reduction in exact leading-coefficient degree drops at the all-one fiber. Forward and inverse maps, signed units, multiplicities and complete source products are checked. Pre-native admission failure can decline; post-native reconstruction failure is an error, not a silent second unbounded native attempt. Worker processes bound indivisible algebra calls.

### Positive-root certificates: source size, guard size and mathematical applicability

After the affine improvement, an original 300-second continuation passed the old factor in 0.152 s and added 384 operations, then stopped at a 6,938-term degree 16 discriminant. At the directly recorded prefix `[1/2,2]`, exact Sturm counting/native isolation find two positive roots, and the 17 original coefficients have exactly two sign variations. A separator at 2 gives an exact saturated Descartes witness. All 17 coefficient guards plus the separator remain. Discovery and checked factorization of all 18 guards finish in 2.623493 s, 16,324 KiB. The source simply exceeded the old 4,096-term admission limit.

The generic production option `local_descartes_witness_max_terms` was introduced with default 4,096, explicit valid range 1..65,536, and the same immutable budget in discovery, template reuse and independent replay. Historical missing fields retain the default. No theorem, finite template quota or guard obligation was relaxed. A subsequent original run added 4,554 operations and reached a 19,942-term degree 24 polynomial with active degrees [41,33,24].

At prefix `[2,1/2]`, this larger polynomial has **two positive roots but four coefficient variations**. Raising the saturated-witness source allowance cannot fix an unsaturated root bound. The interval-Descartes theorem can: `(0,1)` and `(1,infinity)` each have one variation after exact rational transformation. Every transformed coefficient, the seam 1, zero endpoint and LC are retained. This gives 49 distinct guards, 49,870 total terms, largest 1,247 terms. Construction is 0.809820 s; construction plus all checked guard factorizations is 9.271372 s. The production `local_descartes_interval_max_terms` option similarly keeps default 4,096 and ceiling 65,536, while leaving per-guard and aggregate caps intact. The next original 600-second run adds 6,422 operations and reaches 16,537 cache records, then stalls on a different factor. Guard inventories and exact proof details are in [GLUZA33_WITNESS_BUDGET.md](reference/GLUZA33_WITNESS_BUDGET.md) and [GLUZA33_INTERVAL_BUDGET.md](reference/GLUZA33_INTERVAL_BUDGET.md).

These certificates explain why full-real discriminants can be excessive for a proven positive domain: negative-root collisions need not obstruct continuous simple positive sections. They **do not** erase positive roots or justify ignoring pairwise ordering with other cuts. Discovery merely proposes a proof; replay reconstructs the exact guards and the complete positive cover.

### The 2,410-term factor: profile, negative controls and optional Wang patch

The 16,537-record frontier is a 2,410-term source, degrees [20,18,8] in `[x2,x1,x7]`, full map retaining four inactive axes. Auto, univariate and bivariate native factor starts each fail within 60 s. Translation/scaling/shear admission provides no successful route. Exact SQF and univariate-fiber inventories are useful diagnostics but do not reconstruct the missing multivariate factors. A bounded cache-divisor search examines 4,096 shortest proposals: 499 fail exact fiber filters and 3,597 fail exact multivariate division. No divisor is accepted; the candidate inventory is explicitly truncated.

A 60-second perf recording gives concrete evidence: bivariate reconstruction and Hensel lifting together occur in 87.26% of samples, with no observed overlap between those two rows. Stack truncation prevents exact phase accounting and does not identify every recursive operand. A cheap exact LC factorization finds a 25-term leading coefficient whose factor images pass the necessary Wang pairwise gates at the generally generated point (11,13).

The optional `symbolica-bounded-small-lc-wang.patch` extends only admission and candidate selection of Symbolica's existing first-image Wang route. It admits bounded small nonmonomial LCs and tries at most 12 tuples from six consecutive prime pairs, filtering divisors of scalar content. It preserves degree, nonzero, pairwise-coprimality, squarefree, primitive-content, multiplicity, assignment and exact signed-product checks; failure retains native fallback. Nineteen selected backend tests pass, including parameter-only content, coincident specialization, bounded exhaustion and empty menus.

Matched enabled/disabled runs on the same binary show 1.039306 s factor time versus a 60.033840-second timeout. A separate export reconstructs two 106- and 821-term factors. This patch is **experimental and not in the ordinary dependency setup**. Its broader four-operand pilot found no additional completed speedup; some inputs have a parity obstruction to every integer specialization under the existing pairwise-image rule. Wang's ordered non-divisor method is a possible future generalization, but merely dropping coprimality would be unsound. See [GLUZA33_FACTOR_PROFILE.md](reference/GLUZA33_FACTOR_PROFILE.md), [GLUZA33_WANG_PREFLIGHT.md](reference/GLUZA33_WANG_PREFLIGHT.md), [WANG_ADOPTION_PILOT.md](reference/WANG_ADOPTION_PILOT.md).

The original 600-second continuation with this patch adds 202 operations (16,739 total) and returns to the 19,942-term discriminant. Clearing one algebra gate is not evidence that only one gate remained.

### Aggregate guards, preconditioner regression and compact covers

At `[2,2/3]`, the large polynomial still has two positive roots, but the old greedy cover requires four pieces and 97 guards/109,726 terms, exceeding the fixed 65,536 aggregate guard budget. A bounded research DAG over rational separator candidates finds three pieces and 73 guards/79,798 terms; that is smaller but still over budget. The finite search does not prove optimality. Under an explicit 131,072 budget all guards factor and signed-reconstruct: 6.536548 s for the greedy guard set, 5.092787 s for the compact set. Full helper times are 33.730334 and 26.328667 s, including reporting and search.

Production gained `local_descartes_interval_max_total_terms`, default 65,536, ceiling 262,144, immutable per stock and checked identically by independent replay. It does not change per-guard 4,096-term bounds, coefficient heights, root/interval/slot caps or template quotas. Generic tests genuinely exceed the default aggregate guard count while remaining below the override; lowered replay caps and altered signs fail. The ensuing original run adds 7,194 operations (23,933 total), then reaches a 1,283-term factor.

Here the translation heuristic **regresses**: raw native factorization completes in 0.042986 s, whereas an admitted translation stalls 30 s on an internal 330-term block. SQF-first alone does not fix it because the extracted 228-term block still admits the same bad translation. Exact diagonal scaling reaches the same regular evaluation point while preserving support and initial LC rankings: old 224-term, old 158-term and new 1,283-term controls factor in 0.013817, 0.005830 and 0.048233 s with full inverse signed reconstruction. The opt-in algebra policy was updated to try support-preserving scaling first. This is still a heuristic, not a theorem that transformed factoring is always faster. Read [AFFINE_COMPONENT_DISPATCH_RESEARCH.md](reference/AFFINE_COMPONENT_DISPATCH_RESEARCH.md) and [DIAGONAL_FACTORIZATION_RESEARCH.md](reference/DIAGONAL_FACTORIZATION_RESEARCH.md) before adjusting dispatch.

An intervening release continuation with affine policy disabled passes the 1,283-term factor but returns to the large discriminant at `[2,1/4]`, where there are now four positive roots. Greedy discovery gives 121 guards/139,654 terms, exceeding 131,072. A research compact cover finds four pieces with 97 guards/109,726 terms. Production then gained only a **small adjacent-coalescence pass**, not the expensive DAG: at most N−1 proposals; merge adjacent intervals only when retained counts sum≤1 and an exact union transform has that variation, with strict finite endpoints and conservative zero-coefficient refusal. The unchanged checker rebuilds every coefficient/endpoint/LC/retained seam. All 22 focused tests pass, including aggregate-budget and permanent-stock regressions.

Fresh production-module replay confirms the quarter-prefix cover `(0,1/2),(1/2,11),(11,19),(19,infinity)`: 97 guards/109,726 terms, construction 1.603566 s, replay 1.538465 s. At `[2,3/2]`, a three-piece proof needs 73 guards/79,798 terms; a global saturated proof needs only 27 guards/22,436 terms (all 25 original coefficients plus two separators), construction 0.059831 s and replay 0.025894 s. These fresh-stock measurements alone did not explain actual solver fallback. The complete measurement and production-admission chronology is in [GLUZA33_FINAL_INTERVAL_FRONTIER.md](reference/GLUZA33_FINAL_INTERVAL_FRONTIER.md).

### Causal stock tracing: the discriminant fallback is finally identified

After coalescence/scaling and larger source/aggregate allowances, original warm trials still stop at the large discriminant: 600.126 s adds 244 operations (24,317 total), then a 300.101-second campaign adds 89 factors (24,406 total). Aggregate counters and stale prefix snapshots cannot establish source-specific causality.

An artifact-only trace therefore binds exact source hash, full map, axis, rational prefix, route and every reuse/discovery/stock branch synchronously before the cached discriminant request. The hash filters **logging only**, never solver decisions. A first unoptimized 180-second trial is inconclusive; an opt-level 2 copy reaches the decisive event:

| Prefix | Global witness | Permanent interval stock |
|---|---|---|
| `[2,1/2]` | 2 roots/4 variations, fails | admits first two-piece template |
| `[2,2/3]` | 2 roots/4 variations, fails | first fails; admits second template |
| `[2,3/2]` | 3 roots/3 variations, succeeds | skipped after global success |
| `[2,1/4]` | 4 roots/8 variations, fails | both stored templates fail; 2/2 slots prohibit discovery |

Thus the quarter-prefix fallback is a confirmed **permanent template-stock gate**. The global stock is only 1/2 full and raising it would not repair the unsaturated bound. Existing interval limit 2→4 is the smallest justified configuration ablation; it retains a finite lifetime bound and unchanged verification. This admits the four-root cover as the third template and bypasses the discriminant. The 180-second trial adds 1,094 operations (25,500 total). The next 600.096-second worker adds 2,397 (27,897 total), 615.6 MiB peak RSS, no result/verification. Its prior pending factor completes in 0.075965 s: a deadline capture alone is not proof of a slow call.

Coalescence has a reuse tradeoff: a smaller cover at one prefix may fail where a finer one succeeds. The checked cubic example in [GLUZA33_WITNESS_STOCK_TRACE.md](reference/GLUZA33_WITNESS_STOCK_TRACE.md) demonstrates this, but does not prove coalescence caused Gluza's actual history. Do not introduce template eviction/reset without revisiting finiteness. No amount of unused RAM changes this mathematical/resource-policy distinction.

### Latest 2,867-term obstruction and validated optional main-axis remedy

The 27,897-record frontier captures a 2,867-term factor, degrees [24,18,11], 64-bit coefficients. Increasing the scaling source allowance to 4,096 would still decline: every active LC is already nonzero at all ones, and the finite menu improves no degree drop. A source-cap increase has no measured rationale.

A source-bound policy replay confirms native fallback. Exact signed SQF decomposition isolates a 1,731-term block, degrees [18,14,10], 56-bit coefficients. Its preferred-axis LC is

`9765625*x1^2*(5*x1+7)*(5*x1^2+73*x1+166)*(x1+x7)^2`.

The current pairwise-coprime image rule fails at **every integer x1**: when even, x1 and the quadratic are even; when odd, 5*x1+7 and the quadratic are even. This obstructs that sufficient sampling gate, not factorability. Forced univariate factoring also times out 60.039 s. More prime points or cap increases cannot fix this selected-axis gate.

A separate optional main-axis patch uses general shape admission (256..8,192 terms, exactly three active axes, degrees 3..32, bounded map/height) and two named policies. `lowest-degree` ranks degree, LC support, stable map index; `second-leading` chooses the second LC-support candidate. It rotates indices without changing polynomial coordinates/maps. Native fallback deliberately keeps the selected order; all native acceptance checks remain unchanged.

Same-backend controls: default-axis component times out 30.032942 s; second-leading times out 60.020913 s; lowest-degree completes in **0.280055 s**, reconstructing 36- and 814-term factors. The **entire original 2,867-term source**, through production preconditioning dispatch, completes in **1.951155 s**, signed product verified, factors [1,3,3,36,814] with multiplicities [1,1,2,1,1]. All 12 optional Wang candidates still reject after changing axis; native fallback succeeds. Attribute the result to axis selection, not successful Wang assignment.

Three pure policy tests, admitted generic synthetic controls with a negative rational unit/repeated parameter-only content/inactive map slot, and an independently verified two-cell solver smoke pass. These are single diagnostic timings with trace overhead/random native sampling, not benchmark medians. No factor was injected into a cache. See [GLUZA33_FINAL_FACTOR_PUSH.md](reference/GLUZA33_FINAL_FACTOR_PUSH.md) and `artifacts/research/gluza33-main-axis-01/REPORT.md`.

### Final original campaign and stopping result

The final original positive Gluza33 continuation **times out after 6,603.858274935 seconds**, with **2,010,632 KiB peak RSS** (1,963.508 MiB, about 1.92 GiB). Total campaign wall time is 6,604.300292846 seconds. Worker CPU time is 6,402.654628 user seconds plus 24.558223 system seconds. The configured solve allowance is 6,600 seconds; the reported wall time includes supervisor polling/termination overhead. There is no result file, no exported cell count and no independent verification run. The reserved verification allowance is unused. **The aggregate remains 303/331, with positive Gluza33 and 27 symbolic ZZ originals unresolved.**

The run uses one CPU (8), a 50-GiB allowance and the frozen optional lowest-degree-axis backend, with a shared 7,200-second solve/verification budget. It starts from a fresh copy of all 27,897 inherited operation records (305,478,455 bytes). The original problem, order `[x0,x2,x1,x7,x4,x6,x3,x5]`, scale anchor `x0`, affine policy, interval stock four, 32,768-term source allowances and 131,072 total guard terms are retained. Only the backend policy, checkpoint location and deadline differ from the preceding audited continuation. The previously obstructing 2,867-term factor completes inside the original solver in 1.996107155 seconds; standalone profiler factors were not injected.

The final directory retains **32,456 operation records, 4,559 new**: 4,013 factorizations, 40 discriminants and 506 resultants. The independent integrity audit passed at 2026-10-11 22:43:44 UTC, checking the original mathematical input/order/options, unchanged bytes of all inherited records, and canonical keys/output hashes of new records. This is artifact-integrity replay, not mathematical replay of every cached operation or verification of a CAD. Its SHA-256 is `50e02619cae6953de858c23bf8e0fb68578e4acf2400c36afe4dbe9109bece60`; the final cache manifest is `29e4f6a1bd7e25ff69d8641b30bd532155f5e393f8648ec0e25675c86c7402f7`. Operation counts cannot establish a covered fraction of the domain or a partial cell count.

The last complete progress snapshot is at 879.837742911 seconds and prefix `[2,6/13]`, with 124,218 speculative nodes and 17,355 refinements. It is stale relative to the terminal worker time: later active-operation captures and saved records demonstrate continued resultant work. The final captured call is a resultant in `x7` (index two in `[x2,x1,x7,x4,x6,x3,x5]`), key `c395fcc77e3ffcf7095dc236dbab6ce7a9e4763e33032358dd7d23ae835838e2`, input hash `8ab3ce7689cc50f11ed077a54f6db59dd943877199b9778a11b4f123e7478068`. The record does not give that individual call's elapsed time; the full campaign duration must not be assigned to it.

Evidence lives under `artifacts/research/gluza33-final-main-axis-campaign-01/`; the public summary and source bindings are in [GLUZA33_FINAL_FACTOR_PUSH.md](reference/GLUZA33_FINAL_FACTOR_PUSH.md). The terminal worker record SHA-256 is `1a7519f7dd970314063d90c2623b086720f82c72a05d807637d58e11f7a34e63`; final operand capture is `c93e651825a53dba90da8e51fdfbde4095a5985e7ef4fec9b08e182cfee1f72d`; campaign provenance is `3cc6a9c5a7ec48db202b928edf651ef10b6d9a76275e956af9e309ae289d65ba`. The original worker and supervisor have exited. No further campaign is started; the all-cases goal remains unfinished at this requested stopping point.

### Optional backend reproduction and evidence anchors

Pinned Symbolica is `75f8350094b90254ee71dc2a391fde0d14b0204a`. Use a separate checkout/target; do not mutate the production dependency. Apply `patches/experimental/symbolica-bounded-small-lc-wang.patch`, then `symbolica-bounded-main-axis.patch`. Enable `symbolica/research_small_lcoeff_wang`, and export `SYMBOLICA_RESEARCH_SMALL_LC_WANG=1`, `SYMBOLICA_RESEARCH_FACTOR_MAIN_AXIS=lowest-degree`. Unset `SYMBOLICA_RESEARCH_WANG_TRACE` for solver timings and all `SYMGCAD_WITNESS_TRACE_*` observation variables. The separate evaluator test-harness patch is only for upstream library tests without native-codegen, not normal execution. Read [GLUZA33_FINAL_FACTOR_PUSH.md](reference/GLUZA33_FINAL_FACTOR_PUSH.md) for exact setup commands.

The clean public patch chain reproduces compiled backend files byte-for-byte. Main-axis patch SHA-256 is `6441cf2ca55d8b4fc1220a5b6e9fa5f35b416fa68f3408a8792fd2025a9c3023`; Wang patch is `e50211910edfa513accfd7cec75718e1a464538bb5fbcd60a10d9898b9c36ce1`. Frozen final solver is `4d4af2f11744ca267723958944987d2e4d608e293074b24d9d7b669ec798198d`; harness `0a1790d3a2ac2f3344d3a04fa1f3eb87127e18fc1a239c6987847f314031ef88`. The artifact copied app retains older observer hooks, disabled; these are distinct from the backend patch. Control report SHA is `bd68eabe6d00f94bd47cbdd9ed013d8e011395cc097395f88de2cadfc4e01927`, ledger `38b2623323128109e322bf3a7bf0e461a62017d7b4a1c4417d8465094328af95`, full-original signed factor export `5eb140e8824afd67a1727a9db8eea83587fac9accf7fd08f0133ed12665d05ed`.

### Root-isolation work relevant to future effort

This work chiefly concerns authentic A377 projection blocks, not a completed Gluza improvement. [FAST_TAYLOR_SHIFT_RESEARCH.md](reference/FAST_TAYLOR_SHIFT_RESEARCH.md) and [POSITIVE_AXIS_ISOLATION_RESEARCH.md](reference/POSITIVE_AXIS_ISOLATION_RESEARCH.md) preserve exact comparisons and optional isolated patches. A factorial-convolution integer shift is algebraically correct with balanced signed encoding, complete decode/re-encode and exact divisions, but loses on the raw degree 1,478 block: 0.671435 s versus native 0.266245 s. Do not promote it based on asymptotic appeal. Captured native VAS shifts instead motivated a cheap exact unit-shift addition path, reducing one authentic shift by 39% with coefficient equality.

The combined isolated unit-shift/positive-root API gives a complete matching-scope comparison: all-real isolation 765.807915 s/832,592 KiB versus strictly-positive 557.982649 s/431,304 KiB. All 145 positive roots and multiplicities match; full patched 294-root inventory also matches the fresh old control. The old control's 1,138.55-second wrapper timing has wider scope, so its difference cannot be attributed entirely to one backend edit. No original CAD completion or corpus speed ratio follows. First-axis full-real proof contracts and domain rejection evidence must be adapted before substituting a smaller root query. Shared roots, zero roots, positive ordinals, endpoint ownership and degree drops remain exact obligations.

### What is reusable, what failed, and how far away completion is

Reusable production work includes exact opt-in algebra preconditioning with inverse signed replay; independently validated witness source/aggregate budgets; bounded adjacent interval coalescence; permanent template accounting; exact positive-ray proof obligations; and captured-operation/killable-worker infrastructure. Optional Wang and main-axis backend policies have concrete successes but remain experimental. The broad route is to improve generic algebra heuristics and certificates, never recognize a fixture.

Unsuccessful or inconclusive directions should not be silently repeated: unchanged larger-RAM discriminants; packed-resultant interruption; univariate/bivariate start switches on the 2,410 source; map permutations mistaken for axis control; 4,096 cached-divisor proposals; translation on the 1,283 source; SQF-first as a cure for that regression; saturated Descartes when variations exceed roots; global stock increases when the theorem fails; source-cap increases with zero degree-drop score; factor convolution on the measured shift; and finite compact-cover searches interpreted as global impossibility. Each failure has a different reason.

There is **no defensible estimate of how many gates remain**. Successful local algebra can expose another factor, resultant, certificate budget, template quota or refinement bottleneck. Cache growth measures completed reusable work, not covered volume or percentage of a CAD. The most useful next step, only if later authorized, is to inspect the final audited original frontier and its exact route before choosing another theorem or heuristic. Further general research could include bounded factor main-axis portfolios with measured low-cost predictors, exact Wang non-divisor assignment, cost/reuse-aware finite witness cover selection, and conditional graph/domain certificates. Every such change still needs full guards, termination arguments, independent replay and original-domain tests. The user requested stopping after the final attempt; retain these as handoff directions, not automatically scheduled work.


## Physics, graph structure, CFF and tropical research

The physics-motivated investigations have explicit mathematical limits. The committed reference notes are the durable account; the `artifacts/research/...` directories contain ignored, potentially machine-local inputs, binaries, intermediate polynomials, process records and hash manifests. Their absence from a fresh clone must not be mistaken for a missing implementation dependency. Reproduce the documented helpers and pinned inputs rather than committing those directories or reference checkouts.

At the audited symbolic-inventory snapshot there were **27 unresolved symbolic ZZ representatives**, each depending genuinely on both `s` and `t`, plus the separate fixed-kinematics positive Gluza33 problem. The final original Gluza33 campaign is tracked elsewhere in this handoff; none of the research results below alter its terminal outcome. [Unresolved kinematic scope](reference/UNRESOLVED_KINEMATIC_SCOPE.md) binds the inventory and explains why the current global quadratic-ladder premises cannot hold on all unrestricted symbolic inputs.

### The invariant contract: finite-kinematics cells, not integration equivalence

The intended downstream application is no-contour-deformation numerical integration, but this repository's output remains an exact generic decomposition of the specified original polynomial/domain. Preserve every ordinary interior `F=0` sheet, including same-sign double zeros, and all coefficient-drop, endpoint, root-order and nullification obligations needed for the declared cylindrical order. A physical Landau singularity requires additional stationarity/pinch conditions; an ordinary `F=0` sheet need not be a Landau singularity. Conversely, an intermediate cylindrical discriminant need not be a singularity of the integrated amplitude.

The symbolic ZZ problems have `mt2=1`, `mz2=5/18`, `s>4`, unrestricted real `t`, and positive Feynman parameters. The fixed workload specializes `s=10,t=-3` before algebra. The symbolic domain is **not** the physical scattering strip. Any physical/nonphysical partition must retain the complement. Threshold equality slices can be explicitly exceptional for generic coverage, but cannot silently become covered by neighboring cells or disappear when a slice is later specialized. Fixed orders and variable groups remain binding: kinematics lift outside the Feynman block.

Distinguish three types of evidence throughout these notes: a checked algebraic identity; a complete conditional decomposition with independent verification; and a completed original corpus problem. Only the last increases original completion counts. A transformed chart can have exact domain coverage without being cylindrical in the original coordinates. A partial projection basis, reduced term count, successful positivity leaf or large activity count is not a decomposition.

### Native graph workflow and the available metadata

[FastSecDec interoperability](docs/FASTSECDEC_INTEROP.md) is implemented, with Rust import in [src/fastsecdec.rs](src/fastsecdec.rs). The separate FastSecDec `no_deformation` branch exports pre-sector native U/F, dressed HEPKit DOT provenance, routing, exact substitutions, Gram data, roles and a stable edge/denominator/parameter map. `symgcad import-fastsecdec` checks sparse-polynomial versus expression agreement, symbolic-to-specialized substitutions and the companion problem. It does not perform sector decomposition, integration or implicit physical-domain selection. Its report deliberately distinguishes interchange validation from graph derivation.

This addresses a real provenance gap: the original two-loop fixtures generally contain sparse U/F arrays and variable labels, not edge connectivity. Graph names or coincident supports cannot reconstruct external momentum placement, masses or a unique labeled graph. The authenticated A446 export now provides an explicit graph binding, exact symbolic/fixed-point F agreement and an independent spanning-tree U reconstruction. Other fixtures still require their own bindings before graph certificates are trusted.

The inspected GammaLoop/FeynKit revision is `259df8790f27b8d3ef32778cd7195942691b4ef0`. `feynkit-cff` exposes bounded orientation generation and `EnergySurface` metadata containing original edge IDs, vertex sets and external shifts. Its default orientation bound is effectively unlimited, so future graph-side proposal generation must impose one. Numerical GammaLoop E-surface existence/tolerance decisions are not exact polynomial certificates; E and H surfaces must remain distinct. See [CFF interface audit](reference/CFF_KINEMATIC_PROJECTION.md). FastSecDec and the reference repositories remain separate clones, never vendored into symGCAD.

### What CFF and Landau analysis actually contributed

CFF expresses threshold denominators through sums of positive on-shell energies associated with connected cuts. A normal-threshold candidate is `Q_S^2-(sum m_e)^2`, with the correct incoming-energy orientation. This supplies channels, edge groupings, compatible cut families and promising chambers. It does not supply all positive-Feynman-parameter roots or every cylindrical collision. In particular, two cuts not appearing together in one cross-free family do not authorize deleting their parametric resultant.

The primary-source audit covers [Capatti's CFF construction](https://arxiv.org/html/2211.09653), [Landau Singularities Revisited](https://arxiv.org/html/2311.14669), [Whitney-stratified Landau analysis](https://arxiv.org/html/2402.14787), and [Recursive Landau Analysis](https://arxiv.org/html/2406.05241). Scaling strata, projective boundaries, massless limits and second-type singularities prevent a naive normal-threshold alphabet from being complete. Proper stratification of projection to kinematics also does not automatically provide root-indexed cylindrical transport.

On the authenticated A446 graph, 15 connected cuts give invariants `0`, `m=5/18`, `s`, and `u`; the only new normal-threshold candidate beyond the original `s>4` is `u=4`. Use `u=5/9-s-t` for A446 and `u=t` for its crossed fixture. Exact Gram identities and original signed U/F identities were replayed, not inferred from channel labels. This was a bounded connected-cut diagnostic, not generation of a full CFF sum, an energy-orientation proof, or a complete Landau stratification. [Connected-cut and endpoint evidence](reference/CFF_ENDPOINT_COMPOSITION_DIAGNOSTIC.md) records the graph mapping and hashes.

The useful exact reduction came from graph contraction. Write

```text
a=x1+x2+x3, b=x4+x5+x7, z=x8,
U=a*b+(a+b)*z,
F=(a+b)*U-s*x2*x4*z
  -u*(b*x1*x3+a*x5*x7+z*(x1+x5)*(x3+x7))
  -m*(b*x2*x3+a*x4*x7+z*(x2+x4)*(x3+x7)).
```

The original normalized split is exactly `18F`. At `z=0`, contraction creates two articulation blocks, giving `B=F|z=0=b*F1+a*F2`. On `u<=4`, the explicit square-plus-positive-residual decompositions of `F1,F2` prove `B>0`. This is a checked sum/product identity; deleting rather than contracting the rung would be the wrong graph operation.

The complementary obstruction is proved, not an unlucky search: **for every `u>4`, B takes both signs** on the original positive orthant, including the difficult `C0<0` portion where `C0=(X+Y)^2-uXY`, `X=x1+x5`, `Y=x3+x7`. Exact positive parameter families exhibit both signs. Further purely kinematic subdivision inside `u>4` cannot make B globally positive. Likewise, the [filtered-Jacobian study](reference/A446_FILTERED_JACOBIAN_RESEARCH.md) exhibits genuine positive critical points; do not repeat the disproved universal-Jacobian-nonvanishing search.

### Positivity work that did reach production

The research produced broad, opt-in exact rules rather than graph-ID dispatch:

* `weighted_square_positivity` discovers positive-monomial-weighted binomial squares plus a nonzero positive-coefficient residual. It proves the cubic B on `u<4`; the research six-square witness took approximately 0.00242 seconds to discover/replay initially and 0.00157 seconds for separate serialization replay.
* `quadratic_component_positivity` reconstructs connected quadratic subforms and uses exact rational Schur/LDL elimination, then independently checks the signed square decomposition and strict residual. It proves `C0` and the quadratic constant coefficient that weighted squares alone left inconclusive. Negative pivots, nonzero rows at zero pivots, pure squares and unsupported domains decline. No floating-point eigenvalues or SDP decisions enter acceptance.
* The generic `quadratic_ladder` engine uses extracted coefficient identities, six strict whole-domain premises and an independently verified base. It is not an A446-specific formula in the solver.

The kernels live in [src/domain/weighted_squares.rs](src/domain/weighted_squares.rs) and [src/domain/quadratic_components.rs](src/domain/quadratic_components.rs); [production component documentation](reference/QUADRATIC_COMPONENT_PRODUCTION.md) records resource bounds, typed evidence, malformed-input rejection and four-way ablations. The methods are complementary: PSD components alone do not prove the cubic endpoint. A strict residual matters because a pure square may vanish on an interior same-sign seam.

[Conditional ladder validation](reference/QUADRATIC_LADDER_VALIDATION.md) completes the entire crossed-A446 chamber `t<4` with five cells, signs `[+,+,+,-,+]`, preserving the original F boundary and explicit auxiliary exceptional seams. Solver time was 0.022253026 seconds in the recorded debug diagnostic; fresh verification passed. The independent supplemental sanity check used seed `20261010`, two fresh points in each cell, 10 F evaluations and 90 constraint checks, with zero failures/skips, outside timings. The unrestricted original remains unresolved because its complement and a correct threshold-section composition are missing. A five-cell conditional result must never replace its row in the original benchmark table.

### Rational critical maps and selected-root analysis: important missing interfaces

[Rational kinematic pair maps](reference/RATIONAL_KINEMATIC_PAIR_MAP.md) derive, for an exact affine structure,

```text
F=q*z+B,  q=Q-s*H-u*C,  B=M-u*T,  H,T>0,
U=M/T,  S=(Q*T-C*M)/(H*T).
```

Positive common roots satisfy `u=U,s=S`. A checked turning polynomial D is independent of s and equals `H*Delta` at common roots, where `Delta=q_y*B_x-q_x*B_y`. This distinguishes kinematic critical values from simply projecting an unconditional expanded resultant. The authentic identity/premise replay succeeds, but the subsequent `Res_x(B,D)` has **5,863 terms and degree 11 in y**. Signed factor replay returns a rational scalar and one nonconstant polynomial with unchanged support: no smaller factor was exposed. The replay is not an independent irreducibility proof. Increasing a helper export cap enabled inspection; it did not solve the critical classification.

The [selected positive-root theorem](reference/SELECTED_POSITIVE_ROOT_DELINEABILITY.md) gives a sound narrower target. Over a connected verified base cell, selected zeros form ordered continuous sheets if they are regular and the selected projection is proper; an alternative is regularity plus a globally constant finite selected count. Properness requires exact endpoint/infinity exclusion, not just sampled simplicity. `v*y-1` loses a root at infinity and `y-v` loses one at zero despite regular fibers.

For quadratic pairs, eliminating x gives a chart `x=-K/J` with selection predicates such as `y>0,C<0,J*K<0`. The identities binding the resultant, Jacobian and selector are exact, but `J=K=0` needs another chart or ordinary fallback. Positive roots of one resultant are not the same thing as positive common pairs: an unselected negative-x branch can collide at a positive y and alter the ordinary positive-y ordinal. A new predicate-bound root selector, whole-cell transport proof, equality-stratum handling and sanity-point reconstruction path are all still missing. Existing `RootIndexDomain::Positive` is insufficient.

Two certificate mechanisms remain research:

1. [The selected critical exclusion gate](reference/SELECTED_CRITICAL_CERTIFICATE_GATE.md) proposes bounded exact identities `P=A*q+E*B+L*Delta>0`. This proves the simultaneous critical system empty on the independently certified domain. It is sufficient, not a complete search; genuine critical chambers cannot pass globally. Current `strict_bezout` separates a pair of cuts and is not this system-selector interface.
2. [Weighted Hermite queries](reference/PARAMETRIC_PROJECTION_NEXT_RESEARCH.md) can count selected real roots through signatures of exact multiplication/trace matrices. An unrelated three-dimensional quotient example has `H_(x^2)=diag(1,2,2u)` and `H_x=diag(1,-2,-2u)`, whose signatures cancel the irrelevant negative-x event. **146 identities**, generically and again at `u=0`, were executed with Symbolica; a corrupted trace entry was rejected. Signature cancellation/properness are mathematical deductions from those identities, not a general implemented parametric Hermite verifier. The full remaining CAD prefix—including Feynman variables—is the parameter space; this is not automatically a two-kinematic-variable problem.

A generic gcd over a rational-function field cannot replace specialization-aware analysis: it may be one while specialized gcds become nonconstant on interior equality strata. Counting roots on neighboring open sectors does not certify continuity through the intervening section.

### Graph/worldline coordinates and complete ordered quadratic projection

[The CFF/worldline investigation](reference/CFF_PROJECTION_NEXT_INVESTIGATION.md) found two exact coordinate representations of both original A446/Ax12 signed polynomials. The broader motivation is the worldline skeleton period matrix: at fixed skeleton lengths and fixed ordering of degree-two-vertex insertions, Green-function position dependence is quadratic. This is a proposal source, not a theorem for arbitrary graph contraction or nonlinear sector substitutions.

The three-fraction chart is

```text
X=x1+x5, Y=x3+x7, R=x2+x4,
x1=X*alpha, x5=X*(1-alpha),
x3=Y*beta,  x7=Y*(1-beta),
x2=R*gamma,x4=R*(1-gamma).
```

It bijects the positive six-coordinate block with `X,Y,R>0` and the unit cube, has explicit inverse and absolute Jacobian `X*Y*R`. Complete F is jointly quadratic in the three fractions. Its 3×3 coefficient matrix K has exact compact identities

```text
T=X+Y+R, C=T^2-uXY-mRY,
D=det(K[1:2,1:2])=-u*C-(u-m)^2*R^2/4,
det(K)=(u-m)^2*T*C/4+s*z*D.
```

The fraction Hessian is `2*diag(X,Y,R)*K*diag(X,Y,R)`. Curvature changes depend on block lengths, not external thresholds alone. Exact same-kinematics examples give opposite signs of D. On the specified `u>4` chamber, monotonicity gives one possible positive R switch and, when `D>0`, one positive z inertia switch. These deductions guide conditional analysis; curvature does not determine the zero topology or root ordering.

The alternative worldline chart groups the two three-edge paths into lengths a,b and four ordered insertion positions `0<p<q<1`, `0<r<v<1`. It has Jacobian `a^2*b^2`, U independent of insertion positions, and F jointly quadratic in all four. Both original signed Fs, native U and Jacobian blocks were exactly replayed. Its domain is a product of ordered simplices, not a unit cube. Neither representation currently supplies original cylindrical restoration; future FastSecDec could consume verified mapped cells under a separate explicit contract.

[The face diagnostic](reference/QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md), implemented in [examples/profile_quadratic_faces.rs](examples/profile_quadratic_faces.rs), checks all 27 cube faces. For `Q=v^T H v+l^T v+c`, it constructs `d=det H`, `n=-adj(H)l`, `g=4dc-l^T adj(H)l`. A nonsingular candidate is `n/(2d)`, its value has sign **g*d**, and strict membership uses signed determinant products. Singular specializations remain obligations; identically singular faces also export rank/consistency minors. Discovery by derivatives and replay from sparse coefficients are separate.

Authentic face inventory: 51 unique raw guards/2,320 terms, about 0.446 seconds. Complete ordinary block projection: 71 factors/41,150 terms, 8.643 seconds, with 8.100 seconds in its last axis. Those outputs have different mathematical scope and their time ratio is not a speedup. The final six-variable prefix still needs CAD and lifting.

The decisive counterexample is `Q=a^2+a+b^2+t*b-1/4` on `0<a,b<1`, near `t=-1`. Every face stationary/value/membership guard keeps its sign, yet opposite b-endpoint roots in a exchange order at `t=-1`; their resultant is `(t+1)^2`. Therefore face extrema cannot replace ordered projection.

[Ordered quadratic flag identities](reference/QUADRATIC_FLAG_IDENTITIES_RESEARCH.md) instead reduce particular **existing** obligations. For jointly quadratic `Q=A*c^2+B(b)*c+C(b)`, A is independent of b, B has degree at most one in b, and C has degree at most two. With `P_e=Q(b,e)` and `K_e=B+2Ae`, the exact identity `Delta=K_e^2-4A*P_e` makes endpoint/discriminant resultants squares of smaller resultants, with the actual-degree leading-coefficient factors preserved. Opposite constant endpoints have linear differences; rational/affine endpoints require denominator guards and exact clearing. Repeated/shared factors and zero resultants need fallback. A smaller resultant must never be stored as the value of the original exact-resultant cache key.

Eighteen formula identities were replayed. The final standalone audit then reconstructed the authentic block, checked exact mapped equality with the complete 71-factor basis, and verified three actual final-axis replacements **728→87, 708→88 and 142→79 terms**, each with signed identity and complete factor-union replay. It timed out at 60.024 seconds/95,104 KiB before classifying the largest remaining pairs. Raw counts and the two largest hashes matched history; the full historical raw array was not available for equality comparison. Recorded resultant construction totaled 1.979 seconds; repeated whole-basis replacement audits were expensive, and the final factorization timer was not persisted. No dispatcher, full scan or solver speedup was established. Evidence: `artifacts/research/quadratic-final-axis-audit-01/`; report SHA-256 `ac9e36c506f7f5d45fc2527007a627868f48860f1ecfd8420829fde64f5b7fe1`.

### Two-point graph certificates and the Gluza matrix viewpoint

[The CFF note](reference/CFF_KINEMATIC_PROJECTION.md) derives a broad, unimplemented rational-flow certificate for connected two-point graphs. With `F=U*sum(m_e^2*x_e)-sV`, Thomson's principle bounds `V/U` by the energy of any checked rational unit flow. A cycle Gram matrix gives a polynomial square-completion identity. Exact incidence, rank, routing, U/F reconstruction, chamber slacks and a strict witness are required; graph metadata alone is insufficient. Rational lower mass capacities avoid irrational coefficients but may be conservative near threshold. The multi-leg Gram-majorant extension remains a proposal, not a generic Minkowski positivity theorem.

[The two-point matrix-pencil helper](reference/TWO_POINT_MATRIX_PENCIL.md) goes further algebraically: for fixed positive s, its checked symmetric pencil satisfies `det S=-s^ell F`, with a positive definite cycle Gram block. Consequently `F<0` corresponds to S positive definite, `F=0` to one-dimensional kernel, and `F>0` to exactly one negative eigenvalue. Massless edges give positive rank-one updates; massive edges generally do not.

Exact Gluza33 and unrelated massive/massless bubble replays passed in roughly 0.042 and 0.021 seconds. Eliminating Gluza's two massless directions reproduces the known base determinant `10*(10*A*B-(A+B)^2)`. This is a genuine graph-derived identity, but does not solve the positive-F endpoint switches, pinch seams or remaining base. Arbitrary projections of spectrahedra cannot be assumed to remain one simple matrix inequality. Do not confuse these identity diagnostics with the separate final factorization/campaign work.

### Borinsky tropical sampling, SubTropica and executed ordering experiments

[The initial tropical audit](reference/LITERATURE_TROPICAL_SAMPLING.md) inspected Borinsky's sampling, Minkowski extensions, graph/matroid valuations, toric charts, and pinned code in `momtrop`, `feyntrop` and tropical-feynman-quadrature. Subset recurrences can replace permutation enumeration for their sampling problem, but preprocessing is still generally exponential. A first-Symanzik matroid/support theorem does not identify externally labeled F or prove positivity. Complete nonvanishing is a hypothesis of useful tropical lower bounds, not a conclusion that can remove GCAD cuts. Generic Minkowski support results and conjectural exceptional-case equalities must remain distinguished.

[SubTropica](https://subtropi.ca/) is tropical subtraction and hyperlogarithmic integration, distinct from tropical Monte Carlo. The audited source revision is `cbb4d91fdf5c261e8687ba01dfb94d65498e8b73`. Its dynamic programming/Fubini reductions and expression-size order scores inspired a concrete **joint homogeneous-anchor and short exact linear-prefix portfolio**. Its intersection of reduction sets is not a CAD projection rule. Integration gauge admission, convergence, subtraction cancellation and one-way specialization do not replace per-constraint scaling, sign or coverage checks.

[The SubTropica review](reference/TROPICAL_SUBTROPICA_RESEARCH.md) and [linear-prefix experiments](reference/LINEAR_PREFIX_RESEARCH.md) document 24 two-stage candidates across B471, D375 and C470, completing in 3.945 seconds combined. Exact second-stage bases distinguish elimination pairs; tested anchors tie on best prefix costs. A later eight-candidate third-linear-stage probe for B471/D375 takes 2.638 seconds and produces an exactly equal 11-factor/112-term basis across tested permutations within each family. The two families' bases are distinct. Promoted full original solves still time out. C470's new gauge/order also remains incomplete. These are executed order proposals, not a demonstrated original-solve improvement.

One reduced polynomial involves all five remaining variables in each B471/D375 basis, making its variable-dependency graph K5. Thus a small original graph separator does not automatically provide sparse CAD after projection. The same bases offer no new coupled binomial premise beyond `s-4`. This is a recorded support inventory, not new algebraic simplification.

The basic caution has an exact two-variable example: `P_s=(x-y)^2+(4-s)xy`. At s=3,4,5 the support, coefficient-sign pattern and coefficient-free tropical polynomial coincide, while P changes from positive to an interior double seam to two positive ratio roots. Coefficient-zero chambers alone also miss the threshold at 4. Sum/fraction coordinates reveal the stationary value `1-s/4`, illustrating why exact quadratic analysis complements rather than follows automatically from tropical support.

### Sparse circuits, conditional certificates and root-gap screens

[Sparse AM–GM proposals](reference/TROPICAL_FOLLOWUP_RESEARCH.md) use bounded rational exponent relations, strict coefficient margins and explicit allocation of positive coefficients. [The executed circuit screen](reference/TROPICAL_CIRCUIT_SCREEN.md) inspected 21 distinct factor strings from an earlier Gluza witness. Only three fit the initial 128-term cap, all already coefficient-positive. All 18 mixed-sign factors have opposite-sign exposed Newton vertices in the metadata inventory. Increasing a global positivity cap is therefore not a promising first response. Abundant support relations do not check coefficient margins or prevent double spending. This screen was metadata, not a production Symbolica nonvanishing certificate; a global exposing ray may lie outside a restricted chamber.

[Conditional circuit research](reference/TROPICAL_CONDITIONAL_CERTIFICATES.md) derives a precise unimplemented extension. Independently proved positive monomial bounds `x^u_j>=L_j` can enter the integer exponent balance, and a strict polynomial AM–GM margin then certifies positivity on that domain. Laurent denominators need positive clearing. The worked family `x^3+y^3-k*z^3` under `xy>=q*z^2` has strict margin `4*q^3-k^2`; equality retains an interior zero. Current domain certificates authenticate coordinate-affine bounds, not arbitrary ancestor-cell coupled inequalities. Typed premise provenance, acyclic dependencies and domain-sensitive caching are missing. A maximum-coordinate atlas could supply bounded ratios, but ties, all charts and original cylindrical refinement remain obligations.

A separate tropical root-gap idea received an exact bounded screen, documented in the later SubTropica note. The same monomial must strictly dominate the sum of all others at both positive endpoints; convexity in log x then proves a root-free gap. This retains the full polynomial and uses exact integer/rational arithmetic, not tropical root counts. On a saved degree-1478 A377 polynomial, a completed 44-endpoint dyadic scan found only outer near-zero/large-radius gaps, **no useful internal gap**. The completed screen took about 0.102 seconds including parsing/controls; it did no new root isolation. The earlier tighter work budget left 40 endpoints unexplored and must not be called a full negative result. No band-isolation implementation followed, since existing VAS already skips outer empty regions.

### SOFIA, regions and transformation failures worth preserving

[The SOFIA/regions audit](reference/LITERATURE_REGIONS_SOFIA.md) corrects a naming ambiguity: SOFIA means *Singularities of Feynman Integrals Automatized*, by Correia, Giroux and Mizera, and computes candidate singularities/symbol alphabets; it is not an expansion-by-regions enumerator. Its finite `SolverBound`, unfinished physical-sheet/pinch reconstruction and potentially one-way `Symmetry` specializations make its output unsuitable as a GCAD coverage certificate. Its fetched sources also need pins for reproducibility.

The separate region literature—asy/asy2, pySecDec, dissection and hidden-region work—supports exact cancellation-resolving substitutions and boundary-scale proposals. It warns that Newton polytopes can miss interior pinches. Extended/regularized region integrals and scaleless cancellations do not partition the finite original sign domain. No SOFIA installation or native solver experiment was run here.

Six executed triangular sum preparations preserve all original constraints and inverse identities. Their complete F term counts change A377 **42→23**, A414 **45→41**, A446 **78→68** after both shifts. This is preparation evidence only. A positive block sum creates the wedge constraint `new_axis>sum(earlier_axes)`, not merely positivity. Full-real root ordinals survive an increasing triangular translation; positive-only ordinals may not, because zero moves relative to the cutoff. The export/transport wrapper remains absent.

The separate [triangular kinematic-shift experiment](reference/TRIANGULAR_KINEMATIC_SHIFT.md) reduces A446 from 78 to 68 terms but times out in both a 120-second debug trial and 600-second release promotion. The latter stalls on a degree-6/degree-8 resultant; alternative resultant strategies also time out. Fewer input terms are not a projection-complexity guarantee. Crossing transport similarly requires endpoint/orientation changes and original inverse replay; a matching A/Ax graph does not make its counterpart solved.

### What a successor should prioritize, and what not to repeat

1. Preserve the hard counterexamples: above-threshold B has both signs; authentic critical points exist; face extrema miss endpoint order; support does not encode threshold magnitudes; selected regular roots can escape; same-sign zero seams remain boundaries. These exclude entire classes of tempting shortcuts.
2. For graph workflows, bounded coordinate discovery followed by exact full U/F/domain replay remains the strongest structural direction. Complete ordered quadratic obligations and original-coordinate restoration are the missing work. The final-axis partial audit demonstrates real identities but does not yet show that they address the expensive remaining cross-event operations.
3. For kinematic chambers, first add a small typed, independently replayable premise interface. CFF predicates, graph flow slacks, conditional circuits and selected-root certificates all need that interface. Do not silently let a sample, target sign or later variable become a whole-cell premise.
4. Selected-system positive identities or small weighted-Hermite certificates are preferable initial experiments to building a full unconditional critical discriminant. Include equality strata, boundary/infinity exclusion and genuine-critical negative tests from the existing notes.
5. Retain the inexpensive order portfolio, but measure completed original solves including search cost. The already executed second/third-linear-prefix candidates must not be rediscovered and reported as new. C470 cache improvements are a separate measured controller issue; they do not validate a tropical/CFF theorem or establish coverage.
6. Any future experiment needs a bounded worker, exact input/source/backend hashes, a clear fallback and a success criterion stronger than smaller support. This handoff records possibilities for a later explicitly resumed effort, not authorization to launch additional work after the requested final campaign and publication.


## Other algebra, root and proof-maintenance bottlenecks

The following work is easy to miss when focusing only on A446 and Gluza33. It matters because an improved projection often moves the dominant cost to factoring, root isolation, retained proof size or repeated local reconstruction.

### Exact resultant acceleration and the A377 high-degree frontier

Production's conservative default is deterministic exact algebra. Two separate accelerators were investigated: bounded interpolation with sufficient exact reconstruction checks, and signed integer packing/binary segmentation. The latter substitutes a sufficiently large integer base under proved degree and coefficient bounds, decodes signed digits, reconstructs and checks the complete encoding, and restores all rational scale/sign data. It is not a heuristic agreement of a few modular images. The production option remains disabled by default, and proof verification can use the ordinary exact backend independently. See [binary segmentation](reference/BINARY_SEGMENTATION_RESULTANTS.md), its [production check](reference/BINARY_SEGMENTATION_PRODUCTION_CHECK.md), and [src/resultant_segmentation.rs](src/resultant_segmentation.rs).

Two authentic captured pair resultants were exactly reproduced through that production path: A317, 9,835 output terms, 38.16 seconds algebra; A318, 18,939 terms, 20.03 seconds algebra. Their independently completed numeric-Ducos grids contain 34,945 and 42,813 valid points. Builds differ, so those two times are not a head-to-head speed comparison. Both original symbolic decompositions remain unresolved. The static degree-product admission minimum was changed from 64 to 60, with a regression admitting a 10-by-6 pair and declining a 9-by-6 pair; the other bounds were unchanged.

The more expensive [A377 degree-32 discriminant](reference/A377_D32_GRID_VERIFICATION.md) has a separately completed global signed identity proof. Its original input is degree 32 in t with 2,502 terms; the candidate D(s) has degree 4,058 and 3,261 terms. Candidate computation took 1,538.878 seconds, and a fresh independent grid took 2,093.931 seconds. The complete grid checks **4,773 valid points**, from a proved bound on the unknown true resultant, not merely the candidate's smaller degree. Images with elimination-degree drops are skipped. The signed resultant is `lc_t(p)*D`, so cancellation of a nonzero polynomial proves the discriminant identity globally without removing the leading-coefficient-zero boundary.

This identity did not finish A377. [Content/basis measurements](reference/A377_D32_CONTENT_AND_BASIS_LIMITS.md) found that even the primitive positive-leading candidate has 33,667-bit maximum coefficients and 102,695,453 total coefficient bits, exceeding the first-axis prototype's 20,000/64,000,000 limits. Its approximately 31 MB canonical text fitting a different evidence cap does not override those coefficient limits. Native square-free decomposition actually returned in 293.533 seconds in a 300-second probe, but the complete product replay timed out. That is a failed checked export, not evidence that native SQF itself never returned. The exact phase record was corrected in the durable note; do not resurrect the earlier misdiagnosis or accept its unfinished factor list.

There is a general lesson here: certificate construction, mathematical replay and serialization can each dominate. Retaining a proposal before a replay deadline can make diagnosis easier, but it must remain untrusted and must not acquire a completed-cache type until full verification succeeds. Additional memory is not a substitute for identifying which stage exceeded its limit.

### Composite support bases and their sharply limited safe scope

A separate [matched root pipeline study](reference/A377_ROOT_ISOLATION.md) completed full root inventories from square-free support blocks and ordinary irreducible factors. The useful first-axis observation is elementary: after an exact signed source-product decomposition, the union of all real roots of the blocks is exactly the source's zero set. At the first lifting axis, there is no higher-dimensional base over which each block must already be delineable. Exact shared/repeated-root handling and every sector still have to be retained.

This motivated the implemented, default-off [first-axis support-basis prototype](reference/BASE_UNIVARIATE_BASIS_PROPOSAL.md), with dedicated typed evidence and independent regeneration of every raw guard, signed rational unit, multiplicity and variable map. Original constraints/splits retain their protected factorization. A composite support block cannot masquerade as an irreducible factor-cache record. Resource admission, retained source-plus-block work, products, bytes and lifetime stocks have separate bounds. See [resource policy](reference/BASE_UNIVARIATE_RESOURCE_POLICY.md), [proposal cache](reference/BASE_SUPPORT_PROPOSAL_CACHE.md), and [registry capacity](reference/BASE_SUPPORT_REGISTRY_CAPACITY.md).

The [higher-dimensional extension](reference/HIGHER_DIMENSIONAL_SUPPORT_BASES.md) remains a proposal with additional proof obligations. An exact product alone is not sufficient there. Every relevant content, leading coefficient, discriminant and distinct-pair resultant must be controlled over the actual connected parent cell. Identically zero discriminants or resultants require exact rebasing or fallback. Existing local finiteness and irreducibility arguments cannot simply be cited for arbitrary composite blocks. Lazard evaluation is a separate algorithm, not a name for ordinary specialization with missing guards.

### Local refinement, repeated proofs and progress-report overhead

Local projection can spend substantial time reconstructing successful children whose newly discovered obligations enlarge an ancestor's cuts. This can happen even when almost all algebra lookups hit caches. The [liveness note](reference/LOCAL_PROJECTION_LIVENESS.md) and [refinement profile](reference/LOCAL_REFINEMENT_PROFILE.md) separate mathematical closure from optional cache hints. Cut compaction, stable rational samples, learned-cut caches, archived hints and complete-subtree reuse each need their own validity and finite-lifetime contracts. A cached suggestion may avoid rediscovery; it is not itself a accepted child proof.

In C470, an [observer](reference/C470_REFINEMENT_OBSERVER.md) identified repeated children after individual completed-proof admission declined. Raising the per-entry node cap reduced observed revisits but increased evictions under the unchanged total budget. The production option `local_subtree_cache_entry_nodes` now permits 128 through 4,096, with default 128. Same-sign seams, depth, payload and total memory bounds remain enforced. Matched ordinary-release trials at the same 20,001-refinement limit took 8.398 and 6.958 seconds for 128 versus 1,024 nodes. These controls have a matched refinement-activity limit but different speculative work (100,422 versus 89,680 nodes); equal mathematical work or coverage was not established, and no full-solve speedup follows. The subsequent 600.084-second original attempt remained incomplete, retaining 1,295 new operations for a 61,671-record snapshot.

Earlier C470 attempts reached one million refinements with different scales/caches. A changed scale gave cheaper activity without completing the original. The [progress-I/O correction](reference/BASE_PROGRESS_THROTTLE.md) is a clearer implementation improvement: a previously unthrottled support-fallback report inflated system CPU. Correcting it reduced system CPU from 17.505 to 1.282 seconds in matched 60-second controls, preserving first/failure snapshots and the mathematics. Neither control completed the original. Inspect profiling evidence before treating unused RAM or high cache-hit counts as the explanation for every timeout.

### Exact symmetries can save future solves, but cannot invent donors

The [symmetry audit](reference/CORPUS_SYMMETRY_AUDIT.md) and later [crossing-transport design](reference/CROSSING_TRANSPORT_DESIGN.md) checked true polynomial/domain identities, including the ZZ crossing `t -> 5/9-s-t`. The later 303/331 audit found 12 unresolved symbolic pairs and no already verified donor for them. In principle, 15 suitably ordered symbolic representatives could cover the 27 unresolved symbolic entries after a verified transport implementation. The original denominator must remain 331.

Transport is not relabeling a result. It needs the full independently verified donor, exact bijection/inverse, transformed original constraints/splits/exceptional polynomials, endpoint reversal, sample reconstruction, root-index changes, and exhaustive exported-cell comparison. The t reflection is triangular only when s precedes t. A t-first cylinder can become noncylindrical in the target order. Fixed `s=10,t=-3` crosses to `t=-58/9`, so a fixed proof does not automatically certify its counterpart's original fixed point. Positive reciprocals similarly reverse ray endpoints and require the owning polynomial's exact positive-root count. The transport interface is still a design; no unresolved original is marked complete because its polynomial has a known symmetry.

### Known integer-shift hazard in wider reuse of a research helper

The handoff audit found one additional limitation in the frozen tropical root-gap helper, not a failure of the executed screen. Its `|e|<=128` dyadic admission is broader than the safe primitive-Integer shift range in the pinned pristine Numerica. A future exponent such as 63 or 127, or a sufficiently large shift of a small primitive coefficient, can take the known value-overflow path. Before reusing that helper on wider inputs, force `MultiPrecisionInteger` shifts or apply the separate [integer-left-shift fix](patches/numerica-integer-left-shift.patch); the bound is not itself a safe general API contract.

The PR #2 review also identified a separate pre-existing portability edge in `domain::affine_replace`: the expression `m[variable] as usize + 1` can overflow on a 32-bit target for exponent `u32::MAX`, before the enclosing saturating addition. The new checked Bernstein tensor count does not fix that other path. The published Nix targets and measured campaigns are 64-bit, so this is an outstanding extreme-input/32-bit portability concern, not an observed failure of the recorded corpus. Address it with a focused exact refusal test before claiming broader 32-bit support.

The measured 44-endpoint experiment is unaffected: it uses exponents -28 through 15, small toy shifts, and authentic coefficients already represented as multiprecision integers. The retained source is `artifacts/research/tropical-root-gap-screen-02/screen.rs` (admission line 12, shifts lines 35 and 63), SHA-256 `52f279f6cdc0e09d70ab1d68e221cceac5de4c57a37edbbd0eec0167a47ae465`; its report is `04749592aed2dc28b255698c549bae7d494619bc42769e14630e7387c936a8a1`. This qualification protects future reuse without relabeling the completed negative screen or expanding the production solver's correctness claims.


## Practical resumption, publication and stopping record

### Reproduce the ordinary solver before changing its mathematics

Start from [docs/SETUP.md](docs/SETUP.md) and [PROVENANCE.toml](PROVENANCE.toml). The root Cargo manifest fetches Symbolica, Numerica and Graphica from one pinned public Git revision, so ordinary builds do not require a local source bootstrap. For research, bootstrap fetches optional exact source checkouts and checks the existing tree without silently resetting unrelated edits. The committed Nix/Cargo locks provide the toolchain and native libraries. Optional experimental backend patches require explicitly selecting those local sources in an isolated build; applying a patch to an ignored checkout alone does not affect the public Cargo pins. Preserve the ordinary manifest and lockfile when reproducing the default benchmark.

```sh
nix develop
cargo build --release --locked
mkdir -p artifacts
target/release/symgcad solve examples/unit_square.toml --output artifacts/square.json
target/release/symgcad verify artifacts/square.json
scripts/test.sh fast --locked
```

The Symbolica license is supplied through the environment. It is not embedded in committed configuration or this handoff. The ignored local `.symgcad-env` can be sourced when available. Keep an immutable measured binary and source/dependency hashes for each experiment; a filename such as `release` is not enough provenance.

For graph input use `scripts/bootstrap-deps.sh --fastsecdec` and [docs/FASTSECDEC_INTEROP.md](docs/FASTSECDEC_INTEROP.md). For independent native comparison use `--reference` and the separate reference Nix shell. The Python/Cython bridge is the comparison boundary; the production engine and benchmark supervision are Rust. For slow originals inspect `scripts/test.sh slow --help`, the frozen classification and [reference/SLOW_SUITE.md](reference/SLOW_SUITE.md). Do not rebuild the parity denominator around whichever cases a new Rust release happens to finish.

### Resume a hard case from its audited evidence, not its name alone

If a later instruction authorizes further solving, first read the terminal campaign section and follow its exact input, binary, backend and cache bindings. Preserve the failed attempt unchanged. Copy a completed operation snapshot to a fresh directory, compare every inherited byte/hash, and audit the normalized mathematical input, actual order, groups, scale transformation and solver options before changing a single experimental variable. All temporary outputs need a new destination. Retain resource-limit failures and counterexamples.

The final optional factorization policy has isolated reconstruction successes, but its broader adoption still needs representative same-build controls and original solve/verify checks. For the original Gluza case, the next operation can be a resultant or root-lifting bottleneck even after all currently captured factors are easy. Prefer a profile or a source-bound route diagnostic over larger random menus, new caps or speculative rewrites. A proof-stock limit, an unsaturated Descartes theorem, a primitive coefficient overflow, an indivisible native call and repeated child obligations have different remedies.

A valid next result must include the full original domain or be explicitly labeled a subdomain. Protect `F=0`, leading-degree drops, positive endpoints, infinity escapes and crossing seams. For original completeness, require a complete exported result, fresh verification and the original-mathematical-identity audit. A process exiting zero can merely be a successful supervisor that recorded a worker timeout; inspect the worker status and verification fields.

### Highest-value future directions, with success gates

The handoff does not schedule these investigations. They are the most defensible directions if work is later resumed.

1. **Follow the final Gluza frontier.** The new main-axis result demonstrates that a structural backend heuristic can clear a real obstacle. A bounded generic axis portfolio, better lifting/assignment heuristics or smaller exact resultants may help the next one. Success means progressing a complete original solve and eventually verifying it, not a faster isolated factor alone.
2. **Typed whole-cell premises and selected-root proofs.** CFF chambers, graph-flow slacks, conditional AM–GM identities and positive-system exclusion all need authenticated inequalities on the full current base, with acyclic premise dependencies and domain-sensitive caches. Implement one narrow checker with negative tests before expanding discovery. A root selector needs a stable exact meaning through permitted specializations, not a sample-only index.
3. **Ordered quadratic blocks with original-coordinate restoration.** The graph-derived fraction/worldline charts expose real structure. The final-axis identities preserve existing obligations and are safer than deleting a projection alphabet. Measure the actual large remaining factor/resultant costs, then solve the export/recylindrification contract. A smaller chart basis without original cells is not the requested deliverable.
4. **Root isolation where positivity is proved.** The optional positive-axis/unit-shift experiments give a concrete API-time and memory improvement on an authentic large operand. Production integration still needs boundary-domain/index semantics, exact coincident-root behavior and complete verifier adaptation. Keep the full-real fallback and compare whole original solves.
5. **Local proof reuse guided by measured rejection reasons.** Larger complete-proof entries and better finite witness-cover selection can reduce rebuilding, but memory/stock changes can hurt reuse elsewhere. Profile fixed work, then confirm full solves. Do not remove finite-closure restrictions without replacing their termination argument.
6. **Transport solved symbolic representatives.** The checked crossing identities could avoid duplicate work once a suitable donor exists. Implement the independent transformation checker and legal-order obligations first. Current donor absence means this is a future multiplier, not an immediate completion.
7. **Use tropical/region information for proposals with exact acceptance.** Joint scale/order choice, sparse conditional circuits and graph-supported coordinates remain plausible. The executed negative screens and failed promoted orders delimit what has already been tried. Do not promote support geometry into a universal sign or singularity theorem.

Useful routes therefore remain, but there is no reliable estimate of remaining Gluza33 work or guaranteed breakthrough. The user requested stopping after this final bounded push and publication, rather than an indefinite sequence of campaigns.

### Publication and independent workstreams

The workspace is `/common/dev/gcad`, with working branch `codex/symgcad`. The public branch is also `codex/symgcad` at `git@github.com:alphal00p/symGCAD.git`. Local historical commits originally contained excluded reference material, so publication uses a **filtered copied history**, described in [reference/PUBLICATION_HISTORY.md](reference/PUBLICATION_HISTORY.md), with [reference/PUBLICATION_COMMIT_MAP.tsv](reference/PUBLICATION_COMMIT_MAP.tsv) mapping source IDs to public IDs. Do not raw-push the unfiltered local history. Ensure a non-forced ancestry-preserving update and verify every public author/committer is the requested identity.

The local ignored publication helper is `artifacts/prepare-clean-publication.sh`; it creates `codex/published` from committed trees using an alternate index, removing excluded reference/artifact/dependency paths. Before publishing, compare the final committed source tree with the copied public tree, inspect the entire public object inventory for excluded paths, fetch the remote and verify ancestry. Keep the map update in a separate provenance commit when needed. A future fresh public clone already has clean history and does not need to recreate the contaminated local history.

Preserve the independently authored `benchmark/ordering-mcts.typ` and PDF. They are separate from the main table's two-page notes. An unrelated alternate-mass A381 workstream left `reference/A381_MZ2_HALF.md`, `tests/a381_mz2_half.rs`, its fixture and `reports/` outside this task's staged changes. Their presence in a shared working tree does not authorize incorporating, reverting, deleting or counting them in this task's results.

Render the main benchmark with the locked Typst environment and inspect its final pages. The final main document must retain all 331 rows, truthful censoring/cache labels and no more than two explanatory pages. Do not re-render or edit the separate MCTS companion merely to refresh the main report. The public README remains concise; this handoff and the detailed notes carry the long research history.

At the requested stopping point, terminate no unrelated process, leave no task-owned campaign or build deliberately running, publish the final terminal evidence, and keep the all-cases goal **paused/unfinished**, not complete. This handoff is the saved starting point for a future explicit continuation.

## Reference index

This index preserves the full set of tracked research/validation notes available at the handoff, plus the final factorization note. Entries are evidence pointers, not a claim that every proposed algorithm was implemented. Many are chronological snapshots: their local completion counts and pending plans may be superseded by this handoff and the latest audit. Read their dates, measurement scopes and follow-up sections. No untracked alternate-mass A381 workstream note is included.

| Note | Repository path |
| --- | --- |
| [A377 captured discriminant: signed exact verification](reference/A377_BINARY_DISCRIMINANT_DIAGNOSTIC.md) | `reference/A377_BINARY_DISCRIMINANT_DIAGNOSTIC.md` |
| [A377 captured resultant follow-up](reference/A377_BINARY_RESULTANT_DIAGNOSTIC.md) | `reference/A377_BINARY_RESULTANT_DIAGNOSTIC.md` |
| [A377 triangular block-sum pilot](reference/A377_BLOCK_SHEAR_PILOT.md) | `reference/A377_BLOCK_SHEAR_PILOT.md` |
| [A377 degree-32 discriminant: content and support-basis limits](reference/A377_D32_CONTENT_AND_BASIS_LIMITS.md) | `reference/A377_D32_CONTENT_AND_BASIS_LIMITS.md` |
| [A377 degree-32 discriminant: complete signed identity verification](reference/A377_D32_GRID_VERIFICATION.md) | `reference/A377_D32_GRID_VERIFICATION.md` |
| [A377 D32: checkpointed per-block root isolation](reference/A377_D32_ROOT_INVENTORY.md) | `reference/A377_D32_ROOT_INVENTORY.md` |
| [A377 discriminant: boundary factors do not establish nonvanishing](reference/A377_DISCRIMINANT_BOUNDARY.md) | `reference/A377_DISCRIMINANT_BOUNDARY.md` |
| [A377 generic-resource continuation](reference/A377_GENERIC_RESOURCE_CONTINUATION.md) | `reference/A377_GENERIC_RESOURCE_CONTINUATION.md` |
| [A377 D32: bounded known-factor screening](reference/A377_KNOWN_FACTOR_SCREEN.md) | `reference/A377_KNOWN_FACTOR_SCREEN.md` |
| [A377: exact isolation of the square-free blocks](reference/A377_ROOT_ISOLATION.md) | `reference/A377_ROOT_ISOLATION.md` |
| [A377 D32: native square-free/GCD path and retained proposals](reference/A377_SQF_GCD_AUDIT.md) | `reference/A377_SQF_GCD_AUDIT.md` |
| [A377: square-free decomposition before irreducible factorization](reference/A377_UNIVARIATE_FACTORIZATION.md) | `reference/A377_UNIVARIATE_FACTORIZATION.md` |
| [Cached quadratic guards: fixed-work A381 controls](reference/A381_QUADRATIC_CACHE_PROFILE.md) | `reference/A381_QUADRATIC_CACHE_PROFILE.md` |
| [A381 root-union profile after the heap change](reference/A381_ROOT_UNION_PROFILE.md) | `reference/A381_ROOT_UNION_PROFILE.md` |
| [Fixed A381 traversal profile and stable sample experiment](reference/A381_STACK_SAMPLING.md) | `reference/A381_STACK_SAMPLING.md` |
| [Audited original A414 deferred-projection pilot](reference/A414_DEFERRED_PILOT.md) | `reference/A414_DEFERRED_PILOT.md` |
| [A414 fixed-work release comparison](reference/A414_FIXED_WORK_RELEASE.md) | `reference/A414_FIXED_WORK_RELEASE.md` |
| [A414 completed-subtree CPU profile](reference/A414_SUBTREE_CPU_PROFILE.md) | `reference/A414_SUBTREE_CPU_PROFILE.md` |
| [A446 algebra and traversal after sum-dependent factor preconditioning](reference/A446_ALGEBRA_FOLLOWUP.md) | `reference/A446_ALGEBRA_FOLLOWUP.md` |
| [Exact root geometry in the symbolic A446/Ax446 complement](reference/A446_COMPLEMENT_ROOT_GEOMETRY.md) | `reference/A446_COMPLEMENT_ROOT_GEOMETRY.md` |
| [A446 degree-eight positive-fiber diagnostic](reference/A446_DEGREE8_FIBER_DIAGNOSTIC.md) | `reference/A446_DEGREE8_FIBER_DIAGNOSTIC.md` |
| [Exact Jacobian research for the unresolved symbolic A446 complement](reference/A446_FILTERED_JACOBIAN_RESEARCH.md) | `reference/A446_FILTERED_JACOBIAN_RESEARCH.md` |
| [Audit of a proposed positive-common-root projection rule for A446](reference/A446_FILTERED_PROJECTION_AUDIT.md) | `reference/A446_FILTERED_PROJECTION_AUDIT.md` |
| [Fixed-parent ratio diagnostic for the A446 common-root arc](reference/A446_FIXED_PARENT_RATIO.md) | `reference/A446_FIXED_PARENT_RATIO.md` |
| [Exact kinematic partition and quadratic ladder for symbolic A446/Ax446](reference/A446_KINEMATIC_QUADRATIC_LADDER.md) | `reference/A446_KINEMATIC_QUADRATIC_LADDER.md` |
| [Acceptance obligations beyond mathematical completion](reference/ACCEPTANCE_AUDIT.md) | `reference/ACCEPTANCE_AUDIT.md` |
| [Affine preconditioning can hide cheap native structure](reference/AFFINE_COMPONENT_DISPATCH_RESEARCH.md) | `reference/AFFINE_COMPONENT_DISPATCH_RESEARCH.md` |
| [Exact affine preconditioning of the A377 degree-32 operand](reference/AFFINE_DISCRIMINANT_PRECONDITIONING.md) | `reference/AFFINE_DISCRIMINANT_PRECONDITIONING.md` |
| [Bounded affine factor preconditioning](reference/AFFINE_FACTORIZATION_POLICY.md) | `reference/AFFINE_FACTORIZATION_POLICY.md` |
| [Original Gluza33 continuation with affine factor preconditioning](reference/AFFINE_GLUZA33_CONTINUATION.md) | `reference/AFFINE_GLUZA33_CONTINUATION.md` |
| [Bounded affine prefix evaluation](reference/AFFINE_PREFIX_EVALUATION.md) | `reference/AFFINE_PREFIX_EVALUATION.md` |
| [Follow-up campaigns after exact square and sum reductions](reference/AFFINE_SUM_FOLLOWUP_CAMPAIGNS.md) | `reference/AFFINE_SUM_FOLLOWUP_CAMPAIGNS.md` |
| [Slow original ordering APIs](reference/API_SLOW_SUITE.md) | `reference/API_SLOW_SUITE.md` |
| [Bound progress I/O on optional base-support fallbacks](reference/BASE_PROGRESS_THROTTLE.md) | `reference/BASE_PROGRESS_THROTTLE.md` |
| [Restartable first-axis support proposals](reference/BASE_SUPPORT_PROPOSAL_CACHE.md) | `reference/BASE_SUPPORT_PROPOSAL_CACHE.md` |
| [Bounded first-axis support registry capacity](reference/BASE_SUPPORT_REGISTRY_CAPACITY.md) | `reference/BASE_SUPPORT_REGISTRY_CAPACITY.md` |
| [Exact support blocks at the first lifting axis](reference/BASE_UNIVARIATE_BASIS_PROPOSAL.md) | `reference/BASE_UNIVARIATE_BASIS_PROPOSAL.md` |
| [Differentiated support-source limits](reference/BASE_UNIVARIATE_RESOURCE_POLICY.md) | `reference/BASE_UNIVARIATE_RESOURCE_POLICY.md` |
| [First-axis support-basis retention diagnostics](reference/BASE_UNIVARIATE_RETENTION_DIAGNOSTICS.md) | `reference/BASE_UNIVARIATE_RETENTION_DIAGNOSTICS.md` |
| [Exact checks of the bounded production resultant kernel](reference/BINARY_SEGMENTATION_PRODUCTION_CHECK.md) | `reference/BINARY_SEGMENTATION_PRODUCTION_CHECK.md` |
| [Exact binary segmentation of captured resultants](reference/BINARY_SEGMENTATION_RESULTANTS.md) | `reference/BINARY_SEGMENTATION_RESULTANTS.md` |
| [C470: exact observation of local refinement and cache admission](reference/C470_REFINEMENT_OBSERVER.md) | `reference/C470_REFINEMENT_OBSERVER.md` |
| [Exact centering as a discriminant reduction](reference/CENTERED_DISCRIMINANT_RESEARCH.md) | `reference/CENTERED_DISCRIMINANT_RESEARCH.md` |
| [Connected cuts and compositional endpoint positivity](reference/CFF_ENDPOINT_COMPOSITION_DIAGNOSTIC.md) | `reference/CFF_ENDPOINT_COMPOSITION_DIAGNOSTIC.md` |
| [CFF-informed kinematic projection and exact graph certificates](reference/CFF_KINEMATIC_PROJECTION.md) | `reference/CFF_KINEMATIC_PROJECTION.md` |
| [CFF-guided quadratic blocks for parametric projection](reference/CFF_PROJECTION_NEXT_INVESTIGATION.md) | `reference/CFF_PROJECTION_NEXT_INVESTIGATION.md` |
| [Combined fast-suite validation](reference/COMBINED_FAST_VALIDATION.md) | `reference/COMBINED_FAST_VALIDATION.md` |
| [Conditional quadratic endpoint peeling](reference/CONDITIONAL_QUADRATIC_PEEL.md) | `reference/CONDITIONAL_QUADRATIC_PEEL.md` |
| [Unresolved-corpus symmetry audit](reference/CORPUS_SYMMETRY_AUDIT.md) | `reference/CORPUS_SYMMETRY_AUDIT.md` |
| [Crossed A318: measured operand improvement and warm continuation](reference/CROSSED_DISCRIMINANT_CONTINUATION.md) | `reference/CROSSED_DISCRIMINANT_CONTINUATION.md` |
| [Exact crossing transport: implementation proposal](reference/CROSSING_TRANSPORT_DESIGN.md) | `reference/CROSSING_TRANSPORT_DESIGN.md` |
| [Cross-order algebra cache: proposal and bounded Ax318 audit](reference/CROSS_ORDER_ALGEBRA_CACHE.md) | `reference/CROSS_ORDER_ALGEBRA_CACHE.md` |
| [Saturated Descartes witnesses: proof and captured A446 cost](reference/DESCARTES_SEPARATOR_PROFILE.md) | `reference/DESCARTES_SEPARATOR_PROFILE.md` |
| [Bounded source admission for exact positive-root witnesses](reference/DESCARTES_SOURCE_BUDGET_PRODUCTION.md) | `reference/DESCARTES_SOURCE_BUDGET_PRODUCTION.md` |
| [Deterministic CRT feasibility and a one-prime diagnostic](reference/DETERMINISTIC_CRT_PRIME_IMAGE.md) | `reference/DETERMINISTIC_CRT_PRIME_IMAGE.md` |
| [Support-preserving diagonal factor preconditioning](reference/DIAGONAL_FACTORIZATION_RESEARCH.md) | `reference/DIAGONAL_FACTORIZATION_RESEARCH.md` |
| [Exact factorial-convolution Taylor shifts (research)](reference/FAST_TAYLOR_SHIFT_RESEARCH.md) | `reference/FAST_TAYLOR_SHIFT_RESEARCH.md` |
| [Final implementation checkpoint](reference/FINAL_CHECKPOINT.md) | `reference/FINAL_CHECKPOINT.md` |
| [Exact full-real root covers from untrusted hints](reference/FULL_REAL_ROOT_COVER.md) | `reference/FULL_REAL_ROOT_COVER.md` |
| [Generic gates for support bases and affine discriminant preconditioning](reference/GENERIC_SUPPORT_AND_AFFINE_GATES.md) | `reference/GENERIC_SUPPORT_AND_AFFINE_GATES.md` |
| [Positive Gluza30 release controls](reference/GLUZA30_BILINEAR_RELEASE_BENCHMARK.md) | `reference/GLUZA30_BILINEAR_RELEASE_BENCHMARK.md` |
| [Determinantal Rayleigh identities for positive Gluza30](reference/GLUZA30_DETERMINANT_RAYLEIGH.md) | `reference/GLUZA30_DETERMINANT_RAYLEIGH.md` |
| [A two-fiber positive ladder for Gluza 30](reference/GLUZA30_POSITIVE_LADDER.md) | `reference/GLUZA30_POSITIVE_LADDER.md` |
| [Exact Gluza30 negative-domain reduction (research)](reference/GLUZA30_RAYLEIGH_REDUCTION.md) | `reference/GLUZA30_RAYLEIGH_REDUCTION.md` |
| [Gluza30 local-refinement audit](reference/GLUZA30_REFINEMENT_AUDIT.md) | `reference/GLUZA30_REFINEMENT_AUDIT.md` |
| [Gluza30 positive release traversal diagnosis](reference/GLUZA30_RELEASE_TRAVERSAL.md) | `reference/GLUZA30_RELEASE_TRAVERSAL.md` |
| [Gluza33: the next factor has regular evaluation fibers](reference/GLUZA33_AFFINE_BUDGET.md) | `reference/GLUZA33_AFFINE_BUDGET.md` |
| [Original positive Gluza33: an affine-last ordering trial](reference/GLUZA33_AFFINE_ORDER_CONTINUATION.md) | `reference/GLUZA33_AFFINE_ORDER_CONTINUATION.md` |
| [Gluza33: factorization profile and leading-coefficient preflight](reference/GLUZA33_FACTOR_PROFILE.md) | `reference/GLUZA33_FACTOR_PROFILE.md` |
| [Final Gluza33 factorization investigation](reference/GLUZA33_FINAL_FACTOR_PUSH.md) | `reference/GLUZA33_FINAL_FACTOR_PUSH.md` |
| [Gluza33: the next interval-witness resource gate](reference/GLUZA33_FINAL_INTERVAL_FRONTIER.md) | `reference/GLUZA33_FINAL_INTERVAL_FRONTIER.md` |
| [Gluza33: bounded interval-Descartes guard measurement](reference/GLUZA33_INTERVAL_BUDGET.md) | `reference/GLUZA33_INTERVAL_BUDGET.md` |
| [Positive Gluza33: the next algebra gate](reference/GLUZA33_NEXT_INVESTIGATION.md) | `reference/GLUZA33_NEXT_INVESTIGATION.md` |
| [Positive Gluza33: strict-premise obstruction and a generic two-fiber proposal](reference/GLUZA33_POSITIVE_BILINEAR.md) | `reference/GLUZA33_POSITIVE_BILINEAR.md` |
| [Exact two-step Gluza33 reduction (research)](reference/GLUZA33_RAYLEIGH_REDUCTION.md) | `reference/GLUZA33_RAYLEIGH_REDUCTION.md` |
| [Bounded Wang reconstruction resolves the captured Gluza33 factor](reference/GLUZA33_WANG_PREFLIGHT.md) | `reference/GLUZA33_WANG_PREFLIGHT.md` |
| [Gluza33: the saturated-witness source budget](reference/GLUZA33_WITNESS_BUDGET.md) | `reference/GLUZA33_WITNESS_BUDGET.md` |
| [Gluza33: the source-specific cause of the discriminant fallback](reference/GLUZA33_WITNESS_STOCK_TRACE.md) | `reference/GLUZA33_WITNESS_STOCK_TRACE.md` |
| [Gluza retries and identity-substitution cost](reference/GLUZA_CERTIFICATE_PROFILE.md) | `reference/GLUZA_CERTIFICATE_PROFILE.md` |
| [Positive Gluza domains: exact one-fiber reduction and bounded screen](reference/GLUZA_POSITIVE_LINEAR_BASES.md) | `reference/GLUZA_POSITIVE_LINEAR_BASES.md` |
| [Negative Gluza release controls](reference/GLUZA_RAYLEIGH_RELEASE_BENCHMARK.md) | `reference/GLUZA_RAYLEIGH_RELEASE_BENCHMARK.md` |
| [Square-free support bases above the first axis](reference/HIGHER_DIMENSIONAL_SUPPORT_BASES.md) | `reference/HIGHER_DIMENSIONAL_SUPPORT_BASES.md` |
| [Indexed compatibility merging](reference/INDEXED_COMPATIBILITY_MERGE.md) | `reference/INDEXED_COMPATIBILITY_MERGE.md` |
| [Exact integer evaluation of rational prefixes](reference/INTEGER_PREFIX_PROFILE.md) | `reference/INTEGER_PREFIX_PROFILE.md` |
| [Exact integer sign evaluation](reference/INTEGER_SIGN_PROFILE.md) | `reference/INTEGER_SIGN_PROFILE.md` |
| [Interval Descartes campaign outcomes](reference/INTERVAL_DESCARTES_CAMPAIGNS.md) | `reference/INTERVAL_DESCARTES_CAMPAIGNS.md` |
| [Rational-interval Descartes diagnostic](reference/INTERVAL_DESCARTES_DIAGNOSTIC.md) | `reference/INTERVAL_DESCARTES_DIAGNOSTIC.md` |
| [Bounded source admission for interval root certificates](reference/INTERVAL_SOURCE_BUDGET_PRODUCTION.md) | `reference/INTERVAL_SOURCE_BUDGET_PRODUCTION.md` |
| [A377: an exact discriminant-of-resultant reduction](reference/ITERATED_RESULTANT_RESEARCH.md) | `reference/ITERATED_RESULTANT_RESEARCH.md` |
| [Joint homogeneous anchors and exact short projection prefixes](reference/LINEAR_PREFIX_RESEARCH.md) | `reference/LINEAR_PREFIX_RESEARCH.md` |
| [Generic CAD literature: priorities for the unresolved corpus](reference/LITERATURE_GENERIC_GCAD.md) | `reference/LITERATURE_GENERIC_GCAD.md` |
| [Graph structure, SOFIA and region analysis for GCAD](reference/LITERATURE_REGIONS_SOFIA.md) | `reference/LITERATURE_REGIONS_SOFIA.md` |
| [Tropical sampling and graph structure: a plan for exact CAD](reference/LITERATURE_TROPICAL_SAMPLING.md) | `reference/LITERATURE_TROPICAL_SAMPLING.md` |
| [Local cut archive restart experiment](reference/LOCAL_HINT_ARCHIVE.md) | `reference/LOCAL_HINT_ARCHIVE.md` |
| [Bounded observation of repeated local guard failures](reference/LOCAL_OBSTRUCTION_OBSERVER.md) | `reference/LOCAL_OBSTRUCTION_OBSERVER.md` |
| [Deferred projection and live-cut compaction](reference/LOCAL_PROJECTION_LIVENESS.md) | `reference/LOCAL_PROJECTION_LIVENESS.md` |
| [Local projection follow-up pilots](reference/LOCAL_PROJECTION_PILOT_OUTCOMES.md) | `reference/LOCAL_PROJECTION_PILOT_OUTCOMES.md` |
| [B471 local refinement and exact sample reuse](reference/LOCAL_REFINEMENT_PROFILE.md) | `reference/LOCAL_REFINEMENT_PROFILE.md` |
| [Local ordering promotion: paired Gluza30 base experiment](reference/LOCAL_SEARCH_PROMOTION_BENCHMARK.md) | `reference/LOCAL_SEARCH_PROMOTION_BENCHMARK.md` |
| [Symbolic massless core completion](reference/MASSLESS_SYMBOLIC_COMPLETION.md) | `reference/MASSLESS_SYMBOLIC_COMPLETION.md` |
| [Bounded monomial-square positivity screen](reference/MONOMIAL_SQUARE_SCREEN.md) | `reference/MONOMIAL_SQUARE_SCREEN.md` |
| [Exact peel inspection without projection](reference/MONOTONE_INSPECTION.md) | `reference/MONOTONE_INSPECTION.md` |
| [Bounded multivariate square-free product diagnostic](reference/MULTIVARIATE_SQF_DIAGNOSTIC.md) | `reference/MULTIVARIATE_SQF_DIAGNOSTIC.md` |
| [Direct certification of a negative monotone inequality](reference/NEGATIVE_MONOTONE_TARGET.md) | `reference/NEGATIVE_MONOTONE_TARGET.md` |
| [Positive-root pair separators: proof and A446 operand profile](reference/PAIR_SEPARATOR_PROFILE.md) | `reference/PAIR_SEPARATOR_PROFILE.md` |
| [A next step for threshold-informed parametric projection](reference/PARAMETRIC_PROJECTION_NEXT_RESEARCH.md) | `reference/PARAMETRIC_PROJECTION_NEXT_RESEARCH.md` |
| [Exact physical elliptic coefficient certificate](reference/PHYSICAL_ELLIPTIC_POSITIVITY.md) | `reference/PHYSICAL_ELLIPTIC_POSITIVITY.md` |
| [Physical symbolic endpoint projection audit](reference/PHYSICAL_ENDPOINT_FACTORS.md) | `reference/PHYSICAL_ENDPOINT_FACTORS.md` |
| [Restricting root isolation to a certified positive axis](reference/POSITIVE_AXIS_ISOLATION_RESEARCH.md) | `reference/POSITIVE_AXIS_ISOLATION_RESEARCH.md` |
| [Proposed positive-endpoint quadratic-ladder wrapper](reference/POSITIVE_ENDPOINT_QUADRATIC_LADDER_PLAN.md) | `reference/POSITIVE_ENDPOINT_QUADRATIC_LADDER_PLAN.md` |
| [Positive pair roots above a conditional quadratic endpoint](reference/POSITIVE_PAIR_ROOT_STRATIFICATION.md) | `reference/POSITIVE_PAIR_ROOT_STRATIFICATION.md` |
| [Positive-root remainder signs: existing support and a narrow extension](reference/POSITIVE_ROOT_REMAINDER_INTERFACE.md) | `reference/POSITIVE_ROOT_REMAINDER_INTERFACE.md` |
| [Prefix evaluation: negative power-cache experiment](reference/PREFIX_EVALUATION_PROFILE.md) | `reference/PREFIX_EVALUATION_PROFILE.md` |
| [Direct prepared-problem worker diagnostics](reference/PREPARED_WORKER_DIAGNOSTICS.md) | `reference/PREPARED_WORKER_DIAGNOSTICS.md` |
| [Avoiding persistence for exact primitive-linear identities](reference/PRIMITIVE_LINEAR_CACHE.md) | `reference/PRIMITIVE_LINEAR_CACHE.md` |
| [Integer fibers for exact root refinement](reference/PRIMITIVE_ROOT_FIBERS.md) | `reference/PRIMITIVE_ROOT_FIBERS.md` |
| [Published source history](reference/PUBLICATION_HISTORY.md) | `reference/PUBLICATION_HISTORY.md` |
| [Bounded quadratic-face projection experiment](reference/QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md) | `reference/QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md` |
| [Exact quadratic components with a strict residual](reference/QUADRATIC_COMPONENT_CERTIFICATE.md) | `reference/QUADRATIC_COMPONENT_CERTIFICATE.md` |
| [Exact PSD quadratic-component certificates](reference/QUADRATIC_COMPONENT_PRODUCTION.md) | `reference/QUADRATIC_COMPONENT_PRODUCTION.md` |
| [Proposed next bounded certificate: PSD quadratic components](reference/QUADRATIC_COMPONENT_PRODUCTION_PLAN.md) | `reference/QUADRATIC_COMPONENT_PRODUCTION_PLAN.md` |
| [Quadratic identities for actual cylindrical projection obligations](reference/QUADRATIC_FLAG_IDENTITIES_RESEARCH.md) | `reference/QUADRATIC_FLAG_IDENTITIES_RESEARCH.md` |
| [Exact quadratic opening: conditional validation](reference/QUADRATIC_LADDER_VALIDATION.md) | `reference/QUADRATIC_LADDER_VALIDATION.md` |
| [A conditional quadratic-pair lift](reference/QUADRATIC_PAIR_LIFT_PLAN.md) | `reference/QUADRATIC_PAIR_LIFT_PLAN.md` |
| [Exact generalizations of the quadratic positivity rules](reference/QUADRATIC_POSITIVITY_GENERALIZATIONS.md) | `reference/QUADRATIC_POSITIVITY_GENERALIZATIONS.md` |
| [Quadratic certificates and ladder: release measurements](reference/QUADRATIC_RELEASE_ABLATION.md) | `reference/QUADRATIC_RELEASE_ABLATION.md` |
| [A rational kinematic map for positive pair roots](reference/RATIONAL_KINEMATIC_PAIR_MAP.md) | `reference/RATIONAL_KINEMATIC_PAIR_MAP.md` |
| [Native upstream comparison](reference/README.md) | `reference/README.md` |
| [Bounded recursive quadratic positivity screen](reference/RECURSIVE_QUADRATIC_SCREEN.md) | `reference/RECURSIVE_QUADRATIC_SCREEN.md` |
| [Adaptive native-GCAD comparison budgets](reference/REFERENCE_BUDGETS.md) | `reference/REFERENCE_BUDGETS.md` |
| [Finite moving-scale Descartes sections](reference/SCALED_DESCARTES_SECTIONS.md) | `reference/SCALED_DESCARTES_SECTIONS.md` |
| [Durable bounded ordering search](reference/SEARCH_CHECKPOINTS.md) | `reference/SEARCH_CHECKPOINTS.md` |
| [A small exact gate for selected critical-point exclusion](reference/SELECTED_CRITICAL_CERTIFICATE_GATE.md) | `reference/SELECTED_CRITICAL_CERTIFICATE_GATE.md` |
| [Selected positive common roots without a full resultant discriminant](reference/SELECTED_POSITIVE_ROOT_DELINEABILITY.md) | `reference/SELECTED_POSITIVE_ROOT_DELINEABILITY.md` |
| [Fresh public checkout validation](reference/SETUP_VALIDATION.md) | `reference/SETUP_VALIDATION.md` |
| [Exact shear preconditioning for factorization](reference/SHEAR_FACTORIZATION.md) | `reference/SHEAR_FACTORIZATION.md` |
| [Slow original aggressive-merge assertions](reference/SLOW_MERGE_ASSERTIONS.md) | `reference/SLOW_MERGE_ASSERTIONS.md` |
| [One-hour original-problem queue](reference/SLOW_SUITE.md) | `reference/SLOW_SUITE.md` |
| [Strict Bézout certificates for positive pair projection](reference/STRICT_BEZOUT_PROJECTION.md) | `reference/STRICT_BEZOUT_PROJECTION.md` |
| [Exact sum reduction for a stalled A446 factorization](reference/SUM_FACTOR_PRECONDITIONING.md) | `reference/SUM_FACTOR_PRECONDITIONING.md` |
| [Supplemental exact sampling](reference/SUPPLEMENTAL_SANITY.md) | `reference/SUPPLEMENTAL_SANITY.md` |
| [Bounded support-informed order proposals](reference/SUPPORT_ORDER_SEEDS.md) | `reference/SUPPORT_ORDER_SEEDS.md` |
| [Bounded symbolic ZZ screen after exact sum preconditioning](reference/SYMBOLIC_AFFINE_SUM_SCREEN.md) | `reference/SYMBOLIC_AFFINE_SUM_SCREEN.md` |
| [Symbolic A381/Ax381: a larger peelable piece and an exact obstruction](reference/SYMBOLIC_CONDITIONAL_PARTITION.md) | `reference/SYMBOLIC_CONDITIONAL_PARTITION.md` |
| [Captured symbolic A317/A318 discriminants](reference/SYMBOLIC_DISCRIMINANT_PROFILES.md) | `reference/SYMBOLIC_DISCRIMINANT_PROFILES.md` |
| [Symbolic Ax12/A446 on the negative-t subdomain](reference/SYMBOLIC_HALF_DOMAIN.md) | `reference/SYMBOLIC_HALF_DOMAIN.md` |
| [Bounded three-parameter binary-segmentation research](reference/THREE_PARAMETER_SEGMENTATION_RESEARCH.md) | `reference/THREE_PARAMETER_SEGMENTATION_RESEARCH.md` |
| [Constant lower-ray certificates: proposal and A377 evidence](reference/TRANSLATED_LOWER_RAY_PROPOSAL.md) | `reference/TRANSLATED_LOWER_RAY_PROPOSAL.md` |
| [Exact triangular kinematic shift experiment](reference/TRIANGULAR_KINEMATIC_SHIFT.md) | `reference/TRIANGULAR_KINEMATIC_SHIFT.md` |
| [Sparse AM–GM support screen on captured Gluza33 guards](reference/TROPICAL_CIRCUIT_SCREEN.md) | `reference/TROPICAL_CIRCUIT_SCREEN.md` |
| [Tropical follow-up: conditional circuits on proved monomial chambers](reference/TROPICAL_CONDITIONAL_CERTIFICATES.md) | `reference/TROPICAL_CONDITIONAL_CERTIFICATES.md` |
| [Tropical follow-up: sparse certificates for signed graph polynomials](reference/TROPICAL_FOLLOWUP_RESEARCH.md) | `reference/TROPICAL_FOLLOWUP_RESEARCH.md` |
| [SubTropica and tropical structure for exact GCAD](reference/TROPICAL_SUBTROPICA_RESEARCH.md) | `reference/TROPICAL_SUBTROPICA_RESEARCH.md` |
| [Two-coordinate positive homogeneous chart: export and verification audit](reference/TWO_COORDINATE_CHART_AUDIT.md) | `reference/TWO_COORDINATE_CHART_AUDIT.md` |
| [Two-point convexity: conservative local sign certificates](reference/TWO_POINT_CONVEXITY.md) | `reference/TWO_POINT_CONVEXITY.md` |
| [A matrix-pencil route for two-point domains](reference/TWO_POINT_MATRIX_PENCIL.md) | `reference/TWO_POINT_MATRIX_PENCIL.md` |
| [Unresolved symbolic kinematics and ladder scope](reference/UNRESOLVED_KINEMATIC_SCOPE.md) | `reference/UNRESOLVED_KINEMATIC_SCOPE.md` |
| [Original aggressive merging: defects and compatibility](reference/UPSTREAM_MERGE_AUDIT.md) | `reference/UPSTREAM_MERGE_AUDIT.md` |
| [Disposition of all eleven upstream utility tests](reference/UPSTREAM_UTILITY_TESTS.md) | `reference/UPSTREAM_UTILITY_TESTS.md` |
| [Broader pilot of the optional Wang policy](reference/WANG_ADOPTION_PILOT.md) | `reference/WANG_ADOPTION_PILOT.md` |
| [Opt-in weighted-square certificates](reference/WEIGHTED_SQUARE_PRODUCTION.md) | `reference/WEIGHTED_SQUARE_PRODUCTION.md` |
| [Exact guide audit for the remaining symbolic ZZ inputs](reference/ZZ_RAYLEIGH_GUIDE_AUDIT.md) | `reference/ZZ_RAYLEIGH_GUIDE_AUDIT.md` |
| [Isolated positive-axis API experiment](reference/research/positive_axis_api/README.md) | `reference/research/positive_axis_api/README.md` |
