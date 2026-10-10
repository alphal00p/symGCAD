# Exact Jacobian research for the unresolved symbolic A446 complement

This is research evidence, not a production projection rule or an original-problem completion. The full original A446 and Ax446 problems retain `s>4`, unrestricted original `t`, and their mandatory parameter-first variable groups. Definitions of `A,B,R,u` are in [the quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md) and [the corrected resultant geometry](A446_COMPLEMENT_ROOT_GEOMETRY.md). The independent [projection audit](A446_FILTERED_PROJECTION_AUDIT.md) lists the remaining proof and selector obligations.

## Exact critical-point elimination

Write `p=x1,q=x3,r=x5,w=x7,x=x2,y=x4`, `m=5/18`, and

```text
a=p+q+x, b=r+w+y, L=a+b, Y=q+w,
T=b*p*q+a*r*w,
E=L*a*b-m*(b*x*q+a*y*w),
A0=L^2-m*(x+y)*Y,
P=b*p-a*r, Q=b*q-a*w,
V=L*P+m*Y*(r*x-p*y).
```

At any positive common zero of `A,B`, exact elimination gives

```text
u=E/T, s=S/(x*y*T), S=A0*T-(p+r)*Y*E=Q*V.
Nx=E_x*T-E*T_x > 0,
Ny=E_y*T-E*T_y > 0,
Jnum=y*(x*A0_x*T-S)*Ny-x*(y*A0_y*T-S)*Nx.
```

The original oriented Jacobian `A_x*B_y-A_y*B_x` is `Jnum/(x*y*T^2)`. Symbolica checks the identities, and `Nx,Ny` have strictly positive coefficients. This establishes coordinate monotonicity of `u` without a numerical assumption.

`examples/a446_positive_critical.rs` specializes 84 exact positive rational fibers (12 outer-coordinate sets and seven `x` values), isolates all positive roots of `Jnum` in `y`, and determines signs with exact rational intervals, Sturm counts and common-root checks. Among 117 positive critical points, 89 have `s<0` and 28 have `s=0`; none have `s>0`. All sampled critical points satisfy `u>4,C0<0`. This is a finite diagnostic, not a universal nonvanishing certificate.

Evidence: `artifacts/research/a446-positive-critical-01/report-v2.json`, SHA-256 `45f02e06c2fa4d05022d176714ed7a238d032965c92789d27aa47e00f588013c`; process record `2105df6d50f3bc5bf9892a0ee785d3b278271058340d628a005b0e5302b5469d`. The helper has two exact regression tests. The worker is bounded by the Rust supervisor, 120 seconds and 2 GiB; this diagnostic completed in about 0.31 seconds.

## A disproved monotonicity shortcut

Along a fixed-`u` arc, `f=Q/T` strictly decreases. Writing `s=f*g`, `g=V/(x*y)`, suggests proving that `g` decreases too. Its derivative has the numerator

```text
M=y*(x*V_x-V)*Ny-x*(y*V_y-V)*Nx.
```

The exact product identity is

```text
Jnum*T=x*y*V*(fxnum*Ny-fynum*Nx)+Q*M*T,
fxnum=-b*q*w*(p+r), fynum=a*q*w*(p+r).
```

`M` has 430 terms, 132 positive and 298 negative coefficients. The existing strict-sign certificate portfolio returned no certificate; checked factorization returned one mixed-sign factor. More decisively, a positive rational point disproves `M<0` even on the required kinematic region:

```text
(p,q,r,w,x,y)=(3,100,1/10,10,1,1/3),
s=9211/1260 > 4, u=3978697/103950 > 4,
C0=-983557/3780 < 0,
M=917488163954131/81000 > 0,
Q=10/3, Jnum=-3952165941293647/8100.
```

Thus the proposed global monotonicity of `g` must not justify removing any discriminant. The narrower Jacobian claim is not refuted by this point. A deterministic 4096-point exact rational screen found 1303 points with `s>0`; all had `-Q*Jnum>0`. Again, these samples do not prove that inequality universally.

Evidence: `artifacts/research/a446-filtered-jacobian-01/sample-v2.json`, SHA-256 `07f7ffd2f1286d08bfcce2e009129a08c19e550b921f843dec9ac81b6c6efeeb`; process record `5d69ac23a5c59a5b9345d5b2e42321eaf04c165c1ca891b2529f01f5b49a7f9b`. All full rational values are retained, not only floating-point signs. The bounded Rust worker completed in 4.69 seconds with about 11 MiB peak RSS.

## A complete chart of the same-sign cone

The exact identity

```text
a*V=Z*P-m*p*Y*Q,
Z=a*(L-m*Y)+m*q*Y > 0
```

proves that `S=Q*V>0` implies `P,Q` have the same nonzero sign. The positive cone has the polynomial parametrization

```text
p=h*(r+v), q=h*(w+z), x=h*t, y=t+v+z,
h,r,w,t,v,z > 0.
```

Here `a=h*b`, `P=h*b*v`, `Q=h*b*z`. Conversely every positive physical point with `P,Q>0` has this representation, with `h=a/b`, `t=x/h`, `v=p/h-r`, `z=q/h-w`. The exchange `(p,q,x)<->(r,w,y)` maps the negative cone to this positive cone and reverses `Q,V,Jnum`; Symbolica checks the symmetry exactly.

This chart has complete coverage of the same-sign cones, but it alone does not certify the Jacobian sign. The transformed `Jnum` has 2719 terms, including 74 positive coefficients. Some chart points have `V<0` and hence `s<0`, so the remaining positivity premise is essential. The helper attempts bounded exact identities of the form

```text
D*(-Jnum)=C_V*V+C_G*(Q*V-4*x*y*T)+R,
```

with strictly positive `D`, coefficient-nonnegative `C_V,C_G`, and a nonzero coefficient-positive `R`. Only a fully reconstructed identity with all required signs could certify the claim. Partial cancellation or a mixed residual is retained as inconclusive. No production code uses these research attempts.

All 12 tested choices `D=(1+h)^j*b^k`, `j+k<=2`, with the two premise orders, remained inconclusive. The simplest attempt performs 22 exact cancellations and leaves 44 negative residual coefficients. Every attempted identity is reconstructed exactly and stored. Positive-leading-coefficient pseudo-division in `v` also failed the sufficient coefficient test: the quotient has 2729 terms with 36 negative coefficients, and the degree-at-most-two remainder has 2964 terms with 227 negative coefficients. These failures do not prove that the Jacobian vanishes; they establish only the limits of this bounded certificate portfolio.

Final diagnostic: `artifacts/research/a446-filtered-jacobian-01/chart-v6.json`, SHA-256 `a3593c0277b458d946892e36f7a5541186a28cf7c921e18dfb6f243bab2e6d82`; process record `3ffb3ad01d3aee29e936ad27adbab91d989d05596a4f4811ee36fddee0f8b77a`; frozen executable `6a99104877374755d9e8c2b1b96140a2653936df943d8dae2dae20bb4fda192a`. All algebra and coefficient arithmetic are Symbolica; the Rust supervisor enforces a 120-second/2-GiB bound. The final diagnostic took about 10 seconds, with all attempts terminating normally. The helper's tests include exact exchange and chart identities, successful and failed coefficient-certificate examples, pseudo-division reconstruction, and the strict-complement counterexample above.

## Exact squares and a limitation of the saved residuals

`examples/a446_jacobian_sos.rs` replays those saved identities before attempting rational binomial-square extraction. For a negative monomial with exponent `e`, it searches positive terms at exponents `u,v` with `u+v=2e`. Each accepted cancellation is an explicit positive rational weight times a squared binomial times a positive monomial. Every resulting square decomposition and the full conditional identity are reconstructed with Symbolica. Several pairs may supply one cross coefficient; exact integer square-root bounds avoid treating a numerical square root as proof. Strictness requires a nonzero positive-coefficient residual, not merely a sum of squares.

The first bounded portfolio completed 336 attempts in 44.9 seconds. Its best attempt removes four squares from the previously best residual, reducing 44 negative coefficients to 40. This remains inconclusive. The report is `artifacts/research/a446-jacobian-sos-01/report-v1.json`, SHA-256 `9fae108d69b2a9eb6696de5a1ad80c67c0b61ad72e7dc8b967c962b6ad9046a3`.

The next diagnostic found an exact obstruction to this particular portfolio: **all 14 fixed saved residuals are negative** at the positive chart point

```text
(h,r,w,t,v,z)=(1/10,10,1/100,1/100,1/10,100).
```

Here `V<0` and `Q*V-4*x*y*T<0`. Therefore this point does **not** refute the desired conditional Jacobian inequality on `s>4`. It does prove that none of these fixed residuals can have a globally nonnegative square/coefficient decomposition, even after multiplication by a strictly positive polynomial. A useful next certificate must change the conditional multipliers or use the kinematic inequality more directly.

The second report retains all exact rational values and input-identity checks: `artifacts/research/a446-jacobian-sos-01/report-v2.json`, SHA-256 `93c84c409018654b12b132f24dfded766e38a0d1bf98b7411ceb53fbbc3480af`; process record SHA-256 `35fb14836a9b5d2373eb799517cc47fab1c0027c310bb43d9dc2f3b410d715a2`. Its Rust-supervised run completed in 10.63 seconds with about 24 MiB RSS. The frozen helper SHA-256 is `4461862b7dbbb4042619469a6d25f1d6e6dd8f3037beab6a1e3274fd3d4d1293`; four exact tests pass. No production rule or original-problem completion follows from these experiments.
