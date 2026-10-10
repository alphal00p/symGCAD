# Integer fibers for exact root refinement

The A414 CPU profile attributes 11.91% of sampled cycles to the exact sign
evaluator, almost entirely its general rational-coefficient evaluation path.
The existing integer Horner shortcut cannot help when a retained fiber has
rational coefficients, even though a positive constant can clear them once.
These inclusive percentages overlap other frames; see
[the measured profile](A414_SUBTREE_CPU_PROFILE.md).

Root-union construction now optionally divides a retained nonlinear fiber by
the absolute value of its rational content. Symbolica performs the content
and exact division. This produces a primitive integer polynomial with the
same signs, roots, multiplicities and full-real root indices. It makes later
sign bisections eligible for the existing integer evaluator.

Normalization takes place **after** the original Symbolica isolation and
square-free reduction, before interval endpoint correction and union
refinement. Initial isolating intervals and all subsequent exact sign
decisions are preserved. Linear and rootless fibers avoid this work; already
integer fibers keep their coefficients. The public polynomial table, root
ownership, exported bounds, verifier and proof obligations do not change.

This is a bounded optional shortcut. At most 257 coefficients are considered.
The maximum numerator bit length plus the sum of nonunit denominator bit
lengths bounds each cleared coefficient. Candidates above 65,536 bits per
coefficient or 8,388,608 total coefficient bits retain their original rational
fibers. There is no floating-point decision or inferred nonvanishing premise.

## Reconstructed-fiber experiment

`examples/profile_rational_fibers.rs` reconstructs exact fibers from bounded
reads of retained A414 factor records at a saved rational prefix. The actual
selected map and prefix were independently checked against the A414 worker.
This sample contains **two quadratic fibers and four nonexact roots**; 24 was
only the requested selection ceiling. These are not claimed to be the actual
simultaneous cuts of the profiled worker.

A frozen helper using the original root code compared both paths within one
process on CPU0. Five alternating repeats each perform 100 passes; input
cloning, normalization and exact output comparisons are inside both timing
loops. Root owners, indices and interval endpoints agree exactly on every
pass. The second experiment starts with unchanged original isolation output,
then performs 32 bisections per root, matching the proposed placement.

| Work per repeat | Original, ms | Normalized, ms |
| --- | ---: | ---: |
| Complete reconstructed union, normalization before isolation | 14.554 | 12.822 |
| Existing roots followed by 32 exact refinements | 35.449 | 12.899 |

The full bounded worker completed in 0.872 seconds, with 10,436 KiB peak RSS.
Only the second row measures normalization after unchanged isolation. The
first row additionally tests an earlier normalization placement that is not
used in production. These are development-build microbenchmarks with optimized
dependencies, not fresh-process solve medians, native GCAD ratios or evidence
of a newly completed original. The helper records its selected sources rather
than assuming portable directory iteration; future executable comparisons
must check identical source hashes, coefficients, prefix and variable map.

| Evidence | SHA-256 |
| --- | --- |
| Frozen helper | `d219136417c101b525c8b2d3556635a47400d20c560d374526df9344f9ef421f` |
| Five-repeat report | `1b785644a4aa68ea4c9511546ad11223161997cf3e07d5bc97e9f66c671287fc` |
| Initial ten-pass report | `029fc62d65e13f683b77fc1f0910953fa28a37294029c4ed8a1905552175cb23` |
| Original root source | `2b575f1fe61837f23d8015c0031e107b734e5287fcab2bfc41820c225a0d82e4` |
| Saved prefix/progress | `df7b362708ddd4da3feccfc6ac0eb963782a41343107a01b20c4765890363b48` |

The reports are under `artifacts/profile/rational-fibers-01/`; the frozen
helper is `artifacts/bin/profile-rational-fibers-before-01`.

## Correctness checks

Focused regressions compare the new path with the original internal root
construction, requiring identical owners, ordinals, complete intervals,
16 further refinements and open-sector samples. Cases include negative
rational scales, shared and extremely close roots, roots at isolation
endpoints, repeated real and nonreal factors, duplicate external IDs and
rootless fibers. Separate tests check exact positive scaling and bounded
fallbacks. The initial focused run passed six tests, including the three new
root regressions; its log is `artifacts/primitive-root-tests-01.log`, SHA-256
`32e1ccef798db188b2a7125281336f85ab144c43a5c9ab52e66da5b0306d0d99`.

With this change and the independently measured affine-prefix shortcut, the
full fast suite passes **512 tests across 34 targets**. Its log is
`artifacts/primitive-root-affine-fast-tests-01.log`, SHA-256
`f5f98201ee335153f4ae1d11d08b847ff26a1c4407b706b2f2be83f33a48d02f`.

The release built from `875f537` in 468 seconds has SHA-256
`2887414c30abddd437da4a216f50f6bbddfdd176577dacbec75734df80e64d85`.
A fresh public CLI solve and independent verification of the bounded square
with deferred local projection accepted two cells, two stacks and four
rejected branches. The same source's debug verifier also accepted the unchanged
historical fixed-A446 proof: eight cells, five stacks and six rejected branches.
That replay neither regenerates the proof nor repeats the supplemental sampling.
The build, test and verification manifest is
`artifacts/primitive-root-affine-release-build-01.json`, SHA-256
`51a906f519ae8a8a38aa9069e32b8a2f355c8fd29aa997a5916cd4864ba6a584`.

A [three-pair release diagnostic](A414_FIXED_WORK_RELEASE.md) subsequently
compared this change together with the affine-prefix shortcut against the
previous release. Identical capped A414 work took median 7.747562 seconds
before and 6.916713 seconds after; all exact outputs and counters matched.
Every run remained incomplete at the same node cap. This does not establish
a completed-solve speedup or isolate the benefit of either shortcut.

## Shared immutable defining polynomials

Each `FiberRoot` now holds an `Arc` to its immutable defining polynomial.
Roots isolated from one fiber share that allocation; their rational intervals,
owners and full-real indices remain separate. Exact refinement, collision
checks and root counting borrow the polynomial as before. Cloning a root no
longer copies all of its coefficients. This applies to every input fiber,
without a graph, degree or fixture selector.

The [D32 block inventory](A377_D32_ROOT_INVENTORY.md) exposed the cost of the
former representation: one degree-1478 block has 21,322,867 coefficient bits
and 294 real roots. Copying its coefficients into every root represents roughly
783,615,362 bytes of coefficient payload alone. That calculation motivates the
change; it is not a measured reduction in peak RSS. Symbolica's native isolator
also constructs its own interval records, and its temporary allocations are
unchanged. No isolation algorithm, root boundary or proof schema changed.

`FiberRoot::fiber` is a public Rust field whose type changes from an owned
polynomial to `Arc<UnivariatePolynomial<Q>>`. Manual callers can construct it
with `polynomial.into()` and explicitly obtain an owned copy with
`root.fiber.as_ref().clone()` when needed. Existing serialized cell bounds are
unaffected.

The combined resource-policy and root change passed 222 focused tests,
including all 169 library tests and 18 root integration tests. The latter
cover repeated and close roots, exact collisions, endpoint refinement and
independent intervals. Library results are preserved in
`artifacts/base-univariate-resource-focused-02.log` (SHA-256
`96feaae8de56eda03778cef1018d8878e038f351700787458aa505d11ec502b7`);
that command subsequently encountered one test assertion requiring the new
`Arc` field access. After that test-only adaptation, all remaining integration
and helper targets passed in
`artifacts/base-univariate-resource-focused-03.log` (SHA-256
`6c54464c6b61e6e669665c271556141cffdfe8bd65beac8165bcdda89a8ef31f`).

The release from `9773999` built in 494 seconds with unchanged source hashes.
Its executable SHA-256 is
`acd80197327fb6c8dfe9454c47be6b9ecf26491d3b1755d01bf33deebd1f20b1`.
Three fresh public-CLI solves and independent verifier processes accepted
7, 4 and 3 cells respectively. The first two preserve all mathematical/export
fields of the previous release's support-basis and two-parameter discriminant
smokes. The third uses
`(y^2-(2^12000*x+1)^2*(x-1)^3)^2` on `1<x<2`: its generated discriminant
source has 24,002-bit coefficients, above the old 20,000-bit source ceiling.
Evidence records the actual admitted source and exact block multiplicities
two and three. All three exported cells have positive split sign, with both
interior zero sheets retained as boundaries. These are correctness smokes,
not benchmark repetitions or newly solved corpus problems.

The complete smoke audit is
`artifacts/shared-fibers-resource-release-smoke-01/audit.json`, SHA-256
`95e0839d47215b91415a6f0cfdec6b1147734ba70a190fe5b29f2a0eee0b82e9`.
Its separate metadata/provenance audit checked all 19 bound files and the
exported root boundaries; SHA-256
`5371fb6153cff6d14a5fa029b3bcc3fb9e19717e2577cb8da5e5673eca8ab4c4`.
The immutable build manifest is
`artifacts/shared-fibers-resource-release-build-01.json`, SHA-256
`0374083b427c00c03693b7ce5205245f027472a59566e5219fa68832f8ca09a3`.
