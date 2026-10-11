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
