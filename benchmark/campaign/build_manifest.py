#!/usr/bin/env python3
"""Build the 331-group benchmark manifest and per-group symGCAD recipe TOMLs.

Read-only on the repository.  Writes only under the bench/ directory given by
--out.  Sources:
  * benchmark/data.json rows (331 groups, verified measurement provenance),
  * the completion ledgers / comparison reports those rows cite (recipe input),
  * unresolved/inputs/*.toml (improved recipe for the 28 unresolved groups),
  * the corpus TOML of each group (original problem, sign expansion for native).
"""
import argparse
import hashlib
import json
import pathlib
import re
import sys
import tomllib

REPO = pathlib.Path("/common/dev/gcad")
RESEARCH = REPO / "artifacts/research/claude-2026-10-12"


def sha256(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def safe(name):
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", name).strip("_")


def ledger_input(ledger, run_index, identifier):
    data = json.loads((REPO / ledger).read_text())
    run = data["runs"][run_index]
    names = set(run.get("aliases", [])) | {run["name"]}
    if identifier not in names:
        raise SystemExit(f"{identifier}: ledger run {run_index} is {run['name']}")
    if run["status"] not in ("complete_verified", "verification_failed", "verification_timeout"):
        raise SystemExit(f"{identifier}: ledger run {run_index} status {run['status']}")
    # verification_failed = verifier resource failure; data.json credits a
    # supplemental verification record for these (ex38, C531, C695).
    return run["input_file"], dict(ledger=ledger, run_index=run_index, run_name=run["name"],
                                    ledger_status=run["status"],
                                    input_sha256=run.get("input_sha256"),
                                    executable_sha256=run.get("executable_sha256"))


def comparison_input(comparison, identifier, aliases):
    path = REPO / comparison
    data = json.loads(path.read_text())
    names = set(aliases) | {identifier}
    # gluza-rayleigh / gluza30-bilinear: trials with input_file
    for case in data.get("cases", []):
        if case.get("name") in names:
            for trial in case["trials"]:
                if trial.get("status") == "complete_verified":
                    return trial["input_file"], dict(comparison=comparison, case=case["name"])
    if "rust_trials" in data:
        for trial in data["rust_trials"]:
            if trial.get("status", "complete_verified") == "complete_verified":
                return trial["input_file"], dict(comparison=comparison)
    # conditional-fixed-control: per-variant ledgers new-<k>/results/ledger.json
    for ledger in sorted(path.parent.glob("new-*/results/ledger.json")):
        led = json.loads(ledger.read_text())
        for run in led["runs"]:
            if (set(run.get("aliases", [])) | {run["name"]}) & names and run["status"] == "complete_verified":
                return run["input_file"], dict(comparison=comparison,
                                               ledger=str(ledger.relative_to(REPO)),
                                               run_name=run["name"])
    raise SystemExit(f"{identifier}: no verified input in {comparison}")


def check_recipe(text):
    """Recipes are copied verbatim (including their [limits], whose memory_mib
    also sizes local-engine retention budgets).  The runner overrides only the
    wall-time limit and injects a fresh per-run checkpoint_dir (cold caches)."""
    if re.search(r'^checkpoint_dir\s*=\s*"', text, re.M):
        raise SystemExit("recipe names a persistent checkpoint directory; refusing")
    return text


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(RESEARCH / "bench"))
    args = ap.parse_args()
    out = pathlib.Path(args.out)
    inputs_dir = out / "symgcad_inputs"
    inputs_dir.mkdir(parents=True, exist_ok=True)
    data_path = REPO / "benchmark/data.json"
    data = json.loads(data_path.read_text())
    rows = data["rows"]
    assert len(rows) == 331, len(rows)
    unresolved_dir = RESEARCH / "unresolved/inputs"
    unresolved = {}
    for p in sorted(unresolved_dir.glob("*.toml")):
        name = tomllib.loads(p.read_text())["name"]
        unresolved[name] = p
    groups = []
    sign_cases = 0
    for index, row in enumerate(sorted(rows, key=lambda r: (r["scope"], r["original_input"]))):
        ident = row["identifier"]
        original = REPO / row["original_input"]
        problem = tomllib.loads(original.read_text())
        splits = problem.get("split", [])
        n_cases = 1 << len(splits)
        sign_cases += n_cases
        meas = row.get("measurement")
        prior = None
        if row["verified"]:
            src = meas["source"]
            if "ledger" in src:
                recipe_path, provenance = ledger_input(src["ledger"], src["run_index"], ident)
                provenance["kind"] = "ledger"
            else:
                recipe_path, provenance = comparison_input(src["comparison"], ident, row.get("aliases", []))
                provenance["kind"] = "comparison"
            prior = {k: meas.get(k) for k in ("build", "cells", "method", "order", "solve_seconds",
                                                  "verify_seconds", "peak_rss_mib", "n", "cache_policy",
                                                  "executable_sha256")}
            status = "verified"
        else:
            hits = [unresolved[a] for a in [ident] + row.get("aliases", []) if a in unresolved]
            if not hits:
                raise SystemExit(f"{ident}: unresolved without improved recipe")
            recipe_path = str(hits[0])
            provenance = dict(kind="unresolved-improved", path=recipe_path)
            status = "unresolved"
        recipe_path = pathlib.Path(recipe_path)
        recipe_text = recipe_path.read_text()
        recipe = tomllib.loads(recipe_text)
        # The recipe must be the same mathematical problem as the original row.
        for key in ("variables", "constraints", "split", "parameters", "variable_groups"):
            if recipe.get(key) != problem.get(key) and key != "variable_groups":
                # completion inputs keep original fields verbatim; flag any change
                provenance.setdefault("field_differences", []).append(key)
        out_name = f"{index:03d}_{safe(ident)}.toml"
        normalized = check_recipe(recipe_text)
        (inputs_dir / out_name).write_text(normalized)
        solver = recipe.get("solver", {})
        groups.append(dict(
            index=index, identifier=ident, scope=row["scope"], aliases=row.get("aliases", []),
            original_input=row["original_input"], original_sha256=sha256(original),
            native_ordering=problem.get("solver", {}).get("ordering", "given"),
            native_order=problem.get("solver", {}).get("order", []),
            splits=len(splits), sign_cases=n_cases, variables=problem["variables"],
            status_in_data_json=status,
            recipe=dict(source=str(recipe_path), source_sha256=sha256(recipe_path),
                        file=f"symgcad_inputs/{out_name}", provenance=provenance,
                        engine=solver.get("engine"), ordering=solver.get("ordering"),
                        order=solver.get("order", []),
                        homogeneous_block=solver.get("homogeneous_block", []),
                        homogeneous_anchor=solver.get("homogeneous_anchor"),
                        merge=solver.get("merge"),
                        limits=recipe.get("limits", {}),
                        options={k: v for k, v in solver.items() if k not in (
                            "engine", "ordering", "order", "homogeneous_block", "homogeneous_anchor", "merge")}),
            prior_measurement=prior,
            notes=(["Gluza33 positive: a projective-sections recipe is expected from another track; "
                    "current file is the unresolved/inputs local-engine recipe"]
                   if ident.startswith("test_gluza_ex33/solve/3") else [])))
    manifest = dict(schema_version=1, generator=str(pathlib.Path(__file__).resolve()),
                    generator_sha256=sha256(__file__), data_json_sha256=sha256(data_path),
                    groups=len(groups), sign_expanded_cases=sign_cases,
                    verified=sum(g["status_in_data_json"] == "verified" for g in groups),
                    unresolved=sum(g["status_in_data_json"] == "unresolved" for g in groups),
                    entries=groups)
    (out / "groups.json").write_text(json.dumps(manifest, indent=1))
    print(json.dumps({k: v for k, v in manifest.items() if k != "entries"}, indent=1))


if __name__ == "__main__":
    main()
