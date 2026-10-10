# Fixed-parent ratio diagnostic for the A446 common-root arc

This is exact algebraic research, not a projection omission, an original-coordinate chart wrapper, or a completed case. The universal Jacobian sign conjecture is false: the [later critical-point construction](A446_FILTERED_JACOBIAN_RESEARCH.md#a-positive-critical-point-refutation-of-universal-nonvanishing) supplies a zero Jacobian at positive common roots with `s>4,u>4,C0<0`. The older [filtered-projection audit](A446_FILTERED_PROJECTION_AUDIT.md) now points to that result explicitly.

The surviving question is how to classify the critical sections while preserving every original sign sheet. The existing cubic stratification fixes its chart variables `(r,w,t,v,z)` and varies `h`; this also changes original `x1,x3,s,u`. Its zero-or-one critical-root conclusion is therefore **not** a zero-or-one conclusion on a fixed original CAD parent. On a fixed original parent, its `v=x1/h-x5` and `z=x3/h-x7` vary with `h`. Substituting these rational functions into that conclusion without rechecking the hypotheses and polynomial would be invalid.

## A fixed-parent equation and its admissible branch

Use the notation of the [kinematic quadratic ladder](A446_KINEMATIC_QUADRATIC_LADDER.md):

```text
p=x1, q=x3, r=x5, w=x7, x=x2, y=x4, m=5/18,
l=p+q, R=r+w, Y=q+w,
a=l+x, b=R+y, h=a/b.
```

Keep `p,q,r,w,u` fixed. The invertible positive-domain reparameterization of the two remaining coordinates is

```text
x=h*b-l, y=b-R,
h>0, b>R, h*b>l.
```

It is a change of coordinates in the two-dimensional fiber. It is not triangular with either original `x,y` order: the new `h` depends on both. Its use below is an elimination diagnostic, not permission to export its cells in the original order.

The helper `examples/a446_ratio_critical.rs` checks the original identities

```text
B=b*G,
G=h*(1+h)*b^2-m*h*Y*b+m*l*q-u*p*q
  +h*(m*R*w-u*r*w),

A=N-s*D,
N=(1+h)^2*b^2-m*Y*((1+h)*b-l-R)-u*(p+r)*Y,
D=(h*b-l)*(b-R)=x*y.
```

The term `m*R*w` includes `m*w^2`; dropping it confuses the coordinate `r=x5` with the sum `R=x5+x7`. Exact reconstruction caught that potential notational error before any experiment used it.

On the admissible branch,

```text
G_b=h*(2*(a+b)-m*Y)>h*(2-m)*Y>0.
```

Here `a>q`, `b>w`, and `m<2`. Thus for each positive `h`, at most one root of `G` is admissible. If one exists, it is simple and is the larger real root of this upward quadratic. At a root, the discriminant equals `G_b^2>0`. Admissibility still requires both strict inequalities `b>R` and `h*b>l`; real-root existence alone is insufficient.

On an original fixed-`u` arc `B=0`, the previously checked derivative identities give `B_x,B_y>0`, hence `dx/dy=-B_y/B_x<0`. Therefore

```text
dh/dy=((dx/dy)*b-a)/b^2<0.
```

So `h` is a strict parameter along each such arc. This fact does not turn it into an original cylindrical coordinate.

## Exact critical equation

On `G=0`, set `s=N/D`, where `D>0`. Differentiating with `p,q,r,w,u` fixed gives

```text
C=(N_h*D-N*D_h)*G_b-(N_b*D-N*D_b)*G_h,
ds/dh=C/(D^2*G_b).
```

The helper separately differentiates the original `A,B` with respect to original `x,y`, before substitution. If `J=A_x*B_y-A_y*B_x`, it checks the full polynomial identity

```text
b*(D*J-C)=(N_h*D-N*D_h)*G,
```

where `s=N/D` is denominator-cleared on the left. Consequently `C=0` is exactly the original Jacobian condition on the admissible common-root locus, not a derivative obtained by varying the outer parameters accidentally.

Pseudo-division by the quadratic `G` yields an exactly replayed identity

```text
scale*C=quotient*G+(L*b+M),
```

with `scale` a power of `h*(1+h)>0`. For `G=alpha*b^2+beta*b+gamma`, the candidate critical equation in `h` is

```text
alpha*M^2-beta*L*M+gamma*L^2=0.
```

The helper checks this result against the independent production exact quadratic-linear resultant. It removes only explicitly reconstructed factors `h^5*(1+h)^6`, which are strictly positive on the stated domain. It does not infer that any other factor is irrelevant.

## What the measurement does and does not simplify

The exact identity and elimination run took 1.012 seconds of supervised wall time with 21,996 KiB peak RSS. It used one CPU, a 60-second hard deadline and an 8-GiB memory ceiling.

| Polynomial | Terms | Degree in `b` | Degree in `h` |
|---|---:|---:|---:|
| `G` | 10 | 2 | 2 |
| `N` | 18 | 2 | 2 |
| `C` | 234 | 5 | 4 |
| Pseudo-remainder | 702 | 1 | 10 |
| Critical resultant | 8,030 | 0 | 22 |
| After positive factors | 3,376 | 0 | 11 |

The remaining polynomial has degree five in `u` and coordinate degrees `[6,10,6,10]` in `[p,q,r,w]`. This is a finite, fixed-parent description of candidate critical points, but these sizes alone do not demonstrate a useful projection-cost reduction. A separate immutable-profiler run reconstructed the exact native factorization in 0.061 seconds of algebra time (0.184 seconds supervised wall time, 13,780 KiB RSS). The sum of factor term counts remained 3,376. The profiler does not export the individual factors, so this is not a claim of irreducibility. It supplies no measured reduction in the total symbolic inventory.

## Required filtering and original-order obligations

The candidate resultant includes both quadratic branches and points outside the positive fiber. A root can only be used after proving an admissible common `b`, `N-4D>0`, and the relevant `u,C0` conditions exactly. If `L!=0`, the only candidate is `b=-M/L`. If `L=0,M!=0`, there is no common root; if `L=M=0`, one must inspect the original quadratic and its admissible larger root. Dividing by `L` without that case split would lose genuine coincidences or invent roots. A zero resultant does not, by itself, certify positive-domain criticality. An identically zero specialized resultant requires a separate proof or ordinary fallback; it must not be discarded as an empty exceptional set.

Coordinate exits `b=R` and `h*b=l`, degree drops, and distinct eligible critical roots colliding remain separate obligations. The equation also parameterizes critical **points** before their values `s=N/D` are compared to the fixed original outer `s`. The required original group has both kinematic parameters before all physical coordinates. No current result supplies a finite simple critical-value inventory in that original order, or a transport theorem for this nontriangular ratio chart.

This explains why the current local engine retains its discriminant fallback. The upstream GCAD projection in `.reference/gcad/gcad/gcad.py::PR` uses leading coefficients, discriminants and pair resultants of a square-free basis. The local engine strengthens this with sample-local root-bearing classification and independently replayed positive-ray coefficient/interval witnesses. When such a witness fails or its permanent finite stock is exhausted, `positive_projection_raw` keeps the exact full discriminant; the new ratio identities do not discharge that obligation. The source behavior is conservative rather than an omission of a proved domain shortcut.

A useful next experiment would first demonstrate a small, completely filtered critical inventory over a fixed original parent and exact treatment of the `L=M=0` seam. Until then, no production rule, selector, stock increase, or original completion is proposed from this diagnostic.

## Reproduction and evidence

Two helper tests pass: independent original-equation/Jacobian reconstruction, and pseudo-division identity replay. Logs are `artifacts/a446-ratio-critical-tests-02.log` and `artifacts/a446-ratio-critical-build-01.log`. Evidence under `artifacts/research/a446-ratio-critical-01/`:

- `report.json`: SHA-256 `dc2e6ce0d58c8cccd3c8a110abcb971dac362fd2b170d765678d2a8b30a7613b`.
- `report.process.json`: SHA-256 `c9552bca3fcebbe45afa44725a246ed68f500d916cc7f55f4e284c907750b8be`.
- Frozen helper: SHA-256 `4b1db47fead6a314b9d8460f0e349e0d3eaf30d6b928aea917c53bc4c53ad122`.
- Helper source: SHA-256 `84260e156af2e53d2b85519c3604f41ebdb586321635b03e79d70293e2b1c0e4`.
- `factor-v2.json`: SHA-256 `cf41ab91cfd99b659d775020ebd597bce8ce741b96100014f16917e00f29167a`; its process sidecar has SHA-256 `7cb92a01a85906a600b437089f9dd13d7191dfb024f8d8907d2431ac44b614a2`. They retain the checked factor result, input hash, resource outcome and profiler hash. The earlier `factor.json` preserves a capture-schema failure (`Factor` instead of `factor`); it is an infrastructure attempt, not a failed factorization.

No existing case proof, original input or benchmark report was changed.
