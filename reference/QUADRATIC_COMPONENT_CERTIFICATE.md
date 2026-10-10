# Exact quadratic components with a strict residual

Research-only follow-up to the [weighted-square diagnostic](CFF_ENDPOINT_COMPOSITION_DIAGNOSTIC.md).
The generic helper `examples/profile_quadratic_components.rs` proves two
authentic symbolic operands that the current portfolio and the bounded
weighted-binomial prototype both decline. This does not complete an original
ZZ problem. It is a separate candidate certificate, with no production change.

## Discovery and proof

After the same exact bound-derived shifts used by the weighted-square helper,
form a graph on variables occurring in negative homogeneous quadratic cross
terms. For each nontrivial connected component, extract the entire homogeneous
quadratic subform supported on that component, including its positive cross
terms and diagonals. Terms of other degrees, and quadratic terms involving
variables outside the component, remain in the residual.

Use an exact rational Schur-complement/LDL pass on each component matrix.
A positive pivot produces a square of a rational linear form with positive
rational weight. A zero pivot is allowed only when the entire remaining pivot
row/column is exactly zero; otherwise the matrix cannot be positive
semidefinite and discovery declines. Negative pivots also decline. No
floating-point eigenvalues, square roots, or SDP proposal enters this step.

The certificate stores only explicit positive-weight linear-form squares and
the residual, not a claim that discovery's matrix was PSD. Independent replay
rederives the coordinate shifts from original strict constraints, validates
all coefficient/map/height bounds, requires a strictly positive residual,
and reconstructs the complete signed polynomial with Symbolica. It also
checks a serialized/deserialized copy. A failed search or zero residual cannot
remove a polynomial or any of its interior zero sets.

This differs from the existing rational quadratic LDL rule, which requires
strict definiteness of the entire polynomial matrix. Here a semidefinite
subform is allowed because a separately strict residual proves nonvanishing.
The variable-component heuristic avoids folding unrelated positive terms
into an indefinite quadratic matrix. It is based only on algebraic support;
there is no graph/topology identifier or coefficient template dispatch.

## Authentic chamber and seam controls

Use the original A446/Ax12 coordinate binding and signed reconstruction from
the companion diagnostic. Let `h=4-u>0`, `X=x1+x5`, `Y=x3+x7`, and
`W=2*X+(31/18)*Y`. The new discoveries reconstruct

```text
C0 = (X-Y)² + h*X*Y,
A_constant = (X-Y)² + h*X*Y + x4² + W*x4.
```

The component is `{x1,x3,x5,x7}`; its matrix has one positive pivot and three
exact zero pivots. The second operand's variable `x4` has no negative
quadratic edge, so its positive contribution remains in the strict residual.
The squared four-term form is found from expanded coefficients, without
providing `X`, `Y`, or this identity to discovery.

| Input domain | Operand | Existing portfolio | Component prototype |
|---|---|---|---|
| `u<4` | C0 | Inconclusive | New verified positive certificate |
| `u<4` | A_constant | Inconclusive | New verified positive certificate |
| `u<4` | Cubic endpoint B | Inconclusive | Inconclusive; needs weighted-square rule |
| exact `u=4` | C0 | Inconclusive | Inconclusive: residual zero |
| exact `u=4` | A_constant | Already positive | Verified positive; consistency check only |
| `u>4` | All three | Inconclusive | Inconclusive |
| unrestricted u | All three | Inconclusive | Inconclusive |

At `u=4`, C0 vanishes at positive points with `X=Y`, for example
`x1=x3=x5=x7=1`. The rejection therefore preserves an actual interior seam.
The entire `u>4` complement remains unresolved. Combining this rule with the
weighted-binomial B certificate would provide three useful strict premises
on `u<4`, but does not itself produce a cylindrical decomposition or certify
transport back to the original variables.

## Bounded experiment

All seven tests passed: the four shared weighted-square tests plus separate
rank-one/rank-two larger-support families, negative orientation, zero-seam
rejection, an indefinite matrix with a zero first pivot, an indefinite
positive-diagonal matrix, forged linear coefficients, unrestricted residual
coordinates, and work exhaustion. Input and frozen binaries were hashed
before and checked unchanged after the runs. No production source changed.

| Diagnostic on `u<4` | Discovery including first replay | Separate serialization/replay |
|---|---:|---:|
| A_constant | 0.000781 s | 0.000611 s |
| C0 | 0.000548 s | 0.000344 s |

Each success used six Schur updates. These are single development-profile
diagnostics, outside benchmark timings; the whole three-operand `u<4` process
took 0.02 seconds and 3,072 KiB RSS. Each of the four domain runs was pinned to
CPU 3 under 2 GiB and 120 seconds. The helper limits are 16 variables,
256 terms, exponent 32, combined rational height 512 bits, 16 squares, and
100,000 Schur updates shared across both sign orientations. Reconstructed
polynomials and every matrix update must also respect their bounds.

Artifacts are under `artifacts/research/quadratic-components-01/`:

| File | SHA-256 |
|---|---|
| `u-below-4.json` (same authenticated operands as weighted experiment) | `e5bfed73cac360a50886a06980e4bd2b48a8dd3390c32ae66efa9ff5ae518609` |
| `below.json` | `a035d7e7c36276686e666c6efdc37af17562c424ee9b6d49e72581172d58e42e` |
| `equal.json` | `87264286af561f32e36a2982f5911addf7369533181c5cad1ab18c9ed6b8dbac` |
| `above.json` | `a43e4256553e3ce34c87043bf2a8e6754f553cbeedc55890d560af5a4dfded27` |
| `unrestricted.json` | `b81da3a807d914ba591f7d941ab035555cff2947f384457d24c9e41d45eb3eb8` |
| `helper` | `31c09cf156d87ec8c74beedaa1690b310e02afc188e0c680e7fca120c5623c8e` |
| `examples/profile_quadratic_components.rs` (repository source) | `d8067f079750b09af5353bcbfdce0df6353eeef5b43937b98dc6f77d7a0d86ac` |
| Shared `examples/profile_weighted_squares.rs` (repository source) | `df2eab1f24f5280c40f875d29951539549f063e3b07f8861466e282f4b5d37ac` |

The first production proposal remains the independently switchable weighted
binomial rule. A future component rule should use its own opt-in option,
typed evidence, independent replay, and whole-problem ablations, preserving
the distinction between this measured premise gain and full-domain coverage.
