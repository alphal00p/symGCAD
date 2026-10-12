// symGCAD: inventory of improvements over the original GCAD by Vitaly Magerya
// (github.com/magv/gcad, revision fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e, pinned in PROVENANCE.toml).
//
// Content only: no page, document or global text setup. Included by benchmark/benchmark.typ
// as Section 2 (level-1 heading = section, level-2 headings = categories).
// Every item carries a grey ID; improvements-sources.txt (same directory) maps each ID to the
// code, tests, reference notes, commits and session reports it is based on.
// State described: codex/symgcad 81d9ac5 plus the 2026-10-12 session, integrated at
// 9baf91c (claude/projective-sections: algebra fixes, local-engine sibling retention,
// bilinear_general, kinematic portfolio ordering, opt-in projective sections).

#let inv-tag(fill, ink, body) = box(
  fill: fill, inset: (x: 2.4pt, y: 0pt), outset: (y: 1.6pt), radius: 1.5pt,
  text(size: 0.74em, weight: "bold", fill: ink, body),
)
#let inv-new = inv-tag(rgb("FDE7A6"), rgb("6B4A00"))[NEW 2026-10-12]
#let inv-on = inv-tag(rgb("DDF1E3"), rgb("1F5E33"))[default-on]
#let inv-opt = inv-tag(rgb("DCE8F7"), rgb("1E3F73"))[opt-in]
#let inv-patch = inv-tag(rgb("EFE3F6"), rgb("5A2A72"))[optional patch]
#let inv-wip = inv-tag(rgb("F9DADA"), rgb("8A1F25"))[in progress]
#let inv-diag = inv-tag(rgb("ECEFF1"), rgb("37474F"))[diagnostic]
#let inv-up = inv-tag(rgb("ECEFF1"), rgb("37474F"))[upstream only]
#let inv-id(s) = text(size: 0.72em, fill: rgb("8896A3"), s)
#let inv-eff = text(style: "italic")[Effect:]

= Improvements over the original GCAD

The reference is Vitaly Magerya's GCAD (Python/SymPy front end over FLINT), which implements Strzeboński's 2000 generic CAD: generic projection (leading coefficients, discriminants and pairwise resultants of a square-free basis), recursive lifting with exact real-root isolation, the MODS/SOTD/T1 ordering heuristics and an aggressive cell merge.
Everything listed below goes beyond that.

*Vocabulary.* A CAD splits the solution set into _cells_. _Projection_ eliminates variables one at a time and collects the polynomials (resultants, discriminants, leading coefficients) whose zeros may change the picture; _lifting_ goes back up, picking rational _sample_ points and isolating real roots above them. The _order_ fixes which variable is eliminated first. A _certificate_ is an exact, replayable proof (e.g. that a polynomial never vanishes on the domain) that lets a polynomial be dropped. A _guard_ is a condition that must hold for a local shortcut to be valid. The _anchor_ of a homogeneous problem is the positive variable that is fixed to 1. _Generic_ means correct outside an explicit finite list of lower-dimensional exceptional sets.

*Tags.* #inv-on active without any setting; #inv-opt enabled by an option, engine or ordering choice; #inv-patch optional dependency patch, never in the default build; #inv-diag measurement or audit tool, not part of a solve; #inv-new written in the 2026-10-12 session (integrated at commit `9baf91c`: 753 fast tests pass there, against 662 at the `81d9ac5` checkpoint).

*Reading the numbers.* Unless a line says otherwise, an effect is one measured case or operand: it shows what the change does, not a corpus-wide speedup (corpus-wide timings are in Section 1). "Timeout" results never count as solved. All reductions keep the exact default contract: every cell is proved, every interior `F=0` sheet and same-sign seam stays a boundary. The only exception is the opt-in contract extension X1, which must be switched on explicitly and is never counted in the strict totals.

== Exact algebra and kernels

- #inv-id[A1] *Rust and Symbolica exact backend.* All polynomial arithmetic, GCDs, factorization and resultants run in Rust on Symbolica, in single-threaded workers. #inv-eff in the matched parity audit (release `9025e71`, identical orders, plain `baseline` projection), symGCAD reproduced the raw cell counts of all 483 sign-expanded problems GCAD completes; projection + lifting + merging took a geometric mean of 0.1466 of GCAD's time (6.82× faster), whole processes 0.0620 (16.1×). The current end-to-end comparison is in Section 1. #inv-on
- #inv-id[A2] *Sign-preserving exact input normalization.* Inequalities become signed positive polynomials; rational content is cleared without flipping a sign. `false`, a strict zero constraint and an identically zero split are told apart, and exponent overflow, undeclared symbols and approximate kinematics are rejected. #inv-on
- #inv-id[A3] *Full variable maps.* Every polynomial keeps its complete ordered variable map, inactive variables included, so cached algebra can never be reused under the wrong ordering or chart. #inv-on
- #inv-id[A4] *Deterministic resultants.* Production resultants use Symbolica's deterministic integer Lazard–Ducos routine. Symbolica's automatic dispatcher, whose modular CRT path can stop after random-prime checks, is confined to profiling; caches built with it were invalidated. #inv-on
- #inv-id[A5] *Closed-form low-degree discriminants.* Exact invariant formulas up to degree 4 replace the generic `Res(p, p')`. #inv-eff captured quartic (642 → 69,474 terms): 2.71 s / 48 MiB against 29.77 s / 472 MiB (10.98× time, 9.83× memory). #inv-on
- #inv-id[A6] *Exact evaluation/interpolation resultants.* One-parameter resultants and discriminants are rebuilt from (proved degree bound + 1) exact integer images. #inv-eff captured symbolic A446 pair (degrees 23 and 17, 588/387 terms): 5.888 s with 872 images; every Ducos variant exceeded 60 s. #inv-on
- #inv-id[A7] *Assignment degree bound and size-derived admission.* The image count uses the smaller of the closed-form Sylvester bounds and an exact assignment (Hungarian) bound on the Sylvester matrix. Plans outside the historical envelope (closed-form bound ≤ 1,024), up to 65,536, are admitted only if an a-priori bit bound on every image fits the caps, so no resource stop can fire after admission. #inv-eff degree-1,828 discriminant: 27.80 s (old multivariate-image interpolation 52.31 s, old default path > 660 s); degree-2,220: 58.83 s against 106.79 s. #inv-new #inv-on
- #inv-id[A8] *Exact integer images with a zero-image guard.* Each image is an exact integer resultant; an exact gcd test first decides whether it is zero, because Symbolica 3.0.1's Lazard–Ducos panics on a vanishing subresultant (see U3). The same guard protects the full-Ducos path. #inv-eff a pair on which the previous release crashed now solves (7 cells) and verifies. #inv-new #inv-on
- #inv-id[A9] *Binary (Kronecker) segmentation.* Two-parameter resultants and discriminants are packed into one integer polynomial under proved degree and coefficient bounds, computed once and decoded with complete signed checks. #inv-eff authentic A317/A318 resultants of 9,835/18,939 terms in 38.16/20.03 s; forced on a 56-degree-product discriminant 2.17 s against Ducos 7.80 s. Default since 2026-10-12 (`binary_resultants = false` opts out). #inv-new #inv-on
- #inv-id[A10] *Second-route spot check.* Every interpolated or segmented result is re-evaluated at one extra full-degree point through Symbolica's own substitution and Brown resultant, in the solver and in the verifier. #inv-eff cost within noise (27.86 s against 27.87 s). #inv-new #inv-on
- #inv-id[A11] *Sum-variable factor preconditioning.* A small polynomial that depends on two variables only through their sum is factored in fewer variables; the inverse substitution and the factor product are checked. #inv-eff a stalled A446 factorization (native > 60 s) took 0.003–0.025 s. #inv-on
- #inv-id[A12] *Affine factorization preconditioning.* Before factoring, a bounded exact integer translation moves the polynomial to a regular evaluation fiber; factors are mapped back and the signed product rebuilt. #inv-eff 224-term Gluza33 factor 1.63 s; the native call timed out at 30.02 s. #inv-opt (`affine_factorization`)
- #inv-id[A13] *Support-preserving diagonal scaling.* Tried first inside the affine policy: nonzero scalings reach the same regular fiber without changing the monomial support. #inv-eff a 1,283-term factor on which translation stalled 30 s factors in 0.048 s. #inv-opt
- #inv-id[A14] *Bounded exact shear.* Third strategy of the affine policy (`x -> x + c·y` with exact inverse). #inv-eff captured C470 obstruction 0.088 s; the original C470 continuation then saved 56,671 more exact operations (still incomplete). #inv-opt
- #inv-id[A15] *Primitive parameter-linear irreducibility shortcut.* Gauss's lemma proves a primitive polynomial linear in one variable irreducible without calling the factorizer. #inv-eff 4.840 → 0.120 ms and 1.463 → 0.079 ms on two core inputs. #inv-on

== Root isolation and lifting

- #inv-id[L1] *Algebraic cell boundaries re-isolated at every prefix.* A boundary is a root index of a specialized polynomial; roots are isolated exactly at each new rational prefix and coincident roots are separated by gcd and refinement under a work bound. #inv-on
- #inv-id[L2] *Open-interval Sturm counting.* Signed primitive pseudo-remainder sequences count roots on open or unbounded intervals (Symbolica only counts on `(0, ∞)`). #inv-eff degree-12 A381 guard 3.184 → 0.602 ms, largest coefficient 2,281 → 555 bits. #inv-on
- #inv-id[L3] *Integer sign evaluation.* Signs at rational points use homogeneous integer Horner evaluation instead of rational arithmetic. #inv-eff 2.64–11.01× in a microbenchmark. #inv-on
- #inv-id[L4] *Integer prefix evaluation.* Assigned prefixes are substituted through exact integer numerators. #inv-eff guard batches 3.79× (A446) and 5.33× (A381); the matched whole-solver control moved < 1% (43.68 → 43.30 s). #inv-on
- #inv-id[L5] *Single-pass batched specialization.* All coordinates of a prefix are substituted in one pass with shared powers. #inv-eff 2,000 prefixes of `ex_e5/2`: 0.401 → 0.252 s; `ex38`: 0.079 → 0.052 s. #inv-on
- #inv-id[L6] *Deferred GCDs and borrowed root signs.* Redundant GCDs are postponed and sign evaluations reuse root data. #inv-eff improves the three cases where symGCAD was slowest relative to GCAD; their time ratios remain 2.09, 1.61 and 2.75. #inv-on
- #inv-id[L7] *Primitive shared root fibers.* Root intervals share one immutable defining polynomial; retained rational fibers are made primitive integer polynomials; affine prefix guards skip power tables. #inv-eff A414 fixed-work traversal 7.748 → 6.917 s (−10.7%; a first pair was 5.3% slower). #inv-on
- #inv-id[L8] *Ordered root union.* A priority queue merges the roots of many cuts with exact equality and bisection checks, including coincident roots. No speedup established. #inv-on
- #inv-id[L9] *Positive and descending root selectors.* A boundary may count roots among the positive roots only (`positive`) or from the largest real root down (`real_descending`), so collisions among negative roots cannot change its meaning. #inv-on (produced only by certified rules)

== Projection operators and certified reductions

- #inv-id[P1] *Faithful generic projection.* The `baseline` engine reproduces Strzeboński's projection and lifting exactly (the reference for A1). #inv-on
- #inv-id[P2] *Optimized projection engine.* The same projection with canonical operation reuse, prefix reuse and certified removals (P3). #inv-opt (`engine = "optimized"`)
- #inv-id[P3] *Certified factor removal.* A generated projection factor is dropped only with an exact certificate that it is strictly nonzero on the domain (nonnegativity is not enough); original constraints and splits stay protected. `input_positivity` extends this to input factors. #inv-eff fixed A446: 1.326 s with it, timeout at 60 s without. #inv-opt (`positivity`)
- #inv-id[P4] *Operation pruning.* Tries a nonvanishing proof before factoring a whole projection operation; off by default because a failed proof costs time. #inv-opt (`prune_operations`)
- #inv-id[P5] *Strict-Bézout pair omission in global projection.* A supplied identity `p*L + q*M = k*S` with `k > 0`, where `S = H^2` with `H` certified nonzero or `S` certified positive, proves that `p, q` have no common zero in the box, so `Res(p, q)` is skipped; `p`, `q`, their leading coefficients and discriminants stay, face pairs are never skipped, and each omission is replayed by the verifier. #inv-eff Gluza33 piece `R--`, third level: 1,033 → 661 polynomials and 228,998 → 112,759 terms (−36%, −51%), reached at 122.4 s instead of 164.2 s. #inv-new #inv-opt (`projection_strict_bezout_hints`)

== Positivity and domain certificates

- #inv-id[C1] *Exact nonvanishing certificate portfolio.* Coefficient signs, positive coordinate shifts, Pólya/Bernstein tests, univariate Sturm on open and unbounded domains and exact quadratic rules, all bounded; each certificate is serialized and replayed on the original domain. #inv-on (used when `positivity` is set)
- #inv-id[C2] *Parameterized quadratics.* Proves positivity of quadratics whose discriminant is exactly negative on the domain. #inv-on
- #inv-id[C3] *Square plus positive residual.* Writes a polynomial as an exact square (monomial or affine, also after a homogeneous scale is fixed) plus a strictly positive residual, with oriented bounds. #inv-on
- #inv-id[C4] *Weighted-square certificates.* Sums of positive-monomial-weighted binomial squares plus a positive residual. #inv-eff "positive endpoint" case: 20 → 1 cell, 37.2 → 9.3 ms solver time. #inv-opt (`weighted_square_positivity`)
- #inv-id[C5] *Positive-semidefinite quadratic components.* Exact rational LDL elimination of quadratic subforms plus a strictly positive residual; no floating-point eigenvalues. #inv-eff full quadratic constant 8 → 1 cell (18.3 → 7.6 ms); local linear split 12 → 2 cells, interior zero sheet kept. #inv-opt (`quadratic_component_positivity`)
- #inv-id[C6] *Caller-chosen certificate budgets.* Library callers may raise the limits to 262,144 Bernstein coefficients and 10#super[9] work units (defaults 4,096 and 10#super[6]); the chosen limits are serialized and replayed (symGCAD PR \#2). #inv-opt
- #inv-id[C7] *Early abandonment from exact samples.* An exact zero or a sign change at a sample ends a hopeless certificate attempt early; samples never prove anything. #inv-on
- #inv-id[C8] *Certified local guards.* In the local engine, a certificate replaces a generated guard, and its own premises become protected ancestor obligations. #inv-eff fixed A446 local: 8 certificates replaced 436 guard occurrences; 35 cells in 0.427 s, where the unreduced run timed out at 60 s. #inv-opt (`positivity` with `engine = "local"`)

== Composite engines

- #inv-id[E1] *Homogeneous positive-scale reduction.* For a homogeneous block, one proved-positive anchor coordinate is fixed to 1; the smaller problem is solved and the original cells are rebuilt exactly. #inv-eff fixed A446 2.409 → 1.326 s, fixed Ax12 1.032 → 1.445 s (mixed). #inv-opt (`homogeneous_block`, `homogeneous_anchor`)
- #inv-id[E2] *Monotone engine.* Peels positive axes on which exact linear or quadratic certificates prove strict increase, solves the endpoint problem and restores every root sector. #inv-eff fixed A446/Ax12: 35 → 8 cells; A446 1.326 → 0.825 s. #inv-opt (`engine = "monotone"`)
- #inv-id[E3] *Conditional quadratic peel.* Positive-multiplier certificates justify consecutive quadratic endpoint reductions. #inv-eff fixed A381 and Ax381: 836 verified cells each, medians 0.186 and 0.102 s; all 6 trials without the rule timed out at 60 s. #inv-opt (inside `monotone`)
- #inv-id[E4] *Negative-target mode.* Solves `Q > 0` as the negative side of `F = -Q` through its endpoint inequality, restoring only the interval below the positive root. #inv-eff full original physical elliptic problem: 269,061 cells, solve 33.86 s, verify 73.09 s. #inv-opt (`monotone_negative_target`)
- #inv-id[E5] *Sign filter.* For an unsplit problem, decomposes the full sign arrangement of one non-affine constraint and keeps exactly the required cells, with an exhaustive selection map. #inv-opt (inside `monotone`)
- #inv-id[E6] *Linear-negative engine.* A target linear in `z` with `A*z + B < 0` is reduced to `A < 0` using a guide whose positivity and square identity `B*C - A*D = k*H^2` are proved; the lower boundary is the largest root of `z*q`. #inv-eff negative Gluza30: 1 cell, 0.185 s; negative Gluza33: 3 cells, 0.164 s solve, 0.021 s verify; GCAD timed out at 60 s on both. #inv-opt (`engine = "linear_negative"`)
- #inv-id[E7] *Bilinear-positive engine.* For `a*x*y + b*x + c*y + d > 0` on the last two positive axes, seven strict guide certificates fix the fiber shape; the base is solved over the signs of `a, b, c`. #inv-eff positive Gluza30: 80,980 verified cells, solve/verify medians 22.57/16.85 s; GCAD timed out. (Under the opt-in contract X1 the same problem needs 1 cell.) #inv-opt (`engine = "bilinear_positive"`)
- #inv-id[E8] *General bilinear engine with pieces.* Handles all 16 sign patterns of `a, b, c, d`: ratio premises found by square-free decomposition, a separator `G` with `b*c - a*d = ±m*G^2`, and a planner that splits the base into independently solvable pieces plus proved-empty regions. The verifier checks the partition by a truth table and verifies every piece. #inv-eff Gluza33: plan of 4 pieces + 1 empty claim in 0.034 s; synthetic chain 47 verified cells; under the strict contract the Gluza33 pieces still time out (1,200 s), with X1 the engine completes positive Gluza33. #inv-new #inv-opt (`engine = "bilinear_general"`)
- #inv-id[E9] *Quadratic ladder.* When a quadratic opens two positive roots below a linear fiber, restores a proved five-sector arrangement over a verified base, keeping both sides of `F = 0`. #inv-eff crossed A446 chamber `t < 4`: 5 verified cells in 0.022 s (debug); the unrestricted problem stays unresolved. #inv-opt (`engine = "quadratic_ladder"`)
- #inv-id[E10] *Selectable base engines.* Monotone and bilinear wrappers solve their base with `baseline`, `optimized` or `local`, optionally with an inner homogeneous anchor and block that keeps kinematic parameters outside the scale block. #inv-opt (`bilinear_general_base_block` is #inv-new)
- #inv-id[E11] *Projection-free peel inspection.* Reports which peels are certifiable for an input without solving. #inv-diag

== Local projection engine

- #inv-id[LP1] *Local projection.* An adaptation of Strzeboński's local projection: cuts are projected only for the stacks actually sampled, missing ancestor cuts are learned and the affected stacks rebuilt; the whole tree is checked by the independent verifier. #inv-opt (`engine = "local"`)
- #inv-id[LP2] *Batched sibling discovery.* Collects obligations from several siblings before rebuilding a stack. #inv-on
- #inv-id[LP3] *Direct first-axis base.* On the first lifting axis, univariate cuts need no pair resultants: their real-root union is isolated directly. #inv-eff a fixed A381 run avoided 4,150,168 potential first-axis pairs. #inv-on
- #inv-id[LP4] *Positive-ray Descartes rule.* On a proved-positive axis, zero or one coefficient sign variation replaces the discriminant (no or exactly one simple positive root). #inv-on (`local_descartes_one`)
- #inv-id[LP5] *Quadratic positive-ray classifier with cached guards.* Exact quadratic sign cases decide positive roots; guard results are cached. #inv-eff fixed-work A381 debug controls: solver time −15.0% and −14.5% (process wall time mixed). #inv-on
- #inv-id[LP6] *Leading-term cancellation separation.* Up to two exact cancellations whose coefficients all have one certified sign prove that two polynomials share no positive root, so their resultant is skipped. #inv-on (`local_pair_cancellations = 2`)
- #inv-id[LP7] *Adjacent-root pairs.* Projects only pairs whose roots are adjacent at the sample. #inv-eff no completion gain: all 27 symbolic release attempts timed out. #inv-opt (`local_adjacent_pairs`)
- #inv-id[LP8] *Descartes witnesses for 2–8 positive roots.* Rational separators with alternating signs plus coefficient guards prove each interval holds one simple root; at most two templates per polynomial keep the guard set finite; source size budget 1–65,536 terms. #inv-eff with 8,192 terms, the original Gluza33 passed a 6,938-term degree-16 discriminant and saved 4,554 more operations. #inv-opt (`local_descartes_witnesses`)
- #inv-id[LP9] *Interval Descartes covers.* Covers the positive ray by rational intervals with zero or one sign variation each, for fibers where the global count is not exact. #inv-eff Gluza33 19,942-term degree-24 discriminant (2 roots, 4 variations) bypassed with 49 guards (factored in 9.27 s): +6,422 operations; a 131,072-term aggregate budget gave +7,194; template limit 4 gave +1,094 and +2,397. #inv-opt (`local_descartes_intervals` and its budgets)
- #inv-id[LP10] *Adjacent interval coalescing.* Merges neighbouring pieces when an exact recount keeps at most one variation. #inv-eff four-root cover with 97 guards / 109,726 terms built in 1.60 s; the greedy cover needed 121 / 139,654, over budget. #inv-on (when intervals are enabled)
- #inv-id[LP11] *Positive pair witnesses.* Once both polynomials' positive roots are proved delineable, rational separators between them replace the pair resultant. #inv-opt (`local_positive_pair_witnesses`)
- #inv-id[LP12] *Pair-witness admission gate and axis budget.* A new separator template is admitted only when the resultant it replaces would be large (dense term bound above a threshold), and per-axis template counts can be capped. #inv-eff Gluza33 base piece `a > 0` with witnesses: 20.4 million nodes in 570 s without finishing → 0.02 s, 178 verified cells at gate 4,096; on 19 of 20 corpus cases the gate behaved like witnesses off. Default 0 keeps the old policy. #inv-new #inv-opt (`local_pair_witness_min_resultant_terms`, `local_pair_witness_axis_budget`)
- #inv-id[LP13] *Local strict-Bézout hints.* The local counterpart of P5 for chosen cut pairs. #inv-eff positive Gluza30: 49 hints skipped 2,918 resultants (no completion). #inv-opt (`local_strict_bezout_hints`)
- #inv-id[LP14] *Completed-subtree reuse.* Complete conditional proofs are reused at identical rational prefixes after their obligations are rechecked; budget 32 MiB, entries 128–4,096 nodes. #inv-eff C470 at an equal refinement limit 8.40 → 6.96 s with 1,024-node entries; no full-solve gain. #inv-on
- #inv-id[LP15] *Learned-cut hints.* Unfinished stacks keep their cuts as hints (8 MiB), always re-projected. #inv-on
- #inv-id[LP16] *Persistent hint archive.* Saves cut hints across restarts (8 MiB). #inv-eff no completion gain in matched restarts. #inv-opt (`local_hint_archive`)
- #inv-id[LP17] *Stable samples.* Keeps a rational sample across refinements when an exact comparison places it strictly inside a new sector, preserving cache hits. #inv-opt (`local_stable_samples`)
- #inv-id[LP18] *Deferred projection with cut compaction.* Discovers complete children before projecting, then drops learned cuts no child needs. #inv-eff A414 pilot still timed out at 300 s. #inv-opt (`local_deferred_projection`)
- #inv-id[LP19] *Retained sibling proofs.* Completed children survive a stack's refinement rounds and move to the new sector containing their unchanged sample, under a shared memory budget (a quarter of the memory limit at 4 KiB per node). #inv-eff Gluza33 piece `Ppm`, 30 min: 12.80 → 1.38 million speculative nodes for the same progress (still incomplete); fixed `B6_469` (witness recipe, gate 4,096) completes only with retention: 112,124 verified cells in 9.91 s. #inv-new #inv-opt (`local_retain_siblings`)
- #inv-id[LP20] *First-axis square-free support basis.* Replaces irreducible factorization of bounded generated first-axis guards by checked square-free blocks. #inv-eff 92.06 s against 185.83 s on one A377 block set. #inv-opt (`local_base_univariate_basis`)
- #inv-id[LP21] *Discovery caps.* `local_max_refinements` (10,000) and `local_max_nodes` (100,000) bound speculative work; exhaustion returns `incomplete`. #inv-on
- #inv-id[LP22] *Refinement-thrash diagnostics.* Read-only counters of restarts, cut origins, discarded children and cache-miss causes (`SYMGCAD_LOCAL_THRASH_DIAG=1`). #inv-eff found 337,116 completed children discarded in 60 s on one Gluza33 piece, which motivated LP12 and LP19. #inv-new #inv-diag
- #inv-id[LP23] *Obstruction observer.* Records which guard fails at an exact prefix without changing proofs. #inv-diag

== Ordering and search

- #inv-id[O1] *Extra ordering heuristics.* Brown and sparsity orders join GCAD's MODS, SOTD and T1; variable groups and explicit orders bind every candidate. #inv-opt
- #inv-id[O2] *Bounded MCTS order search.* UCT over legal elimination prefixes (seed 0, at most 256 rollouts, search time `min(300 s, 20%)`, 1/10/60 s promotion tiers); only a completed exact solve can win. #inv-eff 126/126 cold ablation solves verified, but no end-to-end gain over cheap heuristics. #inv-opt (`ordering = "mcts"`)
- #inv-id[O3] *Search pruning.* Abandons trials whose completed prefixes show severe term growth; a prediction, never a region rejection. #inv-on (with MCTS)
- #inv-id[O4] *Local-progress promotion score.* Ranks timed-out local trials by polynomial inventory and refinement growth. #inv-eff reduced positive Gluza30 base: 3/3 verified, median 65.10 s, against 3/3 incomplete at 313.63 s. #inv-opt (`search_local_progress`)
- #inv-id[O5] *Kinematic portfolio ordering.* For parametric Feynman inputs, probes every certified homogeneous anchor × kinematic order in killable workers, counts the exact open kinematic cells, and solves with the fewest; decoupled kinematic variables are lifted last. #inv-eff 26 solved cases: 105.88 → 92.97 s total; C531 21.22 → 2.27 s; 4 held-out cases where the default ran out of 5 GiB now solve (5.7–47.6 s); losses `Cx12_438` 32.30 → 59.82 s and 36 tiny cases 2.07 → 7.73 s. #inv-new #inv-opt (`ordering = "kinematic_portfolio"`)
- #inv-id[O6] *Reproducible portfolio budgets.* Fixed per-candidate budgets give the same choice on 21/21 cases, at higher cost (1,317 s on the 26 cases). #inv-new #inv-opt (`portfolio_fixed_budgets`)
- #inv-id[O7] *Order proposal helpers.* Support-order seeds and short linear-prefix probes; measured, no gain. #inv-diag

== Caching and checkpointing

- #inv-id[K1] *Exact operation cache.* Factorizations, resultants, discriminants and completed projection prefixes are shared across stages and ordering trials (128 MiB). #inv-on (optimized and local engines)
- #inv-id[K2] *Persistent checkpoints.* Atomic, hash-validated records of every completed exact operation, kept through interruptions mid-stage. #inv-eff the final original Gluza33 campaign retained 32,456 operations (4,559 new) for continuation. #inv-opt (`checkpoint_dir`; automatic with MCTS)
- #inv-id[K3] *Domain-free operation keys.* Pure algebra records are keyed by variable map, axis, backend version and exact input, no longer by the domain; old records are validated and migrated. #inv-eff Gluza33 `R--` after `R+-`: 50 of 127 operations from disk through level 2 (old scheme: 0). #inv-new #inv-on (with `checkpoint_dir`)
- #inv-id[K4] *Primitive-linear cache bypass.* Trivially irreducible factors skip cache I/O. #inv-eff 30 polynomials, fresh memory and disk caches: 15.531 → 0.415 ms. #inv-on
- #inv-id[K5] *Search journal.* MCTS state (8 MiB, 4,096 records) is replayed for the same problem and executable; replayed claims count as unverified until re-solved. #inv-on (with MCTS)
- #inv-id[K6] *Untrusted proposal caches.* First-axis support proposals and hint archives are re-factored and re-checked on load; verification never reads a cache. #inv-opt
- #inv-id[K7] *Cache integrity audits.* Inherited bytes, operation keys and output hashes are checked before a continuation reuses work. #inv-diag

== Verification and evidence

- #inv-id[V1] *Explicit generic-coverage contract.* A result proves that its cells lie in the solution set `S` and cover `S` up to the zero set of an explicit finite list of nonzero exceptional polynomials; GCAD returns cells without this list. #inv-on
- #inv-id[V2] *Self-contained JSON proof.* Records the normalized problem, actual order, polynomial table, projection levels, rational samples, root-index boundaries, an exhaustive tree of stacks, rejections and accepted cells, and one of four statuses (`complete_generic`, `incomplete`, `invalid_input`, `computation_failure`). #inv-on
- #inv-id[V3] *Independent verifier.* `symgcad verify` re-normalizes the input, rebuilds every projection and reduction obligation, checks roots with exact arithmetic and walks every open interval; every engine has its own typed replay; an incomplete result cannot pass. #inv-on
- #inv-id[V4] *Raw cells and seams by default.* No merging unless asked: interior `F = 0` sheets and same-sign seams always remain boundaries (GCAD merges aggressively). #inv-on
- #inv-id[V5] *Indexed compatibility merge.* GCAD-style merging as a separate output that records the excluded seams. #inv-eff the 131,256-cell `ex38` now completes (median 40.72 s, 4.77 s merging; GCAD 26.50/7.45 s). #inv-opt (`merge = "aggressive"`)
- #inv-id[V6] *Replay hardening for the new rules.* Strict-Bézout omissions (11 tampered variants rejected), `bilinear_general` evidence (about 40 forgeries rejected), the assignment degree bound (checked through its LP dual) and unresolved portfolio inputs are all checked or refused by the verifier. #inv-new #inv-on
- #inv-id[V7] *Certified comparison of decompositions.* Exact common refinement of two results, across different orders and reductions. #inv-diag
- #inv-id[V8] *Seeded interior sanity check.* Fresh rational points in every cell, checked against the original TOML expressions. #inv-eff fixed A446: 8 cells, 16 `F` and 112 constraint evaluations, 0 failures, 0.41 s. #inv-diag
- #inv-id[V9] *Native comparison harness and parity audit.* Frozen GCAD cohort, three paired runs, adaptive native cap `max(300 s, 3 × symGCAD solve)`; documents GCAD merge and assertion defects (e.g. `failing_test_ex43`). #inv-diag
- #inv-id[V10] *Signed grid verification of huge algebra.* The A377 degree-32 discriminant candidate (degree 4,058, 3,261 terms) was proved by 4,773 exact evaluation points in 2,093.9 s. #inv-diag
- #inv-id[V11] *Original-identity audit.* Every claimed completion is re-checked against the original polynomials, domain, groups and order: 303 of 331 distinct problems (124/124 fixed ZZ, 97/124 symbolic ZZ, 82/83 core). #inv-diag

== Robustness and supervision

- #inv-id[R1] *Killable worker processes.* Solves, verifications and probes run in separate process groups with hard wall-time and memory limits (defaults 1 h, 64 GiB, 16 workers; 2 h on request), so a blocking native call cannot hang the run. #inv-on
- #inv-id[R2] *Campaign and slow-suite runners.* `symgcad bench` and the resumable one-hour slow queue record wall, CPU and RSS; timeouts stay incomplete. #inv-on
- #inv-id[R3] *Fast regression suite.* 753 tests at `9baf91c` (744 at `4dbe440`, 662 at `81d9ac5`), including proof mutations, same-sign seams, degree drops and worker termination. #inv-on
- #inv-id[R4] *Operand capture and replay.* `SYMGCAD_CAPTURE_DIR` saves the exact operand of the latest uncached operation (16 MiB cap) for offline profiling. #inv-diag
- #inv-id[R5] *Throttled progress reporting.* #inv-eff an unthrottled report had cost 17.505 s of system CPU in a 60 s C470 trial; now 1.282 s. #inv-on
- #inv-id[R6] *Buffered result export.* #inv-eff an A286 run that failed through 392,791 tiny writes completes 3,308 cells in 1.354 s. #inv-on
- #inv-id[R7] *Interrupted-worker accounting.* Timed-out workers keep their elapsed time and typed diagnostics. #inv-on
- #inv-id[R8] *Bounded parsing of untrusted hint text.* Size, degree and coefficient bounds are checked on the parse tree before Symbolica expands anything. #inv-eff `(x+y+z+1)^300` was rejected after 6.9 s, now in < 0.5 s; `7^(3^20)`, which never returned, is rejected. #inv-new #inv-on
- #inv-id[R9] *Fully supervised portfolio and pieces.* Portfolio enumeration, probes and `pieces plan/assemble` run in bounded workers; any failure falls back to MODS with the reason recorded. #inv-eff 3 s-wall test stops at 3.08 s with 491–518 MB peak (before: 19.1 s, 1.97 GB). #inv-new #inv-on
- #inv-id[R10] *Reproducible environment.* Nix flake and Cargo lock with a pinned public Symbolica revision; no private checkout needed (symGCAD PR \#1); CPU-affinity tools for pinned measurements. #inv-on

== Interfaces

- #inv-id[I1] *Rust library and TOML command line.* `solve`, `verify`, `project`, `order`, `bench`, `corpus`, `import-fastsecdec`. Inputs take exact rationals, `[parameters]`, `variable_groups`, binding orders and `[limits]`; unknown fields are refused (GCAD: a Python API on SymPy expressions). #inv-on
- #inv-id[I2] *`project` matches `solve`.* `symgcad project` now exports exactly the projection that `solve` lifts, including homogeneous reduction. #inv-eff Gluza33 `R+-`: 354.5 s / 453 MB, before a timeout at 1,500 s. #inv-new #inv-on
- #inv-id[I3] *Piece-wise solving from the command line.* `symgcad pieces plan` writes one TOML per piece; pieces are solved as separate processes in any order; `pieces assemble` checks and joins them. #inv-new #inv-opt
- #inv-id[I4] *FastSecDec graph import.* Reads native signed `U`/`F`, exact kinematics and edge-to-parameter maps from dressed HEPKit DOT graphs and checks them before solving. #inv-opt
- #inv-id[I5] *Rust corpus builder.* Turns GCAD's 177 collected tests, 265 literal two-loop and 12 generated one-loop fixtures into 331 exactly deduplicated problems (124 fixed ZZ, 124 symbolic ZZ, 83 core). #inv-on
- #inv-id[I6] *Generated benchmark document.* This report: `benchmark/build_report.py` turns the cold symGCAD runs, the native GCAD runs, the long attempts and the FLINT measurements into `report-data.json` with per-row provenance; Typst renders it. `benchmark/run_final.sh` repeats the whole campaign with one command. #inv-new #inv-on
- #inv-id[I7] *Reproducible full-corpus campaign.* One manifest of the 331 groups and their recipes, a cold symGCAD runner (fresh checkpoint directory and `TMPDIR` per run, one pinned CPU, independent `verify`) and a native runner (unmodified GCAD through the unmodified bridge, 300 s per sign case, 8 GiB). Both are resumable and record binary hashes. #inv-new #inv-diag
- #inv-id[I8] *Symbolica vs FLINT harness.* Exact operands captured from real solves are replayed through Symbolica's production kernels and FLINT 3.6 with canonical output comparison (Section 3). #inv-new #inv-diag

== Upstream dependency fixes

- #inv-id[U1] *Symbolica PR \#69: resultant sign with a linear argument.* `Res(x^3+y, x+2)` returned `y-8` instead of `8-y` in every public strategy. symGCAD uses its own signed identity for linear operands; the patch is optional. #inv-patch
- #inv-id[U2] *Symbolica PR \#75: integer left-shift overflow.* `Integer::from(1) << 63` gave a negative number; the patch is checked by 546 comparisons. No production path was affected. #inv-new (PR opened) #inv-patch
- #inv-id[U3] *Symbolica PR \#76: Lazard–Ducos panic on a vanishing subresultant.* Explicit zero coefficients made Ducos index out of bounds; patch with a regression test that fails before and passes after; production avoids it through A8. #inv-new #inv-patch
- #inv-id[U4] *Further upstream reports.* PRs \#61 (determinant sign on odd row swaps), \#67 and \#68 (F4 Gröbner zero generators and cache), \#73 (zero-variable exponent iteration), found while auditing Symbolica; no local patch is needed. #inv-up
- #inv-id[U5] *Bounded small-leading-coefficient Wang factorization.* #inv-eff 2,410-term Gluza33 factor 1.039 s, against a 60.03 s timeout without it. #inv-patch (experimental)
- #inv-id[U6] *Bounded main-axis choice for factorization.* #inv-eff 1,731-term component 0.280 s against a 30.03 s timeout; the whole 2,867-term source 1.951 s. #inv-patch (experimental)
- #inv-id[U7] *Positive-only isolation and unit Taylor shift.* #inv-eff unit shift −39% on an authentic operand; positive-root-only isolation 557.98 s / 421 MiB against 765.81 s / 813 MiB for all roots. #inv-patch (research backend)
- #inv-id[U8] *Idempotent patch bootstrap.* `scripts/bootstrap-deps.sh --apply-patches` applies the optional fixes to a separate research checkout; the default build stays pristine. #inv-opt

== Opt-in contract extensions

- #inv-id[X1] *Projective sections (opt-in contract extension).* With `solver.projective_sections = true` (engine `bilinear_general` only, default off), a cell's upper boundary may be "the k-th positive root of P, or +∞ where P has fewer roots": inside one cell the boundary can pass continuously through infinity (cells stay connected and cylindrical after the map `u = x/(1+x)`). For `F = a*x*y + b*x + c*y + d` with the replayed ratio premises and `b*c - a*d = m*G^2`, the fiber over a base point is empty when `d < 0` and exactly `{0 < y < Y*, 0 < x < X(y)}` when `d > 0`, so the base only has to decompose `d > 0`, one cell per base cell. The verifier replays the premises and the plan; every other verifier, the merge and the cell comparison refuse or handle such bounds explicitly. Approved by the user on 2026-10-12. #inv-eff positive Gluza33 (unresolved under the strict contract, longest attempt 7,200 s): 2,372 verified cells, solve 0.42 s, verify 0.55 s; a seeded sanity check of 4,744 fresh points evaluated the original `F` exactly with 0 failures. Positive Gluza30: 1 cell instead of 80,980. Not counted in the strict totals. #inv-new #inv-opt
- #inv-id[X2] *Aggressive merging.* GCAD-compatible merging that may cross seams; only as a separate output (see V5). #inv-opt

== Investigated, not adopted

- #inv-id[N1] *Convex two-point theory (2026-10-12).* For two-point graphs, `F/U` is convex; dropping touching pairs shrinks the Gluza33 level-2 projection from 86 to 71 factors under the current cell contract, and to 34 only with "generalized sectors", a contract change that was not implemented. #inv-diag
- #inv-id[N2] *Relevance-restricted projection prototype (2026-10-12).* No-go: about 1.9 million base and 3.0 million 8-variable cells for positive Gluza33 under the strict contract (ordinary CAD: about 10#super[10] on one piece). #inv-diag
- #inv-id[N3] *Literature and Symbolica audits (2026-10-12).* Landau, PLD and Whitney sets are necessary but not sufficient for cylindrical cells; Symbolica already supplies every primitive used, and over `Q(√10)` it splits Gluza33 discriminant factors (450 → 43 × 43 terms in 1.05 s) but stalls on 2,250 terms. #inv-diag
- #inv-id[N4] *Domain-aware ordering profile (2026-10-12).* Anchors and orders picked by predicted kinematic cells made solved symbolic test beds 2–312× faster (led to O5); no effect on the unresolved two-loop cases. #inv-diag
- #inv-id[N5] *Earlier research lines.* CFF and Landau chambers, tropical and SubTropica ordering, ordered quadratic flag identities, crossing transport, deterministic CRT and factorial-convolution Taylor shifts produced exact identities or proposals but no production gain. #inv-diag
