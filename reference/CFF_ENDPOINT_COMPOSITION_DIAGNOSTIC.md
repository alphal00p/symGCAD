# Connected cuts and compositional endpoint positivity

This follow-up to [CFF-informed projection](CFF_KINEMATIC_PROJECTION.md)
tests a graph-derived chamber proposal against actual symbolic ZZ operands.
The new result is a measured certificate-search gap: on `u<4`, existing
certificates prove two positive graph blocks independently, while the current
portfolio cannot prove their expanded positive combination. This is a candidate
for a general certificate improvement, not an original A446/Ax12 completion or
a solver speedup measurement. No production implementation changed.

## Graph, coordinates, and connected cuts

The native graph export is
`artifacts/fastsecdec-a446-interop-01/export/symanzik.json`, SHA-256
`0a9f511085e9ebdb81904f0ee5e8b00aeca1783596d22e0fb6a09c6bbeed9795`.
An unchanged full copy, including dressed DOT, routing, Gram matrix, native
polynomials, and provenance, is retained as `original-native-graph.json` in
`artifacts/research/cff-kinematic-followup-01/`. These generated files remain
outside commits. The full internal incidence and parameter map are:

| Edge | Directed endpoints | Native parameter | Corpus parameter | Squared mass |
|---|---|---|---|---|
| 4 | 2 → 0 | x0 | x1 | 1 |
| 5 | 0 → 4 | x1 | x2 | 1 |
| 6 | 4 → 3 | x2 | x3 | 1 |
| 7 | 1 → 2 | x3 | x5 | 1 |
| 8 | 5 → 1 | x4 | x4 | 1 |
| 9 | 3 → 5 | x5 | x7 | 1 |
| 10 | 2 → 3 | x6 | x8 | 0 |

External edges 0 and 1 inject `P0,P1` at vertices 0 and 1; edges 2 and 3
extract `P2,P3` at vertices 4 and 5. Momentum conservation gives
`P3=P0+P1-P2`. The independent Gram entries are

```text
P0²=P1²=0, P2²=m,
P0·P1=s/2, P0·P2=(s+t-m)/2, P1·P2=(m-t)/2.
```

Thus the convention is `t=(P0-P3)²` and `u=(P0-P2)²=2*m-s-t`.
Only `m=5/18` and the internal mass `MT=1` are used for the symbolic
experiment; the export's additional fixed-point substitutions `s=10,t=-3`
are **not** applied. Native graph-to-corpus U/F reconstruction was checked by
the previously recorded FastSecDec tests, including the exact edge map and
signed coefficients; see its `examples/no_deformation/a446/README.md`.
The current symGCAD importer still does not confer graph-derivation authority
on arbitrary metadata, so a production graph certificate must replay those
identities itself.

A bounded combinatorial enumeration inspected all 31 unoriented nontrivial
bipartitions of these six vertices, retaining the 15 whose two sides are
connected. Bounds were 12 vertices and 32 internal edges. Each retained cut
crosses exactly two massive edges, optionally also the massless rung.
Using the exported routing, the 15 external-injection norms were independently
reduced with Symbolica against the exported Gram matrix:

| Exact cut invariant | Connected cuts | Sum of cut masses | Candidate normal threshold |
|---|---:|---:|---|
| 0 | 6 | 2 | Never reaches 4 |
| m | 4 | 2 | `m=4`, absent at `m=5/18` |
| s | 1 | 2 | `s=4`, already excluded by `s>4` |
| u | 4 | 2 | `u=4` |

The exact comparison packs each Gram-norm identity in its own power of an
auxiliary variable, with constant term 1 to prevent normalization from hiding
a scale. No numerical momentum evaluation is used. This is a connected-cut
proposal plus exact polynomial identity, not a generated full CFF expression,
an energy-orientation proof, or a complete Landau stratification. It supplies
no additional normal-cut chamber within `u>4`. Anomalous, projective/infinity,
and intermediate projection loci must still be retained when required.

## Exact endpoint block identity

In corpus coordinates set

```text
a=x1+x2+x3, b=x4+x5+x7, z=x8,
U=a*b+(a+b)*z,
F=(a+b)*U-s*x2*x4*z
  -u*(b*x1*x3+a*x5*x7+z*(x1+x5)*(x3+x7))
  -(5/18)*(b*x2*x3+a*x4*x7+z*(x2+x4)*(x3+x7)).
```

The complete signed identity `F_normalized=18*F` was freshly replayed with
Symbolica against both original TOMLs: `u=5/9-s-t` for A446 and `u=t` for its
Ax12 crossed fixture. Their original `s>4`, unrestricted `t`, and all-positive
Feynman-parameter domains are preserved by these invertible changes. This
research identity does not yet provide certified cylindrical transport.

The endpoint `z=0` contracts the rung (it does not delete it). Vertices 2 and
3 become a common articulation vertex between the two three-edge loops. With
`U1=a`, `U2=b`, the exact contraction and block composition are

```text
U|z=0 = U1*U2,
B=F|z=0 = U2*F1 + U1*F2,
F1=a²-u*x1*x3-(5/18)*x2*x3,
F2=b²-u*x5*x7-(5/18)*x4*x7.
```

The endpoint composition was checked by exact Symbolica expansion against
the original signed F; articulation metadata alone was not accepted as a
proof. Writing `h=4-u>0` gives separate strict certificates

```text
F1=(x1-x3)²+h*x1*x3+x2²+2*x1*x2+(31/18)*x2*x3,
F2=(x5-x7)²+h*x5*x7+x4²+2*x4*x5+(31/18)*x4*x7.
```

Both `Ui>0` and both residuals are strictly positive on the whole chamber.
Consequently `B>0`; this is a sum/product proof with independently checkable
leaves. The implication also holds at `u=4`, but a generic chamber split must
either specialize this seam or record `u-4` in its explicit exceptional set.
It must keep the entire `u>4` complement and every interior `F=0` sheet.

## Measured distinction between search failure and sign obstruction

The compared operands include `B`, the two `Fi`, and the constant coefficient
of the quadratic slope `A=dF/dz` in `x2`:

```text
X=x1+x5, Y=x3+x7, C0=(X+Y)²-u*X*Y,
W=2*X+(31/18)*Y,
A=x2²+(W-(s-2)*x4)*x2+(x4²+W*x4+C0).
```

Frozen `profile_recursive_quadratic` tried the current positivity portfolio
and recursive proposals through depth 3 (128 terms, 128 nodes, and one million
work units per trial). All trials completed without exhausting those budgets.

| Operand | Original `u` unrestricted | `u<4` current portfolio | `u<4` recursive proposals | `u>4` |
|---|---|---|---|---|
| F1 | Inconclusive | Positive square plus strict residual | Already proved | Inconclusive |
| F2 | Inconclusive | Positive square plus strict residual | Already proved | Inconclusive |
| B | Inconclusive | Inconclusive | Inconclusive | Inconclusive |
| C0 | Inconclusive | Inconclusive | Inconclusive | Inconclusive |
| x4²+W*x4+C0 | Inconclusive | Inconclusive | Inconclusive | Inconclusive |

An independent current release solve containing only the two block splits on
this chamber exported their actual `square_plus_positive_residual` evidence.
The saved result has one positive cell and independently verifies with nine
stacks and nine rejected branches. This completes **only that leaf test**.
Its certificate residual uses the transformed positive coordinate `h=4-u`;
the serialized coordinate name `u` must not be mistaken for the original
unrestricted invariant when reading the shifted expression.

For every `u>4`, B really takes both signs on the positive orthant. The
freshly replayed original-identity helper retains two exact parameter families
proving this, as documented in [the quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md).
For example, with `delta=u-4>0`, take
`x1=x3=x5=x7=10*(1+delta)` and `x2=x4=delta`; then

```text
B=-2*delta*(20+21*delta)
  *(100*delta²+(1456/9)*delta+565/9)<0.
```

Taking `x1=2*u+1` and the other five endpoint parameters equal to 1 instead
gives `B=4*u²+(427/9)*u+157/3>0`. Thus no further purely kinematic partition
inside `u>4` can give uniform B positivity over the original entire orthant.
The `u<4` failures are certificate-search limitations, whereas trying to prove
global B positivity on the complementary chamber is mathematically impossible.

An initial historical `profile_certificate` binary predates the square
kernel. Its records are retained, but its failures are not evidence about the
current portfolio. It proves B, C0, and the quadratic constant under the
stronger `u<2` bound by coefficient signs. That narrower sufficient bound
should not replace the graph-derived `u<4` threshold or erase `2<=u<4`.

## General implementation candidates, ranked

1. **Compose existing positive proofs across graph blocks.** Enumerate a
   bounded number of contractions suggested by affine endpoint variables,
   decompose their graphs at articulation vertices, and propose
   `U=product Ui` and `F=sum_i Fi*product_{j!=i} Uj`. Reconstruct the complete
   identities with Symbolica and require a bijective edge/parameter map,
   incidence/routing checks, correct inherited external injections, the
   endpoint map, and every sign/scale. Admit only a bounded acyclic proof of
   sums, products, and already verified leaves. A positive product requires
   every factor positive; a positive sum needs nonnegative summands and an
   explicitly strict summand. This benefits graphs with articulation blocks
   or such endpoint contractions without testing topology names. A positive
   endpoint is only an endpoint premise; it does not prove the full F positive.

2. **Integrate a bounded weighted multi-square residual rule.** Here B is
   `b*(x1-x3)²+a*(x5-x7)²+R`, `R>0`. Expanding a and b supplies six squares
   weighted by positive coordinate monomials. This gives an algebra-only
   route independent of graph availability. Keep rational weights, exact
   full reconstruction, whole-domain weight signs, strict residual evidence,
   and term/height/work/square-count caps. This class is already implemented
   experimentally in `examples/a446_jacobian_sos.rs`, and proposed in
   [positivity generalizations §3](QUADRATIC_POSITIVITY_GENERALIZATIONS.md).
   Its earlier Jacobian campaign removed four negative terms but remained
   inconclusive; that does not test the endpoint B success above. Production
   `domain.rs` currently tries one constant-weight coordinate-binomial or
   affine square. Neither the research loader's chart-specific assumptions
   nor its topology-specific source layout should enter the generic kernel.

3. **Retain a mixed parameter/kinematic fallback for the hard complement.**
   Normal-cut chambers do not finish this problem. The existing exact
   `C0>=0 => B>0` implication is a coupled premise, not a kinematic-only
   certificate; its complement needs separate exact boundary analysis. For
   C0 on `u<4`, the natural identity is
   `C0=(x1+x5-x3-x7)²+h*(x1+x5)*(x3+x7)`.
   Its four-term square exceeds coordinate-binomial support, so the proposed
   six-square B rule alone does not imply every missed operand becomes easy.
   Bounded larger-support squares or exact positive block-variable grouping
   are distinct follow-ups. Preserve the critical/interior projection
   obligations already identified in the filtered Jacobian research.

## Timing and immutable evidence

Every helper was sequential, pinned to CPU 3, limited to 2 GiB and 120 seconds;
inputs and frozen executables were hashed before and checked unchanged after.
No Cargo build, native FastSecDec generation, or original full solve was run.
Times below are single diagnostic runs, outside benchmark timings.

| Diagnostic | Supervised wall time | Peak RSS |
|---|---:|---:|
| Original signed identities and endpoint composition | 0.0633 s | 11,268 KiB |
| Current five-operand portfolio on `u<4` | 0.0623 s | 7,280 KiB |
| Two-leaf release solve | 0.07 s | 6,168 KiB |
| Independent leaf-result verification | 0.02 s | 6,184 KiB |
| Fifteen exact Gram channel identities | 0.01 s | 3,084 KiB |

All paths below are relative to `artifacts/research/cff-kinematic-followup-01/`:

| Artifact | SHA-256 |
|---|---|
| `connected-cuts.json` | `8c9cc294dbb057ddc9650f5f8a4ec093eec587aaae8bde9733d2b438b19ee1ea` |
| `channel-identities.json` | `132b09c63cb8ec304bd4a7fec136a1691ad52edba77fd1cbb1fdabb80fcd8871` |
| `original-identities.json` | `74e88a01ff8b7ba9f247347448a8f13d69bd7ec6e3e69cc46df43af5814b93e1` |
| `current_u_below_4.json` | `8752c63dcae3006e17d79d475ea66c11428f4de9955208fdb6c61df43ecfae5f` |
| `block-leaves.toml` | `644c0b16b92dc09b6299a32cbcaec9271f9f13b0207114d5997a3f4745a7d0fe` |
| `block-leaves-solve.json` | `00f373c7982ede5e8a6c51937d834f54a1fe2f44de721ac027df2d64d7075164` |
| `block-leaves-verify.json` | `f3843423a13c1167b985194ab08142f6d6453a192edb3b1023c3a84cde9affdf` |

The original TOML hashes are
`75a51a1b359b0cef36e4b65965bfe830df13d80f1e04babc6c58df0218667047`
(A446) and
`ca89810d876b2bcde20ffc86b18928380eb4b26a418b6ab0f1accbb382bf1e3d`
(Ax12), also retained with their paths and the scale/coordinate changes in
`original-identities.json`. Executable hashes:

| Executable | SHA-256 |
|---|---|
| Original `a446_kinematic_partition` frozen helper | `e1c5498f37d03398951d5c14367466477efee8b59f2a418902d21b2556c50bc7` |
| `bin/profile_recursive_quadratic` | `bbaf53c15f0f7cec8a1ec7455635d2969a3b7ae421c574c741901c5079ceff0f` |
| `symgcad-shared-fibers-resource-9773999-release` | `acd80197327fb6c8dfe9454c47be6b9ecf26491d3b1755d01bf33deebd1f20b1` |
| `bin/endpoint_identity` | `addc74a94abc1762c02822c4a73ab619fba97603ec9e70e502d5831492d22381` |

The reproducible candidate-generation policy is graph incidence and symbolic
identity driven. No topology identifier is a proposed solver dispatch rule.

## Generic weighted-square prototype: measured gain

The independent research helper `examples/profile_weighted_squares.rs`
now implements the second proposal without graph data or topology dispatch.
The earlier `examples/profile_monomial_squares.rs` instead proposes one
constant-weight square of monomials with an existing certificate for the
residual; its even-diagonal support does not include the six odd-degree,
monomial-weighted squares required here.
It accepts a JSON variable map, original strict positivity constraints, and
labelled polynomial expressions. Exact affine coordinate bounds are derived
from those constraints, then each active coordinate is translated/reflected
to a strictly positive coordinate. An unbounded active coordinate causes this
conservative prototype to decline, including when more elaborate even-power
reasoning might have been possible.

For a negative coefficient `-c*x^e`, the discovery pass searches positive
coefficients `a*x^v`, `b*x^w` with `v+w=2*e` and `4*a*b>=c²`.
It chooses a rational weight in `[c²/(4*b),a]` and records the exact
monomial-weighted binomial square that cancels that negative term. Used
diagonal coefficients remain nonnegative. This prototype only tries full
single-pair cancellation; the older Jacobian helper's partial multi-pair
search is deliberately not needed for this gain. A fixed portfolio combines
four negative-term orders, three pair rankings, and three rational weight
choices, in both sign orientations.

Independent replay rederives the shifts from the original constraints,
validates every exponent and rational weight, checks the support of each
monomial weight, requires a nonzero positive-coefficient residual in strictly
positive coordinates, and reconstructs the entire signed shifted polynomial
with Symbolica. The saved certificate is serialized/deserialized and replayed
again. Discovery's mutable coefficient map is not trusted by replay.

On the authentic B operand, the existing production portfolio is inconclusive;
the prototype finds exactly the six monomial-weighted squares obtained by
expanding `b*(x1-x3)²+a*(x5-x7)²`. It succeeds in its first strategy with
252 pair scans. The residual has 24 positive-coefficient monomials, so zero
seams are excluded by strictness rather than presumed absent.

| Authentic operand on `u<4` | Current portfolio | Prototype | Discovery including first replay | Separate serialization/replay |
|---|---|---|---:|---:|
| B, 42 terms | Inconclusive | Six squares; verified positive | 0.00242 s | 0.00157 s |
| Quadratic constant, 19 terms | Inconclusive | Inconclusive | 0.00494 s | — |
| C0, 14 terms | Inconclusive | Inconclusive | 0.00403 s | — |

These are single development-profile diagnostics, not benchmark speedup
ratios. The whole three-operand process took 0.03 seconds with 3,076 KiB peak
RSS, under the same CPU 3 / 2 GiB / 120-second envelope. The two inconclusive
operands exhausted all 72 sign/strategy combinations, without reaching the
work cap. Their four-term-square limitation remains as described above.
Neither this proof nor the leaf test completes an original symbolic domain.

Limits are 256 terms, 16 variables, exponents at most 32, 512-bit combined
rational numerator/denominator height, 32 squares per trial, and one million
pair scans shared across all trials for one polynomial. External process
limits remain necessary because a single Symbolica call is not interruptible.
The four focused tests pass, covering two structurally different weighted
polynomial families, negative orientation, an explicit upper-bound shift,
work exhaustion, a pure square with an interior zero seam, a sign-changing
negative weight, an unrestricted weight, and tampered residual/weight/domain
evidence. No production sources were changed in this experiment.

Retained artifacts are in `artifacts/research/weighted-squares-01/`:

| File | SHA-256 |
|---|---|
| `a446-b-u-below-4.json` (input) | `e5bfed73cac360a50886a06980e4bd2b48a8dd3390c32ae66efa9ff5ae518609` |
| `endpoint.json` (proof and timings) | `11850cf9e41a80b35e683d3236e8e92b143c08fb1bed5ef4d7065664382ca6bd` |
| `helper` (frozen executable) | `3377f82eeb6032c42101fac8e9885334446429e310b1e9ccba6f3180a88674b3` |
| `examples/profile_weighted_squares.rs` (source at `9d1db51`) | `4584bdbff30a9e53bac814164244abfb667a50078ee6a9fa3fbd0e1a869b1d5e` |

The actual proof includes `u_original=4-u_positive` explicitly. Before/after
hash checks bind the input, source, and frozen binary; build/test logs and
separate process records are retained alongside the report.

A subsequent, separate [quadratic-component experiment](QUADRATIC_COMPONENT_CERTIFICATE.md)
now certifies C0 and the quadratic constant on `u<4` using a four-term linear
square, while correctly retaining C0's interior zero seam at `u=4`.
