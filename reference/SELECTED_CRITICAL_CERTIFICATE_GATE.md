# A small exact gate for selected critical-point exclusion

Research design, 10 October 2026. No new executable, production behavior or
authentic critical exclusion is claimed. This complements the reviewed
[selected-root theorem](SELECTED_POSITIVE_ROOT_DELINEABILITY.md); properness,
root counts and the new selector remain separate obligations. The authentic
5,863-term critical resultant exposed no smaller factor, so a bounded
identity check should precede another elimination experiment.

## Recommended first prototype: a positive ideal identity

Extract `q,B` from the original signed polynomial `F=q*z+B`, in its full
ordered variable map, and reconstruct `Delta=q_y*B_x-q_x*B_y`. Given three
bounded proposed polynomial multipliers, independently form

```text
P = A*q + E*B + L*Delta.
```

If an existing strict positivity certificate proves `P>0` throughout the
declared original domain or certified base cell with `x,y>0`, then there is
no selected solution of `q=B=Delta=0` there. At a hypothetical critical
point the exact identity gives `P=0`, a contradiction. This check includes
all equality sections automatically. It uses no sampled nonvanishing,
division, generic gcd, resultant discriminant or sign-invariant open CAD.

The special case `A=0` excludes positive `B=Delta=0` and is stronger than
needed. More generally, a proposed `d*Delta^2=A*q+E*B+P` is covered by
taking `L=d*Delta` and changing the other multiplier signs. For this
zero-exclusion conclusion no sign assumption on `L` or `d` is necessary:
the product vanishes at `Delta=0`. Strict positivity of `P` is essential.

This is a complete checker for a sufficient certificate, not a complete
search procedure for every empty critical system. A failed positivity
search means no certificate was found; it proves neither existence nor
absence of a critical point. Genuine critical chambers in the authentic
input prevent any such global exclusion certificate.

## Existing operations and actual missing work

| Facility | Available operation and limit |
|---|---|
| `src/algebra.rs::Algebra`, `Poly` | Exact Symbolica rational polynomials, full variable map, products, derivatives and equality. Reuse these operations. |
| `src/domain.rs` | `derive_domain`, `certify_nonzero`, and fresh `verify_certificate`; require sign `+1`. They certify the reconstructed domain from original bounds, not arbitrary assumed nonlinear cell predicates. |
| `src/local_projection/strict_bezout.rs` | Already checks proposed `p*A+q*E=k*H^2`, with positive rational `k`, a bounded coefficient-sign proof for `H`, original coordinate provenance, signed identity reconstruction and actual canonical cut binding. It is a two-cut separation rule, not a selected-root regularity interface. |
| `src/roots.rs` | Exact rational-fiber root isolation and Sturm counts on open intervals, including one-sided endpoint handling. These do not establish a parametric whole-cell sign query. |
| Symbolica `poly/univariate.rs`, `poly/resultant.rs` | Public exact quotient/remainder and pseudo-remainder operations; Brown/Ducos resultant code. The Brown linear-subresultant helper is crate-private and does not expose a complete signed query sequence. |

The first examples-only executable should accept an original affine-in-`z`
polynomial, its variables/axes and strict constraints, and three proposed
multipliers. It derives the bad equations itself, reconstructs `P`, and
uses the existing positivity certificate and fresh serialization replay.
There is no need to require `B` quadratic in both axes for this particular
emptiness implication; quadratic admission belongs to the later lift.
Therefore do not weaken the existing endpoint helper's separate theorem
just to accommodate this experiment.

For the first gate, use whole rational boxes and positive rays justified by
the input bounds. A standalone conditional box must be labeled as such.
Production use on a previously computed cell would additionally bind the
verified cell/proof and all supporting predicates; an arbitrary extra
inequality is not a certified chamber. Never add `P>0` or the desired
conclusion to the assumptions used to prove it. Preserve every original
constraint and signed normalization; when certifying the shared domain
of both signs of `F`, exclude split-target sign constraints from the
positivity premises using the existing exclusion mechanism. Regenerate the critical equations
from `F` even if discovery supplied convenient expressions.

The evidence need only contain the bound input, derived polynomials,
multiplier identity and ordinary positivity certificate. Fresh replay
recomputes them all. Reuse the research helper's raw-token preflight before
Atom normalization, sparse-support/degree/height limits, cumulative product
budget, fresh-output policy and external killable supervisor. Initial caps
can remain 12 variables, 256 input terms, 2,048 derived terms, degree 24,
512 coefficient bits, two million monomial products and 4 MiB output. No
Gröbner basis, factorization or new rational-arithmetic implementation is
needed. Discovery is a supplied bounded proposal, not an unbounded search.

## Two mandatory unrelated fixtures

Use the theorem note's family on `-1<u<1`, `x,y,z>0`:

```text
f=y-4, g=(y-1)^2-u,
B=x^2-1,
q=x^2+(f-g)*x/2+(f+g)/2-1, F=q*z+B.
```

Its selected pair is always `(1,4)`, although unselected negative-`x`
roots collide at `u=0` and change the ordinary positive-`y` ordinal. The
hand-derived identity is

```text
Delta=(3-2*y)*x^2+(2*y-1)*x,
(x+1)*Delta - ((3-2*y)*x+2)*B = 2*(x+1) > 0.
```

This supplies `A=0`, `E=-((3-2*y)*x+2)`, `L=x+1`. The positivity proof
needs only the original `x>0`. Multiplying the identity by `x+1` gives the
existing strict-Bezout-shaped right-hand side `2*(x+1)^2`. Thus the first
gate exercises already available algebra, while the new meaning is a
critical-system certificate. It does not yet export a filtered root.

For a genuine critical event, keep `B` and the definition of `q` but take
`f=(y-4)^2-u`, `g=1`. At `u=0,x=1,y=4`, exactly
`q=B=Delta=0`; all declared strict bounds hold. The positive pair inventory
changes from zero for `u<0` to two for `u>0`. A claimed exclusion over
`-1<u<1` must fail, and checking only those two open parameter sectors
would miss the critical point. This is an exact counterexample, not a
sample-based test for absence. Both fixture calculations remain hand-derived
pending the authorized replay gate.

An additional small test must exercise all three ideal generators. Put
`w=y-4`, `B=x^2-1`, `q=x^2+w^3*(x+1)/2`. Then
`Delta=3*w^2*x*(x+1)` and the exact proposed identity is

```text
(x+1)*q + (x+1)*(w^3/2-1)*B - w*(x+1)*Delta/6 = x+1 > 0.
```

Here `q=B=0` at `(1,3)`, while `B=Delta=0` at `(1,4)`; neither pair can
be excluded on its own. The three-generator certificate excludes their
simultaneous zero and tests more than the existing pair separator.

Tests should also reject altered multipliers, critical derivatives,
normalization or axes; a substituted fake critical polynomial; changed
evidence after serialization; removal of the supporting `x>0`; a weak
nonnegative remainder with an interior zero; and output/cap violations.
No result of this executable may claim properness, a root ordinal, full
lift coverage or permission to remove an original `F=0` boundary.

## Narrow extension for selected nonlinear predicates

If box positivity is too restrictive, a small later checker can use the
selection's strict predicates `g_i>0` directly. Verify an identity whose
right-hand side is a bounded sum

```text
P = sum_j k_j*h_j^2*product_i g_i^e_ij,
```

with `k_j>0`, nonnegative integer exponents, and at least one distinguished
summand with `h_j=1`. Then `P>0` on that precise selection. All `g_i` must
be regenerated from the selected-system definition or bound to certified
cell premises, rather than supplied as arbitrary favorable assumptions.
For the chart use ideals `(R,R_y)` and `g=(y,-C,-J*K)`; for the system
version use `(q,B,Delta)` and its positive-coordinate/cell predicates.
The chart certificate says nothing on `J=K=0` or selection boundaries;
the system version can cover those points when its own premises hold.
This finite template is sufficient, not a completeness claim for bounded
positive-identity search.

## Why parametric Sturm–Tarski is a later gate

At a fixed rational prefix, exact selected critical counting is feasible.
Compute `G=gcd(R,R_y)` after specialization; a constant `G` gives no
critical roots. Otherwise eight Tarski queries over its distinct real
roots count the strict predicate conjunction, using the theorem note's
indicator formula. Alternatively its sixteen-query difference over `R`
avoids explicitly constructing this gcd. Zero polynomials require an
explicit separate case, not a zero count.

[Li, Passmore and Paulson](https://arxiv.org/pdf/1506.08238), Section 5,
give the Sturm–Tarski computation using the signed remainder sequence of
`P,P'*Q`, with zero signs removed and appropriate endpoint hypotheses.
[Cordwell, Tan and Platzer](https://arxiv.org/html/2102.03003v2), Section
2.3, explain exact restricted sign determination and caution that arithmetic
operation counts omit coefficient-size growth. These are foundations for
the query primitive, not a ready parametric certificate in this crate.

For a whole base cell, a generic gcd over `Q(v)` is insufficient: it can
be 1 although the specialized gcd is nonconstant on an interior equality
stratum. A valid parametric remainder proof must retain all zero/nonzero
leading-coefficient and remainder cases, justify every division or positive
pseudo-remainder scaling, and prove its parameter strata exhaustive,
including their equality sections. Numerical samples or `CompleteGeneric`
do not supply that proof. Merely projecting the last generic remainder can
recreate the expensive full discriminant.

A later minimal query experiment could use a single bounded parameter and
a supplied exhaustive partition with rational endpoints, explicitly
replaying isolated zero strata as well as open intervals. General algebraic
parameter sections require an additional equality-aware representation.
Neither the local rational Sturm utilities nor the current open-cell proof
interface supplies that automatically. Start with the positive-identity
checker and the two fixtures above before introducing this larger layer.
