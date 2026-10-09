# Symbolica patch for escalation

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
