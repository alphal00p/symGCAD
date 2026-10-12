# Symbolica patches for escalation

The separate [main-axis experiment](../reference/GLUZA33_FINAL_FACTOR_PUSH.md)
adds `experimental/symbolica-bounded-main-axis.patch` after the bounded-Wang
patch. It changes the factorization variable heuristic for bounded sparse
three-variable inputs, retaining exact backend acceptance checks. The note
provides isolated setup, signed reconstruction controls and default-off flags.

The separate [bounded Wang experiment](../reference/GLUZA33_WANG_PREFLIGHT.md)
provides two optional patches under `experimental/`: a factorization policy and
an unrelated test-feature fix. They are **not** applied by bootstrap or enabled
in production. The linked note gives isolated setup, feature/environment flags,
exact regression evidence and matched measurements.

Fetch the pinned dependencies and apply the three fixes with
`scripts/bootstrap-deps.sh --apply-patches`. Repeating that command is safe;
`scripts/bootstrap-deps.sh --check --apply-patches` verifies them without
changing files. The default bootstrap leaves sources pristine, matching the
recorded release benchmarks. Numerica lives in the pinned Symbolica monorepo.
See [setup instructions](../docs/SETUP.md) for exact revisions and prerequisites.
Cargo's ordinary public pins do not use these optional checkouts. For a modified
symGCAD build, select all three local packages in an isolated working copy with
[`scripts/use-local-symbolica.sh`](../scripts/use-local-symbolica.sh), then
refresh its lock through `cargo metadata` as described in setup.

`symbolica-linear-resultant-sign.patch` targets the local Symbolica revision
`75f8350094b90254ee71dc2a391fde0d14b0204a` (`symbolica-v3.0.1-1-g75f83500`).
It is **not applied** to the reference checkout.

The homogeneous Horner formula in `resultant_with_linear_second` currently
returns `b1^degree(f) * f(-b0/b1)`. This is `Res(b0+b1*x, f)`; the advertised
`Res(f, b0+b1*x)` requires an additional `(-1)^degree(f)` factor.
For example, `Res(x^3+y, x+2)` should be `8-y`, but this revision returns `y-8`.
The patch fixes that sign, corrects the existing degree-seven expected value,
and adds independent cubic/linear and linear/linear determinant regressions
covering all public resultant strategies.

Check applicability from the Symbolica checkout with:

```sh
git apply --check /absolute/path/to/symGCAD/patches/symbolica-linear-resultant-sign.patch
```

After applying in a separate checkout, run:

```sh
cargo test --lib resultant_linear_operand_has_sylvester_sign
cargo test --lib resultant_small_formulas
```

symGCAD uses the signed linear identity directly in `projection::exact_resultant`
and otherwise calls deterministic integer Ducos. Its own scale/sign regression
tests this behavior without modifying Symbolica. Projection zero sets were
unaffected by the backend sign defect because scalar content and sign are
discarded; the wrapper also makes the public exact resultant convention correct.
This identity replaces the affected backend call rather than negating its
return value. Applying the patch therefore does not double-correct any
production path. The separate Kronecker research helper uses the same direct
identity for its integer linear operands.

The separate finding that `resultant_crt` can stop after probabilistic modular
checks is not patched here. Production projection and verification explicitly
avoid that method; it remains available only in the profiling example.

## Exact integer left shifts

`numerica-integer-left-shift.patch` targets the same Symbolica revision, in
`lib/numerica/src/domains/integer.rs`. It is **not applied** to the reference
checkout or linked into symGCAD's production binaries.

`Integer::shl_usize` currently interprets primitive `checked_shl` as an arithmetic
overflow check. Rust only checks whether the shift distance fits the primitive
width; nonzero high bits may be discarded and the sign may change. On the pinned
backend, `Integer::from(1) << 63usize` incorrectly gives
`-9223372036854775808`, instead of `9223372036854775808`. Both the `i64` and
`i128` representations have this defect, including the owned, borrowed and
assignment overloads that delegate to this method.

The patch retains a primitive result only when arithmetic right-shifting it
recovers the original signed value. Otherwise it promotes to the next exact
representation. This also preserves valid negative boundary results such as
`-1 << 63 == i64::MIN`; no overflowing primitive multiplication is needed.

The new regression compares 13 signed values across 14 shift distances and all
three overloads against the multiprecision implementation (546 comparisons).
It covers sign-bit crossings, discarded high bits, zero, `i64`/`i128` extrema,
and already-large values. In a separate copied numerica crate, the added test
failed on the original method at `1 << 63`, then all 18 integer-related tests
passed after the patch. Evidence is retained under
`artifacts/research/numerica-integer-shift-01/` in `before-regression.log`,
`after.log`, and `provenance.json`.

Check and apply **in a separate Symbolica checkout**, then run:

```sh
git apply --check /absolute/path/to/symGCAD/patches/numerica-integer-left-shift.patch
git apply /absolute/path/to/symGCAD/patches/numerica-integer-left-shift.patch
cargo test -p numerica --lib integer_left_shifts_preserve_value
cargo test -p numerica --lib integer_
```

A source audit found no direct affected shift in symGCAD's production solver.
The only `Integer` shifts in `src/` are two oversized-value regression fixtures,
which already take the multiprecision fallback. Numerica's integer sampling and
packed polynomial arithmetic use `MultiPrecisionInteger` for their large shifts;
the vulnerable Python float-sampling call is outside the Rust solver's features.
This is a scoped source audit, not a claim about every external user of numerica.
The research Kronecker resultant helper uses
`Integer::from(value.to_multi_prec() << bits)` explicitly; this workaround does
not require modifying the reference checkout.

## Ducos resultant with a vanishing subresultant

`symbolica-ducos-zero-subresultant.patch` targets the same revision, in
`src/poly/univariate.rs` and `src/poly/resultant.rs`. It applies to the pristine
checkout and on top of the two patches above. It is **not applied** to the
reference checkout or linked into symGCAD's production binaries; an upstream
PR still has to be opened from a Symbolica fork.

`UnivariatePolynomial::mul_exp` turns the zero polynomial into a vector of
explicit zero coefficients. In `ducos_next_subresultant`, a monomial regular
subresultant `S_e = c*x^e` makes every `H_j` zero, so the next subresultant
comes back as such an untruncated zero whenever it vanishes. The main Ducos
loop then misses `b.is_zero()`: debug builds panic with a subtraction overflow
at `a.degree() - b.degree()` (resultant.rs:172), release builds wrap and index
out of bounds (resultant.rs:375). A vanishing subresultant means a common
factor, so every affected call has resultant zero. The minimal case is
`Res(3x^5, -x^3-3x)`. The defect never produced a wrong nonzero value in the
review's 60,000 checks; it crashes instead. In symGCAD it reached production
through exact interpolation images, whose specialized operands share a root
at some sample points (2026-10-12 algebra-fixes review).

The patch makes `mul_exp` return the zero polynomial unchanged and truncates
the Ducos subresultant before returning it. It adds a regression covering
three univariate integer cases, a shared-factor product, a case with
polynomial coefficients, and a coprime control.

symGCAD does not depend on the patch: `resultant_segmentation::integer_resultant`
decides a vanishing integer resultant first, exactly (a degree-zero gcd modulo
a fixed prime that keeps both degrees proves coprimality, otherwise
Symbolica's integer gcd decides), and only calls Lazard-Ducos on coprime
operands, and `projection::exact_resultant_ducos` first proves that the
resultant does not vanish identically (coprime integer images at a full-degree
specialization, otherwise Symbolica's multivariate gcd) before calling Ducos.

Check and apply **in a separate Symbolica checkout**, then run:

```sh
git apply --check /absolute/path/to/symGCAD/patches/symbolica-ducos-zero-subresultant.patch
git apply /absolute/path/to/symGCAD/patches/symbolica-ducos-zero-subresultant.patch
cargo test --lib --no-default-features --features integer-gmp,float-mpfr ducos_resultant_of_operands_with_a_common_factor_is_zero
```

Without `native_code_generation`, the library test target additionally needs
the test-only `experimental/symbolica-factor-tests-without-native-codegen.patch`.
