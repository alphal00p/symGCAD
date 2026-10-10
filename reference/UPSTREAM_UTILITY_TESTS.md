# Disposition of all eleven upstream utility tests

The six rational-helper and five native-conversion tests in the 177-test
inventory are not additional GCAD problems. They are explicitly reproduced
by `tests/upstream_utilities.rs`, with the following boundary mapping.

| Upstream test | Rust assertion and disposition |
| --- | --- |
| `test_shortest_fraction_zero` | Same endpoints and exact zero result; both endpoint orders |
| `test_shortest_fraction_int` | Same endpoints and exact integer 2; both endpoint orders |
| `test_shortest_fraction_negint` | Same endpoints and exact integer −2; both endpoint orders |
| `test_shortest_fraction_close` | Same endpoints and exact result 5/17; both endpoint orders |
| `test_shortest_fraction_closer` | Upstream defect: its expected 9/332 lies outside the interval; check the valid exact sample 1/37 in both endpoint orders and explicitly reject the legacy expectation |
| `test_shortest_fraction_bigint` | All original 50 exponents 0,3,…,147; exact closed endpoint returned, also checked in reverse/equal intervals |
| `test_identity_int` | Exact Symbolica values 3, −7 and −2^90, decimal round trips and invalid `zzz` rejection |
| `test_identity_fraction` | Exact values 3/4, −9, 2^150/3 and 7/2^160; round trips and exact denominator reconstruction |
| `test_identity_fraction_conversions` | Integer 17 and rational 23/71 accepted by the Rust exact-input boundary |
| `test_identity_univariate_Poly` | Original polynomial parsed and serialized exactly; dense univariate coefficient vector checked independently |
| `test_identity_multivariate_Poly` | Original polynomial round trip plus exact sparse coefficients/exponents checked under both explicit variable maps |

The Python `int`, `fractions.Fraction` and SymPy-object adapters are
backend-specific: symGCAD has no Python/Cython FFI, so those object types
are not part of its public API. The mathematical values and conversion
invariants are covered at the actual Rust/Symbolica boundary. This is an
explicit API substitution, not a claim that Python objects can be passed to
the Rust library.

The audit found one actual contract difference. The upstream shortest-fraction
helper accepts endpoints in either order, and its `closer` test uses them
in reverse. Its expected result is invalid, since
`9*7121 - 193*332 = 13 > 0`, so `9/332` is above both supplied endpoints.
The smaller-denominator rational `1/37` is strictly between them:
`193*37 - 7121 = 20 > 0` and `7109 - 191*37 = 42 > 0`.
`roots::simplest_between` now normalizes endpoint order before the existing
exact continued-fraction algorithm and returns that valid sample. This is
an independently resolved upstream assertion defect, not an unresolved
compatibility failure. It retains closed-interval semantics; the separate
`sector_sample` routine constructs strictly interior samples for open CAD
sectors.

A fresh invocation of the independent native extension reproduced both
behaviors: reversed inputs returned `9/332` outside the interval, and
ordered inputs returned `1/37` inside it. The result is retained in
`artifacts/upstream-utility-audit-01/native-reversed-interval.json`.
Its SHA256 is
`dda6417a6ed091657b56fbc09943bd730c277f0a20c434aa260d5fc478826859`.
All eleven utility tests and all eighteen root tests passed in the focused
bundle recorded by `artifacts/search-checkpoint-focused-tests-01.log`.

Source provenance at upstream revision
`fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`:

- `tests/test_shortest_fraction.py`, SHA256
  `1da0cfbca3d52efb4f1a878a65a6bb691453e27e6ace97fec58e10edf8e22977`.
- `tests/test_gcad_ext.py`, SHA256
  `10ce51388daea228497d8322ec493c19aa3f06f5dc49f221149f0b05aca4e016`.
