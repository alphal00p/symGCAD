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

## Quadratic certificates and ladder gate

The subsequent complete fast suite, including the new PSD-component
certificate and quadratic-ladder engine, passed **615 tests across 45 targets,
zero failed, two ignored**. One pass is the separate uncommitted alternate-mass
regression, so the tracked suite accounts for **614 passes**. No production,
test or Cargo input changed during the run. Formatting checks also passed for
the new modules and regressions.

The same command and CPU3 / 2 GiB / 900-second supervisor completed in
476 seconds, including 7 minutes 3 seconds of compilation; test bodies summed
to 51.93 seconds. Sampled aggregate peak RSS was 1,634,192 KiB. These are
correctness-gate costs, not benchmark solver timings. The original corpus
count remains unchanged. See the [conditional result](QUADRATIC_LADDER_VALIDATION.md)
and [four-way certificate diagnostics](QUADRATIC_COMPONENT_PRODUCTION.md).

Evidence under `artifacts/research/quadratic-fast-suite-01/`:

| Record | SHA-256 |
| --- | --- |
| `run.log` | `ff8eaa44e7782c70606cfba4031222684cc281f02935b8cab9e5cccbd42ae21d` |
| `run.process.json` | `d9f462003ed91485951262d1af9fea4c90b48c373cb223acec3ec86a0354da32` |
| `source-hashes.txt` | `ceefe93aee544944f472652d05858de88aa36b71c299b96088d34f074a945e7c` |
| `source-check.txt` | `5495c5de11f506dc9249a87a079c224bcb535b0d5fb39ce2ddcff65b982419db` |

## Affine factor preconditioning gate

On 11 October 2026, the production sources from `8de480f` passed **622
tracked tests across 45 targets, zero failed, two ignored**, using the normal
unpatched Symbolica checkout. The source snapshot was taken at `2ba8dfa`,
which additionally contains research helpers and documentation. The run
selected the library, binary and every tracked top-level integration target;
the separate workstream's untracked alternate-mass regression was excluded.

The eight new production checks cover the bounded affine policy, signed
factor reconstruction, finite shift admission, context isolation,
serialization, independent verification and same-sign interior seams.
All tracked source/dependency hashes matched after the run. The default
policy remains disabled. Formatting and pinned-dependency checks passed.

The locked Nix run used CPUs 6 and 7, two Cargo build jobs and one test thread
per target. Only symGCAD's test/development debug information was disabled;
Cargo.toml and release settings were unchanged. Two compile-only attempts
are retained: an unused dependency profile was stopped, and the ordinary
backend rebuild reached its 900-second build limit before tests ran. The
cached continuation compiled in 2 minutes 33 seconds and completed every
target successfully; test-body times summed to **63.11 seconds**. These
build and validation costs are not benchmark solve times.

Evidence under `artifacts/research/affine-factor-policy-01/validation/`:

| Record | SHA-256 |
| --- | --- |
| `fast.log` | `f4b61793c223dd5a52061f31677259175eb082e460c6016b80b73cae3861c247` |
| `summary.json` | `484492d948f70fa5fdd7ef5b4828371eb31ff94d67acefb14320347a12a6dd49` |
| `source-hashes.txt` | `8af1db6c4e04d46e3ad6cecbddbdcbbe003154bea550d2019da8a9bc381f17ca` |
| `test-targets.txt` | `51538194c0e8fa1f82c949e2645fadea234e749b75782f3fab22fc52614646d2` |
| `source-check.log` | `8d00848355741807ddb3dc2c1238e7a0747564f89e8fbc2cc7abe390b3977493` |

The separate [quadratic-face diagnostic](QUADRATIC_BLOCK_FACE_DIAGNOSTIC.md)
passed six helper tests, and the [larger witness-budget probe](GLUZA33_WITNESS_BUDGET.md)
passed seven. These helper checks are outside the 622-test production count.
Neither diagnostic adds an original-corpus completion.

## Descartes source-admission gate

The subsequent production change `8e28c21` passed **627 tracked tests across
46 targets, zero failed, two ignored** on the ordinary Symbolica backend.
All source hashes matched after the run. The separate workstream's untracked
alternate-mass fixture remained excluded. This adds five checks for bounded
source admission, full raw guard preservation, reuse, independent replay,
historical defaults and altered evidence.

The locked Nix run used CPUs 6 and 7, two Cargo jobs and one test thread per
target, with the same package-specific debug settings as the preceding gate.
Test bodies summed to **50.96 seconds**. This is regression validation, not a
solver timing comparison. All 27 earlier focused tests also passed.

Evidence under `artifacts/research/descartes-source-budget-production-01/validation/`:

| Record | SHA-256 |
| --- | --- |
| `fast.log` | `208f0e9d261c07775faf305858814d30cedf5a649a9e5f0d057da8818d33f35d` |
| `summary.json` | `c09881f179377150120437176b8dcfa99797c0afcdafbee4c735433364cec567` |
| `source-hashes.txt` | `cceffcc8c4e264bfbd46828b58c3e26d62dc9335a9aadaa369fab64c6131226e` |
| `test-targets.txt` | `8d7df6063ab855c89d80d7b99d3e8bb64994f5cc3b58829b2877c0779a6d92ba` |
| `source-check.log` | `d7caaf1f26ed6570c71a643c861dec5f97b11c26cac2b103cc0f171ff92e9b3e` |

## Interval-Descartes source-admission gate

The subsequent interval-source option passed **633 tracked tests across
47 targets, zero failed, two ignored** on the ordinary Symbolica backend.
All source hashes matched after the run. The six additional checks cover
discovery, template reuse, independent raw-projection replay, serialization,
historical defaults, rejected tampering and the unchanged per-guard limit.
The separate workstream's untracked alternate-mass fixture remained excluded.

The same locked Nix environment, CPUs 6 and 7, two Cargo jobs and one test
thread per target were used. Test bodies summed to **52.49 seconds**.
All 34 focused tests also passed. Neither this validation gate nor the
separate warm Gluza33 continuation adds an original completion by itself.

Evidence under `artifacts/research/interval-source-budget-production-01/validation/`:

| Record | SHA-256 |
| --- | --- |
| `fast.log` | `bc856fe7d1fc387b57c35b7869e1d5952810aea896a073560dbd1b8a06ee8b74` |
| `summary.json` | `7af3e228495dcf404f02a5a96d348b4255192bbdf4457b014d549f26d51c0e55` |
| `source-hashes.txt` | `2c2e1835209b925f94a790891babefe757c1b162ef7e8d0b2758bc442d49bf89` |
| `test-targets.txt` | `668b3507cccc2f5f9c80076f615332fcac5a848797b9ce0986c9517442596127` |
| `source-check.log` | `0419b0862c0b4aae1c31e80964b02865aa1b62d9a5d1282dab1217d2cbe76cf5` |

## Exact shear factor preconditioning

The production shear option passed **642 tracked tests across 48 targets,
zero failed, two ignored**, using the unchanged pinned Symbolica backend.
All source hashes matched after the run; test bodies summed to **51.48 seconds**.
The separate workstream's untracked alternate-mass fixture remained excluded.
The release build also passed a fresh solve and independent verification of
four cells separated by same-sign interior zero seams. See
[the policy and exact reconstruction checks](SHEAR_FACTORIZATION.md).

The optional Wang backend experiment has its own 19 selected tests; those
are separate from this production count and from original corpus completion.

Evidence under `artifacts/research/shear-factor-policy-01/validation/`:

| Record | SHA-256 |
| --- | --- |
| `fast.log` | `eece188a8b4bd5a0294c4aec01a51abbece355ba9571d529e1cfdcaad2885179` |
| `summary.json` | `d918247ab51ff6f1993f41465ba0722683f8cb5bbc503e23b2b3a54c0ee632fe` |
| `source-hashes.txt` | `b6bb96ea1457b24ea0dbe5f00b4ee861ad7e2e737bbeb2efc06f910f44ccb163` |
| `test-targets.txt` | `3bf641694675e5a0cb5d6f00aff5ba64f8fa575cde589c51c4e1674b5a4698ad` |
| `source-check.log` | `32aa676d98c657cbd3e981690da38d82998c3449c49bba5adc162dbfa9955e79` |

## Interval aggregate-guard budget and progress I/O

Commit `37cff30` passed **646 tracked tests across 49 targets, zero failed,
two ignored**, with unchanged production/test source hashes and **59.23 seconds**
of test bodies. This includes the configurable aggregate guard budget and
the corrected optional progress throttle. The separate workstream's untracked
alternate-mass fixture remains excluded.

The added generic guard regression exceeds the historical 65,536-term budget
and checks exact discovery, template reuse, full-map guards and independent
replay. Three integration tests cover configuration, serialization, historical
defaults, rejected budget tampering and incomplete output. Existing progress
tests preserve immediate first snapshots, capture refresh and diagnostic
failure routing. Research-helper checks are separate from this count.

Evidence under `artifacts/research/interval-total-budget-production-01/validation/`:

| Record | SHA-256 |
| --- | --- |
| `fast.log` | `21bde3fc9e43f231a4c3b190efba8f0a92ad772d76942752f1d507cd35fc1a6d` |
| `summary.json` | `40f0e2597cbef33695aab467e8571dd7d252d7a72bcf1628e41bdf8bbf1d3fc7` |
| `source-hashes.txt` | `0583e3e146339bc52e468fd435222759f975e638075f8f05ce6260e8a1654259` |
| `test-targets.txt` | `49d9f28ecdf60fd4674681cf9b22f95e880511efed84a302eae5f8484bf194bf` |
| `source-check.log` | `51a78641599cba12c001b2e2103406e7aee0a995a009f8746d7371a7449dce6f` |

All examples also passed `cargo check --locked --examples`. The release CLI
built successfully, then independently verified a four-cell same-sign-seam
smoke result with four stacks and ten exactly rejected branches. Source hashes
remained unchanged. The frozen release SHA-256 is
`571f37a5c868aaa2055fb1c2a667332d0f7ff1df0a7e26d87a9edab493ee2c30`;
build, solve, verification and source-check logs are retained in the sibling
`release/` directory. This smoke check adds no corpus completion.

## Support-preserving scaling and exact interval coalescence

The subsequent combined gate passed **653 tracked tests across 49 targets,
zero failed, two ignored**, using the unchanged pinned Symbolica backend.
All production/test source hashes matched after the run; test bodies summed
to **58.57 seconds**. The separate workstream's untracked alternate-mass test
remained excluded. The build used the same locked Nix environment, CPUs 6
and 7, two Cargo jobs, one test thread per target and package-specific debug
settings as the preceding gate.

New checks cover diagonal scaling, exact inverse maps, signed rational and
repeated factors, shared transformation/replay budgets, and adjacent interval
coalescence with exact root counts and endpoint obligations. Existing generic
aggregate-budget and finite lifetime-stock regressions remain unchanged and
pass. The optional affine policy still defaults to false. Research operand
and helper measurements are separate from this production count and do not
add original-corpus completions.

Evidence under `artifacts/research/support-preserving-production-01/validation/`:

| Record | SHA-256 |
| --- | --- |
| `fast.log` | `f04ab2188ec3be019bad93c5dbe5406fa5d378bc7c95f1785a80f2abc498294e` |
| `summary.json` | `50d408399c3018d58a89fdf563605da558d959448ca3d1f5a748261f08af4b1e` |
| `source-hashes.txt` | `beddfbdf52cd029c06c17f5cdb3866377e79cda9e9ccf7fde41562c81e3cbfa8` |
| `test-targets.txt` | `49d9f28ecdf60fd4674681cf9b22f95e880511efed84a302eae5f8484bf194bf` |
| `source-check.log` | `da104b58c0aa983c519243d0e29935fab83e7eff8069b6355921618db72ee6f8` |

All examples passed `cargo check --locked --examples`, and the four changed
Rust files passed formatting checks. The ordinary-backend release build then
solved and independently verified a repeated-factor interior-seam smoke problem:
four cells, four stacks and nine exactly rejected branches. All source hashes
remained unchanged. The frozen release SHA-256 is
`40f8740952452433f9a68d67db54a3b8632f2827f660f1ad83e2240eb6e324f3`;
build, solve, verification and source-check records are in the sibling
`release/` directory. The separate optional-Wang hard-case worker has its own
backend and binary provenance and is not this production release.
