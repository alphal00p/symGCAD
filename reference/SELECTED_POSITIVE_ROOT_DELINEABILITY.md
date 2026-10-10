# Selected positive common roots without a full resultant discriminant

Research design, 10 October 2026. No production implementation, new algebra
run or original-problem completion is claimed. This supplies a sufficient
theorem for the missing selector in
[the quadratic-pair lift](QUADRATIC_PAIR_LIFT_PLAN.md), not permission to
declare the entire resultant delineable.

## A sufficient selected-root theorem

Let `S` be a connected, verified cell of the complete original prefix before
`y`. Let `Omega` be a semialgebraic open subset of `S x R`, given by strict
polynomial predicates and, optionally, already certified continuous sector
bounds. For a polynomial `R(v,y)`, put

```text
Z = {(v,y): v in S, (v,y) in Omega, R(v,y)=0}.
```

Require both:

1. `R_y != 0` at **every** point of `Z`.
2. `Z -> S` is proper: over every compact subset of `S`, selected zeros
   cannot escape to infinity or accumulate at an excluded boundary of
   `Omega`.

Then `Z` is the disjoint union of a fixed finite number of continuous,
semialgebraic functions `y_1<...<y_N` on `S`. They are locally analytic when
the base cell is analytic. The implicit-function theorem gives a local
homeomorphism; properness makes this a finite covering. Ordering real
fiber coordinates globally labels its sheets, so simple connectedness of
`S` is unnecessary. The empty inventory is allowed.

No assumption is made about unselected roots, their multiplicities, or the
degree of `R` away from selected branches. This is therefore weaker than
ordinary delineability of the whole polynomial. An alternative sufficient
check is regularity plus an exact proof that the total selected-root count
is the same finite `N` at every base point. The `N` local branches around
each fiber then exhaust all nearby selected roots; a separate properness
argument is unnecessary in that variant.

## Apply it to the quadratic pair

Reconstruct, in the complete map and with all signed units retained,

```text
q=a*x^2+p*x+c,       B=b*x^2+E*x+C,
a*B-b*q=J*x+K,      R=Res_x(q,B),
b*R=b*K^2-E*J*K+C*J^2.
```

The premises `a,b,h>0` and `h*E-C>0` are whole-cell statements. On `C<0`,
`B` has one simple positive root. In the chart

```text
Omega = {y>0, C<0, J*K<0},
```

`R=0` is equivalent to the positive common solution `x=-K/J`. Let
`Delta=q_y*B_x-q_x*B_y`. On the common zero set the exact identity is
`R_y=J*Delta`. Thus selected critical exclusion can use either derivative,
but only after this identity and the nonzero chart denominator are checked.
Any additional original predicates must remain in the selection or in the
already verified base/fiber domains.

Moving `J,K` sign sectors are usable if their own complete boundary
inventory and order have been certified. Inside each sector one counts
only its selected `R` roots. A whole-cell endpoint exclusion must ensure
that no such root reaches the moving sector boundary. Merely observing the
same signs at a sample is insufficient.

The norm identity simplifies finite chart exits. With `b>0,C<0`, an
`R=0,J=0` boundary requires `K=0`, and an `R=0,K=0` boundary requires
`J=0`. At `C=0` and `J*K<=0`, it requires `K=0` if the endpoint witness
remains strict there, hence `E>0`. These reductions expose small boundary
systems; they do not prove those systems empty. If strict premises do not
extend to a boundary, that boundary needs its own check.

At `J=K=0`, the two quadratics can be proportional in `x`. There may still
be a transverse positive common pair with `Delta!=0`, although `R_y=0`.
The first charted implementation must stratify this locus or decline to
the ordinary exact path. It must not erase it or silently divide by `J`.

A stronger **system selector** uses the positive solutions of `q=B=0`
directly. If this set is proper over `S`, `Delta!=0` on it, and each `y`
has at most one selected positive `x`, the same proof gives ordered
selected `y` branches. This can cross `J=K=0`. Its finite-fiber solver is a
larger interface: an identically zero resultant caused by a shared
negative branch does not imply infinitely many positive solutions.

## Endpoint and infinity checks are substantive

A useful sufficient properness certificate is a verified finite positive
`B=0` arc between continuous axis endpoints `(alpha,0)` and `(0,eta)`,
with locally uniform finite bounds, plus `q!=0` at both endpoints.
Selected common roots are then a closed subset of that compact arc over
every compact base subset. The arc and endpoint claims require exact
replay; a bounded arc drawn at one sample is not enough.

Alternatively, certify explicit continuous finite barriers and exclusion
of every selected zero on their boundaries. For an unbounded sector,
`y=t/(1-t)` and exact denominator clearing give a compactified polynomial
problem on `0<=t<=1`. Excluding all zeros at `t=0,1` compatible with the
closure of the selection predicates is sufficient, possibly conservative.
This does not require a globally nonzero leading coefficient of `R` when
all degree drops concern excluded roots.

Regularity alone fails: `R=v*y-1` has only regular positive zeros, but a
root escapes to infinity as `v` tends to zero. Likewise `R=y-v` loses a
selected positive root through `y=0`. Interior folds such as
`R=(y-1)^2-v` require a genuine critical cut. Leading/endpoint obligations
of the individual quadratic `q`, and every original `F=0` sheet in the
final `z` lift, remain separate from this pair certificate.

## Exact replay gate, not an equality oracle based on open cells

An implementation must reconstruct all polynomials, units and selection
predicates from the original problem and bind the certified prefix cell.
One bounded sufficient regularity witness is

```text
d*R_y^2 = A*R + P,           d>0, P>0 throughout Omega.
```

For the system selector, use `d*Delta^2=A*q+T*B+P` on its entire selected
domain. Every identity and positivity premise is independently checked.
These are conditional certificate formats, not assertions that a cheap
certificate exists on the authentic domain. Other valid gates include
equality-aware sign determination or Sturm/subresultant certificates for
the filtered critical and boundary systems.

An ordinary `CompleteGeneric` open CAD is **not** an emptiness proof for
these systems: the critical solutions lie on equality sections. Any
internal branch proof must include all relevant zero branches, degree
drops and identically zero specializations.

For a concrete counting alternative, take `g=(y,-C,-J*K)`. At each
nonzero specialized `R`, counting distinct real roots gives

```text
8*N = sum_{e in {1,2}^3} TaQ(product_i g_i^e_i ; R).
```

This follows by expanding the strict-positive indicators
`(sign(g)^2+sign(g))/2`. Eight times the selected critical count is this
sum minus the queries with each product multiplied by `R_y^2`. Hermite
signatures or signed subresultants can compute these queries exactly,
including multiple unselected roots. A whole-base proof of constant `N`
and zero critical count must handle every specialization; the individual
queries need not stay constant when their sum does. This is a concrete
equality-aware alternative, not a demonstrated cheaper algorithm.
An identically zero `R` fiber requires a separate exact branch or an
explicit decline; it must not be passed to a finite-root enumerator as an
empty inventory.

At an exact prefix sample, independently isolate the entire specialized
`R` root inventory and determine the signs of `y,C,J,K` at every root,
including exact zero tests. Filter, order and check the complete selected
inventory against the certificate. Reapply this procedure at later
prefixes. A stored Thom encoding is only a point identifier unless its
derivative signs are also certified throughout the cell.

The existing `RootIndexDomain::{Real,Positive,RealDescending}` in
`src/output.rs` cannot express this filtered rank. A new typed selector
must bind the resultant, predicates/sector, selected ordinal, and its
whole-cell evidence. `PositiveRayProjection` and
`PositivePairWitness` in `src/local_projection.rs` currently certify
individual positive roots and pair order; they do not certify this new
projection of a selected common-root set. No ordinary ordinal may be
silently reinterpreted. The original coordinate order remains unchanged.

## Unrelated witness separating relevant and irrelevant collisions

Let `-1<v<1` and define

```text
f=y-4,  g=(y-1)^2-v,
B=x^2-1,
q=x^2+(f-g)*x/2+(f+g)/2-1.
```

Direct substitution gives `q(1)=f`, `q(-1)=g`, and `Res_x(q,B)=f*g`.
The only positive common pair is always `(x,y)=(1,4)`, with `Delta=2`.
At that pair, `J=(9-v)/2>0`, `K=-(9-v)/2<0`, and
`q_x=(v-5)/2<0`, so no incidental repeated `q` root explains the event.

At `v=0`, the negative-common-coordinate roots coalesce at `y=1`.
The zero-based distinct positive-root index of `y=4` in the full resultant
changes from 0 to 1 to 2 as `v` crosses zero. Its selected-positive-pair
rank remains 0. This hand-derived identity is an appropriate exact
regression fixture; it has not been executed as a new CAS test here.

There is a small exact regularity witness on the same fixture:

```text
Delta=(3-2*y)*x^2+(2*y-1)*x,
(x+1)*Delta-((3-2*y)*x+2)*B=2*(x+1).
```

The right side is strictly positive for `x>0`. Multiplication by `x+1`
also puts it in a positive-square Bezout form. Thus an equality-aware
checker can exclude `B=Delta=0` without classifying the irrelevant
`v=0` collision; no numerical sample or full resultant discriminant is
needed for this witness.

For the authentic problem, the retained
[positive-critical counterexample and cubic stratification](A446_FILTERED_JACOBIAN_RESEARCH.md)
prove that global Jacobian nonvanishing is false even inside
`s>4,u>4,C0<0`. Actual selected critical loci must therefore be retained and
the theorem applied only on certified noncritical chambers. The existing
cubic chart mixes original coordinates and is not an original-order CAD.
No repeated global positivity search or omission of those critical cuts
is justified by this proposal.

## Primary context

Strzebonski's [local-projection paper](https://arxiv.org/pdf/1405.4925),
Section 2 and Lemmas 15–16, states the ordinary root-function and
delineability contracts. The restricted covering argument above is an
independent sufficient theorem, not a claimed corollary that permits
dropping those contracts unchanged.

Perrucci and Roy's [Thom-encoding and sign-determination paper](https://arxiv.org/pdf/1609.02879),
Section 2.2, Proposition 4, Section 2.4/Proposition 11 and Section 3.1,
provides exact root ordering, filtered sign counts and parametric
specialization machinery. These support the proposed replay primitives;
their full elimination construction is not claimed to avoid the expensive
guards or to improve this corpus without measurement.
