#!/usr/bin/env python3
"""Invoke the unchanged native merger on a saved Rust raw-cell inventory."""
import argparse
import hashlib
import json
import pathlib
import traceback

from adapter import native_modules


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference-root", default=".reference/gcad")
    parser.add_argument("--proof", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    import sympy as sp
    gcad = native_modules(args.reference_root)
    proof_path = pathlib.Path(args.proof)
    encoded = proof_path.read_bytes()
    proof = json.loads(encoded)
    assert proof["status"] == "complete_generic"
    symbols = tuple(sp.Symbol(name) for name in proof["order"])
    locals_ = dict(zip(proof["order"], symbols))
    # One object per exact dictionary entry preserves all raw-cell sharing.
    polynomials = [sp.Poly(sp.sympify(p.replace("^", "**"), locals=locals_), *symbols)
                   for p in proof["polynomials"]]
    ids = {id(p): i for i, p in enumerate(polynomials)}

    def root(data, axis):
        if data is None:
            return None
        assert data.get("index_domain", "real") == "real", "only ordinary all-real bounds supported"
        return gcad.PolyRoot(polynomials[data["polynomial"]], axis, data["index"],
                             sp.Rational(data["lower"]), sp.Rational(data["upper"]))

    cells = []
    for cell in proof["cells"]:
        assert not cell["signs"], "adapter requires one unsplit sign problem"
        assert [a["variable"] for a in cell["axes"]] == proof["order"]
        cells.append([gcad.AxisBound(symbols[i], sp.Rational(axis["sample"]),
                                    root(axis["lower"], i), root(axis["upper"], i))
                      for i, axis in enumerate(cell["axes"])])

    def key(root):
        return None if root is None else [ids[id(root.poly)], root.idx]

    def rust_key(root):
        return None if root is None else [root["polynomial"], root["index"]]

    output = {
        "schema_version": 1, "scope": "Native merger applied to an identical Rust raw inventory; no proof credit",
        "proof": str(proof_path), "proof_sha256": hashlib.sha256(encoded).hexdigest(),
        "helper_sha256": hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),
        "native_source_sha256": hashlib.sha256(pathlib.Path(gcad.__file__).read_bytes()).hexdigest(),
        "raw_cells": len(cells), "rust_merged_cells": len(proof["merged_cells"]),
    }
    try:
        merged = gcad.merge(cells)
        native_geometry = [[(key(a.cell_lo), key(a.cell_hi)) for a in cell] for cell in merged]
        rust_geometry = [[(rust_key(a["lower"]), rust_key(a["upper"])) for a in cell["axes"]]
                         for cell in proof["merged_cells"]]
        output.update(status="returned", native_merged_cells=len(merged),
                      structural_geometry_equal=(sorted(map(json.dumps, native_geometry)) ==
                                                 sorted(map(json.dumps, rust_geometry))),
                      native_geometry=native_geometry)
    except Exception as error:
        output.update(status="exception", type=type(error).__name__, message=str(error),
                      traceback=traceback.format_exc())
    destination = pathlib.Path(args.output)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps({k: v for k, v in output.items() if k != "native_geometry"}))


if __name__ == "__main__":
    main()
