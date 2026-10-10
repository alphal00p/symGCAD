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

## Completed ordinary operation and warm continuation

The second bounded A377 continuation ended after 3600.028 seconds at
424,292 KiB peak RSS, without a result or verification. Its cache audit shows
that the ordinary degree-24 discriminant above **did complete**, recording
3299.107 seconds. The final active capture is a different discriminant of
degree 18 in `t` and degree 42 in `s`; the deadline did not interrupt the
already completed degree-24 calculation.

The completed native operation and the frozen accelerated replay have the
same input digest, backend, variable map, elimination axis and canonical
output-array digest
`f38e7a98dab6d31382f4db9d44f6a229fa711cda8b2b9efac960001b35007493`.
This corroborates the canonical polynomial only: normalization removes signed
rational scale. The separate full-grid comparison above establishes the
signed-scale identity. The 3299.107-second cache duration and 146.381-second
accelerated algebra duration are individual operation diagnostics, not a
three-run benchmark or a whole-case speed ratio.

`artifacts/research/a377-segmentation-warm-hour-02/` retains the final audit
(SHA-256 `cfa6b86da35d52b7e4052414932b8a0a3afcc791121c0dd15bde17c4a34f9da1`),
comparison (`f933089c777fbe2791e7a448b0d604973476c2b03e535e2a80f287e740f6e216`)
and final cache manifest
(`816f7d6bbd2d074323d440b0b1c6a9834ddecebd5e322dfbd3778cae2444d2bb`).
All original 1,880 entries are unchanged; the new discriminant record brings
the total to 1,881 completed entries and 159,699,803 bytes.

The next release was built from source `d03f2ea6a1c2d1fb4e4a1271da6976390ef61078`
with all compiled source hashes checked before and after the build. Its
solver SHA-256 is
`e6ccecc0db581272a791cee3c3d5cb04cec7bb8fc87d807ce2efcef60c7a2650`;
the build manifest digest is
`c2031c8c4bc8eb7ecf7a586a01b0f37962a1935efe052cbfe8f87bbe735e229c`.
A fresh two-cell solve/verification and the native A446 checked import and
eight-cell proof replay passed with that executable.

`artifacts/research/a377-segmentation-warm-hour-03/` was launched at
11:52:25 UTC on 10 October 2026, on CPU 8 with 4 GiB and a 3600-second campaign
budget. Only the completed native cache entries were copied; no research
result was injected. Mathematics, variable groups/order, solver options and
limits match the previous run, apart from its fresh checkpoint path and the
new executable. Its initial-cache manifest digest is
`6b07016d8cecd254dd8548908cd8f4304c78faa12434d0c3bd8b697b0223e532`;
the launch record digest is
`772953b0dd528800284aa73adbef2590f8b18a8d477dfc0d86f959f28f039f27`.
This records a warm attempt starting, not a completion or cold benchmark.

### Third-continuation snapshot and the next larger operand

At 12:20:16 UTC, the third continuation had retained **167 additional
completed discriminants**, bringing its operation cache to 2,048 entries.
All 1,881 carried entries remained byte-identical. Every saved output checksum
and filename/key matched. The initial degree-18 discriminant completed in
9.418092 seconds; the longest newly recorded operation took 89.958847 seconds.
These are saved operation durations, not a completed-case timing. The snapshot
is `artifacts/research/a377-warm03-progress-01/cache-snapshot.json`, SHA-256
`ccbd1a10b1784a4e136408165bacc1184f66bc8baa8e94949786a7b9dae9a1b2`.

The active capture then contained a new degree-32 discriminant in `t`:

| Quantity | Exact inspection |
|---|---:|
| Input terms | 2,502 |
| Degrees in `(s,t)` | `(84,32)` |
| Maximum coefficient bits | 691 |
| Resultant degree bound in `s` | 4,772 |
| Encoding digits / bits per digit | 4,773 / 44,035 |
| Encoded output bit bound | 210,179,055 |

This exceeds the production 96-million-bit bound. The current dispatch
therefore correctly uses its ordinary fallback without entering segmentation.
The immutable capture digest is
`bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75`;
its operand-array digest is
`99f94af85a41a5dd07ae6f18f51b78b7b041d3c5ed47007e8179107f320b2de7`.
The inspected layout is retained alongside the snapshot, SHA-256
`b583beab7b90d34b282fce7308f180d4cc51059e2d5586f8778780f999a3d8b9`.

A separate research-only attempt allowed 256 million encoded bits, CPU9,
4 GiB, and 600 seconds. It **timed out after 600.145446 seconds**, using
1,267,020 KiB peak RSS, with no discriminant candidate. No result was inserted
into a solver cache and no production cap changed. The retained process
report is `artifacts/research/a377-degree32-segmentation-01/process.supervisor.json`,
SHA-256 `5db91d9381aa215bdada5677e6f099d5c01dc62fafa7a0a3c8eea8124486981c`.

The modest memory use justified a longer bounded research attempt after the
separate polynomial-centering test declined this operand. At **12:22:49 UTC**,
`artifacts/research/a377-degree32-segmentation-02/` began on CPU9 with 4 GiB
and an outer one-hour deadline. Computation receives at most 2,400 seconds;
if it completes, the script constructs the signed resultant candidate and
uses only the remaining campaign time for the independent full-degree grid.
A missing or interrupted grid remains unverified. The frozen computation
helper is the same `44428e76…` executable as above; the adapter digest is
`2b5143864a964b80f00aad7a6ce786d672f64f595b038be5f451ebf0b8a8db4a`.
The launch script digest is
`bd4b615ee0bc9eb4b565d160c2c2f543898d76ce6a65a1002416d3c94351c23c`.
This is a launched operation diagnostic, not a completed algebra identity or
an original-problem decomposition. All interior-boundary obligations remain.

### Third-continuation outcome

The third continuation timed out at 12:52 UTC after **3600.022407 seconds**,
with 613,324 KiB peak RSS and no result or verification file. Its final cache
contains the same 2,048 completed operations seen in the earlier snapshot:
all 1,881 inherited files are unchanged, and the 167 new records are completed
discriminants. Their recorded algebra time totals 179.382270 seconds; that
sum excludes all other solver work and is not a solved-case timing.

Every output checksum and filename/key was rechecked. The original mathematics,
actual worker input, and frozen executable hashes still match. The final active
capture is the same degree-32 operand above; this attempt supplies no value
for that discriminant. All completed work remains available for a later bounded
continuation. Verified original coverage remains **303 / 331**.

The final audit is
`artifacts/research/a377-segmentation-warm-hour-03/final-audit.json`, SHA-256
`73be22ca104a3d32d470e104bd610c02f76beabcae54a4de3c153d3034c64825`.
The final cache manifest has SHA-256
`053435cb2d173274bdf845c9ffd5005b6eef6ba9a9239ff5644b5a23dbfe253e`;
the final ledger has SHA-256
`b23781a4160426756abc9bcf91008fd0b599de50d570b71c6b09398c1059ea72`.

A separate 60-second release probe of Symbolica's existing primitive-resultant
strategy on that same degree-32 operand also timed out (60.034019 seconds,
90,176 KiB peak RSS). It produced no candidate. This bounded diagnostic does
not establish that the strategy cannot finish; it provides no measured reason
to change the production dispatch. Evidence is retained in
`artifacts/research/a377-degree32-native-strategies-01/primitive.process.json`.

### Fourth continuation: exact support basis

The fourth continuation started at **13:01:56 UTC** on 10 October 2026,
on CPU8 with 4 GiB and a one-hour campaign budget. It uses the frozen
`9b7dd56` release worker documented in
[the support-basis replay](BASE_UNIVARIATE_BASIS_PROPOSAL.md). The only
semantic option change is enabling `local_base_univariate_basis`; the
original mathematics, full order, groups and resource limits remain bound
to the preceding attempt. The option preserves every real root and its
interior boundary. It does not authorize replacing a polynomial by its
sign at a sample.

All 2,048 completed native cache records, totaling 179,997,346 bytes, were
copied with matching keys and output checksums. No research discriminant
or factorization was inserted. In particular, the larger degree-32
discriminant can still block this attempt before the new basis is useful.
This warm continuation is not a cold benchmark or a completion claim.

The directory is `artifacts/research/a377-segmentation-warm-hour-04/`.
Its launch record has SHA-256
`2b0a62ea34ba926ea13dc2297f981552181f4a49701fa9488fd7183c576b7c4f`;
the input audit and initial cache manifest respectively have digests
`a92b61e1e6306a4aacb723e81a8cbb446e6668ed1052c0766c8fc936389413a9` and
`d45aeb92dda6d68cd87708bdbb16adf492975d5149a3b40f59440589cb6c8798`.
The launch also binds the authenticated 84-root production replay and
separate CLI smoke verification.

The remaining native strategy probes also timed out without candidates:
Brown took 60.020651 seconds at 67,124 KiB peak RSS; CRT took 60.020616
seconds at 18,432 KiB. These short censored observations do not establish
relative completed-operation performance. The combined strategy summary
has SHA-256
`9c95059f3f8c34285564ae130caab5d8551ed6a5e19d1484395a96514906471e`.

### Support-only degree-bound diagnostic

A separate metadata calculation constructed the 63-by-63 Sylvester support
matrix for the degree-32 operand and its derivative. Each nonzero entry's
weight is its exact coefficient degree in `s`. An integer row/column dual
certificate bounds every determinant term by **4,714**, compared with the
current rectangular/diagonal bound of **4,772**. A matching attains the dual
sum; every edge inequality was checked with integers. Cancellation can lower
the actual resultant degree further, but the proposed candidate's degree
alone is not a bound on the unknown resultant.

This improves the bound by only 1.2%, so no production implementation or
change to the running grid was made. The metadata-only Python/SciPy pilot
proposed the combinatorial certificate; it performed no polynomial algebra.
The complete parsed coefficient-degree profiles, matching and exact integer
dual are retained in
`artifacts/research/a377-sylvester-support-bound-01/report.json`, SHA-256
`79282107b2a64a73904470bbbd2b15ddda8981f7e4984afaaa1a2a1f42b85216`.
This support bound does not itself verify the discriminant or any CAD cell.

### Degree-32 candidate and interrupted independent grid

The wider research computation returned a signed candidate after
**1538.878302 seconds**, using **1,364,132 KiB peak RSS**. It has degree
4058 in `s` and 3261 terms. The immutable candidate file is
`artifacts/research/a377-degree32-segmentation-02/discriminant.json`, SHA-256
`b798275222aa98597a4261224e714272d0d21dafcd805937e57b7397d337b2c1`.
The signed expression digest is
`e6cfd9d4634329847158eac82e4282db28474f4543c2887c13188c9f3ec57920`.
These measurements are for one research algebra call, not the original
decomposition or an upstream GCAD comparison.

The adapter exactly reconstructed the corresponding resultant candidate
`LC_t(p)*D`, with the positive degree-32 discriminant sign, before the
independent numeric Ducos grid began. That grid received the remaining
2043 seconds of the one-hour campaign. It timed out after 2043.032161
seconds, with the last atomic progress record showing **4608 / 4773**
checked points. No final verification report exists. Consequently this
candidate remains **independently unverified**; the partial grid supplies
no completed identity or CAD coverage credit.

The compute and grid supervisor digests are respectively
`def194b83514a3af1fae344c2df08772b66a663397caf3cc05b2bcbdb78ec3df` and
`8940f1a68025fa85f8bcbbdeffd8200577f5b7d6c30464985169cff1a9122077`.
The final partial-progress digest is
`8b6668c5ab32259c2af4773649ad2875f16095d921bfbfdb59e424549f617ac7`.
The candidate and all failed/interrupted evidence remain unchanged.

A separate verification-only campaign was launched on CPU9 with 4 GiB and
a fresh one-hour limit. It rechecks the entire grid from the immutable
derived inputs; it does not turn the incomplete counter into a certificate.
Its directory is `artifacts/research/a377-degree32-verification-03/`, and
the launch record has SHA-256
`f3920c504331dc75e990b8aeccb1ac4abf595dd30599ddcb605fb1cec5ecd826`.
The preparation retains a corrected metadata assertion that initially
confused the capture file digest with its distinct operand-array digest;
that failed check launched no worker. No production envelope or cache
contents were changed by these research runs.

### Fourth-continuation outcome

The first original-problem run with typed first-axis support bases ended after
**3600.029138 seconds**, at **792,584 KiB peak RSS**, with no result or
verification file. Its final cache has **2,056 completed records**
(194,141,514 bytes): all 2,048 inherited files remain byte-identical, and eight
new discriminants were retained. Their recorded operation time totals
356.207048 seconds, dominated by one `t`-discriminant at 356.142056 seconds
(input hash `5b861c651da97a5c6ca859ceba31834ac30a1e8de2c491fd85e5ba1924650b83`).
These operation timings exclude all other solver work.

The last incomplete capture is byte-identical to warm03's degree-32
`t`-discriminant: 2,502 terms, degrees `(s,t)=(84,32)`, 691-bit coefficients,
operand hash
`99f94af85a41a5dd07ae6f18f51b78b7b041d3c5ed47007e8179107f320b2de7`.
No completed native cache record exists for that request. This run does not
establish a completed-solve benefit or regression for the support-basis mode;
the same upstream algebra obligation remains pending. The separately
[grid-verified discriminant](A377_D32_GRID_VERIFICATION.md) remains research
evidence and was not inserted into this cache.

The metadata-only final audit rechecked the original mathematics, order,
anchor, trial options, frozen executable, every native tuple key and output
checksum, and all inherited hashes. It did not repeat native algebra.
Evidence in `artifacts/research/a377-segmentation-warm-hour-04/` is:

- `final-audit.json`: `1c48d084d4fa368b0322c487d8096600af0ce73005ae93991791c3d505a83953`.
- `final-cache-manifest.json`: `7387502aab2db9298019ef9c452761b9e627bae4dde0251db1114df1fd492c59`.
- `final-capture.json`: `bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75`.
- `results/ledger.json`: `4a1b9700f88226146731ffbd9427a028c5ffd71eb3f56c22725c1ec352b3193b`.

All completed work is preserved. No new solve was launched by this audit;
original verified coverage remains **303 / 331**.
