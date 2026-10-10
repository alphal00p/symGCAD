# Square-free support bases above the first axis

Read-only assessment, 2026-10-10. **A higher-dimensional extension is
mathematically possible, but an exact product alone is insufficient.** It
would need its own projection and basis-maintenance contract. Keep the
current [first-axis prototype](BASE_UNIVARIATE_BASIS_PROPOSAL.md) narrow until
a complete factor/GCD/projection/lifting comparison demonstrates a benefit.
No implementation, build, or solver experiment was performed for this note.

## Literature and the restricted theorem

[Brown, *Improved Projection for Cylindrical Algebraic Decomposition*
(2001), §2 Theorem 2.1 and §3 Theorem 3.1](https://fitelson.org/pm/brown_projection.pdf)
give single-polynomial delineability under discriminant order-invariance,
degree-invariance and non-nullification; the latter theorem relates degree
invariance to the leading coefficient. The projection definitions use a
square-free basis, although the implementation in §4 factors irreducibly.

[Strzeboński, *Cylindrical Algebraic Decomposition Using Local Projections*
(2014), Algorithm 8, Lemma 15 and Proposition 18](https://arxiv.org/pdf/1405.4925v1)
explicitly use irreducible inputs. Its local correctness and finite-closure
arguments therefore cannot simply be cited unchanged after replacing that
inventory with arbitrary products. In particular, Lemma 15 uses
irreducibility to exclude nullification in its lowest parameter dimension.

[McCallum, Parusiński and Păunescu, *Validity proof of Lazard's method for CAD
construction* (2017 revision), Remark 2.2](https://arxiv.org/pdf/1607.00264v2)
explicitly allows a modified square-free basis, while reporting that
irreducible bases are usually more efficient. Its §5.3 uses the product
discriminant identity. This supports the basis distinction; it does not
authorize replacing our ordinary substitution with Lazard evaluation.
All three primary sources were checked on 2026-10-10.

The following **restricted generic-cell corollary** is sufficient for a
conservative extension of this repository. Let `C` be an actual connected
analytic parent cell, `z` the next coordinate, and let exact identities be

```
p_j(x,z) = c_j(x) product_i b_i(x,z)^m_ji,
m_ji >= 0,   c_j != 0.
```

Require every content `c_j`, every `lc_z(b_i)`, every nonconstant-degree
`disc_z(b_i)`, and every distinct pair `res_z(b_i,b_k)` to be nonzero
throughout `C`. Linear blocks have no discriminant obligation. Then each
block has fixed degree, simple ordered real root sections, and no root
section crosses another block's. Isolating the full union at the exact
sample and retaining every sector transports that inventory over `C`.
Every source is nonzero with constant sign on each sector. Original input
signs must still come from the original polynomials, including their scalar
signs and multiplicities.

Here nonzero discriminants have order zero, nonzero leading coefficients
exclude nullification, and nonzero resultants exclude common roots. Thus the
quoted single-polynomial result applies without irreducibility. Equivalently,
apply it to the product of the blocks. This is deliberately stronger than
the most general McCallum/Lazard hypotheses: a nonzero guard polynomial
vanishing at the sample is refined as an explicit ancestor seam. An
identically zero discriminant/resultant instead requires rebasing or
ordinary-factor fallback, not a new nullification rule. All real
source zeros, including even-multiplicity interior sheets, remain root
boundaries or explicit exceptional sets under the existing generic proof
contract. No sign-only or measure-zero deletion is justified.

## What exact GCD preprocessing would establish

In `R[z]`, `R=Q[x_lower]`, first split off the polynomial content in `R`.
Over its fraction field `K`, derivative GCDs and exact divisions can produce
square-free blocks; cross-GCD splitting can make the active blocks pairwise
coprime. Retain the complete content and multiplicity reconstruction in the
polynomial ring. Coefficient denominators introduced by working in `K`
cannot be silently cancelled across their zero sets.

These computations avoid an **irreducibility** claim. They ensure that the
discriminants and pair resultants are not identically zero; they do not show
that those guards stay nonzero on the parent cell. Independent replay can
check primitive/GCD claims explicitly, or enforce the stronger nonzero
projection obligations above. A claimed GCD of one in a multivariate ring
does not in general supply a polynomial Bézout identity equal to one.

## Current implementation dependencies

| Path | Actual dependency and extension obligation |
|---|---|
| [`algebra::factors_checked`](../src/algebra.rs), [`ProjectionCache::factor_basis`](../src/optimized_projection.rs) | The public contract is a canonical irreducible basis. Cache loading checks exact products, but the independent checker recomputes factorization. Composite records need a separate type/namespace; existing factor-cache entries cannot be reinterpreted. |
| [`verify_local_parts`, `Builder::guard_basis`, `Checker::guard_basis`](../src/local_projection.rs) | Registry validation explicitly requires irreducibility outside typed axis-zero blocks. Keep original input factors protected. A new typed mode must reconstruct every regenerated raw guard and every content, not just selected cuts. |
| `local_projection_raw` | Nonzero LC/discriminant and required resultants already supply the restricted theorem's ingredients. `D≡0` and `Res≡0` currently fail because irreducible inventory guarantees they cannot occur; they must cause exact rebasing/fallback, never omission. |
| `positive_projection_raw` | A non-coordinate irreducible has a nonzero endpoint polynomial. A composite can contain `z`; it needs exact coordinate extraction or an ordinary all-real path. Every original zero and endpoint remains protected. |
| Pair cancellation and [`strict_bezout`](../src/local_projection/strict_bezout.rs) | Their division-free identities and strict no-common-positive-root arguments are valid for composites too. Irreducibility is an inventory assumption, not the algebraic reason for cancellation. Individual delineability, complete obligations and correct owner IDs remain necessary. |
| [`descartes`](../src/local_projection/descartes.rs), [`interval_descartes`](../src/local_projection/interval_descartes.rs), [`pair_witnesses`](../src/local_projection/pair_witnesses.rs) | Root-count/separator arguments concern the whole supplied polynomial. They can be generalized only with exact whole-block ordinals, endpoint/degree checks, and permanent finite admissions. A factor's ordinal or template cannot be copied to a composite. |
| Rootless/adjacent-pair selection | Requires prior individual delineability and the complete root inventory, rather than irreducibility itself. Grouping can hide rootless components and force more work. Use full pairs in an initial experiment. |
| Learned cuts, completed subtrees and durable hints | Immutable IDs currently have fixed polynomial meanings. Algebraic equivalence of source products is not the current subset-of-cut-IDs test. Preserve old meanings, explicitly transport requirements or invalidate affected cached proofs, and keep durable hints disabled initially. |

Square-free bases are particularly awkward **online**. If a new source shares
a proper factor with an admitted block, splitting only the new source leaves
an identically zero resultant with the old block. Replacing the old polynomial
in place would invalidate root labels, completed subtrees and template stock.
An atomic per-stack basis snapshot with checked source-to-block transport is
needed before dynamic refinement can be sound. The fallback must rebuild the
affected stack with ordinary factors, not mix incompatible representations.

Finite closure also needs an explicit rule: immutable canonical IDs, no
merging split blocks into arbitrarily many regroupings, finite per-source
representations, and unchanged non-evicted per-ID template budgets. Exact
GCD splitting reduces factor support; arbitrary sample-dependent regrouping
is not covered by that argument. This is a polynomial-closure argument, not
a proof that every current speculative traversal terminates within its budget.

## Counterexamples an extension must retain

- `x*(z−1)^2`: taking `p/gcd(p,p')=z−1` drops the entire content boundary
  `x=0`. The even-multiplicity sheet `z=1` also remains a boundary.
- `(z²−x)^2`: its discriminant is identically zero, while its distinct
  real roots change from zero to two across `x=0`. Skipping the zero
  discriminant without square-free projection loses this collision seam.
- `(z−x)(z+1)` and `(z−x)(z+2)`: each is generically square-free, but their
  resultant is identically zero. Separate per-source square-free
  decomposition does not create a pairwise-coprime basis.
- `z²−x` and `z²+x`: generic coprimality does not prevent specialization
  collision at `x=0`; the discriminants/resultant retain that seam.
- `z(z−1)`: the endpoint polynomial is identically zero although the
  positive root at one must remain. It cannot enter the existing
  non-coordinate positive-ray case unchanged.
- For `(z−x)(z+1)`, the root `z=x` changes whole-block ordinal at `x=−1`.
  Its old linear-factor ordinal is not a valid composite root label.

## Measured motivation, cost and recommendation

The [matched first-axis experiment](BASE_UNIVARIATE_BASIS_PROPOSAL.md)
measured total square-free/reconstruction/isolation time 92.063 s versus
185.827 s for irreducible factors, on one univariate A377 operand. It had no
further projection step. It supplies no higher-dimensional timing result.

A concrete multivariate test candidate already exists in the
[A446 follow-up](A446_ALGEBRA_FOLLOWUP.md): the largest completed factor
operation took 7.739 s (independent replay 7.753 s), reducing 32,905 terms
to five factors totalling 203 terms. Their `x2` degrees are
`[2,1,1,1,4]`, with multiplicities `[2,4,4,4,2]`. Merely grouping equal
multiplicities yields degree-six and degree-three blocks. The latter contains
the coordinate `x2`; native square-free preprocessing may extract that
monomial separately. This grouping is a metadata-derived cost warning,
**not a measured native square-free decomposition**.

The record is `artifacts/hard-campaigns/a446-symbolic-x5-extended-debug-01/`
`checkpoints/operations/9d35ce435f9231120fedfc9421e758a782504544342c2bc80184b2690d4acd93.json`,
SHA-256 `f7064b1deb1ab975621c5c3adf31e007771d6817e7a32cac65b5ed775c31e7df`.
Its raw input digest is
`f1ee2a7f91218f5bd59df2f6b09c280b0980abeb73858dd7f74c125eb6a8ec66`.

The exact identity
`disc(fg)=disc(f)*disc(g)*res(f,g)^2` explains the risk: composite
discriminants package internal cross-resultants into a larger polynomial;
pair resultants likewise multiply across components. Factorization savings
can be outweighed by the next discriminant, expansion, root isolation and
lost selective-pair opportunities. Most completed factors in that A446
inventory were already cheap; A414's separate profile primarily showed
repeated traversal. Neither justifies a global basis switch.

**Recommended next gate:** a bounded standalone comparison on existing
completed operands, measuring content/GCD decomposition, exact replay,
the *next complete projection*, and identical fiber root unions. Compare
against the retained irreducible bases in the same build and cold process;
count extra lower guards and degree/term growth as well as time. Include
both an expensive reducible operand and an expensive single-factor decline.
Only a demonstrated total-cost gain would justify a typed prototype, first
for generated guards in `Q[x0,x1]` at the actual second lifting axis, with
full ordinary projection, separate source/basis records, and all higher
levels and original input factors unchanged. Static inapplicability uses
ordinary factors; an interrupted algebra stage remains incomplete. No
production extension or new completion is recommended on present evidence.
