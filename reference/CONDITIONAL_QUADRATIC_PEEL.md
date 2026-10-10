# Conditional quadratic endpoint peeling

This exact rule extends the `monotone` engine's positive-ray fiber theorem. It
does not change the baseline projection engine and does not assert that the
polynomial is increasing everywhere.

For `q(z)=a*z²+b*z+c` on the complete remaining-coordinate domain, certify
`a>0`, a multiplier `h>0` independent of `z`, and `h*b-c>0`. Then:

* If `c>=0`, `h*b>c>=0`, hence `b>0` and `q(z)>0` for every `z>0`.
* If `c<0`, the product of the two real roots is negative. There is exactly
  one positive, simple root, with full-real root ordinal 1; `q` is negative
  below it and positive above it.

Thus the existing endpoint-sign CAD and restoration are sufficient. An
identically zero endpoint gives the whole positive ray, while ordinary
nonzero endpoint seams retain the existing generic-exception treatment.
Strict negative-target problems use exactly the `c<0` base and lower fibers.
The certificates use the original domain with the transferred target removed.
They cannot use a target sign that is still being established.

`MonotoneStep.conditional_linear` stores the exact multiplier and separate
strict certificates for `h` and `h*b-c`; the leading-coefficient proof remains
mandatory. Replay reconstructs the latter expression from the original
quadratic. It checks multiplier support, proof bounds, strict signs, and
exclusivity with the direct linear-coefficient proof. Historical missing fields
retain the old `b>=0` meaning. Boundary formats and root ordinals are unchanged.

Discovery is a bounded sufficient-condition search. It considers at most 128
sums of positive coordinate variables, optionally plus 1. Variables absent
from the leading coefficient receive initial priority. Inputs have at most
8 active axes, 64 map axes, 256 terms, total degree 8, and 512-bit numerator
and denominator coefficients. A cumulative 2M product-work estimate and
2048-term difference cap bound this extra work. Failure leaves the existing
exact base fallback available. As with other certificate calls, the CLI's
killable worker supplies the hard deadline for a native algebra operation.

## Exact A381 and Ax381 identities

The original fixtures use `mt2=1`, `m=mz2=5/18`. Define

```
A = x0+x2       B = x4+x5+x6       z = x8
k = 2-m        h = A+z             a = B+z
delta_A  = s+t-m
delta_Ax = m-t
b = B²-s*x4*x6 + k*(A*(B+z)+B*z) + delta*x5*z
R = (k-1)*(A²*(B+z)+2*A*B*z) + k*(A+B)*z²
    + s*(x0*x2*(B+z)+z*(x0*x6+x2*x4)) + delta*h*x5*z
F = a*x3² + b*x3 + h*b-R
```

The last identity was checked by Symbolica against each full original fixed
and symbolic A381/Ax381 fixture, with no coordinate set to zero or one. The
identity helper compares exact primitive positive normalizations; the matching
raw `x3²` coefficient fixes the scalar as well. At `s=10,t=-3`,
`delta_A=121/18` and `delta_Ax=59/18`. Consequently `a,h,R` are strictly
positive on the entire positive orthant. Independent certificate replay
accepts all three. The expanded `R` has only 23 terms. The direct certificate
for `b` fails; `b` can change sign, which is why the conditional rule matters.

After the certified first peel, set `x3=0`. With `C=x4+x6`, the endpoint has a
second exact quadratic representation in `x5`:

```
a2 = A+z                    h2 = C+z
b2 = A²-s*x0*x2 + 2*(A+z)*C + 2*A*z
R2 = (A+z)*C² + 2*(A+z)*C*z + 2*A*z²
     + s*(A+z)*x4*x6 + s*z*(x0*x6+x2*x4)
F|x3=0 = a2*x5² + b2*x5 + h2*b2-R2
```

This identity was also checked against all four full fixtures by substituting
its expression for the constant coefficient of `x3`. The three strict
certificates for `a2,h2,R2` pass already over `s>4`; `R2` has 22 terms.
This supports the fixed-case peel chain `x3`, then `x5`, including positive
scale fixing of `x8` (or `x0`). A compatible lifting order is
`[x8,x4,x6,x0,x2,x5,x3]`, with the homogeneous block containing all seven
Feynman variables and anchor `x8`.

The final residual is linear in `x8`. Writing `B0=x4+x6`, exact comparison
against the four full original inputs also verified

```
F|x3=x5=0 = A*B0*(A+B0)-s*(x0*x2*B0+x4*x6*A)
             + x8*((A+B0)^2-s*(x0+x4)*(x2+x6)).
```

Thus fixing `x0` as the scale and lifting `x8` last in the remaining base is
another legal ordering candidate. Linearity is an algebraic observation;
the coefficient changes sign, so it is not an additional positive-slope peel.

The full symbolic first-step identity remains exact, but `delta` is not
positive on the unrestricted `s>4,t∈R` domain. The first strict certificate is
therefore not claimed there. Nor does a valid endpoint identity by itself
authorize setting `x3` or `x5` to zero. Existing ordinary monotone-candidate
probes did not certify either first peel on these full original fixtures.

## Research evidence and limitations

Artifacts are under `artifacts/research/convex-peel-audit-01/`:

* `a381-{fixed,symbolic}-identity.json` and corresponding `ax381` files:
  four full-polynomial checks, all `equal=true`, with empty substitution lists.
* `*-two-peel-identity.json`: four exact second-representation checks.
* `*-residual-identity.json`: four exact checks of the final linear-in-`x8`
  representation, embedded below the original two quadratic coefficients.
* `a381-fixed-certificates.json`, `ax381-fixed-certificates.json`: strict
  certificates for `a,h,R`; the fourth entry records the failed direct `b`
  certificate. `R` certification took 0.000340/0.000306 seconds respectively.
* `second-peel-symbolic-certificates.json`: `a2,h2,R2` strict certificates and
  failed direct `b2` attempt; `R2` certification took 0.000350 seconds.
* `*-candidates.json`: old direct-monotonicity probes on the full original
  domains. These are diagnostic failures, not completeness claims.

The bounded research commands used CPU9, 2GiB virtual memory, and 10/15-second
shell timeouts; no projection campaign ran during these checks. Helper hashes:

```
endpoint_identity: addc74a94abc1762c02822c4a73ab619fba97603ec9e70e502d5831492d22381
monotone_candidates: 57e13838a0767ecf9479883286bd92a9ae03d31a3f0b75d128e29d3987f1b229
profile_certificate: e01cae5392f6f5fcbcbd1420642e09c0c7f273ef87541f5d1657f6a2b05eeea9
```

An early tentative Ax formula used `delta=-t-m`; Symbolica rejected it. The
rejected `ax381-identity.toml` is retained in the artifact directory. All named
`ax381-{fixed,symbolic}-identity` and `*-two-peel-identity` evidence uses the
correct `delta=m-t`. The coefficient of `x3*x5*x8` in the Ax fixture is `2-t`.

These measurements establish a cheap exact coefficient obligation, not a
completed original A381 solve or a whole-solver speedup. General convex
quadratic peeling has a genuine two-positive-root branch when `c>0,b<0,D>0`;
the existing local quadratic classifier already handles it. A446 exhibits
that branch in an exactly checked positive-coordinate fiber (saved in
`symbolic-identity.txt`), so the endpoint-only rule cannot be assumed there.
