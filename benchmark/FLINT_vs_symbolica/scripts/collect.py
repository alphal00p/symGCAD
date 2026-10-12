"""Collect campaign results into summary.json, per_op.csv, per_job.csv/json and
the Typst snippet table.typ.

usage: python3 -I collect.py <manifest.json> <results-dir> <out-dir> [--title-note TEXT]

Job states: ok | timeout (killed at its cap) | memory | error | flint-failed |
pending (no .status yet: the job has not run). Ratios are Symbolica time /
FLINT time (minimum over repetitions, one pinned CPU each, identical exact
integer inputs); > 1 means FLINT is faster.
"""
import argparse, csv, json, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from orchestrate import jobs_for, annotate  # noqa: E402

FLINT_KERNELS = ("flint-mpoly", "flint-poly", "flint-polyeuclid")


def load(base, cap):
    if not os.path.exists(base + ".status"):
        return {"state": "pending", "timeout": cap}
    st = json.load(open(base + ".status"))
    rec = {"rc": st["rc"], "wall": st["wall"], "max_rss_mib": (st["max_rss_kib"] or 0) / 1024, "timeout": st["timeout"]}
    if st["rc"] == 0 and os.path.exists(base + ".json"):
        j = json.load(open(base + ".json"))
        rec.update({"state": "ok", "seconds": j["min_seconds"], "median_seconds": j["median_seconds"], "reps": j["reps"],
                    "chosen": j.get("chosen"), "output_terms": j.get("output_terms"),
                    "output_max_bits": j.get("output_max_bits"), "backend": j.get("backend")})
        if "sizes" in j:
            rec["sizes"] = j["sizes"]
        if os.path.exists(base + ".canon.json"):
            try:
                rec["canon"] = json.load(open(base + ".canon.json"))
            except json.JSONDecodeError:
                pass
    elif st["rc"] in (137, 124) and st["wall"] >= st["timeout"] - 2:
        rec["state"] = "timeout"
    else:
        err = open(base + ".stderr").read()[-600:] if os.path.exists(base + ".stderr") else ""
        rec["state"] = "memory" if ("memory" in err.lower() or "alloc" in err.lower()) else "error"
        if st["rc"] == 4:
            rec["state"] = "flint-failed"
        rec["stderr_tail"] = err[-300:]
    return rec


CLASSES = ["<1 ms", "1-100 ms", "0.1-10 s", ">=10 s", "capped", "pending"]


def size_class(t):
    if t < 1e-3:
        return "<1 ms"
    if t < 0.1:
        return "1-100 ms"
    if t < 10:
        return "0.1-10 s"
    return ">=10 s"


def kind(op):
    if op.get("derived") == "segmentation-encoded":
        return "Z[x] resultant (segmentation)"
    if op.get("derived") == "interpolation-image":
        return "Z[x] resultant (interp. image)"
    return op["operation"]


KINDS = ["resultant", "discriminant", "factor", "gcd", "Z[x] resultant (interp. image)", "Z[x] resultant (segmentation)"]


def geo(xs):
    xs = [x for x in xs if x and x > 0]
    return math.exp(sum(map(math.log, xs)) / len(xs)) if xs else None


def collect(manifest, res_dir):
    per_op, per_job, mismatches = [], [], []
    checked = 0
    for op in manifest["ops"]:
        recs = {}
        for side, kernel, secs in jobs_for(op):
            key = f"{side}-{kernel}"
            recs[key] = load(os.path.join(res_dir, f"{op['id']}.{key}"), secs)
        prodkey = "sym-native" if op["operation"] == "gcd" else "sym-production"
        prod = recs.get(prodkey, {})
        fl = {k: recs[k] for k in FLINT_KERNELS if k in recs}
        fl_ok = {k: r for k, r in fl.items() if r.get("state") == "ok"}
        t_prod = prod.get("seconds") if prod.get("state") == "ok" else None
        t_fl = min((r["seconds"] for r in fl_ok.values()), default=None)
        fl_best = next((k[6:] for k in FLINT_KERNELS if k in fl_ok and fl_ok[k]["seconds"] == t_fl), None)
        # exact-output agreement against FLINT (fmpz_mpoly first); without a FLINT
        # output, Symbolica alternatives are checked against Symbolica production.
        ref_key = next((k for k in FLINT_KERNELS if k in fl_ok and fl_ok[k].get("canon")), None)
        if ref_key is None and prod.get("canon"):
            ref_key = prodkey
        agree = {}
        if ref_key:
            ref = recs[ref_key]
            for k, r in recs.items():
                if not r.get("canon") or k == ref_key:
                    continue
                if op["operation"] in ("resultant", "discriminant"):
                    field = "exact"
                elif op["operation"] == "factor":
                    field = "unit_set" if "sym-production" in (k, ref_key) else "unit"
                else:
                    field = "unit"
                if r["canon"][field] is None or ref["canon"][field] is None:
                    agree[k] = "unchecked (output above canon.py digit limit)"
                    continue
                checked += 1
                ok = r["canon"][field] == ref["canon"][field]
                agree[k] = "equal" if ok else ("equal-up-to-unit" if r["canon"]["unit"] and r["canon"]["unit"] == ref["canon"]["unit"] else "MISMATCH")
                if not ok:
                    mismatches.append({"op": op["id"], "kernel": k, "reference": ref_key, "field": field, "result": agree[k]})
        states = [r["state"] for r in recs.values()]
        main_states = [prod.get("state")] + [r["state"] for r in fl.values()]
        if t_prod is not None and t_fl is not None:
            cls = size_class(max(t_prod, t_fl))
        elif "pending" in main_states:
            cls = "pending"
        else:
            cls = "capped"
        row = {"id": op["id"], "kind": kind(op), "problem": op["problem"], "source": op["source"], "tier": op.get("tier"),
               "sizes": op["sizes"], "description": op.get("description"), "capture_elapsed_seconds": op.get("capture_elapsed_seconds"),
               "size_class": cls, "symbolica_production_seconds": t_prod,
               "symbolica_production_kernel": prod.get("chosen"), "symbolica_production_state": prod.get("state"),
               "flint_seconds": t_fl, "flint_kernel": fl_best,
               "flint_states": {k: r.get("state") for k, r in fl.items()},
               "kernels": {k: {kk: v.get(kk) for kk in ("state", "seconds", "median_seconds", "reps", "chosen", "max_rss_mib", "wall", "output_terms", "timeout")}
                           for k, v in recs.items()},
               "agreement": agree, "agreement_reference": ref_key,
               "ratio_symbolica_production_over_flint": (t_prod / t_fl) if (t_prod and t_fl) else None}
        for alt in ("ducos", "brown", "crt", "native"):
            r = recs.get(f"sym-{alt}")
            if r and r.get("state") == "ok" and t_fl:
                row[f"ratio_symbolica_{alt}_over_flint"] = r["seconds"] / t_fl
        ducos = recs.get("sym-ducos")
        if ducos is None and prod.get("chosen") == "integer-ducos":
            ducos = prod  # production kernel is integer Ducos itself (no duplicate job)
            row["kernels"]["sym-ducos"] = dict(row["kernels"][prodkey], alias="sym-production")
        if recs.get("flint-mpoly", {}).get("state") == "ok" and ducos and ducos.get("state") == "ok":
            row["ratio_symbolica_ducos_over_flint_mpoly"] = ducos["seconds"] / recs["flint-mpoly"]["seconds"]
        per_op.append(row)
        for k, r in recs.items():
            c = r.get("canon") or {}
            per_job.append({"op": op["id"], "job": k, "state": r["state"], "cap_seconds": r.get("timeout"),
                            "seconds": r.get("seconds"), "median_seconds": r.get("median_seconds"), "reps": r.get("reps"),
                            "chosen": r.get("chosen"), "wall": r.get("wall"), "max_rss_mib": r.get("max_rss_mib"),
                            "output_terms": r.get("output_terms"), "canon_exact": c.get("exact"),
                            "canon_unit": c.get("unit"), "canon_unit_set": c.get("unit_set")})
    return per_op, per_job, mismatches, checked


def summarize(manifest, per_op, per_job, mismatches, checked):
    groups = []
    for k in KINDS:
        for c in CLASSES:
            rows = [r for r in per_op if r["kind"] == k and r["size_class"] == c]
            if not rows:
                continue
            rp = [r["ratio_symbolica_production_over_flint"] for r in rows if r["ratio_symbolica_production_over_flint"]]
            rd = [r.get("ratio_symbolica_ducos_over_flint_mpoly") for r in rows if r.get("ratio_symbolica_ducos_over_flint_mpoly")]
            groups.append({"kind": k, "size_class": c, "n": len(rows), "n_ratio": len(rp),
                           "geomean_production_over_flint": geo(rp), "min": min(rp) if rp else None, "max": max(rp) if rp else None,
                           "symbolica_faster": sum(1 for x in rp if x < 1),
                           "geomean_ducos_over_flint_mpoly": geo(rd), "n_ducos": len(rd),
                           "symbolica_capped": sum(1 for r in rows if r["symbolica_production_state"] not in ("ok", "pending")),
                           "flint_capped": sum(1 for r in rows if r["flint_states"] and all(s not in ("ok", "pending") for s in r["flint_states"].values())),
                           "pending": sum(1 for r in rows if r["size_class"] == "pending")})
    by_kind = []
    for k in KINDS:
        rows = [r for r in per_op if r["kind"] == k]
        both = [r for r in rows if r["ratio_symbolica_production_over_flint"]]
        if rows:
            by_kind.append({"kind": k, "n": len(rows), "n_ratio": len(both),
                            "n_pending": sum(1 for r in rows if r["size_class"] == "pending"),
                            "n_capped": sum(1 for r in rows if r["size_class"] == "capped"),
                            "geomean_production_over_flint": geo([r["ratio_symbolica_production_over_flint"] for r in both]),
                            "time_weighted_production_over_flint": (sum(r["symbolica_production_seconds"] for r in both) /
                                                                     sum(r["flint_seconds"] for r in both)) if both else None,
                            "symbolica_seconds_total": sum(r["symbolica_production_seconds"] for r in both),
                            "flint_seconds_total": sum(r["flint_seconds"] for r in both)})
    ranked = sorted([r for r in per_op if r["ratio_symbolica_production_over_flint"]],
                    key=lambda r: r["ratio_symbolica_production_over_flint"])
    states, caps = {}, {}
    for j in per_job:
        states[j["state"]] = states.get(j["state"], 0) + 1
        if j["state"] == "timeout":
            caps[str(j["cap_seconds"])] = caps.get(str(j["cap_seconds"]), 0) + 1
    return {"schema": 2, "n_ops": len(per_op), "jobs": len(per_job), "job_states": states, "timeouts_by_cap": caps,
            "ops_pending": sum(1 for r in per_op if r["size_class"] == "pending"),
            "outputs_compared": checked, "mismatches": mismatches,
            "ratio_definition": "Symbolica time / FLINT time (min over repetitions, one pinned CPU each); >1 means FLINT faster",
            "by_kind": by_kind, "groups": groups,
            "best_for_symbolica": [{"id": r["id"], "ratio": r["ratio_symbolica_production_over_flint"]} for r in ranked[:8]],
            "best_for_flint": [{"id": r["id"], "ratio": r["ratio_symbolica_production_over_flint"]} for r in ranked[::-1][:8]],
            "not_ok": [{"id": r["id"], "kernels": {k: v["state"] for k, v in r["kernels"].items() if v["state"] not in ("ok", "pending")}}
                       for r in per_op if any(v["state"] not in ("ok", "pending") for v in r["kernels"].values())],
            "per_op": per_op}


def write_csvs(out_dir, per_op, per_job):
    cols = ["id", "kind", "problem", "tier", "source", "active_variables", "degrees", "terms", "coefficient_bits",
            "size_class", "symbolica_production_kernel", "symbolica_production_state", "symbolica_production_seconds",
            "flint_kernel", "flint_seconds", "ratio_symbolica_production_over_flint",
            "symbolica_ducos_seconds", "flint_mpoly_seconds", "ratio_symbolica_ducos_over_flint_mpoly", "agreement"]
    with open(os.path.join(out_dir, "per_op.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(cols)
        for r in per_op:
            s = r["sizes"]
            k = r["kernels"]
            w.writerow([r["id"], r["kind"], r["problem"], r["tier"], r["source"], " ".join(s["active_variables"]),
                        ";".join(",".join(map(str, d)) for d in s["degrees"]), "+".join(map(str, s["terms"])),
                        "+".join(map(str, s["coefficient_bits"])), r["size_class"], r["symbolica_production_kernel"] or "",
                        r["symbolica_production_state"], r["symbolica_production_seconds"] or "", r["flint_kernel"] or "",
                        r["flint_seconds"] or "", r["ratio_symbolica_production_over_flint"] or "",
                        (k.get("sym-ducos") or {}).get("seconds") or "", (k.get("flint-mpoly") or {}).get("seconds") or "",
                        r.get("ratio_symbolica_ducos_over_flint_mpoly") or "",
                        " ".join(f"{a}={b}" for a, b in sorted(r["agreement"].items()))])
    jcols = list(per_job[0].keys()) if per_job else []
    with open(os.path.join(out_dir, "per_job.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=jcols)
        w.writeheader()
        for j in per_job:
            w.writerow({k: ("" if v is None else v) for k, v in j.items()})
    json.dump(per_job, open(os.path.join(out_dir, "per_job.json"), "w"), indent=0)


# ---------------------------------------------------------------- Typst output
def esc(s):
    s = str(s)
    for ch in "\\*_#$@<>[]`~":
        s = s.replace(ch, "\\" + ch)
    return s


def fmt_t(t):
    if t is None:
        return "–"
    if t < 1e-3:
        return f"{t * 1e6:.0f} µs"
    if t < 1:
        return f"{t * 1e3:.1f} ms"
    return f"{t:.1f} s" if t < 100 else f"{t:.0f} s"


def fmt_r(r):
    if r is None:
        return "–"
    return f"{r:.2f}" if r < 10 else f"{r:.1f}" if r < 100 else f"{r:.0f}"


def state_t(k):
    if not k:
        return "–"
    if k.get("state") == "ok":
        return fmt_t(k.get("seconds"))
    if k.get("state") == "timeout":
        return f"> {int(k.get('timeout') or 600)} s"
    return {"memory": "memory", "error": "error", "flint-failed": "failed", "pending": "not run"}.get(k.get("state"), k.get("state") or "–")


def typst(summary, path, note):
    L = ["// benchmark/FLINT_vs_symbolica/results/table.typ -- generated by scripts/collect.py from summary.json.",
         "// Self-contained content (no imports, no page/text settings): #include it from benchmark.typ.",
         "// Ratios: Symbolica time / FLINT time (minimum over repetitions, one pinned CPU each, identical exact",
         "// integer inputs); > 1 means FLINT is faster.", "",
         "// Long tables may span pages (rule scoped to this included file).",
         "#show figure: set block(breakable: true)", ""]
    st = summary["job_states"]
    done = summary["jobs"] - st.get("pending", 0)
    caps = summary.get("timeouts_by_cap", {})
    capped = ", ".join(f"{n} at {c} s" for c, n in sorted(caps.items(), key=lambda x: -int(x[0])))
    L.append(f"#text(size: 8pt)[FLINT 3.6.0 vs Symbolica (symGCAD production kernels): {summary['n_ops']} operands, "
             f"{done} of {summary['jobs']} timing jobs run{esc(note)}; {summary['outputs_compared']} exact outputs compared, "
             f"{len(summary['mismatches'])} mismatches. "
             f"Jobs stopped at their cap: {st.get('timeout', 0)}{' (' + capped + '; the 120 s cap applies only to exploratory kernels)' if capped else ''}; "
             f"not yet run: {st.get('pending', 0)}.]")
    L.append("")
    L.append("#figure(table(")
    L.append("  columns: 7, align: (left, right, right, right, right, right, right), stroke: 0.4pt,")
    L.append("  table.header([*Operation*], [*n*], [*both timed*], [*capped*], [*not run*], [*geo. mean ratio*], [*total-time ratio*]),")
    for k in summary["by_kind"]:
        L.append(f"  [{esc(k['kind'])}], [{k['n']}], [{k['n_ratio']}], [{k['n_capped']}], [{k['n_pending']}], "
                 f"[{fmt_r(k['geomean_production_over_flint'])}], [{fmt_r(k['time_weighted_production_over_flint'])}],")
    L.append("), caption: [Symbolica (symGCAD production path) / FLINT 3.6.0 time ratio by operation type. "
             "Total-time ratio = sum of Symbolica times / sum of FLINT times over operands timed on both sides.])")
    L.append("")
    L.append("#figure(table(")
    L.append("  columns: 7, align: (left, left, right, right, right, right, right), stroke: 0.4pt,")
    L.append("  table.header([*Operation*], [*Size class*], [*n*], [*Production / FLINT* (geo. mean)], [*Range*], [*Symbolica faster*], [*Ducos / FLINT mpoly*]),")
    for g in summary["groups"]:
        if g["size_class"] == "pending":
            continue
        rng = f"{fmt_r(g['min'])}–{fmt_r(g['max'])}" if g["min"] is not None else "–"
        L.append(f"  [{esc(g['kind'])}], [{esc(g['size_class'])}], [{g['n']}], [{fmt_r(g['geomean_production_over_flint'])}], [{rng}], "
                 f"[{g['symbolica_faster']}/{g['n_ratio']}], [{fmt_r(g['geomean_ducos_over_flint_mpoly'])}{'' if not g['n_ducos'] else ' (' + str(g['n_ducos']) + ')'}],")
    L.append("), caption: [Time ratios by operation type and size class (class of the slower side; capped = a side "
             "hit its cap). Last column: Symbolica integer Ducos (production's deterministic fallback) / fmpz_mpoly.])")
    L.append("")
    big = [r for r in summary["per_op"] if r["size_class"] != "pending" and (
        (r["symbolica_production_seconds"] or 0) >= 1 or (r["flint_seconds"] or 0) >= 1 or r["size_class"] == "capped")]
    big.sort(key=lambda r: (KINDS.index(r["kind"]), r["id"]))
    L.append("#figure(table(")
    L.append("  columns: 8, align: (left, left, right, left, right, right, right, right), stroke: 0.4pt,")
    L.append("  table.header([*Operand*], [*Active vars; degrees*], [*Terms*], [*Production kernel*], [*Symbolica production*], [*Best FLINT*], [*Ratio*], [*Symbolica Ducos*]),")
    for r in big:
        s = r["sizes"]
        degs = "; ".join("(" + ",".join(map(str, d)) + ")" for d in s["degrees"])
        dk = r["kernels"].get("sym-ducos")
        fk = r["flint_kernel"]
        fl = r["kernels"].get("flint-" + fk) if fk else r["kernels"].get("flint-mpoly")
        L.append(f"  [{esc(r['id'])}], [{esc(','.join(s['active_variables']))}; {esc(degs)}], [{esc('+'.join(map(str, s['terms'])))}], "
                 f"[{esc(r['symbolica_production_kernel'] or '–')}], [{state_t(r['kernels'].get('sym-production') or r['kernels'].get('sym-native'))}], "
                 f"[{state_t(fl)}{'' if fk in (None, 'mpoly') else ' (' + fk + ')'}], "
                 f"[{fmt_r(r['ratio_symbolica_production_over_flint'])}], [{state_t(dk) if dk and not dk.get('alias') else ('= production' if dk else '–')}],")
    L.append("), caption: [Operands where either side needs at least 1 s or hits its cap "
             "(caps: 600 s production and FLINT, 300 s Symbolica Ducos, 120 s exploratory kernels).])")
    open(path, "w").write("\n".join(L) + "\n")


def markdown(summary, path):
    st = summary["job_states"]
    L = [f"Generated by scripts/collect.py. {summary['n_ops']} operands, {summary['jobs']} jobs: " +
         ", ".join(f"{v} {k}" for k, v in sorted(st.items())) +
         f". Exact outputs compared: {summary['outputs_compared']}, mismatches: {len(summary['mismatches'])}.", "",
         "Ratio = Symbolica (symGCAD production kernel) time / best FLINT time; < 1 means Symbolica is faster.", "",
         "| Operation | operands | timed on both sides | geo. mean ratio | total-time ratio | Symbolica total | FLINT total |",
         "|---|---:|---:|---:|---:|---:|---:|"]
    for k in summary["by_kind"]:
        L.append(f"| {k['kind']} | {k['n']} | {k['n_ratio']} | {fmt_r(k['geomean_production_over_flint'])} | "
                 f"{fmt_r(k['time_weighted_production_over_flint'])} | {fmt_t(k['symbolica_seconds_total']) if k['n_ratio'] else '–'} | "
                 f"{fmt_t(k['flint_seconds_total']) if k['n_ratio'] else '–'} |")
    L += ["", "| Operation | size class | n | geo. mean ratio | range | Symbolica faster | Ducos / fmpz_mpoly |",
          "|---|---|---:|---:|---:|---:|---:|"]
    for g in summary["groups"]:
        rng = f"{fmt_r(g['min'])}–{fmt_r(g['max'])}" if g["min"] is not None else "–"
        L.append(f"| {g['kind']} | {g['size_class']} | {g['n']} | {fmt_r(g['geomean_production_over_flint'])} | {rng} | "
                 f"{g['symbolica_faster']}/{g['n_ratio']} | {fmt_r(g['geomean_ducos_over_flint_mpoly'])} |")
    open(path, "w").write("\n".join(L) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("manifest")
    ap.add_argument("results")
    ap.add_argument("out_dir")
    ap.add_argument("--note", default="", help="appended to the Typst status line, e.g. ' (snapshot 12:00 UTC)'")
    ap.add_argument("--no-typst", action="store_true")
    a = ap.parse_args()
    manifest = annotate(json.load(open(a.manifest)))
    os.makedirs(a.out_dir, exist_ok=True)
    per_op, per_job, mismatches, checked = collect(manifest, a.results)
    summary = summarize(manifest, per_op, per_job, mismatches, checked)
    json.dump(summary, open(os.path.join(a.out_dir, "summary.json"), "w"), indent=1)
    write_csvs(a.out_dir, per_op, per_job)
    markdown(summary, os.path.join(a.out_dir, "summary.md"))
    if not a.no_typst:
        typst(summary, os.path.join(a.out_dir, "table.typ"), a.note)
    print(json.dumps({k: summary[k] for k in ("n_ops", "jobs", "job_states", "outputs_compared", "mismatches", "by_kind")}, indent=1))


if __name__ == "__main__":
    main()
