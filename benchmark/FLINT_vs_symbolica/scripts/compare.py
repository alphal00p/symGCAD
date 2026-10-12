"""Compare a re-run (collect.py output directory) with the committed campaign
results job by job: exact-output hashes must be identical, states should
agree, and timings are reported as new/campaign ratios.

usage: python3 -I compare.py <campaign per_job.json> <new per_job.json> [--out report.json]
Exit status 1 if any job that completed in both runs produced a different
exact output, or if the new run itself has a cross-library mismatch
(pass --new-summary to check that too).
"""
import argparse, json, math, sys


def geo(xs):
    xs = [x for x in xs if x and x > 0]
    return math.exp(sum(map(math.log, xs)) / len(xs)) if xs else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("campaign")
    ap.add_argument("new")
    ap.add_argument("--new-summary")
    ap.add_argument("--out")
    a = ap.parse_args()
    old = {(j["op"], j["job"]): j for j in json.load(open(a.campaign))}
    new = [j for j in json.load(open(a.new)) if j["state"] != "pending"]
    rows, ratios, differ, state_changes, new_only = [], [], [], [], []
    for j in new:
        o = old.get((j["op"], j["job"]))
        row = {"op": j["op"], "job": j["job"], "new_state": j["state"], "new_seconds": j["seconds"],
               "campaign_state": o["state"] if o else None, "campaign_seconds": o["seconds"] if o else None}
        if o is None or o["state"] == "pending":
            new_only.append(row)
            row["note"] = "not measured in the campaign snapshot"
        elif j["state"] == "ok" and o["state"] == "ok":
            same = all(j[k] == o[k] for k in ("canon_exact", "canon_unit", "canon_unit_set"))
            row["output_identical"] = same
            if not same:
                differ.append(row)
            row["time_ratio_new_over_campaign"] = j["seconds"] / o["seconds"] if o["seconds"] else None
            ratios.append(row["time_ratio_new_over_campaign"])
        elif j["state"] != o["state"]:
            state_changes.append(row)
        rows.append(row)
    mism = []
    if a.new_summary:
        mism = json.load(open(a.new_summary)).get("mismatches", [])
    both = [r for r in rows if "output_identical" in r]
    report = {
        "jobs_run": len(new),
        "jobs_ok_in_both": len(both),
        "outputs_identical": sum(1 for r in both if r["output_identical"]),
        "outputs_different": differ,
        "state_changes": state_changes,
        "new_only": new_only,
        "same_state_not_ok": [r for r in rows if r["new_state"] == r["campaign_state"] and r["new_state"] != "ok"],
        "cross_library_mismatches_in_new_run": mism,
        "time_ratio_new_over_campaign": {"geomean": geo(ratios), "min": min(ratios) if ratios else None,
                                         "max": max(ratios) if ratios else None, "n": len(ratios)},
        "rows": rows,
    }
    if a.out:
        json.dump(report, open(a.out, "w"), indent=1)
    print(json.dumps({k: v for k, v in report.items() if k not in ("rows", "same_state_not_ok", "new_only")}, indent=1))
    print(f"new-only jobs: {len(new_only)}; same non-ok state: {len(report['same_state_not_ok'])}")
    sys.exit(1 if differ or mism else 0)


if __name__ == "__main__":
    main()
