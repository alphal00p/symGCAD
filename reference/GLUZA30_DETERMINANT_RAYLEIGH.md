# Determinantal Rayleigh identities for positive Gluza30

All 21 coordinate pairs have exact square Rayleigh differences. Only one
of their square factors is nonzero throughout the positive orthant. The
other 20 have certified positive and negative rational values in that
orthant, so they cannot supply the proposed global strict Bézout premise.
This is an algebraic diagnostic, not a decomposition or a new completion.

The input is the original positive problem
`artifacts/corpus/symbolic/0332_test_gluza_ex30_solve_3.toml`, SHA-256
`9fdfee0cdf8e45c19e6ba7e7fb6ebda50fdb4a223129aa61ed841bca53942841`.
Every variable `x0,x1,x2,x4,x5,x6,x7` has an original literal strict
positive-coordinate constraint. The target polynomial is used with its
original sign. No homogeneous normalization, kinematic specialization,
coordinate transformation or domain restriction is introduced.

## Exact determinant and square identities

The momentum routing is the one already reconstructed in
[the negative Gluza30 report](GLUZA30_RAYLEIGH_REDUCTION.md), from
[the example in Section 3 of arXiv:2201.02576](https://arxiv.org/html/2201.02576v2#S3).
Set

```text
v0=(1,-1,0,0)    v1=(0,1,0,0)    v2=(1,0,-1,0)
v3=(0,1,-1,0)    v4=(0,0,1,0)    v5=(1,0,0,1)
v6=(1,-1,0,1)    v7=(0,0,1,1)

K = v3*v3^T - e4*e4^T + sum_{i != 3} xi*vi*vi^T.
```

The helper expands the four-dimensional determinant and verifies
`F=-det(K)` against the exact original target. It then constructs every
entry of `adj(K)` through exact three-dimensional minors and checks

```text
H_ij = vi^T adj(K) vj
F_i*F_j - F*F_ij = H_ij^2.
```

The square identity follows from rank-one determinant differentiation;
it does not require `K` to be positive definite. The broader connection
between rank-one symmetric/Hermitian pencils and square Rayleigh
differences appears in Al Ahmadieh and Vinzant,
[Section 5.2, equation (7) and Theorem 5.6](https://arxiv.org/pdf/2205.05267).
Here the exact expanded identities, rather than a recognition theorem,
are the checked evidence.

For every pair the helper also verifies independence of `H_ij` from
`xi,xj` and, writing `F=a*xi*xj+b*xi+c*xj+d`, the identities

```text
b*c-a*d = H_ij^2
F_i*c - (F|xi=0)*a = H_ij^2
F_j*b - (F|xj=0)*a = H_ij^2.
```

These are potential exact Bézout combinations for factors of the leading
and endpoint polynomials. Squareness establishes nonnegativity only.
Omitting a pair resultant by the current strict rule also requires
`H_ij != 0` throughout its certified domain.

## All-pair outcome

| Pair | Terms in H | Result on the positive orthant |
| --- | ---: | --- |
| x0, x1 | 11 | Both signs certified |
| x0, x2 | 16 | Both signs certified |
| x0, x4 | 11 | Both signs certified |
| x0, x5 | 9 | Both signs certified |
| x0, x6 | 16 | Both signs certified |
| x0, x7 | 11 | Both signs certified |
| x1, x2 | 9 | Both signs certified |
| x1, x4 | 16 | Both signs certified |
| x1, x5 | 14 | Both signs certified |
| x1, x6 | 12 | Both signs certified |
| x1, x7 | 9 | Both signs certified |
| x2, x4 | 11 | Both signs certified |
| x2, x5 | 14 | Both signs certified |
| x2, x6 | 9 | Both signs certified |
| x2, x7 | 12 | Both signs certified |
| x4, x5 | 9 | Both signs certified |
| x4, x6 | 11 | Both signs certified |
| x4, x7 | 16 | Both signs certified |
| x5, x6 | 14 | Both signs certified |
| x5, x7 | 14 | Both signs certified |
| x6, x7 | 9 | Strictly positive coefficients |

The last square factor is

```text
H_67 = x5 + x2*x4 + x1*x2*x4 + x0*x4 + x0*x4*x5
       + x0*x2*x4 + x0*x1 + x0*x1*x4 + x0*x1*x2.
```

For each of the other 20 polynomials, the diagnostic checks multiaffinity
and constructs two exact positive rational points. To make one chosen
monomial dominant, it sets coordinates present in that monomial to `T`
and the others to `1/T`. The search uses `T=2,4,...,4096`, verifies the
evaluated rational sign exactly and retains both full points and values.
Every coordinate is strictly positive, so every original coordinate-domain
constraint is satisfied. The original `F>0` target is not assumed or
tested: these witnesses concern the target-free positive domain needed by
the proposed global certificate.

All 20 searches succeed in both signs. The segment joining the two
positive points stays in the positive orthant, and continuity gives an
interior zero of each corresponding `H`. This rules out a global strict
nonvanishing certificate for these particular square factors. It does not
prove that the associated leading and endpoint polynomials actually share
a zero, nor does it rule out certificates restricted to sign chambers.

## Reproduction and limits

`examples/gluza_determinant_rayleigh.rs` has three passing tests: exact
cofactor-square identities for positive and indefinite matrix pencils,
and exact sign witnesses with rejection of a non-multiaffine square.
The helper supervises its worker with a 60-second, 4-GiB, one-worker cap.

The final immutable diagnostic is under
`artifacts/research/gluza-determinant-rayleigh-02/`:

- Helper SHA-256:
  `3ff3932fa5fb810be51c2fdd357744311ee588c49ceb90938557b585b9d7707d`.
- Source SHA-256:
  `b618019e5354b20f9179a762832fdcfd8cc74286e671ea7f65bb96529d25e6d2`.
- Report SHA-256:
  `3956d9b6577d9c8e502ecf2ee45593b32f3fcb7e1231c7049b1bbee6254d2edc`.

The CPU8 run took 0.0606 seconds inside the helper and 0.0843 seconds
under supervision, with peak RSS 7,388 KiB. These are single diagnostic
measurements, not solver timings. The earlier `-01` report preserves the
initial square-only run. No production hint or theorem is accepted from
either report without the independent exact checks in the solver.
