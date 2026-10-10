# Saturated Descartes witnesses: proof and captured A446 cost

This is a research diagnostic of one captured local projection obligation. It
does not solve the original A446 problem or restrict its kinematic domain.
All timings below come from the same **debug** helper, not a release comparison.

## Exact certificate

Let the nonzero coefficients of a polynomial `p(u,z)` have fixed, nonzero signs
on a connected ancestor cell. Suppose their Descartes variation count is `K`,
the constant and leading coefficients are nonzero, and fixed rational numbers
`0 < c1 < ... < c(K-1)` give strictly alternating signs for

```
p(u,0), p(u,c1), ..., p(u,c(K-1)), leading_coefficient_z(p).
```

Each of the `K` disjoint positive intervals separated by these numbers contains
a root, by the intermediate value theorem. Descartes' bound counts positive
roots with multiplicity and gives at most `K`. Hence each interval contains
exactly one root, each root is simple, and there are no other positive roots.
The implicit function theorem supplies continuous local root graphs; the
unique interval labels make them consistent throughout the connected cell.
No assertion about the negative roots or their multiplicities is needed.

The coefficient-sign and evaluation-sign conditions must hold over the entire
ancestor cell, through recursively checked projection obligations. Checking only
the rational sample does not suffice. All root bounds from this rule use
**positive-root ordinals**, since negative root collisions can change real-root
ordinals. Pairwise positive-root separation between different polynomials remains
a separate obligation.

This argument combines Descartes' bound and sign-separated intervals; it is not
a claim of a new Descartes theorem. See the primary discussion in Mehlhorn and
Sagraloff, [A Deterministic Descartes Algorithm for Real Polynomials, §3](https://www.mpi-inf.mpg.de/~mehlhorn/ftp/DeterministicBitstreamDescartes.pdf).

## Finite discovery is essential

Unlimited fresh rational separator discovery can keep refining forever, even
though every individual witness is valid. For example,

```
p(a,z) = (z^2 - 2)^2 - a,     0 < a < 1, z > 0
```

has two positive roots coalescing at `sqrt(2)` as `a` tends to zero. A rational
separator `c` between those roots requires `a > (c^2-2)^2 > 0`. No finite stock of
fixed rational separators covers arbitrarily small positive `a`.

The production experiment therefore retains at most two distinct separator
vectors per interned polynomial for the full builder lifetime. It reuses a valid
stored vector and otherwise falls back to ordinary discriminant obligations when
that stock is exhausted. The stock guides discovery only: the checker validates
the supplied separator vector directly and accepts a stronger full-real proof
without relying on discovery history. Guards have lower main variable, so this
bounded stock prevents this particular source of an infinite projection closure.
Overall resource limits still apply.

The first integration run was stopped after observing this refinement problem.
Its reconstructed execution record is retained at
`artifacts/profile/a446-descartes-witness-01/initial-aborted-test.json`; it is
explicitly not represented as a complete raw log. The bounded-stock version
passed the previously problematic cases.

## Captured A446 operand

- Capture: `artifacts/hard-campaigns/a446-adjacent-release-01/initial-x5-0/capture/active-1531373.json`.
- Capture SHA256: `9cf0933ab7e5d259ba92d1ae463bb3ce88dd4e543e30eb42228d8df3efc38cbe`.
- Operand hash: `a4b175a55f1e4baa98e018e81c58f9e42f28d5626d09d68b572fdc0a6bc6fd36`.
- Variable map: `[s,t,x1,x2,x3,x4,x7,x8]`; main variable `x3`.
- Rational sample: `(s,t,x1,x2) = (13/3,-1505,1,1)`.
- Input: degree 8 in `x3`, 1,232 terms, degree vector `[4,4,8,8,8,0,0,0]`.

The coefficient signs from constant to leading term are
`[+,+,-,-,-,+,-,-,+]`, with four variations. Exact isolation finds eight distinct
real roots, four positive. Continued-fraction sector sampling chooses the
low-height rational separators **`2`, `874`, `6008`**. The endpoint, separator,
and leading-coefficient signs are **`+,-,+,-,+`**.

This same operand was still the active, unfinished discriminant when the prior
release campaign reached its 300-second deadline (`300.421` seconds measured
wall time; `initial-x5-0/record.json` beside the capture). That is the motivation
for this replacement, not a measured standalone discriminant runtime or a
whole-solve speedup ratio: the new timings cover only discovery and factoring of
the alternative guards.

| Work | Measured seconds |
| --- | ---: |
| Exact fiber root isolation | 0.01067 |
| Symbolic separator evaluations | 0.00281 |
| Complete witness discovery | 0.02069 |
| Checked factorization of all 12 guards | 0.10074 |
| Supervised worker wall time, including setup/output | 0.34866 |

The nine coefficient guards have respectively
`305,271,223,185,140,74,25,8,1` terms. Each separator evaluation has 582 terms;
their maximum numerator-plus-denominator coefficient bit sizes are respectively
41, 105, and 127. Each separator guard remains one factor, with checked
factorization times 0.01249, 0.01529, and 0.01024 seconds. These checks reconstruct
the exact original guard including coefficient scaling and multiplicities.

This makes the individual replacement inexpensive on the captured sample.
Guard propagation and any additional ancestor subdivisions have not been
included in that conclusion. A bounded end-to-end campaign is necessary to
measure the resulting GCAD algorithm.

## Reproduction and evidence

The helper is Rust throughout, uses Symbolica for exact algebra, and delegates
process limits to `benchmark::run_bounded`. It checks the operand checksum,
rejects nonsaturation, repeated positive roots, degree drops and vanishing
coefficient samples, and checks each symbolic evaluation against the exact
specialized fiber. Optional checked factorization publishes atomic progress
after every completed guard, retaining evidence if the worker is stopped.

```
nix develop --command cargo build --example profile_descartes_witness
nix develop --command target/debug/examples/profile_descartes_witness \
  artifacts/hard-campaigns/a446-adjacent-release-01/initial-x5-0/capture/active-1531373.json \
  --prefix '13/3,-1505,1,1' --factor-guards --seconds 60 --cpu 7 \
  --output artifacts/profile/a446-descartes-witness-01/cost.json
```

The recorded run used CPU 7, one worker, a 4 GiB memory limit, and a 60-second
deadline. It completed with 13,304 KiB peak RSS, 0.25135 user CPU seconds and
0.07524 system CPU seconds.

- Frozen helper: `artifacts/profile/a446-descartes-witness-01/profile_descartes_witness`.
- Helper SHA256: `a460c51288e299de8d1062e2d3efcb07fafb9ae26d9b1c4bf0f4728a8b6c875d`.
- Helper source SHA256: `c873e266c18480fd029183358137a10bbf52556f8d20677779ac076ff78eb394`.
- Report: `artifacts/profile/a446-descartes-witness-01/cost.json`.
- Report SHA256: `927ccef9ea77d25b0047e5f60add25fa9b42a8b2c8ed1cc38b5328722e9dbe24`.
- Focused test log: `artifacts/profile/a446-descartes-witness-01/tests.log`.
- Test-log SHA256: `46f19f7393c8786ed3b0145f7f2cdeec05655891232553d6ec02c71f59504e7b`.

The coordinated run passed **65 library tests, five witness integration tests,
and five helper tests**. The helper cases include four positive roots, repeated
positive roots, vanished coefficients/degree drops, nonsaturation, and a pair of
positive branches across a collision of negative roots.
