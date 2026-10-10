# Exact two-step Gluza33 reduction (research)

The negative original fixture is
`artifacts/corpus/symbolic/0336_test_gluza_ex33_order_1.toml`, SHA-256
`7c55d029c15b1816315785559d12e2cbab3677cec77fe5da6d5fc8f2779bdfed`.
It has eight positive coordinates, one variable group and no fixed order.
The following identities are independently expanded and checked with
Symbolica. The generic `linear_negative` implementation has subsequently
completed and independently verified the original negative problem in
three cells; canonical campaign metadata appears below.

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

## Canonical original-problem completion

Production commit `18cd5d0` solved the original eight-variable negative
problem with the exact guide and legal order above. Both massless axes were
peeled, and the ordinary optimized engine solved the quadratic base. Exact
fiber restoration and a fresh independent verifier produced **three cells**.
The positive fixture remains a different problem; the negative proof makes
no positive-domain coverage claim.

The cold campaign used one worker on CPU5, an 8 GiB cap and separate
120-second solve/verification budgets. This was **one debug trial**:

| Measurement | Result |
|---|---:|
| Public solve, including preparation/export | 0.348312438 s |
| Fresh independent verification | 0.041349322 s |
| Solve peak RSS | 23,208 KiB |
| Verification peak RSS | 15,596 KiB |
| Verified base stacks / rejected branches | 13 / 31 |

The ledger is
`artifacts/hard-campaigns/linear-negative-debug-01/gluza33/results/ledger.json`.
The ledger SHA-256 is
`c9c5a0f5e1a934fd601f0794951c3c05d48d21bdae900b19c8d9e137586391f9`.
Its mathematical identity is
`2fca160a061f5167336220dc49346af1f10187fe6734b5ea51a303f9eac41542`.
The immutable worker SHA-256 is
`4da623f39bc7f552fd3dbb716f85162d7000f92a3d0178b5685333362dd173af`;
the result SHA-256 is
`7beb9347653d054a6af4d3d1f3edf91a858dc11024c9060d70ce1e7602f4a2bc`;
the independent verification SHA-256 is
`b369fad13d53cb6eb1475085debf2d42ded3af54dd135daf6c5e499661f42706`.
These are completed-case measurements, not matched release benchmarks.
