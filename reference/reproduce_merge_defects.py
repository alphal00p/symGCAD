#!/usr/bin/env python3
"""Invoke the original merge on exact tiny cells; no replacement algorithm."""
import argparse
import hashlib
import itertools
import json
import pathlib
import traceback

from adapter import native_modules


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference-root", default=".reference/gcad")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    import sympy as sp
    gcad = native_modules(args.reference_root)
    x = sp.Symbol("x")
    polynomial = sp.Poly(x * (x - 1), x)
    roots = [gcad.PolyRoot(polynomial, 0, i, sp.Rational(i), sp.Rational(i)) for i in range(2)]
    cells = [
        [gcad.AxisBound(x, sp.Rational(-1), None, roots[0])],
        [gcad.AxisBound(x, sp.Rational(1, 2), roots[0], roots[1])],
        [gcad.AxisBound(x, sp.Rational(2), roots[1], None)],
    ]

    def invoke(selected):
        try:
            merged = gcad.merge(selected)
            return {"status": "returned", "merged_count": len(merged), "cells": repr(merged)}
        except Exception as error:
            return {"status": "exception", "type": type(error).__name__, "message": str(error), "traceback": traceback.format_exc()}

    permutations = [dict(order=list(order), **invoke([cells[i] for i in order])) for order in itertools.permutations(range(3))]
    first = sp.Poly(x, x)
    second = sp.Poly(x, x)
    assert first == second and first is not second
    shared = gcad.PolyRoot(first, 0, 0, sp.Rational(0), sp.Rational(0))
    distinct = gcad.PolyRoot(second, 0, 0, sp.Rational(0), sp.Rational(0))
    object_identity = {
        "equal_polynomials": first == second,
        "same_object": first is second,
        "shared_boundary_object": invoke([[gcad.AxisBound(x, -1, None, shared)], [gcad.AxisBound(x, 1, shared, None)]]),
        "equal_distinct_polynomial_objects": invoke([[gcad.AxisBound(x, -1, None, shared)], [gcad.AxisBound(x, 1, distinct, None)]]),
    }
    # A four-square T has legitimate two- or three-rectangle greedy covers.
    # Both invocations return normally, isolating queue-order dependence from
    # the stale-index exception above.
    y = sp.Symbol("y")
    xp = sp.Poly(x * (x - 1) * (x - 2) * (x - 3), x)
    yp = sp.Poly(y * (y - 1) * (y - 2), y)
    xr = [gcad.PolyRoot(xp, 0, i, sp.Rational(i), sp.Rational(i)) for i in range(4)]
    yr = [gcad.PolyRoot(yp, 0, i, sp.Rational(i), sp.Rational(i)) for i in range(3)]
    squares = [[gcad.AxisBound(x, sp.Rational(2*a+1, 2), xr[a], xr[a+1]), gcad.AxisBound(y, sp.Rational(2*b+1, 2), yr[b], yr[b+1])] for a, b in [(0,0),(1,0),(2,0),(1,1)]]
    ordering_control = [dict(order=order, **invoke([squares[i] for i in order])) for order in [[0,1,2,3],[0,2,1,3]]]
    source = pathlib.Path(gcad.__file__)
    output = {"schema_version": 1, "source": str(source), "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "helper_sha256": hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(), "shared_polynomial": str(polynomial), "input_cells": repr(cells), "permutations": permutations, "object_identity": object_identity, "ordering_control": ordering_control, "scope": "Original upstream merge only; inputs are exact disjoint open cells. No corrected algorithm was executed."}
    destination = pathlib.Path(args.output)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps({"permutations": len(permutations), "exceptions": sum(p["status"] == "exception" for p in permutations), "object_identity": object_identity, "ordering_control": ordering_control}))


if __name__ == "__main__":
    main()
