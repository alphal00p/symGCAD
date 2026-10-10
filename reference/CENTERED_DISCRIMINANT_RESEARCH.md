# Exact centering as a discriminant reduction

Research diagnostic, 2026-10-10. The captured A377 degree-32 operand does
**not** admit the tested polynomial center: exact division has a nonzero
remainder. No coordinate shift, discriminant replacement, domain restriction
or production change was made. A rational-center symmetry remains untested.

## Algebraic proposal and boundary obligations

For `p(s,t)=a(s)t^n+b(s)t^(n-1)+...`, the only translation `t=u+h(s)`
that removes the degree-`n-1` term is `h=-b/(n*a)`. If this is a small
polynomial, exact expansion and inverse substitution can check whether the
shifted polynomial has the form `Q(s,u^k)`. This is an operation-level
identity; it does not change the original CAD coordinate system.

In the even case, for `m=degree_u(Q)` and nonzero leading coefficient,

```text
Disc_u(Q(s,u^2)) = (-1)^m * 4^m * LC(Q) * Q(s,0) * Disc(Q)^2.
```

The identity follows by taking the product of differences between the roots
`+sqrt(r_i),-sqrt(r_i)` of each factor `u²-r_i`. Equivalently, use
`Disc(f*g)=Disc(f)*Disc(g)*Res(f,g)^2`,
`Disc(u²-r_i)=4*r_i` and
`Res(u²-r_i,u²-r_j)=(r_i-r_j)^2`, retaining the leading-coefficient
powers. Translation leaves the signed discriminant unchanged. A future
implementation must reconstruct the signed full result and retain **all**
factors, including `LC(Q)`, `Q(s,0)` and `Disc(Q)`. The square on the last
factor is not a license to omit its roots or an interior touching seam.

A rational center requires explicit denominator treatment; its poles cannot
silently narrow the original domain. This first helper declines it entirely.
Successful algebraic compression would still require an independently
verified lower-degree discriminant and a measured advantage before promotion.

## Bounded helper and exact result

`examples/profile_centered_discriminant.rs` reads a saved operation capture,
checks its operand-array SHA-256, and parses only bounded canonical signed
integer monomials. Bounds apply before general expression parsing: 1 MiB
expression, 5,000 terms, 2,048 coefficient bits, 16 declared axes, degree
256 and two active axes. The fiber degree is at most 64. A candidate center
must be polynomial, independent of the fiber axis, at most three terms,
degree two and rational height 1,024.

The helper checks division reconstruction before accepting or declining a
center. If accepted, it expands `p(t+h)`, verifies the removed penultimate
coefficient and exact inverse `p_shifted(t-h)=p`, then computes the gcd of
fiber exponents and reconstructs any `Q(t^k)` proposal. Shift contributions
are bounded, with a 20,000-term output cap. A fresh-process supervisor caps
the complete operation at 60 seconds and 4 GiB; the diagnostic cannot
write a solver checkpoint. Killed or failed workers are not successes.

The frozen A377 operand has degrees `(s,t)=(84,32)`, 2,502 terms and maximum
rational height 692 (691 numerator bits plus the denominator-one bit).
The two top coefficients have degrees 60 and 61 in `s`. Exact division of
`-b` by `32*a` produced:

| Quantity | Terms | Degree in `s` | Maximum rational height |
|---|---:|---:|---:|
| Quotient | 2 | 1 | 328 |
| **Nonzero remainder** | **60** | **59** | **952** |

Thus no polynomial translation can remove the penultimate coefficient,
and the proposed polynomial-center power compression is inapplicable to
this operand. The worker completed in 0.082430252 seconds, 7,880 KiB peak
RSS; exact quotient/reconstruction took 0.001426514 seconds. This is a
small diagnostic, not a solver speed comparison or a whole-case result.

Evidence under `artifacts/research/a377-centered-discriminant-01/`:

* `report.json`, SHA-256
  `bdb523d1714d33aa796a5b93ca41d981790456568fe8a4bb4c6f2268f3efda70`.
* Frozen helper SHA-256
  `403c632aa3395180c3d32c056ac3b15a94c9cef764fbdb641cd3170423616c6d`;
  tested source SHA-256
  `8e853913d3122c046e3072afb97218177eb60b71267e87388ba364a2d7ee3243`.
* Input `artifacts/research/a377-warm03-progress-01/capture.json`, SHA-256
  `bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75`;
  operand-array hash
  `99f94af85a41a5dd07ae6f18f51b78b7b041d3c5ed47007e8179107f320b2de7`.

Three focused tests passed: translated-even reconstruction, asymmetric and
non-polynomial declines, and restricted-parser caps/signed round trips.
Build/test commands used `nix develop -c taskset -c 14,15`, `cargo -j2`, a
4-GiB virtual-memory limit and 300-second external deadlines; logs preserve
two initial compile-only type fixes and the final green `tests-03.log` and
`build.log`. The actual probe used CPU12 with its own 60-second/4-GiB
supervisor and a 75-second external deadline. No active campaign/cache was
modified.

## Deferred rational-center obstruction

There is a cheaper necessary test than expanding a dense rational shift.
Let `a,b,c,d` be the top four coefficients of an even degree-`n` polynomial.
The centered coefficient of `u^(n-3)`, multiplied by `3*n²*a²`, is

```text
E = 3*n²*a²*d - 3*n*(n-2)*a*b*c + (n-1)*(n-2)*b³.
```

This follows directly by expanding only those four leading terms. A
nonzero `E` rules out an identically even centered polynomial over `Q(s)`;
`E=0` is only necessary and does not certify evenness. Since every
nontrivial divisor of 32 is even, it would also obstruct any degree-32
power compression after a translation. A nonzero polynomial may vanish
at special parameter values, whose fibers remain untouched by this
operation-level negative result.

This additional check is proposed only, not part of the tested helper or
the measured result above. An unbuilt draft is retained under the ignored
diagnostic directory for a later coordinated research build. No claim
about rational symmetry follows from the polynomial-division failure.
