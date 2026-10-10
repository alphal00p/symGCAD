# Exact Jacobian research for the unresolved symbolic A446 complement

This is research evidence, not a production projection rule or an original-problem completion. The full original A446 and Ax446 problems retain `s>4`, unrestricted original `t`, and their mandatory parameter-first variable groups. Definitions of `A,B,R,u` are in [the quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md) and [the corrected resultant geometry](A446_COMPLEMENT_ROOT_GEOMETRY.md). The independent [projection audit](A446_FILTERED_PROJECTION_AUDIT.md) lists the remaining proof and selector obligations.

**The final exact cubic construction below disproves universal Jacobian nonvanishing on positive common roots, even with `s>4,u>4,C0<0`.** Earlier finite screens and unsuccessful certificate searches are retained as research history, not evidence for that disproved statement. No production projection rule used it.

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

## A positive critical point: refutation of universal nonvanishing

The research helper `examples/a446_jacobian_cubic.rs` removes the exactly reconstructed positive factor `h^4*b^2` from `-Jnum`, leaving a cubic `C(h)` with 1104 terms and 33 negative coefficients. Write

```text
b=r+w+t+v+z, D=(r+v)(w+z),
A=v*(b^2-m*(w+z)*(r+t+v+z))-m*r*z*(w+z),
B=v*(b^2-m*w*(r+t+v+z))-m*r*w*z,
A4=z*A-4*t*(t+v+z)*D,
B4=z*B-4*t*(t+v+z)*r*w.
```

Symbolica checks `V=h*(h*A+B)`, `G=Q*V-4*x*y*T=h^2*b*(h*A4+B4)`, and the strictly positive coefficient expansion of `B4-A4`. Thus the region `s>4` is either all `h>0` when `A4>=0`, or the bounded interval `0<h<B4/(-A4)` when `A4<0<B4`. The helper also reconstructs the full denominator-cleared Möbius substitution for the bounded interval.

The unbounded case supplies a counterexample to the proposed theorem. Fix

```text
(r,w,t,v,z)=(1/100,10,1/100,3,100).
A4=648739417501/225000 > 0,
B4=842833784291/225000 > 0.
```

After multiplying `C(h)` by the positive constant `253125000000`, its exact cubic is

```text
246482922013046330125393
+412544870199872500653992284*h
+317543526023029208134644139*h^2
-1200636308364242347752*h^3.
```

Its coefficients have one sign variation, and exact Sturm computation finds exactly one positive root in `(1,524288)`. The direct, independently differentiated original `A_x*B_y-A_y*B_x` is negative at the first endpoint and positive at the second. Both original equations `A=B=0` are checked at the reconstructed exact rational kinematics; three further rational `h` values check the cubic against the original derivatives.

The whole connecting family stays strictly inside the proposed complementary domain. The positive denominator is `ud=h*(r+v)*(w+z)+r*w`. Exact numerators are

```text
(u-4)*ud = 70239271/5625 + (128799167/11250)*h,

(-C0)*ud = [1114846327 + 321733012951*h
            +3404446760921*h^2 + 582494297*h^3]/900000.
```

Every coefficient is positive, and independent Sturm checks also confirm no guard roots in the bracket. Together with positive `A4,B4`, this proves `s>4,u>4,C0<0` throughout. Continuity of the directly checked original Jacobian therefore proves an interior positive-common critical point. The full original parameter domains are respected: original `t=5/9-s-u` for A446 and `t=u` for Ax446 are unrestricted. No variable-group obligation is changed by this existential witness.

This is a **projection critical point**, not a proved physical Landau singularity, and a nondegenerate fold has not yet been established. It rules out simply omitting all relevant critical cuts using universal Jacobian nonvanishing. Further work must stratify the critical locus or use a different decomposition; repeating global sign-certificate searches for this false statement is inappropriate.

Final evidence: `artifacts/research/a446-jacobian-cubic-01/counterexample-v4.json`, SHA-256 `f885c73d46bf5aa7eec596afd24946daf35c781cb03930c29b554ad80542999b`; process record `115101dff31e797b1317af16a0a515017d007691b3c378ca69c7c0b2050009aa`. Frozen helper SHA-256 `5d3d65d75a1f6b30088a49853d756c446f8cb992b54e31610f034f6aaed34d65`. Four exact helper tests pass, including the strict-domain critical-point regression. The final Rust-supervised run completed in 6.45 seconds under the 120-second/8-GiB cap, with about 38 MiB RSS. The earlier bounded Möbius threshold factorization also completed, retaining signed scalar and multiplicities; its mixed factors do not supply a sign certificate.

## A checked stratification of the unbounded chart branch

The critical point need not be handled by an unrestricted cubic discriminant. On the branch `A4>=0`, the first three coefficients of `C(h)=c0+c1*h+c2*h^2+c3*h^3` are strictly positive. The helper proves this with exact boundary identities. Set `b0=r+w+v+z`, `y0=v+z`, `A40=A4|t=0`, `B40=B4|t=0`, and

```text
U0=(r+v)(w+z)*(b0^2-m*w*(r+w))+r*w*b0^2,
V0=r*w*b0*(2*b0-m*w).

c0|t=0=y0*V0*B40,
c1|t=0=y0*(V0*A40+U0*B40),
c2|t=0=y0*U0*A40.
```

Symbolica reconstructs each identity and checks that `A40-A4`, `B40-A40`, `U0`, `V0`, and each `ci-ci|t=0` for `i=0,1,2` have strictly positive coefficients. Thus `A40>A4>=0` and all three coefficients are positive throughout this branch. Consequently `c3>=0` gives no positive critical `h`; `c3<0` gives exactly one simple positive critical `h`, by the intermediate value theorem and Descartes' bound with multiplicity. This includes the degree drop `c3=0` without introducing a spurious critical point.

The sign of `c3` itself has a smaller nested description. Its exact signed factorization is

```text
c3=t*(w+z)*(r+v)*(36r+31w+36t+36v+31z)*H3/324.
```

The 52-term factor `H3=q0+q1*r+q2*r^2+q3*r^3` satisfies

```text
q1=5*w^2*(v-z)^2+R1,
q0=w*v*K(t+v,w,z),
K(j,w,z)=-5*z^3+(8*j-5*w)*z^2+(31*j^2+26*w*j)*z
         +18*j^3+31*w*j^2+18*w^2*j.
```

The helper checks the square identity, and `R1`, `q2`, `q3` have respectively 22, 9 and 2 strictly positive monomials. Hence `H3` strictly increases with `r>0`. For fixed `j,w>0`, `K` has exactly one simple positive root in `z`: its coefficient signs in increasing degree are `+,+,arbitrary,-`. If `K>=0`, then `H3>0` for every `r>0`. If `K<0`, then `H3` has exactly one simple positive root in `r`, with negative sign below and positive sign above. These exact cubic thresholds give a stratification of the possible critical `h` values rather than incorrectly deleting them all.

A separate checked coefficient identity proves `c0>0` on the larger region `B4>0`. The remaining bounded branch `A4<0<B4` is still unresolved. This research chart mixes original coordinates; the nested roots are not yet a transported cylindrical decomposition in the original mandatory order. There is no new original-problem completion or production projection rule.

Evidence: `artifacts/research/a446-jacobian-cubic-01/stratification-v9.json`, SHA-256 `4d65e03b68e270396b8f7b2b1eaa51f1e777244f4074c5de1a7deade34472370`; process SHA-256 `58d40bb5e6b62eeddc33a8aab5ef76f13b33857ed30164dde60372aebe55d9e5`. Frozen helper SHA-256 `692fc77017ee807ec3cd23b2be43e00fb945a69a0a8637bb5262bcc21fa9ea29`; helper source SHA-256 `7bdb0e894ad757ba0ed256c4353df545d4458ea81bda6879221abe68add41dec`. Five tests pass, including a rejection when the purported positive square residual is negative. The exact worker completed in 0.49 seconds (0.53 seconds supervised wall time, about 21 MiB RSS), under the 120-second/8-GiB cap. This is algebraic research timing, not a solver benchmark.
