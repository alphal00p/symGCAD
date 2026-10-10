# Restricting root isolation to a certified positive axis

This investigation implements an isolated, replayable Symbolica API experiment.
It is not a production integration or an original-problem completion. The
published solver's root selection and exported ordinals remain unchanged.

## Measured reason to investigate

The completed native inventory of the degree-1478 A377 block has 294 real
roots and took 1138.558141164 seconds. Of its saved intervals, 149 have
strictly negative upper endpoints, 144 have strictly positive lower
endpoints, and one is `(0,8192/47009388797)`. Exactly 96 intervals lie above
4. Under the saved strict isolating-interval contract, the interval with
left endpoint zero also isolates a strictly positive root: 145 roots are
positive. This uses the exact endpoint contract, not a floating-point sign
inference or a claim that zero is a root.

The source report is
`artifacts/research/a377-d32-block-roots-01/block-002.json`, SHA256
`ddc4bc009730e017b5d04cea3fa46ef5ad8f24c4e201b6c149fb61280c7f679a`.
These counts describe one completed block inventory, not a completed CAD.
The symbolic input requires `s>4`, so its negative-axis roots do not bound
an admissible first-axis interval.

At pinned Symbolica revision `75f8350094b90254ee71dc2a391fde0d14b0204a`,
`UnivariatePolynomial<IntegerRing>::isolate_real_root_intervals` performs
square-free factorization and calls its private positive-ray isolator on
`[f(-x), f(x)]`, in that order. Consequently the first captured expensive
Taylor shift in the [observer experiment](FAST_TAYLOR_SHIFT_RESEARCH.md)
belongs to the negative-root branch of that block. It is an internal VAS
operand, not the original positive-axis polynomial. The 83.7 seconds of
observed completed shifts do not establish the negative branch's fraction
of the earlier complete 1138-second run.

`src/roots.rs` currently requests the full native real inventory. Several
positive-ray paths in `src/local_projection.rs` likewise isolate all roots
before filtering. This identifies possible unnecessary work when the
entire rejected half-axis has an independently certified domain exclusion.

## Isolated backend experiment

The [replay package](research/positive_axis_api/README.md) supplies two separate
patches against the pinned revision. The positive-axis patch exposes integer
and rational APIs using Symbolica's existing square-free and positive-ray
isolation, preserving multiplicities and the native cross-factor refinement.
It omits negative roots and the exact zero root. The independent unit-shift
patch replaces multiplication by one with exact addition in the existing
triangular Taylor recurrence; other shifts retain the original path. Neither
patch is applied to the normal dependency checkout. No floating-point root
recovery or alternate algebra backend is used.

The final comparison helper passed all **11 focused tests**, including the
shared exact integer-sign regressions. The unit-shift adapter passed both
integer/rational regressions, comparing full coefficients against the old
fused recurrence and checking exact evaluations. The same patched backend
also passed all 18 production root tests. These are separate recorded gates;
the production fast-suite result is documented in
[the final checkpoint](FINAL_CHECKPOINT.md).

Three initial comparison tests failed because the helper incorrectly treated
every nonsingleton native interval as open. The corrected checking path uses
the existing Symbolica-based exact open-root counter and the source's full
or positive ordinal when an endpoint is a zero. It normalizes before positive
filtering, then proves a common root by an exact rational value or a
sign-changing intersection. Interval overlap alone remains insufficient.
Square-free preparation, endpoint normalization and matching are outside the
native isolation timer and have separate timing evidence.

A preserved diagnostic uses the same raw `x^3-x` inventories: full-real
positive-root bounds `[1/2,1]` and positive-only bounds `[0,2]`. The old checker
rejects them; the corrected checker proves that both select exactly 1, with
multiplicity 1. Its independent audit is
`artifacts/research/positive-axis-api-01/endpoint-normalization-diagnostic-01/`.
The tentative native containment repair remains unapplied; these failures
required a checker correction, not a relaxed test or a backend change.

The replay setup was tested from a fresh clean clone. Both patches apply
independently, and applying both reproduces the frozen experiment's three
native source files byte for byte. The package pins the Cargo inputs and
shared root-checking sources, initializes isolated Git metadata, and leaves
the active dependency pristine. Failed build and test logs remain retained
alongside the successful gates.

## Fresh full-real control and measurement protocol

The new unmodified-arithmetic control independently reconstructed the signed
source product and completed the degree-1478 block in **1138.549870559 s**.
All **294** intervals exactly match the historical inventory: 149 negative,
145 positive and 96 above 4. The fresh product replay took 45.500059069 s;
the whole worker took 1184.540818838 s with 821288 KiB peak RSS. This control
uses the earlier observer-capable binary with observation disabled. Its
timer includes symGCAD's retained-fiber and interval preparation around the
native full-real call, so it is not directly interchangeable with the narrower
API timer below. Its memory includes that wrapper as well.

Control output SHA256:
`b9e11d22bceb2b4f4bdd6208f6db5a24db8306f86e6555fc7c54fcc9fe65946a`.
The independent audit SHA256 is
`da24ce0c5b620dfa671912a3364b5ebc378d563b6aa9bb20ec5cbb362659c735`.
Both are under
`artifacts/research/positive-axis-api-01/native-full-baseline-01/`.

The combined-patch full and positive API runs use the same frozen helper,
rational coefficient API, input, CPU 2 and 4 GiB worker allowance. Each gets
at most 1800 seconds in a fresh process, and they run sequentially. Native
dependencies use optimization level 2; this is an exploratory one-run
comparison, not a three-repeat release CAD benchmark. The exact input SHA256
is `4fa804c83098bf885ff26f11a301b3645f4aa38d4c2eb901ee341e80a0d7a058`;
the frozen helper SHA256 is
`25b4872636fbdcc8e078642983ac29daa7cb1453fd0e84bc42051481b33e6e86`.
The checks compare all real roots against the control, then all positive
roots and native multiplicities between the two new API runs. The historical
control does not save internal root multiplicities, so that first comparison
cannot independently confirm them.

The patched full-real API completed in **765.807915240 s**, returning 294
roots (149 negative, 145 positive), all with native multiplicity one. Its
whole worker took 766.624508318 s with 832592 KiB peak RSS. Exact ordered
comparison against the fresh control proved all **294 matches** by opposite
nonzero signs on rational intersections contained in both source intervals.
No interval refinement, endpoint ownership correction or square-free GCD
was needed for this operand. Comparison used 34.577002046 s in a separate
process, including 22.085688992 s of endpoint checks and 11.734336565 s of
matching. It is excluded from the API timing.

The full API report SHA256 is
`aaa79aebbdc4eb8aa4d048789373a165acaaae37189064cfdff3267904139319`;
the exact comparison report SHA256 is
`6b69261278cf8bcf443169542cd1b68b37a061f891971be454ebb96b4f31c6bc`.
They are retained under `patched-full-01/` and `comparison-full-01/` in the
same experiment directory. The lower time than the old control is descriptive:
the two timers have different wrapper scopes, so their ratio does not isolate
the unit-shift patch's benefit. The coefficient-level unit-shift measurement
has its own matched timing in the Taylor-shift research note.

## Completed positive-axis comparison

Both new API measurements completed using the same combined-patch binary:

| Native isolation mode | Roots returned | API time (s) | Whole worker (s) | Peak worker RSS (MiB) |
| --- | ---: | ---: | ---: | ---: |
| All real roots | 294 | 765.807915240 | 766.624508318 | 813.078125 |
| Strictly positive roots | 145 | 557.982648830 | 558.754017246 | 421.195313 |

On this operand, positive-only isolation used **0.73 times the full API time**
(27.1% less) and **48.2% less peak worker RAM**. This is a single-run result
for one root-isolation workload, not a three-repeat median or an end-to-end
CAD speedup. Both runs already include the unit-shift patch; this comparison
measures the additional effect of selecting only the positive axis.

The exact comparison matched all **145 positive roots and their native
multiplicities**, each equal to one. Every ordered witness was a
sign-changing rational intersection contained in both isolating intervals.
It needed no refinement, endpoint ownership correction or square-free GCD.
The separate comparison process took 20.528488159 s with 29640 KiB peak RSS;
endpoint checks took 14.669611379 s and matching took 5.116726370 s. These
checks are outside the isolation timings. They rely on Symbolica's certified
unique-root interval contract, rather than constituting an independent
degree-1478 root count.

Positive output SHA256:
`b5889499d50d240ad35ca3a63cb78075d9fde8ec0ef6c563f99c210a19674cc0`.
Exact comparison SHA256:
`5c30b6cdb7243af4004e0ee497d22177448078b32df887c0f59d79a18140a900`.
Independent witness audit SHA256:
`83cc478acec3e96d8b197a43b45e0841757772750a50bacd9cadbf0cb1115e14`.
These are retained under `patched-positive-01/` and
`comparison-positive-01/` in the same experiment directory. All input,
executable and patch bindings remained unchanged throughout both runs.

The bounded investigation is complete. The replayable API is promising for
domains with a certified positive axis, but production integration remains
unimplemented: it requires the following explicit changes to coverage and
root-index evidence. No original corpus problem gains completion credit.

## Production proof obligations

Backend availability alone does not justify changing an exported root index.
Existing ascending full-real indices count the omitted negative roots. A
positive-only path must use explicitly positive indices, or prove an exact
full-real offset; it must never copy an ascending full-real index unchanged.
Alternatively, `RealDescending` ordinals for positive roots are unchanged
when the entire positive inventory is retained, because omitted roots lie
below them. A rational root at zero still requires explicit handling. Existing
`RootIndexDomain::Positive` is a useful interface but is not itself a
delineability theorem.

Admission must prove that the omitted half-axis violates the original domain
throughout the relevant parent cell. Lifting evidence must record its exact
rejection, and verification must check that rejection plus every remaining
positive interval. A sample's positivity does not prove a domain bound.
Endpoint, leading-coefficient, nullification and positive-root collision
obligations still apply; a root crossing zero may change the positive ordinal.
Composite factors, shared roots and degree drops need the same exact handling
as the current union. Preserve all interior positive `F=0` boundaries.

The first-axis support-block proof currently records a full-real inventory;
changing it requires a new explicit proof contract and checker. Likewise,
skipping all roots below 4 would require a translated or interval-restricted
selector with verified transport. A positive-only API does not supply that
stronger change automatically. Keep the faithful baseline's behavior and
compatibility evidence intact, and measure the new path as a separate
optimization before promoting it.
