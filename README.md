# symGCAD

Exact generic cylindrical algebraic decomposition in Rust, using Symbolica for all symbolic algebra. symGCAD implements the projection/lifting strategy of [GCAD](https://github.com/magv/gcad), adds certified reductions and bounded ordering search, and exports independently checkable JSON proofs.

The audited checkpoint solves **303/331 distinct original problems**, including **124/124 fixed-kinematics ZZ cases**. The remaining 27 symbolic ZZ problems and positive Gluza33 are explicitly incomplete. A446 and its crossing complete at fixed kinematics; their full symbolic domains remain unresolved.

The [benchmark](benchmark/benchmark.pdf) reports every original problem, including unresolved one-hour attempts and Gluza33's subsequent two-hour timeout. The tracked fast suite passes **662 tests**, with two expensive tests ignored. Completed algebra is retained between bounded campaigns; timeouts and conditional chambers receive no original-completion credit.

The [implementation checkpoint](reference/FINAL_CHECKPOINT.md) records the certified reductions, measured improvements and remaining work. The latest optional [factorization-axis experiment](reference/GLUZA33_FINAL_FACTOR_PUSH.md) clears a captured Gluza33 obstruction: its 1,731-term component factors in 0.280 seconds, and the full 2,867-term source in 1.951 seconds, with exact signed reconstruction. These are isolated algebra measurements; the production dependency is unchanged.

The detailed [HANDOFF.md](HANDOFF.md) covers the complete implementation, failed and successful investigations, CFF/graph and tropical research, unresolved proof obligations, and reproduction and resumption instructions.

## Quick start

Requires Git and Nix with flakes enabled. Cargo fetches the public dependencies at the locked revisions; no source bootstrap or private checkout is required.

```sh
git clone --branch codex/symgcad git@github.com:alphal00p/symGCAD.git
cd symGCAD
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
| `bilinear_general` | Certified two-variable positive-target reduction over a verified partition of the base into independently solvable pieces (`symgcad pieces plan/assemble`). Opt-in `projective_sections = true` (an explicit contract extension, default off) lets upper sections reach +infinity and then needs only the single piece `{d>0}`; see [docs/USAGE.md](docs/USAGE.md#projective-sections-opt-in-contract-extension). |
| `quadratic_ladder` | Certified five-sector arrangement when a quadratic opens two positive roots below a linear fiber. |

For bounded MCTS, select `ordering = "mcts"`; `search_time_secs`, `max_rollouts` and the process limits cap exploration. `checkpoint_dir` retains validated algebra work and a restart journal. Trial pruning predicts cost only: it cannot discard a mathematical region or turn a timeout into a proof. Homogeneous scale reduction is explicit and checks the original domain and order obligations.

Optional improvements include exact positivity certificates, invertible factorization preconditioning, positive-root Descartes covers and completed-proof caches. Each retains its mathematical checks and explicit resource limits. [Detailed usage](docs/USAGE.md) describes the switches; the [handoff](HANDOFF.md) records measured benefits, failed trials and promotion requirements.

A `complete_generic` result covers full-dimensional open cells outside its recorded algebraic exceptional set. Lower-dimensional sections are excluded. The verifier reconstructs exact projection/reduction identities, root bounds and exhaustive coverage; an incomplete result cannot pass. Exceptional parameter specializations require a new solve after substitution.

Exact boundary evidence retains ordinary `F=0` sheets, repeated roots and same-sign interior seams. A sign-based reduction requires an exact strict-nonvanishing certificate on the relevant domain; nonnegativity or sampled signs alone are insufficient.

## Results and validation

The deterministic matched-baseline study covers all **331 groups / 579 sign-expanded problems**. Native GCAD completed 483 problems under the recorded limits; symGCAD matched all 483 raw cell counts and independently verified every result. Across three paired measurements, the geometric mean of matched algebra phases was **0.1466× native** (about 6.82× faster). This is a workload-wide average with documented regressions and censored timeouts, not a claim that every case improved.

New reference comparisons are capped at `max(300 s, 3 × symGCAD solve time)`, with shorter campaign limits respected. Native timeouts receive no exact ratio; see the [reference budget policy](reference/REFERENCE_BUDGETS.md).

Later certified reductions complete fixed A381/Ax381 with 836 cells each, negative Gluza30/Gluza33 with one/three cells, and positive Gluza30 with 80,980 cells. Discovery costs, verification costs, failed attempts and exact original-domain checks are recorded separately.

The quadratic ladder also gives five verified cells for the crossed A446 chamber `t < 4`, retaining the original interior `F=0` sheet. Its [conditional proof and supplemental check](reference/QUADRATIC_LADDER_VALIDATION.md) leave the unrestricted symbolic problem unresolved. [Release ablations](reference/QUADRATIC_RELEASE_ABLATION.md) report all 66 fresh solve-and-verify trials, including certificate overhead and cell-count reductions.

The [handoff](HANDOFF.md) covers CFF threshold chambers, graph-derived quadratic blocks, selected-root proofs, Borinsky/SubTropica sampling and region analysis. These studies produced exact identities and useful ordering proposals, but no additional full symbolic completion. Physical threshold intuition and tropical support alone never justify removing an interior boundary.

A [supplemental seeded sanity check](reference/SUPPLEMENTAL_SANITY.md) sampled two fresh interior points in each fixed A446 cell: all 8 cells, 16 exact F evaluations and 112 original-constraint checks passed. This check recomputes boundaries at each new prefix and is excluded from benchmark timings; it does not replace the coverage proof.

- [Benchmark document](benchmark/benchmark.pdf) and [Typst source](benchmark/benchmark.typ): per-problem comparison and methodology.
- [Ordering and MCTS companion](benchmark/ordering-mcts.pdf): search mechanics and evidence.
- [Validation record](BENCHMARKS.md): measured milestones, limits, artifact hashes and unresolved cases.
- [Reference reproduction](reference/README.md): corpus extraction, native adapter and audit protocols.
- [Implementation plan](SYMGCAD_PLAN.MD) and [algorithm research](ALGORITHM_RESEARCH.md): design, evidence and remaining work.

Run the fast regression suite with `scripts/test.sh fast --locked`. The separate [slow corpus runner](reference/SLOW_SUITE.md) is `scripts/test.sh slow --help`; it gives each unresolved original up to one hour, with explicit memory limits and independent verification. For longer individual investigations, problem and campaign budgets accept up to 7,200 seconds explicitly; solve and verification share the campaign deadline. Defaults remain one hour, with a 64 GiB / 16-worker ceiling. The classification distinguishes historical verified completions from unresolved workloads; it does not promise that all historically completed cases run quickly. Research helpers under `examples/` have additional focused tests. Large proof files, private licenses and external source checkouts remain ignored by Git.
