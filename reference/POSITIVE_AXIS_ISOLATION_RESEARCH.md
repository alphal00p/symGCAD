# Restricting root isolation to a certified positive axis

This is a proposed general optimization, not an implemented API, a production
change or an original-problem completion. The current root-isolation evidence
and exported ordinals remain unchanged.

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

## Minimal backend experiment

A separate replayable Symbolica patch could expose a positive-only API
using its existing square-free and positive-ray root routines. It should
preserve multiplicities and exact cross-factor interval refinement, return
only strictly positive roots, and handle the rational zero root separately.
No floating-point approximations or alternate polynomial backend are needed.

First compare the complete positive inventory against the existing full
inventory, matching roots by exact algebra and refined intervals rather than
requiring identical interval endpoints. Include repeated/shared roots,
rational zero, close roots, signed constants and the zero polynomial. Then
measure this authenticated block with a fresh process and a declared limit.
The native unit-shift and recursive-shift experiments remain separate
comparisons; any combined optimization needs its own measured configuration.

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
