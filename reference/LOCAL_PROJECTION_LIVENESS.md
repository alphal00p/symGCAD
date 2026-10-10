# Deferred projection and live-cut compaction

Experimental implementation and soundness design, 2026-10-10. The opt-in
construction preserves the existing proof format and independent checker.
No additional hard-case completion is claimed here.

## Finding and scope

The default `Builder::visit` initializes a stack from its original input factors,
then unions learned/archive cuts. Every same-axis child requirement or returned
vanishing guard is added permanently for that visit. Each round calls
`project(cuts)` **before** isolating the fiber and visiting its children.
Consequently an obsolete discovery hint can incur a discriminant and pair
resultants before the algorithm knows whether any final child requires it.
`LearnedCuts::remember` also unions inventories across repeated exact prefixes.
This is sound but may preserve unnecessary work.

The captured reversed-A446 degree-eight x3 polynomial is generated: the
original constraints and split have x3 degree at most two, as do their factors.
Its presence in a projection call does **not** establish that it is dispensable.
The retained capture/progress reports contain no complete child dependency
frontier proving its liveness or absence of liveness. In particular, neither
its two positive roots nor the failed Descartes saturation is a removal proof.

The existing source already contains the key proof abstraction. `Built`
returns a complete conditional subtree and its `required` set. A completed
subtree is reusable at the identical rational prefix even when ancestor
intervals change, provided **all** returned obligations are discharged. The
current subtree cache is explicitly based on this property.

## Primary algorithm and the remaining distinction

[Strzeboński, *Cylindrical Algebraic Decomposition Using Local Projections*
(2014), Algorithm 13](https://arxiv.org/html/1405.4925v1#S3.SS2) discovers a
child before projecting its requirements. It derives an interval where that
child applies, tracks the uncovered intervals, and finally projects selected
boundary polynomials. Its correctness argument retains the relevant lower
projection obligations; it does not infer validity from sample truth values.
See also Lemma 15 and Proposition 18 in the same paper. The repository's
sibling-wide saturation is a conservative variant, not that interval algorithm
verbatim. Its generic-open certificates and distinct-root selectors must keep
their existing semantics rather than import the paper's section/multiplicity
conventions implicitly.

## Conditional extension lemma

Write a stack prefix as `a`, its current coordinate as `z`, and its eventual
connected parent cell as `C`. Let a completed child proof at `(a,z0)` return
requirements `R`. Suppose a connected cylindrical sector `D` over C:

1. contains `(a,z0)`;
2. makes every member of R nonzero, with the sign it has at `(a,z0)`;
3. discharges the recursively required projection/domain premises.

Then the same child proof describes the entire remaining-coordinate fiber
over D. This is the existing conditional local-proof induction: leaf signs
follow from their input-factor obligations; internal boundary regularity,
ordering and exhaustive sectors follow from their projection obligations.
Continuity and connectedness propagate each nonzero polynomial's sign. The
original narrower ancestor bounds are not an extra premise hidden in Built.

This lemma permits **one whole conditional child** to cover an enlarged
sector. It does not permit concatenating pieces of different children based
on matching signs, or retaining an incomplete child/vanishing-guard return.

## Algorithm preserving the final certificate format

Separate discovery from certification at each stack. Keep original input
factors protected in this first version. A speculative object is never
exportable as a `LocalNode::Stack` until its final projection has succeeded.

1. **Numerical discovery.** At the exact fixed prefix, isolate the cut fibers
   without multivariate discriminants/resultants. Determine tentative sectors
   and recursively obtain complete conditional children. Enforce the same
   budgets and original positive-ray rejection premise. If a child requires
   a missing current-axis factor, add it and restart discovery as today.
   Earlier-axis guards and resource/unsupported exits still propagate.
2. **Close the frontier.** Continue until every tentative sector has a complete
   child and every child requirement in z belongs to the current cut set C0.
   All children refer to the same exact earlier prefix, each with its own
   rational z sample. No delineability assertion has yet been made for C0.
3. **Compute live cuts.** Let C1 be the original protected factors together
   with the union of current-axis requirements of these children. Thus
   `C1 ⊆ C0`. Lower-axis requirements are collected separately. Requirements
   from discarded rounds are not proof obligations of this frontier.
4. **Coarsen transactionally.** If C1 is smaller, isolate its numerical fibers.
   For each new sector choose one old child whose exact rational sample lies
   strictly inside. Such a sample exists: roots of a subset cannot subdivide
   an old sector, and each new open sector contains an old open sector.
   Retain that child's entire node and entire required set; discard the other
   children. Recompute the union from the selected children and repeat only
   when the cut set strictly decreases. Do not call fresh children inside this
   compaction phase.
5. **Certify once.** Compute the existing full projection of the final cuts,
   including all selected positive-ray, domain, pair and adjacency evidence.
   Re-isolate/validate the final root union and every retained sample. Return
   its exact lower obligations together with those of the selected children.
   Emit a final stack only after this succeeds. If the projection generates
   an earlier vanishing guard, propagate it; numerical discovery was not proof.

At step 5 the final projection proves delineability and root order on C.
Every selected child's current-axis requirements belong to the final cuts,
so each is nonzero throughout the whole corresponding sector. Lower
requirements are passed upwards exactly as before. The extension lemma then
proves every selected child valid throughout its enlarged sector, including
points on a deleted cut. The final sectors exhaust the whole ray/line outside
the retained explicit seams, and their children exhaust all remaining fibers.
This is a coverage argument over connected cells, not sampling evidence.

The independent checker can retain its present obligations: recompute final
projection, require the complete final root union and every sector, recursively
recover each child's requirements, insist every current-axis requirement is
in the final cut set, and exactly compare `required_below`. No removed-cut
certificate is necessary because the final proof never relies on that cut.
The polynomial registry may retain unused entries as it already does; output
levels and exceptional IDs must still be collected from the final proof only.

### Necessary implementation details

* Reused samples need the existing **certified arbitrary interior sample**
  policy (`stable_sector_samples=true`) or an explicit equivalent new mode.
  With that option false the checker requires the canonical `sector_sample`,
  which an old interior sample need not equal. Never bypass this check.
  Each representative retains its original rational coordinate and entire
  child node. Its earlier prefix is fixed throughout this visit. No sample
  coordinate is rewritten to fit the enlarged interval.
* Preserve the entire transitive required set, including original-domain
  support, strict Bézout support, separator evaluations, coefficient-sign
  guards and lower-axis projection obligations. A polynomial absent from
  exported boundary ownership may nevertheless be a required guard.
* For numerical discovery, detect zero leading coefficients before trusting
  generic degree. Propagate an exact nonconstant coefficient factor on a
  nullified/degree-drop prefix. Repeated/coincident numerical roots may guide
  proposals, but final projection must resolve or reject their obstruction.
* If using the original positive-coordinate premise, include its zero
  boundary and every distinct positive root in tentative discovery. Do not
  borrow a yet-unproved rootless or Descartes classification. Final typed
  selectors and ownership are reconstructed only from certified projection.
* Recompute lower requirements from the retained children and final projection;
  neither omit a live premise nor union unnecessary discarded-round premises.
  Cached completed children remain keyed by their full exact prefix and return
  all requirements unchanged. Cached cut inventories remain hints, not roots
  that the final proof is obliged to keep.
* Parent projection may reveal a cheap obstruction only after costly child
  discovery. Cheap leading-coefficient checks first, cost-triggered opt-in
  deferral, and bounded discovery work are reasonable scheduling choices.
  There is no unconditional performance improvement theorem.

## Termination and cycle controls

Enable `solver.local_deferred_projection = true` together with
`solver.local_stable_samples = true`. Both flags default to false, and the
configuration and direct local API reject deferred construction without the
strict-interior sample policy. The new implementation is in
`src/local_projection/lazy_projection.rs`; the existing checker is unchanged.

Each deferred visit admits at most 2,048 cuts, 4,096 tentative sectors and
131,072 retained requirement entries. It performs at most 32 strict cut-set
decreases. Hitting one of these scheduling caps resumes the ordinary
project-first loop with the current cuts; it never certifies a partial
frontier. Whole children are moved, not deeply cloned. These caps bound the
additional scheduling work and obligation storage; individual exact root and
algebra backend calls still depend on the solver's existing worker limits.
Node/refinement/cell/time stops propagate as incomplete and never start a fresh
global budget. Earlier-axis vanishing guards propagate normally.

Keep growth monotone during a discovery epoch. Once its frontier is closed,
compaction only selects existing complete children and strictly reduces a
finite cut set, so it terminates in at most its initial number of cuts. There
is no add/drop oscillation within compaction. A removed learned hint must not
be re-seeded into that same epoch after compaction.

If a later ancestor refinement changes the exact prefix, begin a new epoch.
The existing node/refinement/time/memory caps still govern the whole solve.
Our finite compaction argument does not prove total termination of the whole
experimental generator with sample-dependent separator templates. If future
designs re-run children while coarsening, bound those attempts, detect repeated
cut/frontier fingerprints, and fall back to monotone saturation; never declare
completion merely because a cycle or resource cap was reached.

## Stronger interval-local projection is a separate extension

If g remains required by even one selected child, the simple algorithm must
retain it. Ordinary local projection of that child's requirements still needs
g's discriminant (or an existing certified replacement). Moving the call later
does not establish that the discriminant is irrelevant.

A closer interval-local algorithm would store each child's validity interval
and support separately, certify nonvanishing of its guards **on that entire
interval**, and certify the final interval boundary chain over C. Projecting
only actual boundary polynomials can reduce cross-sibling resultants. However,
omitting `disc(g)` when g is still needed requires an independent regional
nonvanishing/root-continuity theorem: absence of roots at the sample is not
enough, since pairs of roots may be born inside the interval. Direct domain
certificates or exact interval-root certificates could supply that theorem.
This needs explicit interval-support evidence and checker work; it is not a
safe one-line deletion from the existing sibling-wide guard set.

## Tests and measurement before a hard retry

1. Inject a stale hint `g=z^8-a` into the z stack for `a,z,w>0`, split `w-z`.
   Every positive child needs z but not g. Compaction should remove g, avoid
   its discriminant, and retain the two exact sign cells of `w-z`; compare
   with baseline by certified common refinement.
2. Change the split to `w^2-(z^8-a)`. A live child needs g; it must remain.
   Forged omission must fail independent replay even if sample signs match.
3. Reuse a child across a removed root; test noncanonical interior samples,
   samples on/outside the new boundary, coincident roots, positive-ray zero,
   and exact earlier-prefix mismatch.
4. Exercise domain/Descartes/pair/Bézout support dependencies, earlier guard
   propagation, missing/duplicated sectors, rejected children, partial trees,
   resource stops, and historical proof replay. Record strict cut descent;
   no test should assume fewer mathematical refinements on every input.
5. Add bounded diagnostic provenance before the expensive operation: cut ID,
   origin (input/hint/child request), current prefix, and final-frontier
   reference count/complexity. For A446, compare the same capped input and
   checkpoint regime; report whether the particular octic is actually absent
   from live requirements before claiming this explains the timeout.

## Focused validation

The combined focused run passed 142 library tests, five initial deferred
integration tests, four interval tests, three interval-stock tests and two
helper tests (156 total). Its log is
`artifacts/local-cap-deferred-focused-tests-02.log`. The final deferred target
then passed all six integration tests in
`artifacts/local-deferred-guards-tests-03.log`, including actual original-domain
guard removals, strict Bézout support, Descartes separator guards, and exact
common refinement against the independent global solver.

The stale-hint regression disables the existing Descartes-one rule in both
variants to isolate scheduling. Eager construction retains four cells and
computes the octic discriminant. Deferred construction retains two cells,
performs no discriminant calls, and preserves the old `z=1/2` child sample in
the enlarged `z>0` sector; exact common refinement proves equal coverage.
The live-octic fixture retains the guard and rejects forged removal. These
are small exact regressions, not measurements on the unresolved A446 input.

An independent source review found no soundness blocker. Full-suite validation
and any hard-case pilot are separate steps. The implementation targets a
structural source of excess work; the captured octic's dispensability remains
an untested hypothesis.
