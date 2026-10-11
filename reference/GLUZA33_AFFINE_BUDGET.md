# Gluza33: the next factor has regular evaluation fibers

The 600-second interval-witness continuation ended on a 2,410-term factor
operation. A bounded exact diagnostic shows that raising the affine source-term
cap alone would **not** activate the current preconditioner: the source has no
degree drops at the all-ones evaluation fiber. None of the fixed translation
candidates improves that count. Further exact checks find one square-free
block and no repeated roots in any all-ones univariate image. Consequently
neither diagnostic justifies a translated factor trial. A third bounded scout
finds stable low-degree factor partitions in exact univariate images, providing
a concrete lifting target. No production setting was changed, and no
multivariate irreducible factorization or original CAD completion is claimed.

The source is factor key
`5b9185fc073a9f5e3d402596fc271650fafa18cc8c9b9a01a206b3273e723066`,
with map `[x2,x1,x7,x4,x6,x3,x5]` and degrees `[20,18,8,0,0,0,0]`.
The frozen capture SHA-256 is
`aa44f4fb3da0145236f42254989933015ad6d6eda25cada5b49e65d83f60be43`.
It comes from the original positive Gluza33 continuation described by
`artifacts/research/gluza33-interval-production-01/frontier-agent/summary.json`.
The diagnostic concerns algebra-only preconditioning; it does not alter the
problem's coordinates or assume that a sampled parameter slice proves coverage.

## Research helper and measurement

`examples/profile_support_projection.rs` now accepts a research source budget
`--max-source-terms`, default 2,048 and bounded above by 8,192. The separate
`--admission-only` mode evaluates each active axis's leading coefficient after
specializing the other active variables. A zero value records a degree drop,
including specialization to the zero polynomial. This agrees with the existing
production heuristic without expanding translated polynomials.

For candidate `k`, the active axes receive shifts `k,2k,3k`. Thus their all-ones
fibers correspond to original points `(1+k,1+2k,1+3k)`. Every leading-coefficient
value in this table is nonzero; complete exact values are in `admission/worker.json`.

| Candidate | Original evaluation point | Degree-drop axes |
|---|---|---|
| Original | `(1,1,1)` | None |
| `k=1` | `(2,3,4)` | None |
| `k=-1` | `(0,-1,-2)` | None |
| `k=2` | `(3,5,7)` | None |

With source admission 4,096, the fresh supervised diagnostic completed in
**0.042962 seconds**, with **8,196 KiB** peak RSS. Exact degree-drop evaluation
took **0.001718 seconds**. It ran on CPU 8 with one worker, a 60-second wall
limit and a 4-GiB memory limit. Both stages use Symbolica revision
`75f8350094b90254ee71dc2a391fde0d14b0204a`, without the optional root-isolation
patches; the dependency checkout is clean.
The helper was a debug build. `first_improving_shift` is `null`.

There is consequently no justified translated candidate under the degree-drop
heuristic. The follow-up below also checks the distinct repeated-root criterion.

The helper also provides `--native-factor` for future matched controls and
retains exact inverse signed-factor reconstruction in translated mode. Both
use the same frozen helper/backend, preserve every variable-map entry, scalar
unit and multiplicity, and check each multiplication. Translation work is
bounded by 1,048,576 estimated terms per translation call, intermediate
polynomials by 20,000 terms, and inverse product work cumulatively by 8,000,000
term products. New admission, square-free-only and native/translated control
workers are killable within at most 60 seconds. Ordinary projection retains its
historical ceiling of 600 seconds, including existing 120-second replay recipes.
These research changes do not increase any production cap.

The initial 16 helper/parser tests pass, including a generic 3,267-term source admitted
only by an explicit larger research budget, invalid budgets and variable maps,
exact shifted leading-coefficient identities, negative shifts, scalar units,
multiplicities and rejection of corrupted reconstructed products.

## Square-free and full-fiber follow-up

The existing frozen `profile_multivariate_square_free` helper first declined
this operand before algebra because its source parser caps input at 2,048 terms.
That failed admission is preserved in `legacy-sqf-admission/`; it is not a
square-free factorization timeout.

The reviewed helper adds `--square-free-only`, stopping after a complete signed
product replay. It retains scalar units, multiplicities and the entire variable
map, checking the cumulative product budget at every multiplication. This
source produces **one 2,410-term block, multiplicity one**. The exact square-free
operation took **0.016090 seconds**, signed reconstruction **0.003103 seconds**;
the supervised process used **0.062955 seconds and 11,116 KiB**. No projection
was performed. Square-free decomposition does not assert irreducibility.

Separately, `--admission-only --fiber-gcds` constructs the exact univariate image
for each active axis at every fixed point and computes its gcd with its
derivative. Both old and candidate degrees must equal the source degree before
a reduction in gcd degree counts as an improvement. Zero fibers are explicitly
recorded and never qualify. The report preserves all exact image and gcd
coefficients for replay.

| Original point | Image degrees in `x2,x1,x7` | Gcd degrees | Improvement |
|---|---|---|---|
| `(1,1,1)` | `20,18,8` | `0,0,0` | Baseline |
| `(2,3,4)` | `20,18,8` | `0,0,0` | None |
| `(0,-1,-2)` | `20,18,8` | `0,5,3` | Worse |
| `(3,5,7)` | `20,18,8` | `0,0,0` | None |

The lowest-degree active axis is `x7`, the candidate used by Symbolica's
univariate specialization scout. Its all-ones image is already degree
preserving and square-free. The full-fiber diagnostic took **0.025960 seconds**;
the process used **0.062155 seconds and 11,448 KiB**. Both workers had the same
60-second, 4-GiB, CPU-8 limits and used the same frozen debug binary.

All **19** reviewed helper/parser tests pass. Added checks distinguish accidental
and structural repeated factors, ordinary and zero fibers, missing and duplicate
evaluation-map entries, inactive axes, exact translated coefficient identities,
signed square-free replay and historical projection deadline compatibility.
At this stage there were no native or translated irreducible-factor trials: neither exact
criterion improves at any tested shift. All interior boundaries remain intact.

## Bounded univariate factor inventory

The additional opt-in `--fiber-factors` requires both `--admission-only` and
`--fiber-gcds`. It factors only nonzero, degree-preserving, square-free
univariate images of degree at most 32, beginning with the lowest-degree axis.
There are at most twelve images: three axes and the four fixed points. Each
factor product is reconstructed exactly, including rational units and
multiplicities, before recording success. Partial evidence is saved before a
potential source-content operation.

An irreducible image would trigger an exact `univariate_content(axis)` check.
Only constant nonzero content, together with preserved degree and the
irreducible image, could certify the original source. The helper would stop at
the first such certificate. Tests reject the counterexample
`(y+1)*(x^2+y)` at `y=1`: its image is irreducible up to a scalar, but its
nonconstant content prevents a source certificate. Degree drops, repeated
images and scalar signed reconstruction are tested separately.

| Retained axis | Source degree | All-ones image | `k=1` image | `k=-1` image | `k=2` image |
|---|---:|---|---|---|---|
| `x7` | 8 | `2+6` | `2+6` | Repeated; declined | `2+6` |
| `x1` | 18 | `6+12` | `6+12` | Repeated; declined | `6+12` |
| `x2` | 20 | `6+14` | `6+14` | `6+14` | `6+14` |

The fresh process completed in **0.325167 seconds**, peaking at **12,156 KiB**,
under the same CPU-8, 60-second, 4-GiB limits. It factored and reconstructed
**ten** images; the two repeated images were declined, not counted as factored.
The sum of univariate factor calls was **0.124083 seconds**. No image was
irreducible, so **no source-content check or source irreducibility certificate**
was produced. All **21** final helper/parser tests pass.

The stable partitions suggest a candidate small factor with multidegree
`(6,6,2)` and a complementary factor with multidegree `(14,12,6)` in
`(x2,x1,x7)`. This is a hypothesis: matching factors between separate
specializations and lifting them has not been performed. It is more useful
evidence for a bounded lifting/recombination experiment than a larger affine
term cap. Symbolica's existing minimum-axis scout is conditional on the chosen
bivariate start, but its all-ones image here is reducible in any case.

## Existing backend start policies

Static leading-coefficient layers contain **25, 50 and 215 terms** in
`x2,x1,x7`. The pinned backend orders the axes by smallest leading layer and
then degree, yielding `[x2,x1,x7]`. Its two-axis box score is
`2410 / ((20+1)*(18+1)) = 6.0401002506`, above the bivariate-start threshold
of 5. The sparse univariate reordering also declines immediately above this
threshold. The minimum active degree is 8: the early degree-at-most-two scout
does not apply, and the later scout requires a planned bivariate start, which
is absent. These are static predicates from `factor.rs`, not sampled call-stack
evidence about a timed-out invocation. `strategy-controls/static-shape.json`
records the full variable map and source-bound term counts.

Three fresh workers compare the existing `auto`, `univariate` and `bivariate`
start settings through unchanged `examples/profile_operation.rs`. They use the
same frozen debug binary, source, CPU 8, 60-second deadline and 4-GiB limit.
The helper reconstructs the full signed product before a successful factor
result is accepted. No production setting or dependency checkout is modified.

| Start policy | Status | Wall seconds | Peak RSS (KiB) |
|---|---|---:|---:|
| Auto | Timeout | 60.026640 | 14,060 |
| Univariate | Timeout | 60.035439 | 13,536 |
| Bivariate | Timeout | 60.034761 | 14,116 |

None produced a factor result, so none is counted as reconstructed or solved.
The three short controls show no simple policy winner; they do not establish
that any strategy would fail with a longer limit. Low memory consumption and
nearly full CPU use in these trials point to algebraic work rather than the
memory cap. No new factor result was inserted into a production cache.

## Evidence and next useful experiment

Ignored artifacts live in `artifacts/research/gluza33-affine-budget-02/`.
`SHA256SUMS`, `manifest.json`, source capture and build logs bind all stages.
The first binary, source, patch and results are retained unchanged; reviewed
files live under `reviewed/`, and the final univariate factor scout is frozen
separately under `factor-scout/`. The source base is
`ab7df9b70c0f77a9ee2380dbce44346dbe398400`; the helper binary SHA-256 is
`ac6ec0f9449277dfc11aea84daf6bd6b9ce5f769ed22a09e2cda28c5ce0f7837`.
The reviewed helper binary SHA-256 is
`1f4a87c966ddce01494c330aed2915d292d6b3c534eb2976e7ef3e867f5ff93b`,
and its source SHA-256 is
`bc2458f17186293be8b27a0fc7972533aa90d18a9fa5a57e3c63b212ceaaba4f`.
The final factor-scout binary SHA-256 is
`aad9ba59a722f243c28045fa171f9f70abbb056dbce7833361c7be390fcbe173`,
and its source SHA-256 is
`d0e6ef95baff7604e4f9d6755725541f01caf67ee0c803ba69e44b55864deeb4`.
The unchanged `profile_operation` control binary is frozen under
`strategy-controls/`, SHA-256
`d482dbaefcec1dcbc2d69df3fd7a232a14e61d55c612b9a33dca8b6b70367061`;
its source SHA-256 is
`eee465d57baff7256be44ad9dc9caf55d6eae7d528b5b89e0dbcdcd3b27d99a5`.

```sh
profile_support_projection source-capture.json \
  --capture-sha256 aa44f4fb3da0145236f42254989933015ad6d6eda25cada5b49e65d83f60be43 \
  --output admission --admission-only --max-source-terms 4096 \
  --seconds 60 --cpu 8

# Same source binding and limits, with fresh output directories:
# --square-free-only --output square-free
# --admission-only --fiber-gcds --output fiber-gcds
# --admission-only --fiber-gcds --fiber-factors --output inventories

# Separate existing backend controls; use a fresh output for each policy:
taskset -c 8 profile_operation source-capture.json \
  --factor-start auto --seconds 60 --memory-mib 4096 --output auto.json
# Repeat with --factor-start univariate and bivariate.
```

The next useful experiment is a bounded lift of the degree-two image factor,
with exact division into the full source and complete signed reconstruction
before accepting any proposal. Scalar normalization, leading coefficients,
cross-fiber matching and variable-dependent content must all be handled; a
reducible image alone proves no multivariate factorization. Increasing the
current production affine source cap is unsupported by these diagnostics.
Likewise, the bounded controls provide no evidence for changing the production
factorization-start policy.
