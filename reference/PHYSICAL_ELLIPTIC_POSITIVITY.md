# Exact physical elliptic coefficient certificate

For the untouched symbolic core fixture
`test_elliptic2l_physical_symb/order/1`, let F denote the polynomial in its
original strict inequality F<0. As a polynomial in positive x4,

```text
F = a*x4^2 + b*x4 + c,
a = x3+x5+x6,
b = (x3-x6)^2 + R,
R = (4-pp4)*x3*x6 + 2*(x0+x1)*(x3+x5+x6)
    + 2*x3*x5 + (2-t)*x5*x6.
```

The original domain has `0<pp4<4`, `t<0`, and positive x0 through x6.
Consequently a>0 and R>0, so b>0 even where the square vanishes. This proves
strict increase of F on x4>0. It does not by itself solve the endpoint
problem c or constitute a completed original decomposition.

The two coefficient identities were checked exactly with Symbolica by
normalizing their differences to zero. Evidence is retained in
`artifacts/research/core-structural-identities-01/physical-result.json`.

## Existing proof and the bound-orientation limitation

The existing coefficient prover declines b and R on the full box because it
chooses the lower pp4 bound. Its finite-box Bernstein member does not apply
when the active x coordinates have no finite upper bound. A research replay
omits only the original pp4>0 constraint, retaining pp4<4 and every other
original coordinate bound. This is a strictly larger domain. Substituting
`pp4=4-u` and `t=-v` makes every coefficient of R positive, with u,v and all
active x coordinates strictly positive.

The frozen research helper constructs and independently replays a complete
`ShiftedCoefficients` DomainCertificate for R on that larger domain. Replaying
the same certificate with no omitted constraint is correctly rejected:
the current verifier requires its recorded domain to equal the independently
derived bounds. The certificate must not silently be relabeled as a proof on
the original box.

The proposed bounded production rule instead retains the original Domain
and explicitly records which available upper bounds orient the affine
substitution. Every original-domain point maps to a positive coordinate;
the additional lower bound need not be used in the coefficient argument.
After that substitution, a positive rational binomial square plus an exact
strictly positive residual certifies the requested sign. Independent replay
must check bound availability, distinct valid indices, positive weight, the
polynomial identity and strict residual signs. Discovery can remain a small
portfolio of rational weights drawn from positive diagonal coefficients.

## Retained research evidence

The input, complete certificate, original constraint inventory, explicit
omitted index, successful replay and rejected no-omission replay are under
`artifacts/research/physical-elliptic-square-01`. The frozen helper and its
source are in that directory's `helper` subdirectory. Its SHA-256 is
`e01cae5392f6f5fcbcbd1420642e09c0c7f273ef87541f5d1657f6a2b05eeea9`.
The full certificate report SHA-256 is
`13e24a1b4233f3f25539931fbec951e78eec881232a0ff7372418679f28b5be2`;
the exact coefficient-identity result SHA-256 is
`96295074cf70242d16373e9831ff17731bb4d1741d8d7ba0c6594cb74bc2f6a6`.

Earlier direct original-problem local trials with anchors x0 and x1 both
exhausted 100,000 refinements without a completed decomposition. They remain
in `artifacts/research/physical-elliptic-structural-01`. The monotone engine's
current strict-positive leading-coefficient convention also matters: the
original F<0 sign filter supplies -F. Using the F monotonicity argument there
requires checked sign orientation, not an unrecorded negation of the target.
