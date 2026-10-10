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

The bounded production rule instead retains the original Domain
and explicitly records which available upper bounds orient the affine
substitution. Every original-domain point maps to a positive coordinate;
the additional lower bound need not be used in the coefficient argument.
After that substitution, a positive rational binomial square plus an exact
strictly positive residual certifies the requested sign. Independent replay
must check bound availability, distinct valid indices, positive weight, the
polynomial identity and strict residual signs. Discovery uses a small
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
strict-positive leading-coefficient convention also matters: the original
F<0 constraint normalizes to the positive target -F. Sign-filter evidence now
records the arrangement orientation and selected sign, so it can solve the
increasing split F and exhaustively select its negative cells. Independent
verification reconstructs that exact identity without repeating discovery.

The combined square-residual and sign-orientation implementation passed all
330 library/integration tests, including strictness, identity, index and bound
tampering, negative target selection and historical proof replay. This checks
the implementation; completion of the original endpoint problem is still a
separate requirement.

## Homogeneous-anchor effect and affine square extension

Three bounded original-problem trials under the initial binomial rule did not
complete. The x0-anchored local-base trial performed one certified x4 peel and
timed out after 120 seconds. The x6-anchored local and global-base trials each
timed out after 60 seconds **without any certified peel**. Fixing x6=1 changes
the useful square to `(x3-1)^2`, which the two-variable binomial discovery did
not cover. Those two runs therefore do not measure the endpoint reduction.
The third run reused completed checkpoints from the second; none is a cold,
repeated performance benchmark. Exact inputs, captures and the correction are
retained in `artifacts/research/physical-square-monotone-01/portfolio-analysis.json`.

The typed `AffineSquarePlusPositiveResidual` certificate covers this missing
case. In the coordinates obtained from the recorded original-bound
orientations, it proves the exact identity
`sign*p = weight*(u_i-center)^2 + residual`, with positive rational weight,
rational center and a strictly positive coefficient-sign residual. The
coordinate is an explicit valid variable index; the constant is never encoded
as a fictitious variable. The checker reconstructs this supplied identity
without repeating candidate discovery.

Discovery completes one constant-coefficient diagonal square: if the selected
quadratic and linear coefficients are a>0 and b, it tries weight=a and
center=-b/(2a). Affine and binomial candidates share the existing attempt and
work budgets. The new affine method additionally bounds weight/center rational
height by 256 bits and polynomial/residual coefficient height by 512 bits,
where height is the sum of numerator and denominator bit counts. Existing
binomial evidence retains its previous meaning.

For x6=1 and upper-oriented pp4, this selects weight=1 and center=1. Its exact
residual is `u_pp4*x3 + 2*(x0+x1)*(x3+x5+1) + 2*x3*x5 + (2+u_t)*x5`,
strictly positive on the original domain. This enables the first reduction;
the subsequent endpoint problem still requires a completed proof. The exact
endpoint factor identities and the suggested x3,x1,x0,x5,x4 lifting order are
documented in [the endpoint audit](PHYSICAL_ENDPOINT_FACTORS.md).

The b8139e0 original-problem runs confirmed a real x4 peel. With x6 anchored,
the global endpoint projection completed eight stages, but the process then
exhausted 4 GiB after 145.95 seconds without an output. Its exact failure phase
after projection is unknown because lifting progress was not enabled. The
local-base counterpart timed out after 180.04 seconds, using 76 MiB and visiting
566,945 speculative nodes. An x3-anchor alternative with larger caches also
timed out: its release promotion reached 2,654,747 speculative nodes in 180.04
seconds with no completed proof. That promotion reused completed algebra
checkpoints, so it is not a cold runtime comparison. These failures remain in
`artifacts/research/physical-affine-square-x6-global-04`,
`physical-affine-square-x6-local-05`, `physical-affine-square-x3-local-06` and
`physical-affine-square-x3-release-07` beneath `artifacts/research`.

One exact remaining opportunity is the constant coefficient of A in x0:

```text
C = (x1+x3+1)^2 - pp4*(x1+x3)
  = (x1+x3-1)^2 + (4-pp4)*(x1+x3).
```

The two forms are verified by a zero Symbolica difference in
`core-structural-identities-01/physical-positive-endpoint-result.json`. Viewed
as a quadratic in x1, C has leading coefficient 1 and discriminant
`pp4*(pp4-4)<0` on the original box. The bounded nonrecursive quadratic rule
proves this using the existing Bernstein certificate for the negative
discriminant; it needs no new multi-variable square search. A scan of the
actual retained factors found six further valid applications, four involving
only s and pp4. This establishes concrete pruning opportunities; the original
physical decomposition remains unresolved.
