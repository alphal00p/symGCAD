# Generic gates for support bases and affine discriminant preconditioning

Read-only follow-up, 2026-10-10. The next cheap experiment should measure
multivariate square-free decomposition and complete product replay on the
captured Gluza guard. This is a proposed algebra diagnostic, **not** a
recommendation to replace irreducible local-projection bases globally.
Automatic affine preconditioning is another generic operation-level route,
but its sole authentic static improvement has not yet produced a faster
complete discriminant.

The subsequently authorized [SQF diagnostic](MULTIVARIATE_SQF_DIAGNOSTIC.md)
completed: 0.006578250 seconds of native SQF and 0.000682642 seconds of exact
signed reconstruction, producing a linear block and a 158-term degree-five
main-variable block. This is an algebra-stage result, not a projection or
original-case speedup. The implementation assessment below remains a proposal.

## New motivation, and what was actually observed

The Gluza33 positive, `x0`-anchor campaign ended after 3600.025 seconds with
no result. Its final pending factor operand has 224 terms, maximum integer
coefficient height 17 bits, and degrees `[9,9,6,0,0,0,0]` in the complete map
`[x2,x1,x7,x4,x6,x3,x5]`. The guard's highest-axis/main variable is `x7`;
the capture alone does not identify the next visitor step.
The capture alone does **not** time the native factor call for one hour;
its elapsed field is absent and the last progress snapshot is stale.

- Capture: `artifacts/research/gluza33-x0-anchor-hour-01/final-capture.json`.
- Capture SHA-256: `162d5b9b98efeed85cadf3f375ad914fa902682a58764d927505927e66100646`.
- Ordered operand-array digest: `692447d2758a0dc4af4c3cb0ef38f2a17deb480ab1589f00a1d2e40a9d8421a1`.
- Native cache key: `8d83459833dd9bee9259906b756bb75aa94c659c02c88dd0a69f88338bd661d3`.

The final audited cache has no completed record for this key. The original
eight-coordinate positive domain, exact polynomial and legal order are
retained by that campaign. This guard is not eligible for the existing
first-axis support mode, nor the earlier proposed two-variable-only scope.
The complete three-variable support matters; a numerical fiber that becomes
univariate is not a substitute for it.

## A conservative generic multivariate policy

The [earlier theorem and counterexamples](HIGHER_DIMENSIONAL_SUPPORT_BASES.md)
still apply. Over a connected analytic parent cell, exact source/content
products together with nonvanishing leading coefficients, discriminants and
all required pair resultants establish delineable disjoint root sections.
Irreducibility is not needed for that restricted statement. Brown's
[Theorem 2.1 and projection definition, §§2–3](https://fitelson.org/pm/brown_projection.pdf)
support this square-free-basis distinction. Strzeboński's
[Algorithm 8 and Lemma 15](https://arxiv.org/pdf/1405.4925v1) use irreducible
inputs, so their local hypotheses cannot be transferred without the stronger
checks. These primary sources were reopened on 2026-10-10.

A viable bounded policy would have the following contract:

1. Admit only projection-generated polynomials using explicit size limits,
   support maps and actual highest axis. Keep original input factors and
   ordinary factor-cache records unchanged. There are no case names,
   coefficient fingerprints or preferred physics topologies in eligibility.
2. Propose content and square-free blocks using exact coefficient GCDs and
   division. Reconstruct the complete signed source in the polynomial ring,
   retaining every content factor and multiplicity. Fraction-field
   denominator clearing must remain explicit. Native SQF is a producer,
   not proof of primitive, square-free or coprime status.
3. For the initial prototype, retain full ordinary LC/discriminant/pair
   projection for all active blocks; disable positive-ray specializations,
   pair omission, durable hints and other factor-ancestry shortcuts for the
   new typed basis. Nonzero discriminants/resultants can themselves establish
   the required generic simplicity/coprimality, while exact GCD replay is an
   alternative. No square-free assertion based only on product equality.
4. A nonzero ancestor guard vanishing at the sample causes the usual explicit
   ancestor refinement. An identically zero discriminant or resultant
   requires exact rebasing or ordinary-factor fallback. It cannot be fixed
   by inserting the zero polynomial or declaring the affected fiber absent.
   Full-degree LC checks exclude accepted-cell nullification; content
   boundaries remain separately represented. No silent Lazard-evaluation
   substitution is proposed.
5. Basis records must be immutable and stack-scoped. A later source may
   share a proper factor with an earlier block, even if each source was
   separately square-free. Replacing a global ID's polynomial or preserving
   an old ordinal after splitting is invalid. An initial implementation can
   abandon that proposed stack basis and rebuild with ordinary factors;
   completed subtree/learned-cut reuse is allowed only after exact requirement
   transport, otherwise it is a miss. This is a correctness requirement,
   not an optional caching optimization.
6. Permit only finitely many representations per source, with permanent
   fallback decisions and finite source/block/work budgets. Never regroup
   blocks from sample to sample without a checked immutable basis version.
   Every exported root uses its actual block's full-real ordinal, and every
   original zero—including an even-multiplicity interior sheet—remains in
   the root union or existing explicit exceptional inventory.

Ordinary-backend fallback applies to static applicability or proposal declines.
Once an algebra stage has started, its failure, cancellation or resource stop
must propagate; it must not automatically trigger a second expensive attempt.

The main implementation risk is online basis overlap, not the standalone
SQF call. A one-time whole-frontier product followed by SQF can avoid pair
overlap algebraically, but may merge many unrelated components into a much
larger discriminant. It is not a cheap general workaround.

## Smallest measured gate to run before implementing that policy

The inspected Symbolica revision `75f8350094b90254ee71dc2a391fde0d14b0204a`
already calls `square_free_factorization` before its multivariate
irreducible-factor phase (`src/poly/factor.rs:4698`). That routine strips
scalar/monomial content, uses coefficient-GCD `factor_separable` (`:2967`),
then Yun decomposition. A standalone SQF measurement distinguishes whether
the expensive stage can be bypassed at all. The pending capture does not
identify its internal native phase.

The separately authorized research stage uses a killable single-core/4-GiB
worker with an explicit deadline of at most one hour and no restarted native
stage after a timeout. Its generic input limits are at most three active variables,
64 map entries, 2048 input terms, per-axis degree 32, total degree 64 and
4096-bit coefficients. Parse only the bounded captured sparse syntax, bind
the complete map and operand digest, and record native SQF separately from
signed reconstruction. Before product powers, cap at 64 blocks, 4096 total
block terms, a 2-million-bit block inventory and a conservative finite
product-work bound. Static failure or an interrupted stage is incomplete.
The limits are generic; this small operand merely supplies authentic evidence.

If SQF completes, record each block's support, degree, coefficient height,
multiplicity and content before proposing further work. A later bounded
stage must measure the **next full projection**, exact reconstruction and
identical sampled root unions, with all pair obligations. The single pending
factor capture does not include the entire active stack's companion cuts;
those must be recovered before claiming a full local projection comparison.

Even the first next discriminant can be worse: an unsplit degree-six block
with lower-variable degrees nine has a discriminant bidegree bound
`(2*6−2)*9 = 90` in each lower coordinate. The next projection of those
bivariate guards may dominate any factor saving. Actual block degrees,
not the raw operand bound alone, determine whether that second stage is
reasonable.

No earlier experiment rules this route out. Conversely, none establishes
a multivariate whole-pipeline gain. The first-axis D24 comparison had no
further projection. A446's completed 32,905-term factor operation reduced
to five factors totaling 203 terms; that is a concrete warning against
assuming less factorization always helps. The unchanged default remains
appropriate until this new gate gives evidence.

## Automatic affine discriminant preconditioning

The A377 transformation must not become a hardcoded rule `t=(5+s*u)/18`.
Its [checked experiment](AFFINE_DISCRIMINANT_PRECONDITIONING.md) supplies a
generic discovery pattern: repeated rational roots in a boundary fiber can
suggest a parameter-dependent linear scale that exposes exact polynomial
content. The measured encoded-size bound fell by 18.13%, but both operands
exceeded the historical 96-million-bit cap used in that comparison. The later
generic cap expansion does not supply a transformed-discriminant timing;
no transformed discriminant was computed in this experiment.

A finite generic producer can start with bounded rational parameter values
from a fixed menu or original literal threshold constraints. Initially it
should accept only bivariate operation operands; inactive entries in the
full original map remain intact. First retain
any exact parameter content; never mistake an identically zero fiber for
useful roots. Factor the bounded univariate boundary fiber, enumerate its
repeated rational roots `t0=r/q`, and consider only a fixed finite integer
scale menu `k=1,...,4`. Newton support slopes can rank these candidates, but
cannot certify their content. Fractional slopes would require a different
ramified substitution and are outside this proposed gate.

For parameter `s`, boundary `s0`, degree `n`, let `h=(s−s0)^k` and form
the following identity over rational polynomial coefficients:

```
P*(s,u) = q^n p(s,(r+h*u)/q) = d(s) Q(s,u).
```

Require exact forward expansion, an independently denominator-cleared
inverse identity, full signed coefficient-content reconstruction and static
degree/height/storage bounds before accepting a proposal. Then the exact
polynomial identity is

```
q^(n(n−1)) h^(n(n−1)) Disc_t(p)
    = d^(2n−2) Disc_u(Q).
```

This equation retains every scalar, parameter zero and collision factor.
If integer storage requires multiplying `P*` by an additional rational
unit `lambda`, retain `lambda^(2n−2)` on the identity's left-hand side and
recompute the signed content of that scaled polynomial explicitly.
Recovering the original discriminant requires exact polynomial quotient
and full reconstruction, including at `h=0`; no numerical division there
and no omission of a boundary factor is allowed. An internal algebra
preconditioner does not change original CAD coordinates or root labels.

Rank candidates by measured static algebra bounds, with a strict finite
candidate/work budget and ordinary discriminant fallback for static declines
only. Post-entry algebra/resource failures propagate. An attractive
Newton polygon or smaller parameter degree is insufficient if transformed
coefficient height or expansion cost increases. This operation-level
approach has simpler integration obligations than multivariate basis
replacement, but weaker current performance evidence. The immediate cheap
priority is the captured multivariate SQF/reconstruction gate; any production
priority should follow complete measured costs rather than this ranking.

## Concrete integration boundary after the measured SQF gate

The current local architecture has several distinct contracts. A new typed
multivariate basis must address each; changing the registry's irreducibility
check alone is insufficient.

| Existing mechanism | Required conservative extension |
|---|---|
| `Builder::intern` | Preserve canonical polynomial identity forever. Adding new block IDs is permitted; replacing the polynomial behind an old ID is not. |
| Producer and checker `guard_basis` | They currently select one raw-source-to-ID set from a global support registry. A changing stack basis requires an explicit immutable version and exact signed source-to-block transport; a later GCD split cannot silently replace the expected set. |
| `local_projection_raw` | It checks LC and discriminant before classifying roots, and computes pairs for root-bearing cuts. Either compute all block pairs initially or separately justify/replay the existing rootless classification from individual delineability. The claim "all pairs" must not describe an unchanged selectively projected path. |
| Positive-ray and adjacent-pair paths | Initially use the ordinary all-real route with full pairs. Whole-polynomial root-count witnesses may be extendable later, but their original factor labels/ordinals cannot be copied. |
| Guard removals and ancestor propagation | Reconstruct every generated raw guard, including contents and scalar units. Each nonzero guard required by the cell theorem must be carried by the returned obligations or an independently replayed strict nonvanishing certificate. No sampled sign is a substitute. |
| Completed-subtree and learned-cut reuse | Current keys/requirements refer to literal cut IDs. Reuse after basis refinement requires exact obligation transport and compatible root labels; absent that proof, it is a miss. |
| Independent registry replay | Typed blocks need separate evidence/namespace. Protected original input factors retain ordinary validation. Do not reinterpret durable irreducible-factor cache records as composite proposals. |

Individual native SQF calls cannot ensure pairwise coprimality across different
sources. For example, two valid source decompositions can both contain a proper
factor `z-x` while grouping it with different other factors. Their resultant
is then identically zero. A finite structural response is exact cross-GCD
splitting, content extraction and signed product replay for the entire active
stack snapshot. Every split must preserve the union and multiplicities; any
cached proof that used the former snapshot retains its old meanings.

A deliberately simpler first policy can abandon an overlapping proposed
snapshot and rebuild that affected stack with ordinary factors, with a permanent
bounded fallback decision. This is a successful mathematical applicability
decline, not permission to retry after an algebra error, cancellation or resource
stop. Arbitrary repeated regrouping, source-ID mutation and unbounded alternate
basis attempts are excluded. Whether a bounded fallback policy is useful is
unknown: frequent shared factors could make it decline most interesting stacks.

Full LC/discriminant/nonzero-pair obligations are a conservative correctness
gate over each connected lower cell. They prevent degree loss, nullification,
multiple fiber roots and coincident sections there, without an irreducibility
claim. Their nonzero-polynomial status over the rational function field is
different from nonvanishing throughout a cell. Every lower guard still needs
ordinary decomposition or exact strict certification on that cell. Structural
zero discriminants/resultants require rebasing or decline; they cannot become
exceptional "zero polynomials". Same-sign interior sheets remain represented.

## Next research projection gate, not yet run

The measured source satisfies `p=(1+x2+x7)*q`, with `deg_x7(q)=5`, lower
degrees `[8,9]`, 158 terms and 16-bit coefficients. A generic next helper would
consume any authenticated bounded block proposal, reconstruct the whole signed
source, then compute **all** block LCs, nonconstant-degree discriminants and
distinct-pair resultants. It would save every signed result and checkpoint
each completed operation, never write a solver cache, and propagate its common
deadline rather than start another backend after a timeout.

For this two-block input there are only three potentially nonconstant outputs:
`LC(q)`, `Disc(q)` and `Res(1+x2+x7,q)`. This observation is a consequence of
the authenticated block degrees, not a case-name dispatch rule. The conservative
discriminant bidegree bound is `[64,72]`; the linear/quintic resultant bound is
`[13,9]`. All source contents and linear factors remain explicit. If all native
results are nonzero polynomials, the report may state that these formal guards
meet the restricted theorem's algebraic gate **on cells where they are all
nonzero**. It must not claim that such a lower CAD or the original CAD was built.

A further exact product-discriminant check is available when all block
multiplicities are one: `Disc(f*g)=Disc(f)*Disc(g)*Res(f,g)^2`, including an
explicit scalar factor to power `2*degree(p)-2` if present. Repeated-block
sources instead have identically zero raw discriminant, so that check cannot
replace their support-basis obligations. Any direct raw-discriminant comparison
must be separately timed and bounded; it is not free verification.

The first pilot should bound main-variable degree, active lower variables,
coefficient height, dense resultant/discriminant support, output storage and
total exact replay work before expensive operations. Such limits should be
generic numerical budgets, not a whitelist of these degrees or this operand.
Keep subsequent lower guards on their ordinary factoring path initially. If
a lower guard of degree 72 in the next main variable `x1` and degree 64 in
the surviving parameter `x2` were projected unfactored, its next univariate
discriminant bound would be `2*(72-1)*64 = 9088`. Avoiding one
factor call can therefore move much greater work downstream.

The capture still lacks the complete contemporaneous stack of companion cuts.
The pilot can establish an exact projection for this proposed subset only.
Before a solver experiment, recover that stack or use an explicitly complete
test problem, compare all required guards and sampled full-real root unions,
and include the lower projection/factoring costs. The cheap SQF result now
justifies this bounded next measurement; it does not yet justify production
multivariate support bases.
