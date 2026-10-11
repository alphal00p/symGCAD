# Sparse AM–GM support screen on captured Gluza33 guards

Research diagnostic, 11 October 2026. **Increasing the proposed sparse
certificate's term cap is not a promising first response to these guards.**
The small factors are already coefficient-positive; every mixed-sign factor
in this snapshot has opposite-sign Newton vertices in its parsed support.
Sparse exponent relations are plentiful, but they do not supply a global
positive-orthant nonvanishing certificate.

This follows the proposal in [TROPICAL_FOLLOWUP_RESEARCH.md](TROPICAL_FOLLOWUP_RESEARCH.md).
There is **no Symbolica certificate, no guard removal, and no measured GCAD
gain** from this screen.

## Source and bounded execution

The input is the [degree-16 witness diagnostic](GLUZA33_WITNESS_BUDGET.md),
`artifacts/research/gluza33-witness-budget-01/report.json`. It records the
18 coefficient/separator guard factorizations of the earlier 6,938-term
Gluza33 source at prefix `[1/2,2]`. The complete map is
`[x2,x1,x7,x4,x6,x3,x5]`. This is a frozen earlier snapshot, not the latest
remaining projection obstruction.

The factorization report contains 38 boundary-factor occurrences and 21
distinct expression strings. Source factorization used Symbolica and checked
signed reconstruction, as documented in the source note. This new screen
does not repeat or independently verify that algebra.

Available frozen helpers included `profile_certificate`,
`profile_monomial_squares`, `profile_quadratic_certificate`, and
`endpoint_identity`; none implements the proposed AM–GM certificate and
independent replay. Therefore this experiment used **support/sign metadata
only**, with no new build and no polynomial coefficient arithmetic.

The Perl metadata screen ran on CPU 15 with a 60-second timeout and a 1 GiB
address-space cap. It finished successfully in **0.59 seconds wall time**
(0.56 user, 0.03 system); its internal parsing/search timer was 0.183 seconds.
The cap is not a measurement of peak RAM. A preceding launch failed before
execution because `/usr/bin/time` was absent; its stderr is preserved. The
successful launch used the shell's `time -p`.

## What was counted

The parser accepts the source's explicit parenthesized rational coefficients
and monomials, checks every variable against the full map, rejects duplicate
support points, and checks the reported term count. It reads coefficient
signs from their strings. For the two active axes it constructs the planar
convex hull with integer orientation tests.

At the proposed **128-term admission cap**, only three factors qualify:
`x1`, `1+x2`, and `x1^2+8*x2*x1+x2^2`. All are coefficient-positive;
**zero mixed-sign factors qualify**. The other 18 have 137–725 terms.

The four smallest mixed-sign factors were then screened outside that proposed
cap, purely to assess the supply of support relations. The search tests
distinct positive support points with equal weights and denominators 2, 3
or 4. A relation means `sum(alpha_i)=n*beta`, where `beta` carries a negative
coefficient. Counts are per distinct negative support point and unordered
positive support set.

| Factor terms | Positive / negative terms | Two-point relations | Three-point relations | Four-point relations |
| ---: | ---: | ---: | ---: | ---: |
| 137 | 70 / 67 | 289 | ≥1,000 | ≥1,000 |
| 181 | 91 / 90 | 508 | ≥1,000 | ≥1,000 |
| 239 | 126 / 113 | 873 | ≥1,000 | ≥1,000 |
| 249 | 122 / 127 | 951 | ≥1,000 | ≥1,000 |

Two-point enumeration completed. Each three/four-point count stops at 1,000;
these are lower bounds, not total counts. Unequal weights were not searched.
Four-point relations in a plane need not be minimal circuits. **None of
these counts checks the coefficient margin, avoids reuse of allocated
positive coefficients, or proves positivity of the residual.**

All 18 mixed-sign factors have at least one positive and one negative vertex
coefficient in the parsed hull inventory. The mathematical implication of
an exposed vertex is precise: if a vector `w` uniquely maximizes
`w·alpha` there, then along `x_i=t^(w_i)`, its signed monomial dominates as
`t` tends to infinity. Opposite-sign exposed vertices therefore imply a
sign change somewhere in the full positive orthant. A connected-domain
argument then forces a zero.

For example, the 137-term factor's support inventory gives leading term
`35674344000*x2^14` on `x1=1`, and `-25*x1^16` on `x2=1`. These are useful
obstructions to trying to prove a global constant sign. They remain
**metadata findings**, not production certificates: there was no independent
Symbolica evaluation, exposing-ray replay, or verification of a real
solver chamber's containment of those rays. In a bounded or coupled chamber
the rays may be outside the domain entirely.

## Consequence for the next experiment

Try **chamber-specific candidates**, not a larger global term cap. Start
with a retained factor and a domain already justified independently of
that factor. If an exact rational box contains the relevant whole chamber
or block fiber, use the existing verified shifts/box representation to
look for sparse positive components or a small residual. Compare the new
AM–GM proposal against the existing Bernstein, square and quadratic
certificate arms on that same domain.

A sample neighborhood is insufficient. The box must cover the domain for
which removal would be claimed; narrower boxes require an explicit covering
decomposition. Exclude the target's sign from premises, preserve all
uncovered regions and equality boundaries, and replay reconstruction and
strict coefficient margins in Symbolica before any acceptance. This screen
provides no reason to omit any of the 18 guards or their interior roots.

## Evidence and reproduction

Artifacts are under `artifacts/research/tropical-circuit-screen-01/`.
`report.json` preserves the complete expression/support inventory,
occurrence mapping, hull vertices, search limits and example relations.

| File | SHA-256 |
| --- | --- |
| Source witness `report.json` | `e469db1bcc6cc3164ec8acc23e0d17f360d7fa99e45a5ac2e1e5f6af3ca0a5a2` |
| `screen.pl` | `cb0e54fe1616abf26d15b2281d83706593f310098598096ba988bc1ae36ea13e` |
| Screen `report.json` | `9b8fef10e76dc0da645fb0b189b4fea38b8cb578338a4fe497a1ed3c1efcec59` |
| `stdout.json` | `4ab06b0986cc96ea5cdaa3497669808a2d165e0329d4142f7c011465b7050c59` |
| `timing.log` | `d6d64a958545f3b2543663fcaa50b79bccdfa23f79bf65b5dbc7d67d3242829b` |

From the repository root, the core command is:

```sh
ulimit -v 1048576
timeout 60s taskset -c 15 perl \
  artifacts/research/tropical-circuit-screen-01/screen.pl \
  artifacts/research/gluza33-witness-budget-01/report.json \
  artifacts/research/tropical-circuit-screen-01/report.json
```

Timing fields make the regenerated report's byte hash run-specific. No
production files, benchmark results, dependency code, or solver policies
were changed.
