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

By default, `search_pruning = true` abandons ordering trials when completed projection prefixes show severe term growth and avoids repeating that prefix within the current search. These are cost predictions: they never reject a mathematical region, and the final solve does not use them. Set the option to false for the search ablation. When no trial completes, the result stays incomplete and retains a restart candidate. `SYMGCAD_CAPTURE_DIR` optionally records exact operands and timing for the latest uncached operation in each worker; captures are bounded to 16 MiB and can be replayed with `cargo run --example profile_operation` to compare existing Symbolica resultant strategies.

An explicit `homogeneous_block = ["x1", "x2", "x3"]` with `homogeneous_anchor = "x2"` requests positive-scale reduction. The engine checks every constraint and split polynomial, proves the anchor positive, and reconstructs the original cylindrical boundaries. A fixed order must place the anchor before the other block variables. Nonhomogeneous inputs or incompatible fixed orders are rejected.

The `monotone` engine accepts one split polynomial and detects positive-axis variables where exact linear/quadratic coefficient certificates prove strict increase and positive unbounded growth. It solves the endpoint problem using the optimized engine, then reconstructs every positive root sector. Its proof contains the coefficient certificates, exact endpoint identities, and complete base decomposition. Additional constraints involving a peeled variable prevent that peel; fixed orders and variable groups remain binding. This engine composes with homogeneous scale reconstruction.

The experimental `local` engine uses a generic-open stack-saturation adaptation of Strzeboński's local projection. Descendant guards refine their owning ancestors, all sibling boundary sets are projected jointly, and the verifier checks the resulting exhaustive tree independently. It supports explicit/given orders and inexpensive Brown/sparsity orders; MCTS supplies explicit trial orders. Other ordering requests have an explicit global fallback. Persistent algebra caches and progress snapshots survive interrupted discovery. With `positivity = true`, exact domain certificates can replace generated guards; their original bound factors remain protected ancestor obligations, proving the certificate's premises throughout the cell. Input factors and singular seams remain protected. The exceptional polynomial list is a global algebraic superset; the local tree records where each excluded section actually applies.

## Results and verification

JSON results retain the normalized input, variable order, canonical polynomial table, projection levels, rational sample points, and algebraic boundaries. A boundary references a polynomial and its zero-based index among distinct real roots on the current fiber, with a rational isolating interval at the sample point. Missing lower/upper boundaries mean negative/positive infinity.

`complete_generic` means complete outside an explicit algebraic exceptional set. Lower-dimensional sections are excluded. Specializing a symbolic answer onto an exceptional parameter value requires recomputation after substitution.

The verifier reconstructs projection obligations and checks an exhaustive lifting trace. It checks all real-root gaps, exact rejection witnesses, accepted cells, and coverage accounting. An incomplete result cannot pass verification. `merge = "aggressive"` additionally produces compatibility-only merged cells; certified raw cells and their seams remain available. A same-sign zero of an integrand must not disappear merely because adjacent signs agree.

Production resultants explicitly use Symbolica's deterministic integer-Ducos routine, with exact low-degree identities. Automatic CRT dispatch is confined to the profiling example because the current Symbolica checkout permits probabilistic early termination there. The separately replayable upstream sign correction in `patches/` is documented but is not applied to the reference checkout. `comparison::compare_generic` validates small decompositions through a certified common refinement, including differing variable orders and reconstructed scale/monotone cells.

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
