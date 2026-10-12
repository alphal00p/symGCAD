"""Run every (operand, side, kernel) job of a manifest with at most N concurrent
single-threaded, CPU-pinned workers (one per listed CPU). Resumable: a job
whose <results>/<id>.<side>-<kernel>.status exists is skipped.

usage: python3 -I orchestrate.py <manifest.json> <ops-dir> <results-dir> <cpu[,cpu...]>
                                 [--ids id,id,...|@file] [--filter substring] [--dry-run]

Operand files are <ops-dir>/<id>.op (scripts/materialize.py writes them).
The job matrix and caps (jobs_for) are the ones of the reported campaign.
"""
import argparse, json, os, queue, subprocess, sys, threading, time

HERE = os.path.dirname(os.path.abspath(__file__))

# Per-job wall-clock caps (seconds) of the reported campaign.
PRODUCTION_CAP = 600   # Symbolica production kernel and every FLINT kernel
ALTERNATIVE_CAP = 300  # Symbolica integer Ducos when it is not production's own choice
EXPLORATORY_CAP = 120  # Symbolica brown/crt (never used by production); fmpz_poly on segmentation encodings
SKIP_ALTERNATIVES = {"g33-disc4", "g33-final-stalled"}


def univariate(sizes):
    return len(sizes["active_variables"]) <= 1


def jobs_for(op):
    """(side, kernel, cap seconds) jobs of one operand."""
    kind, s = op["operation"], op["sizes"]
    big = op.get("tier") == "large"
    prod_t = PRODUCTION_CAP
    alt_t = ALTERNATIVE_CAP
    jobs = []
    if kind in ("resultant", "discriminant"):
        jobs += [("sym", "production", prod_t), ("flint", "mpoly", prod_t)]
        if univariate(s):
            # fmpz_poly_resultant picks its multimodular method for few, huge
            # coefficients (segmentation encodings), where it is very slow:
            # cap it at 120 s there and also time the Euclidean variant.
            seg = op.get("derived") == "segmentation-encoded"
            jobs.append(("flint", "poly", EXPLORATORY_CAP if seg else prod_t))
            if kind == "resultant":
                jobs.append(("flint", "polyeuclid", prod_t))
        ed = s["elimination_degrees"]
        alt_ok = (min(ed) >= 2) if kind == "resultant" else (ed[0] >= 3)
        # A large operand without interpolation/segmentation children runs
        # integer Ducos as its production kernel: no duplicate Ducos job.
        prod_is_ducos = big and not op.get("has_derived") and (ed[0] > 4 or kind == "resultant")
        if alt_ok and not op.get("derived"):
            if not prod_is_ducos:
                jobs += [("sym", "ducos", alt_t)]
            if not op.get("skip_alternatives"):
                jobs += [("sym", "brown", EXPLORATORY_CAP), ("sym", "crt", EXPLORATORY_CAP)]
        elif op.get("derived"):
            jobs += [("sym", "ducos", alt_t)]
    elif kind == "factor":
        jobs += [("sym", "production", prod_t), ("sym", "native", prod_t), ("flint", "mpoly", prod_t)]
        if univariate(s):
            jobs.append(("flint", "poly", prod_t))
    elif kind == "gcd":
        jobs += [("sym", "native", prod_t), ("flint", "mpoly", prod_t)]
        if univariate(s):
            jobs.append(("flint", "poly", prod_t))
    return jobs


def annotate(manifest):
    parents = {o.get("parent") for o in manifest["ops"]
               if o.get("derived") in ("segmentation-encoded", "interpolation-image")}
    for o in manifest["ops"]:
        o["has_derived"] = o["id"] in parents
        if o["id"] in SKIP_ALTERNATIVES:
            o["skip_alternatives"] = True
    return manifest


def parse_ids(spec):
    if not spec:
        return None
    if spec.startswith("@"):
        with open(spec[1:]) as f:
            data = json.load(f)
        return set(data["ids"] if isinstance(data, dict) else data)
    return set(spec.split(","))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("manifest")
    ap.add_argument("ops_dir")
    ap.add_argument("results")
    ap.add_argument("cpus")
    ap.add_argument("--ids", help="comma-separated ids or @file.json ({'ids': [...]} or a list)")
    ap.add_argument("--filter", default="")
    ap.add_argument("--min-seconds", default="1.0")
    ap.add_argument("--max-reps", default="100")
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()
    manifest = annotate(json.load(open(a.manifest)))
    ids = parse_ids(a.ids)
    cpus = a.cpus.split(",")
    os.makedirs(a.results, exist_ok=True)
    work = queue.Queue()
    ops = sorted(manifest["ops"], key=lambda o: (o.get("order_hint", 0), o["id"]))
    n = 0
    for op in ops:
        if a.filter and a.filter not in op["id"]:
            continue
        if ids is not None and op["id"] not in ids:
            continue
        path = os.path.join(a.ops_dir, op["id"] + ".op")
        if not os.path.exists(path):
            print(f"missing operand file {path}", file=sys.stderr)
            continue
        for side, kernel, secs in jobs_for(op):
            base = os.path.join(a.results, f"{op['id']}.{side}-{kernel}")
            if os.path.exists(base + ".status"):
                continue
            work.put((op, path, side, kernel, secs, base))
            n += 1
    print(f"{n} jobs", flush=True)
    if a.dry_run:
        while not work.empty():
            op, path, side, kernel, secs, base = work.get()
            print(op["id"], side, kernel, secs)
        return
    lock = threading.Lock()

    def worker(cpu):
        while True:
            try:
                op, path, side, kernel, secs, base = work.get_nowait()
            except queue.Empty:
                return
            t = time.time()
            r = subprocess.run(["bash", os.path.join(HERE, "run_one.sh"), path, side, kernel, cpu,
                                str(secs), base, a.min_seconds, a.max_reps], capture_output=True, text=True)
            with lock:
                print(f"{time.strftime('%H:%M:%S')} cpu{cpu} {op['id']} {side}-{kernel} "
                      f"{time.time() - t:.1f}s {r.stdout.strip()}", flush=True)

    threads = [threading.Thread(target=worker, args=(c,)) for c in cpus]
    for th in threads:
        th.start()
    for th in threads:
        th.join()


if __name__ == "__main__":
    main()
