"""Materialize operand files <ops-dir>/<id>.op from the committed sources and
check every file against the sha256 recorded in the manifest.

  * operands committed under operands/sources/<id>.op.xz are decompressed;
  * derived operands (binary-segmentation encodings, interpolation images,
    gcd(P, dP/dv)) are regenerated from their parent with the Symbolica
    harness `derive` subcommand;
  * operands that are neither (six large Gluza33 factor operands and their gcd
    children, omitted for size) are reported as needing a harvest
    (scripts/build_manifest.py over a fresh harvest/harvest.sh run).

usage: python3 -I materialize.py <manifest.json> <sources-dir> <ops-dir>
                                 [--ids id,id,...|@subset.json] [--sym-bin PATH] [--report out.json]
Exit status 1 if a regenerated file does not match its recorded sha256.
"""
import argparse, hashlib, json, lzma, os, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from symenv import REPO, symbolica_env  # noqa: E402


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("manifest")
    ap.add_argument("sources")
    ap.add_argument("ops_dir")
    ap.add_argument("--ids")
    ap.add_argument("--sym-bin", default=os.environ.get("SYM_BIN", os.path.join(REPO, "target/release/examples/flint_vs_symbolica")))
    ap.add_argument("--report")
    a = ap.parse_args()
    manifest = json.load(open(a.manifest))
    by_id = {o["id"]: o for o in manifest["ops"]}
    if a.ids:
        if a.ids.startswith("@"):
            data = json.load(open(a.ids[1:]))
            wanted = list(data["ids"] if isinstance(data, dict) else data)
        else:
            wanted = a.ids.split(",")
    else:
        wanted = [o["id"] for o in manifest["ops"]]
    os.makedirs(a.ops_dir, exist_ok=True)
    tmp = os.path.join(a.ops_dir, ".derive")
    os.makedirs(tmp, exist_ok=True)
    status = {}

    def target(i):
        return os.path.join(a.ops_dir, i + ".op")

    def ensure(i):
        if i in status:
            return status[i]
        o = by_id[i]
        out = target(i)
        if os.path.exists(out) and sha(out) == o["sha256"]:
            status[i] = "present"
            return status[i]
        src = os.path.join(a.sources, i + ".op.xz")
        if os.path.exists(src):
            with lzma.open(src) as f, open(out, "wb") as g:
                shutil.copyfileobj(f, g)
            how = "decompressed"
        elif o.get("derived"):
            parent = o["parent"]
            if ensure(parent) not in ("present", "decompressed", "derived"):
                status[i] = f"needs parent {parent} ({status[parent]})"
                return status[i]
            if o["derived"] == "gcd-squarefree":
                cmd = [a.sym_bin, "derive", target(parent), "--kind", "gcd", "--out", out]
            else:
                cmd = [a.sym_bin, "derive", target(parent), "--kind", "auto", "--out", os.path.join(tmp, parent)]
            r = subprocess.run(cmd, capture_output=True, text=True, env=symbolica_env())
            if r.returncode != 0:
                status[i] = "derive failed: " + r.stderr[-300:]
                return status[i]
            if o["derived"] != "gcd-squarefree":
                ext = ".segenc.op" if o["derived"] == "segmentation-encoded" else ".interpimg.op"
                produced = os.path.join(tmp, parent + ext)
                if not os.path.exists(produced):
                    status[i] = "derive produced no " + ext
                    return status[i]
                os.replace(produced, out)
                for leftover in (".segenc.op", ".interpimg.op"):
                    p = os.path.join(tmp, parent + leftover)
                    if os.path.exists(p):
                        os.remove(p)
            how = "derived"
        else:
            status[i] = "not committed: regenerate with harvest/harvest.sh + scripts/build_manifest.py"
            return status[i]
        got = sha(out)
        if got != o["sha256"]:
            status[i] = f"SHA256 MISMATCH ({how}): {got}"
        else:
            status[i] = how
        return status[i]

    for i in wanted:
        ensure(i)
    shutil.rmtree(tmp, ignore_errors=True)
    counts = {}
    for i, s in status.items():
        key = "needs parent" if s.startswith("needs parent") else s.split(":")[0].split(" (")[0]
        counts[key] = counts.get(key, 0) + 1
    report = {"requested": len(wanted), "counts": counts, "status": status}
    if a.report:
        json.dump(report, open(a.report, "w"), indent=1)
    print(json.dumps({"requested": len(wanted), "counts": counts}))
    bad = {i: s for i, s in status.items() if "MISMATCH" in s or "failed" in s}
    for i, s in bad.items():
        print(i, s, file=sys.stderr)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
