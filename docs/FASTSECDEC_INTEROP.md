# Graph input through FastSecDec

FastSecDec owns the dressed HEPKit DOT graph, momentum routing, masses and
external Gram products. Its separate
[`no_deformation` branch](https://github.com/alphal00p/fastSecDec/tree/no_deformation) exports the native
Symanzik polynomials and graph metadata for symGCAD. This interface prepares
homogeneous sign analysis before sector decomposition. It does not implement
the no-deformation integration method.

FastSecDec remains an external repository. Optional setup fetches the exact
tested commit from that branch recorded in `PROVENANCE.toml`:

```sh
scripts/bootstrap-deps.sh --fastsecdec
nix develop
cargo build --release --locked
cargo build --locked --manifest-path DO_NOT_PUSH_REFERENCES/fastSecDec/Cargo.toml \
  -p fastsecdec-cli
```

The two executables use their own locked dependency trees and communicate
through versioned JSON and TOML. FastSecDec is not required for ordinary
symGCAD polynomial inputs or the fast regression suite. Setup rejects an
existing checkout at a different revision or containing unrelated edits;
it never resets development work.

## Export, import, solve and verify

From the symGCAD root, export the massive bubble directly from its graph:

```sh
mkdir -p artifacts
DO_NOT_PUSH_REFERENCES/fastSecDec/target/debug/fastsecdec export-symanzik \
  DO_NOT_PUSH_REFERENCES/fastSecDec/examples/no_deformation/bubble.toml \
  --kinematic-constraint 's > 4' --output artifacts/bubble-symbolic
target/release/symgcad import-fastsecdec artifacts/bubble-symbolic/symanzik.json \
  --problem artifacts/bubble-symbolic/problem.toml \
  --output artifacts/bubble-checked.toml
target/release/symgcad solve artifacts/bubble-checked.toml \
  --output artifacts/bubble-result.json
target/release/symgcad verify artifacts/bubble-result.json
```

Export and checked-import destinations must be fresh. To specialize before
symbolic processing, give the exporter an exact point file with `--point`:

```toml
[parameters]
s = "5"
```

Point values are integers or rational strings such as `"5/18"`; floating-point
integration defaults are not imported. Unbound kinematics stay symbolic.
Point keys and strict kinematic constraints accept native symbol names or
exported aliases. Ambiguous short names require fully qualified native names.
The export records each alias and the exact native symbol it represents.

The massive bubble with mass squared one and `s=5` has interior `F=0` sheets;
both signs are retained. The exporter always supplies the original signed F
as a split polynomial, with every Feynman parameter positive. Kinematic
constraints are explicit. No physical region is inferred from the graph.

## What the checks establish

FastSecDec uses native `GraphIntegral` and `IntegralFamily::symanzik` APIs.
It exports sparse signed U/F, positive propagator powers, stable edge IDs,
the native denominator-to-parameter map, stable DOT, routing and kinematic
provenance without contracting the numerator. Native tests bind the A446 graph
to all original symbolic U/F coefficients and to the fixed point
`mt2=1, mz2=5/18, s=10, t=-3`.

The symGCAD importer independently checks sparse polynomial reconstruction,
signed rational specialization, Feynman homogeneity, the companion TOML hash,
the variable groups, explicit domain and parameter map. It preserves sign and
rational scale before any solver normalization. Altering a split's sign,
point binding or denominator index is an import error.

Graph metadata is provenance and future heuristic input. Import reports
`graph_derivation_verified=false` and `coverage_claimed=false`; it does not
derive U/F again from DOT or certify a CAD. The ordinary solver and independent
verifier establish generic coverage for the imported polynomial problem.

The source graph integral uses projective parameters. The current analysis
domain is the positive orthant, where the checked homogeneous U/F scaling
preserves signs and zeros. Transport of exported CAD cells to the projective
simplex, sector maps, measure factors and numerical integration remain
downstream work. No graph-based deletion of a projection factor is authorized
by this interface alone.

Input-size and sparse-support limits reject unsupported bundles. Native graph
algebra still needs external process limits when used in a bounded campaign.
Graph preparation, import and validation costs belong outside historical
polynomial-only benchmark timings and must be reported explicitly in future
end-to-end comparisons.

## Optional tests driven by FastSecDec

After building the two executables, run FastSecDec's explicit interoperability
tests against the selected symGCAD binary:

```sh
SYMGCAD_BIN="$PWD/target/release/symgcad" \
  cargo test --locked --manifest-path DO_NOT_PUSH_REFERENCES/fastSecDec/Cargo.toml \
  -p fastsecdec-cli --test symgcad_interop --test a446_symgcad_interop \
  -- --ignored --test-threads=1
```

These tests are opt-in because they require a separately installed executable.
They retain the exported bundle, checked input, proof and command logs in a
fresh temporary directory printed by the test. Each workflow has a common
180-second execution deadline; `FASTSECDEC_INTEROP_SECONDS` can set a value
from 1 to 600. Compilation is separate. Allocate an external memory limit and
CPU affinity when running them alongside other campaigns.

The bubble test crosses interior zero sheets. The A446 test materializes its
model through native `Model::standard_model().to_json()`, checks the exported
polynomials against the original corpus after exact specialization, and uses
the previously successful order translated through the explicit edge map.
Homogeneity and the monotone engine retain their ordinary verification
obligations. The expected result is eight cells with both signs of F.

## Validated checkpoint

The initial interface passed 41 focused native core/CLI/preparation/supervisor
tests and nine independent consumer tests. Actual export/import/solve/verify
workflows passed for the fixed massive bubble (three cells), symbolic bubble
with `s > 4` (three cells), and fixed A446 (eight cells, both F signs). These
are interface correctness checks outside the historical benchmark study;
they do not add distinct original corpus completions.

A supplemental A446 check used seed `20261010` and two fresh interior points
per cell: all eight cells, 16 original-F evaluations and 112 original-constraint
checks passed, with zero failures or unchecked cells. Exact prefix-dependent
boundaries and a separate original-polynomial evaluation path were used.
This is a sanity check, not a coverage proof, and is outside benchmark timings.

Initial A446 evidence is retained locally under
`artifacts/fastsecdec-a446-interop-01/`:

| Evidence | SHA-256 |
|---|---|
| Native bundle | `0a9f511085e9ebdb81904f0ee5e8b00aeca1783596d22e0fb6a09c6bbeed9795` |
| Eight-cell proof | `25c85937d3870d4c7390862504a0cecffb6a8be0337f35b553a94456df8b330f` |
| Independent verification | `c25219106f3efa6cadf9071c2f7b0ebe11fb048b588090b602180bf927e16bdf` |
| Supplemental sanity report | `9ec7a4bb60bbdda781332ddbac6ccaf7c0b809650deabf53af6a4b07d4695c6a` |

The dependency pin identifies the tested producer source. Initial build/workflow
logs are recorded in FastSecDec's native review; regenerated evidence may have
different hashes due to source identity, paths and timings. Large generated
bundles, native model JSON and solver proofs remain outside Git.

The initial producer pin is `99f96a5623eb5059f83c340833fafc2858498966`.
Its locked native dependencies include FeynKit/Linnet
`259df8790f27b8d3ef32778cd7195942691b4ef0` and Symbolica
`1deccb8538ccb91dc2c1e58fc0a2e900d2276bf4`; these need not match symGCAD's
separate source dependency because the process boundary carries exact data.

Optional setup was also exercised against a fresh public HTTPS clone into
`artifacts/setup-fastsecdec-pin-01/deps/fastSecDec`. Both fetch and check-only
runs accepted the exact pin. The already validated pinned Symbolica checkout
was reused. With the frozen executable built from the producer pin, the fresh
clone's bubble graph/model/point exported successfully and the independent
consumer verified three cells. Only the memory limit changed between
checked import and solving; external process deadlines bounded each command
to at most 35 seconds on one CPU. This checks clone/setup and relative input paths;
it is not a second clean dependency build.

The replay evidence is under `artifacts/setup-fastsecdec-pin-01/workflow/`:
bundle SHA-256 `534807e3ea90055ded9b4b6f332db88af8db3cad18af403fd975caf2d83bc8c9`,
proof SHA-256 `6c79d8bf7516f4fb8d6e6fce2e9b20ecc41f6e390875f58c7ef34e21e3b2a16e`,
and independent verification SHA-256
`a0dc82c5f803fc9d9f65d1e9ea71e0e5ab898b09de296f900ea2e172aa010d5a`.
