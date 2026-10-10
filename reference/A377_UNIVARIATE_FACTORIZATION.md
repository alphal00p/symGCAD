# A377: square-free decomposition before irreducible factorization

The captured univariate discriminant has a complete native square-free
decomposition within the 60-second diagnostic budget. Explicitly removing
the known threshold factor does not show a total-time improvement. These
measurements support testing root isolation of the square-free blocks before
changing the final projection level. A subsequent independent probe completed
the 881-degree block's inventory and found 35 real roots above four, refuting
its proposed nonvanishing. Neither result completes the original A377
decomposition.

The signed degree-2169 D and its independent 2,480-image verification are
documented in [the boundary diagnostic](A377_DISCRIMINANT_BOUNDARY.md).
That diagnostic proves `D=s^140(s-4)^8 R`, with `deg(R)=2021`, and refutes
nonvanishing on `s>4` by exact sign changes. Nothing here removes an interior
zero or uses coefficient signs to omit a guard.

## Existing native behavior and exact preparation

The current dependency is the local Symbolica checkout at
`75f8350094b90254ee71dc2a391fde0d14b0204a`. Its integer
`Factorize::square_free_factorization` already removes common monomial
powers before the Yun derivative/GCD stage. Rational factorization clears
content, uses that integer implementation, then restores content. Native full
`factor()` calls square-free decomposition before irreducible factorization.
Thus explicit extraction of `s^140` is not a new optimization.

The research helper uses native exact univariate division by `s-4`, checking
each multiplication identity, until evaluation at 4 is nonzero. A fixed
64-division limit prevents unbounded threshold search. It retains both
boundary factors with their exact multiplicities and reconstructs signed D,
including its content. There is no domain-dependent omission. The resulting
R has 2,022 terms and coefficient height at most 15,738 bits; its expression
SHA-256 is
`b4cbffce2743f9c0555a4f29bcc9a90c5cc3fc5e61ec452006147ae55a793c0c`.

## Separate bounded stages

Each row uses one fresh CPU9 process, 4 GiB, and a 60-second hard deadline.
The frozen development helper and candidate are identical between rows.
Preparation, native arithmetic, and final reconstruction are separately
recorded; a reported completion requires the entire signed original identity.

| Stage | Total process wall | Native stage | Exact reconstruction | Peak RSS |
|---|---:|---:|---:|---:|
| Boundary deflation and identity | 2.662 s | 1.929 s preparation including identity | included | 65,128 KiB |
| Native square-free decomposition of D | 36.598 s | 30.579 s | 5.246 s | 61,912 KiB |
| Deflation, then native square-free decomposition of R | 37.059 s | 29.001 s | 5.367 s | 73,628 KiB |
| Deflation, then full factorization of R | timeout at 60.026 s | incomplete | not reached | 74,292 KiB |

The residual row additionally spends 1.925 seconds on exact preparation.
These are individual diagnostics, not repeated performance comparisons.
They provide no measured total-time gain for a threshold-deflation production
prepass. The original square-free result retains `s` with multiplicity 140
and includes `s-4` in a degree-three block of multiplicity eight.
The full-factor timeout left its phase snapshot inside the native call and
returned no factor list. The earlier unprepared full-D factor probe, using
a different frozen release helper, also timed out at 60.024 seconds; neither timeout locates a specific internal
Hensel or recombination stage. The separate successful square-free probes
establish that a complete square-free decomposition is achievable within
this budget, even though full factorization was not.

For the residual, the nonconstant square-free blocks are:

| Degree | Multiplicity | Maximum coefficient bits |
|---:|---:|---:|
| 111 | 1 | 690 |
| 881 | 2 | 5,017 |
| 10 | 6 | 33 |
| 2 | 8 | 9 |
| 4 | 10 | 26 |
| 1 | 12 | 9 |
| 1 | 20 | 1 |

Their multiplicity-weighted degrees sum to 2021; their distinct degrees sum
to 1010. Scalars remain in the checked product. The helper records each block
hash, degree, term count, height and multiplicity, and checks both the residual
and full original reconstruction. These blocks are not asserted to be
irreducible. This square-free stage did not measure real-root counts.

## Subsequent isolation closes the nonvanishing proposal

The independent isolation adapter reconstructed the complete signed D from
the exported factors before selecting entry 4. Native exact isolation of that
881-degree block completed in 73.830 seconds, with 69 distinct real roots.
Its ordered rational intervals put 34 roots strictly below four and 35
strictly above four; none straddles four. The first interval above four is

```
16384/4077 < root < 16384/4045,   full-real root index 34.
```

Both endpoints exceed four. Thus this block has interior zeros in the
original s domain even though it appears with even multiplicity in D.
Sign changes of D reveal odd-multiplicity factors; they cannot rule out these
additional even-multiplicity zeros. The 881-degree block therefore cannot be
discarded by a whole-domain nonvanishing certificate. No positive-t collision
or original-F singularity is inferred from these projection roots.

The report is `artifacts/research/a377-isolation-01/block881.worker.json`,
SHA-256 `216adf694d109953b0f5977d0f300d09d6adde214f251ca73ea2cfd07c6de5cc`.
It binds the original candidate, factor bundle, selected block and interval
inventory. This completed **block** inventory does not yet supply the union
for all factors. The proposed shifted-coefficient screen is superseded by
this exact result. Its unbuilt example-only draft is retained separately as
`artifacts/research/a377-deflation-02/unbuilt-factor-screen.patch`; no factor
was relabelled as a discriminant, and no extra screen or production change ran.

## Cache and proof implications

`algebra::factor_polynomial` is shared by baseline projection, cached
projection and independent replay. An exact deflation preconditioner that
fully factors R and prepends the two linear factors would preserve its
contract and canonical output. The existing operation checkpoint stores
canonical factors and positive multiplicities, validates degree totals and
exact reconstruction, and is keyed by the original operand. It must not be
relabelled as a complete irreducible factorization merely because a partial
decomposition reconstructs: the local verifier independently enforces its
stronger irreducible-registry contract.

At the zero-dimensional base, irreducibility is not mathematically needed
to isolate the complete real-root union. `roots::isolate_union` already calls
the native square-free/VAS isolator on general rational polynomials and
handles shared roots exactly. However, current local proof checking requires
`factors_checked([p]) == [p]` for **every** registry polynomial, and repeats
complete factorization of projected raw guards. The existing base-point
shortcut only removes ancestor projection operations; it does not bypass
those earlier factor calls. The ordinary global verifier also recomputes
the exact factor basis and compares the entire registry and levels.

A future explicit base-univariate support-basis mode would therefore need:

1. A deterministic producer/checker rule restricted to polynomials whose
   exact support is the first lifting coordinate only, with no ancestors;
   all higher-level inputs retain the ordinary factor contract.
2. Exact raw-obligation reconstruction, retained coordinate/threshold factors
   and complete distinct-root union replay. Composite-polynomial full-real
   ordinals must be freshly derived, rather than copied from constituent
   factors. Repeated and shared roots must remain covered.
3. Separate cache typing/context, so composite support blocks never enter a
   factor cache as purported irreducibles or a later higher-dimensional
   pair projection. Historical proofs retain their current semantics.
4. A measured complete root union of the 881-degree block and remaining blocks
   before claiming that avoiding irreducible factorization solves this
   bottleneck; completing the single block above is only one part. Raw
   repeated D also causes `isolate_union` to compute another
   derivative GCD after isolation when a real multiplicity exceeds one.
   Exact product reconstruction is enough to justify a decomposition into
   support blocks; the unchanged root isolator does not need the checker to
   assert that each supplied block is square-free or irreducible. Reusing
   internal isolation/SQF work would require a separate checked interface.

This is a research design, not an implemented proof exception. Preserving
the whole polynomial is sound at the base only because every real zero is
still isolated and retained; no nonvanishing inference follows from this
argument.

## Reproduction and bounds

`examples/profile_univariate_deflation.rs` has three focused tests covering
signed content and multiplicities, repeated nonreal factors, failed threshold
division, identity tampering, the threshold cap, input grammar/power caps and
startup-banner handling. The candidate grammar admits only bounded integer
coefficients and increasing explicit `s` exponents: 16 MiB input, 4,096
terms/degree and 6,000 decimal digits per coefficient. Each stage runs under
the existing killable worker supervisor; timeout records preserve the last
atomic phase snapshot and cannot report a partial factor list as success.

Build/test commands used `source .symgcad-env`, `ulimit -v 4194304`, and
`nix develop /common/dev/gcad -c taskset -c 14,15 cargo {test,build} -j2
--example profile_univariate_deflation`. The frozen executable is
`artifacts/research/a377-deflation-01/profile_univariate_deflation`, SHA-256
`f6c01b423116fb0a195f9516d1eaf4016165dcda5416536488fe444bf60a60bf`.
The adjacent `build.json` binds source head `d50794f`, source/dependency files,
candidate, independent grid, tests and build logs.
The stage-summary SHA-256 is
`bdb5803a317bdc8e794882d78e2cb0515d82dd924ed26436a05d42f6e2e92b29`;
it binds all four process reports. The successful residual square-free report
SHA-256 is `dc27df111359c427066f92a8aef399636714530833c4a176fff2371ffd113da0`.

For example, with a fresh report path:

```sh
source .symgcad-env
artifacts/research/a377-deflation-01/profile_univariate_deflation \
  artifacts/research/a377-univariate-segmentation-01/discriminant.json \
  --stage square-free --operand residual --cpu 9 --seconds 60 \
  --output artifacts/research/a377-deflation-01/square-free-residual.json
```

Use `--operand original` for the unprepared square-free control and
`--stage factor --operand residual` for the complete residual factor call.
The optional research `--save-factors` writes a bounded fresh sidecar only
after signed reconstruction succeeds. It preserves all original factors,
scalars and multiplicities, with explicit bounded integer coefficient/power
syntax and a checked serialization roundtrip. Consumers must independently
reconstruct the original candidate before using that sidecar; it supplies
neither irreducibility nor root-count authority.

The separately frozen export-enabled helper passed the same three tests,
including export/reparse/reconstruction, in a seven-test bundle with the
independent isolation adapter. Its SHA-256 is
`3b1a1acbc3d1299f7c154a70c984242beeac0946d48ff282b3144c9cc65c1e89`.
The export replay completed in 35.971 seconds at 73,536 KiB. The complete
1,302,803-byte factor sidecar is
`artifacts/research/a377-deflation-02/square-free-residual.factors.json`,
SHA-256 `e83563f9721a38ebf3de7fa9ff95ff2c28adb0e0056a6151d81540bcd5f20e5c`.
Its eleven entries include the two boundary factors and both scalar factors;
entry 4 is the 881-degree block, and entry 3 the 111-degree block. This later
export run is separate from the original timing table. The adjacent build
manifest binds the updated helper source and combined test/build logs.
All original and failed earlier artifacts remain unchanged.
