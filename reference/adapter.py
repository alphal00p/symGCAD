#!/usr/bin/env python3
"""Independent native-GCAD oracle. Not part of symGCAD's algebra backend.

Use only inside `nix develop .#reference`. Fixture extraction records the
reference tests' inputs; it never supplies algebra to the production engine.
"""
import argparse
import ast
import contextlib
import itertools
import json
import pathlib
import sys
import time


def native_modules(root):
    sys.path.insert(0, str(pathlib.Path(root).resolve()))
    import gcad.gcad as gcad
    import gcad_ext
    assert gcad.RSFC is gcad_ext.RSFC, "reference must use its native extension"
    return gcad


def parse_math(text, symbols, sp):
    """Construct SymPy objects from arithmetic AST nodes, never eval input."""
    text = text.replace("^", "**")
    def visit(node):
        if isinstance(node, ast.Name):
            if node.id in ("true", "false"):
                return sp.true if node.id == "true" else sp.false
            return symbols[node.id]
        if isinstance(node, ast.Constant):
            if isinstance(node.value, bool):
                return sp.true if node.value else sp.false
            if isinstance(node.value, (int, float)):
                return sp.Rational(ast.get_source_segment(text, node))
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.UAdd, ast.USub)):
            value = visit(node.operand)
            return value if isinstance(node.op, ast.UAdd) else -value
        if isinstance(node, ast.BinOp):
            left, right = visit(node.left), visit(node.right)
            if isinstance(node.op, ast.Add): return left + right
            if isinstance(node.op, ast.Sub): return left - right
            if isinstance(node.op, ast.Mult): return left * right
            if isinstance(node.op, ast.Div): return left / right
            if isinstance(node.op, ast.Pow): return left ** right
        if isinstance(node, ast.Compare) and len(node.ops) == 1:
            left, right = visit(node.left), visit(node.comparators[0])
            if isinstance(node.ops[0], ast.Lt): return left < right
            if isinstance(node.ops[0], ast.Gt): return left > right
        raise ValueError("unsupported input expression node: " + type(node).__name__)
    return visit(ast.parse(text.strip(), mode="eval").body)


def solve(args):
    import sympy as sp
    gcad = native_modules(args.reference_root)
    preparation_started = time.perf_counter()
    problem = json.loads(args.problem_json)
    symbols = {name: sp.Symbol(name) for name in problem["variables"]}
    symbols.update({name: sp.Symbol(name) for name in problem.get("parameters", {})})
    def expression(text):
        return parse_math(text, symbols, sp)
    substitutions = {symbols[name]: expression(value) for name, value in problem.get("parameters", {}).items()}
    constraints = [expression(s).subs(substitutions, simultaneous=True) for s in problem.get("constraints", [])]
    splits = [expression(s).subs(substitutions, simultaneous=True) for s in problem.get("split", [])]
    variables = [symbols[name] for name in problem["variables"]]
    options = problem.get("solver", {})
    order = [symbols[s] for s in options.get("order", [])] or variables
    groups = [[symbols[s] for s in g] for g in problem.get("variable_groups", [])] or [variables]
    stages = dict(ordering=0.0, projection=0.0, lifting=0.0, merging=0.0)
    preparation_seconds = time.perf_counter() - preparation_started
    started = time.perf_counter()
    # Preserve stdout exclusively for the machine-readable result. Native
    # tracing is emitted on stderr by upstream's C++ logger.
    with contextlib.redirect_stdout(sys.stderr):
        ordering = options.get("ordering", "given")
        if ordering != "given":
            chooser = {"mods": gcad.greedy_mods_order, "t1": gcad.greedy_t1_order, "sotd": gcad.greedy_sotd_order}.get(ordering)
            if chooser is None:
                raise ValueError("upstream has no ordering policy " + ordering)
            t0 = time.perf_counter()
            order = chooser(constraints + [f < 0 for f in splits], groups)
            stages["ordering"] += time.perf_counter() - t0
        outputs = []
        # The baseline deliberately repeats projection for opposite signs,
        # reproducing the reference test driver rather than sharing work.
        for signs in itertools.product((-1, 1), repeat=len(splits)):
            relations = constraints + [sign * f > 0 for sign, f in zip(signs, splits)]
            if any(relation == False for relation in relations):
                outputs.append(dict(signs=list(signs), raw_cells=0, merged_cells=0, projection=[]))
                continue
            t0 = time.perf_counter()
            positives = gcad.relations_to_positives(relations, order)
            projection = gcad.GPROJ(positives, order)
            stages["projection"] += time.perf_counter() - t0
            t0 = time.perf_counter()
            cells = gcad.RSFC(positives, projection, order)
            stages["lifting"] += time.perf_counter() - t0
            raw_count = len(cells)
            if options.get("merge", "none") == "aggressive":
                t0 = time.perf_counter()
                cells = gcad.merge(cells)
                stages["merging"] += time.perf_counter() - t0
            entry = dict(signs=list(signs), raw_cells=raw_count, merged_cells=len(cells), projection=[
                dict(polynomials=len(level), terms=sum(p.length() for p in level),
                     max_degree=max((p.total_degree() for p in level), default=0))
                for level in projection])
            if args.cells:
                def root(value):
                    if value is None:
                        return None
                    return dict(polynomial=str(value.poly.as_expr()), variable_index=value.var_idx,
                                root_index=value.idx, lower=str(value.value_lo), upper=str(value.value_hi))
                entry["cells"] = [[dict(variable=str(axis.var), sample=str(axis.point),
                                        lower=root(axis.cell_lo), upper=root(axis.cell_hi))
                                   for axis in cell] for cell in cells]
            outputs.append(entry)
    print(json.dumps(dict(schema_version=1, status="complete_generic", name=problem.get("name"),
                          order=list(map(str, order)), elapsed_secs=time.perf_counter()-started,
                          preparation_seconds=preparation_seconds, phases=stages, results=outputs)))


def extract(args):
    """Capture every core test input without running expensive GCAD calls.

    The source AST's preparation assignments are executed with SymPy, while
    decomposition and ordering calls are intercepted. Assertions are retained
    as input-producing calls only; test-output inspections are omitted.
    """
    import sympy as sp
    path = pathlib.Path(args.source) / "tests" / "test_gcad.py"
    source = path.read_text()
    tree = ast.parse(source)
    output = []
    current = None
    sequence = 0
    class Order(list):
        def __init__(self, groups, method):
            super().__init__(v for group in groups for v in group)
            self.groups, self.method = groups, method
    def capture(relations, variables, operation="solve"):
        nonlocal sequence
        sequence += 1
        if isinstance(relations, (sp.Expr, sp.logic.boolalg.Boolean)):
            relations = [relations]
        constraints = [str(r.as_expr()) + " > 0" if isinstance(r, sp.Poly) else str(r) for r in relations]
        constraints = [s.replace("**", "^") for s in constraints]
        item = dict(name=f"{current}/{operation}/{sequence}", variables=list(map(str, variables)),
                    constraints=constraints, split=[], parameters={},
                    variable_groups=[[str(v) for v in g] for g in getattr(variables, "groups", [])],
                    solver=dict(ordering=getattr(variables, "method", "given")))
        output.append(dict(source=f"tests/test_gcad.py::{current}", operation=operation, problem=item))
        return []
    def chooser(method):
        def choose(relations, groups):
            order = Order(groups, method)
            capture(relations, order, "order")
            return order
        return choose
    environment = dict(sp=sp, GCAD=capture,
        GPROJ=lambda relations, variables: capture(relations, variables, "project"),
        RSFC=lambda relations, projection, variables: capture([r.as_expr() > 0 if isinstance(r, sp.Poly) else r > 0 for r in relations], variables, "solve"),
        merge=lambda cells: cells,
        greedy_mods_order=chooser("mods"), greedy_t1_order=chooser("t1"), greedy_sotd_order=chooser("sotd"))
    environment.update({s: sp.Symbol(s) for s in ["x", "y", "z", "t", "u"]})
    failures = []
    for function in (n for n in tree.body if isinstance(n, ast.FunctionDef)):
        current = function.name
        sequence = 0
        body = []
        for statement in function.body:
            if isinstance(statement, ast.Assert):
                # Calls embedded in assertions (e.g. len(GCAD(...))==1) are
                # genuine test inputs, unlike accesses to the resulting cells.
                for node in ast.walk(statement.test):
                    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id in ("GCAD", "GPROJ", "RSFC", "greedy_mods_order", "greedy_t1_order", "greedy_sotd_order"):
                        body.append(ast.Expr(value=node))
            elif isinstance(statement, (ast.Assign, ast.Expr)):
                body.append(statement)
            else:
                raise ValueError("unexpected test statement " + type(statement).__name__)
        module = ast.fix_missing_locations(ast.Module(body=body, type_ignores=[]))
        try:
            exec(compile(module, str(path), "exec"), environment.copy())
        except Exception as error:
            failures.append(dict(test=current, error=str(error)))
    result = dict(schema_version=1, source=str(path), entries=output, failures=failures)
    print(json.dumps(result, indent=2))
    if failures:
        raise SystemExit(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    solve_parser = commands.add_parser("solve")
    solve_parser.add_argument("--reference-root", required=True)
    solve_parser.add_argument("--problem-json", required=True)
    solve_parser.add_argument("--cells", action="store_true")
    extract_parser = commands.add_parser("extract-core")
    extract_parser.add_argument("--source", required=True)
    args = parser.parse_args()
    if args.command == "solve": solve(args)
    else: extract(args)


if __name__ == "__main__":
    main()
