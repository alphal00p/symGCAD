# Support-preserving diagonal factor preconditioning

11 October 2026. A research-only diagonal scaling factors both historical
translation successes and the newly captured translation regression. Every
result passes exact inverse and full signed product replay. **This is an
operand diagnostic, not a new completed GCAD problem or a corpus speedup.**
The initial pilot changed no production dispatch or dependency code. The
subsequent default-off production integration is described below.

## Principle and bounded implementation

For nonzero rational constants `a_i`, substitution `x_i -> a_i*x_i` is an
automorphism of the rational polynomial ring. Its inverse divides each axis by
the same constant. It preserves monomial support, coordinate-zero factors,
degrees and leading-coefficient term counts. Consequently the native initial
variable-order heuristic based on those counts is unchanged. Coefficient
heights and subsequent modular choices can change.

The all-one leading coefficient after scaling is `a_i^d` times the original
leading coefficient evaluated at the other scales. Thus `[2,3,4]` reaches the
same regular evaluation point as the successful translation menu's
`[1,2,3]` shifts, without expanding its support. This is a general evaluation
point mechanism; no graph or captured-operand identifier selects an algorithm.
The reported scale vector was explicitly supplied to a research helper.

[`profile_support_projection`](../examples/profile_support_projection.rs)
now accepts `--factor-scale 2,3,4`, with one integer scale per active axis in map
order. Scalars must be nonzero and have absolute value at most 64. Existing
source caps remain: at most three active axes, degree 32, and 2,048 source
terms by default (explicit ceiling 8,192). The full map, inactive coordinates,
20,000-term intermediate cap and 8,192-bit rational coefficient cap are checked.
Scaling directly multiplies Symbolica rational coefficients by cached powers;
it verifies identical support and an exact inverse before one native factor call.

The helper verifies the complete signed factor product of the scaled operand,
maps every factor back, then verifies the complete original product. Units,
rational coefficients and multiplicities are retained. Both replays share an
8,000,000-term-product work limit; there are at most 64 factors and each
multiplicity is at most 64. The inverse automorphism also preserves
irreducibility of factors returned by the native engine. An external killable
worker still bounds the native algebra call. No factor cache is populated and
no original interior zero sheet is removed.

## Fresh-process measurements

All three trials used one frozen helper, CPU 12, a 30-second worker deadline
and effective 3-GiB inherited address-space cap. The helper's process report
records its requested 4-GiB cap; the supervisor clamps that to the inherited
3-GiB hard limit. The driver is a development build and dependencies use
optimization level two. Source parsing and admission diagnostics are outside
the separate factor/replay times, but inside worker time.

| Original operand | Scale + inverse check | Native factor | Both signed replays | Worker wall | Peak RSS |
|---|---:|---:|---:|---:|---:|
| Historical 224 terms | 0.000343 s | 0.013817 s | 0.001010 s | 0.042478 s | 11,172 KiB |
| Its 158-term block | 0.000224 s | 0.005830 s | 0.000514 s | 0.021399 s | 9,256 KiB |
| New 1,283 terms | 0.001377 s | 0.048233 s | 0.009461 s | 0.082617 s | 13,332 KiB |

The first two inputs are related, not independent families. The restored
nonconstant factors have term counts `[3,50,10]`, `[10,50]`, and
`[2,63,16,3,3]`; the last retains multiplicities `[3,1,1,2,3]`. Each result also
retains its exact scalar factor. Every factor uses the original seven-variable
map `[x2,x1,x7,x4,x6,x3,x5]`.

Earlier [translation measurements](AFFINE_FACTORIZATION_POLICY.md) completed
the first two at about 1.7 seconds while their unshifted controls timed out.
The [new regression control](AFFINE_COMPONENT_DISPATCH_RESEARCH.md) completed
unshifted at 0.043 seconds but timed out after translation. These are historical
controls from separately frozen profilers, not a new matched performance ratio.
The scaling pilot preserves their exact results and avoids both observed
obstructions; three single measurements do not justify unconditional adoption.

## Reproduction, tests and next decision

Build and test the committed helper with:

```sh
nix develop -c cargo --config profile.test.package.symgcad.debug=0 \
  --config profile.dev.package.symgcad.debug=0 test --locked -j1 \
  --example profile_support_projection -- --test-threads=1
nix develop -c cargo --config profile.dev.package.symgcad.debug=0 \
  build --locked -j1 --example profile_support_projection
target/debug/examples/profile_support_projection CAPTURE \
  --capture-sha256 SHA256 --factor-scale 2,3,4 --seconds 30 --cpu 12 \
  --output FRESH_DIRECTORY
```

All **25 helper/parser tests passed**, including four new tests for support,
regular fibers, signed/repeated/rational factors, inverse and map mutations,
shared replay limits, zero/duplicate/oversized scales, coefficient growth and
the algebra-only deadline. A first formatting invocation outside Nix failed
because `rustfmt` was unavailable; the successful bounded Nix build formatted
and tested the final source.

Ignored evidence is under `artifacts/research/diagonal-factor-scale-01/`, with
the exact launcher, build log, frozen source/binary, per-worker factor exports
and hashes. Input, binary, source and backend hashes were checked again after
the trials. The backend revision is `75f8350094b90254ee71dc2a391fde0d14b0204a`;
this helper uses the normal factor implementation, **without** the experimental
small-LC Wang patch.

| Binding | SHA-256 |
|---|---|
| Frozen helper | `8492e30577346488604454c9413608a756adcd6abaa852ba4efd77856850450c` |
| Helper source | `710ba3c5b7011a678c9b22e8a6bfad222ff11223a9f84df8703126e190d00130` |
| Normal backend factor source | `d5eaea5bc76e04aa29a0f481f94ba43580230d314fb0b182faebd9fe469e3fa1` |
| Historical 224-term capture | `162d5b9b98efeed85cadf3f375ad914fa902682a58764d927505927e66100646` |
| Historical 158-term capture | `94613312dd14f5ce2bd5c9f38f8855e9c90d9e90b8416ca1e523ef4b11fafac4` |
| New 1,283-term capture | `22f92bf277215836d66a4ef034e38be593eb87e34ec73bb7fbb0254234985cf9` |

Scaling can increase coefficient costs and does not promise good modular
images. Unrelated captured controls and complete-solve measurements remain
necessary before changing defaults; the first matched dispatch measurements
follow below.

## Default-off production integration and matched operand controls

The existing `affine_factorization = true` policy now first attempts diagonal
scaling at the regular point selected by its unchanged finite degree-drop
heuristic. All scales must be nonzero and resource-valid; otherwise it retains
integer translation. The existing later shear/native paths remain available
only after pre-native declines. There is exactly one native call after a
transform is selected: a blocked scale factorization cannot trigger a timed
translation fallback. The option still defaults to false. No Symbolica,
configuration schema, projection geometry or exceptional-set rule changed.

The production module checks support and full maps, a conservative rational
coefficient-height bound, the exact inverse, and both complete signed products.
All preparation and factor-restoration transforms share one finite work
budget, and both product replays share another. Post-native resource failures
or false identities propagate errors. The profiler now reports actual
`diagonal_scaling` or `integer_translation` selection after successful
preparation, including before a potentially interrupted native call.
Admission-only metadata explicitly denotes the regular-fiber heuristic and
does not claim successful transform preparation.

A frozen profiler linked against the same normal backend ran three fresh
policy processes on each original operand, three native processes on the new
operand, and one 30-second historical native control. The historical timeout
was not repeated merely to fill a measurement count. CPU, effective memory
cap and deadline were identical: CPU 12, 3 GiB, 30 seconds.

| Original operand | Mode | Median operation time | Median worker wall | Median peak RSS |
|---|---|---:|---:|---:|
| Historical 224 terms | Scaling policy, 3 completions | 0.008534 s | 0.021334 s | 9,284 KiB |
| Historical 224 terms | Native, 1 incomplete control | timeout | 30.036239 s | 12,256 KiB |
| New 1,283 terms | Scaling policy, 3 completions | 0.059292 s | 0.082385 s | 12,788 KiB |
| New 1,283 terms | Native, 3 completions | 0.041839 s | 0.062360 s | 12,232 KiB |

Policy operation time includes transform discovery, native factoring, inverse
mapping and both internal signed replays; native mode directly factors the
original source. Both modes then run the same separate independent signed
replay. All six policy runs selected diagonal scaling. The new cheap operand
therefore retains **1.42× operation overhead** relative to direct native
factoring. This fixes the observed catastrophic translation regression, but
does not make scaling universally faster. These are operand controls, not
solver benchmark ratios or additional corpus completions.

All nine completed factorizations passed full signed reconstruction. As a
supplemental comparison, a separately frozen Symbolica helper normalized and
exactly compared all eight nonconstant factors against the earlier historical
translation/new-native controls, retaining the original map and matching
multiplicities. All differences were zero. Constant units remain covered by
each full signed replay, not discarded by that comparison. Repeated exports
and the current/prior native exports also match exactly after record ordering.

Focused validation passed eight production module tests, three solve/fresh-
verify integration tests and 26 profiler/module/parser tests (including
repeated module tests). They cover negative rational units, repeated powers,
inactive and permuted maps, zero-scale translation fallback, resource limits,
shared budgets, corrupted evidence and actual transform labels. An independent
read-only review found no correctness blocker. The combined production suite
and release validation are tracked separately; these focused results alone
do not claim that gate.

Evidence is under `artifacts/research/diagonal-factor-production-01/`, including
the exact launcher, all process reports, signed factors, supplemental Symbolica
comparisons, build log and frozen hashes. The final module is
`cb68c3c2ef70111184c87907f32fd0b5c646f57b95888e3c932f24a250e592f5`, the
profiler source is
`31160d96d45f7a6829eab79a1381ffc14a72dfc1ebfabca962c5274a67a5a5b1`, and its
frozen executable is
`e49db5d3c110108563e05095f4d0c26e2169d35612e52a0870d53c8fc6297a83`.
