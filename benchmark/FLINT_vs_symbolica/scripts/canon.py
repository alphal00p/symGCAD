"""Benchmark tooling: canonical hashes of an exact output dump.

Dump format: blocks "poly <multiplicity>" followed by "coef e0 e1 ..." lines
(coef an integer or p/q, canonical decimal). Prints JSON with
  exact      sha256 of the exact polynomials (coefficients, scale and sign kept;
             blocks and terms sorted, so output order is irrelevant). Computed on
             the canonical decimal strings, so huge integers are never converted.
  unit       sha256 of the multiset {(primitive integer associate with positive
             lex-leading coefficient, multiplicity)}, constants dropped
             (null when the dump has more than 2,000,000 coefficient digits)
  unit_set   same as unit without multiplicities
"""
import hashlib, json, sys
from fractions import Fraction
from math import gcd

sys.set_int_max_str_digits(0)
UNIT_DIGIT_LIMIT = 2_000_000


def read(path):
    polys, digits = [], 0
    with open(path) as f:
        for line in f:
            if line.startswith("poly "):
                polys.append((int(line.split()[1]), []))
                continue
            fields = line.split()
            if not fields:
                continue
            digits += len(fields[0])
            polys[-1][1].append((tuple(int(x) for x in fields[1:]), fields[0]))
    return polys, digits


def lines(terms):
    return "\n".join(f"{c} " + " ".join(map(str, e)) for e, c in sorted(terms))


def primitive(terms):
    if not terms:
        return terms
    fr = [(e, Fraction(c)) for e, c in terms]
    den = 1
    for _, c in fr:
        den = den * c.denominator // gcd(den, c.denominator)
    ints = [(e, int(c * den)) for e, c in fr]
    g = 0
    for _, c in ints:
        g = gcd(g, c)
    lead = max(ints)[1]
    sign = -1 if lead < 0 else 1
    return [(e, str(sign * c // g)) for e, c in ints]


def main(path):
    polys, digits = read(path)
    h = lambda xs: hashlib.sha256("\n#\n".join(xs).encode()).hexdigest()
    exact = sorted(f"m{m}\n" + lines(t) for m, t in polys)
    out = {"exact": h(exact), "unit": None, "unit_set": None, "polys": len(polys),
           "terms": sum(len(t) for _, t in polys), "digits": digits,
           "zero": all(len(t) == 0 for _, t in polys)}
    nonconst = [(m, t) for m, t in polys if any(any(e) for e, _ in t)]
    out["nonconstant"] = len(nonconst)
    if digits <= UNIT_DIGIT_LIMIT:
        unit = sorted(f"m{m}\n" + lines(primitive(t)) for m, t in nonconst)
        out["unit"] = h(unit)
        out["unit_set"] = h(sorted(set(lines(primitive(t)) for _, t in nonconst)))
    print(json.dumps(out))


if __name__ == "__main__":
    main(sys.argv[1])
