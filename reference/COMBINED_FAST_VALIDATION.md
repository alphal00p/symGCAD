# Combined fast-suite validation

The combined production changes through `0409343` passed the complete fast
suite on 10 October 2026: **599 tests passed across 43 targets, zero failed,
and two explicitly expensive tests remained ignored**. This includes one
uncommitted alternate-mass regression from a separate workstream; the tracked
suite accounts for 598 passes. That alternate input neither enters the
original-corpus completion count nor belongs to this milestone's changes.

The run covers the untrusted support-proposal cache, schema-2 support registry,
weighted-square certificates and existing solver, verifier, serialization,
ordering, worker and graph-interface regressions together. Input source hashes
matched after the run. This is a correctness gate, not an original hard-case
completion or a new timing comparison with native GCAD.

Command, after entering the locked Nix environment and loading the local
Symbolica license:

```sh
taskset -c 3 bash scripts/test.sh fast --locked -j1 \
  --config profile.test.package.symgcad.debug=0
```

The command-only test debug setting keeps compilation within the research
slot's memory budget; it does not alter Cargo.toml or release optimization.
The outer supervisor allowed 900 seconds and 2 GiB aggregate sampled RSS.
The command completed successfully in 456 seconds, including compilation,
with a sampled aggregate peak of 1,144,388 KiB. Cargo reported 6 minutes
42 seconds of compilation; the sum of test-target body times was 52.34
seconds. The two expensive algebra replays are deliberately separate from
the fast suite, as are full corpus campaigns.

Evidence is retained outside Git under
`artifacts/research/combined-fast-suite-01/`:

| Record | SHA-256 |
| --- | --- |
| `run.log` | `0c5c2d896fa67025ebdeb86696ee1ca39ebb496a7d72b87183caa36ab5e2f6f6` |
| `run.process.json` | `5c4a61841d1617c1ae93605fd58b4a2869772ed4b7f058e444fe655fe4e0e0c7` |
| `sources.sha256` | `39216baca56fe757929419283f0c3b5a61d4c4d5158e1d4964895ce10663f003` |
| `source-check.log` | `01978e5f76e28c77050a086891439fe0414d81192bc061954fa03c7da00fc3b2` |
| `source-commit.txt` | `d400614fca8a17f23bae13d719e8f6614ca636898acdf16945a5929cd5485768` |
| `untracked-tests.txt` | `3eaf83fa68cbeef4978677efc81fa62ab53377d1ff7843f2bddda8e29d497353` |

See the individual [proposal-cache](BASE_SUPPORT_PROPOSAL_CACHE.md),
[registry](BASE_SUPPORT_REGISTRY_CAPACITY.md) and
[weighted-square](WEIGHTED_SQUARE_PRODUCTION.md) notes for the proof contracts,
focused regressions and narrowly scoped measurements.
