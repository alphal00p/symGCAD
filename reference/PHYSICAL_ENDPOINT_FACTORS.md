# Physical symbolic endpoint projection audit

This is an exact algebra audit, **not a completion certificate** for the original integral domain. The original fixture is `artifacts/corpus/symbolic/0317_test_elliptic2l_physical_symb_order_1.toml`; its original constraint is `F<0`, with `s>4`, `t<0`, `0<pp4<4`, and positive Feynman coordinates. The helper reads that original input, retains its provenance, and uses Symbolica throughout.

Set `u=x0`, `v=x1`, `w=x3`, `z=x6`, `m=pp4`, and

```text
K = w+z
E = K²-m*w*z = (w-z)²+(4-m)*w*z > 0
H = s*w*z-K²
J = s*K²-s*m*z*K+m²*z²
T = s*K-m*z > 0
L = s*w*K-K²-m*w*z.
```

The exact endpoint identities checked against the original polynomial are

```text
c = F|x4=0 = A*x5+B
A = (u+v+w+z)²-s*u*(v+w)-m*z*(v+w)
B = K*((u+v)²-s*u*v)+(u+v)*E
disc_u(A) = (v+w)*(s*(s-4)*(v+w)-4*(s-m)*z)
disc_u(B) = E²-2*s*K*v*E+s*(s-4)*K²*v²
disc_v(disc_u(B)) = 16*s*K²*E².
```

The leading coefficients in `u` are `1` and `K`. The helper checks the quadratic discriminant formulas and independently compares a monic quadratic remainder formula with the resultant implementation. In particular it checks the following complete, signed resultant identity, including its scalar:

```text
Res_u(A,B) = P*Q
P = H*v-w*E
Q = -J*v-T*E.
```

Thus the 65-term resultant splits into two factors linear in `v`, with 8 and 16 expanded terms. Its discriminant in `v` has square-free factors `z`, `E`, `s-m`, and `L`. The first three are nonzero on the original domain; `L` remains a genuine possible boundary. This is a mathematical domain observation, not a claim that every current certificate heuristic automatically detects it.

Let `U=s*(s-4)*(v+w)-4*(s-m)*z` and `D_B=disc_u(B)`. All six subsequent pair resultants in `v` were factored and their factorizations reconstructed independently. Up to nonzero constants and multiplicity their factors are:

| Pair | Square-free factors |
| --- | --- |
| `U,D_B` | `s`, `s-4`, one 32-term quartic in `w,z` |
| `U,P` | `z`, `s-m`, `s*w-2*K` |
| `U,Q` | `z`, `s-m`, `s*K-2*m*z` |
| `D_B,P` | `E`, `s*w²-K²` |
| `D_B,Q` | `E`, `T²-s*K²` |
| `P,Q` | `z`, `E`, `s-m`, `L` |

The expanded quartic and all endpoints/leading coefficients are retained in the report. The quartic prevents interpreting the small factors as a finished decomposition. Its next projection and all root-order obligations still require certification.

These degrees support the legal lifting order

```text
s, t, pp4, x6, x2, x3, x1, x0, x5, x4
```

with positive homogeneous anchor `x6`: peel `x4`, eliminate the linear `x5`, then the quadratic `x0`, and eliminate `x1` before the generally higher-degree `x3`. The inactive coordinate `x2` remains in the original problem and order. This does not authorize changing any explicitly fixed fixture order.

At the time of this audit, the new binomial-square certificate recognizes `(x3-x6)²` before homogeneity but does not yet recognize `(x3-1)²` after fixing `x6=1`. The observed `x6`-anchor trial therefore had **zero actual monotone peels**. A constant-one square certificate is needed before that trial tests the endpoint reduction described here. An original solve and independent verification are required before counting any completion.

## Reproduction and bounded evidence

```sh
nix develop --command cargo test --example physical_endpoint_factors
nix develop --command cargo build --example physical_endpoint_factors
nix develop --command taskset -c 5 target/debug/examples/physical_endpoint_factors \
  --input artifacts/corpus/symbolic/0317_test_elliptic2l_physical_symb_order_1.toml \
  --output artifacts/research/physical-endpoint-factors-replay/report.json \
  --seconds 60
```

The Rust parent supervises the worker with one CPU allocation, 4 GiB memory, and the requested wall limit. Exact operation checkpoints, active-operation captures, a partial report, and a process outcome remain available if interrupted. No Python algebra or external CAS is used.

The frozen expanded audit in `artifacts/research/physical-endpoint-factors-02/` completed in **0.206462408 seconds wall time**, using 0.114316 seconds user CPU, 0.037437 seconds system CPU, and 13,548 KiB peak RSS. Its internal algebra/factor verification section took 0.174074289 seconds in this debug executable. These are small-operation measurements, not whole-GCAD timings.

| Artifact | SHA-256 |
| --- | --- |
| Original input | `e7a7dcf135b7043f1b7c9ee61a8f97997da7b26674086f17c82047fd9ad6ab6b` |
| Frozen helper executable | `a3ca7639f5c0506d20a83ec59ad71be9325f56a2a8af11e60e4967eea518f1a5` |
| Frozen helper source | `bae7cc49b7d54995312205e8603e95e1651f879f22b63f680affa947811487bf` |
| `report.json` | `14a9f1568a8678e041131aff55f1f6df9b7b7019ac5a4ca41cc1ce2420e46501` |
| `report.process.json` | `ea3192496a2b7fedf12e7a37b29e706c53a70bd0f5c992dfd432cdd1e7a064b5` |

The preceding `physical-endpoint-factors-01` report remains available; the second run adds exact compact resultant reconstruction and the six subsequent pair projections. Each helper build passed its exact signed-resultant/discriminant regression.
