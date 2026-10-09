# Native upstream comparison

The Python files in this directory only build/invoke the independent upstream
GCAD oracle and extract its test inputs. symGCAD's production parser, algebra,
solver, resource supervisor and benchmark records are Rust.

Build the unmodified C++/Cython sources against Nix FLINT/GMP/MPFR:

```sh
nix develop .#reference -c python reference/build_reference.py
```

The isolated `.reference/gcad` copy contains generated extension files. The
original `DO_NOT_PUSH_REFERENCES/gcad` remains unchanged. Full source SHA-256
hashes, the upstream revision, Python/compiler versions and release flags are
saved to `.reference/provenance.json`. The upstream setup.py dependency download
hook is bypassed because Nix already supplies those libraries.

Extract core mathematical inputs with the reference environment:

```sh
nix develop .#reference -c python reference/adapter.py extract-core \
  --source DO_NOT_PUSH_REFERENCES/gcad > artifacts/corpus/core-inputs.json
```

The extraction intercepts GCAD/order/projection calls and preserves source test
identifiers. It includes disabled examples and does not run their expensive
algebra. Assertions involving GCAD calls still contribute their problem inputs;
assertions examining returned cells are not input definitions.

The Rust importer parses two-loop sparse literal fixtures without executing
Python. Generated one-loop fixtures use the exact one-loop Symanzik identity.
All fixture identifiers remain present, including power variants. Algebraic
deduplication must establish equivalence with Symbolica and retain aliases.

Inventory categories at upstream revision
`fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`:

| Category | Count |
| --- | ---: |
| Collected core tests | 51 |
| Collected ZZ tests | 115 |
| Exact rational helper tests | 6 |
| Native conversion tests | 5 |
| Deliberately disabled core tests | 3 |
| Literal two-loop fixture files | 265 |
| Generated one-loop fixture files | 12 |

The original ZZ fixtures keep `s,t` symbolic, substitute `mt2=1,mz2=5/18`,
and impose `s>4,x_i>0`. The fixed workload additionally uses `s=10,t=-3`.
ZZ tests only check completion, so their results are compatibility observations,
not independent mathematical certificates.

Generated `finiteEx12` scripts contain an overlapping `finiteE` substring
condition before their crossed condition. The importer preserves the branch
actually selected by the source and records this in the inventory; it does not
silently change the fixture's mathematical input.

For same-algorithm timing, pin the same order, merge policy, affinity and memory
budget on both engines. The adapter separately records ordering, projection,
lifting and merging times, and intentionally repeats projection for both signs
as the original tests do. The Rust supervisor captures wall time, `wait4` user
and system CPU time, and peak RSS; timeouts and memory failures remain explicit
incomplete records. RSS sampling follows every worker thread's children across
nested process groups, without scanning unrelated host processes. These
controlled workers retain their parent tree and use parent-death cancellation;
an arbitrary executable that rapidly orphans descendants is outside this
supervisor's process-accounting contract.

A benchmark configuration placed at the repository root can contain:

```toml
cases = ["examples/unit_square.toml"]
output_dir = "artifacts/bench/smoke"
reference_root = ".reference/gcad"
adapter = "reference/adapter.py"
repeats = 3
reference_timeout_secs = 300.0
solve_timeout_secs = 300.0
campaign_time_secs = 3600.0
workers = 1
memory_mib = 65536
```

Run `symgcad bench campaign.toml`. All paths are relative to the configuration
file. The Python interpreter is read from `.reference/provenance.json`, or can
be supplied explicitly as `reference_python`.

Qualification runs on the independent reference before observing any Rust
timings. Its complete set and selected orders are frozen to JSON. Both engines
then run three fresh-process measurements at the same fixed order. Split
polynomials are expanded into individual positive/negative problems so the
baseline comparison does not credit shared projection work against duplicate
reference work. The report includes medians and geometric means for both wall
time and algorithm time, preparation time, peak RSS and raw-cell counts. A Rust timeout stays in the
frozen set and prevents a passing parity claim. The parity gate uses the same
projection + lifting + merging phase sum on both engines (target 1.25).
Parsing, normalization and initial CAS setup are reported separately, and
fresh-process wall time includes language startup. Startup savings therefore
cannot hide an algebra regression. Raw-cell differences are flagged for
independent verification.

Every completed Rust case is verified once through a separate bounded worker
after its measured repetitions. Verification consumes the campaign budget but
is excluded from solve timings; the parity gate also requires verification.
The exact-equivalence manifest retains every input alias. Equivalence keys use
Symbolica-normalized polynomials after substitutions and preserve variables,
grouping, ordering and solver options, so different ordering workloads are not
silently combined.

Completion and ablation ledgers use a separate configuration mode, because
certified preprocessing or a changed order can legitimately change cell counts:

```toml
mode = "completion" # use "ablation" for three measured repetitions
cases = ["examples/unit_square.toml"]
output_dir = "artifacts/completion/smoke"
equivalence_manifests = []
solve_timeout_secs = 60.0
verify_timeout_secs = 60.0
campaign_time_secs = 3600.0
workers = 1
memory_mib = 8192
resume = true
cache_policy = "auto" # cold for ablations; shared for completion

[[variants]]
name = "optimized-mods"
[variants.solver]
engine = "optimized"
ordering = "mods"
```

Each solver variant overrides only the specified input options. The ledger
measures a fresh public `solve` process, including order search, preprocessing,
normalization, decomposition and export. It independently verifies every
completed repetition. Verification time is recorded separately. Completion
runs use one repetition; ablations default to three. These runs certify results
without requiring the same number of cells as the upstream engine.

Ablations default to a fresh persistent-cache directory for each repetition,
including when the source input names an existing checkpoint directory. Ordering
trials and the final solve share work within that one measurement. The ledger
records each actual trial input and its hash. Set `cache_policy = "shared"`
explicitly for warm-cache measurements, or `"cold"` to isolate completion runs
as well. Shared results must be labelled as warm; a new process alone does not
make a persistent algebra cache cold.

`ledger.json` retains every scheduled identifier, equivalence alias and attempt,
including timeouts, resource failures and inputs not started within the budget.
Resume skips a case only if its input and executable hashes match, all required
repetitions were verified, and the result and verification artifacts still match
their recorded hashes. Changed binaries rerun the case. The history retains old
attempts. Each campaign snapshots its executable, so rebuilding the working tree
cannot change later trials. Optional equivalence manifests must use the Rust
importer's schema.

Full-corpus completion is a mathematical solve workload. In particular, solving
an inequality extracted from an ordering/projection unit test does not reproduce
that test's original API assertion; the corpus inventory retains this distinction.

The separate API comparison invokes all 22 extracted ordering calls and the one
projection call through both implementations. Projection comparison canonicalizes
the per-level polynomial sets with Symbolica; ordering comparisons retain the
actual returned variable lists. Resource failures stay explicit in the report:

```sh
nix develop -c cargo run --example api_compare -- \
  --solver artifacts/bin/symgcad-baseline-3125469
```

To replay a previously frozen reference-completing set after an infrastructure
fix, without changing its denominator or orders, use the Rust replay driver:

```sh
nix develop -c cargo run --example frozen_replay -- \
  --frozen artifacts/bench/zz-symbolic-completing-release/frozen-reference-set.json \
  --solver artifacts/bin/symgcad-baseline-buffered-3125469 \
  --output artifacts/bench/zz-symbolic-buffered-release
```

The output directory must be empty. The replay records the frozen manifest's
hash, runs three paired measurements and independently verifies each complete
case. Its kernel and fresh-process timing policies match the original baseline
campaign. It preserves the frozen worker input schema so new CLI options cannot
silently alter the committed baseline's mathematical workload.

To qualify a new cohort with the current supervisor before selecting a release
worker, run the standalone Rust oracle driver:

```sh
nix develop -c cargo run --example qualify_reference -- \
  --config artifacts/campaigns/core-remaining-release.toml \
  --output artifacts/bench/core-remaining-clean-qualification
```

Its frozen manifest records every candidate, including native timeouts, and the
remaining campaign budget. Supply that manifest to `frozen_replay`, with
`--campaign-time-secs` no larger than the recorded remaining budget. A separate
`--verification-timeout` can give exhaustive proof checking more time than each
measured solve while remaining inside the shared campaign limit.

`cargo run --example aggregate_parity -- --report REPORT_JSON` accepts repeated
`--report` arguments for the final corpus audit. It maps aliases through both
equivalence manifests and requires all 331 groups: 124 fixed ZZ, 124 symbolic ZZ,
and 83 core groups. Sign expansion gives 579 solve problems. The 83 core groups
include 70 original solve calls and 13 ordering-only inputs treated as additional
solve problems; their original ordering assertions remain separate API checks.

Supply the fast-core, remaining-core, supplemental API-input, active ZZ, and all
three remaining ZZ batch reports for each kinematic domain. Do not include the
aggressive-merge replay or API comparison in this aggregate: they check separate
assertions and would duplicate solve groups. The audit rejects duplicate groups
and mixed worker executables. Missing qualification coverage or missing timing
ratios suppress the aggregate geometric mean and prevent a passing gate. Native
timeouts stay explicit. The output records every report and manifest hash, all
aliases, missing cases, and every matched-phase slowdown above two times native.

Reproduce the completed immutable `9025e71` parity aggregation from the retained
reports with:

```sh
nix develop -c cargo run --example aggregate_parity -- \
  --report artifacts/bench/core-fast-deterministic-release/report.json \
  --report artifacts/bench/core-remaining-deterministic-release/report.json \
  --report artifacts/bench/core-api-only-solves-deterministic-release/report.json \
  --report artifacts/bench/zz-fixed-completing-deterministic-release/report.json \
  --report artifacts/bench/zz-fixed-remaining-batch1-deterministic-release/report.json \
  --report artifacts/bench/zz-fixed-remaining-batch2-extended-deterministic-release/report.json \
  --report artifacts/bench/zz-fixed-remaining-batch3-deterministic-release/report.json \
  --report artifacts/bench/zz-symbolic-deterministic-release/report.json \
  --report artifacts/bench/zz-symbolic-remaining-batch1-deterministic-release/report.json \
  --report artifacts/bench/zz-symbolic-remaining-batch2-deterministic-release/report.json \
  --report artifacts/bench/zz-symbolic-remaining-batch3-deterministic-release/report.json \
  --output artifacts/bench/parity-aggregate-9025.json
```

The extended fixed batch 2 preserves its entire original qualification set and
uses 120-second paired solve limits; the earlier 60-second failures remain in
the adjacent non-extended report. Do not supply both versions to aggregation.

Full-problem completion is audited separately from parity. The following audit
streams large result files, checks retained executable/input/result/verification
hashes, and compares exact normalized original constraints, the complete ordered
split list, variables, variable groups, and explicit fixed-order obligations.
It allows different solver strategies while preserving every failed attempt.
Supplemental verification records are linked to a result hash in the original
ledger. This validates retained verification evidence; it does not rerun the
mathematical checker. Separate positive/negative parity proofs never substitute
for one proof of the original full split problem.

```sh
nix develop -c cargo run --example aggregate_completion -- \
  --ledger artifacts/completion-deterministic/fixed/results/ledger.json \
  --ledger artifacts/completion-deterministic/symbolic/results/ledger.json \
  --ledger artifacts/completion-unscaled-retry-01/results/ledger.json \
  --ledger artifacts/completion-baseline-retry-02/results/ledger.json \
  --supplemental artifacts/completion-deterministic/extra-verifications/report.json \
  --output artifacts/completion-aggregate-296.json
```

Add further `--ledger` arguments for completed retries. The report includes all
331 expected groups, their aliases, proof provenance, and an explicit unresolved
list; `all_complete` stays false until every group has matching valid evidence.

For unresolved problems with a verified full unsplit baseline result, or split
problems whose two baseline sign branches both completed, prepare a new
full-problem campaign using the frozen baseline variable order:

```sh
nix develop -c cargo run --example prepare_completion_retry -- \
  --completion artifacts/completion-aggregate-294.json \
  --parity artifacts/bench/parity-aggregate-9025.json \
  --output artifacts/completion-baseline-retry-02
nix develop -c taskset -c 0 artifacts/bin/symgcad-fast-lifting-1d15d03 bench \
  artifacts/completion-baseline-retry-02/campaign.toml
```

This keeps the original constraints and split polynomial, respects original
groups and explicit fixed orders, disables reductions, and uses one worker,
8 GiB, 120-second solves, 240-second verifications, and a shared one-hour limit.
Only a new independently verified full-problem result can add a completion.
If no unresolved input is eligible, preparation records the skipped groups and
does not emit an empty campaign configuration.

Research-only strict subdomain probes use `core_piece_probe`. Their plan is a
JSON array of `{input, extra_constraints, label}` objects; each input is an
existing TOML problem, and restrictions such as `"x0>1"` are appended unchanged.
Every completed piece is independently verified and every timeout is retained.
The helper permits at most 20 seconds per solve/verification and 120 seconds
for the whole run, on one worker with an 8 GiB address-space cap. Its reports
explicitly claim zero original completions and must not enter the full-problem
completion aggregate without a separately verified exhaustive union proof.

```sh
nix develop -c cargo run --example core_piece_probe -- \
  --executable artifacts/hard-campaigns/sign-filter-3ddfe2a-debug/symgcad \
  --plan artifacts/profile/core-piece-candidates/plan.json \
  --output artifacts/research/core-pieces-replay \
  --campaign-secs 120 --trial-secs 20
```

`monotone_candidates --input INPUT --extra-constraint 'x0<1'` checks both target
orientations against exact coefficient certificates without running projection.
Repeat `--extra-constraint` for joint restrictions. An uncertified sign is
reported as unknown; it does not prove that the coefficient changes sign.

`crossing_identity` compares a selected symbolic ZZ original against verified
same-size donors from the completion audit using exact Symbolica polynomial
equality, rational content normalization, compatible propagator permutations,
both overall polynomial signs, and the crossing `t -> 5/9-s-t`. Matches record
the required split-sign orientation. It also checks the transformed strict domain and
original fixed-order/group obligations. Matches emit retry inputs, never reused
completion certificates. For example:

```sh
nix develop -c cargo run --example crossing_identity -- \
  --audit artifacts/completion-aggregate-296.json \
  --target finiteC_6_470_ --seconds 30 \
  --output artifacts/c470-crossing-replay
```

The C438/Cx438 positive control finds the expected crossing. The retained
all-27 unresolved batch and its hashes are documented in `BENCHMARKS.md`.
The subsequent search including both polynomial orientations also found zero
matches for all 27 unresolved originals (0.317 seconds combined polynomial
work); every search finished. Its summary is
`artifacts/crossing-all-symbolic-signed-02/summary.json`, SHA-256
`68ed9a6eaf9194617fc3d2a391d56fd749638e098e1762df812a231e928f0f1d`.
The frozen helper SHA-256 is
`8fd22238bd0f86bd537b3cf3c6247b94898ca88741c3d824b379b9909d38c4fd`.
Its regression checks the exact combination of negative rational content,
crossing, and propagator permutation; the C438/Cx438 control again finds four
matches. No additional full-problem completion is claimed.
