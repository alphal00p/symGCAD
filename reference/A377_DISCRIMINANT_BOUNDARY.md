# A377 discriminant: boundary factors do not establish nonvanishing

The exact degree-2169 discriminant generated from the captured A377 degree-24
t polynomial has at least two zeros in the original domain `s>4`. It cannot
be removed by a global strict-sign certificate. This is a result about the
projection polynomial; it does not establish a collision of relevant real t branches, a
singularity of the original F, or a completed decomposition.

The independent reconstruction checked all 2,480 required numeric Ducos grid
images using the conservative degree bound 2,479. Its adapter checked the
signed identity `Res_t(p,p') = lc_t(p)*D` (the degree-24 sign is positive),
without primitive-part normalization. The boundary probe then used Symbolica
integer/rational polynomial arithmetic only; it performed no factorization or
root isolation.

## Exact result

The supplied D has 2,030 terms, consecutive s exponents 140 through 2,169,
and coefficient height at most 15,757 bits. Removing the monomial factor and
applying native `UnivariatePolynomial::shift_var(4)` gives

```
D(s) = s^140 q(s),
q(u+4) = u^8 r(u),
D(s) = s^140 (s-4)^8 r(s-4),   deg(r) = 2021.
```

The probe checked the monomial reconstruction and exact inverse shift. The
first eight shifted coefficients vanish and the ninth does not, so the
boundary multiplicity is exactly eight. The remaining coefficients comprise
720 positive and 1,302 negative entries, which gives no coefficient-sign
certificate. More decisively, exact evaluations give:

| s | sign(q(s)) = sign(D(s)) |
|---|---|
| 9/2, 5, 6, 8, 10 | − |
| 16, 32 | + |
| 64, 128 | − |

Continuity therefore gives a root in each of `(10,16)` and `(32,64)`. Removing
the positive factors `s^140` and `(s-4)^8` on `s>4` preserves these interior
zeros. This refutes the proposed whole-domain nonvanishing route, rather than
merely failing to discover its certificate. The brackets are not isolating
intervals, and the total root count remains unknown.

## Existing workflow and next step

The current shifted-coefficient portfolio conservatively estimates 2,345,665
expanded contributions for raw `D(s+4)`, beyond its 20,000-term budget, although
the final univariate polynomial can contain at most 2,170 terms. Native dense
univariate shifting makes this particular diagnostic inexpensive, but the
measured sign changes provide no reason to raise production positivity caps.
The degree-64 Sturm portfolio also correctly declines this input.

Global optimized `prune_operations` can certify an entire raw projection
operation before factoring. The local visitor currently factors raw guards
before domain certificates, and its checker repeats that factorization.
Simply adding a producer-only pre-factor shortcut would therefore neither
provide the needed proof semantics nor eliminate verifier work. No such
change is made here.

If subsequent profiling needs a smaller exact operand, stripping the two
known domain-boundary factors reduces degree 2169 to 2021 and is justified by
the identities above. It does not justify discarding the residual or predict
that its factorization will be cheap. The original A377 domain leaves t
unrestricted, so a positive-ray-only argument cannot be substituted here.
Any more selective omission would have to classify whether the discriminant
zeros affect required real branches under the full local proof hypotheses.
No new full-case campaign follows from this probe.

## Bounded execution and evidence

`examples/profile_univariate_boundary.rs` accepts only bounded canonical
integer-univariate input: 16 MiB JSON, 4,096 terms/degree, 6,000 decimal digits
per coefficient, explicit nonnegative ordered exponent fields, and no expression
powers or arbitrary parser syntax. It bounds the shift operation count and
result coefficient storage before computation, and supervises one CPU worker
with 4 GiB and at most 60 seconds. It never supplies proof evidence to the
solver. Three focused tests passed, covering exact boundary identities, mixed
signs, parser rejection and startup-banner handling.

The clean CPU9 run took 1.836 seconds wall, 70,308 KiB peak RSS. Forward shift
took 0.572 seconds and inverse identity checking 0.567 seconds. These are
single development-build diagnostics, not end-to-end speed measurements.
The earlier `a377-discriminant-boundary-01` worker also succeeded, but its
supervisor rejected a Symbolica startup banner before the final JSON; that
failed supervisor record is retained unchanged. Version 2 handles the final
JSON line and requires successful exit with untruncated output.

| Evidence under `artifacts/research/` | SHA-256 |
|---|---|
| `a377-univariate-segmentation-01/discriminant.json` | `0c457cfce6ed2df4545054b7ba79b76e62123e284a665ea39db12bc4124c09c2` |
| `a377-univariate-segmentation-01/grid-verification.json` | `4c85072f4229b2f1cacb53fe49bb6679d8441381dabbee9467251bef1d00ab9b` |
| `a377-discriminant-boundary-02/report.json` | `1c472330f95305c61337c92189eda0d9a342e4b6ee048558aa544abda1c8f25d` |
| `a377-discriminant-boundary-02/profile_univariate_boundary` | `fd8df30b71cb089b594c36caab713bef26a544fb56d14ea2848d78d0c3ba6f2a` |

The adjacent `build.json` binds source files and test/build logs, candidate
and grid-preparation hashes. The exact D expression SHA-256 is
`ad2b39aac6b79964c9eed60e8681f11248991ffaf7dd1a8bfe4b9da2298b17cc`.
