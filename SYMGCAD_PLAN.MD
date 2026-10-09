# symGCAD: exact Rust GCAD with Symbolica

## 1. Objective and execution contract

**Objective:** Build symGCAD entirely in Rust, using Symbolica for symbolic and algebraic operations, with a reusable library and TOML-capable CLI. Reproduce the original GCAD strategy with a baseline runtime geometric mean no greater than 1.25 times the native upstream implementation. Develop and benchmark mathematically sound performance improvements, including bounded heuristic-guided MCTS for variable ordering. Produce validated, complete generic decompositions for every distinct problem in the reference corpus, including current intractables, prioritizing the pictured A446 topology at fixed kinematics before its symbolic counterpart.

At implementation start:

- Write this plan’s entire contents verbatim, excluding the surrounding plan tags, to `/common/dev/gcad/SYMGCAD_PLAN.MD`.
- Call `create_goal` with the Objective paragraph verbatim, and execute the saved plan under that goal.
- Create a `codex/` branch and make local commits at the milestones below.
- Keep reference checkouts, literature, generated benchmark corpora, and large results outside commits. Record their revisions and hashes.
- Keep the goal unfinished while any distinct corpus problem remains unresolved. A timeout or exhausted campaign is an incomplete result.

The deliverable is the GCAD engine, algebraic cell export, validation, and benchmarks. Numerical integration and a complete sector-decomposition pipeline remain downstream.

## 2. Environment, library, and CLI

- Provide a locked Nix development environment with Rust ≥1.96, Cargo, native build tools, GMP/MPFR, and profiling tools. Provide a separate reference environment containing upstream GCAD’s Python/Cython/FLINT dependencies.
- Initially depend on the local Symbolica checkout, recording its revision. Repeat its local `numerica` and `graphica` dependency patches in the root manifest. Disable unnecessary Python, native-code-generation, and FLINT benchmark features.
- Keep all production algebra in Symbolica: exact coefficients, polynomial operations, factorization, resultants, differentiation, substitution, and certified root isolation. Implement discriminants by composing Symbolica operations.
- Keep projection data in canonical sparse polynomial form with explicit variable maps. Avoid repeated expression/string conversions and detect exponent overflow.
- Make Symbolica changes only when a measured limitation requires them; retain separately replayable patches with regression tests and upstream-ready explanations.

Expose Rust interfaces for problem normalization, projection, decomposition, ordering search, verification, and serialization. Provide CLI commands:

| Command | Purpose |
|---|---|
| `symgcad solve problem.toml` | Produce a generic decomposition |
| `symgcad project problem.toml` | Inspect projection sets and costs |
| `symgcad order problem.toml` | Run bounded ordering search |
| `symgcad verify result.json` | Check a saved decomposition and its evidence |
| `symgcad bench corpus.toml` | Compare engines and optimization configurations |

TOML specifies variables, polynomial expressions, strict constraints, exact parameter substitutions, optional polynomials to split by sign, variable groups or a fixed order, engine, merging policy, and resource limits. Support positive-orthant and bounded-box problems without assuming either domain globally.

Versioned JSON output contains the normalized problem, actual order, polynomial table, cells, rational samples, root-index boundaries, sample-specific isolating intervals, exceptional-set information, validation evidence, and timings. Distinguish `complete_generic`, resource-limited/incomplete, invalid input, and computation failure.

Default export preserves unmerged cells. Aggressive upstream-compatible merging is opt-in and retains excluded-seam provenance; it must not silently erase an interior \(F=0\) singularity.

## 3. Faithful baseline, corpus, and correctness

Implement the upstream mathematical pipeline before introducing algorithmic reductions:

1. Convert strict inequalities into positive polynomials, clearing denominators without changing signs.
2. Factor, normalize, and deduplicate projection polynomials.
3. Eliminate variables from last to first using leading coefficients, discriminants, and pairwise resultants, factoring at each level.
4. Lift from first to last using certified real-root isolation and rational samples in open intervals.
5. Preserve upstream MODS, SOTD, and T1 ordering conventions, including variable groups and tie-breaking.

Use Symbolica’s exact root facilities. Handle coincident roots through exact algebra and interval refinement rather than representation equality.

Inventory all 177 collected upstream tests, the three disabled core examples, and every distinct mathematical problem represented by the ZZ fixtures. Preserve provenance for commented variants and generated one-loop inputs. Deduplicate only through checked polynomial/domain equivalence, retaining every original identifier. Rust owns the importer and benchmark supervisor; Python is confined to invoking the independent reference implementation and extracting reference data.

Keep two ZZ workloads separate:

- **Existing symbolic tests:** \(m_t^2=1,\ m_Z^2=5/18,\ s>4,\ x_i>0\), with \(s\) and applicable \(t\) remaining symbolic.
- **Fixed-kinematics workload:** substitute \(s=10,\ t=-3\) before algebraic processing, as specified in the [HP2 talk](https://indico.kit.edu/event/5167/contributions/23879/attachments/11093/19138/talk_hp2.pdf).

For an input solution set \(S\), certified raw output \(A\) must satisfy
\(A\subseteq S\) and \(S\setminus A\subseteq Z(E)\), where \(E\) is an explicit finite collection of nonzero exceptional polynomials. Cell interiors must be disjoint. This is generic coverage; exceptional parameter slices require separate specialization.

Verification must check factor reconstruction, projection obligations, root counts and ordering, and exhaustive lifting: every open interval is either explored or rejected with an exact reason. Sample checks alone are insufficient. Interrupted projection or unexplored branches prevent complete status.

Treat upstream results as compatibility evidence, not an infallible oracle. Independently resolve incorrect assertions or implementation defects, including strict zero constraints and aggressive merging.

## 4. Performance research and bounded MCTS

Implement and measure improvements in this order, retaining the baseline engine for comparison.

**Reuse and algebra costs**

- Cache canonical factorizations, resultants, and completed projection prefixes.
- Reuse projection work performed during ordering and share projection between \(F>0\) and \(F<0\).
- Avoid redundant degree-zero resultants; use exact low-degree identities and endpoint evaluation where applicable.
- Profile Symbolica’s existing resultant strategies on captured operands before proposing backend changes.
- Reuse equivalent problems only with recorded, invertible transformations.

**Domain and Feynman-specific reductions**

- Add exact coefficient-sign tests, bounded Pólya certificates on suitable homogeneous domains, and Bernstein certificates on rational boxes.
- Remove a factor only after proving strict nonvanishing throughout a suitable superset of the relevant projected domain. Preserve domain boundaries; retain every inconclusive factor.
- Exploit verified homogeneity by solving with one positive parameter fixed to one, then reconstructing the positive scale and original cylindrical bounds. Check every constraint’s scaling identity and transformation coverage. Respect fixed orders and keep kinematic variables outside the scale block.
- Use low degrees, endpoint behavior, support sparsity, discriminant complexity, and threshold geometry as ordering features.
- Use Landau analysis to guide searches and transformations. Physical Landau loci alone do not justify removing projection factors or ordinary \(F=0\) sheets.
- Evaluate local projection as a separate experimental engine, implementing its applicability, delineability, and fallback requirements from [Strzeboński’s algorithm](https://arxiv.org/abs/1405.4925).

**Ordering portfolio and MCTS**

First benchmark the existing orderings against Brown-style and sparsity-based candidates, informed by [CAD ordering research](https://arxiv.org/abs/2206.13480). Then implement UCT search over legal elimination prefixes, with heuristic-ranked expansion and seeded stochastic greedy rollouts.

- Respect variable groups and seed the search with the best inexpensive candidate orders.
- Track degree, term and factor growth, coefficient sizes, projection time, and estimated lifting cost.
- Use completed exact solves to select winners; intermediate scores remain predictions.
- Key cached states by canonical polynomials, remaining variables, domain assumptions, transformation, and algorithm version.
- Default to seed 0, at most 256 rollouts, and a search budget of the smaller of 300 seconds or 20% of the campaign. Promote promising trials through 1-, 10-, and 60-second limits, reusing completed work.
- Enforce deadlines with killable worker processes: an individual algebra call cannot be bounded reliably through checks between calls.
- Abandoning an ordering never discards a mathematical solution region. If nothing completes, retain an incomplete status and the best restart candidate.

MCTS has precedent in related [polynomial ordering optimization](https://arxiv.org/abs/1207.7079); its benefit for this GCAD workload must be demonstrated through ablations against the heuristic portfolio.

## 5. Milestones, tests, and acceptance

| Milestone | Required result before local commit |
|---|---|
| M0 — Foundation | Saved plan, active goal, Nix environment, dependency provenance, CLI skeleton, corpus inventory |
| M1 — Correct baseline | Exact projection/lifting, root export, verifier, core regressions, upstream comparison adapter |
| M2 — Runtime parity | Required upstream-completing cases reproduced; geometric-mean runtime ratio ≤1.25; regressions above 2× investigated and reported |
| M3 — Certified reductions | Separately measured reuse, positivity, and homogeneity improvements with verification evidence |
| M4 — Ordering search | Bounded MCTS, reliable termination/checkpointing, and comparisons including search overhead |
| M5 — Hard topology | Verified A446 and Ax12 decompositions at fixed kinematics, followed by symbolic domains |
| M6 — Full completion | Every distinct corpus problem solved and verified; reproducible benchmark report and outstanding defects resolved |

The pictured topology matches `finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1`; its Ax12 counterpart is the crossed regression. The graph’s spanning-tree and two-forest polynomials match the fixture exactly.

Benchmark the accelerated native upstream engine against Rust release builds using identical inputs, fixed orders, merge policies, CPU allocation, and limits. Freeze the reference-completing set before measuring Rust so Rust timeouts cannot disappear from the parity calculation. Use three fresh-process measurements and report medians, phase timings, peak RSS, projection growth, and raw/merged cell counts. Report timeouts separately. Optimized comparisons include search and preprocessing costs; warm-cache results are labelled separately.

Test exact rational handling, false and zero constraints, repeated/shared/close roots, narrow and disconnected regions, inactive variables, degree drops, nullification, coordinate boundaries, same-sign singular seams, transformation reconstruction, serialization, worker termination, and checkpoint recovery. Compare small decompositions through exact common refinements or symmetric differences; use random sampling only as supplemental evidence.

Each hard-case campaign is capped at **16 CPU cores, 64 GiB aggregate memory, and one hour**, respecting Symbolica’s available execution capabilities. Preserve completed work between campaigns. Continue unresolved cases through further bounded campaigns and algorithmic work; neither the fixed-topology milestone nor a benchmark report substitutes for the requested all-cases completion.
