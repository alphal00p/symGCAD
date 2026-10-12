#!/usr/bin/env python3
"""Run symGCAD solve + independent verify on the per-group recipe TOMLs.

Usage (normally via run_symgcad.sh, which sources the license env):
  symgcad_runner.py --binary BIN --parallel N --out DIR
      [--cpus 366,367,368] [--manifest groups.json] [--inputs symgcad_inputs]
      [--only ID_OR_INDEX ...] [--timeout 600] [--timeout-unresolved T]
      [--verify-timeout T] [--memory-mib M] [--keep-results small|all|none]

Per group (one fresh process each, cold caches):
  * input.toml = the recipe verbatim, except [limits].wall_time_secs := timeout
    (and memory_mib := --memory-mib when given); only an MCTS search without a
    fixed order (the one case where the solver itself switches persistent
    checkpoints on, defaulting to the shared ./artifacts/checkpoints) gets a
    fresh per-run [solver].checkpoint_dir = <run>/checkpoints.  Every other
    recipe runs as written, without persistent checkpoint writes inside the
    timed solve (item K2 is opt-in);
  * GNU time -v -> taskset -c CPU -> prlimit --as (memory + 2 GiB backstop for
    the supervising parent; the solver itself enforces memory_mib on its
    worker) -> timeout -k 10 T -> BIN solve input.toml --output result.json;
    cwd = run dir, TMPDIR = run dir/tmp;
  * if status complete_generic: same pinning, BIN verify result.json;
  * the whole group (timing, result parsing, hashing, clean-up) runs in its own
    Python worker process pinned to the group's CPU, so parsing a large result
    of one group can never delay the clock of another (the GIL is per process);
  * record: status, wall (perf_counter in that worker, around the wrapper
    chain; GNU time's own 10 ms elapsed is kept as gnu_elapsed_secs), user/sys
    CPU and max RSS of the process tree (GNU time), verified cell count,
    raw_cells, chosen order, internal statistics (ordering/search/projection/
    lifting seconds), hashes.
Resumable: groups with a record in <out>/records.jsonl for the same binary
sha256 are skipped.  Parallelism re-read from <out>/parallel.txt if present.
"""
import argparse
import concurrent.futures
import datetime
import hashlib
import json
import multiprocessing
import os
import pathlib
import re
import shutil
import subprocess
import sys
import threading
import time
import tomllib

BENCH = pathlib.Path(__file__).resolve().parent
GNU_TIME = "/run/current-system/sw/bin/time"
FULL_PARSE_LIMIT = 96 * 1024 * 1024


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="milliseconds")


def sha256(path, chunk=1 << 22):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def parse_time(path):
    info = {}
    try:
        for line in pathlib.Path(path).read_text().splitlines():
            line = line.strip()
            if line.startswith("User time (seconds):"):
                info["user_cpu_secs"] = float(line.split(":")[-1])
            elif line.startswith("System time (seconds):"):
                info["system_cpu_secs"] = float(line.split(":")[-1])
            elif line.startswith("Maximum resident set size (kbytes):"):
                info["peak_rss_kib"] = int(line.split(":")[-1])
            elif line.startswith("Elapsed (wall clock) time"):
                seconds = 0.0
                for part in line.rsplit(" ", 1)[1].split(":"):
                    seconds = 60 * seconds + float(part)
                info["gnu_elapsed_secs"] = seconds
            elif line.startswith("Exit status:"):
                info["exit_status"] = int(line.split(":")[-1])
            elif line.startswith("Command terminated by signal"):
                info["signal"] = int(line.split()[-1])
    except FileNotFoundError:
        pass
    return info


def solver_enables_checkpoints(solver):
    """Mirror of src/main.rs: `solve` switches persistent checkpoints on by itself only for an
    MCTS search without a fixed order (enable_search_checkpoints, default ./artifacts/checkpoints)."""
    return solver.get("ordering") == "mcts" and not solver.get("order")


def prepare_input(recipe_text, run_dir, timeout, memory_mib):
    text = recipe_text
    if re.search(r'^checkpoint_dir\s*=', text, re.M):
        raise ValueError("recipe already sets checkpoint_dir")
    ckpt = run_dir / "checkpoints"
    # Only where the solver would otherwise write to the shared ./artifacts/checkpoints: a fresh
    # per-run directory keeps that run cold.  Other recipes keep the default (no persistent
    # checkpoints), so no checkpoint I/O is timed that a default solve would not do.
    use_ckpt = solver_enables_checkpoints(tomllib.loads(recipe_text).get("solver", {}))
    if use_ckpt:
        text, n = re.subn(r'^\[solver\]\n', f'[solver]\ncheckpoint_dir = "{ckpt}"\n', text, count=1, flags=re.M)
        if n != 1:
            text = text.rstrip("\n") + f'\n\n[solver]\ncheckpoint_dir = "{ckpt}"\n'
    m = re.search(r'^\[limits\]\n((?:(?!\[).*\n?)*)', text, re.M)
    limits = tomllib.loads(m.group(0)) ["limits"] if m else {}
    limits["wall_time_secs"] = float(timeout)
    if memory_mib:
        limits["memory_mib"] = int(memory_mib)
    limits.setdefault("memory_mib", 8192)
    limits["workers"] = 1
    block = "[limits]\n" + "".join(
        f"{k} = {json.dumps(v) if not isinstance(v, float) else repr(v)}\n" for k, v in limits.items())
    text = (text[:m.start()] + block + text[m.end():]) if m else text.rstrip("\n") + "\n\n" + block
    parsed = tomllib.loads(text)  # must stay valid
    assert parsed.get("solver", {}).get("checkpoint_dir") == (str(ckpt) if use_ckpt else None)
    return text, parsed["limits"], use_ckpt


def summarize_result(path):
    """status/order from the head (top-level keys precede 'cells'), full
    statistics only for moderately sized files."""
    size = path.stat().st_size
    out = dict(result_bytes=size)
    with open(path, "rb") as f:
        head = f.read(8 << 20).decode("utf-8", "replace")
    idx = head.find(',"normalized_constraints":')
    if idx > 0:
        try:
            top = json.loads(head[:idx] + "}")
            out.update(status=top.get("status"), order=top.get("order"))
        except json.JSONDecodeError:
            pass
    if "status" not in out:
        m = re.match(r'\{"schema_version":\d+,"status":"([^"]+)"', head)
        if m:
            out["status"] = m.group(1)
    if size <= FULL_PARSE_LIMIT:
        try:
            data = json.loads(path.read_bytes())
            st = data.get("statistics") or {}
            out.update(status=data.get("status"), order=data.get("order"),
                       raw_cells=st.get("raw_cells"),
                       statistics={k: st.get(k) for k in ("ordering_seconds", "search_seconds", "projection_seconds",
                                                          "lifting_seconds", "merging_seconds", "total_seconds",
                                                          "raw_cells", "visited_stacks", "rejected_branches")},
                       homogeneous_anchor=(data.get("homogeneous") or {}).get("anchor"),
                       diagnostic=(json.dumps(data.get("diagnostic"))[:2000] if data.get("diagnostic") else None))
        except Exception as e:  # keep the head summary
            out["parse_error"] = repr(e)
    else:
        # Large result: the cells array dominates.  Decode only the last
        # '"statistics":{...}' object in the final 64 MiB.  For a homogeneous
        # (anchor) result this is the reduced problem's statistics, where the
        # whole decomposition runs; the source is labelled accordingly.
        with open(path, "rb") as f:
            f.seek(max(0, size - (64 << 20)))
            tail = f.read().decode("utf-8", "replace")
        pos = tail.rfind('"statistics":{')
        if pos >= 0:
            try:
                st, _ = json.JSONDecoder().raw_decode(tail, pos + len('"statistics":'))
                out.update(raw_cells=st.get("raw_cells"),
                           statistics={k: st.get(k) for k in ("ordering_seconds", "search_seconds",
                                                              "projection_seconds", "lifting_seconds",
                                                              "merging_seconds", "total_seconds", "raw_cells",
                                                              "visited_stacks", "rejected_branches")},
                           statistics_source="last statistics object in file tail (large result)")
            except json.JSONDecodeError as e:
                out["statistics_note"] = f"tail statistics not decodable: {e}"
        else:
            out["statistics_note"] = "no statistics object in the last 64 MiB"
    return out


def run_one(entry, args, cpu, out_dir, binary_sha):
    """One group, executed in its own worker process (see main), pinned to the group's CPU: the
    result parsing after the timed solve runs on that CPU between solve and verify, never on a
    CPU where another group is being timed."""
    os.sched_setaffinity(0, {cpu})
    run_dir = out_dir / "runs" / pathlib.Path(entry["recipe"]["file"]).stem
    if run_dir.exists():
        shutil.rmtree(run_dir)
    (run_dir / "tmp").mkdir(parents=True)
    recipe_path = pathlib.Path(entry.get("_override") or (BENCH / entry["recipe"]["file"]))
    unresolved = entry["status_in_data_json"] == "unresolved"
    timeout = args.timeout_unresolved if unresolved else args.timeout
    text, limits, use_ckpt = prepare_input(recipe_path.read_text(), run_dir, timeout, args.memory_mib)
    (run_dir / "input.toml").write_text(text)
    backstop = (int(limits["memory_mib"]) + 2048) * 1024 * 1024
    env = dict(os.environ, TMPDIR=str(run_dir / "tmp"), OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1",
               RAYON_NUM_THREADS="1")

    def bounded(argv, tag, limit):
        cmd = [GNU_TIME, "-v", "-o", str(run_dir / f"{tag}_time.txt"), "taskset", "-c", str(cpu),
               "prlimit", f"--as={backstop}", "timeout", "-k", "10", f"{limit:g}"] + argv
        t0 = time.perf_counter()
        with open(run_dir / f"{tag}_stdout.txt", "wb") as so, open(run_dir / f"{tag}_stderr.txt", "wb") as se:
            p = subprocess.run(cmd, stdout=so, stderr=se, cwd=str(run_dir), env=env)
        wall = time.perf_counter() - t0
        info = parse_time(run_dir / f"{tag}_time.txt")
        info.update(exit_code=p.returncode, wall_secs=wall, timed_out=p.returncode in (124, 137))
        for name in (f"{tag}_stderr.txt", f"{tag}_stdout.txt"):
            q = run_dir / name
            if q.stat().st_size > 1 << 20:  # keep the tail only
                data = q.read_bytes()[-(1 << 20):]
                q.write_bytes(b"[truncated]\n" + data)
        return info

    rec = dict(group_index=entry["index"], group=entry["identifier"], scope=entry["scope"],
               data_json_status=entry["status_in_data_json"], recipe_file=entry["recipe"]["file"],
               recipe_sha256=sha256(recipe_path), input_sha256=sha256(run_dir / "input.toml"),
               recipe_override=entry.get("_override"),
               engine=entry["recipe"]["engine"], ordering=entry["recipe"]["ordering"],
               recipe_order=entry["recipe"]["order"], anchor=entry["recipe"]["homogeneous_anchor"],
               cpu=cpu, solve_timeout_secs=timeout, memory_mib=limits["memory_mib"],
               cache_policy=("cold: fresh run directory and TMPDIR; fresh per-run checkpoint_dir for the MCTS search"
                             if use_ckpt else
                             "cold: fresh run directory and TMPDIR; no persistent checkpoints (default)"),
               checkpoint_dir_set=use_ckpt, timing="perf_counter in a per-group worker process pinned to the CPU",
               binary=str(args.binary), binary_sha256=binary_sha, started=now(),
               load_before=pathlib.Path("/proc/loadavg").read_text().split()[:3],
               run_dir=str(run_dir.relative_to(out_dir)))
    solve = bounded([str(args.binary), "solve", "input.toml", "--output", "result.json"], "solve", timeout)
    rec["solve"] = solve
    result = run_dir / "result.json"
    summary = summarize_result(result) if result.exists() and result.stat().st_size > 0 else {}
    rec["result"] = summary
    status = summary.get("status")
    if status == "complete_generic" and solve["exit_code"] == 0:
        rec["result_sha256"] = sha256(result)
        verify = bounded([str(args.binary), "verify", "result.json"], "verify", args.verify_timeout)
        rec["verify"] = verify
        try:
            v = json.loads((run_dir / "verify_stdout.txt").read_text())
        except Exception:
            v = {}
        rec["verification"] = v
        if verify["exit_code"] == 0 and v.get("verified") is True:
            rec["status"] = "complete_verified"
            rec["cells"] = v.get("cells")
        elif verify["timed_out"]:
            rec["status"] = "verify_timeout"
        else:
            rec["status"] = "verify_failed"
    elif solve["timed_out"]:
        rec["status"] = "solve_timeout"
    elif status:
        rec["status"] = f"solve_{status}"
    else:
        rec["status"] = "solve_error"
        rec["stderr_tail"] = (run_dir / "solve_stderr.txt").read_bytes()[-2000:].decode("utf-8", "replace")
    rec["order"] = summary.get("order")
    rec["finished"] = now()
    rec["load_after"] = pathlib.Path("/proc/loadavg").read_text().split()[:3]
    keep = args.keep_results
    if result.exists() and (keep == "none" or (keep == "small" and result.stat().st_size > 64 << 20)):
        result.unlink()
        rec["result_deleted"] = True
    ck = run_dir / "checkpoints"
    if ck.exists():
        rec["checkpoint_bytes"] = sum(p.stat().st_size for p in ck.rglob("*") if p.is_file())
        shutil.rmtree(ck)
    shutil.rmtree(run_dir / "tmp", ignore_errors=True)
    return rec


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", required=True, type=pathlib.Path)
    ap.add_argument("--parallel", required=True, type=int)
    ap.add_argument("--out", required=True, type=pathlib.Path)
    ap.add_argument("--cpus", default="366,367,368,369,370,371")
    ap.add_argument("--manifest", default=str(BENCH / "groups.json"))
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--timeout-unresolved", type=float)
    ap.add_argument("--verify-timeout", type=float)
    ap.add_argument("--memory-mib", type=int)
    ap.add_argument("--keep-results", choices=["small", "all", "none"], default="all",
                    help="all (default): keep every result.json so each proof can be re-verified later; "
                         "small: delete results over 64 MiB after hashing them")
    ap.add_argument("--recipe-override", action="append", default=[], metavar="ID_OR_INDEX=TOML",
                    help="run this TOML instead of the group's recipe (e.g. the Gluza33 projective-sections "
                         "recipe: 302=/path/g33.toml); use a separate --out to keep both variants")
    args = ap.parse_args()
    args.binary = args.binary.resolve()
    args.timeout_unresolved = args.timeout_unresolved or args.timeout
    args.verify_timeout = args.verify_timeout or max(600.0, 2 * max(args.timeout, args.timeout_unresolved))
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(pathlib.Path(args.manifest).read_text())
    entries = manifest["entries"]
    if args.only:
        sel = set(args.only)
        entries = [e for e in entries if e["identifier"] in sel or f"{e['index']:03d}" in sel
                   or str(e["index"]) in sel]
    overrides = {}
    for item in args.recipe_override:
        key, path = item.split("=", 1)
        overrides[key] = str(pathlib.Path(path).resolve())
    for e in entries:
        for key in (e["identifier"], f"{e['index']:03d}", str(e["index"])):
            if key in overrides:
                e["_override"] = overrides[key]
                tomllib.loads(pathlib.Path(overrides[key]).read_text())  # must parse
    binary_sha = sha256(args.binary)
    records = out / "records.jsonl"
    done = set()
    if records.exists():
        for line in records.read_text().splitlines():
            if line.strip():
                r = json.loads(line)
                if r.get("binary_sha256") == binary_sha:
                    done.add(r["group"])
    todo = [e for e in entries if e["identifier"] not in done]
    cpus = [int(c) for c in args.cpus.split(",")]
    par_file = out / "parallel.txt"
    if not par_file.exists():
        par_file.write_text(f"{args.parallel}\n")
    with open(out / "launches.jsonl", "a") as f:
        f.write(json.dumps(dict(started=now(), argv=sys.argv, binary_sha256=binary_sha, todo=len(todo),
                                skipped=len(done), cpus=cpus, timeout=args.timeout,
                                timeout_unresolved=args.timeout_unresolved, verify_timeout=args.verify_timeout,
                                memory_override=args.memory_mib, manifest_sha256=sha256(args.manifest),
                                recipe_overrides=overrides,
                                hostname=os.uname().nodename,
                                loadavg=pathlib.Path("/proc/loadavg").read_text().split()[:3])) + "\n")
    cond = threading.Condition()
    free = list(cpus)
    state = dict(running=0)

    def allowed():
        try:
            return max(1, min(len(cpus), int(par_file.read_text().split()[0])))
        except Exception:
            return max(1, min(len(cpus), args.parallel))

    def work(entry, cpu):
        try:
            # A fresh single-use worker process per group: no interpreter (and no GIL) is shared
            # between groups, so one group's result parsing cannot delay another group's clock,
            # and a crash affects only this group.
            with concurrent.futures.ProcessPoolExecutor(
                    max_workers=1, mp_context=multiprocessing.get_context("spawn")) as pool:
                rec = pool.submit(run_one, entry, args, cpu, out, binary_sha).result()
        except Exception as e:
            rec = dict(group_index=entry["index"], group=entry["identifier"], status="runner_error",
                       error=repr(e), binary_sha256=binary_sha, cpu=cpu)
        with cond:
            with open(records, "a") as f:
                f.write(json.dumps(rec) + "\n")
            free.append(cpu)
            state["running"] -= 1
            cond.notify_all()
        print(f"{now()} {rec['status']:18s} cells={rec.get('cells')} solve={rec.get('solve', {}).get('wall_secs', 0):.2f}s "
              f"cpu={cpu} {rec['group']}", flush=True)

    threads = []
    for entry in todo:
        if (out / "STOP").exists():
            break
        with cond:
            while state["running"] >= allowed() or not free:
                cond.wait(timeout=5)
            cpu = free.pop(0)
            state["running"] += 1
        t = threading.Thread(target=work, args=(entry, cpu))
        t.start()
        threads.append(t)
    for t in threads:
        t.join()
    print(f"{now()} finished {len(threads)} groups", flush=True)


if __name__ == "__main__":
    main()
