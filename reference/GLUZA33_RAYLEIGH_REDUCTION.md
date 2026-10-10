# Exact two-step Gluza33 reduction (research)

The negative original fixture is
`artifacts/corpus/symbolic/0336_test_gluza_ex33_order_1.toml`, SHA-256
`7c55d029c15b1816315785559d12e2cbab3677cec77fe5da6d5fc8f2779bdfed`.
It has eight positive coordinates, one variable group and no fixed order.
The following identities are independently expanded and checked with
Symbolica; this report does not itself provide a completed original CAD.

## Exact graph identity

Equation (10) of
[Dubovyk et al., arXiv:2201.02576](https://arxiv.org/html/2201.02576v2#A1.SS3)
gives the massive self-energy routing. Its propagators in paper order map
to fixture coordinates `[x5,x0,x6,x1,x2,x3,x4,x7]`. Thus `x3,x5` are
massless, while the other six edges have the common normalized mass one.

Use the same loop matrix and external vector as in the
[Gluza30 calculation](GLUZA30_RAYLEIGH_REDUCTION.md), with its fixed `1`
entries restored to `x3`:

```text
M = [[x0+x2+x5+x6, -x0-x6,        -x2],
     [-x0-x6,        x0+x1+x3+x6,  -x3],
     [-x2,           -x3,           x2+x3+x4+x7]]
j = [x5+x6, -x6, x7]
c = x5+x6+x7
U = det(M)
V = c*U - j^T*adj(M)*j
L = x0+x1+x2+x4+x6+x7
F = L*U - 10*V.
```

The last equality reconstructs the full original polynomial exactly.
For either massless axis `i=x3,x5`,

```text
F_i*U-F*U_i = -10*H_i^2.
```

The massive axes have an additional mass derivative and are not covered by
this identity. No sign claim for those directions is assumed.

## Hereditary identities and the quadratic base

For each step `q=A*z+B`, `guide=C*z+D`, the helper checks `C,D>0` by their
strictly positive coefficients and verifies `B*C-A*D=10*H^2` exactly:

| Peeled axis | Current target | Square numerator |
|---|---|---|
| `x3` | `F` | An 11-term polynomial retained in the artifact |
| `x5` | `F_x3` | `(x0+x2)*(x1+x4)-x6*x7` |

Both square numerators have mixed signs; their signs are irrelevant to the
nonnegative-square proof. The exact negative-fiber theorem is the one
proved in the Gluza30 report: `q<0,z>0` is equivalent to `A<0` and a lower
bound at the largest real root of `z*q`.

After these two steps, the residual is exactly

```text
F_x3x5 = A^2 - 8*A*B + B^2
A = x0+x2+x7
B = x1+x4+x6
U_x3x5 = A+B.
```

This is a 21-term quadratic in the six remaining original coordinates.
Because `A,B>0`, its negative region is
`(4-sqrt(15))*B < A < (4+sqrt(15))*B`. The proposed legal lifting order is
`[x6,x1,x4,x0,x2,x7,x5,x3]`, placing the `B` coordinates before the `A`
coordinates and the two restored fibers last. An ordinary certified base
solve can handle the quadratic; neither an affine coordinate change nor a
homogeneous normalization is required for this proposal.

## Reproduction and scope

Run the Rust helper with the original fixture and `--graph33-minors`.
The guide and order are also saved in
[`gluza33_rayleigh_guide.json`](gluza33_rayleigh_guide.json). The helper's
own worker supervision limits it to one core, 4 GiB and 60 seconds.

The exact check on CPU8 completed in 0.042832 seconds wall time, with
10,828 KiB peak RSS. Internal algebra/report time was 0.026381 seconds.
These are research identity-check timings, not end-to-end solver results.

* Report: `artifacts/research/gluza33-rayleigh-chain-01/report.json`.
* Frozen helper SHA-256:
  `eb1fc8c5f5684bd0432c24d4f9c02ecbadb0af1d76bbe921291f56c1bf88834c`.
* Frozen source SHA-256:
  `31c37527ea072b0d13176dedf8a34eae7a825d54724df70e304f824977e2f0b1`.

The original negative problem needs a complete base solve, exact fiber
restoration, and independent coverage verification before receiving
completion credit. The positive fixture is a different problem and is not
solved by these two negative-domain identities alone.
