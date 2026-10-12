# Benchmark report

`benchmark.pdf` (source `benchmark.typ`) has three parts:

1. **Benchmark results**: every one of the 331 distinct corpus problems, solved by symGCAD in a
   fresh process with cold caches and re-checked by `symgcad verify`, next to the original GCAD
   by Vitaly Magerya (300 s cap per sign case). Per row: status, engine and ordering policy, the
   variable order that was picked (with the homogeneous anchor), verified cells, solve and verify
   seconds, the cache tag (`cold` for every row measured by the report run), native seconds or
   `timeout`, and the time ratio. Rows that complete only under the opt-in projective contract
   (`solver.projective_sections = true`) are in a separate, labelled table (section 1.2) and never
   count towards the strict total.
2. **Improvements over the original GCAD**: `improvements.typ`, an exhaustive list (engines,
   certificates, kernels, local-engine mechanisms, ordering, caching, verification, supervision,
   interfaces, upstream fixes, opt-in contracts, investigated ideas). Each item has a plain-language
   explanation, its default/opt-in status with the option name, and its measured effect where one
   exists. `improvements-sources.txt` maps each item ID to code, tests, reference notes and session
   reports. This section replaces the earlier rule of at most two pages of notes after the table.
3. **Symbolica vs FLINT** on captured resultants, discriminants, factorizations and GCDs: summary
   cards, measurement status, interpretation, reproduction pointer, and the per-operation tables of
   `FLINT_vs_symbolica/results/table.typ`. The full campaign is documented in
   [`FLINT_vs_symbolica/README.md`](FLINT_vs_symbolica/README.md).

`ordering-mcts.typ`/`.pdf` is a separate, independently authored companion and is not generated.

## Files

| Path | Role |
| --- | --- |
| `build_report.py` | Builds `report-data.json` from the runs below (stdlib Python ≥ 3.11). |
| `report-data.json` | Generated table data with per-row provenance (binary sha256, input sha256, cache policy, date, record path) and a `sources` map of every file read with its sha256. |
| `run_final.sh` | One command: cold symGCAD runs (3 repeats by default), opt-in projective rows, `report-data.json`, PDF. |
| `campaign/groups.json` | Manifest: the 331 groups, aliases, original corpus inputs (+sha256), sign cases, native ordering policy, symGCAD recipe and the 2026-10-10 measurement. |
| `campaign/symgcad_inputs/` | The symGCAD recipe of every group (engine, ordering, options, limits). |
| `campaign/projective/` | Opt-in recipes for positive Gluza33 (group 302) and positive Gluza30 (group 298). |
| `campaign/symgcad_runner.py` | Cold runner: fresh run directory and `TMPDIR` per run, no persistent checkpoints (a fresh `checkpoint_dir` only for MCTS searches, where the solver would otherwise use the shared one), one pinned CPU, `prlimit`, GNU `time`, solve then independent verify; each group runs and is timed in its own worker process pinned to its CPU; every `result.json` is kept; resumable `records.jsonl`. |
| `campaign/native_runner.py` | Native GCAD runner: sign expansion as in `src/benchmark.rs`, unmodified `.reference/gcad` through `reference/adapter.py`, 300 s / 8 GiB per case. |
| `campaign/build_manifest.py`, `campaign/symgcad_summarize.py` | How `groups.json` was built; per-group summary of runner output. |
| `FLINT_vs_symbolica/` | Reproducible Symbolica-vs-FLINT kernel campaign (harness, operands, `results/summary.json` and `results/table.typ` read by the report); see its README. |
| `data.json`, `followups.json` | Frozen 2026-10-10 audit snapshot (from `examples/benchmark_report.rs`); read for history (earlier measurements, longest earlier attempts of unresolved problems). |

The campaign scripts are verbatim copies of the ones used on 2026-10-12 (they assume this host:
repository at `/common/dev/gcad`, GNU time at `/run/current-system/sw/bin/time`, the reference
Python of `nix develop .#reference`).

## Regenerate the final report

Prerequisites: Nix, the Symbolica license environment in `/common/dev/gcad/.symgcad-env`
(override with `SYMGCAD_ENV=...`), and a release `symgcad` binary of the commit being reported.

```sh
benchmark/run_final.sh /path/to/release/symgcad --cpus 360,361,362,363,364,365 --parallel 6 \
    [--out DIR] [--native DIR]
```

Check that the chosen CPUs are idle first (e.g. `mpstat -P 360-365 5 1`); `--parallel` must not
exceed the number of CPUs. The published report was produced this way on 2026-10-12 (binary
sha256 and run directory on the title page and in `report-data.json`).

This runs, in order:

1. all 303 groups verified in the 2026-10-10 audit, cold, `--timeout 600` s each, into
   `artifacts/bench-report/symgcad-<sha12>/symgcad/`, repeated `--repeats N` times (default 3;
   repeat k > 1 into `symgcad-k/`; about 7 minutes per repeat at parallel 2, 2.5 minutes at
   parallel 6; about 6.5 GiB of results per repeat); add `--attempt-unresolved 300` to also give the 28 unresolved groups a cold
   300 s attempt (about 70 minutes; not done for the published report, whose unresolved rows show
   the recorded one-hour attempts with their cache tags);
2. after each strict repeat, the two opt-in projective rows into `.../projective/` (`projective-k/`);
3. `build_report.py` → `benchmark/report-data.json`;
4. `typst compile benchmark/benchmark.typ benchmark/benchmark.pdf` (Typst from `flake.lock`, 0.15.1).

Every stage is resumable (records are keyed by binary sha256). `--skip-runs` only rebuilds the
data and the PDF; `--out DIR` chooses the run directory; `--native DIR` points to the native run.
The title page lists every reason the report is still a draft (groups without a cold run, mixed
binaries, native run incomplete, no FLINT results, opt-in rows not run, machine-local evidence
missing when `report-data.json` was built, or FLINT results that no longer match the snapshot the
Section 3 text describes); a final report has none. `build_report.py` defaults `--unresolved`
and `--projective-evidence` to paths on this host; on another machine pass them explicitly, or the
build warns and the PDF is marked as a draft instead of silently dropping those rows' evidence.

Direct use of the builder:

```sh
nix develop .#reference --command python3 -I benchmark/build_report.py \
    --symgcad RUN/symgcad RUN/symgcad-2 RUN/symgcad-3 --projective RUN/projective RUN/projective-2 RUN/projective-3 \
    --native /common/dev/gcad/artifacts/research/claude-2026-10-12/bench/native_300s_run \
    [--unresolved DIR] [--flint-dir DIR] --output benchmark/report-data.json
nix develop --command typst compile benchmark/benchmark.typ benchmark/benchmark.pdf
```

## Native GCAD reference run

The native run is separate because it takes hours (579 sign cases, many at the 300 s cap). The
2026-10-12 run lives in
`/common/dev/gcad/artifacts/research/claude-2026-10-12/bench/native_300s_run/` and was launched by
`.../bench/run_native.sh` (provenance JSON in the run directory). To repeat it:

```sh
out=artifacts/bench-report/native-300s; mkdir -p "$out"
echo 340-345 > "$out/cpus.txt"; echo 3 > "$out/parallel.txt"   # editable while running; touch $out/STOP to drain
nix develop .#reference --command python3 -I benchmark/campaign/native_runner.py \
    --out "$out" --timeout 300 --memory-mib 8192
```

It needs the isolated upstream copy `.reference/gcad` (built by `reference/build_reference.py`,
revision pinned in `PROVENANCE.toml`). `build_report.py` reads `cases/*/*/record.json` directly and
accepts a partial run: a group is `complete` only when all its sign cases completed, `timeout` or
`memory` as soon as one case failed that way, and `not run` otherwise. The 2026-10-12 run is
complete (579/579 sign cases: 486 complete, 93 timeout; 278 groups complete, 53 timeout). Two
sign cases (`finiteA_6_159…/symbolic` and `finiteAx12_6_159…/symbolic`, signs `+`) first failed
with `std::bad_alloc` under the 8 GiB cap and were re-run with 24 GiB
(`provenance-rerun-*.json`); both then timed out at 300 s, and the failed attempts are kept.

## Symbolica vs FLINT section

`build_report.py` reads the committed campaign results `FLINT_vs_symbolica/results/summary.json`
and `table.typ` (`--flint-dir` overrides the directory; both files are required). `table.typ` is
included verbatim; the summary cards, the verdicts and the measurement-status line are computed
from `summary.json` (`by_kind`, `job_states`, `ops_pending`, `best_for_symbolica`,
`best_for_flint`). The committed results are a snapshot (2026-10-12, 12:57 UTC) of a campaign that
kept running afterwards: 30 large-tier operands (126 of 772 jobs) were not measured and are shown
as pending, never as timeouts. To refresh them, resume the campaign and re-run
`FLINT_vs_symbolica/scripts/collect.py` as described in `FLINT_vs_symbolica/README.md`, then
re-run `run_final.sh ... --skip-runs`.

## Definitions used in the tables

* **verified**: a `complete_generic` result accepted by `symgcad verify` in a separate fresh process;
  only these count. Strict default contract unless the row is in the opt-in table.
* **solve s / verify s**: process wall seconds, one pinned CPU, cold caches; solve includes process
  start-up, in-run ordering search, preprocessing and the export of `result.json`. Measured with
  `perf_counter` around the wrapper chain by a worker process of its own per group, pinned to that
  group's CPU (GNU `time`'s 10 ms elapsed is kept as `gnu_elapsed_secs`); the table shows the
  median of the repeats, and a row counts as verified only if every repeat verified the same cell
  count (`rows[].symgcad.solve_s_runs` lists each repeat). On the smallest problems most of the
  time is start-up and export, and the export's file copy on the shared ZFS file system can wait
  for a file-system sync (up to about 0.5 s with 0.01 s of CPU); the repeats damp this.
* **native s**: sum over sign cases of the native process wall time (Python start-up, SymPy parsing,
  native ordering, projection, lifting), taken from GNU `time`'s elapsed time in each case's
  `time.txt` (10 ms resolution; the native runner's own `wall_secs` polls every 0.2 s and is kept
  as `native_runner_wall_s`); `report-data.json` also has the internal time
  (`rows[].native.internal_s`).
* **ratio**: symGCAD solve / native time, only where both complete; the summary gives geometric
  means overall and for groups where native needs ≥ 1 s or ≥ 10 s of internal time.
* **cache**: `cold` = measured by the report run (fresh process, fresh run directory and `TMPDIR`,
  no persistent checkpoints); every verified row is cold. Unresolved rows were not re-attempted and carry the tag of
  the attempt shown.
* **unresolved rows**: the solve column is the longest attempt (lower bound): 2026-10-10 audit (h,
  `hist.`), 2026-10-12 one-hour attempt with a warm shared cache (w, `warm`), or a cold attempt of
  this run (c, `cold`; only with `--attempt-unresolved`).
