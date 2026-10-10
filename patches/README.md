# Symbolica patches for escalation

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
