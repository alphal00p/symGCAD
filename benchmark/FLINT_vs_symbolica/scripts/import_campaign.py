"""One-off: turn a campaign's ops/manifest.json (absolute paths) into the
committed operands/manifest.json and operands/sources/<id>.op.xz.

Every non-derived operand is committed (xz) unless it is larger than
--max-source-bytes and not listed in --keep; derived operands are never
committed because scripts/materialize.py regenerates them from their parent.

usage: python3 -I import_campaign.py <campaign-manifest.json> <campaign-ops-dir> <operands-out-dir>
           --repo-root DIR --harvest-root DIR [--max-source-bytes N] [--keep id,id] [--campaign-info JSON]
"""
import argparse, hashlib, json, lzma, os

KEEP_FIELDS = ["source", "problem", "description", "tier", "selection", "order_hint", "skip_alternatives",
               "input_hash", "capture_sha256", "capture_elapsed_seconds", "capture_complete", "scaled_to_integer",
               "derived", "parent", "point", "images", "parameter", "derivative_variable"]


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("manifest")
    ap.add_argument("ops_dir")
    ap.add_argument("out_dir")
    ap.add_argument("--repo-root", required=True)
    ap.add_argument("--harvest-root", required=True)
    ap.add_argument("--max-source-bytes", type=int, default=1_000_000)
    ap.add_argument("--keep", default="")
    ap.add_argument("--campaign-info", default="{}")
    a = ap.parse_args()
    keep = set(filter(None, a.keep.split(",")))
    m = json.load(open(a.manifest))
    os.makedirs(os.path.join(a.out_dir, "sources"), exist_ok=True)
    ops = []
    for o in m["ops"]:
        src = os.path.join(a.ops_dir, os.path.basename(o["path"]))
        digest = sha(src)
        assert digest == o["sha256"], o["id"]
        size = os.path.getsize(src)
        e = {"id": o["id"], "operation": o["operation"], "file": o["id"] + ".op", "sha256": digest, "bytes": size,
             "sizes": o["sizes"]}
        e.update({k: o[k] for k in KEEP_FIELDS if k in o})
        cap = o.get("capture")
        if cap:
            if os.path.abspath(cap).startswith(os.path.abspath(a.harvest_root) + os.sep):
                e["harvest_capture"] = os.path.relpath(cap, a.harvest_root)
            else:
                e["capture"] = os.path.relpath(cap, a.repo_root)
        if o.get("derived"):
            e["committed"] = None
            e["regenerate"] = ("derive --kind gcd from " if o["derived"] == "gcd-squarefree" else "derive --kind auto from ") + o["parent"]
        elif size <= a.max_source_bytes or o["id"] in keep:
            rel = os.path.join("sources", o["id"] + ".op.xz")
            with open(src, "rb") as f:
                data = lzma.compress(f.read(), preset=9 | lzma.PRESET_EXTREME)
            with open(os.path.join(a.out_dir, rel), "wb") as g:
                g.write(data)
            e["committed"] = rel
            e["regenerate"] = "decompress"
        else:
            e["committed"] = None
            e["regenerate"] = f"harvest {o['problem']} (harvest/harvest.sh) + scripts/build_manifest.py; omitted for size"
        ops.append(e)
    out = {"schema": 2, "campaign": json.loads(a.campaign_info), "selection": m.get("selection", {}), "ops": ops}
    json.dump(out, open(os.path.join(a.out_dir, "manifest.json"), "w"), indent=1)
    committed = [e for e in ops if e["committed"]]
    print(json.dumps({"ops": len(ops), "committed": len(committed),
                      "committed_xz_bytes": sum(os.path.getsize(os.path.join(a.out_dir, e["committed"])) for e in committed),
                      "derived": sum(1 for e in ops if e.get("derived")),
                      "omitted": [e["id"] for e in ops if not e["committed"] and not e.get("derived")]}))


if __name__ == "__main__":
    main()
