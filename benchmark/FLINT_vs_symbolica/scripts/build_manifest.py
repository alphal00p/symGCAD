"""Select real symGCAD operands, convert them to plain operand files and write
<ops-dir>/manifest.json. Deterministic given the same captures (no randomness;
harvest samples are ranked by recorded capture time, then by file name).

usage: python3 -I build_manifest.py --harvest-dir DIR --ops-dir DIR
                                    [--selection harvest/selection.json]
                                    [--captures-root DIR] [--sym-bin PATH] [--cpu N]

  --harvest-dir    output of harvest/harvest.sh: <problem>/capture/op-*.json
                   (every completed operation) and active-*.json (the
                   operation in progress at the deadline)
  --captures-root  directory the "large" capture paths of selection.json are
                   relative to (default: the repository root, whose ignored
                   artifacts/ tree holds them in the original checkout)

Operand file naming: <ops-dir>/<id>.op for every operand, derived ones included.
"""
import argparse, glob, hashlib, json, os, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from symenv import REPO, symbolica_env  # noqa: E402

FOLDER = os.path.dirname(HERE)


def op_sizes(path):
    with open(path) as f:
        assert f.readline().strip() == "symgcad-flint-bench-op 1"
        f.readline()
        names = f.readline().split()[1:]
        var = int(f.readline().split()[1])
        npolys = int(f.readline().split()[1])
        polys = []
        for _ in range(npolys):
            n = int(f.readline().split()[1])
            deg = [0] * len(names)
            tdeg = 0
            bits = 0
            for _ in range(n):
                fields = f.readline().split()
                e = list(map(int, fields[1:]))
                c = fields[0].lstrip("-")
                b = abs(int(c)).bit_length() if len(c) < 4000 else int((len(c) - 1) * 3.321928094887362) + 1
                bits = max(bits, b)
                deg = [max(a, b) for a, b in zip(deg, e)]
                tdeg = max(tdeg, sum(e))
            polys.append((n, deg, tdeg, bits))
    active = [i for i in range(len(names)) if any(p[1][i] > 0 for p in polys)]
    return {
        "nvars": len(names), "active_variables": [names[i] for i in active],
        "terms": [p[0] for p in polys], "degrees": [[p[1][i] for i in active] for p in polys],
        "total_degrees": [p[2] for p in polys], "coefficient_bits": [p[3] for p in polys],
        "elimination_degrees": [p[1][var] for p in polys] if var >= 0 else None,
        "eliminated": names[var] if var >= 0 else None,
    }


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


class Builder:
    def __init__(self, a):
        self.a = a
        self.ops = a.ops_dir
        self.tmp = os.path.join(self.ops, ".derive")
        os.makedirs(self.tmp, exist_ok=True)
        self.manifest = {"schema": 1, "ops": [], "selection": {}}
        self.env = symbolica_env()

    def sym(self, *args):
        r = subprocess.run(["taskset", "-c", self.a.cpu, self.a.sym_bin, *args], capture_output=True, text=True,
                           env=self.env, timeout=1800)
        if r.returncode != 0:
            print("failed:", args, r.stderr[-400:], file=sys.stderr)
            return None
        return json.loads(r.stdout)

    def add(self, opid, meta):
        path = os.path.join(self.ops, opid + ".op")
        meta = dict(meta)
        meta.update({"id": opid, "file": opid + ".op", "sha256": sha(path), "bytes": os.path.getsize(path),
                     "sizes": op_sizes(path)})
        with open(path) as f:
            f.readline()
            meta["operation"] = f.readline().split()[1]
        self.manifest["ops"].append(meta)

    def convert(self, capture, opid):
        return self.sym("convert", capture, "--out", os.path.join(self.ops, opid + ".op"))

    def derive_auto(self, opid, tier, problem, order_hint, describe=False):
        rep = self.sym("derive", os.path.join(self.ops, opid + ".op"), "--kind", "auto", "--out", os.path.join(self.tmp, opid))
        for d in (rep or {}).get("derived", []):
            seg = d["kind"] == "segmentation-encoded"
            did = f"{opid}-{'segenc' if seg else 'interpimg'}"
            os.replace(d["path"], os.path.join(self.ops, did + ".op"))
            dmeta = {"source": "derived", "derived": d["kind"], "parent": opid, "problem": problem,
                     "tier": tier, "order_hint": order_hint}
            if describe:
                dmeta["description"] = ("exact Kronecker-encoded univariate integer operands of production binary segmentation"
                                        if seg else
                                        f"exact integer image at parameter={d.get('point')} (one of {d.get('images')} production interpolation images)")
            dmeta.update({k: v for k, v in d.items() if k not in ("path", "kind")})
            self.add(did, dmeta)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--harvest-dir", required=True)
    ap.add_argument("--ops-dir", required=True)
    ap.add_argument("--selection", default=os.path.join(FOLDER, "harvest", "selection.json"))
    ap.add_argument("--captures-root", default=REPO)
    ap.add_argument("--sym-bin", default=os.environ.get("SYM_BIN", os.path.join(REPO, "target/release/examples/flint_vs_symbolica")))
    ap.add_argument("--cpu", default="0")
    a = ap.parse_args()
    sel = json.load(open(a.selection))
    os.makedirs(a.ops_dir, exist_ok=True)
    b = Builder(a)
    seen = set()

    # 1. Large captured operands (+ derived production-kernel operands).
    large = [(e["id"], os.path.join(a.captures_root, e["capture"]), e["description"]) for e in sel["large"]]
    g = sel["large_glob"]
    for p in sorted(glob.glob(os.path.join(a.captures_root, g["pattern"]))):
        base = os.path.basename(p)[:-5]
        if len(base) == 12 and all(c in "0123456789abcdef" for c in base):
            large.append((g["id_prefix"] + base, p, g["description"]))
    large += [(e["id"], os.path.join(a.captures_root, e["capture"]), e["description"]) for e in sel["large_after_glob"]]
    for opid, cap, desc in large:
        if not os.path.exists(cap):
            print("missing capture", cap, file=sys.stderr)
            continue
        c = json.load(open(cap))
        key = (c["input_hash"], c.get("variable"))
        if key in seen:
            continue
        seen.add(key)
        rep = b.convert(cap, opid)
        if rep is None:
            continue
        meta = {"source": "captured", "problem": opid.split("-")[0], "description": desc, "tier": "large",
                "capture": os.path.relpath(cap, a.captures_root), "capture_sha256": rep["capture_sha256"],
                "input_hash": c["input_hash"], "capture_elapsed_seconds": c.get("elapsed_seconds"),
                "capture_complete": c.get("complete"), "scaled_to_integer": rep["scaled_to_integer"], "order_hint": 2}
        if opid.startswith(g["id_prefix"]) and opid not in sel["a377_full_alternatives"]:
            meta["skip_alternatives"] = True
        b.add(opid, meta)
        b.derive_auto(opid, "large", meta["problem"], 3, describe=True)

    # 2. Systematic harvest samples: per problem and operation, the slowest half
    #    (by recorded capture time) plus an even stride through the rest.
    for prob, spec in sel["harvest"].items():
        desc, quotas = spec["description"], spec["quotas"]
        caps = []
        for p in sorted(glob.glob(os.path.join(a.harvest_dir, prob, "capture", "op-*.json"))):
            c = json.load(open(p))
            if c.get("operands") is None or not c.get("complete"):
                continue
            caps.append((c["operation"], c.get("elapsed_seconds") or 0.0, p, c))
        active = sorted(glob.glob(os.path.join(a.harvest_dir, prob, "capture", "active-*.json")))
        chosen = []
        for operation, n in quotas.items():
            group = sorted([x for x in caps if x[0] == operation], key=lambda x: (-x[1], x[2]))
            uniq, keys = [], set()
            for x in group:
                k = (x[3]["input_hash"], x[3].get("variable"))
                if k in keys or k in seen:
                    continue
                keys.add(k)
                uniq.append(x)
            top = uniq[: (n + 1) // 2]
            rest = uniq[(n + 1) // 2:]
            m = n - len(top)
            stride = [rest[int(i * len(rest) / m)] for i in range(m)] if m > 0 and rest else []
            chosen += [(x, "top") for x in top] + [(x, "stride") for x in stride]
            b.manifest["selection"][f"{prob}/{operation}"] = {"available_unique": len(uniq), "top": len(top), "stride": len(stride)}
        for i, ((operation, el, p, c), how) in enumerate(chosen):
            seen.add((c["input_hash"], c.get("variable")))
            opid = f"{prob}-{operation[:3]}-{i:02d}-{c['input_hash'][:10]}"
            rep = b.convert(p, opid)
            if rep is None:
                continue
            meta = {"source": "harvest", "problem": prob, "description": desc, "tier": "harvest",
                    "selection": how, "harvest_capture": os.path.relpath(p, a.harvest_dir), "capture_sha256": rep["capture_sha256"],
                    "input_hash": c["input_hash"], "capture_elapsed_seconds": el,
                    "scaled_to_integer": rep["scaled_to_integer"], "order_hint": 1}
            b.add(opid, meta)
            if el > 0.5:
                b.derive_auto(opid, "harvest", prob, 1)
        # the operation in progress at the harvest deadline, if any
        for p in active:
            c = json.load(open(p))
            if c.get("operands") is None or c.get("complete") or (c["input_hash"], c.get("variable")) in seen:
                continue
            seen.add((c["input_hash"], c.get("variable")))
            opid = f"{prob}-inprogress-{c['operation'][:3]}-{c['input_hash'][:10]}"
            rep = b.convert(p, opid)
            if rep is None:
                continue
            b.add(opid, {"source": "harvest-inprogress", "problem": prob,
                         "description": desc + "; operation in progress at the harvest deadline",
                         "tier": "large", "harvest_capture": os.path.relpath(p, a.harvest_dir), "capture_sha256": rep["capture_sha256"],
                         "input_hash": c["input_hash"], "scaled_to_integer": rep["scaled_to_integer"], "order_hint": 2,
                         "skip_alternatives": True})
            b.derive_auto(opid, "large", prob, 3)

    # 3. gcd(P, dP/dv) on the largest selected factor operands (square-free step).
    factors = [o for o in b.manifest["ops"] if o["operation"] == "factor" and o["source"] == "harvest"]
    factors.sort(key=lambda o: (-sum(o["sizes"]["terms"]), o["id"]))
    for o in factors[: sel["gcd_from_largest_factors"]]:
        if max(max(d) for d in o["sizes"]["degrees"]) < 2:
            continue
        gid = o["id"] + "-gcd"
        info = b.sym("derive", os.path.join(a.ops_dir, o["id"] + ".op"), "--kind", "gcd", "--out", os.path.join(a.ops_dir, gid + ".op"))
        if info:
            b.add(gid, {"source": "derived", "derived": "gcd-squarefree", "parent": o["id"], "problem": o["problem"],
                        "tier": "harvest", "order_hint": 1, "derivative_variable": info["derivative_variable"],
                        "description": "gcd(P, dP/dv): first square-free step on a harvested factor operand"})
    shutil.rmtree(b.tmp, ignore_errors=True)
    json.dump(b.manifest, open(os.path.join(a.ops_dir, "manifest.json"), "w"), indent=1)
    print(len(b.manifest["ops"]), "ops")


if __name__ == "__main__":
    main()
