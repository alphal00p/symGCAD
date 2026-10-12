#!/usr/bin/env python3
"""Run the original (native) GCAD of Vitaly Magerya on all sign-expanded cases.

Protocol (mirrors src/benchmark.rs qualification, with a uniform 300 s cap):
  * every group of bench/groups.json, original corpus TOML;
  * each split polynomial expanded into individual sign problems exactly like
    run_campaign(): constraint "(split) > 0" / "(split) < 0", name
    "<name>/signs=<+->", split cleared; mask bit i set = positive;
  * the unmodified upstream package (.reference/gcad) through the unmodified
    bridge reference/adapter.py, with the problem's own ordering policy
    (mods/t1/sotd greedy order chosen by native GCAD, or the given order);
  * one pinned CPU per case, RLIMIT_AS cap, hard wall-clock timeout,
    OPENBLAS/OMP threads = 1, GNU time -v for CPU/RSS.

Parallelism is read from <out>/parallel.txt before every launch (so it can be
raised without restarting) and capped by the CPU list in <out>/cpus.txt.
Resumable: a case with a final record.json is skipped.
"""
import argparse
import datetime
import hashlib
import json
import os
import pathlib
import re
import resource
import subprocess
import sys
import time
import tomllib

REPO = pathlib.Path("/common/dev/gcad")
PYTHON = "/nix/store/7m1r2n97i2bydzz1jjwmwhy93dmzcpid-python3-3.14.7-env/bin/python"
GNU_TIME = "/run/current-system/sw/bin/time"


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="milliseconds")


def safe(name):
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", name).strip("_")


def expand(original_path):
    """Sign expansion identical to src/benchmark.rs run_campaign()."""
    problem = tomllib.loads(pathlib.Path(original_path).read_text())
    splits = problem.get("split", [])
    out = []
    for mask in range(1 << len(splits)):
        p = json.loads(json.dumps(problem))
        signs = []
        for i, split in enumerate(splits):
            positive = mask & (1 << i) != 0
            p["constraints"] = p.get("constraints", []) + [f"({split}) {'>' if positive else '<'} 0"]
            signs.append("+" if positive else "-")
        p["split"] = []
        name = problem.get("name") or "unnamed"
        p["name"] = f"{name}/signs={''.join(signs)}" if signs else name
        p["limits"] = dict(wall_time_secs=300.0, memory_mib=8192, workers=1, max_cells=None)
        out.append(("".join(signs) or "none", p))
    return out


def parse_gnu_time(path):
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
            elif line.startswith("Exit status:"):
                info["exit_status"] = int(line.split(":")[-1])
            elif line.startswith("Command terminated by signal"):
                info["signal"] = int(line.split()[-1])
    except FileNotFoundError:
        pass
    return info


def tail(path, limit=4000):
    try:
        data = pathlib.Path(path).read_bytes()
    except FileNotFoundError:
        return ""
    return data[-limit:].decode("utf-8", "replace")


class Job:
    def __init__(self, group, signs, problem, case_dir):
        self.group, self.signs, self.problem, self.dir = group, signs, problem, case_dir
        self.proc = None
        self.cpu = None
        self.started = None
        self.start_iso = None
        self.load = None


def launch(job, args, cpu):
    job.dir.mkdir(parents=True, exist_ok=True)
    problem_json = json.dumps(job.problem)
    (job.dir / "problem.json").write_text(problem_json)
    memory = args.memory_mib * 1024 * 1024

    def preexec():
        os.setpgid(0, 0)
        resource.setrlimit(resource.RLIMIT_AS, (memory, memory))
        os.sched_setaffinity(0, {cpu})

    cmd = [GNU_TIME, "-v", "-o", str(job.dir / "time.txt"),
           "timeout", "--kill-after=10", str(args.timeout),
           PYTHON, str(REPO / "reference/adapter.py"), "solve",
           "--reference-root", str(REPO / ".reference/gcad"),
           "--problem-json", problem_json]
    env = dict(os.environ, OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1", PYTHONHASHSEED="0")
    (job.dir / "command.json").write_text(json.dumps(dict(
        argv=cmd[:-1] + ["<problem.json>"], cpu=cpu, rlimit_as_mib=args.memory_mib,
        timeout_secs=args.timeout, env=dict(OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1",
                                             PYTHONHASHSEED="0"))))
    job.cpu = cpu
    job.load = pathlib.Path("/proc/loadavg").read_text().split()[:3]
    job.start_iso = now()
    job.started = time.monotonic()
    job.proc = subprocess.Popen(cmd, stdout=open(job.dir / "stdout.json", "wb"),
                                stderr=open(job.dir / "stderr.log", "wb"), env=env,
                                preexec_fn=preexec, cwd=str(job.dir))


def finish(job, args):
    rc = job.proc.returncode
    wall = time.monotonic() - job.started
    gt = parse_gnu_time(job.dir / "time.txt")
    result = None
    try:
        text = (job.dir / "stdout.json").read_text()
        if text.strip():
            result = json.loads(text)
    except (ValueError, UnicodeDecodeError):
        result = None
    inner = gt.get("exit_status", rc)
    if result is not None and result.get("status") == "complete_generic" and inner == 0:
        status = "complete"
    elif inner in (124, 137) or gt.get("signal") == 9 or wall >= args.timeout - 0.5:
        status = "timeout"
    else:
        err = tail(job.dir / "stderr.log")
        status = "memory" if "MemoryError" in err or "std::bad_alloc" in err else "error"
    record = dict(
        schema_version=1, group_index=job.group["index"], identifier=job.group["identifier"],
        scope=job.group["scope"], signs=job.signs, name=job.problem["name"], status=status,
        cpu=job.cpu, start_utc=job.start_iso, end_utc=now(), wall_secs=wall,
        loadavg_at_start=job.load, exit_code=rc, inner_exit_status=inner,
        timeout_secs=args.timeout, rlimit_as_mib=args.memory_mib, **gt)
    if result is not None:
        record.update(
            order=result.get("order"), adapter_elapsed_secs=result.get("elapsed_secs"),
            preparation_secs=result.get("preparation_seconds"), phases=result.get("phases"),
            raw_cells=sum(r["raw_cells"] for r in result.get("results", [])),
            merged_cells=sum(r["merged_cells"] for r in result.get("results", [])),
            projection=[r.get("projection") for r in result.get("results", [])])
    if status in ("error", "memory"):
        record["stderr_tail"] = tail(job.dir / "stderr.log")
    # keep stderr small: native tracing can be verbose
    err_path = job.dir / "stderr.log"
    if err_path.exists() and err_path.stat().st_size > 200000:
        data = err_path.read_bytes()
        err_path.write_bytes(b"[truncated head]\n" + data[-100000:])
    tmp = job.dir / "record.json.tmp"
    tmp.write_text(json.dumps(record, indent=1))
    tmp.rename(job.dir / "record.json")
    return record


def read_int(path, default):
    try:
        return int(pathlib.Path(path).read_text().split()[0])
    except (FileNotFoundError, ValueError, IndexError):
        return default


def read_cpus(path):
    text = pathlib.Path(path).read_text().replace(",", " ").split()
    cpus = []
    for item in text:
        if "-" in item:
            a, b = item.split("-")
            cpus.extend(range(int(a), int(b) + 1))
        else:
            cpus.append(int(item))
    return cpus


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--groups", default=str(pathlib.Path(__file__).with_name("groups.json")))
    ap.add_argument("--out", required=True)
    ap.add_argument("--timeout", type=float, default=300.0)
    ap.add_argument("--memory-mib", type=int, default=8192)
    ap.add_argument("--only", nargs="*", help="restrict to these group identifiers")
    ap.add_argument("--priority", help="JSON {case name: expected seconds} for scheduling")
    args = ap.parse_args()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(pathlib.Path(args.groups).read_text())
    priority = json.loads(pathlib.Path(args.priority).read_text()) if args.priority else {}
    jobs = []
    for group in manifest["entries"]:
        if args.only and group["identifier"] not in args.only:
            continue
        for signs, problem in expand(REPO / group["original_input"]):
            case_dir = out / "cases" / f"{group['index']:03d}_{safe(group['identifier'])}" / safe(
                "s" + signs.replace("+", "p").replace("-", "m"))
            if (case_dir / "record.json").exists():
                continue
            jobs.append(Job(group, signs, problem, case_dir))
    # Known-fast cases first (cheap, validates the pipeline), then the rest
    # longest-expected-first; unknown expectations count as 300 s.
    def key(job):
        expected = priority.get(job.problem["name"], args.timeout)
        return (expected >= 60.0, -expected if expected >= 60.0 else expected, job.group["index"])
    jobs.sort(key=key)
    print(f"{now()} pending={len(jobs)}", flush=True)
    running = []
    while jobs or running:
        for job in list(running):
            if job.proc.poll() is not None:
                running.remove(job)
                rec = finish(job, args)
                print(f"{now()} done {rec['name']} cpu={rec['cpu']} status={rec['status']} "
                      f"wall={rec['wall_secs']:.2f} cells={rec.get('raw_cells')}", flush=True)
        if (out / "STOP").exists():
            if not running:
                break
            time.sleep(1.0)
            continue
        cpus = read_cpus(out / "cpus.txt")
        limit = min(read_int(out / "parallel.txt", 1), len(cpus))
        busy = {job.cpu for job in running}
        while jobs and len(running) < limit:
            free = [c for c in cpus if c not in busy]
            if not free:
                break
            job = jobs.pop(0)
            launch(job, args, free[0])
            busy.add(free[0])
            running.append(job)
        time.sleep(0.2)
    print(f"{now()} finished", flush=True)


if __name__ == "__main__":
    main()
