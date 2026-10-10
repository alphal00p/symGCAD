# Exact PSD quadratic-component certificates

The independent opt-in `solver.quadratic_component_positivity=true`, together
with `positivity=true`, adds a bounded certificate to optimized projection,
ordering and local guard pruning. It is disabled by default and is separate
from `weighted_square_positivity`. Neither flag changes the meaning of an
original constraint or permits omission of an uncertified interior zero.
The approved [design](QUADRATIC_COMPONENT_PRODUCTION_PLAN.md) and earlier
[research measurements](QUADRATIC_COMPONENT_CERTIFICATE.md) describe the
motivation; this note concerns the production proof path.

## Rule and replay

After checked strict-bound shifts/reflections, build a coordinate graph from
negative homogeneous quadratic cross terms. For each connected component,
extract its entire quadratic subform, including its positive cross terms and
diagonal terms. Exact rational Schur/LDL elimination produces positive-weight
linear-form squares. A negative pivot declines. A zero pivot is allowed only
when the corresponding remaining row is exactly zero. No floating-point
eigenvalue, irrational square root or numerical SDP result enters the proof.

Subtract the proposed squares using Symbolica and require a **nonzero
strictly positive-coefficient residual** on certified positive coordinates.
Independent replay reconstructs the complete signed shifted original
polynomial as that residual plus the explicit squares. The matrix and its
discovery trace are not trusted. Pure squares and every inconclusive case
retain their root obligations.

Typed JSON evidence is
`quadratic_components_plus_positive_residual`, with bound orientations,
positive rational weights, full-map vectors of signed rational linear
coefficients, and a typed sparse residual. Literal grammar, lengths and
nonzero denominators are checked before Symbolica arithmetic. The checker
binds the expected polynomial's full variable map and canonical spelling,
independently rederives the original domain, replays every square and checks
the exact signed identity. Arbitrary expression parsing is not used for the
new coefficient or residual fields.

Shared bounds within this method are 16 mapped variables, 256 terms,
coordinate exponents at most 32, combined rational height at most 512 bits,
16 squares, 16 bound orientations and 32 sign/orientation attempts. Graph
scans, shifts, matrix operations and replay consume one shared work budget;
matrix updates also have an absolute 100,000-operation ceiling. Intermediate
matrix entries and reconstructed polynomials are checked after updates.
All active coordinates require established positive shifts in this initial
conservative implementation; it does not generalize to unrestricted signed
coordinates or affine multi-variable square centers.

## Scope and compatibility

The kernel is in `src/domain/quadratic_components.rs`; it has no graph names,
fixture identities or coefficient templates. It extends the portfolio after
weighted binomial squares. Optimized input/projection/operation pruning,
ordering and local guards receive the option through the shared solver
certificate-options helper. Monotone and other pre-existing reduction
premise searches that construct defaults remain unchanged. The separate
quadratic-ladder engine explicitly uses the configured portfolio.

Missing historical fields deserialize to disabled and disabled fields are
omitted when serializing defaults. Optimized domain fingerprints preserve
their historical spelling with the option disabled. Enabling it adds
`quadratic-component-positivity-v1`, which conservatively also separates raw
operation-cache keys. Local raw algebra identities remain independent of the
option, while newly used guard evidence is replayed.

For the authenticated chamber `u<4`, write `h=4-u>0`, X=x1+x5 and Y=x3+x7.
The method finds the four-coordinate rank-one square

```text
C0 = (X-Y)^2+h*X*Y,
A_constant = C0+x4^2+(2*X+(31/18)*Y)*x4.
```

Both had remained inconclusive with the previous portfolio, including the
weighted binomial kernel. Conversely, the cubic endpoint B needs weighted
squares and remains inconclusive with PSD components alone. The two options
are complementary. At u=4 the C0 zero seam remains uncertified, and the
unrestricted/u>4 controls do not acquire false positivity.

## Regression evidence

The first combined gate passed **59 tests**: 31 existing domain, five existing
quadratic, eight weighted-square, eight new component tests and seven new
quadratic-ladder tests. This was a single CPU3 build/run under an aggregate
2 GiB supervisor: 96 seconds including compilation, with 1,500,216 KiB
sampled aggregate peak RSS. Full source/test/manifest hashes were unchanged
before and after the gate. These are test-resource measurements, not solver
benchmarks.

The new tests cover authentic C0/A-constant gains and the complementary B
proof; unrelated rational rank-one/rank-two forms and both signs; zero,
negative and indefinite pivots; pure-square interior seams; upper-bound
orientation; unsupported coordinates; full-map substitution; malformed huge
expression literals, zero denominators and forged sparse records; work and
exponent limits; historical defaults/TOML roundtrip; a complete optimized
result; and a local C0*v−1 result preserving both signs across v=1/C0.
Serialized results and certificates receive fresh independent verification.

Independent read-only review checked the exact LDL theorem, full signed
reconstruction, strict residual, resource parsing and final original-map and
domain binding. No remaining review finding was reported.

Logs and source hashes are under
`artifacts/research/quadratic-production-01/focused-tests-rss.*` and
`source-hashes-before.txt`. Generated diagnostics remain outside commits.
An eighth ladder regression subsequently passed with the other seven: the
unused smaller discriminant root crosses zero inside one base cell. Thus the
combined focused gates cover 60 distinct tests. The complete conditional
graph result is recorded in [the chamber evidence](QUADRATIC_LADDER_VALIDATION.md);
it does not complete the original unrestricted symbolic corpus.

## Four-way production comparison

Each entry is the raw cell count followed by solver seconds in parentheses.
All 20 results completed and passed fresh verification. These are single
debug-build diagnostics, not release medians or native-GCAD speed ratios.

| Workload | Options off | Weighted only | Components only | Both |
|---|---:|---:|---:|---:|
| Cubic endpoint B | 20 (.1313) | 1 (.02464) | 20 (.1268) | 1 (.02462) |
| Four-coordinate C0 | 2 (.01556) | 2 (.03099) | 1 (.01132) | 1 (.01376) |
| Quadratic constant coefficient | 8 (.02864) | 8 (.07282) | 1 (.01587) | 1 (.01915) |
| Unrelated rational family | 6 (.01344) | 6 (.01628) | 1 (.007836) | 1 (.008806) |
| Local C0*v−1 sign arrangement | 12 (.01500) | 12 (.02495) | 2 (.003368) | 2 (.005210) |

The local result retains both signs and their common original zero sheet.
The endpoint row also shows why these two opt-ins are separate: the component
method alone does not prove the cubic endpoint. Input, source and executable
hashes were unchanged throughout this sequential CPU3 run. Per-solve limits
were 10 seconds and 2 GiB; measured peak process RSS was 12,344–15,560 KiB.
The summary `artifacts/research/quadratic-production-01/ablation-summary.json`
has SHA256 `dee45fb54b79ccb42f9e7d0177d51a17b86f6d0a802d4529d3464fe4c3ca11ae`.
