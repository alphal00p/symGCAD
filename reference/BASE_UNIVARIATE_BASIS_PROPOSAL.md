# Exact support blocks at the first lifting axis

This is a proposed, opt-in local proof extension, not an implemented exception.
It can avoid irreducible factorization of projection guards already in
`Q[x_0]`, where `x_0` is the **actual first normalized lifting coordinate**.
It does not remove any real zero, change the original domain, or establish
completion of A377. Complete root-isolation cost is still unmeasured.

## Why the base case permits composites

Let a nonzero, nonconstant rational polynomial satisfy the exact identity

\[
p(x_0)=c\prod_i q_i(x_0)^{m_i},\qquad c\in\mathbb Q\setminus\{0\},\quad m_i>0.
\]

Its real zero set is exactly the union of the real zero sets of the blocks
`q_i`, including repeated and shared roots. Isolate this complete distinct-root
union and retain every complementary open interval. Every block, and hence
`p`, is continuous and nonzero with constant sign on each such interval.
This is lifting over a zero-dimensional base: there are no parameters along
which roots can collide, disappear or escape. No discriminant, pair resultant,
irreducibility or square-freeness assertion is needed for this base theorem.

Higher-dimensional local proofs remain conditional on their complete raw
guard inventory. Exact product reconstruction and nonvanishing of **all**
blocks establish each replaced raw guard's nonvanishing on the base cell.
The existing induction can then use those obligations unchanged. Original
constraint and split signs must still be evaluated from the original
normalized polynomials, not inferred from discarded multiplicities or scalar
signs. Every retained zero remains an excluded generic seam.

This argument does not cover a multivariate polynomial that merely becomes
univariate after substituting a tentative prefix. For example,
`(y-x)*(y+x)` at `x=1` still has a parameter-dependent collision at `x=0`.
Such a polynomial must retain the ordinary higher-dimensional projection
requirements.

## Current implementation boundary

The relevant contracts are:

| Source | Present behavior and required boundary |
|---|---|
| [`local_projection::check_base_point_cuts`](../src/local_projection.rs) | Already requires level zero, empty prefix and exact highest axis zero. |
| `Builder::project` and `Checker::walk` in the same file | Base-point certificates already use the complete root union, no positive-ray restriction, and no ancestor projection guards. |
| `verify_local_parts` in the same file | Independently requires `factors_checked([p]) == [p]` for every registry entry, checks input factor inventories, and refactors raw guards during tree replay. All three contracts must be addressed explicitly; relaxing the registry check alone is insufficient. |
| [`lazy_projection::discovery_roots`](../src/local_projection/lazy_projection.rs) | A vanishing leading coefficient is factored before propagating its ancestor obligation; this path also needs the same support-basis policy. |
| [`roots::isolate_union_using`](../src/roots.rs) | Native rational isolation performs square-free decomposition and VAS isolation; shared roots are checked exactly. A further derivative GCD is performed when returned real-root multiplicities exceed one. |
| [`ProjectionCache::factor_basis`](../src/optimized_projection.rs), [`factors_checked`](../src/algebra.rs) | These retain the ordinary irreducible-factor contract. Composite blocks must not be inserted into their caches or returned under their existing API names. |

The existing test `base_point_root_union_handles_duplicate_shared_and_repeated_roots`
in [`tests/local_projection.rs`](../tests/local_projection.rs) first factors
its inputs. It therefore does **not** test acceptance of a composite registry.

## Minimal evidence and independent replay

Use a new default-false local option and an optional, explicitly versioned
`BaseUnivariateBasisV1` member of `LocalEvidence::Local`. Missing fields retain
historical semantics. Require the existing base-point projection option too.
One possible minimal record contains a canonical source polynomial and a
sorted list of `(registry_id, positive_multiplicity)` block references.
The union of these references supplies the explicitly typed block IDs.

Use the existing primitive, positive-leading canonical normalization for the
source and every block. The checker then verifies exact equality of the
canonical source with the block product. Equivalently, multiplying by the
independently recovered signed rational content reconstructs each raw source
exactly. A format retaining nonprimitive blocks instead must record and check
the nonzero rational scalar explicitly. Neither variant may compare only
degrees, hashes, samples or approximate values.

Replay must enforce all of the following:

1. Every source and typed block is nonconstant, canonical, in the full current
   variable map, and has exact highest axis zero. IDs, records and
   multiplicities are bounded and unambiguous. Every untyped registry entry
   retains the existing independent irreducibility check.
2. The checker independently regenerates each raw projection obligation.
   A replacement record must match its canonical source exactly and reproduce
   its entire block inventory. All other raw polynomials use the ordinary
   factor basis. Retained guards plus independently certified removals must
   equal this complete inventory, as in current replay.
3. The same policy applies to initial input witnesses if v1 permits composite
   original inputs. A still narrower first version may keep original input
   factorization unchanged and allow only projection-generated base guards.
   Protected input/coordinate obligations and domain-certificate provenance
   retain their existing checks.
4. A typed block may be an ancestor guard of a higher stack, but may occur as
   a **cut** only at level zero with an empty prefix and `base_point=true`.
   It must never enter higher-dimensional cut classification, discriminants,
   pair selection or irreducibility-based cancellation as a current-axis cut.
5. Independently isolate every base cut, merge shared roots exactly, and
   recompute all full-real ordinals from each owning block. Do not copy an
   irreducible factor's index onto a composite polynomial. Check the complete
   roots-plus-one sector inventory, strict samples, all children, propagated
   requirements, exported cells and exceptional-polynomial inventory.

Native square-free factorization may propose blocks, but its square-free
claim need not become certificate authority. The product identity alone
establishes the required zero-set equality; unchanged isolation can handle
reducible or repeated blocks. The smallest valid record is the whole raw
canonical polynomial as one block of multiplicity one. That variant avoids
irreducible factorization but can repeat substantial square-free/GCD work.

## Lifetime, caches and resource limits

Use a separate bounded local support-basis helper, memoized by exact canonical
source. Once admitted, its block inventory must remain immutable for that
builder. Wire ordinary projection, any enabled input path and deferred
leading-coefficient discovery consistently. Deterministic normalization and
one inventory per source avoid sample-dependent regrouping. Existing finite
projection and witness-template limits remain in force.

Do not relabel a completed algebra-operation factor checkpoint as a support
basis. In-process learned cuts and completed subtrees can retain append-only
typed IDs because their obligations are still replayed. For a minimal first
experiment, omit typed composite IDs from durable hint snapshots, or disable
that archive with an explicit reason. Its current load contract refactors
every candidate and admits only a single irreducible factor; silently reusing
that contract would either restore the bottleneck or misstate its meaning.
Proof verification must remain independent of all disk caches and hints.

Check record counts, term/height limits, maps, nonzero factors, positive bounded
multiplicities and the multiplicity-weighted degree before taking powers or
constructing products. Preserve the existing root-degree cap and hard worker
time/memory limits. Native square-free decomposition, reconstruction and
isolation require separate timings. A static inapplicability may use the old
path; an interrupted entered stage must remain incomplete rather than trigger
a second unbudgeted factorization attempt. Reusing private native isolation
or square-free state would be a separate interface change.

## Evidence and acceptance tests

The [A377 factorization diagnostic](A377_UNIVARIATE_FACTORIZATION.md) records
one fresh bounded process per stage. Native square-free decomposition of the
original degree-2169 discriminant completed in 36.598 s total (30.579 s native).
Deflation followed by square-free decomposition of its degree-2021 residual
completed in 37.059 s total (29.001 s native). Residual full factorization
timed out after 60.026 s. Thus explicit threshold deflation has no measured
total-time benefit. The residual's seven blocks have degrees
`111, 881, 10, 2, 4, 1, 1`, with multiplicities `1, 2, 6, 8, 10, 12, 20`;
their distinct-degree sum is 1010. Coordinate and threshold factors remain
in the exact original reconstruction.

Residual report SHA-256:
`dc27df111359c427066f92a8aef399636714530833c4a176fff2371ffd113da0`.
That report binds the signed discriminant and successful reconstruction; it
does not contain a completed root inventory. **Root isolation, resulting CAD
size and original-problem completion remain unmeasured.** Test isolation of
the 881-degree block and full block union before claiming this bypass resolves
the A377 bottleneck.

Required focused regressions before production admission:

- Composite/repeated inputs, odd/even multiplicities, shared roots between
  blocks, rational and irrational roots, rootless quadratics and zero split
  annotations. Compare independently verified generic coverage with the old
  factor basis; polynomial IDs and owner ordinals need not match.
- False scalar/product/multiplicity, missing block, duplicate/invalid ID,
  changed exponent, later-axis support, or a typed cut placed above the base.
- Missing root/sector/seam, wrong composite ordinal, an unproved child guard,
  and a degree-drop guard propagated through deferred discovery.
- Original strict coordinate constraints and protected input obligations;
  empty or contradictory domains; exact original sign labels.
- Historical missing fields, opt-out and global fallback; nested homogeneity
  and monotone wrappers; independent replay after cache/archive deletion.
- Bounded rejection of oversized records and incomplete status after stops
  in square-free decomposition, reconstruction or root isolation.

No production code or default behavior is changed by this proposal.
