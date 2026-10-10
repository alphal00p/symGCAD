# Symbolic A381/Ax381: a larger peelable piece and an exact obstruction

This is research evidence, not a completion of either unrestricted symbolic
original. The full fixed-kinematics completions are recorded separately in
`CONDITIONAL_QUADRATIC_PEEL.md`.

Use its notation `A=x0+x2`, `C=x4+x6`, `z=x8`, `m=5/18`, `k=2-m`, and define

```
gamma_A  = s+t+13/9
gamma_Ax = 2-t
delta = gamma-k.
```

The original polynomials differ only through this coefficient of `x3*x5*x8`.
In the first report, the x3-first certificate is immediate for `delta>=0`.
A different first axis extends the sufficient region to `gamma>=0`.

## x5 first, then x3

Writing the full polynomial as `F=a5*x5²+b5*x5+c5`, let

```
a5 = A+x3+z                 h5 = C+z
b5 = b2+x3²+(2*C+k*A+gamma*z)*x3
h5*b5-c5 = R2 + x3*(C²+(m+gamma)*C*z+gamma*z²+s*x4*x6),
```

where `b2,R2` are the exact endpoint expressions in the first report. Both
full symbolic identities were checked with Symbolica against the original
A381/Ax381 fixtures, with no zero or unit coordinate substitutions.
For `s>4`, positive Feynman coordinates and `gamma>=0`, the right side is
strictly positive. Thus the same conditional quadratic rule peels x5 first.
At the resulting endpoint x5=0, the earlier x3 certificate loses its delta
term and is strict for all s>4. The final endpoint is the same polynomial
linear in x8 as in the fixed-case reduction.

On the open gamma-positive chart, the existing certificate portfolio proves
and independently replays `a5>0`, `h5>0`, and the 31-term residual. The latter
took 0.000562 seconds in the recorded debug research call. A compatible
lifting-order suffix is `[x3,x5]`; the elimination order is x5 then x3.
Scale fixing x8 keeps the first input within the production discovery's
8-active-variable bound. This is a possible piece trial, not a recorded solve.

The original kinematic pieces are

* A381: `s>4` and `t>-s-13/9`;
* Ax381: `s>4` and `t<2`.

The A piece is not a coordinate rectangle in the original variables. For the
research certificate we used the invertible triangular coordinates
`t_A=gamma-s-13/9` or `t_Ax=2-gamma`. The first variable remains s. A production
piece transport would need exact identity and inverse-boundary evidence,
including reversal of the t-axis for Ax. These affine charts preserve the
kinematic cylindrical variable order, but no transport wrapper is currently
claimed by this report. The seam gamma=0 can be recorded as an explicit generic
exception; an exact nongeneric decomposition would need to handle it too.

## The remaining region has a real two-root obstruction

The opposite charts use a positive variable w with gamma=-w:

```
t_A  = -w-s-13/9
t_Ax = 2+w.
```

Symbolica confirms exact equality of the two transformed full polynomials on
both gamma-sign charts. Consequently crossing between A and Ax preserves the
same unresolved negative-gamma polynomial; it does not turn this region into
the positive-gamma case.

At `s=10,w=20`, corresponding to original `t_A=-283/9`, `t_Ax=22`, each of the
six quadratic Feynman axes has an exact positive-coordinate fiber with
coefficient signs `(c,b,a)=(+,-,+)` and **two distinct positive roots**. Both
exact Sturm counting and exact root isolation return 2 in every row:

| Quadratic axis | Other coordinates |
| --- | --- |
| x0 | x2=1; all remaining coordinates=1/1000 |
| x2 | x0=1; all remaining coordinates=1/1000 |
| x4 | x6=1; all remaining coordinates=1/1000 |
| x6 | x4=1; all remaining coordinates=1/1000 |
| x3 | x5=1/10, x8=1; x0=x2=x4=x6=1/1000 |
| x5 | x3=1/10, x8=1; x0=x2=x4=x6=1/1000 |

These are different fibers, all in the same original kinematic slice. A
global endpoint-only peel cannot hold for any quadratic axis on that entire
slice, regardless of the polynomial or rational positive multiplier chosen:
at these points c>0 and b<0 already imply `h*b-c<0` for every h>0. They also
rule out a globally negative-endpoint certificate. The linear x8 coefficient
changes sign: it is negative when the other six coordinates all equal 1 and
positive when only x3 is changed to 100. Permuting these coordinate axes does
not remove these obstructions. This does not rule out other coordinate
changes or finer Feynman-coordinate partitions.

## What a complete piece strategy would still require

A genuine convex-quadratic reconstruction on the negative-gamma piece must
retain the two-positive-root case. For an innermost quadratic axis with a>0,
a common certified base CAD for c, b and `D=b²-4*a*c` supports:

* c<0: one positive simple root, full-real ordinal 1;
* c>0 and b>=0: no positive roots;
* c>0, b<0, D<0: no real roots;
* c>0, b<0, D>0: two positive simple roots, full-real ordinals 0 and 1.

Zero/identically-zero cases must follow explicit exact rules. This case
classification is already present in the local positive-ray machinery. A
separate whole-fiber wrapper could change scheduling and remove repeated
work, but it supplies no new justification for dropping D on the last case.
Its base typically contains a quartic D, so the endpoint-only chain no longer
continues automatically.

Combining independently solved pieces also needs a common decomposition of
their shared outer prefixes. The gamma sign boundary is cylindrical over s,
but two solvers may split the s-axis differently. Merely concatenating their
cells would not establish one cylindrical output. A wrapper must prove the
piece coverage/disjointness, affine transport, common outer refinement, and
all restored branch boundaries. The current evidence neither completes that
construction nor claims an original symbolic completion.

## Reproduction evidence

All artifacts are in `artifacts/research/conditional-symbolic-partition-01/`.

* `a381-x5-first-identity.json`, `ax381-x5-first-identity.json`: exact full
  original identities, empty substitution lists.
* `{positive,negative}-gamma-chart-identity.json`: exact A/Ax equality after
  the explicitly saved affine coordinate substitutions.
* `positive-gamma-certificates.json`: strict a5, h5 and residual certificates,
  each independently replayed against the stated chart constraints. Capture
  SHA256 `84a9592e7b84c05a685943e1d60eb3af6d826c80a4831937ab4c520ae18d9d66`.
* `two-positive-{x0,x2,x3,x4,x5,x6}.json`: exact coefficient signs, root
  intervals and independent positive-root counts. Each file records its full
  variable map and rational prefix; internal t denotes w=-gamma here.
* `linear-x8-{negative,positive}.json`: opposite exact leading-coefficient
  signs on the two stated fibers.

These existing Symbolica helpers ran on CPU9 with a 2GiB virtual-memory cap
and 10/15-second shell deadlines; no new production code or Cargo build was
used. The `profile_operation` binary SHA256 is
`5ac1762e814fb2f81bbe8fc67bdd6a3d2bb40ebc5e404163870922b67a94f0b7`;
the other two helper hashes are recorded in the first report. Positive-root
diagnostics took roughly 0.025–0.033 seconds per fiber. These are diagnostic
costs, not whole-solver timing claims.
