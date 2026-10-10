# Exact support blocks at the first lifting axis

The default-off local prototype `solver.local_base_univariate_basis = true`
avoids irreducible factorization of bounded projection-generated guards
already in `Q[x_0]`, where `x_0` is the **actual first normalized lifting
coordinate**. It preserves every real zero, the original domain and the
complete generic proof. Initial constraint/split factorization is unchanged.
The focused implementation tests pass; original A377 completion and total
CAD cost with this option remain unmeasured.

A matched release-build experiment motivated the prototype: complete
square-free-decomposition/reconstruction/root-union work took 92.063 s,
versus 185.827 s for full factorization followed by the same checks, on one
captured A377 discriminant. Both returned the same 84 distinct real roots.
This is one operand pair, not a repeated benchmark or a solver speedup claim.

## Why the base case permits composites

Let a nonzero, nonconstant rational polynomial satisfy the exact identity

\[
p(x_0)=c\prod_i q_i(x_0)^{m_i},\qquad c\in\mathbb Q\setminus\{0\},\quad m_i>0.
\]

Its real zero set is exactly the union of the blocks' real zero sets,
including repeated and shared roots. Isolate this complete distinct-root
union and retain every complementary open interval. Each block, and hence
`p`, is continuous and nonzero with constant sign on each such interval.
The base is zero-dimensional: there are no parameters along which roots
can collide, disappear or escape. This argument needs no discriminant,
pair resultant, irreducibility or square-freeness assertion.

Higher-dimensional local proofs still require their complete raw guard
inventory. Exact product reconstruction and nonvanishing of **all** blocks
establish each replaced raw guard's nonvanishing. Original constraints and
split labels are evaluated from the original normalized polynomials, not
from discarded multiplicities or scalar signs. Every retained zero remains
a generic seam. No `s>4` boundary or even-multiplicity zero is erased.

This does not cover a multivariate polynomial that only becomes univariate
after substituting a tentative prefix. For example, `(y-x)*(y+x)` at `x=1`
still has a parameter-dependent collision at `x=0`; its higher-dimensional
projection requirements remain mandatory.

## Implemented scope and independent replay

The option requires `local_base_point_projection = true`. It is omitted
when false, and historical proofs without the new fields retain their
ordinary irreducible-factor contract. Global projection, root-index types,
the ordinary factor cache and native root isolation are unchanged.

| Source | Responsibility |
|---|---|
| [`base_univariate.rs`](../src/local_projection/base_univariate.rs) | Bounded sparse source parsing, native square-free proposals, exact canonical products and versioned evidence validation. |
| [`local_projection.rs`](../src/local_projection.rs), `Builder::guard_basis` | Admit only generated guards with full-map support in the actual first axis; maintain one immutable block inventory per canonical source. |
| The same file, `Checker::guard_basis` and `verify_local_parts` | Independently validate every recorded product, regenerate each raw guard, check protected inputs, and restrict typed cut use. |
| [`lazy_projection::discovery_roots`](../src/local_projection/lazy_projection.rs) | Apply the identical basis policy to leading-coefficient obligations during deferred discovery. |
| [`roots::isolate_union_using`](../src/roots.rs) | Retain existing exact isolation, repeated/shared-root handling, full-real owner ordinals and interval separation. |
| [`ProjectionCache::factor_basis`](../src/optimized_projection.rs), [`factors_checked`](../src/algebra.rs) | Keep the ordinary irreducible-factor contract; composite blocks never enter this cache under a factorization identity. |

`LocalEvidence::Local` has an optional `base_univariate_basis` record with
`schema_version = 1`. Each source stores its canonical sparse polynomial
and sorted `(polynomial registry ID, positive multiplicity)` references.
Source and blocks use primitive positive-leading normalization. The checker
verifies their exact product equals the canonical source. It independently
recovers the signed rational content of each regenerated raw guard and
checks the full signed identity too.

Replay enforces all of these obligations:

1. Every source is nonconstant of degree at least two, canonical, in the
   complete current map and supported only on axis zero. Blocks may be
   linear, reducible, repeated or share roots, but obey the same map/support
   and canonicality checks. Every record is checked, including an unused
   discovery record. Duplicate source records and repeated block IDs fail.
2. Only IDs explicitly referenced by checked products receive the typed
   exception. Every untyped registry entry retains independent
   irreducibility checking. Original input-factor IDs remain protected and
   independently irreducible even when they overlap a generated basis.
   Every original input's factor inventory is recomputed independently.
3. Each regenerated raw projection obligation either matches its exact
   canonical source record or uses ordinary factorization. Retained guards
   plus independently certified removals must equal that entire inventory.
   Hashes, samples or supplied discovery history never establish equality.
4. Typed blocks may be ancestor obligations, but may occur as cuts only
   at level zero, with an empty prefix and `base_point = true`. They cannot
   enter a higher-dimensional discriminant, pair selection or
   irreducibility-based cancellation as current-axis cuts.
5. The checker isolates every base cut, merges shared roots exactly and
   recomputes each owning block's full-real ordinal. It checks all roots,
   all roots-plus-one sectors, strict samples, children, propagated guards,
   exported cells and exceptional-polynomial inventory. An irreducible
   factor's ordinal is never copied onto a composite owner.

Native square-free decomposition only proposes blocks. The certificate
asserts an exact product, not that the blocks are square-free or irreducible.
The unchanged root kernel can handle repeated/reducible blocks. A single
whole-source block would also satisfy the theorem, but could repeat costly
square-free/GCD work and is not the discovery strategy used here.

## Lifetime, resource and failure rules

The separate registry is immutable per admitted canonical source. Initial
inputs keep ordinary factorization. In-memory learned cuts and completed
subtrees can reference append-only typed IDs because their obligations are
replayed. Combining this mode with `local_hint_archive = true` is explicitly
rejected in v1; no stored hint is silently reinterpreted. Verification reads
only the proof and original problem, independently of disk caches.

Static admission bounds the complete map, first-axis name, degree, term
count and rational coefficient height before primitive normalization and
native work. Sources have degree 2–4096; linear blocks remain allowed. The
registry permits at most 128 sources and 64 blocks per source, with a
128-million-coefficient-bit cumulative source-plus-block allowance. Source
strings use a bounded sparse integer grammar, not arbitrary expression
expansion. Degree, multiplicity and coefficient/product-work estimates are
checked before powers and products; the source-text cap is 32 MiB.

| Bound | Prototype limit |
|---|---:|
| Complete variable map / first-axis name | 64 axes / 1,024 bytes |
| Source degree / terms | 2–4,096 / 4,096 |
| Individual rational numerator or denominator | 20,000 bits |
| Per-polynomial coefficient bits / summed denominator bits | 64,000,000 / 65,536 |
| Sources / blocks per source / multiplicity | 128 / 64 / 4,096 |
| Product coefficient-bound estimate / `(degree + 1) × bound` work | 65,536 / 256,000,000 |

These are algebraic size estimates, not a process-memory guarantee; the
separate hard worker memory and wall limits remain necessary.

Static inapplicability can use ordinary factorization. Once the optional
algebra stage starts, a deadline or output/storage/work cap returns
**incomplete**; an invalid algebraic identity or map returns
**computation failure**. Neither starts another factorization backend or a
fresh global solve. The hard worker remains necessary to bound an individual
native call. Decomposition, reconstruction and root-union costs are reported
separately. Existing root limits and all other projection/template limits
remain in force.

## Measured motivation and its limits

Earlier [factorization](A377_UNIVARIATE_FACTORIZATION.md) and
[root-isolation](A377_ROOT_ISOLATION.md) diagnostics showed that threshold
predeflation had no measured total-time benefit. A 60-second full-factor
run timed out, but a later release control completed in 177.241 s; this is
not an intractable factorization barrier. A development-build matched
full-factor pipeline also timed out at 300 s. Neither timeout is used as
a completed timing or an exact speed ratio.

The subsequent matched release experiment used the same frozen helper,
raw degree-2169 signed discriminant, map `[s]`, CPU10, 4 GiB and 300-second
limit. Both fresh processes started without a sidecar, predeflation or
factor cache and used identical signed reconstruction and root-union code.

| Pipeline | Process wall | Decomposition | Signed reconstruction | Root union | Peak RSS |
|---|---:|---:|---:|---:|---:|
| Native square-free blocks | 92.063 s | 16.608 s | 2.544 s | 72.618 s | 71,724 KiB |
| Native irreducible factors | 185.827 s | 172.125 s | 3.346 s | 10.104 s | 69,828 KiB |

Both inventories contain **84 distinct real roots: 46 below 4, the exact
root 4 and 37 above 4**. Every corresponding ordered interval intersects by
exact rational comparison; none straddles 4. Full factorization makes
isolation cheaper by splitting the large block, but costs more decomposition
work in this single pair. Neither route removes the degree-881 block's
35 roots above 4. Total CAD size and original-problem completion remain
unmeasured.

Evidence is in `artifacts/research/a377-isolation-matched-release-01/`.
The [root-isolation note](A377_ROOT_ISOLATION.md) records all source,
input, worker and build bindings. Principal SHA-256 values are:

| Artifact | SHA-256 |
|---|---|
| Frozen helper | `6ed81cc04132676c0c9a457a9605eefe52123c92e9b5d00ccb4ed01d86da9797` |
| Square-free process report | `3e34dd241d2d40a27991eee08966e2d142f3c32800d971cca70d6756e44941b8` |
| Full-factor process report | `40e8e5302bf85e0b4e1c3e3ac4cc8c006935abbace553737b548f134169687c8` |
| Exact paired comparison | `1fbc3a2a58ed472725be384b2a4de2506f1a57c97fa958ddfdaa378b282928fd` |

## Validation and remaining gates

The focused bundle passed **277 tests**, including 13 independently written
[`local_base_univariate`](../tests/local_base_univariate.rs) integration
tests. These cover composite owner ordinals, repeated irrational roots,
rootless blocks, original-input overlap, the actual-first-axis restriction,
deferred leading-coefficient refinement, zero split labels, contradictory
constant domains, forged identities/multiplicities/IDs/version/support,
missing roots/sectors, historical fields, incomplete status, nested
homogeneity/monotone proofs and replay after checkpoint deletion. Small
module tests also cover shared blocks, signed rational content, caps and
stops after native decomposition or during product work.

The remaining tests in that bundle cover existing library, local, deferred,
homogeneity, monotone and supervisor behavior. The log is
`artifacts/base-univariate-focused-tests-04.log`, SHA-256
`57893e9e1dcc02869edc2f43aa5fd79793fbca3750903d68e6f56b1f751e4b73`.
Earlier compile/expectation failures are retained as separate logs.

The normal fast suite also passed: **553 committed tests**, plus one existing
test from the separate A381 workstream, across 40 targets with no failures.
Test execution took 54.58 seconds after a 3m23s development build. The log
`artifacts/base-univariate-fast-tests-01.log` has SHA-256
`9f9ec5ff2264d8fcf6a5176c1a70cd4bca9302a5e1d527d2f91cef0c456df28f`.
The unrelated A381 files are not part of this change or original coverage.

The next gates are a frozen worker, a captured
production guard-basis replay with its full independent root inventory,
and a bounded original A377 solve including preprocessing and verification.
Only those measurements can establish whole-solver benefit. The option
remains default-off; this document does not authorize boundary removal or
promotion to the default policy.
