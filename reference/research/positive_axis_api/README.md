# Isolated positive-axis API experiment

This package reconstructs the research helper and two separate Symbolica
patches. It is outside Cargo's automatic `examples/` discovery because the
ordinary pinned backend has no positive-axis API. It does not change production
root selection, a CAD certificate, or the active dependency checkout. Measurement
results and promotion decisions belong to the accompanying research record;
the presence of this package is not a successful-measurement claim.

`symbolica-positive-axis.patch` adds strictly positive root isolation for
integer and rational univariate polynomials, retaining the existing full-real
entry point. `symbolica-unit-shift.patch` changes the existing Taylor shift's
unit-shift recurrence from fused multiplication by one to addition. The files
are separate so their mathematical changes remain reviewable. Each applies
independently to the pristine pinned backend. The replay chooses a fixed order
(positive-axis, then unit-shift) and applies both, matching the combined
isolated measurement. It does not attribute a combined speed change
to either patch individually.

## Prepare the pinned workspace

Run the repository's normal dependency setup first if the pinned Symbolica Git
objects are missing: `bash scripts/bootstrap-deps.sh`. Do not request optional
production patches for this experiment. Then, from the repository root:

```sh
bash reference/research/positive_axis_api/setup.sh /absolute/new/replay
```

The destination must not exist. Setup archives historical public symGCAD commit
`2d1b630a1170e05d3fb276bd8bc8dd0464bda4ca`, whose exact source tree is
`1e1cae62b867d7f84da47f3322309629811144f3`, and Symbolica
`75f8350094b90254ee71dc2a391fde0d14b0204a`, including the pinned
Numerica/Graphica sources. Its equivalent original local commit is accepted
only with the same exact tree. A shallow clone missing both must first fetch
the public commit from origin; setup itself does not fetch.

This retains the historical Cargo manifest and lockfile, including their local
backend paths. The current standalone public-Cargo packaging is deliberately
not substituted into the old measurement. Setup checks the packaged hashes and
the four unchanged frozen dependency/source hashes against that historical
tree, requires a pristine active Symbolica checkout, then applies only the two
research patches in the new tree. `REPLAY_PROVENANCE.txt` records the replay
revision/tree and current packaging revision/hashes separately. Unrelated work
in the active symGCAD checkout is never archived or changed. There is no build,
license printing, or mutation of the active checkout.
The helper imports the archived `src/roots/integer_sign.rs` directly and records
its hash; no integer-sign implementation is duplicated.

## Compile and test

Configure the Symbolica license as for normal repository use. For a local
license environment file, source it without printing its contents. Use the
isolated tree's own target directory; sharing a target can invalidate another
checkout's Cargo dependency fingerprints.

```sh
cd /absolute/new/replay
# If needed: source /absolute/private/license-environment-file
nix develop path:. -c cargo test --locked -j1 --example positive_axis_probe --example unit_shift_regression
nix develop path:. -c cargo build --locked -j1 --example positive_axis_probe
sha256sum target/debug/examples/positive_axis_probe
```

The inherited development profile builds dependencies at optimization level 2.
Use the same profile and CPU for timed full/positive comparisons. Build under
an external wall-time/memory supervisor appropriate to the machine; native
dependency compilation is not governed by the helper's runtime supervisor.
These commands run eleven positive-axis helper/shared-sign tests and two public
unit-shift regression tests; they do not run the dependency's internal test
suite. The tests cover integer/rational APIs, zero and constant inputs, repeated,
irrational and close roots, negative/zero omissions, exact matching, malformed
literal input, and cross-square-free singleton containment. An ordered/disjoint
interval failure is a failed gate: retain it and investigate the backend;
do not relax the check or report a successful API result.

## Generic literal input and measurements

An input is a bounded JSON object with schema 1, a complete variable-name map,
one active axis, and nonzero integer coefficients in strictly increasing
exponent order. Every inactive-axis exponent must be zero. Expressions such as
`2^1000` are rejected; write their exact integer value. For example, this is
`s^2 - 2` with an inactive parameter retained in the map:

```json
{"schema_version":1,"variables":["s","t"],"axis":0,"terms":[{"coefficient":"-2","exponents":[0,0]},{"coefficient":"1","exponents":[2,0]}]}
```

Limits are 32 MiB input, degree 4096, coefficient height 20000 bits and a
64-million-bit conservative input-storage estimate. The parser validates
literal grammar and support before constructing algebra objects. Each normal
invocation launches a supervised single-CPU worker with a 4 GiB aggregate
memory limit and wall cap at most 1800 seconds. Choose an allowed CPU, keep it
the same across runs, and use fresh report paths. For example:

```sh
input_file=/absolute/input.json
input_hash=$(sha256sum "$input_file" | cut -d ' ' -f1)
probe=/absolute/new/replay/target/debug/examples/positive_axis_probe
"$probe" "$input_file" --input-sha256 "$input_hash" --mode full --ring rational --cpu 2 --seconds 1800 --output /absolute/full.json
"$probe" "$input_file" --input-sha256 "$input_hash" --mode positive --ring rational --cpu 2 --seconds 1800 --output /absolute/positive.json
full_hash=$(sha256sum /absolute/full.json | cut -d ' ' -f1)
positive_hash=$(sha256sum /absolute/positive.json | cut -d ' ' -f1)
"$probe" "$input_file" --input-sha256 "$input_hash" --mode compare --cpu 2 --seconds 1800 --left /absolute/full.json --left-sha256 "$full_hash" --right /absolute/positive.json --right-sha256 "$positive_hash" --output /absolute/comparison.json
```

`--ring integer` selects the integer API; `--ring rational` converts the same
literal polynomial to rational coefficients before timing the rational API.
Each report binds the exact input bytes, reconstructed literal expression,
executable, and shared sign-evaluator source. It saves every interval and native
root multiplicity. `native_isolation_seconds` times only the API call, excluding
input preparation, coefficient conversion, interval validation and output.
A sibling `*.process.json` records the supervised process outcome. Do not use
the internal `--worker` option for measurement.

Two full reports automatically compare **all real roots**, including zero.
If either report is positive-only, comparison selects the strictly positive
subset. Pair the first full report with another full report to test full-mode
preservation. Before selecting that subset, comparison resolves raw native
zero endpoints with the existing shared exact open-root counter and each
source inventory's full or positive ordinal. An endpoint may be the selected
rational root or a different root: endpoint evaluation alone cannot decide
ownership. The report saves each normalization decision and its separate
timing. Nonzero endpoints take the cheap path without a root-count computation.
Remaining intervals crossing zero are rejected instead of classified by a
sample. A successful match requires an exact rational root or opposite nonzero
signs at the endpoints of the interval intersection, with bounded refinement;
overlap alone is insufficient. This relies on each source isolator's certified
unique-root interval contract and is not an independent large-degree root count.

When supplied native multiplicities indicate repeated real roots, comparison
computes and times one exact square-free reduction before sign matching. With
all real multiplicities one, it uses the original polynomial; repeated nonreal
factors do not impair real sign changes. The helper also imports the historical
schema-1 `native_single_block_root_inventory` format after verifying its full
variable map, degree and literal-polynomial hash. That format does not save
internal root multiplicities: comparisons explicitly leave multiplicity equality
unverified, and never substitute its outer block exponent. An empty finite
inventory for the zero polynomial is not a claim that it has no zeros.
