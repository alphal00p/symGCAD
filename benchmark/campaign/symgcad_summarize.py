#!/usr/bin/env python3
"""Summarize one or more symgcad_runner output directories into a per-group table.

usage: symgcad_summarize.py --output summary.json RUN_DIR [RUN_DIR ...]
Later directories override earlier ones for the same group (latest record wins).
Each row is compared with the verified measurement recorded in benchmark/data.json
(cells, chosen order, solve/verify seconds) via groups.json.
"""
import argparse
import hashlib
import json
import pathlib

BENCH = pathlib.Path(__file__).resolve().parent


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", required=True)
    ap.add_argument("runs", nargs="+")
    args = ap.parse_args()
    manifest = {g["identifier"]: g for g in json.loads((BENCH / "groups.json").read_text())["entries"]}
    latest = {}
    sources = []
    for d in args.runs:
        path = pathlib.Path(d) / "records.jsonl"
        sources.append(dict(path=str(path), sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
        for line in path.read_text().splitlines():
            if line.strip():
                r = json.loads(line)
                r["_source"] = str(path)
                latest[r["group"]] = r
    rows = []
    for ident, g in sorted(manifest.items(), key=lambda kv: kv[1]["index"]):
        r = latest.get(ident)
        p = g["prior_measurement"] or {}
        if r is None:
            rows.append(dict(group_index=g["index"], group=ident, status="not_run"))
            continue
        s, v = r.get("solve") or {}, r.get("verify") or {}
        res = r.get("result") or {}
        st = res.get("statistics") or {}
        rows.append(dict(
            group_index=g["index"], group=ident, scope=g["scope"], data_json_status=g["status_in_data_json"],
            status=r["status"], engine=r.get("engine"), ordering=r.get("ordering"), anchor=r.get("anchor"),
            cells=r.get("cells"), raw_cells=res.get("raw_cells"), order=r.get("order"),
            solve_wall_secs=s.get("wall_secs"),
            solve_cpu_secs=(s.get("user_cpu_secs") or 0) + (s.get("system_cpu_secs") or 0) if s else None,
            solve_peak_rss_kib=s.get("peak_rss_kib"),
            verify_wall_secs=v.get("wall_secs"), verify_peak_rss_kib=v.get("peak_rss_kib"),
            internal=dict(ordering=st.get("ordering_seconds"), search=st.get("search_seconds"),
                          projection=st.get("projection_seconds"), lifting=st.get("lifting_seconds"),
                          total=st.get("total_seconds")),
            solve_timeout_secs=r.get("solve_timeout_secs"), memory_mib=r.get("memory_mib"),
            cache_policy=r.get("cache_policy"), cpu=r.get("cpu"), binary_sha256=r.get("binary_sha256"),
            load_before=r.get("load_before"), source=r["_source"], run_dir=r.get("run_dir"),
            prior=dict(cells=p.get("cells"), order=p.get("order"), solve_seconds=p.get("solve_seconds"),
                       verify_seconds=p.get("verify_seconds"), build=p.get("build"), method=p.get("method")),
            cells_match_prior=(r.get("cells") == p.get("cells")) if p.get("cells") is not None and r.get("cells") is not None else None,
            order_match_prior=(r.get("order") == p.get("order")) if p.get("order") and r.get("order") else None))
    ran = [x for x in rows if x["status"] != "not_run"]
    summary = dict(groups=len(rows), run=len(ran),
                   complete_verified=sum(x["status"] == "complete_verified" for x in ran),
                   by_status={k: sum(x["status"] == k for x in ran) for k in sorted({x["status"] for x in ran})},
                   cells_mismatch_vs_data_json=[x["group"] for x in ran if x.get("cells_match_prior") is False],
                   order_differs_vs_data_json=[x["group"] for x in ran if x.get("order_match_prior") is False],
                   newly_verified_unresolved=[x["group"] for x in ran if x["status"] == "complete_verified"
                                              and x["data_json_status"] == "unresolved"],
                   previously_verified_not_verified_now=[x["group"] for x in ran if x["status"] != "complete_verified"
                                                         and x["data_json_status"] == "verified"])
    pathlib.Path(args.output).write_text(json.dumps(dict(schema_version=1, sources=sources, summary=summary,
                                                         rows=rows), indent=1))
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
