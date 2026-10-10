# A377 captured discriminant: signed exact verification

A captured bivariate discriminant from the second warm A377 follow-up was
computed by the research binary-segmentation helper and independently verified
on a complete exact specialization grid. This is an operation result. It does
not complete the original decomposition, prove nonvanishing on its domain, or
change production dispatch.

The immutable input is `artifacts/research/a377-univariate-segmentation-01/capture-observed.json`,
copied from `a377-segmentation-warm-hour-02/captures/active-979117.json`.
It records an unfinished discriminant operation in `t`, with the original
variable map `[s,t,x0,x3,x5,x6,x8]`. The operand has 1,341 terms and degree 24
in `t`; its derivative has 1,277 terms and degree 23. Only `s` survives.
The snapshot alone does not establish how long this particular operation had
run in the original worker.

| Quantity | Exact diagnostic result |
|---|---:|
| Resultant degree bound in `s` | 2,479 |
| Encoding digits / bits per digit | 2,480 / 17,644 |
| Encoded output bit bound | 43,757,120 |
| Discriminant degree / terms | 2,169 / 2,030 |
| Reconstructed resultant degree / terms | 2,208 / 2,069 |
| Encoding worker wall time / peak RSS | 148.515 s / 260,340 KiB |
| Independent grid worker wall time / peak RSS | 192.097 s / 52,240 KiB |
| Accepted exact grid identities | 2,480 / 2,480 |

Both workers used CPU 0 and a 4 GiB address-space limit. Encoding had a
180-second deadline; grid verification had 300 seconds. The frozen research
helper is a debug executable with the repository's optimized dependency
profile. These are single diagnostic measurements, with no matched release
or whole-solver speedup claim. Integer resultant computation accounted for
147.724 seconds of the encoding worker. Independent algebraic verification
took 191.554 seconds.

## Independent signed identity

For the captured polynomial `p`, degree `n=24`, let `q=dp/dt` and
`l=lc_t(p)`. The research adapter
`examples/prepare_discriminant_grid.rs` constructs the candidate

```text
R = (-1)^(n(n-1)/2) l D = l D.
```

It preserves signed rational scale and checks exact quotient reconstruction
of `D`. It also binds the original capture, operand strings and candidate by
hash, and emits a derived pair capture for `(p,q)`. Preparation itself marks
the candidate unverified.

The existing `profile_kronecker_resultant` verifier independently derives the
degree bound 2,479, rejects candidate support outside that bound, and compares
`R(s)` with fresh ordinary exact numeric Ducos resultants at 2,480 distinct
rational values. It skips a value if either operand loses its degree in `t`.
The accepted values need not lie in the physical domain: polynomial identity
on this complete grid proves equality for every `s`. Thus the signed
resultant is exact, and the checked nonzero polynomial leading coefficient
and exact division prove the signed discriminant identity as well. No
primitive normalization or factor-basis equality substitutes for this check.

The adapter's two tests compare rationally scaled quadratic, cubic and quartic
examples against ordinary Ducos, and demonstrate why the degree-dropping
specialization of `s*t^2+t+1` at `s=0` must not count as a valid grid image.

## Provenance and limits

All paths below are relative to
`artifacts/research/a377-univariate-segmentation-01/`; large expressions and
native dependency sources are not committed.

| Artifact | SHA-256 |
|---|---|
| `capture-observed.json` | `df2a381b027bbbf7e721c47c4533d11177a9172038375ae0a8fd029a7c4aab92` |
| Captured operand-list input digest | `03edf963d7fbdfc0a1da526152ac9b3d887560c18fd901eb9ef21d9f963ce800` |
| Frozen `profile_kronecker_resultant` | `44428e764331e44d295eb4b8dae17201d39fe7b1a8573a240a499b1fea95ec09` |
| `discriminant.json` | `0c457cfce6ed2df4545054b7ba79b76e62123e284a665ea39db12bc4124c09c2` |
| Signed discriminant expression | `ad2b39aac6b79964c9eed60e8681f11248991ffaf7dd1a8bfe4b9da2298b17cc` |
| Derived pair input digest | `7fa59055de963532446b7b72d106852a64c7e440ea122d8eb948b7d797807ca2` |
| Signed resultant expression | `241fa60bb93a87dc3f1e30b1eb89123ecdc12b4301c0ede0f46aa6d44f96f9b9` |
| `grid-verification.json` | `4c85072f4229b2f1cacb53fe49bb6679d8441381dabbee9467251bef1d00ab9b` |
| `grid-supervisor.json` | `6469aee0289135651bc3db459db822fe5a5a2df059b31d242c22193f36c7f18f` |

`grid-input/preparation.json` binds the original and derived identities;
`discriminant-supervisor.json` and `grid-supervisor.json` retain process limits,
exit status and resource observations. The grid report has `verified=true`
and `checked_points=2480`.

At measurement time production segmentation admits pair resultants with two
surviving parameters; discriminants do not use that dispatch. This captured
one-parameter operation also exceeds the existing interpolation degree bound
of 1,024. A future discriminant extension would need its own bounded dispatch,
signed leading-coefficient reconstruction and regression coverage. The
successful operand calculation alone does not justify changing global caps,
discarding a discriminant, or bypassing downstream factor/root work.

A subsequent exact rational screen found `D/s^140` negative at `s=10`,
positive at `s=16` and `32`, and negative at `s=64`. Since `s^140>0` here,
these are also signs of `D`; continuity forces zeros in `(10,16)` and
`(32,64)`. Thus this discriminant cannot be discarded as nonvanishing on
`s>4`. Exact stripping and a native shift by four also identify an
`(s-4)^8` boundary factor, with mixed signs in the remaining shifted
coefficients. The separate bounded screen is retained under
`artifacts/research/a377-discriminant-boundary-02/`; the earlier `-01` supervisor
parsing failure is retained separately. The screen supplies no complete
root count, factorization or original-problem completion.

The next ordinary factorization of this exact signed discriminant was also
profiled independently. The frozen release helper
`artifacts/bin/profile-operation-literature-bbb701b-release` did not finish
within 60.024 seconds on CPU 3, at 55,872 KiB peak RSS and a 4 GiB limit.
No factor list was accepted. The derived factor capture has SHA-256
`46d5122f1f6320fcf98165b0a0382150071264923ff1eb259f83f1ed52ca744c`;
its process report is
`artifacts/research/a377-discriminant-factor-01/supervisor.json`, SHA-256
`3ec1826852dc8019309e9ca3770dd2489a56b606485cbe5fce0fb39b7104c8d1`.
This downstream timeout is retained as inconclusive evidence, not hidden by
the successful discriminant computation.

## Narrow opt-in production path

`solver.binary_resultants=true` now permits this one-parameter discriminant
strategy only after ordinary bounded interpolation statically declines.
Pair-resultant applicability and all encoding caps remain unchanged;
degree-at-most-four discriminant identities, baseline projection and default
behavior remain unchanged. Exact leading-coefficient division checks its
remainder, retains the conventional signed scale, and restores the original
variable map for native constant results, including shared-factor zero.
Errors from an attempted kernel propagate rather than triggering a second
expensive computation. Cache reports count `segmented_discriminants`
separately from pair resultants.

The local proof checker reads the explicit problem option and freshly
reconstructs each required discriminant through the same pure exact helper.
It does not read discovery caches or trust operation statistics. Global
verification and pair-resultant replay retain their ordinary backend, so they
may still encounter algebra bottlenecks. No accelerated solve is accepted as
a completion until independent full proof verification finishes.

The frozen production profiler independently recomputed this captured
operation with the default caps and compared its entire signed polynomial
against the grid-certified candidate: **exactly equal**, 2,030 terms. Its
encoding plan again reports 43,757,120 bits and one surviving parameter.
Algebra took 146.381 seconds; the process took 147.307 seconds and 257,804 KiB
peak RSS under the same CPU 0 / 4 GiB / 180-second allocation. These timings
are diagnostics, not a speedup estimate. The frozen nonconstant replay
predates only the native constant-result map compatibility fix.

- Production profiler SHA-256: `fe37ba440b63b99d0c084e69e18fa226ad43fe7aaefe9b2f933fea3e066b9858`.
- `production-replay.json`: `bdfb10d1a4449e6c6bc02a84b702849b6616fbd235d9790a8c2ed4ec83038e93`.
- `production-supervisor.json`: `64f5a0185b2022e973d892d8c0d40a814284d07361eafee7150dd6a144c04541`.

The focused bundle passed 265 tests before the additional native-zero
regression. Coverage includes signed rational scale, parameter-dependent
leading coefficients and degree drops, repeated factors, inactive variable
maps, interpolation priority, default fallback, cache metrics, independently
recomputed signed discriminants, and rejection of altered local cells. The
subsequent six-test discriminant target also covers the ordinary mapless-zero
backend result. No proof schema or exceptional-set obligation is removed.

The subsequent full fast suite passed all **537 committed tests**. It also
ran one uncommitted A381 regression from a separate workstream, which passed
and was left outside this change. The combined 538-test run covered 39 targets
with no failures or ignored tests; test execution took 52.80 seconds after
3 minutes 38 seconds of compilation. Its log is
`artifacts/discriminant-segmentation-fast-tests-02.log`, SHA-256
`09821c0f63b4a41d45ae6541655d1f5c918f71e1aac1d789b73b28a6fb313f91`.
The first full-suite invocation was stopped during compilation when review
found the native-zero map regression; its interrupted log is retained as
`discriminant-segmentation-fast-tests-01.log` and is not counted as a pass.
