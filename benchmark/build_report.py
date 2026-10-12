#!/usr/bin/env python3
"""Build benchmark/report-data.json, the data behind benchmark/benchmark.typ.

Inputs (every file read is recorded with its sha256 in "sources"):
  --manifest    benchmark/campaign/groups.json: the 331 distinct corpus problems, their
                sign cases and the symGCAD recipe of each group.
  --symgcad     one or more symgcad_runner.py output directories (records.jsonl).  Records of
                the same group and the same binary sha256 in different directories are
                repeats (run_final.sh --repeats): the row shows the median solve and verify
                times, and a row counts as verified only if every repeat verified the same
                cell count.  A record of another binary in a later directory replaces the
                earlier ones.  These are the strict-contract rows: only "complete_verified"
                counts as solved.
  --projective  symgcad_runner.py output directories of the opt-in projective-sections
                recipes (groups 298 and 302), repeats as for --symgcad.  Reported separately,
                never counted as strict.
  --native      native GCAD run directory (cases/<group>/<signs>/record.json, 300 s cap
                per sign case).  Partial runs are accepted: missing cases are "not run".
  --unresolved  latest long attempts at the unresolved groups (run_lanes.sh layout:
                <name>/{status.txt,time.txt,result.json,binary.sha256}).
  --history     benchmark/data.json: the frozen 2026-10-10 audit (prior verified
                measurement, longest earlier attempt of unresolved groups).
  --flint-dir   directory with the Symbolica-vs-FLINT table.typ and summary.json.

A group with no record in --symgcad falls back to its --history measurement and is
labelled "history"; it still counts only if data.json verified it.

Times.  symGCAD solve/verify: the runner's perf_counter around the wrapper chain, measured in a
per-group worker process pinned to the group's CPU (records from runners without that isolation
are used as recorded).  Native: per sign case, GNU time's elapsed wall time from the case's
time.txt (10 ms resolution), summed over sign cases; the native runner's own wall_secs polls
every 0.2 s and is kept only as native_runner_wall_s.

Default evidence paths (--unresolved, --projective-evidence) are machine-local; when one is
missing the build warns on stderr and lists it in provenance.missing_inputs, which the PDF
shows as a DRAFT reason.
"""
import argparse
import datetime
import glob
import hashlib
import json
import math
import pathlib
import re
import statistics
import sys
import tomllib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
RESEARCH = pathlib.Path("/common/dev/gcad/artifacts/research/claude-2026-10-12")
SOURCES = {}
MISSING = []


def gnu_elapsed(path):
    """GNU time -v 'Elapsed (wall clock) time' in seconds, or None."""
    try:
        text = pathlib.Path(path).read_text()
    except FileNotFoundError:
        return None
    m = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (\S+)", text)
    if not m:
        return None
    seconds = 0.0
    for part in m.group(1).split(":"):
        seconds = 60 * seconds + float(part)
    return seconds


def require(path, what):
    """Record (and warn about) a missing optional input instead of silently dropping it."""
    if path and not pathlib.Path(path).exists():
        MISSING.append(dict(input=what, path=str(path)))
        print(f"build_report.py: WARNING: {what} not found: {path}", file=sys.stderr)
        return False
    return bool(path)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 22), b""):
            h.update(block)
    return h.hexdigest()


def rel(path):
    """Repository-relative path for files inside the checkout (any worktree), else absolute."""
    p = pathlib.Path(path).resolve()
    try:
        return str(p.relative_to(REPO))
    except ValueError:
        return str(p)


def load_json(path):
    path = pathlib.Path(path)
    SOURCES[rel(path)] = sha256_file(path)
    return json.loads(path.read_text())


def sig(x, digits=3):
    """Significant-digit display; None -> em dash."""
    if x is None or not math.isfinite(x):
        return "—"
    if x == 0:
        return "0"
    decimals = max(0, min(6, digits - 1 - math.floor(math.log10(abs(x)))))
    return f"{x:.{decimals}f}"


def grouped(n):
    return f"{n:,}" if isinstance(n, int) else "—"


def scope_label(group):
    if group["scope"] == "fixed":
        return "fixed"
    return "symbolic" if group["identifier"].endswith("/symbolic") else "core"


def sign_cases(splits):
    if splits == 0:
        return ["none"]
    return ["".join("+" if mask & (1 << i) else "-" for i in range(splits)) for mask in range(1 << splits)]


def run_input(run_root, record):
    """Engine/ordering/anchor actually used, read from the run's input.toml."""
    path = pathlib.Path(run_root) / record.get("run_dir", "")
    toml = path / "input.toml"
    if not record.get("run_dir") or not toml.exists():
        return {}
    solver = tomllib.loads(toml.read_text()).get("solver", {})
    anchor = solver.get("homogeneous_anchor") or solver.get("bilinear_general_base_anchor") \
        or solver.get("monotone_base_anchor") or solver.get("bilinear_positive_base_anchor")
    return dict(engine=solver.get("engine"), ordering=solver.get("ordering"), anchor=anchor,
                base_engine=solver.get("bilinear_general_base_engine") or solver.get("monotone_base_engine")
                or solver.get("bilinear_positive_base_engine"),
                projective_sections=bool(solver.get("projective_sections", False)),
                input_sha256=sha256_file(toml))


def load_runs(directories):
    """group index -> representative record.  Same group + same binary in several directories =
    repeats: the representative is the run with the median solve time, carrying the median solve
    and verify times (_median) and every repeat (_repeats).  Inconsistent repeats (a repeat not
    verified, or different cell counts) make the row unverified, using the first such record."""
    by_group = {}
    launches = []
    for d in directories:
        d = pathlib.Path(d)
        path = d / "records.jsonl"
        if not path.exists():
            continue
        SOURCES[rel(path)] = sha256_file(path)
        if (d / "launches.jsonl").exists():
            for line in (d / "launches.jsonl").read_text().splitlines():
                if line.strip():
                    launches.append(json.loads(line))
        for line in path.read_text().splitlines():
            if line.strip():
                r = json.loads(line)
                r["_root"] = str(d)
                runs = by_group.setdefault(r["group_index"], {})
                if any(x.get("binary_sha256") != r.get("binary_sha256") for x in runs.values()):
                    runs.clear()  # a later binary replaces the earlier measurement
                runs[str(d)] = r  # within one directory the later record wins
    latest = {}
    for gi, runs in by_group.items():
        recs = list(runs.values())
        if len(recs) == 1:
            latest[gi] = recs[0]
            continue
        summary = [dict(record=f"{x['_root']}/records.jsonl#{x.get('run_dir')}", status=x.get("status"),
                        cells=x.get("cells"), solve_s=(x.get("solve") or {}).get("wall_secs"),
                        verify_s=(x.get("verify") or {}).get("wall_secs"), cpu=x.get("cpu"),
                        started=x.get("started")) for x in recs]
        good = all(x.get("status") == "complete_verified" for x in recs) and len({x.get("cells") for x in recs}) == 1
        if good:
            ranked = sorted(recs, key=lambda x: x["solve"]["wall_secs"])
            rep = dict(ranked[(len(ranked) - 1) // 2])
            rep["_median"] = dict(solve=statistics.median(x["solve"]["wall_secs"] for x in recs),
                                  verify=statistics.median(x["verify"]["wall_secs"] for x in recs))
        else:
            rep = dict(next(x for x in recs if x.get("status") != "complete_verified" or x.get("cells") != recs[0].get("cells")))
            if rep.get("status") == "complete_verified":
                rep["status"] = "repeats_disagree"
            rep["_repeats_inconsistent"] = True
        rep["_repeats"] = summary
        latest[gi] = rep
    return latest, launches


def symgcad_cell(record):
    solve, verify = record.get("solve") or {}, record.get("verify") or {}
    used = run_input(record["_root"], record)
    status = record.get("status")
    rss = [x for x in (solve.get("peak_rss_kib"), verify.get("peak_rss_kib")) if x]
    return dict(
        status=status, verified=status == "complete_verified", cells=record.get("cells"),
        raw_cells=(record.get("result") or {}).get("raw_cells"),
        solve_s=(record.get("_median") or {}).get("solve", solve.get("wall_secs")),
        verify_s=(record.get("_median") or {}).get("verify", verify.get("wall_secs") if verify else None),
        repeats=len(record.get("_repeats") or []) or 1,
        solve_s_runs=[x["solve_s"] for x in record["_repeats"]] if record.get("_repeats") else None,
        verify_s_runs=[x["verify_s"] for x in record["_repeats"]] if record.get("_repeats") else None,
        repeats_inconsistent=bool(record.get("_repeats_inconsistent")),
        solve_gnu_elapsed_s=solve.get("gnu_elapsed_secs"),
        peak_rss_mib=max(rss) / 1024 if rss else None,
        order=record.get("order") or [], engine=used.get("engine") or record.get("engine"),
        ordering=used.get("ordering") or record.get("ordering"),
        anchor=used.get("anchor") if used else record.get("anchor"),
        base_engine=used.get("base_engine"), projective_sections=used.get("projective_sections", False),
        timeout_s=record.get("solve_timeout_secs"), memory_mib=record.get("memory_mib"),
        provenance=dict(kind="cold run", binary_sha256=record.get("binary_sha256"),
                        binary=record.get("binary"), cache_policy="cold",
                        cache_detail=record.get("cache_policy"), date=record.get("started"),
                        checkpoint_dir_set=record.get("checkpoint_dir_set"),
                        checkpoint_bytes=record.get("checkpoint_bytes"), timing=record.get("timing"),
                        cpu=record.get("cpu"), load_before=record.get("load_before"),
                        input_sha256=used.get("input_sha256") or record.get("input_sha256"),
                        result_sha256=record.get("result_sha256"),
                        record=f"{record['_root']}/records.jsonl#{record.get('run_dir')}",
                        repeats=record.get("_repeats")))


def history_cell(group, history_row):
    m = (history_row or {}).get("measurement") or group.get("prior_measurement")
    if not m:
        return None
    method = m.get("method") or ""
    return dict(
        status="complete_verified", verified=True, cells=m.get("cells"), raw_cells=None,
        solve_s=m.get("solve_seconds"), verify_s=m.get("verify_seconds"), peak_rss_mib=m.get("peak_rss_mib"),
        order=m.get("order") or [], engine=method.split("+")[0].rstrip("−"),
        ordering=group["recipe"]["ordering"], anchor=group["recipe"]["homogeneous_anchor"] if "+H" in method else None,
        base_engine=None, projective_sections=False, timeout_s=None, memory_mib=None,
        provenance=dict(kind="history (data.json)", binary_sha256=m.get("executable_sha256"),
                        build=m.get("build"), cache_policy=m.get("cache_policy") or "unknown",
                        date="2026-10-10 audit or earlier", repeats=m.get("n"), source=m.get("source")))


def native_group(group, cases_by_group):
    expected = sign_cases(group["splits"])
    got = cases_by_group.get(group["index"], {})
    recs = [got[s] for s in expected if s in got]
    statuses = [r["status"] for r in recs]
    done = [r for r in recs if r["status"] == "complete"]
    if len(done) == len(expected):
        status = "complete"
    elif "timeout" in statuses:
        status = "timeout"
    elif "memory" in statuses:
        status = "memory"
    elif any(s not in ("complete", "timeout", "memory") for s in statuses):
        status = "error"
    else:
        status = "not run"
    complete = status == "complete"
    return dict(
        status=status, cases=len(expected), cases_recorded=len(recs), cases_complete=len(done),
        cases_timeout=statuses.count("timeout"),
        wall_s=sum(r["_elapsed_s"] for r in recs) if complete else None,
        wall_source=("GNU time elapsed" if all(r["_elapsed_src"] == "gnu" for r in recs)
                     else "runner wall (time.txt missing for some cases)") if complete else None,
        native_runner_wall_s=sum(r["wall_secs"] for r in recs) if complete else None,
        internal_s=sum(r.get("adapter_elapsed_secs") or 0 for r in recs) if complete else None,
        raw_cells=sum(r.get("raw_cells") or 0 for r in recs) if complete else None,
        peak_rss_mib=max((r.get("peak_rss_kib") or 0) for r in recs) / 1024 if recs else None,
        orders={r["signs"]: r.get("order") for r in recs if r.get("order")},
        dates=sorted(r.get("start_utc") for r in recs if r.get("start_utc"))[-1:] or None,
        timeout_cap_s=max((r.get("timeout_secs") or 0) for r in recs) if recs else None)


def load_native(run):
    run = pathlib.Path(run)
    cases = {}
    for path in sorted(run.glob("cases/*/*/record.json")):
        rec = json.loads(path.read_text())
        elapsed = gnu_elapsed(path.parent / "time.txt")
        rec["_elapsed_s"], rec["_elapsed_src"] = (elapsed, "gnu") if elapsed is not None else (rec["wall_secs"], "runner")
        cases.setdefault(rec["group_index"], {})[rec["signs"]] = rec
    provenance = []
    for path in sorted(run.glob("provenance-*.json")):
        p = load_json(path)
        if "native" not in p:
            # partial re-launch of a few cases with the same runner (e.g. the 2026-10-12 re-run of two
            # sign cases that hit std::bad_alloc under the 8 GiB cap, repeated with 24 GiB)
            provenance.append(dict(file=str(path), launched_utc=p.get("launched_utc"), kind="rerun",
                                   purpose=p.get("purpose"), command=p.get("command"),
                                   runner_sha256=p.get("runner_sha256"), cpus=p.get("cpus")))
            continue
        provenance.append(dict(file=str(path), launched_utc=p.get("launched_utc"), kind="launch",
                               upstream_revision=p["native"].get("upstream_revision"),
                               upstream_author=p["native"].get("upstream_author"),
                               extension_sha256=p["native"].get("extension_sha256"),
                               bridge_sha256=p["bridge"].get("sha256"),
                               runner_sha256=p["runner"].get("sha256"),
                               protocol=p.get("protocol"), host=p.get("host")))
    return cases, provenance


def unresolved_attempt(directory, group):
    """Latest long attempt (run_lanes.sh layout); None when absent."""
    d = pathlib.Path(directory) / group["identifier"].replace("/", "_")
    if not (d / "input.toml").exists():
        return None  # never attempted (or a launch whose input was withdrawn before it started)
    status = (d / "status.txt").read_text() if (d / "status.txt").exists() else ""
    out = dict(directory=str(d), cache_policy="warm (shared artifacts/checkpoints cache)")
    m = re.search(r"start=([\d.]+) end=([\d.]+)", status)
    if (d / "binary.sha256").exists():
        out["binary_sha256"] = (d / "binary.sha256").read_text().split()[0]
    if (d / "input.toml").exists():
        limits = tomllib.loads((d / "input.toml").read_text()).get("limits", {})
        out["limit_s"] = limits.get("wall_time_secs")
        out["memory_mib"] = limits.get("memory_mib")
    if "done" not in status or not m:
        out["state"] = "running" if (d / "input.toml").exists() else "queued"
        return out
    out["wall_s"] = float(m.group(2)) - float(m.group(1))
    out["date"] = datetime.datetime.fromtimestamp(float(m.group(1)), datetime.timezone.utc).isoformat(timespec="seconds")
    if (d / "time.txt").exists():
        rss = re.search(r"Maximum resident set size \(kbytes\): (\d+)", (d / "time.txt").read_text())
        out["peak_rss_mib"] = int(rss.group(1)) / 1024 if rss else None
    head = ""
    if (d / "result.json").exists():
        with open(d / "result.json", "rb") as f:
            head = f.read(4 << 20).decode("utf-8", "replace")
    st = re.search(r'"status":"([^"]+)"', head[:400])
    out["state"] = st.group(1) if st else "no result"
    orders = [o for o in re.findall(r'"order":\[([^\]]*)\]', head) if o.strip()]
    if orders:
        out["order"] = json.loads("[" + orders[0] + "]")
    if out["state"] == "complete_generic":
        verify = (d / "verify.txt").read_text() if (d / "verify.txt").exists() else ""
        out["state"] = "complete_verified" if '"verified": true' in verify else "complete_unverified"
        cells = re.search(r'"cells":\s*(\d+)', verify)
        out["cells"] = int(cells.group(1)) if cells else None
        vt = re.search(r"vstart=([\d.]+) vend=([\d.]+)", status)
        out["verify_s"] = float(vt.group(2)) - float(vt.group(1)) if vt else None
        solver = tomllib.loads((d / "input.toml").read_text()).get("solver", {}) if (d / "input.toml").exists() else {}
        out.update(engine=solver.get("engine"), ordering=solver.get("ordering"),
                   anchor=solver.get("homogeneous_anchor"))
    return out


def long_attempt_cell(attempt):
    """A verified long attempt counts as a (warm-cache) verified result; never silently as cold."""
    return dict(
        status="complete_verified", verified=True, cells=attempt.get("cells"), raw_cells=None,
        solve_s=attempt.get("wall_s"), verify_s=attempt.get("verify_s"), peak_rss_mib=attempt.get("peak_rss_mib"),
        order=attempt.get("order") or [], engine=attempt.get("engine"), ordering=attempt.get("ordering"),
        anchor=attempt.get("anchor"), base_engine=None, projective_sections=False,
        timeout_s=attempt.get("limit_s"), memory_mib=attempt.get("memory_mib"),
        provenance=dict(kind="warm long attempt", binary_sha256=attempt.get("binary_sha256"),
                        cache_policy="warm", cache_detail=attempt.get("cache_policy"), date=attempt.get("date"),
                        record=attempt.get("directory")))


def method_label(cell):
    engine = cell.get("engine") or "?"
    if cell.get("base_engine") and engine in ("bilinear_general", "bilinear_positive", "monotone") \
            and cell["base_engine"] not in ("baseline",):
        engine = f"{engine}/{cell['base_engine']}"
    return engine


def ratio_text(x):
    """Two significant digits, three where two would round onto a colour boundary (1 or 2)."""
    if x is None:
        return "—"
    s = sig(x, 2)
    if float(s) in (1.0, 2.0) and x != float(s):
        s = sig(x, 3)
    return s


def ratio_class(x):
    if x is None:
        return "none"
    return "fast" if x < 1 else ("par" if x <= 2 else "slow")


def geomean(xs):
    xs = [x for x in xs if x and x > 0]
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", default=str(HERE / "campaign/groups.json"))
    ap.add_argument("--history", default=str(HERE / "data.json"))
    ap.add_argument("--symgcad", nargs="+", required=True)
    ap.add_argument("--projective", nargs="+")
    ap.add_argument("--projective-evidence", default=str(RESEARCH / "projective-sections/measurements.json"))
    ap.add_argument("--native", required=True)
    ap.add_argument("--unresolved", default=str(RESEARCH / "unresolved/v1-nopw-retain"))
    ap.add_argument("--flint-dir", help="default: benchmark/FLINT_vs_symbolica/results (committed campaign results)")
    ap.add_argument("--output", default=str(HERE / "report-data.json"))
    args = ap.parse_args()

    manifest = load_json(args.manifest)
    history = load_json(args.history)
    hist_rows = {r["identifier"]: r for r in history["rows"]}
    runs, launches = load_runs(args.symgcad)
    proj_runs, proj_launches = load_runs(args.projective) if args.projective else ({}, [])
    native_cases, native_prov = load_native(args.native)
    have_unresolved = require(args.unresolved, "long attempts at the unresolved groups (--unresolved)")

    rows = []
    for g in manifest["entries"]:
        hist = hist_rows.get(g["identifier"])
        rec = runs.get(g["index"])
        cell = symgcad_cell(rec) if rec else (history_cell(g, hist) if g["status_in_data_json"] == "verified" else None)
        if not (cell and cell["verified"]) and have_unresolved:
            attempt = unresolved_attempt(args.unresolved, g)
            if attempt and attempt.get("state") == "complete_verified":
                cell = long_attempt_cell(attempt)
        nat = native_group(g, native_cases)
        row = dict(index=g["index"], identifier=g["identifier"], scope=scope_label(g),
                   aliases=g["aliases"], original_input=g["original_input"], original_sha256=g["original_sha256"],
                   sign_cases=g["sign_cases"], data_json_status=g["status_in_data_json"],
                   symgcad=cell, native=nat)
        row["verified"] = bool(cell and cell["verified"])
        prior = g.get("prior_measurement")
        row["history"] = dict(status=g["status_in_data_json"], cells=prior.get("cells"), order=prior.get("order"),
                              solve_s=prior.get("solve_seconds"), build=prior.get("build")) if prior else \
            dict(status=g["status_in_data_json"])
        row["cells_differ_from_history"] = bool(row["verified"] and prior and prior.get("cells") is not None
                                                and cell["provenance"]["kind"] == "cold run"
                                                and prior["cells"] != cell["cells"])
        if row["cells_differ_from_history"]:
            # Why the count moved: the recipe input is the one the audit measured (same ledger run, or same
            # executable as the measurement) and the exported order is unchanged -> only the code changed.
            src = ((hist or {}).get("measurement") or {}).get("source") or {}
            rp = g["recipe"].get("provenance") or {}
            same_recipe = bool((src.get("ledger") and src.get("ledger") == rp.get("ledger")
                                and src.get("run_index") == rp.get("run_index"))
                               or (prior.get("executable_sha256")
                                   and prior.get("executable_sha256") == rp.get("executable_sha256")))
            same_order = (prior.get("order") or []) == (cell.get("order") or [])
            row["cells_change_reason"] = "code only" if same_recipe and same_order else \
                ("order changed" if not same_order else "recipe changed")
        if not row["verified"]:
            attempt = unresolved_attempt(args.unresolved, g) if have_unresolved else None
            prior = dict(longest_s=(hist or {}).get("longest_incomplete_seconds"),
                         last_status=(hist or {}).get("last_status"),
                         peak_rss_mib=(hist or {}).get("incomplete_peak_rss_mib"),
                         date="2026-10-10 or earlier (data.json)") if hist else None
            row["attempts"] = dict(cold=cell, long=attempt, history=prior)
        ratio = None
        if row["verified"] and nat["status"] == "complete" and cell["solve_s"] and nat["wall_s"]:
            ratio = cell["solve_s"] / nat["wall_s"]
            row["ratio_internal"] = cell["solve_s"] / nat["internal_s"] if nat["internal_s"] else None
        row["ratio"] = ratio
        rows.append(row)

    verified = sorted((r for r in rows if r["verified"]), key=lambda r: (r["symgcad"]["solve_s"], r["index"]))
    incomplete = sorted((r for r in rows if not r["verified"]), key=lambda r: r["index"])
    for rank, r in enumerate(verified + incomplete, 1):
        r["rank"] = rank
    ordered = verified + incomplete

    # display strings for the table
    for r in ordered:
        c = r["symgcad"]
        n = r["native"]
        d = {}
        g = next(x for x in manifest["entries"] if x["index"] == r["index"])
        if r["verified"]:
            d["status"] = "verified" if c["provenance"]["kind"] == "cold run" else "verified (hist.)"
            d["cells"], d["solve"], d["verify"] = grouped(c["cells"]), sig(c["solve_s"]), sig(c["verify_s"])
        else:
            # longest attempt among: cold attempt of this run (c), 2026-10-12 long attempt (w),
            # 2026-10-10 audit history (h); a lower bound, never a solve time
            a = r["attempts"]
            cands = []
            if a["cold"] and a["cold"].get("solve_s"):
                cands.append((a["cold"]["solve_s"], "c"))
            if a["long"] and a["long"].get("wall_s"):
                cands.append((a["long"]["wall_s"], "w"))
            if a["history"] and a["history"].get("longest_s"):
                cands.append((a["history"]["longest_s"], "h"))
            longest = max(cands) if cands else None
            d["status"] = "unresolved"
            d["cells"], d["solve"], d["verify"] = "—", "—", "—"
            d["longest"] = f"> {longest[0]:.0f}" if longest else "—"
            d["longest_tag"] = longest[1] if longest else ""
            if a["long"] and a["long"].get("state") in ("running", "queued"):
                d["longest_note"] = f"2026-10-12 attempt {a['long']['state']}"
        if c:
            d["method"] = method_label(c)
            d["ordering"] = c.get("ordering") or ""
            order = c.get("order")
            d["anchor"] = c.get("anchor") or ""
        else:
            d["method"] = g["recipe"]["engine"]
            d["ordering"] = g["recipe"]["ordering"]
            order = g["recipe"]["order"]
            d["anchor"] = g["recipe"]["homogeneous_anchor"] or ""
        if not order and not r["verified"]:
            order = (r["attempts"]["long"] or {}).get("order")
            d["order_note"] = "order of the 2026-10-12 attempt" if order else ""
        d["order"] = " ".join(order) if order else "—"
        d["cells_mark"] = "†" if r["cells_differ_from_history"] else ""
        if n["status"] == "complete":
            d["native"] = sig(n["wall_s"])
        elif n["status"] == "not run":
            d["native"] = "not run" if n["cases_recorded"] == 0 else f"not run ({n['cases_complete']}/{n['cases']})"
        elif n["status"] == "timeout":
            d["native"] = "timeout"
        else:
            d["native"] = n["status"]
        d["ratio"] = ratio_text(r["ratio"])
        d["ratio_class"] = ratio_class(r["ratio"])
        r["display"] = d

    # opt-in projective rows
    evidence = None
    if args.projective and require(args.projective_evidence,
                                   "projective-sections sanity evidence (--projective-evidence)"):
        evidence = load_json(args.projective_evidence)
    opt_in = []
    for index, label in ((302, "positive Gluza33 (test_gluza_ex33/solve/3)"),
                         (298, "positive Gluza30 (test_gluza_ex30/solve/3)")):
        rec = proj_runs.get(index)
        strict = next(r for r in rows if r["index"] == index)
        entry = dict(index=index, identifier=strict["identifier"], label=label,
                     contract="opt-in projective contract (solver.projective_sections = true)",
                     strict_status="verified" if strict["verified"] else "unresolved",
                     strict_cells=strict["symgcad"]["cells"] if strict["verified"] else None,
                     strict_solve_s=strict["symgcad"]["solve_s"] if strict["verified"] else None,
                     native=strict["native"])
        if rec:
            cell = symgcad_cell(rec)
            entry["symgcad"] = cell
            if index == 302 and evidence:
                # seeded interior sanity check of the projective-sections track (same recipe file)
                g33 = evidence.get("gluza33_positive", {}).get("x5last", {})
                san = g33.get("sanity") or {}
                track_binary = ((evidence.get("binaries") or {}).get("symgcad") or {}).get("sha256")
                # The check ran on the track's result.json; when this run's cells are exactly the
                # same (samples, bounds, signs), the check applies to this run's decomposition too.
                checked = pathlib.Path(args.projective_evidence).parent / (g33.get("input") or "g33_x5last/input.toml")
                checked = checked.parent / "result.json"
                ours = pathlib.Path(rec["_root"]) / rec.get("run_dir", "") / "result.json"
                identical = None
                if checked.exists() and ours.exists():
                    identical = json.loads(ours.read_text()).get("cells") == json.loads(checked.read_text()).get("cells")
                    SOURCES[rel(checked)] = sha256_file(checked)
                entry["sanity"] = dict(points=san.get("points_checked"), failures=san.get("failures"),
                                       seed=san.get("seed"), wall_s=san.get("wall_s"),
                                       cells_checked=san.get("cells_checked"), binary_sha256=track_binary,
                                       same_binary=track_binary == cell["provenance"].get("binary_sha256"),
                                       checked_result=str(checked), cells_identical_to_checked=identical,
                                       evidence=args.projective_evidence)
        opt_in.append(entry)

    # summary
    by_scope = {}
    for s in ("fixed", "symbolic", "core"):
        rs = [r for r in rows if r["scope"] == s]
        by_scope[s] = dict(expected=len(rs), verified=sum(r["verified"] for r in rs),
                           native_complete=sum(r["native"]["status"] == "complete" for r in rs))
    both = [r for r in rows if r["ratio"] is not None]
    nat_status = {}
    for r in rows:
        nat_status[r["native"]["status"]] = nat_status.get(r["native"]["status"], 0) + 1
    cases_recorded = sum(r["native"]["cases_recorded"] for r in rows)
    cases_status = {}
    for recs in native_cases.values():
        for rec in recs.values():
            cases_status[rec["status"]] = cases_status.get(rec["status"], 0) + 1
    cold = [r for r in rows if r["symgcad"] and r["symgcad"]["provenance"]["kind"] == "cold run"]
    binaries = {}
    for r in cold:
        b = r["symgcad"]["provenance"]["binary_sha256"]
        binaries[b] = binaries.get(b, 0) + 1
    dates = sorted(d for r in cold for d in ([x["started"] for x in r["symgcad"]["provenance"]["repeats"]]
                                             if r["symgcad"]["provenance"].get("repeats")
                                             else [r["symgcad"]["provenance"]["date"]]) if d)
    repeats = sorted({r["symgcad"]["repeats"] for r in cold})
    cold_ids = {r["index"] for r in cold}

    def tier(threshold):
        sel = [r for r in both if r["native"]["internal_s"] >= threshold]
        return dict(native_internal_at_least_s=threshold, n=len(sel),
                    geomean_solve_over_native_wall=geomean([r["ratio"] for r in sel]),
                    geomean_solve_over_native_internal=geomean([r.get("ratio_internal") for r in sel]))
    summary = dict(
        groups=len(rows), sign_cases=sum(r["sign_cases"] for r in rows),
        strict=dict(verified=sum(r["verified"] for r in rows), expected=len(rows),
                    verified_cold=sum(r["verified"] for r in cold),
                    verified_history_only=sum(r["verified"] and r["index"] not in cold_ids for r in rows),
                    groups_cold_run=len(cold), groups_without_cold_run=len(rows) - len(cold),
                    verified_groups_without_cold_run=sum(r["data_json_status"] == "verified" and r["index"] not in cold_ids
                                                         for r in rows),
                    unresolved_groups_without_cold_attempt=sum(r["data_json_status"] != "verified"
                                                               and r["index"] not in cold_ids for r in rows),
                    cells_differ_from_history=sum(r["cells_differ_from_history"] for r in rows),
                    cells_change_reasons={k: sum(r.get("cells_change_reason") == k for r in rows)
                                          for k in ("order changed", "recipe changed", "code only")},
                    unresolved=sum(not r["verified"] for r in rows), by_scope=by_scope,
                    newly_verified=[r["identifier"] for r in rows if r["verified"] and r["data_json_status"] != "verified"],
                    lost=[r["identifier"] for r in rows if not r["verified"] and r["data_json_status"] == "verified"]),
        opt_in=dict(rows=len(opt_in), completed=sum(bool(e.get("symgcad") and e["symgcad"]["verified"]) for e in opt_in),
                    new_completions=[e["identifier"] for e in opt_in if e.get("symgcad") and e["symgcad"]["verified"]
                                     and e["strict_status"] != "verified"],
                    counted_in_strict=False),
        symgcad_binaries=binaries, symgcad_run_dates=[dates[0], dates[-1]] if dates else None,
        symgcad_repeats=repeats,
        symgcad_repeats_inconsistent=[r["identifier"] for r in cold if r["symgcad"].get("repeats_inconsistent")],
        native=dict(groups=nat_status, cases_total=sum(r["sign_cases"] for r in rows), cases_recorded=cases_recorded,
                    cases_status=cases_status, timeout_cap_s=300,
                    reruns=[dict(purpose=p.get("purpose"), launched_utc=p.get("launched_utc"))
                            for p in native_prov if p.get("kind") == "rerun"],
                    groups_timeout=nat_status.get("timeout", 0)),
        comparison=dict(
            n=len(both),
            geomean_solve_over_native_wall=geomean([r["ratio"] for r in both]),
            geomean_solve_over_native_internal=geomean([r.get("ratio_internal") for r in both]),
            symgcad_solve_sum_s=sum(r["symgcad"]["solve_s"] for r in both),
            native_wall_sum_s=sum(r["native"]["wall_s"] for r in both),
            native_runner_wall_sum_s=sum(r["native"]["native_runner_wall_s"] for r in both),
            native_wall_source=sorted({r["native"]["wall_source"] for r in both}),
            symgcad_checkpoint_dir_rows=sum(bool(r["symgcad"]["provenance"].get("checkpoint_dir_set")) for r in both),
            native_internal_sum_s=sum(r["native"]["internal_s"] for r in both),
            tiers=[tier(1.0), tier(10.0)],
            symgcad_faster=sum(r["ratio"] < 1 for r in both),
            ratio_above_2=sum(r["ratio"] > 2 for r in both),
            verified_where_native_fails=sum(r["verified"] and r["native"]["status"] in ("timeout", "memory", "error")
                                            for r in rows),
            native_completes_symgcad_not=[r["identifier"] for r in rows
                                          if not r["verified"] and r["native"]["status"] == "complete"]))

    # Symbolica vs FLINT
    flint_dir = (pathlib.Path(args.flint_dir) if args.flint_dir else HERE / "FLINT_vs_symbolica/results").resolve()
    flint = None
    if (flint_dir / "summary.json").exists() and (flint_dir / "table.typ").exists():
        fs = load_json(flint_dir / "summary.json")
        SOURCES[rel(flint_dir / "table.typ")] = sha256_file(flint_dir / "table.typ")
        states = fs.get("job_states") or {}
        flint = dict(table=str((flint_dir / "table.typ").relative_to(HERE)), summary=rel(flint_dir / "summary.json"),
                     draft=False, n_ops=fs.get("n_ops"), n_jobs=fs.get("jobs"),
                     jobs_run=sum(v for k, v in states.items() if k != "pending"),
                     jobs_pending=states.get("pending", 0), jobs_capped=states.get("timeout", 0),
                     ops_pending=fs.get("ops_pending"),
                     outputs_compared=fs.get("outputs_compared"), mismatches=len(fs.get("mismatches") or []),
                     ratio_definition=fs.get("ratio_definition"),
                     by_kind=[dict(kind=k["kind"], n=k["n"], n_ratio=k["n_ratio"],
                                   geomean=k.get("geomean_production_over_flint"),
                                   total=k.get("time_weighted_production_over_flint"))
                              for k in fs.get("by_kind", [])],
                     best_for_symbolica=fs.get("best_for_symbolica", [])[:3],
                     best_for_flint=fs.get("best_for_flint", [])[:3])

    doc = dict(
        schema_version=2,
        title="symGCAD corpus benchmark",
        generated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        generator=dict(path="benchmark/build_report.py", sha256=sha256_file(__file__)),
        definitions=dict(
            verified="strict default contract: complete_generic result accepted by the independent "
                     "`symgcad verify` in a fresh process (cold run) or in the data.json audit (history)",
            solve_s="fresh-process wall seconds of `symgcad solve` (includes process start, in-run ordering "
                    "search, preprocessing and export of result.json), one pinned CPU, cold: fresh run directory "
                    "and TMPDIR, no persistent checkpoints (a fresh checkpoint_dir only for MCTS searches); "
                    "perf_counter around the wrapper chain in a per-group worker process pinned to that CPU; "
                    "median over the repeats (solve_s_runs), each of which must verify the same cell count",
            verify_s="fresh-process wall seconds of `symgcad verify`, measured the same way (median over the repeats)",
            native_s="sum over sign cases of the native GCAD process wall time (Python start, SymPy parsing, "
                     "native ordering, projection, lifting), GNU time elapsed per case (10 ms resolution); "
                     "each case capped at 300 s",
            ratio="symGCAD solve_s / native_s; only where both complete",
            order="variable order of the exported result (projection eliminates from the end); "
                  "H = homogeneous anchor fixed to 1 before solving",
            opt_in="opt-in projective contract: cell boundaries may reach +infinity inside a cell "
                   "(solver.projective_sections); never counted in the strict completion count"),
        summary=summary,
        provenance=dict(symgcad_launches=launches, projective_launches=proj_launches, native=native_prov,
                        missing_inputs=MISSING,
                        native_run=str(args.native), unresolved_attempts=args.unresolved,
                        manifest=rel(args.manifest), history=rel(args.history)),
        rows=ordered, opt_in=opt_in, flint=flint, sources=SOURCES)
    pathlib.Path(args.output).write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps(dict(summary=summary, flint=flint and {k: flint[k] for k in ("table", "draft", "n_ops")}),
                     indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
