# symGCAD

Exact generic cylindrical algebraic decomposition in Rust, with Symbolica as the algebra backend. The library and CLI decompose conjunctions of strict rational-polynomial inequalities into full-dimensional open cylindrical cells. This project follows `SYMGCAD_PLAN.MD`; the all-corpus completion goal is still in progress.

## Build and run

The initial build uses the supplied local Symbolica checkout under `DO_NOT_PUSH_REFERENCES/symbolica`. The locked Nix environment supplies Rust and native dependencies.

```sh
nix develop
cargo build --release
cargo test -- --test-threads=1
target/release/symgcad solve examples/unit_square.toml --output artifacts/square.json
target/release/symgcad verify artifacts/square.json
```

Set `SYMBOLICA_LICENSE` in your environment when applicable. The development shell also reads the optional, ignored `.symgcad-env` file. License keys are never included in result metadata. Each solver worker uses one symbolic thread; search and benchmark subprocesses are bounded independently.

For an interrupted global solve, `SYMGCAD_LIFTING_PROGRESS_PATH=/tmp/lifting.json` records the current lifting prefix, stack/cell counts, and a bounded exact fiber snapshot. These opt-in diagnostics add filesystem overhead and are not restart certificates or benchmark evidence. Keep this path distinct from `SYMGCAD_PROGRESS_PATH`, which records projection work.

## Problem format

```toml
name = "positive square split by F"
variables = ["x", "y"]
constraints = ["x > 0", "1-x > 0", "y > 0", "1-y > 0"]
split = ["x+y-a"]

[parameters]
a = "1/2"

[solver]
engine = "baseline"
ordering = "given"
order = ["x", "y"]
merge = "none"

[limits]
wall_time_secs = 300
memory_mib = 4096
workers = 1
```

Coefficients and parameter substitutions are exact rationals. Constraints use `<` or `>`; powers accept `^` or `**`. Expressions cannot introduce undeclared symbols or functions. `split` requests the signs of extra polynomials in each returned cell. Omitting it gives the ordinary strict-inequality problem.

Orders are written from the first lifted variable to the last; projection eliminates them in reverse. An explicit `order` fixes the order. Otherwise `ordering` accepts `given`, `mods`, `sotd`, `t1`, `brown`, `sparsity`, or `mcts`. Top-level `variable_groups = [["s", "t"], ["x", "y"]]` keeps the first group outside the second while allowing within-group choices.

`symgcad project` exports projection sets. `symgcad order` reports a chosen order, or the bounded search history for `mcts`. The search uses at most 256 trials by default and reserves most of the one-hour campaign for its selected solve. Timeouts never count as completed decompositions.

The `optimized` engine reuses canonical factorizations, resultants, and projection prefixes. `positivity = true` permits removal of generated factors only with exact nonvanishing certificates; `input_positivity = true` extends this to input factors while retaining coordinate boundaries. `checkpoint_dir = "artifacts/checkpoints"` enables atomic, validated persistent prefixes and individual algebra operations shared by ordering trials. Completed operations survive interruption halfway through a projection stage. MCTS enables this directory automatically and includes its search history and cost in the solve result.

The certificate portfolio includes bounded exact Sturm tests for univariate factors, including open and unbounded domains. Optional `prune_operations = true` attempts a nonvanishing proof before factoring an entire projection operation; it requires positivity and retains explicit original-boundary obligations. It defaults off because the additional proof attempt can cost time without succeeding. Both kinds of omission carry independently replayed evidence.

Bounded square-plus-positive-residual certificates also handle polynomials whose coefficient signs alone are inconclusive. They record the exact square, residual and bound orientations, including affine squares after a homogeneous scale is fixed. Verification reconstructs the identity on the original domain. The algebra backend additionally detects small polynomials depending on two variables through their sum, factors in fewer active variables, and checks the inverse substitution and complete factor product. This preserves the original CAD coordinates; see [the captured A446 factorization experiment](reference/SUM_FACTOR_PRECONDITIONING.md).

By default, `search_pruning = true` abandons ordering trials when completed projection prefixes show severe term growth and avoids repeating that prefix within the current search. These are cost predictions: they never reject a mathematical region, and the final solve does not use them. Set the option to false for the search ablation. When no trial completes, the result stays incomplete and retains a restart candidate. `SYMGCAD_CAPTURE_DIR` optionally records exact operands and timing for the latest uncached operation in each worker; captures are bounded to 16 MiB and can be replayed with `cargo run --example profile_operation` to compare existing Symbolica resultant strategies.

An explicit `homogeneous_block = ["x1", "x2", "x3"]` with `homogeneous_anchor = "x2"` requests positive-scale reduction. The engine checks every constraint and split polynomial, proves the anchor positive, and reconstructs the original cylindrical boundaries. A fixed order must place the anchor before the other block variables. Nonhomogeneous inputs or incompatible fixed orders are rejected.

The `monotone` engine accepts one split polynomial and detects positive-axis variables where exact linear/quadratic coefficient certificates prove strict increase and positive unbounded growth. It solves the endpoint problem using the optimized engine by default; `solver.monotone_base_engine = "local"` selects local projection for that exact base problem. It then reconstructs every positive root sector. Its proof contains the coefficient certificates, exact endpoint identities, and complete base decomposition. Additional constraints involving a peeled variable prevent that peel; fixed orders and variable groups remain binding. This engine composes with homogeneous scale reconstruction.

For an unsplit problem (`split = []`), `engine = "monotone"` transfers the first non-coordinate-affine strict constraint into a complete sign arrangement and retains exactly the required sign cells. For example, `constraints = ["z>0", "z^2+z-1<0"]` decomposes the increasing polynomial `z^2+z-1` and selects its negative cells while preserving the original inequality and empty split annotation. A projection-free check of admissible first peels chooses the split orientation; it respects fixed orders and variable groups. The `sign_filter` proof records the orientation, selected sign, full arrangement, and exhaustive selection map. Verification reconstructs the signed polynomial identity without rerunning that heuristic and preserves all exceptional seams. Historical evidence defaults to its original positive orientation and selection. This wrapper also composes with homogeneous reduction and local endpoint solving. Inputs containing only coordinate-affine or constant constraints receive an explicit applicability error.

For an unsplit monotone problem, the opt-in `solver.monotone_negative_target = true` can solve a strict constraint `Q > 0` directly by orienting `F = -Q`. A certified increasing positive-ray peel reduces the required negative fiber to its strict endpoint inequality, then restores only the interval below its positive root. Separate target evidence verifies the original inequality and exhaustive coverage. Requested homogeneous reduction applies to the original constrained problem before this target transfer. If no admissible target peel exists, the ordinary sign-filter arrangement remains the fallback. Existing split-polynomial arrangements are unchanged. The research campaign helper exposes the same option as `--monotone-negative-target`.

The experimental `local` engine uses a generic-open stack-saturation adaptation of Strzeboński's local projection. Descendant guards refine their owning ancestors, all sibling boundary sets are projected jointly, and the verifier checks the resulting exhaustive tree independently. It supports explicit/given orders and inexpensive Brown/sparsity orders; MCTS supplies explicit trial orders. Other ordering requests have an explicit global fallback. Persistent algebra caches and progress snapshots survive interrupted discovery. With `positivity = true`, exact domain certificates can replace generated guards; their original bound factors remain protected ancestor obligations, proving the certificate's premises throughout the cell. Input factors and singular seams remain protected. The exceptional polynomial list is a global algebraic superset; the local tree records where each excluded section actually applies.

On an axis with an original strict positivity constraint, local projection can use exact quadratic sign cases or Descartes sign variations to avoid unnecessary discriminants. Zero variations imply no positive roots; one variation implies exactly one simple positive root. The latter boundary uses `index_domain = "positive"` and index zero, so collisions among negative roots cannot change its meaning. Other boundaries retain indices among all distinct real roots (the default `index_domain = "real"`). Typed evidence rejects the negative half-line explicitly and propagates every coefficient-sign premise to the ancestors. Inconclusive higher-degree cases retain the ordinary projection obligations. `solver.local_descartes_one = false` disables the one-variation shortcut; `LocalOptions::positive_ray = false` retains the full-real-line path for library ablations.

The experimental `solver.local_descartes_witnesses = true` extends this argument to two through eight positive roots. It requires a Descartes upper bound of exactly that many roots and alternating strict signs at rational separators between the isolated roots. Every coefficient and separator evaluation becomes an ancestor guard, proving that each interval contains exactly one simple positive root throughout the parent cell. These boundaries use positive-root ordinals. The attempt is capped at degree 32 and 4,096 terms, with at most two separator sets per canonical polynomial for the entire solve. Existing sets are reused when valid; unsuccessful or exhausted attempts retain the ordinary discriminant. This finite limit prevents repeated separator selection from creating an infinite sequence of new guards near colliding roots. Separator guards can still increase the number of cells, so the option is off by default and needs workload-specific measurement.

The separate opt-in `solver.local_descartes_intervals = true` also handles unsaturated global Descartes bounds. It covers the entire positive ray by rational intervals, transforms each finite interval to the positive ray, and certifies zero or one sign variation per interval. Strict guards protect every transformed coefficient, finite seam, original endpoint, and leading coefficient. The independent verifier reconstructs these identities and the complete cover from typed evidence. This proves zero through eight simple positive branches without assumptions about negative roots. The attempt retains at most two permanent covers per polynomial, in addition to the two global separator sets if both options are enabled; exhausted or inconclusive attempts retain the ordinary discriminant. Bounds include degree 32, 4,096 input terms, 256 coefficient slots and distinct guards, and 65,536 total guard terms. Rational endpoint height is capped at 256 bits and primitive guard coefficient height at 512 bits. The option remains off by default because the extra ancestor guards can increase the decomposition size.

The separate `solver.local_positive_pair_witnesses = true` option can replace a pair resultant after both polynomials' positive roots have been proved delineable. It protects every adjacent edge belonging to different polynomials with a rational separator and two nonvanishing evaluation guards. The verifier checks the complete positive-root order and each separator with exact root counts. Shared positive roots, failed checks or exhausted template stock retain the ordinary resultant; common negative roots do not invalidate a positive-ray certificate. Each pair has at most two templates for the entire solve. Attempts are limited to degree 32, 4,096 terms per polynomial, 16 positive roots total, eight cross-owner edges and 256 bits for a separator's numerator and denominator combined. This option also defaults to false.

For two positive-root-bearing polynomials, local projection also tests exact leading-term cancellation identities. If every coefficient of a nonzero combination has the same certified sign throughout the ancestor cell, the polynomials cannot share a positive root and their pair resultant can be omitted. At most two cancellations are attempted, with fixed multiplication-work bounds; inconclusive cases retain the resultant. The verifier reconstructs the identities and all sign obligations. Set `solver.local_pair_cancellations = 1` for the one-cancellation ablation, or `LocalOptions::positive_pair_separation = false` to disable the shortcut entirely.

Local discovery can collect obligations from several siblings before rebuilding a stack. Every tentative stack with new cuts is discarded; only a fully saturated pass can produce certified cells. `solver.local_batch_siblings = false` restores one-sibling discovery for comparisons. Exact Sturm counts use a signed integer primitive pseudo-remainder sequence to control coefficient growth; positive normalization preserves every required one-sided endpoint sign.

Completed small local subtrees can be reused at the identical rational prefix within one solve. Their full ancestor obligations are returned and checked on every reuse. The cache retains bounded serialized proofs (at most 128 nodes and 1 MiB per entry, 32 MiB total by default); it does not reuse unfinished branches. Set `solver.local_reuse_subtrees = false` for the ablation. The optional nonvanishing portfolio may use exact sample zeros or opposite signs to abandon an unsuccessful certificate attempt early; samples never certify nonvanishing.

`solver.local_max_refinements` and `solver.local_max_nodes` override the local discovery defaults of 10,000 refinements and 100,000 visited nodes. Exhausting either cap returns an incomplete result. The enclosing process deadline and memory limit remain binding even when these discovery caps are raised.

Unfinished local stacks can retain learned cuts at the identical rational prefix. These are hints: every seeded cut is projected again and every sector rebuilt. The cache bounds entries, polynomial IDs, prefix bit sizes, and retained bytes; it never treats an unfinished stack as a proof. Set `solver.local_reuse_learned_cuts = false` for the same-engine ablation.

The experimental `solver.local_hint_archive = true` persists bounded cut hints under an explicit `solver.checkpoint_dir`. It defaults to false; `hard_probe --local-hint-archive` enables the experiment. Archives use the exact ordered problem and projection options, canonical sparse polynomials and exact rational prefixes. Imported polynomials are checked and factored again, and all stacks are projected again. Proof verification never reads the archive. Each archive is capped at 8 MiB, with 10-second atomic snapshots and a best-effort final snapshot on a soft stop; reports include load, validation and I/O costs. The archive is disabled, with a reported reason, when caches or learned cuts are disabled, or when `local_descartes_witnesses`, `local_descartes_intervals` or `local_positive_pair_witnesses` is enabled: preserving those modes across restarts also requires their permanent template-admission ledger. Ordinary `local_descartes_one` remains compatible. Corrupt, incompatible or oversized archives are ignored. Restart performance remains experimental.

The opt-in `solver.local_stable_samples = true` retains a bounded pool of visited rational samples while one stack is refined at a fixed ancestor prefix. A sample is reused only when exact root comparisons place it strictly inside a new sector; the ordinary child visitor and all proof obligations still run. This can preserve exact-prefix cache reuse when root intervals tighten. Each pool has limits of 256 samples, 32 KiB and 1,024 numerator-plus-denominator bits per sample; inconclusive comparisons fall back to ordinary sampling after 128 refinements. The verifier independently checks strict sector membership. The option defaults to false; `hard_probe --stable-samples` enables the experiment.

`solver.local_subtree_cache_mib` and `solver.local_learned_cut_cache_mib` set retained proof and hint budgets, defaulting to 32 and 8 MiB. Their effective sizes cannot exceed 1/16 and 1/128 of the process memory limit. Larger budgets also scale the total node, entry and ID caps; per-entry bounds stay fixed. Zero disables retention. Reports expose effective limits and eviction counts.

At the first local lifting axis, exact univariate cuts need no lower-dimensional projection. The engine isolates their complete real-root union directly; the verifier checks this zero-dimensional-base condition and every resulting sector. Set `solver.local_base_point_projection = false` for the ablation. Root unions use an ordered queue with exact equality and bisection checks, including coincident or arbitrarily close roots.

## Results and verification

JSON results retain the normalized input, variable order, canonical polynomial table, projection levels, rational sample points, and algebraic boundaries. A boundary references a polynomial and its zero-based index among distinct roots in its `index_domain` (`real` by default), with a rational isolating interval at the sample point. Missing lower/upper boundaries mean negative/positive infinity.

`complete_generic` means complete outside an explicit algebraic exceptional set. Lower-dimensional sections are excluded. Specializing a symbolic answer onto an exceptional parameter value requires recomputation after substitution.

The verifier reconstructs projection obligations and checks an exhaustive lifting trace. It checks all real-root gaps, exact rejection witnesses, accepted cells, and coverage accounting. An incomplete result cannot pass verification. `merge = "aggressive"` additionally produces compatibility-only merged cells; certified raw cells and their seams remain available. A same-sign zero of an integrand must not disappear merely because adjacent signs agree.

Production resultants use Symbolica's deterministic integer-Ducos routine, exact low-degree identities, and bounded bivariate interpolation with enough exact images to prove the full polynomial identity. Automatic CRT dispatch is confined to the profiling example because the current Symbolica checkout permits probabilistic early termination there. The separately replayable upstream sign correction in `patches/` is documented but is not applied to the reference checkout. `comparison::compare_generic` validates small decompositions through a certified common refinement, including differing variable orders and reconstructed scale/monotone cells.

## Reference corpus and benchmarks

The reference checkouts and generated inputs/results are ignored by Git. Revisions and initial corpus hashes are recorded in `PROVENANCE.toml`.

```sh
nix develop .#reference -c python reference/build_reference.py
mkdir -p artifacts/corpus
nix develop .#reference -c python reference/adapter.py extract-core \
  --source DO_NOT_PUSH_REFERENCES/gcad > artifacts/corpus/core-inputs.json
target/release/symgcad corpus --fixed --output artifacts/corpus/fixed
target/release/symgcad corpus --output artifacts/corpus/symbolic \
  --core artifacts/corpus/core-inputs.json
target/release/symgcad bench artifacts/campaigns/core-fast.toml
```

The fixed workload uses `mt2=1,mz2=5/18,s=10,t=-3`. The original symbolic ZZ workload keeps `s,t` symbolic and imposes `s>4,x_i>0`. Both remain distinct completion obligations. The hard topology is A446, with Ax12 as its crossing.

The benchmark runner first freezes reference-completing cases and their orders, then performs repeated fresh-process comparisons. The parity ratio uses matched projection, lifting and merging phases; preparation and process wall times are reported separately. See `reference/README.md` for campaign configuration. Local milestone commits do not imply that unresolved corpus cases have been solved.

`BENCHMARKS.md` records validated milestones, artifact hashes, and the limits of each performance claim. The Nix shell includes `perf` and Valgrind for profiling captured algebra workloads.
