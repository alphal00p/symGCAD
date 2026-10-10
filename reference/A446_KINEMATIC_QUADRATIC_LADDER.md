# Exact kinematic partition and quadratic ladder for symbolic A446/Ax446

This is a research reduction, not a completed GCAD or an original-corpus completion. It keeps the original `s,t` outer group, `s>4`, unrestricted original `t`, and every positive Feynman coordinate. The new helper `examples/a446_kinematic_partition.rs` checks all polynomial identities below against both unchanged original fixtures using Symbolica exact arithmetic. The mass is `m=5/18`.

Write `u=t` for Ax446 and `u=5/9-s-t` for A446. The latter is an invertible triangular change of the second kinematic coordinate, reversing its orientation for each fixed `s`. Transport to an original-order proof would need to check that inverse and reverse the corresponding interval endpoints; the research identities alone do not provide this transport.

Define

```text
a=x1+x2+x3, b=x4+x5+x7, l=x1+x3, r=x5+x7,
X=x1+x5, Y=x3+x7, z=x8,
U=ab+(a+b)z,
T=b*x1*x3+a*x5*x7+z*X*Y,
Z=b*x2*x3+a*x4*x7+z*(x2+x4)*Y.
F=(a+b)U-s*x2*x4*z-u*T-m*Z = A*z+B.
```

The original normalized split equals `18F`, including the exact sign and scale.

## A positive endpoint on a larger explicit region

For `u<=4`,

```text
B = b*((x1-x3)^2+(4-u)*x1*x3+x2^2+2*x1*x2+(2-m)*x2*x3)
  + a*((x5-x7)^2+(4-u)*x5*x7+x4^2+2*x4*x5+(2-m)*x4*x7) > 0.
```

Strictness follows from positive coordinates; no physical restriction on `s` or an inferred Landau exclusion is used.

In fact, for `u>4` the condition

```text
C0=(X+Y)^2-u*X*Y >= 0
```

also suffices. Write `B=B0+x2*K2+x4*K4+R`, where `B0=B|x2=x4=0`, `K2` and `K4` are the corresponding first coefficients. Symbolica verifies

```text
(l+r)*B0 = l*r*C0 + u*(x1*x7-x3*x5)^2,
X*Y*K2 - C0*x5*x7  has strictly positive coefficients,
X*Y*K4 - C0*x1*x3  has strictly positive coefficients,
R                  has strictly positive coefficients.
```

Every monomial in the last three nonzero polynomials is positive on the original orthant. Thus `B>0` whenever `u>4,C0>=0` too. For `u<=4`, `C0>0` already follows from the positive coordinates (except a possible equality at `u=4,X=Y`, which does not affect the first direct proof).

## Five sign cells over each admissible base sector

Let

```text
W=2X+(2-m)Y > 0,
b2=W-(s-2)*x4,
c2=x4^2+W*x4+C0 > 0,
A=x2^2+b2*x2+c2,
D=disc_x2(A)
 = s*(s-4)*x4^2-2*s*W*x4+W^2-4*C0.
```

On `s>4,C0>=0`,

```text
disc_x4(D)=16*s*(W^2+(s-4)*C0)>0,
(s-2)^2*D(W/(s-2))
 = -4*((s-1)*W^2+(s-2)^2*C0)<0.
```

Hence the larger real root `rho` of `D` is positive and exceeds `W/(s-2)`. For `0<x4<rho`, either `b2>0` or `D<0`, so `A>0` on the whole positive `x2` axis. For `x4>rho`, `b2<0,D>0,c2>0`, so `A` has exactly two distinct positive roots `r0<r1`.

This gives the following generic sign decomposition, with `x1,x3,x5,x7` in the base and all omitted coordinate ranges positive:

| Cell | x4 | x2 | x8 | sign F |
|---|---|---|---|---|
| 1 | `(0,rho)` | `(0,infinity)` | `(0,infinity)` | + |
| 2 | `(rho,infinity)` | `(0,r0)` | `(0,infinity)` | + |
| 3 | `(rho,infinity)` | `(r0,r1)` | `(0,-B/A)` | + |
| 4 | `(rho,infinity)` | `(r0,r1)` | `(-B/A,infinity)` | - |
| 5 | `(rho,infinity)` | `(r1,infinity)` | `(0,infinity)` | + |

The order `[s,t,x1,x3,x5,x7,x4,x2,x8]` respects the original groups. A positive homogeneous scale can be retained at an early physical coordinate. All displayed root boundaries are simple on their parent cells: `rho` has real ordinal1, the two roots of `A` have real ordinals0/1, and the linear root `-B/A` has real ordinal0. Auxiliary omitted sections are explicitly contained in the zero sets of `D` or `A`, in addition to the true `F=0` sign boundary. A typed wrapper would need to replay those identities, the base-domain certificates, every interval and sign branch, and exhaustive output order.

For `u<4` this base is itself one kinematic region times the positive orthant, giving five symbolic generic cells as a mathematical construction. For `u>4,C0>0`, a prior cylindrical decomposition of `C0>0` is required; five cells are restored *per base sector*, not five for the entire complement. The kinematic `u=4` section and `C0=0` sections must be explicitly handled or recorded as exceptional sets for any generic partition wrapper.

The `u<4` half-plane is `t_A>-s-31/9` for A446 and `t_Ax<4` for Ax446. It strictly enlarges the previously explored `u<0` half-domain.

## What remains on u>4,C0<0

A kinematic-only partition cannot make `B` have one sign on the complete positive Feynman domain. This is an exact obstruction for every `u>4`, not merely a numerical counterexample.

Put `delta=u-4>0`, `d=10(1+delta)`. At the positive point

```text
x1=x3=x5=x7=d, x2=x4=delta,
```

homogeneity and exact substitution give

```text
B=-2*delta*(20+21*delta)
    *(100*delta^2+(1456/9)*delta+565/9) < 0.
```

At `x1=2u+1` and the remaining five endpoint coordinates equal to1,

```text
B=4*u^2+(427/9)*u+157/3 > 0.
```

Both signs also occur while **keeping `C0<0`**: the negative family has `X=Y=2d`, and the positive choice `x1=x3=x5=x7=1`, `x2=x4=u+1` gives

```text
C0=16-4u<0,
B=2*(u+3)*((u+3)^2-u-(5/18)*(u+1))>0.
```

The final positivity is checked by shifting `u=4+delta` and inspecting strictly positive rational coefficients. Consequently the unresolved part needs a Feynman-coordinate sign partition, another certified representation, or a different elimination strategy. This does not rule out a GCAD.

For this part `C0` itself has a simple ratio cone:

```text
(u-2-sqrt(u*(u-4)))/2 < Y/X
                           < (u-2+sqrt(u*(u-4)))/2.
```

It is compatible with the original physical order because `Y=x3+x7` is affine in the later coordinate `x7`. Endpoint comparisons against `x7=0` still need explicit base cuts.

## A lower-degree pair-projection identity

There is a useful exact cancellation before projecting `A` and `B` in `x2`:

```text
B-b*A = J*x2+K,
J=-b^2-u*x5*x7+m*r*x7+s*b*x4,
K=-b^2*(l+b)
  +u*(b*(x1*x7+x3*x5)+(b-l)*x5*x7)
  +m*(b*x4*x3+(b-l)*x4*x7),
Res_x2(A,B)=K^2-b2*J*K+c2*J^2.
```

Moreover,

```text
disc_s(Res_x2(A,B))=x4^2*K^2*disc_x2(B).
```

The helper independently checks the resultant and both discriminants with the production exact Symbolica kernels. This relation is evidence of a quadratic projection structure; it does **not** authorize eliminating outer `s` early or omitting any original-order projection obligation. Factorization measurements and artifact hashes are recorded below once the bounded run completes.

## Reproducible bounded evidence

The final diagnostic ran on CPU9 with a hard120s wall limit and2GiB memory cap through the Rust supervisor. It completed in0.08308s wall,12,900KiB peak RSS. Exact original/partition/resultant identities took0.04760s; checked factorization of the394-term resultant took0.01528s and returned one nonconstant canonical factor. Thus this initial pair resultant does not expose a separate kinematic-only Landau factor in the tested factorization. `disc_x2(B)` has53terms. These are debug research operand timings, not a whole-solver speed ratio or a completion benchmark.

Artifacts under `artifacts/research/a446-kinematic-partition-01/`:

- `report.json`: SHA256 `9adaac7df7cde50989684002f8c5a93e225edfc95885c91f8f48f1cb426506b8`.
- `report.process.json`: SHA256 `224dea72bd1f5be8ee7be41df4daab79b5d7baaf4ea6731e0a895c5cd330c363`.
- Frozen helper: SHA256 `e1c5498f37d03398951d5c14367466477efee8b59f2a418902d21b2556c50bc7`.
- Helper source: SHA256 `3206fdbd158bb1dd20f4b5056e54579d9f76f842fa6cfec0cada2606774add05`.
- Two helper tests passed, including both original identities and strict complementary families at rational `delta=1/1000,1,1000`: `artifacts/a446-partition-helper-tests-06.log`.

A concrete next extension is a typed positive-endpoint quadratic-ladder wrapper: replay the displayed identities and positivity conditions, build the five exact sectors, and compose it with an explicitly verified outer kinematic partition. The remaining `u>4,C0<0` region must still be solved and combined with matching original-order boundaries before either original problem receives completion credit. The low-degree `A,B` cancellation is a candidate starting projection basis for that remaining region, rather than a justification for omitting it.
