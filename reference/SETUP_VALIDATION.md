# Fresh public checkout validation

On 2026-10-10, an isolated clone of the published `codex/symgcad` branch passed
the documented setup and fixed-A446 workflow. No existing build target,
vendored dependency, original proof, or copied credential file was used.

- Public HTTPS fetches reproduced the pinned pristine Symbolica and GCAD sources.
- Locked Cargo metadata and a fresh release build succeeded under the Nix flake.
- The unit-square example produced two independently verified cells.
- Regeneration produced 277 fixed fixture inputs representing 124 original problems.
- The [A446 recipe](a446-reproduce.toml) produced eight independently verified cells.
- The [supplemental checker](SUPPLEMENTAL_SANITY.md), seed `20261010`, checked
  16 fresh points, 112 original constraints and 16 original F signs with no failures.
- Rendering the committed benchmark data reproduced its PDF byte for byte.
- Both dependency checkouts and the public source checkout remained clean.

The main executable was built at public commit
`a37e5615ece8b88ba82c31d16af2a467db471920`. The recipe and checker were tested at
`1128da4bedc0988f244af270ec41fb80b07cb1da`, whose production source, Cargo files
and flake were unchanged. This is a setup validation, not a new benchmark cohort
or additional original-problem completion.

The retained local audit is `artifacts/publication-fresh-clone/audit.json`,
SHA-256 `ea1c24da57e82301a7f44ba18d5e219daf1e2f12c018919b1c3a76d1aa8ed8c5`.
Its logs, regenerated proof and sampling report remain outside Git. Follow
[SETUP.md](../docs/SETUP.md) and the A446 recipe to reproduce the workflow.
