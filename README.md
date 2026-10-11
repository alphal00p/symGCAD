# symGCAD

Exact generic cylindrical algebraic decomposition in Rust, using Symbolica for all symbolic algebra. symGCAD implements the projection/lifting strategy of [GCAD](https://github.com/magv/gcad), adds certified reductions and bounded ordering search, and exports independently checkable JSON proofs.

The audited checkpoint solves **303/331 distinct original problems**, including **124/124 fixed-kinematics ZZ cases**. The remaining 27 symbolic ZZ problems and positive Gluza33 are explicitly incomplete. A446 and its crossing complete at fixed kinematics; their full symbolic domains remain unresolved.

The [benchmark](benchmark/benchmark.pdf) reports every original problem, including unresolved one-hour attempts and Gluza33's subsequent two-hour timeout. The tracked fast suite passes **653 tests**, with two expensive tests ignored. Completed algebra is retained between bounded campaigns; timeouts and conditional chambers receive no original-completion credit.

The [implementation checkpoint](reference/FINAL_CHECKPOINT.md) records the certified reductions, measured improvements and remaining work. An optional [Symbolica Wang experiment](reference/GLUZA33_WANG_PREFLIGHT.md) factors one captured Gluza33 obstruction in 1.04 seconds against a 60-second control timeout. It remains experimental after a [broader pilot](reference/WANG_ADOPTION_PILOT.md); the production dependency is unchanged.

## Quick start

Requires Git and Nix with flakes enabled. External dependencies are fetched at exact revisions; they are not included in this repository.

```sh
git clone --branch codex/symgcad git@github.com:alphal00p/symGCAD.git
cd symGCAD
scripts/bootstrap-deps.sh
nix develop
cargo build --release --locked
mkdir -p artifacts
target/release/symgcad solve examples/unit_square.toml --output artifacts/square.json
target/release/symgcad verify artifacts/square.json
```

Supply a `SYMBOLICA_LICENSE` environment variable when required by Symbolica's licensing terms. The optional `.symgcad-env` file is ignored by Git. See [reproducible setup](docs/SETUP.md) for licenses, optional upstream patches, native GCAD comparison, and benchmark-document generation.

## Input and commands

```toml
name = "square split by a diagonal"
variables = ["x", "y"]
constraints = ["x > 0", "1-x > 0", "y > 0", "1-y > 0"]
split = ["x+y-1"]

[solver]
engine = "optimized"
ordering = "brown"
positivity = true

[limits]
wall_time_secs = 300
memory_mib = 4096
workers = 1
```

Coefficients are exact rationals. Use `<` or `>` for strict constraints; `split` requests additional polynomial signs. Parameters may be substituted through a `[parameters]` table. Orders run from the first lifted variable to the last; projection eliminates in reverse. An explicit `solver.order` is binding, and `variable_groups = [["s", "t"], ["x", "y"]]` keeps parameters outside the integration variables.

```sh
symgcad solve problem.toml --output result.json
symgcad verify result.json
symgcad project problem.toml --output projection.json
symgcad order problem.toml --output order.json
symgcad bench campaign.toml
```

The binary above is `target/release/symgcad`; add that directory to `PATH` or use the full path. `--help` documents each command. [Detailed usage](docs/USAGE.md) covers all options, proof formats and worked reductions.

For dressed HEPKit DOT graphs, the optional [FastSecDec interface](docs/FASTSECDEC_INTEROP.md)
exports native signed U/F, exact kinematics and edge-to-parameter mappings.
`symgcad import-fastsecdec` checks the bundle and produces an ordinary problem
for solving and verification. Setup can clone a pinned revision of FastSecDec's
separate `no_deformation` branch; FastSecDec is never vendored here. This prepares
graph-driven sign analysis; no-deformation numerical integration remains future work.

## Engines and bounded search

| Engine | Purpose |
|---|---|
| `baseline` | Faithful generic projection/lifting baseline. |
| `optimized` | Exact algebra caches and optional nonvanishing certificates. |
| `local` | Experimental local projection with ancestor refinement and positive-root certificates. |
| `monotone` | Certified linear/quadratic endpoint reduction, including conditional quadratic cases. |
| `linear_negative` | Certified negative-target reduction using positive comparison polynomials. |
| `bilinear_positive` | Certified two-variable positive-target reduction. |
| `quadratic_ladder` | Certified five-sector arrangement when a quadratic opens two positive roots below a linear fiber. |

For bounded MCTS, select `ordering = "mcts"`; `search_time_secs`, `max_rollouts` and the process limits cap exploration. `checkpoint_dir` retains validated algebra work and a restart journal. Trial pruning predicts cost only: it cannot discard a mathematical region or turn a timeout into a proof. Homogeneous scale reduction is explicit and checks the original domain and order obligations.

With `positivity = true`, the experimental `weighted_square_positivity = true` adds bounded, independently replayed weighted-square certificates to optimized projection, ordering, and local guards. It preserves interior zero boundaries; see the [scope and ablations](reference/WEIGHTED_SQUARE_PRODUCTION.md).

The separate opt-in `quadratic_component_positivity = true` extends this portfolio to exact positive-semidefinite quadratic components with a strictly positive residual. Its checker replays rational square identities; a nonnegative expression with an interior zero is insufficient. See the [certificate scope](reference/QUADRATIC_COMPONENT_PRODUCTION.md) and the [quadratic-ladder example](examples/quadratic_ladder.toml).

The experimental solver option `affine_factorization = true` preconditions eligible projection factorizations with exact invertible coordinate changes. It first prefers diagonal scaling to correct degree loss at simple evaluation points while preserving monomial support; integer translation remains available when scaling cannot be admitted. A separate bounded shear search looks for a simpler leading coefficient. Every selected map and complete signed factor product is checked, including units and multiplicities. Projection and fresh verification use the same opt-in policy, preserving original coordinates, domains and interior boundaries. See the [scaling](reference/DIAGONAL_FACTORIZATION_RESEARCH.md) and [shear measurements](reference/SHEAR_FACTORIZATION.md). This option is disabled by default.

With `local_descartes_witnesses = true`, the optional `local_descartes_witness_max_terms = 8192` raises the source-term budget for exact positive-root separator certificates. The default remains 4096, the ceiling is 65536, and discovery, reuse and verification share the explicit limit. All coefficient and separator guards remain mandatory, and the lifetime stock stays at two templates per polynomial. Larger inputs can still make the subsequent projection expensive; see [usage and limits](docs/USAGE.md).

For unsaturated root bounds, `local_descartes_intervals = true` supports the separate `local_descartes_interval_max_terms = 32768`. It raises only source admission; individual guards still have a 4096-term cap, and all existing guard, cover and permanent-template limits remain unchanged. The default source cap remains 4096, with a ceiling of 65536.

`local_descartes_interval_max_total_terms = 131072` separately allows a larger total term count across a cover's distinct canonical guards. Its default is 65536 and ceiling is 262144; individual guard, coefficient, cover and lifetime-template limits stay fixed. The chosen budget is recorded and checked again during verification. See the [guard-cost measurements](reference/GLUZA33_FINAL_INTERVAL_FRONTIER.md).

A `complete_generic` result covers full-dimensional open cells outside its recorded algebraic exceptional set. Lower-dimensional sections are excluded. The verifier reconstructs exact projection/reduction identities, root bounds and exhaustive coverage; an incomplete result cannot pass. Exceptional parameter specializations require a new solve after substitution.

Exact boundary evidence retains ordinary `F=0` sheets, repeated roots and same-sign interior seams. A sign-based reduction requires an exact strict-nonvanishing certificate on the relevant domain; nonnegativity or sampled signs alone are insufficient.

## Results and validation

The deterministic matched-baseline study covers all **331 groups / 579 sign-expanded problems**. Native GCAD completed 483 problems under the recorded limits; symGCAD matched all 483 raw cell counts and independently verified every result. Across three paired measurements, the geometric mean of matched algebra phases was **0.1466× native** (about 6.82× faster). This is a workload-wide average with documented regressions and censored timeouts, not a claim that every case improved.

New reference comparisons are capped at `max(300 s, 3 × symGCAD solve time)`, with shorter campaign limits respected. Native timeouts receive no exact ratio; see the [reference budget policy](reference/REFERENCE_BUDGETS.md).

Later certified reductions complete fixed A381/Ax381 with 836 cells each, negative Gluza30/Gluza33 with one/three cells, and positive Gluza30 with 80,980 cells. Discovery costs, verification costs, failed attempts and exact original-domain checks are recorded separately.

The quadratic ladder also gives five verified cells for the crossed A446 chamber `t < 4`, retaining the original interior `F=0` sheet. Its [conditional proof and supplemental check](reference/QUADRATIC_LADDER_VALIDATION.md) leave the unrestricted symbolic problem unresolved. [Release ablations](reference/QUADRATIC_RELEASE_ABLATION.md) report all 66 fresh solve-and-verify trials, including certificate overhead and cell-count reductions.

Current physics-guided research studies [CFF-informed kinematic chambers](reference/CFF_KINEMATIC_PROJECTION.md), [positive-root critical maps](reference/RATIONAL_KINEMATIC_PAIR_MAP.md), and [two-point matrix pencils](reference/TWO_POINT_MATRIX_PENCIL.md). Exact identity replays support these investigations; full projection and boundary-ordering obligations remain.

The latest investigations identify [an exhausted certificate stock behind Gluza33's discriminant fallback](reference/GLUZA33_WITNESS_STOCK_TRACE.md), [rejected large proofs behind some C470 revisits](reference/C470_REFINEMENT_OBSERVER.md), [graph-based quadratic insertion blocks](reference/CFF_PROJECTION_NEXT_INVESTIGATION.md), and [weighted Hermite queries for selected roots](reference/PARAMETRIC_PROJECTION_NEXT_RESEARCH.md). These diagnostics and proposed improvements add no original-problem completions or benchmark speedups.

The [SubTropica review](reference/TROPICAL_SUBTROPICA_RESEARCH.md) led to a [joint anchor/order experiment](reference/LINEAR_PREFIX_RESEARCH.md): 24 exact two-stage candidates took 3.945 seconds; a further eight candidates completed three linear stages in 2.638 seconds. Their full original solves still timed out, so these ordering proposals establish no new completion or solve-time gain. After a negative [global positivity screen](reference/TROPICAL_CIRCUIT_SCREEN.md), [conditional AM–GM certificates](reference/TROPICAL_CONDITIONAL_CERTIFICATES.md) offer a further direction for authenticated bounded domains. All interior zero boundaries remain required.

A [supplemental seeded sanity check](reference/SUPPLEMENTAL_SANITY.md) sampled two fresh interior points in each fixed A446 cell: all 8 cells, 16 exact F evaluations and 112 original-constraint checks passed. This check recomputes boundaries at each new prefix and is excluded from benchmark timings; it does not replace the coverage proof.

- [Benchmark document](benchmark/benchmark.pdf) and [Typst source](benchmark/benchmark.typ): per-problem comparison and methodology.
- [Ordering and MCTS companion](benchmark/ordering-mcts.pdf): search mechanics and evidence.
- [Validation record](BENCHMARKS.md): measured milestones, limits, artifact hashes and unresolved cases.
- [Reference reproduction](reference/README.md): corpus extraction, native adapter and audit protocols.
- [Implementation plan](SYMGCAD_PLAN.MD) and [algorithm research](ALGORITHM_RESEARCH.md): design, evidence and remaining work.

Run the fast regression suite with `scripts/test.sh fast --locked`. The separate [slow corpus runner](reference/SLOW_SUITE.md) is `scripts/test.sh slow --help`; it gives each unresolved original up to one hour, with explicit memory limits and independent verification. For longer individual investigations, problem and campaign budgets accept up to 7,200 seconds explicitly; solve and verification share the campaign deadline. Defaults remain one hour, with a 64 GiB / 16-worker ceiling. The classification distinguishes historical verified completions from unresolved workloads; it does not promise that all historically completed cases run quickly. Research helpers under `examples/` have additional focused tests. Large proof files, private licenses and external source checkouts remain ignored by Git.
