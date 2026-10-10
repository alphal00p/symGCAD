# Exact physical elliptic coefficient certificate

The original symbolic physical fixture now has a verified decomposition of
269,061 cells. The successful negative-target campaign and its exact input
identity are recorded at the end of this report. The earlier experiments
below explain the certificate and memory bottlenecks encountered before that
completion; their incomplete results remain unchanged.

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
only s and pp4. This established concrete pruning opportunities; the original
physical decomposition was still unresolved at that stage.

## Observed quadratic-certificate effect

The cold original-problem trial at production commit `3acd6aa` retained the
same x6 anchor and order as `physical-affine-square-x6-global-04`. Two exact
`QuadraticNoRealRoots` removals were used: C above, and
`s^2-2*s*pp4+4*pp4`, whose discriminant in s is `4*pp4*(pp4-4)`.
Removing these factors also prevents later obligations from being generated.

| Projection stage | Prior factors / terms | New factors / terms |
| --- | --- | --- |
| After x1 | 24 / 161 | 20 / 91 |
| After x3 | 66 / 606 | 37 / 192 |
| After pp4 | 1,814 / 12,756 | 286 / 1,290 |
| Final retained inventory | 1,159 / 8,366 | 178 / 857 |

This is an exact inventory comparison, not an isolated timing ablation: the
new worker also contains intervening arithmetic and evaluation improvements.
The new run completed base lifting in 98.69 seconds, with 1,044,713 stacks and
1,021,643 accepted base cells. It then exhausted its 4 GiB allocation limit at
116.10 seconds, before emitting the original reconstructed result. Separate
lifting progress establishes that the failure occurred after base lifting;
the exact subsequent materialization/reconstruction step is not instrumented.
No new original completion follows from the smaller projection or finished
base lifting.

The full records are under
`artifacts/research/physical-quadratic-x6-global-08`, including `analysis.json`,
the typed removals in `initial-x6-0/progress.json`, and the separate
`lifting-progress.json`. The immutable debug worker SHA-256 is
`463806e5636de61a66ef203d8e67bb6767a0192b56d47b8a92f6ef4f8fb9bed9`;
the projection and lifting reports have SHA-256 values
`3578626ccce6b2beef35f24de076d6d5f8ba7e994a7893fb6f8c1fcadca7f4dc`
and `cf2a16e9b6c3ef7a87929fbcc53def458ac1e84336c2be12c6397630e79d40ba`.

The corresponding fresh-cache local-base trial in
`physical-quadratic-x6-local-09` also remained incomplete: it timed out at
180.02 seconds with 544,266 speculative nodes, 48,901 refinements and 75,864
KiB peak RSS. Thus the new certificate has a measured global projection
benefit but has not by itself resolved local refinement growth or completed
the original problem.

## Verified original negative-target decomposition

The bounded campaign `physical-negative-target-m-first-12` completed the
untouched fixture `test_elliptic2l_physical_symb/order/1` with 269,061 cells,
then verified its full proof in a fresh worker. Its lifting order was
`pp4,t,s,x6,x2,x3,x1,x0,x5,x4`, with x6 as the positive homogeneous anchor.
The order respects both original variable groups; the fixture has no fixed
order. Exact comparison preserves the name, variables, constraints, split
polynomials, parameters and groups. Only solver options and resource limits
are changed.

The monotone engine uses the original inequality directly. After anchoring
x6, the affine-square certificate proves the x4 quadratic strictly increases
on x4>0. A negative fiber exists exactly when its endpoint is negative, and
then consists of the single interval from zero to its unique positive root.
The base therefore solves the strict negative endpoint constraint; positive
endpoint cells are not needed for this inequality. This is a proof of the
original F<0 domain, not a claim to have constructed both signs of F.

The optimized base ends with 168 projection factors and 740 terms. Although
the supplied directory already contained earlier completed checkpoints,
the changed target context produced zero prefix, factor, discriminant or
resultant disk hits. The successful run is therefore cold with respect to
the persisted algebra cache. The proof independently checks all base cells,
the monotone reconstruction and the homogeneous transport into the original
coordinates, including the exceptional-set obligations.

| Measurement | Observed value |
| --- | ---: |
| Original output cells | 269,061 |
| Solve worker wall time | 156.427077 s |
| Solve peak RSS | 3,593,444 KiB |
| Fresh verification wall time | 338.804974 s |
| Verification peak RSS | 7,741,384 KiB |
| Whole supervised campaign | 556.965 s |
| Proof JSON size | 1,393,759,825 bytes |

Both workers used one CPU (CPU6) and a 16 GiB bound; solve and verification
had separate 300 and 600 second bounds inside a 950 second campaign. This is
one exploratory debug-build run, not a repeated release performance claim.
The frozen production commit is `529e708`, with executable SHA-256
`b5f3742b4b4faa68f0d02920ee43e033be4021e28bd38109ce4f817dd1c64ae2`.

Evidence is under
`artifacts/research/physical-negative-target-m-first-12/initial-x6-0`:

- `original-identity.json` records exact original mathematical-field equality.
- `record.json` preserves both bounded process outcomes and cache counters.
- `verification.json` reports `verified=true`, 269,061 cells, 806,840 checked
  stacks and 1,843,170 rejected branches.
- `result.json` has SHA-256
  `bc62bf55da7df70e60a9938b01cf32abeb5a40ddb6678c8bc2283be491d2d9b9`.

The standard completion harness repeated this exact original case with the
immutable `1e29f04` release worker and a fresh cold checkpoint directory.
It again produced and independently verified 269,061 cells. The public solve
process took 33.856314 seconds with 3,588,004 KiB peak RSS; verification took
73.085090 seconds with 6,628,724 KiB peak RSS. The whole campaign took
114.448661 seconds. These are single-run measurements, not repeat medians.
The worker SHA-256 is
`77dc42588f83ff0bd15b68cde873de97ff02213bd2a4d2fd64233d51e548f281`.
The standard ledger is
`artifacts/research/physical-negative-target-release-14/results/ledger.json`,
and its original mathematical identity is
`082baf881fef15de493e1d039955ec79c2803eaaf0411e6f6ec51cca57c0c091`.
The release proof has SHA-256
`453e1ce44186b2bb5e588766f539feac7010a5ab75583655af069ed949fff857`.

For comparison, the earlier full-arrangement route emitted a separate
5,878,358,378-byte original proof with 350,082 negative cells. Its debug
verification timed out at 600 seconds, but a fresh `3acd6aa` release replay
verified it in 163.685577 seconds with 28,021,356 KiB peak RSS. Its preserved
solve campaign is `physical-quadratic-x6-global-10-large`; the independent
release replay is recorded in `physical-release-verification-13/old-release`.
The proof SHA-256 is
`185935901c076b6b9e45751dc58c2cbc411c99756232625532f4a78df4bae8c2`,
and the replay supervisor checked that its bytes were unchanged before and
after verification. The smaller negative-target completion does not rely
on that larger proof.
