# A377 D32: bounded known-factor screening

The two substantial grouped square-free blocks from the earlier D24 discriminant, of degrees **881 and 111**, do **not** divide the new D32 discriminant in full. Native finite-field remainders are nonzero at three valid primes. This does not establish coprimality: either composite block could still share a proper factor with D32. No factor or interior boundary was removed, and no original CAD was completed.

The D32 signed identity is independently established by the [completed grid verification](A377_D32_GRID_VERIFICATION.md). Its raw degree is 4058, with 3261 terms and full variable map `[s,t,x0,x3,x5,x6,x8]`. Its content and the completed native-SQF stage followed by interrupted product replay are recorded in [the content note](A377_D32_CONTENT_AND_BASIS_LIMITS.md). This screen does not repeat SQF or perform exact rational polynomial division.

## Available factors and bounded inventory

A metadata-only pass over the immutable warm03 final manifest examined all 2048 headers and parsed 1486 of 1487 factor records: 46,030,710 bytes in 25.780 seconds. The pass imposed 30 seconds, 64 MiB cumulative data and 8 MiB per record, verifying every parsed file against the manifest's SHA-256 and size. The one excluded 10.5 MiB record was separately inspected under a 16 MiB bound and matched its manifest hash. Its large factor is bivariate; its small s-only factors were already present.

There are **215 distinct s-only factor strings**, degrees 1–32, total degree 954 and only 44,200 expression bytes. These are algebra proposals, not transferred projection, domain or coverage evidence. The checked D24 sidecar additionally supplies grouped blocks of degrees 881, 111, 10, 4, 2 and linear factors. Exact polynomial deduplication produces 220 proposals. The earlier D24 multiplicities are provenance, never assumed to be D32 multiplicities.

The immutable manifest is `artifacts/research/a377-segmentation-warm-hour-03/final-cache-manifest.json`, SHA-256 `053435cb2d173274bdf845c9ffd5005b6eef6ba9a9239ff5644b5a23dbfe253e`. Bounded metadata code/results are in `artifacts/research/a377-known-factor-inventory-01/`; `inventory.json` has SHA-256 `cae328b087aa37214cc46ed8db3b1b65ac68cf635d0a16960d73eed35bf29b13`. No divisibility arithmetic was performed by that inventory.

## Screen and outcomes

[`examples/profile_known_univariate_factors.rs`](../examples/profile_known_univariate_factors.rs) uses Symbolica rational parsing and native finite-field polynomial operations. It hashes the complete candidate, capture, inventory and sidecar before use; checks the capture's operand digest and signed candidate expression; preflights the complete sparse literal grammar; and retains the original full exponent map. Proposals are deduplicated by exact polynomial equality. The native iterator supplies primes `1073741827`, `1073741831`, `1073741833`.

A prime is inconclusive if a coefficient denominator vanishes or either leading degree drops. At a valid prime, nonzero remainder disproves rational divisibility: inversion of the divisor's nonzero leading coefficient makes any rational quotient integral at that prime. Conversely, zero remainders—even at all three primes—are **not** exact divisibility certificates. No coefficient-size argument reconstructs a zero rational remainder here.

The first development-build screen used native dense univariate remainders. It timed out at **120.031025 seconds**, 101,572 KiB RSS, after 120 of 220 proposals: 119 rejected and one modular survivor, `9*s-10`. Its partial progress remains explicitly incomplete; the large D24 blocks had not been reached.

A separate, authorized five-proposal run used native sparse multivariate division with only `s` active. It retained the intact authenticated input inventories but selected D24 indices 0, 1, 3, 4 plus the earlier linear survivor. It completed in **7.337441 seconds**, 101,844 KiB RSS; preparation was approximately 3 seconds and modular work 4.171151 seconds. All 15 images were valid.

| Proposed divisor | Remainder degrees at the three primes | Conclusion |
| --- | --- | --- |
| `s` | zero / zero / zero | Survives; no new multiplicity measurement |
| `s-4` | 0 / 0 / 0, nonzero constants | Not a divisor |
| D24 degree-111 block | 110 / 110 / 110 | Whole block not a divisor |
| D24 degree-881 block | 880 / 880 / 880 | Whole block not a divisor |
| `9*s-10` | zero / zero / zero | Requires exact rational division |

The linear survivor's root `10/9` lies below the original `s>4` domain. Its global factor and multiplicity would still be retained in any later exact factorization. The already known `s^798` coordinate factor gives no new deflation benefit; native SQF already removes coordinate monomials. A future modular GCD could test for proper shared factors of the composite blocks, but none was computed here. Completed SQF/product replay is currently the more direct next step.

## Dense remainder cost: specific API, not an SQF diagnosis

In the inspected Symbolica source, `src/poly/univariate.rs:1119`, generic `UnivariatePolynomial::quot_rem_impl` repeatedly executes `r = r - div.mul_exp(n-m).mul_coeff(...)`. This constructs shifted dense vectors and subtracts full-length vectors even for a linear divisor. The degree-product estimate saved as `dense_work_upper_bound` in the first report does **not** bound those extra allocations; it must not be read as an actual operation count. The revised helper labels it a degree-product admission budget and retains the external process deadline and memory bound.

The specialized integer modular-GCD implementation is different: `DenseZp64UnivariateGcdImage::remainder`, `src/poly/gcd.rs:4220`, updates the coefficient buffer in place and reuses the leading inverse. This screen therefore supplies no evidence that generic dense allocation explains the 293.533-second native SQF stage. No dependency patch or general SQF speed claim follows.

A possible next measurement, **not run**, is a release-build microprofile of the same five authentic divisors and three images, with exact dense/sparse remainder equality checked before timing, alternating strategy order, and a 60-second/4-GiB process cap. Conversion, remainder and report costs should be separate. The two present runs differ in selection and implementation and are not a matched speed comparison.

## Reproduction and provenance

The parser admits at most 64 MiB per input, 4096 terms/degree, 45,000 coefficient bits and 140,000,000 total candidate coefficient bits. Proposals are capped at 256 entries, 2 MiB combined expression text and 16,000,000 retained coefficient bits. Names, sparse ordering and denominator bounds are checked explicitly. The worker is killable, at most 120 seconds/4 GiB, and requires fresh report/progress paths. Static failure or timeout earns no completed screen. Current source uses native sparse remainders; the original dense executable remains frozen only in ignored artifacts.

Eight tests cover signed rational scaling and repeated factors, modular false positives versus rejection, bad denominator and degree-drop primes, grammar/height/support/map caps, inventory and signed-input tampering, exact native dense/sparse equality, and explicit narrow selection. Run `nix develop -c cargo test --locked --example profile_known_univariate_factors`; no captured files are needed for tests.

Both actual probes used CPU10 and development builds with dependency optimization level 2. Commands, frozen executables, source/build hashes and reports are preserved under `artifacts/research/a377-known-factor-screen-01/` and `...-02/`. To reproduce with authenticated captures, build the example and follow `...-02/run.sh`, supplying the four expected file hashes, a fresh output path and `--selection d24-four-and-linear --seconds 30`.

| Evidence | SHA-256 |
| --- | --- |
| D32 candidate JSON | `b798275222aa98597a4261224e714272d0d21dafcd805937e57b7397d337b2c1` |
| D32 capture JSON | `bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75` |
| D24 checked sidecar | `e83563f9721a38ebf3de7fa9ff95ff2c28adb0e0056a6151d81540bcd5f20e5c` |
| Original dense timeout report | `626e800c470c57f31e3fad4fc13f0bdf0731efd9bace83a8dcbb68de54887faf` |
| Original dense partial progress | `281736f265a61eee3730d86c492836c2183fd3b721848808d250098815b143a8` |
| Completed five-proposal report | `5ccc9470c06375addb52595ee7850c8ad9f6d7f031c68264c44b42bc8dd4a0e7` |
| Frozen sparse executable | `442ac3f7d58d61bece0bc6c6ace5f501689db0a9d5debbe77dd7985ff41b7e1c` |
| Sparse helper source | `c6ee7db348e381f58b0f1fd0c75254177f520fcd29db1ecae6a721fc39020354` |
| Eight-test/build log | `a89e234ea7de5e79b1a163e25e58ed60afe7f7f340e30f062957f3caab77a731` |

The inspected generic univariate source has SHA-256 `5e828a92ff501724607b064a004f2c8e8843c7d20776505b9c266d739eeda104`. No caches, production policies, factor multiplicities, root inventories or original mathematical inputs were changed by this experiment.
