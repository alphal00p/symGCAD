# A377 degree-32 discriminant: content and support-basis limits

The authenticated discriminant from the warm A377 operation remains too large for the default-off first-axis support-basis prototype even after exact primitive normalization. The separate [completed 4,773-image grid](A377_D32_GRID_VERIFICATION.md) now proves its signed discriminant identity. The content/SQF measurements below do not themselves establish that identity or an A377 decomposition. No production limits or cache entries were changed.

The capture has full map `[s,t,x0,x3,x5,x6,x8]`, eliminated axis `t`, and input digest `99f94af85a41a5dd07ae6f18f51b78b7b041d3c5ed47007e8179107f320b2de7`. The candidate has degree 4058 in `s`, 3261 terms and minimum exponent 798. Scalar normalization retains that coordinate multiplicity.

| Exact measurement | Signed candidate | Primitive positive-leading candidate |
|---|---:|---:|
| Maximum numerator bits | 40,546 | 33,667 |
| Sum of numerator and denominator bits | 125,127,153 | 102,695,453 |
| Sparse expression bytes | 37,701,130 | 30,948,511 |
| Degree / terms / minimum exponent | 4058 / 3261 / 798 | 4058 / 3261 / 798 |

The extracted scalar is positive, integral and 6879 bits. Symbolica checked the complete signed identity `raw = scalar * primitive`. Its scalar-text SHA-256 is `c92d2a161bb1484151ebc8b704ab7b593f66efcef32dfc6929194dff25bb49a5`. The canonical expression SHA-256 is `bf0c0e1754161e436781472c04e304ca7fcbaf290f175ca628fb5c5a1e9ecdb5`.

The current production source limits are 20,000 bits per coefficient and 64,000,000 total coefficient bits. Both representations exceed them. Canonical text fits the separate 32 MiB evidence limit; that does not override the coefficient limits. Source-plus-block retention and product bounds are additional obligations, not implied by source admission.

The development helper completed on CPU 10 with a 60-second / 4-GiB worker limit: 3.685263 seconds process wall and 127,888 KiB peak RSS. Recorded stages were 2.217256 seconds sparse parse, 0.339217 canonicalization, 0.082817 signed reconstruction and 0.788666 metadata. These are one diagnostic run, not a solver benchmark.

The separate native SQF-only probe **timed out after 300.059206 seconds**, with 211,468 KiB peak RSS. Crucially, native SQF **returned in 293.532938 seconds**; the final atomic progress phase is `factor_reconstruction`. The timeout interrupted complete product replay, and no factor list or checked decomposition was exported. An earlier version of this note misread that phase; the unchanged progress SHA-256 `8a15e741…` records the correction. The probe used the whole canonical candidate, including `s^798`, in a fresh process. No irreducible factoring, root isolation or second backend followed. This outcome motivates preserving bounded native proposals before replay, not a claim of SQF failure or permission to enlarge production caps.

Read-only native-source follow-up rules out a redundant manual `s^798` predeflation experiment. Rational-field SQF removes scalar content and delegates to integer SQF; the latter calls `remove_common_monomial` **before** separability and Yun's derivative/GCD step, preserving each removed coordinate factor and multiplicity. For this input the nontrivial SQF path therefore already operates on degree `4058-798=3260`. The relevant `src/poly/factor.rs` entry points are `square_free_factorization` for `RationalField` (line 4928), the `IntegerRing` implementation (4642), `remove_common_monomial` (4089) and `square_free_factorization_0_char` (2995), in inspected Symbolica revision `75f8350094b90254ee71dc2a391fde0d14b0204a`; file SHA-256 `d5eaea5bc76e04aa29a0f481f94ba43580230d314fb0b182faebd9fe469e3fa1`. The native-stage timer does not attribute its 293.532938 seconds among these internal operations. No further run was launched from this source inspection.

## A repaired independent-checker admission limitation

`ProjectionCache::discriminant` returns the **canonical** discriminant to `Builder::guard_basis`. Independent verification freshly computes the **signed** discriminant. At the time of this measurement, `Checker::guard_basis` applied `base_univariate::source_cost(raw)` before canonical lookup in its validated source registry.

An operand could therefore have an admissible canonical source but an over-cap signed scalar multiple. The builder could use its typed composite blocks while the checker instead invoked ordinary irreducible factoring and failed to match those block IDs. This was a conservative verification/performance limitation, not an unsound acceptance. It was not the deciding issue for this D32 candidate: even its canonical source fails the existing limits.

For example, `2^20001*(s^2-1)` exceeds the signed coefficient cap while its canonical representative `s^2-1` fits. If the registry contains only the composite support block, ordinary replay asking for the irreducible IDs `s-1` and `s+1` cannot use that record. This example explains the code-path distinction; it is not a measured corpus failure.

The separately reviewed fix `0374dde` now checks exact rational proportionality against already validated sources with identical support and full map. It preserves the ordinary fallback for other guards, discovery caps, source-product validation and every root/sector obligation. Direct checker regressions include positive/negative large numerator and denominator units, nearby nonproportional inputs, changed support/maps and a composite-only registry. This is a checker compatibility repair; it neither enlarges D32 admission limits nor removes any zeros.

## Research helper contract and provenance

[`examples/profile_univariate_content.rs`](../examples/profile_univariate_content.rs) checks both caller-supplied whole-file SHA-256 digests, the capture's operand digest, the candidate's operation/input binding and its signed expression digest. It validates the complete sparse literal grammar before constructing an Algebra or parsing coefficients. No general polynomial expression is accepted in the candidate field.

The research envelope is 64 MiB per input, at most 4096 terms and degree 4096, 64 map axes, 45,000 bits per input numerator/denominator and 140,000,000 total input coefficient bits. Decimal-literal bounds and aggregate denominator bounds are checked before rational construction. The actual full exponent map is retained, including inactive axes. Content-only mode performs no square-free decomposition, irreducible factoring or root isolation.

The separately authorized `--stage square-free` mode retains the scalar and every coordinate power, calls native SQF only on the whole canonical input, and requires complete canonical and signed product reconstruction. Native output is bounded before powers: at most 64 blocks, positive multiplicities at most 4096, exact full-map first-axis support, integer coefficients at most 45,000 bits, 140,000,000 retained block bits, weighted degree equal to the source degree, product-height bound 65,536 and `(degree+1)*height_bound <= 256,000,000`. Its maximum worker limit is 300 seconds / 4 GiB. Static or runtime failure is a failed/incomplete research result, never permission to try a second backend. No irreducible factoring or roots are called.

Seven focused helper tests cover signed rational scaling, repeated coordinate factors, full maps and constants, coefficient/term/byte/denominator preflight, native-output work bounds, product/multiplicity tampering and authenticated input tampering. Test log `artifacts/univariate-content-helper-tests-03.log` has SHA-256 `23831a37941f02e77fdf2b56eb8a41eec46b5dd0758cf9bac66d044b501f5eab`.

The initial content-only run is preserved under `artifacts/research/a377-degree32-content-01/`:

- Candidate JSON: `b798275222aa98597a4261224e714272d0d21dafcd805937e57b7397d337b2c1`.
- Capture JSON: `bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75`.
- Frozen development helper: `f4a0efb55ed8fef856de66de5e35aff46735dc6347062a835e0a81f549c51653`.
- Frozen helper source: `e51a73647810302f9c6b43b793c2412a7ab6798ad250ed4a4081c897705a0119`.
- Completed report: `bdb1904db68af85968cb522363342e9f7ba5008be9a88504b86da2908927b2ed`.
- Exact command script: `59e5805a50ee64fa76c539502573cd31fdf8ed0172afccfc23402aa72afd41f7`.

The separate SQF attempt is under `artifacts/research/a377-degree32-content-02/`, with the same authenticated capture/candidate and CPU 10 / 4-GiB limit:

- Frozen development helper: `eb8186f517218ecc14aea9acf72db7bce07a7535c79fb8d6d7a2d54ace761b94`.
- Frozen helper source: `5dc12fbb2c0defe76c146eb46b40f20299cee551cae76eb5e303c1d9c2b58433`.
- Timeout report: `250867950a943bb16869c9d5f5b1255fea6c021fd77797878a22e9a7e7bbab28`.
- Final progress: `8a15e741571bf773cbf4d5878d3d9ac3bd61ed2c1f25f2af954986ae7c787253`.
- Exact command script: `eb7db04e4670d78fc3db29056235c93799671ebaf92693ce8a80b43b47b721a2`.

For available ignored captures, build the helper with `nix develop -c cargo build --locked --example profile_univariate_content`, then use the preserved command's two expected SHA-256 arguments and a fresh `--output` path. Content mode defaults to 60 seconds; the explicit `--stage square-free --seconds 300` invocation is separately bounded. Ordinary test reproduction is `nix develop -c cargo test --locked --example profile_univariate_content` and requires no captured input.

Large candidates and captured operands remain ignored artifacts. Reproduction requires those authenticated files; they are not ordinary test dependencies. The helper always reports `discriminant_verified:false`: scalar/SQF identities establish properties of the supplied candidate only. The initial 4,773-image grid attempt timed out at its last published count of 4,608; that partial result remains incomplete. The fresh [verification-only rerun](A377_D32_GRID_VERIFICATION.md) subsequently completed all 4,773 valid images and supplies the independent discriminant certificate.

## Independent discriminant validation remains a separate obligation

The frozen `prepare_discriminant_grid` adapter checks the full map and hashes, forms `q=∂p/∂t`, and constructs `R_candidate = (-1)^(n(n-1)/2) lc_t(p) D_candidate`. Here `n=32`, so the sign is positive. Exact leading-coefficient division reconstructs the supplied signed candidate; this identity alone does not establish that either candidate is the true resultant/discriminant.

The independent grid evaluates ordinary numeric Ducos resultants at distinct rational `s` values, excludes degree-drop images for either input, and compares the complete signed values. The frozen rectangular/weighted support bound is 4772 for the resultant, requiring 4773 valid images. Candidate degrees 4058 (`D`) and 4118 (`R`) do not themselves bound the unknown true resultant. A separately proved sharper bound could justify a different complete grid, but an unfinished report must not be relabeled as its verification.

After a complete grid identity, cancellation of the nonzero polynomial `lc_t(p)` proves the signed discriminant identity globally, including its leading-coefficient-zero specializations. It does not remove any discriminant zeros or authorize omission of their CAD seams. Factor/SQF identities, root inventories and original coverage remain distinct later obligations.
