# Reproducible setup

The project requires Rust >=1.96. The committed `flake.lock` supplies Rust 1.98.1 and native GMP/MPFR dependencies on Linux. Cargo's exact crate resolution is in `Cargo.lock`. Cargo fetches public source dependencies directly; they are never vendored into this repository.

```sh
nix develop
cargo build --release --locked
```

The standalone root manifest pins Symbolica, Numerica and Graphica to the public Symbolica commit recorded in [PROVENANCE.toml](../PROVENANCE.toml). This preserves the original backend revision without requiring `DO_NOT_PUSH_REFERENCES` or the bootstrap script. The recorded local paths are optional research checkouts, not Cargo prerequisites.

## Rust library consumers

symGCAD declares Symbolica `3.0.1` with its GMP/MPFR and tracing features. Cargo applies `[patch]` only from the workspace root: a downstream application can pin one compatible Symbolica/Numerica source for both its own native objects and symGCAD. The standalone pins above do not force a second backend identity into that application. Check the resolved graph with `cargo tree -i symbolica` and `cargo tree -i numerica`; the consumer owns its lockfile, enabled features and compatibility tests. Public API compatibility with another revision is a tested property, not a promise for every future `3.x` release.

Symbolica has its own [license terms](https://github.com/symbolica-dev/symbolica/blob/75f8350094b90254ee71dc2a391fde0d14b0204a/License.md). Supply `SYMBOLICA_LICENSE` through your environment when required. The Nix development shell may also read the optional ignored `.symgcad-env`; do not commit a license key. Solver workers use one symbolic thread. Parallel worker limits also remain subject to the backend license.

## Optional upstream fixes

Production symGCAD handles the linear-resultant sign case with its own exact identity and does not call the affected primitive integer-shift path. The separate research Kronecker helper uses an explicit multiprecision-shift workaround. The measured releases used pristine upstream sources. Optional patches are available for upstream testing:

```sh
scripts/bootstrap-deps.sh --apply-patches
scripts/bootstrap-deps.sh --check --apply-patches
```

Application is idempotent. Already-applied fixes are recognized and the complete source diff must match those fixes exactly. The direct linear-resultant identities in the solver and research helper work with both pristine and patched dependencies; they do not apply an extra correction to a backend result. The bootstrap never removes applied patches; preserve a separate pristine checkout for reproducing historical measurements. [Patch descriptions and regression commands](../patches/README.md) identify the exact changes. Check the current source state without changing it with `scripts/bootstrap-deps.sh --check`.

Bootstrap fetches detached checkouts from `PROVENANCE.toml` and rejects a different revision or unrelated changes without resetting them. Applying patches there does **not** alter the default Cargo build. Run the patch regressions in that checkout as documented, or use a separate symGCAD working copy whose three root patch entries explicitly point to `DO_NOT_PUSH_REFERENCES/symbolica`, `DO_NOT_PUSH_REFERENCES/symbolica/lib/numerica` and `DO_NOT_PUSH_REFERENCES/symbolica/lib/graphica`. Retain the original public pins and lockfile for unmodified standalone builds.

## Independent upstream GCAD comparison

This additional environment is only needed for reference tests, corpus extraction and native comparisons:

```sh
scripts/bootstrap-deps.sh --reference
nix develop .#reference -c python reference/build_reference.py
mkdir -p artifacts/corpus
nix develop .#reference -c python reference/adapter.py extract-core \
  --source DO_NOT_PUSH_REFERENCES/gcad > artifacts/corpus/core-inputs.json
nix develop -c cargo run --release --locked -- corpus --fixed --output artifacts/corpus/fixed
nix develop -c cargo run --release --locked -- corpus --output artifacts/corpus/symbolic \
  --core artifacts/corpus/core-inputs.json
```

The reference builder makes an isolated `.reference/gcad` copy, compiles against the locked Nix FLINT/GMP/MPFR libraries, and records source/compiler provenance. Python is confined to the independent upstream bridge and fixture extraction; solving, supervision and benchmark accounting are Rust. See [reference protocols](../reference/README.md) for paired measurements and exact completion audits. Historical large `artifacts/` trees and external literature PDFs are not shipped; reports retain hashes and the documented commands regenerate inputs and new runs.

## Benchmark document

The default Nix shell includes Typst and pinned Libertinus/Noto font packages. It sets `TYPST_FONT_PATHS` to their Nix store paths:

```sh
benchmark/render.sh
```

This renders the main benchmark and its separately preserved ordering/MCTS
companion. The table data and Typst sources are committed separately from
generated solver proofs. Regenerating measurements requires the retained
source evidence; rendering the published snapshot does not.

## Isolated root-isolation experiments

The [positive-axis replay package](../reference/research/positive_axis_api/README.md)
prepares a fresh tree with two separate Symbolica patches: exact unit Taylor
shifts and strictly positive root isolation. It includes focused regressions,
bounded measurements and exact inventory comparisons. Its setup leaves the
normal dependency checkout unchanged. These experimental APIs do not change
production cell boundaries or the published corpus completion count; see the
[measurement record](../reference/POSITIVE_AXIS_ISOLATION_RESEARCH.md).

## Optional graph input from FastSecDec

```sh
scripts/bootstrap-deps.sh --fastsecdec
scripts/bootstrap-deps.sh --fastsecdec --check
```

This fetches the tested `no_deformation` branch revision recorded in
`PROVENANCE.toml` into the ignored `DO_NOT_PUSH_REFERENCES/fastSecDec` directory.
FastSecDec stays a separate repository with its own Cargo lockfile; its source
is never included in symGCAD commits. It is optional for graph-driven tests
and exact U/F export, and unnecessary for ordinary polynomial problems.
See [the graph-input workflow](FASTSECDEC_INTEROP.md) for build, export,
checked import and independent solve verification. The full no-deformation
integration pipeline remains downstream.

Historical benchmark source identifiers map to the copied public history through
[the publication commit map](../reference/PUBLICATION_HISTORY.md).
The documented setup and A446 workflow also passed an
[isolated public-clone validation](../reference/SETUP_VALIDATION.md).
