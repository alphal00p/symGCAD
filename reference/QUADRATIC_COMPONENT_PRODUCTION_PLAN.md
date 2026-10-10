# Proposed next bounded certificate: PSD quadratic components

This records the approved design; the implementation and validation are in
[the production note](QUADRATIC_COMPONENT_PRODUCTION.md). The prerequisite weighted-square milestone is
`0409343`; the evidence for this separate rule is the already measured
[quadratic-component research](QUADRATIC_COMPONENT_CERTIFICATE.md).

## Configuration and narrow initial scope

Add an independent `solver.quadratic_component_positivity=false` option,
requiring `positivity=true`. Its corresponding
`CertificateOptions::quadratic_component_candidates` defaults to zero and is
omitted from serialization when zero. A proposed hard cap of 32 covers two
signs and at most 16 safe bound orientations. Retain the existing ordering of
certificate methods and insert this rule after weighted binomial squares.

Use the shared solver certificate-options method for optimized projection,
ordering, and local guards. Do not silently widen monotone premise discovery
or other reduction engines, which currently construct their own default
options. If a whole-chamber experiment requires those call sites, make that
wiring an explicit separately reviewed change with reduction-replay tests.

Preserve all existing disabled fingerprints. Enabled optimized trials receive
their own versioned suffix; this intentionally also separates raw operation
cache keys. Local raw algebra identities remain independent of the option.
Keep the PSD module separate from `domain/weighted_squares.rs`; neither a
topology identifier nor a stored coefficient template selects it.

## Discovery and exact evidence

1. Derive original strict coordinate bounds and perform the existing checked
   positive shifts/reflections. Start with the conservative support assumptions
   of the measured prototype; do not expand to unbounded signed coordinates in
   this first patch.
2. Form a graph whose vertices are polynomial coordinates and whose edges are
   negative homogeneous degree-two cross terms. For each nontrivial connected
   component, extract its **entire** homogeneous quadratic subform, including
   positive cross terms and diagonal terms supported in that component.
   Positive terms joining different components stay in the residual.
3. Build the rational symmetric matrix, halving cross coefficients exactly.
   Apply rational LDL/Schur elimination in stable coordinate order. A negative
   pivot declines. A zero pivot is allowed only when its remaining row/column
   is exactly zero. Every positive pivot yields one positive-weight rational
   linear-form square. No eigenvalue approximation, square root, SDP result,
   or floating-point tolerance supplies proof authority.
4. Subtract those explicit squares with Symbolica. Require the remaining
   polynomial to be nonzero with strictly positive coefficients on certified
   positive coordinates. Otherwise decline and retain all root obligations.
5. Independently replay the full signed shifted identity before accepting the
   proposal. Discovery's graph, matrix and pivot claims are never sufficient.

Proposed evidence is
`QuadraticComponentsPlusPositiveResidual { upper_variables, squares, residual }`,
with each square holding a positive literal rational weight and a full-map
vector of signed literal rational linear coefficients. Store the residual as
typed sparse coefficient/exponent records, reusing the strict residual data
shape from the weighted rule. Before arithmetic, check literal grammar,
lengths, nonzero denominators, vector dimensions and exponents. Allow one
leading minus sign for linear coefficients only. Full expected-polynomial
variable-map equality and canonical spelling checks must precede replay.

The verifier rederives the original domain and orientations, reconstructs each
linear form and square with exact Symbolica operations, checks the strict
residual, and compares the complete signed polynomial. It need not trust or
repeat discovery's LDL search: the explicit square identity is the certificate.
Reject zero linear forms, forged coefficients, zero/negative weights, wrong
maps, wrong domains, zero residuals and any reconstruction mismatch.

Proposed shared bounds are 16 mapped variables, 256 terms, exponent 32,
512-bit combined rational height, 16 squares, 32 sign/orientation trials and
one shared work counter. Charge graph scans, matrix construction, rational
matrix updates, shifts and replay. Check matrix-entry heights during every
update and reconstructed polynomial bounds during replay. The research cap
of 100,000 matrix updates can be an additional absolute ceiling, under the
existing `work_limit`. External worker time/memory limits still apply.

## Why the measured gain is useful and general

On the authenticated `u<4` chamber, write `h=4-u>0`, `X=x1+x5`,
`Y=x3+x7`, and `W=2X+(31/18)Y`. Discovery already found

```text
C0 = (X-Y)^2 + h*X*Y,
A_constant = (X-Y)^2 + h*X*Y + x4^2 + W*x4.
```

The rank-one quadratic component contains four coordinates, which lies beyond
the weighted binomial rule's square support. Its matrix has one positive and
three exact zero pivots. The same algorithm handles unrelated rank-one and
rank-two rational quadratic subforms mixed with positive higher-degree terms;
these synthetic families are already covered by the research helper tests.
This is a broad sparse-polynomial certificate, not a graph-specific formula.

The existing weighted rule supplies the distinct endpoint B certificate.
PSD alone does not certify B, and the new PSD proposal does not establish
strict positivity of a pure square. At `u=4`, C0 has a real interior zero and
must remain uncertified. Its apparent success on A_constant at `u=4` is
already available from the old portfolio and must not be counted as a gain.
The entire `u>4` and unrestricted-u controls remain part of validation.

## Promotion gates and conditional-chamber measurements

First run separate off/weighted-only/PSD-only/both ablations on the three
authenticated premises B, C0 and A_constant, and on unrelated variable
supports. Require typed JSON roundtrip and independent replay. Add negative
and zero pivots, zero rows with nonzero off-diagonals, indefinite positive
diagonals, parameter-bound reflections, forged literals/maps, strict residual
failures and resource exhaustion. A local `P*w-1` case must still export both
signs across its exact interior root.

Next solve the **full original F sign problem**, retaining original constraints,
on a conditional kinematic chamber. Start where a graph-derived channel is
already an input coordinate, so adding its strict threshold bound requires no
unproved coordinate transport. The crossed symbolic ZZ fixture is an authentic
test of this generic situation: its `u=t` mapping permits `t<4` directly,
together with the original `s>4` and all positive Feynman parameters. Run all
four option combinations at identical fixed orders, engine, merge policy,
CPU allocation and limits; record search/preprocessing costs when enabled.

Only a complete result with exhaustive independent verification counts as a
completed conditional chamber. Failed attempts remain incomplete. A premise
certificate or a small endpoint result is not a substitute for this full-F
test. Report the best conditional decomposition's cells, exact root evidence,
time and memory separately from the corpus's original problem status.

The `t>4` complement stays explicitly unresolved until separately solved, and
the `t=4` separator must be represented as an explicit exceptional polynomial
or treated by specialization. Ordinary F=0 boundaries inside either chamber
remain mandatory. Completing both chambers still requires checked separator
coverage/composition before declaring the original problem complete.

For a channel such as `u=5/9-s-t`, either work with an original-coordinate
chamber constraint or first implement an independently verified invertible
triangular change with original-domain and cylindrical-bound transport.
Coefficient certificates must never silently treat that derived invariant as
an independent bounded coordinate. The previous signed U/F identity is useful
evidence, but does not by itself supply cylindrical transport. A generic
graph-channel transformation and chamber compositor remain separate work.

After these gates, larger graph-driven conditional cases may be promoted to
bounded campaigns. Full original-domain success, continued timeout, and
certificate-only gains must remain distinct in reports. No production patch,
new compilation, or additional campaign should start during the current
combined test/release freeze.
