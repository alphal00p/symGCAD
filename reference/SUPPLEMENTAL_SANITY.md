# Supplemental exact sampling

The fixed-kinematics A446 decomposition passed **two fresh interior rational
points in every one of its eight exported cells**. Seed `20261010` produced
16 distinct-per-cell points, all different from each cell's stored sample.
All 112 original strict-constraint checks and 16 original F evaluations passed:
zero failures, zero unchecked cells and zero untested points. Kinematics were
`mt2=1`, `mz2=5/18`, `s=10`, `t=-3`.

This is supplemental testing, **not a coverage proof**. Its approximately
0.41-second checker run is outside all solver benchmark measurements. The
independent complete-result verifier remains the source of mathematical
completion credit.

## Fresh-clone A446 solve and check

Complete the [setup and fixed-corpus regeneration](../docs/SETUP.md) first.
The tracked [A446 recipe](a446-reproduce.toml) contains only operational
settings; the polynomial and kinematics come from the regenerated original
fixture. It selects `engine="monotone"`, MODS ordering, exact input/domain
positivity, the full positive Feynman-variable homogeneous block and anchor
`x4`. It leaves every original constraint, split and parameter unchanged.

From the repository root inside `nix develop`:

```sh
cargo build --release --locked
target/release/symgcad bench reference/a446-reproduce.toml
jq -e '.complete_verified == 1' artifacts/a446-public-reproduction/ledger.json
SANITY_RESULT=$(jq -er '.runs[-1].trials[-1] | select(.verified == true) | .result_file' \
  artifacts/a446-public-reproduction/ledger.json)
cargo run --example sanity_sample -- \
  --original artifacts/corpus/fixed/0093_finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1_fixed.toml \
  --result "$SANITY_RESULT" \
  --output artifacts/a446-public-reproduction/sanity.json \
  --seed 20261010 --points-per-cell 2 --seconds 120 --memory-mib 4096
```

The completion command runs solve and independent verification in fresh workers,
with 120 seconds for each, one symbolic worker and a 4-GiB memory cap. It records
the actual executable/input/proof hashes and uses a fresh checkpoint namespace.
The `jq` checks explicitly require a verified result before sampling. The
recorded eight-cell proof remains historical evidence; a regenerated run records
its own result and sampling report. Choose a fresh sampling output filename when
repeating this recipe. No private proof, literature file or old artifact directory
is needed.

## Method and reproducibility

[`examples/sanity_sample.rs`](../examples/sanity_sample.rs) reads the supplied
original TOML separately from the decomposition. Original expressions are
parsed with a variable map containing the original coordinates and parameter
names. A separate sparse evaluator sums their monomials with Symbolica exact
rational arithmetic; it does not use normalized F, reconstructed polynomials,
chart formulas, or the solver's sign evaluator. The original input's variables,
constraints, splits, parameters, groups and mandatory order must match the
export.

For each fresh prefix, the checker specializes and isolates the actual boundary
polynomial again. It supports real, positive-real and descending-real distinct
root indices, including repeated roots. Stored isolation intervals are ignored.
Seeded rational fractions select candidates inside a newly certified gap or
unbounded ray. A second exact Sturm root-count computation independently checks
strict membership. Each coordinate differs from its stored value and from the
corresponding coordinate of earlier points in that cell. This reproducible
sampling distribution is not uniform on unbounded cells.

Every raw cell, and every merged cell if the export contains them, must be
checked. A nonexistent root, nullified boundary, collapsed interval, failed
sampling, changed input, resource exhaustion or failed evaluation cannot earn a
pass. Zero-dimensional exports are unsupported because they cannot contain two
distinct points. A killable worker enforces time and memory bounds. Partial
reports are saved after each point; failures retain candidate coordinates,
prefixes, boundary checks and exact original-expression values.

Build and run against any complete exported original problem:

```sh
nix develop -c cargo run --example sanity_sample -- \
  --original artifacts/corpus/fixed/0093_finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1_fixed.toml \
  --result /path/to/complete-original-result.json \
  --output artifacts/a446-sanity-new.json \
  --seed 20261010 --points-per-cell 2 --seconds 120 --memory-mib 4096
```

The output path must be fresh. A sibling `.process.json` records the supervisor
outcome; only a successful worker and an exhaustive `passed` report exit zero.
The six focused regressions run with
`nix develop -c cargo test --example sanity_sample`. They cover changing
prefix-dependent roots, deliberately false F labels and original constraints,
exact parameter tokens, reproducibility and uniqueness, distinct-root selectors
with multiplicity, invalid bounds, deadline failure, and rejection of candidates
outside or exactly on a root by the separate membership check.

## Retained evidence

[Full exact points and checks](a446-sanity.json) are committed separately from
the large solver proof and external reference input:

| Item | SHA-256 |
| --- | --- |
| Original TOML | `435291febcf3f9ffc2085d84242ce45b83b791c024233133d7dca424b41cd675` |
| Accepted eight-cell proof | `bb36fe245b505736a1cfef618cecba38540b2d25a8afc95debbd98a84fe55b50` |
| Frozen checker executable | `de7409a97f248743637717f1a983215f7de9e3ffebfb020c0fed98034a47e90f` |
| Checker source | `b9508ad6e09ccdbe2449c845e21f0febead4e56dd3fa38599b6ff83dbd3f103f` |
| Full sampling report | `4f46c4ffaa8056543c5892957706feaf7e503e31b8fd63882a3eb1850f89dead` |

The original accepted proof is the fixed A446 entry in the audited 303/331
completion aggregate. Its preserved local source is
`artifacts/completion-deterministic/fixed/results/00122-fcc950e632c0f6a7-1791579019095793168-0/0-result.json`.
The complete run and resource report are under
`artifacts/sanity-fixed-a446-02/`; these private artifact paths are provenance,
not fresh-clone requirements.

A deliberately corrupted copy changed cell index 2's F label from negative to
positive. The same seed still checked all eight cells and 16 points, found
exactly two sign failures in that cell, and returned a nonzero exit status.
[Both counterexamples](a446-sanity-mutation.json) retain their exact coordinates
and negative F values. The genuine proof was not modified.
