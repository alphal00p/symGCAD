# Reproducible setup

The project requires Rust >=1.96. The committed `flake.lock` supplies Rust 1.98.1 and native GMP/MPFR dependencies on Linux. Cargo's exact crate resolution is in `Cargo.lock`. Source dependencies are fetched separately; they are never vendored into this repository.

```sh
scripts/bootstrap-deps.sh
nix develop
cargo build --release --locked
```

The bootstrap reads [PROVENANCE.toml](../PROVENANCE.toml), fetches the exact Symbolica Git commit over public HTTPS, and checks out a detached revision. Numerica and Graphica are tracked workspace members of that same Symbolica monorepo; their exact subtree IDs are recorded too. Cargo uses all three local paths. Repeating setup verifies the existing checkout. A different revision, unrelated modified file, or untracked source file causes an error without resetting or removing it.

Symbolica has its own [license terms](https://github.com/symbolica-dev/symbolica/blob/75f8350094b90254ee71dc2a391fde0d14b0204a/License.md). Supply `SYMBOLICA_LICENSE` through your environment when required. The Nix development shell may also read the optional ignored `.symgcad-env`; do not commit a license key. Solver workers use one symbolic thread. Parallel worker limits also remain subject to the backend license.

## Optional upstream fixes

Production symGCAD handles the linear-resultant sign case with its own exact identity and does not call the affected primitive integer-shift path. The separate research Kronecker helper uses an explicit multiprecision-shift workaround. The measured releases used pristine upstream sources. Optional patches are available for upstream testing:

```sh
scripts/bootstrap-deps.sh --apply-patches
scripts/bootstrap-deps.sh --check --apply-patches
```

Application is idempotent. Already-applied fixes are recognized and the complete source diff must match those fixes exactly. The direct linear-resultant identities in the solver and research helper work with both pristine and patched dependencies; they do not apply an extra correction to a backend result. The bootstrap never removes applied patches; preserve a separate pristine checkout for reproducing historical measurements. [Patch descriptions and regression commands](../patches/README.md) identify the exact changes. Check the current source state without changing it with `scripts/bootstrap-deps.sh --check`.

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
nix develop -c typst compile benchmark/benchmark.typ benchmark/benchmark.pdf
```

The document's table data and source are committed separately from generated solver proofs.

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
