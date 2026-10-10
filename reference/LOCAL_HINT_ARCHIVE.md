# Local cut archive restart experiment

The optional archive works as a source of independently checked cut hints.
This experiment did **not** demonstrate a completion or timing benefit, and
`solver.local_hint_archive` remains false by default. All seven workers timed
out: one 30-second seed and three pairs of 60-second restarts. No result receives
completion or proof credit.

The experiment used original `test_gluza_ex30/order/1`, preserving every
constraint and the original variable group, with legal explicit order
`x0,x6,x4,x2,x5,x7,x1`. Both modes used the direct local engine, original-domain
positivity, ordinary Descartes-one, one CPU (7), 4 GiB, 100,000 refinement and
2,000,000 node caps. All three sample-dependent template modes were disabled.
The frozen debug executable contains archive commit `f8fdbee` and conditional
endpoint commit `0a95bc2`; its SHA-256 is
`99eba9fb660c95e41f3eecc1efcd03e516de51a3041d2665391b399352ef6113`.

A fresh interrupted seed produced 6,035 checkpoint files, including a
762,873-byte archive with 157 polynomial entries and 2,048 exact prefixes.
Every paired restart began with a separate copy of **all** those files; every
copied SHA-256 was checked before execution. The paired serialized worker
inputs were compared after removing only the archive flag and checkpoint
path. Execution order alternated off/on, on/off, off/on. The complete campaign,
including copying and hashing, lasted 406 seconds under an external 600-second
cap. Independent verification would run for a complete result; none occurred.

| Median last recorded work / cost | Archive off | Archive on |
| --- | ---: | ---: |
| Speculative nodes | 163,927 | 154,556 |
| Refinements | 24,629 | 23,857 |
| Fresh factorization cache misses | 1,124 | 1,900 |
| Fresh resultant cache misses | 505 | 720 |
| Factor phase, seconds | 10.379 | 11.796 |
| Archive load and factor validation, seconds | 0 | 0.197 |
| Five periodic archive snapshots, seconds | 0 | 0.449 |

Every enabled restart used 1,060 exact-prefix entries and seeded 3,287 cuts,
validating 131 polynomial factors with no rejected entries. Loading was
inexpensive, but the extra initial cuts changed the subsequent decomposition
and increased fresh algebra work. The work counters describe different
traversals, **not** a fixed-work speed ratio. Final telemetry was captured at
59.49–59.92 seconds; process outcomes retain the full measured timeout times.
No uninterrupted control is needed to support this limited negative result.

The preserved local evidence is under `artifacts/restart-hints-01/`:

- `run.sh`, `collect-progress.sh` and `capture-inputs.sh` preserve the actual
  driver and progress collection commands; `source-sha256.txt` and
  `collector-sha256.txt` record their hashes. Public `solve` creates its own
  temporary progress path. The collector identifies its exact serialized
  worker input by checkpoint path and copies the corresponding progress file;
  the final available worker snapshots were also retained.
- `seed-manifest.sha256` and each `*-seed-check.log` preserve all 6,035 checks
  for each of the six copies. The seed-manifest hash is
  `07b7f1021698a34cc77e917d19c6c0f0c29cc6169950e9fa30df2bd9398eacb9`.
- Every trial directory retains its campaign, actual worker input, append-only
  completion ledger, latest progress, progress history and summary. The
  `matched-input.json` files establish the paired input equality.
- `summarize.sh` rebuilds `analysis.json` from the retained ledgers and final
  progress snapshots. Its final analysis hash is
  `ddf8825f71914a457b3a8a763964882ee81d094792584dc2231040fadcab36fc`;
  `evidence-sha256.txt` and `outcomes-sha256.txt` cover the underlying files.

The actual launch was:

```sh
nix develop --command bash -c 'timeout --signal=TERM --kill-after=5s 600s artifacts/restart-hints-01/run.sh artifacts/hard-campaigns/conditional-archive-debug-01/bin/symgcad'
```

A repetition must use a fresh experiment directory and the new shell's actual
`TMPDIR`/process IDs for progress collection; existing seed copies and ledgers
must not be overwritten. The checked Rust supervisor supplies each worker's
memory and time limits. This is a discovery-hint measurement, not a persistent
proof-cache benchmark.
