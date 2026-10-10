# Exact root geometry in the symbolic A446/Ax446 complement

This is bounded research on the unchanged symbolic problems, not a GCAD completion. It continues [the quadratic-ladder reduction](A446_KINEMATIC_QUADRATIC_LADDER.md). All identities and root counts below use Symbolica exact arithmetic in `examples/a446_complement_probe.rs`; the helper checks the saved input hash, independently reconstructs the original `A,B` specialization and their resultant, and uses exact Sturm counts.

The original domain remains `s>4`, unrestricted original `t`, and every Feynman coordinate positive. Write `u=t` for Ax446 and `u=5/9-s-t` for A446. The unresolved part is `u>4,C0<0`, where `C0=(x1+x5+x3+x7)^2-u*(x1+x5)*(x3+x7)`. No original-domain completion follows from solving only its complement.

## What the sextic requires

For `F=A*x8+B`, the preceding report gives

```text
A=x2^2+b2*x2+c2,
B-b*A=J*x2+K,
R=Res_x2(A,B)=K^2-b2*J*K+c2*J^2.
```

The exact `R` has394terms and degree6 in `x4`, with leading coefficient `s>0`. Its endpoint at `x4=0` has78terms and factors into two cubics. A full symbolic `disc_x4(R)` attempt exceeded its120s wall limit (133,084KiB peak RSS); no completed discriminant or factorization is claimed for that run.

`R` cannot be uniformly nonzero on the strict complement. Set `x1=x3=x5=x7=1`, put `v=x2+x4>0`, and define

```text
N=(s+4)*(v+4)-20/9,
w=x2*x4=v^2*(v+31/9)/N,
u=((v^2+(67/9)*v+16)*N-s*v^2*(v+31/9))/(4*N).
```

Exact substitution gives `A=B=0`, while

```text
v^2-4*w=s*v^2*(v+4)/N>0.
```

Thus `x2,x4` are distinct positive roots of their sum/product quadratic. For fixed `s>4`, this `u(v)` increases continuously from4 to infinity; its derivative numerator has strictly positive coefficients after `s=4+sigma`. Consequently every `s>4,u>4` admits positive common-root points of `A,B` on this symmetric outer-coordinate slice.

The unrestricted sextic `R` does not have a constant positive-root count throughout the complement. At

```text
s=5, u=104, x1=1/1000, x5=x7=1,
```

it has three positive and three negative roots when `x3=5`, and one positive and three negative roots when `x3=10`. Both `C0` and `R(0)` are convex quadratics in `x3`, strictly negative at both endpoints; hence they stay negative on the whole interval `[5,10]`. The leading coefficient stays5. The lost positive `x4` pair cannot disappear through zero or infinity. However, a positive `x4` resultant root need not have a positive common `x2` root.

On this exact bivariate slice, `disc_x4(R)` has degree18 in `x3` and19terms. Checked factorization returns a quadratic and a degree14 factor. The quadratic

```text
15967801357931+45180717164500*x3+31766939750000*x3^2
```

is positive for `x3>0`; the degree14 factor has **exactly one root in `(5,10)`**. Its full exact coefficients and the discriminant reconstruction are retained in `report-v3.json`. This is a nonquadratic cut for unrestricted real-root projection, but it is **not yet a demonstrated necessary cut for the positive orthant**.

The follow-up `report-v4.json` classifies the common coordinate `x2=-K/J` exactly. At `x3=5`, the three positive `x4` roots have common `x2` signs `[+,-,-]`; the sole positive root at `x3=10` has common `x2` sign `+`. Each sign is proved by isolating the `R` root, ruling out `J,K` zeros throughout its rational interval with exact Sturm counts, and evaluating their signs there. Thus the disappearing pair is associated at the sampled endpoint with **negative** common `x2`; attributing a necessary positive-domain cut solely to those sextic roots would be incorrect. The collision itself and possible positive critical points require a separate Jacobian or filtered-root proof.

An additional rational collision occurs at `s=103/21,u=395/63,x1=x3=x5=x7=x4=1`, with `C0=-572/63`. Here

```text
A=x2^2+(286/63)*x2-40/63,
B=3*A,
```

and `x4=1` is a double root of `R`, shared by one positive and one negative `x2` root. This does not prove a collision of two positive solution branches. The diagnostic checked268 strict-complement rational fibers, finding0,1,2, or3 distinct positive `x4` roots. Filtering by positive common `x2` gives0,1, or2 intersections:88 fibers have0,40 have1,139 have2, and one has an additional `J=K=0` event requiring separate treatment. These samples are not a universal classification; changes can arise from the original coordinate endpoints as well as interior collisions.

## A concrete order experiment on the full problems

The earlier safe-region construction can use original order

```text
[s,t,x1,x5,x3,x7,x4,x2,x8]
```

with the positive homogeneous scale `x1`. This keeps `x2` available for its quadratic role, unlike the earlier `x2` anchor. After the final linear `x8` and quadratic `x2` projections, the remaining problem has six variables `[s,u,x5,x3,x7,x4]`; the `u` notation here is an algebraic description, not a change to the prepared original inputs.

For `u>4`, write `r±=(u-2±sqrt(u*(u-4)))*(x1+x5)/2`. The condition `C0>0` has four cylindrical sectors in `(x3,x7)`: two when `x3<r−`, one when `r−<x3<r+`, and the full positive `x7` ray when `x3>r+`. Thus the safe-region mathematical construction has25 generic sign cells (five for `u<4`, and five over each of those four sectors), subject to explicitly accounting for auxiliary seams. The unresolved `C0<0` part has two corresponding base sectors and the sextic obstruction above.

Prepared full-original configurations under `artifacts/hard-campaigns/a446-quadratic-order-release-01/configs/` preserve the complete original mathematical TOML prefix byte-for-byte. They introduce no kinematic restriction or extra inequality and preserve the original variable groups. They test the existing local engine, not a new piece-only wrapper. Their outcomes are recorded separately from these algebraic results.

Both original trials remained incomplete. With release `dda5980`, separate fresh caches, CPU9 and8GiB limits, A446 hit the120s watchdog (768,876KiB peak RSS). Its last progress snapshot was at1.30s, with3,763 nodes and948 refinements; operand capture was disabled, so the operation consuming the remaining budget is unknown. Ax446 returned `incomplete: local projection refinement limit` after100,001 refinements and393,497 speculative nodes,25.8336s kernel/25.8992s wall (76,952KiB peak RSS). These are exploratory results, not paired parity timings. The order exposes manageable algebra for Ax but does not by itself remove its repeated local refinements.

The unchanged original prefixes, prepared configurations, actual worker inputs, process statistics and separate checkpoint directories are retained with `preparation.json`. Worker SHA256 is `f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`; the immutable Rust supervisor SHA256 is `08bda502f1a5ed580f03fadecd134b3af6e5963641be305e0ca05051fb29b325`. The A/Ax summary hashes are respectively `58289803e478bc1d60bcf4904cf6a2ebe54de93c5b4e591e5494aa0688a281b8` and `923f8e49f55cad807f66992a29b99e596b851286b1d9d3029b9c51af16dd7c3a`.

## Bounded replay evidence

The final helper run used CPU9, a hard120s wall limit and2GiB memory cap through the Rust supervisor. It completed in0.43449s wall with14,180KiB peak RSS. The bivariate slice discriminant took0.00336s, or0.01070s including checked factorization. These are debug research operand timings, not whole-solver speed ratios.

Artifacts under `artifacts/research/a446-complement-01/`:

- Final `report-v3.json`: SHA256 `93391a6718203d98fa2d7d5236a5780238c1df98f29f0a298416ac1880ef1ac1`.
- Final `report-v3.process.json`: SHA256 `b92feaaf9fbcfb33962b57c965e8cca79cf89c4c65bf5bce8fa89375d0e3a0c6`.
- Frozen `helper-v3`: SHA256 `5b29a7eddda4bfa83a0fdaab1b37c906e0e5b82f78fc8c925ebc6b3b16b07558`.
- Helper source: SHA256 `19f69d6a23946f976c900731e78b835e677d857f44107760aa3edd6073930c6e`.
- Full-discriminant timeout `report.process.json`: SHA256 `2508cfb9d1d7694017fca69c131073ba0b8a486c595cf7abfe89b431f1c2b540`; its partial report is `report.json`, SHA256 `2a980cb9f07c254851586491f5c091815ca8983d7977d6e8d8f1b3568d6f0cf7`.
- Two helper tests passed in `artifacts/a446-complement-helper-tests-04.log`.

The subsequent exact common-coordinate classifier passed three helper tests, including a fixture whose two positive resultant roots correspond to opposite common-coordinate signs and a `J=K=0` degeneracy. `report-v4.json` SHA256 is `309548b9be7781562f96cf9d3b75cb7d4cfaa53a2502e6cfab54a188864124b8`; process record SHA256 is `442e6376bceff9e285b8d5a39f1199e6da653106c1fc300df672d878a600a6b7`; frozen helper SHA256 is `e8779a0cb110260baac79a313f4a4366337b303645e194d4da773ee4c2ebb8b6`. It completed under the same120s/2GiB Rust supervisor on CPU10. Formatting after compilation changed only source layout; the formatted source SHA256 is `35fb0424339298df0eacaf9e8a5b4800d9b8de124a4820d1bdd40ab87d1b7952`.

Replay the final diagnostic with `a446_complement_probe --output REPORT --slice-discriminant`; adding `--discriminant` requests the separately bounded full symbolic attempt. All partial outputs remain explicitly research-only with no coverage claim.
