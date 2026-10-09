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
as the original tests do. The Rust supervisor captures wall time and peak RSS;
timeouts and memory failures remain explicit incomplete records.

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
