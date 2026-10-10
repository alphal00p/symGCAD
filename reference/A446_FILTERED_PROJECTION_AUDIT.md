# Audit of a proposed positive-common-root projection rule for A446

This is a historical conditional theorem and implementation audit, not an implemented projection rule or a completion claim. It concerns the `u>4,C0<0` region in [the exact quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md), with `x=x2`, `y=x4`, and `F=A*x8+B`. The full original problem also contains the complementary kinematic regions.

**Superseded sign conjecture:** the later [exact Jacobian research](A446_FILTERED_JACOBIAN_RESEARCH.md#a-positive-critical-point-refutation-of-universal-nonvanishing) proves a positive common root with a zero Jacobian strictly inside `s>4,u>4,C0<0`. Thus neither universal Jacobian nonvanishing nor universal `-Q*Jacobian>0` is available. The 1,303-point screen below is retained as historical evidence, not a reason to resume that false universal certificate search. The later report instead proves a [zero-or-one-critical-root stratification on one chart branch](A446_FILTERED_JACOBIAN_RESEARCH.md#a-checked-stratification-of-the-unbounded-chart-branch); its bounded branch and original-coordinate export remain unresolved. Ordinary projection obligations must remain unless a proof explicitly accounts for those critical sections.

## What the current evidence proves

Using the notation of the quadratic-ladder report, put

```text
a=x1+x3+x, b=x5+x7+y, L=a+b,
T=b*x1*x3+a*x5*x7,
E=L*a*b-(5/18)*(b*x*x3+a*y*x7),
B=E-u*T,
Q=b*x3-a*x7,
V=L*(b*x1-a*x5)+(5/18)*(x3+x7)*(x5*x-x1*y).
```

All six physical coordinates are strictly positive. The exact identities in `examples/a446_filtered_jacobian.rs` prove that `T>0`, both numerators of the coordinate derivatives of `u=E/T` have strictly positive coefficients, and

```text
s=Q*V/(x*y*T)
```

at a common positive zero of `A,B`. Along a fixed-`u` arc, `f=Q/T` strictly decreases. With `g=V/(x*y)`, the remaining proposed premise is that

```text
M=y*(x*V_x-V)*(E_y*T-E*T_y)
 -x*(y*V_y-V)*(E_x*T-E*T_x)
```

is strictly negative, so that `g` also decreases. If this premise holds, `s=f*g>0` makes `f,g` have the same nonzero sign. Therefore the derivative of `s` along the arc cannot vanish: it is negative when both factors are positive and positive when both are negative. Assuming both factors positive would leave half of this argument unchecked. The helper verifies the denominator-cleared identity relating this derivative to the original Jacobian `A_x*B_y-A_y*B_x`.

The initial coefficient diagnostic was inconclusive: `M` has 430 terms, with 132 positive and 298 negative coefficients; the bounded strict-sign certificate attempt for `-M` returned no certificate, and checked factorization returned one mixed-sign canonical factor. A subsequent exact rational screen **disproves the proposed premise**, including on the required region. At

```text
(x1,x3,x5,x7,x2,x4)=(3,100,1/10,10,1,1/3),
s=9211/1260>4, u=3978697/103950>4,
M=917488163954131/81000>0.
```

The common-root parametrization gives `A=B=0` at this point. Strict monotonicity of `E/T` implies `B(0,0)<0`, hence the displayed endpoint identity also proves `C0<0`. Thus restricting to the intended complement does not repair the monotonicity argument. The cleared Jacobian is still negative here, `-3952165941293647/8100`, while `Q=10/3>0`. All 1,303 positive-`s` sampled points had `-Q*Jacobian>0`; this finite observation is not a universal certificate, and the later exact critical-point construction supersedes its suggested universal direction.

Evidence under `artifacts/research/a446-filtered-jacobian-01/`:

- `report.json`: SHA-256 `1eb7abc071c3bbaebb7d75ea9d8dfa05d9b84ac3cf1add8a886bf892ba7234f3`.
- `factor.json`: SHA-256 `8a08c54e07a9936df5862ba41a6b37db6d0e77bea049a0ce0269d95dce077dbf`.
- Frozen helper: SHA-256 `28a47afd8cf65ed64922167778c43c97519cf66003b319949880c2d52d902e88`.
- Follow-up exact screen `sample-v2.json`: SHA-256 `07f7ffd2f1286d08bfcce2e009129a08c19e550b921f843dec9ac81b6c6efeeb`. Its counterexample and sample inventory supersede only the inconclusive sign conclusion of the initial diagnostic.

## Arc existence is a separate condition

Strict coordinate monotonicity implies at most one positive `x` on `B=0` for each positive `y`. A nonempty arc exists precisely when `B(0,0)<0`, equivalently

```text
u > (l+r)*l*r/(r*x1*x3+l*x5*x7),
l=x1+x3, r=x5+x7.
```

It is then one decreasing smooth arc between the positive coordinate axes. Coercivity of `E/T` on each axis makes both endpoints finite. At equality or below, `B>0` throughout the open positive quadrant. In particular, `C0<0` alone does not establish an arc, since

```text
(l+r)*B(0,0)=l*r*C0+u*(x1*x7-x3*x5)^2.
```

An explicit arc-based decomposition must distinguish these cases. A narrower theorem about common positive roots need not add `B(0,0)` as a cut merely for arc birth: at `B(0,0)=0,C0<0`, the other polynomial has `A(0,0)=C0!=0`, so the origin is not a common root.

## Conditions sufficient for filtered delineability

On a connected open base cell, a replacement for the unrestricted self-discriminant would need all of the following exact obligations:

1. The original domain and polynomial identities above hold, including positive coordinates, `s>4`, the appropriate kinematic sector and the mass value. A proved nonzero Jacobian is required at **every** positive common root, not merely at the sampled ones.
2. Common roots cannot cross either coordinate endpoint. Conservative sufficient base guards are `Res_x(A(x,0),B(x,0))=R(0)` and `Res_y(A(0,y),B(0,y))`, both nonzero. The second cannot be replaced by `R(0)`: it controls positive `y` intersections escaping through `x=0`. An identically zero proposed guard requires a separate rule or fallback, not an empty exceptional-set declaration.
3. Common roots cannot escape to infinity inside a base cell. Here the exact identities `lc_y(R)=s>4` and `lc_x(A)=1` provide local uniform Cauchy bounds over compact base subsets. They supply properness without an additional speculative infinity guard.
4. Distinct positive-common branches cannot share one `y`: strict positivity of `B_x` at a positive `B` root gives uniqueness of its positive `x`. Together with the nonzero Jacobian, endpoint exclusion and properness, this gives a finite covering of the base. Ordering the eligible `y` values yields continuous, distinct sections with constant inventory.

These arguments establish a filtered root family; they do not establish simplicity or invariant ordinal positions among **all** roots of `R`. The existing exact `J=K=0` example in [the resultant geometry report](A446_COMPLEMENT_ROOT_GEOMETRY.md) has a double resultant root shared by one positive and one negative `x`. It is compatible with a regular positive-common branch.

## Root selection and output require a new proof type

Existing `Real`, `Positive` and `RealDescending` indices select among roots of a single polynomial. They cannot encode the proposed family: irrelevant positive `y` roots with negative common `x` can appear, disappear or coincide with an eligible branch. The already observed degree-14 collision concerns such an irrelevant pair.

A future selector would have to bind `R`, the original `A,B`, the eliminated `x` axis, the complete variable map and an ordinal among roots with a **positive common `x`**. Exact eligibility at an isolated `R` root follows from

```text
B-b*A=J*x+K,
R=K^2-b2*J*K+c2*J^2,
A=x^2+b2*x+c2.
```

If `J!=0`, the common coordinate is `-K/J`, and eligibility requires `J*K<0`. If `J=0,K!=0`, it is not a resultant root. If `J=K=0`, `B=b*A` and a separate exact positive-root existence test for the quadratic is essential. Evaluations and zero tests must be exact at the algebraic `y` root; floating-point signs or the sign at a nearby rational sample are insufficient. The proven monotonicity of `B` limits this exceptional case to at most one positive common root.

The producer, independent verifier, common-refinement comparison, numerical export and any homogeneous or affine transport must all understand this selector. Ordinary CAD checkers must reject it unless a dedicated wrapper replays its hypotheses and exhaustive sections. A change confined to projection-factor omission would be unsound. No current output schema or proof is being reinterpreted by this audit.

## Obligations the proposed rule does not remove

The theorem would replace only the self-discriminant needed to delineate the filtered `R` sections. Individual positive-root behavior of `A,B`, coordinate endpoints and the relative order of these sections with other `y` cuts still need proofs. A nonzero common-root Jacobian alone does not rule out `A_x=0`; nor does it rule out a different root of `A` crossing zero. Thus existing discriminant, endpoint and pair-order obligations must remain unless another exact identity discharges each one. Some individual crossings may be impossible by monotonicity, but they need their own checked argument.

After lifting `x2`, the final linear `x8` fiber still requires the signs of `A` and `B` and its positive-root existence. Every retained open sector must be exported, and every omitted section must be in the stated generic exceptional set. The `u=4` and `C0=0` seams and the complementary kinematic regions cannot disappear from an original-problem coverage proof. For A446, `u=5/9-s-t` additionally requires checked triangular coordinate transport with reversed second-coordinate interval endpoints; Ax446 uses `u=t`. Original groups and any fixed order remain mandatory.

A minimal regression set for any future implementation includes the irrelevant-root collision, the `J=K=0` double-resultant example, arc absence and birth, both coordinate escapes, malformed selector axes/ordinals/eligibility, non-strict Jacobian evidence, required cross-order cuts, exact comparison with tractable ordinary CAD specializations, and resource exits that remain incomplete. Independent replay must use recorded polynomial identities and certificates, never producer history or sample counts as a universal proof.
