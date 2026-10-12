# Benchmark report

`benchmark.pdf` (source `benchmark.typ`) has three parts:

1. **Benchmark results**: every one of the 331 distinct corpus problems, solved by symGCAD in a
   fresh process with cold caches and re-checked by `symgcad verify`, next to the original GCAD
   by Vitaly Magerya (300 s cap per sign case). Per row: status, engine and ordering policy, the
   variable order that was picked (with the homogeneous anchor), verified cells, solve and verify
   seconds, native seconds or `timeout`/`not run`, and the time ratio. Rows that complete only
   under the opt-in projective contract (`solver.projective_sections = true`) are in a separate
   table and never count towards the strict total.
2. **Improvements over the original GCAD**: `improvements.typ`, one line per item with its effect
   and whether it is on by default. `improvements-sources.txt` maps each item ID to code, tests,
   reference notes and session reports.
3. **Symbolica vs FLINT** on captured resultants, discriminants, factorizations and GCDs.

`ordering-mcts.typ`/`.pdf` is a separate, independently authored companion and is not generated.

## Files

| Path | Role |
| --- | --- |
| `build_report.py` | Builds `report-data.json` from the runs below (stdlib Python ≥ 3.11). |
| `report-data.json` | Generated table data with per-row provenance (binary sha256, input sha256, cache policy, date, record path) and a `sources` map of every file read with its sha256. |
| `run_final.sh` | One command: cold symGCAD runs, opt-in projective rows, `report-data.json`, PDF. |
| `campaign/groups.json` | Manifest: the 331 groups, aliases, original corpus inputs (+sha256), sign cases, native ordering policy, symGCAD recipe and the 2026-10-10 measurement. |
| `campaign/symgcad_inputs/` | The symGCAD recipe of every group (engine, ordering, options, limits). |
| `campaign/projective/` | Opt-in recipes for positive Gluza33 (group 302) and positive Gluza30 (group 298). |
| `campaign/symgcad_runner.py` | Cold runner: fresh checkpoint directory and `TMPDIR` per run, one pinned CPU, `prlimit`, GNU `time`, solve then independent verify; resumable `records.jsonl`. |
| `campaign/native_runner.py` | Native GCAD runner: sign expansion as in `src/benchmark.rs`, unmodified `.reference/gcad` through `reference/adapter.py`, 300 s / 8 GiB per case. |
| `campaign/build_manifest.py`, `campaign/symgcad_summarize.py` | How `groups.json` was built; per-group summary of runner output. |
| `flint-draft/` | Snapshot of the still-running Symbolica-vs-FLINT campaign; used only while `FLINT_vs_symbolica/results/` does not exist. |
| `data.json`, `followups.json` | Frozen 2026-10-10 audit snapshot (from `examples/benchmark_report.rs`); read for history (earlier measurements, longest earlier attempts of unresolved problems). |

The campaign scripts are verbatim copies of the ones used on 2026-10-12 (they assume this host:
repository at `/common/dev/gcad`, GNU time at `/run/current-system/sw/bin/time`, the reference
Python of `nix develop .#reference`).

## Regenerate the final report

Prerequisites: Nix, the Symbolica license environment in `/common/dev/gcad/.symgcad-env`
(override with `SYMGCAD_ENV=...`), and a release `symgcad` binary of the commit being reported.

```sh
benchmark/run_final.sh /path/to/release/symgcad --cpus 376,377 --parallel 2
```

This runs, in order:

1. all 303 groups verified in the 2026-10-10 audit, cold, `--timeout 600` s each, into
   `artifacts/bench-report/symgcad-<sha12>/symgcad/` (about 7 minutes at parallel 2);
   add `--attempt-unresolved 300` to also give the 28 unresolved groups a cold 300 s attempt;
2. the two opt-in projective rows into `.../projective/`;
3. `build_report.py` → `benchmark/report-data.json`;
4. `typst compile benchmark/benchmark.typ benchmark/benchmark.pdf` (Typst from `flake.lock`, 0.15.1).

Every stage is resumable (records are keyed by binary sha256). `--skip-runs` only rebuilds the
data and the PDF; `--out DIR` chooses the run directory; `--native DIR` points to the native run.
The title page lists every reason the report is still a draft (groups without a cold run, mixed
binaries, native run incomplete, FLINT snapshot, opt-in rows not run); a final report has none.

Direct use of the builder:

```sh
nix develop .#reference --command python3 -I benchmark/build_report.py \
    --symgcad RUN/symgcad [MORE_RUN_DIRS...] --projective RUN/projective \
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
`memory` as soon as one case failed that way, and `not run` otherwise.

## Symbolica vs FLINT section

`build_report.py` uses `FLINT_vs_symbolica/results/{table.typ,summary.json}` when that directory
exists (committed by the FLINT_vs_symbolica campaign), otherwise the snapshot in `flint-draft/`;
`--flint-dir` overrides both. `table.typ` is included verbatim; the summary cards and the verdicts
are computed from `summary.json` (`by_kind`, `best_for_symbolica`, `best_for_flint`).

## Definitions used in the tables

* **verified**: a `complete_generic` result accepted by `symgcad verify` in a separate fresh process;
  only these count. Strict default contract unless the row is in the opt-in table.
* **solve s / verify s**: process wall seconds, one pinned CPU, cold caches; solve includes in-run
  ordering search, preprocessing and export.
* **native s**: sum over sign cases of the native process wall time (Python start-up, SymPy parsing,
  native ordering, projection, lifting); `report-data.json` also has the internal time
  (`rows[].native.internal_s`).
* **ratio**: symGCAD solve / native time, only where both complete; the summary gives geometric
  means overall and for groups where native needs ≥ 1 s or ≥ 10 s of internal time.
* **unresolved rows**: the solve column is the longest attempt (lower bound): 2026-10-10 audit (h),
  2026-10-12 one-hour attempt with a warm shared cache (w), or a cold attempt of this run (c).
