# Exact Gluza30 negative-domain reduction (research)

The original negative fixture is
`artifacts/corpus/symbolic/0330_test_gluza_ex30_order_1.toml`, SHA-256
`dd771d262c1eda5ab22e4f43026c5f3c3e29c0a2a8d96e83d96dfa6d814c3830`.
It requests `F<0` with all seven coordinates positive. Its only variable group
is `[x0,x1,x2,x4,x5,x6,x7]`; there is no fixed order. The upstream test fixes
the original homogeneous coordinate `x3=1`.

The identities below are now implemented by the generic `linear_negative`
engine. A canonical cold debug campaign completed and independently verified
the exact original negative problem in one cell; metadata appears below.

## Source graph and exact reconstruction

The momentum routing in equation (4) of
[Dubovyk et al., arXiv:2201.02576](https://arxiv.org/html/2201.02576v2#S3)
describes a three-loop self-energy graph with one massive internal line. Its
fourth propagator corresponds to the upstream coordinate `x3`. Setting that
coordinate and the normalized mass and external invariant to one gives the
following exact construction:

```text
M = [[x0+x2+x5+x6, -x0-x6,       -x2],
     [-x0-x6,        x0+x1+1+x6,  -1],
     [-x2,           -1,           x2+1+x4+x7]]
j = [x5+x6, -x6, x7]
c = x5+x6+x7
U = det(M)
V = c*U - j^T*adj(M)*j
F = U-V.
```

Symbolica reconstructs the actual 59-term input `F` exactly. The 36-term `U`
has strictly positive coefficients. This reconstruction is checked, rather
than inferred from the topology label.

For each original coordinate `z=x_i`, write `F=A*z+B`, `U=C*z+D`. Both `C`
and `D` have strictly positive coefficients. The exact identity

```text
F_i*U - F*U_i = A*D-B*C = -H_i^2
```

holds for all seven axes. The helper constructs `H_i` directly from the
adjugate and the propagator's loop/external momentum coefficients, then
checks the expanded identity. The numerators have 11 or 18 terms. Their
signs need not be known because their squares are nonnegative.

## Fiber theorem and a four-step chain

For a linear polynomial `q=A*z+B`, suppose an exact certificate establishes
`C>0`, `D>0`, and `B*C-A*D>=0` throughout the original positive base domain.
Then `A>=0` implies `B>=0`. Consequently,

```text
q<0 and z>0  <=>  A<0 and z>max(0,-B/A).
```

On the base `A<0`, the lower boundary is continuous and semialgebraic. It is
the **largest distinct real root** of `z*q`. At `B=0` the two roots coincide;
the largest-root selector remains valid even though the ascending distinct
root index changes. This is a specialized continuous-boundary theorem; it
does not assert ordinary discriminant-based analytic delineability across
that collision. No seam `B=0` needs to be removed.

The following exact derivative chain succeeds:

| Peeled axis | Current polynomial | Proof of the implication |
|---|---|---|
| `x0` | `F` | `B*C-A*D` is an 18-term polynomial squared; `C,D>0` |
| `x4` | `F_x0` | `B*C-A*D` is a six-term polynomial squared; `C,D>0` |
| `x1` | `F_x0x4` | `B*C-A*D=x5^2`; `C=1`, `D=1+x2+x5>0` |
| `x5` | `1-x5-x6-x7` | The leading coefficient is directly `-1` |

In the second row the square numerator is
`x7+x5+x5*x7+x2*x7+x2*x5+x1*x7`. The third row's current polynomial is

```text
(1+x1+x2)*(1-x6-x7) - x5*(x1+x2+x6+x7).
```

After the fourth derivative the endpoint problem is the constant `-1<0`
over free positive coordinates `[x2,x6,x7]`. Reversing the chain gives the
legal original lifting order `[x2,x6,x7,x5,x1,x4,x0]`. The theorem gives one
continuous cylindrical negative-domain cell with four largest-root lower
boundaries, as confirmed by the implementation and independent coverage
replay. This is solely the original negative problem; a positive
problem or a common `F±` arrangement needs its own coverage argument.

The generic implementation accepts an optional polynomial guide `U` and
checks each supplied comparison identity independently. It does not trust
the graph name, a sampled sign, or the history of how the guide was found.
The guide need not be a graph polynomial if the algebraic premises hold.

## Failed screens and measured evidence

Before using the source graph, the following exact research screens were
inconclusive:

* Positive coordinate-sum multipliers `h*A±B`, and five constant multipliers
  with the existing domain certificate portfolio, found no global witness.
* The seven minors against the restored-coordinate derivative
  `F_T=2*P2+P3` factored into mixed-sign factors without sign certificates.
  A sign failure away from `F=0` would only refute this particular global
  minor route.
* A deterministic search of 10,000 positive rational points found no
  `A>0,B<0` counterexample. That absence was not used as proof.

The separate Euler identity `sum_i x_i*F_i-3*F=P4-P2`, with `P2>0` and
`P4<0`, proves absence of a simultaneous positive critical zero and one
crossing per positive radial ray. It does not alone provide a cylindrical
transport into the original coordinates.

The self-supervised Rust helper is `examples/gluza_linear_structure.rs`.
All algebra and exact rational checks use Symbolica. The final chain run
used CPU8, a 4 GiB worker cap and a 60-second deadline:

* Artifact: `artifacts/research/gluza30-rayleigh-chain-05/report.json`.
* Wall time: 0.042779 s (internal algebra/report time 0.031066 s).
* Peak RSS: 10,100 KiB.
* Frozen helper SHA-256:
  `50a129d018e5d8973f9d594c90a851cde3eaa7d03e289c3261ca501e288ba353`.
* Frozen source SHA-256:
  `c8409ebd9ab59e4b1a855880fb8e15ff06e42f640fc071774d45cacb8c5f4ead`.

The report contains full `F,U,V,C,D,H` expressions and reconstructed factor
identities. Earlier failed screens are retained in
`gluza30-linear-structure-01`, `gluza30-anchor-minors-02`, and
`gluza30-corner-countersearch-03`; the first successful seven-minor identity
check is in `gluza30-graph-rayleigh-04`. These are exploratory algebra
measurements, not solver benchmark timings.

## Canonical original-problem completion

Production commit `18cd5d0` implements the supplied-guide comparison rule,
the separate strictly-negative-leading rule, descending real-root indices,
and independent exact reconstruction of all fibers. The original fixture's
mathematical fields are preserved; the solver variant supplies the exact
guide and legal order above. The output retains the original coordinates
and domain and contains **one verified cell with four certified peels**.

The cold campaign used one worker on CPU6, an 8 GiB cap and separate
120-second solve/verification budgets. This was **one debug trial**:

| Measurement | Result |
|---|---:|
| Public solve, including preparation/export | 0.265087003 s |
| Fresh independent verification | 0.021131142 s |
| Solve peak RSS | 20,908 KiB |
| Verification peak RSS | 12,392 KiB |
| Verified base stacks / rejected branches | 3 / 3 |

The ledger is
`artifacts/hard-campaigns/linear-negative-debug-01/gluza30/results/ledger.json`.
The ledger SHA-256 is
`dc29054bed2ac51d3dd788e705597e610498c0a73da56c83b54221113c94b095`.
Its mathematical identity is
`8897487deeecc7e1f37d796074efee71a9d00334096414bcf56392c78588dfd4`.
The immutable worker SHA-256 is
`4da623f39bc7f552fd3dbb716f85162d7000f92a3d0178b5685333362dd173af`;
the result SHA-256 is
`11998554e47fabfee080a4d8efb33f4d0d01e57e56d41ae00bb0d87791916aea`;
the independent verification SHA-256 is
`c046865fe529a60d75aefad6b999f68bbd22a4e8ff40533d7ba8059f8197b971`.
These times demonstrate a completed original case. They are not a matched
release speed comparison against upstream or the earlier local campaigns.
