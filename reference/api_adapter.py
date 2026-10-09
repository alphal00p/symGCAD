#!/usr/bin/env python3
"""Invoke upstream ordering/projection APIs for independent comparisons only."""
import argparse
import contextlib
import json
import sys
import time
from adapter import native_modules, parse_math


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("order", "project"))
    parser.add_argument("--reference-root", required=True)
    parser.add_argument("--problem-file", required=True)
    args = parser.parse_args()
    import sympy as sp
    gcad = native_modules(args.reference_root)
    with open(args.problem_file) as file:
        problem = json.load(file)
    symbols = {name: sp.Symbol(name) for name in problem["variables"]}
    symbols.update({name: sp.Symbol(name) for name in problem.get("parameters", {})})
    expression = lambda text: parse_math(text, symbols, sp)
    substitutions = {symbols[name]: expression(value)
                     for name, value in problem.get("parameters", {}).items()}
    constraints = [expression(text).subs(substitutions, simultaneous=True)
                   for text in problem.get("constraints", [])]
    splits = [expression(text).subs(substitutions, simultaneous=True)
              for text in problem.get("split", [])]
    relations = constraints + [polynomial < 0 for polynomial in splits]
    variables = [symbols[name] for name in problem["variables"]]
    options = problem.get("solver", {})
    order = [symbols[name] for name in options.get("order", [])] or variables
    groups = [[symbols[name] for name in group]
              for group in problem.get("variable_groups", [])] or [variables]
    started = time.perf_counter()
    with contextlib.redirect_stdout(sys.stderr):
        method = options.get("ordering", "given")
        if method != "given":
            order = {"mods": gcad.greedy_mods_order,
                     "t1": gcad.greedy_t1_order,
                     "sotd": gcad.greedy_sotd_order}[method](relations, groups)
        result = dict(schema_version=1, status="complete", order=list(map(str, order)))
        if args.operation == "project":
            positives = gcad.relations_to_positives(relations, order)
            projection = gcad.GPROJ(positives, order)
            result["levels"] = [[str(poly.as_expr()).replace("**", "^")
                                 for poly in level] for level in projection]
    result["algorithm_seconds"] = time.perf_counter() - started
    print(json.dumps(result))


if __name__ == "__main__":
    main()
