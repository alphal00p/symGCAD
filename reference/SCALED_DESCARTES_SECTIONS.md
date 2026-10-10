# Finite moving-scale Descartes sections

This is a research proposal, not a production option or an original-case
completion. It addresses a limitation of fixed rational interval templates:
the roots of a translating or scaling family can leave every previously
admitted cover, although simple sections following the parameters would work.
The existing permanent stock still gives a sound finite fallback; the question
is whether a moving cover reduces total guard and traversal work.

## Exact rule

Let `p(y,x)=sum_i a_i(y)x^i` have fixed degree `d` on a connected ancestor cell
`C`. Choose two lower-coordinate polynomials `P,Q` and prove `P>0,Q>0` on
`C`. Put `lambda=P/Q`. For fixed rational `0<=alpha<beta`, the map

```
x = (P/Q) (alpha+beta*u)/(1+u),   u>0
```

is an increasing bijection onto `(alpha*lambda,beta*lambda)`. The exact
polynomial identity after clearing the positive denominator is

```
T_alpha,beta(u)
  = sum_i a_i P^i Q^(d-i) (alpha+beta*u)^i (1+u)^(d-i)
  = Q^d (1+u)^d p((P/Q)(alpha+beta*u)/(1+u)).
```

The final tail uses
`T_alpha,infinity(u)=sum_i a_i P^i Q^(d-i)(alpha+u)^i`, equal to
`Q^d p((P/Q)(alpha+u))`. No assumption about negative roots is needed.

Retain the original leading coefficient and endpoint guards. Reconstruct
every transformed coefficient exactly. A nonzero coefficient must keep its
recorded strict sign throughout `C`; only identically zero coefficients may
be skipped. Each interval must have zero or one Descartes variation, and every
finite seam evaluation must be nonzero. A seam that is identically a root
declines this initial rule; it cannot be silently skipped. A one-variation
interval contains exactly one simple root, including multiplicity, and a
zero-variation interval contains none. These statements and the complete
contiguous cover certify every positive root of the original `p`.

The endpoints are analytic rational functions on `C`. Positive `lambda`
preserves their ordering, interval orientation and the original positive-root
ordinals. Boundaries can therefore still name `p` and its positive ordinal;
there is no coordinate transformation of the returned CAD. Existing pair
ordering obligations remain necessary. Every positive `p=0` sheet over a
certified base cell remains in the root inventory. At a scale pole, zero,
degree drop, seam root or inconclusive sign pattern, use ordinary projection
or the existing exact refinement path; no open sign chamber may be discarded.

## Finite recipes and independent replay

A practical initial menu would consist only of fixed support-derived recipes:

* a positive constant control scale;
* a single earlier active coordinate, with a recorded sign orientation;
* a small fixed list of coefficient ratios `a_i/a_j`, again with recorded
  numerator and denominator orientations.

For the last recipe, `P=epsilon_i*a_i` and `Q=epsilon_j*a_j`, where the two
signs are in `{+1,-1}` and their positive premises are explicitly guarded.
They are not absolute-value expressions used without a branch proof. Both
coefficient indices and the permitted rational knot vectors come from fixed
finite sets. No unbounded search for fresh rational knots is allowed. Thus
each polynomial supplies finitely many possible guard families; the usual
downward-variable finiteness argument still applies. A smaller permanent
admission budget can additionally limit work, without resetting on failed
stacks. Keeping the archive incompatibility is the conservative initial choice.

A proposed typed witness would contain the recipe and indices, its two signs,
a fixed knot-family identifier, and transformed coefficient signs. Replay
derives `P,Q` from the original polynomial and variable map, checks their
lower-axis support and strict premises, reconstructs the exact cleared
identities, checks every seam and the whole cover, and checks ordinary root
boundaries against the original positive ordinals. It does not rerun heuristic
recipe selection or trust a successful numeric sample. An ordinary stronger
discriminant proof remains valid even where a moving template is available.

## Applicability and cost

Dense coefficient ratios are a serious risk. The powers `P^i Q^(d-i)` can
increase guard degrees, term counts and coefficient height much more than a
constant cover. An exact Cauchy scale using sums or maxima of coefficient
ratios has the same problem; a maximum also requires additional sign regions.
An algebraic Fujiwara-style scale introduces algebraic sections and is outside
this small rational proposal. Simple monomials or sparse linear coefficients
are more credible initial candidates.

For the reversed-parameter A446 degree-eight capture
`f09d4745806e20ec3490808ee1ebbc84cb4c86e34a41f8c82fa8af5a5aa820ce`, the exact
coefficient profiles give 224, 248, 223, 185, 140, 74, 25, 7 and 1 terms in
ascending powers of the eliminated coordinate. In particular, `a_8` is a
constant and `a_7` has only seven terms, with degree at most one in each lower
coordinate. This makes `|a_7/a_8|`, interpreted through its explicit signs,
a bounded candidate worth measuring before attempting arbitrary dense ratios.
At the saved prefix its scale is `2292705/524880`.

`examples/profile_scaled_intervals.rs` is a bounded Symbolica diagnostic. It
uses three fixed rational knot families; screens coordinate scales and sparse
top-coefficient ratios; checks actual interval root counts against Descartes
variations; and expands only a successful constant control and sparse ratio.
It checks the symbolic coefficient matrix against the direct cleared
polynomial identity and records all guards, degrees and costs. Limits include
degree 32, 4,096 input terms, 256 coefficient slots and guards, 65,536 guard
terms, 512-bit coefficients and a one-million monomial-product estimate per
multiplication. The supervisor imposes 60 seconds and 8 GiB. Its two tests
check a moving family at three parameter values, exact identities and positive
ordinals, as well as scale/seam rejection. Captured-case measurements will be
reported separately; one successful prefix would not establish a domain proof
or a speedup.

## Six-prefix measurement

The frozen diagnostic completed in 5.335 seconds with 30,996 KiB peak RSS.
All six prefixes use the **same canonical captured polynomial**. Each accepted
numeric cover checks the distinct root count in every interval with exact
Sturm arithmetic. The results were:

| `[t,s,x1,x2]` | Constant scale | `abs(x1)` scale | `abs(a7/a8)` scale |
| --- | --- | --- | --- |
| `[-9,5,1/2,1/16]` | 2 roots | 2 roots | declined |
| `[-8,5,1/2,1/16]` | 4 roots | 4 roots | numeric 4; guard cap |
| `[-10,5,1/2,1/16]` | 2 roots | declined | declined |
| `[-9,6,1/2,1/16]` | 4 roots | 4 roots | numeric 4; guard cap |
| `[-9,5,1/4,1/16]` | declined | declined | declined |
| `[-9,5,1/2,1/8]` | 2 roots | 2 roots | declined |

Here “declined” means none of the three fixed cover families certified that
fiber, not that it has no positive roots. The other coordinate scales
`abs(t)`, `abs(s)` and `abs(x2)` declined at every tested prefix. Coordinate
rows were screened numerically only. Constant-scale accepted rows also
passed the symbolic reconstruction and strict nonzero-coefficient checks.
Both numerically successful coefficient-ratio proposals exceeded 65,536
unique guard terms. Their selected cover has only 63 coefficient slots plus
four scale/endpoint/leading obligations, so the recorded inventory-cap failure
is a term-count failure, not the 256-guard-count ceiling.

The constant cover has 57 stored canonical guards, with 25,173 terms (25,175
for its other successful knot family), versus the earlier adaptive production
cover's 41 guards and 17,159 terms. Its maximum guard degrees are
`[4,4,8,8]` in `[t,s,x1,x2]`; each expansion took approximately 0.26 seconds.
Both raw inventories include the constant unit, which subsequent projection
factorization drops; their nonconstant counts are 56 and 40 respectively.
The sparse coefficient ratio gave no additional accepted prefix and enlarged
the symbolic guards substantially. These data do not justify a new production
moving-scale rule. A monomial scale or a fixed fallback cover may still be
worth later measurement on genuine stock-exhaustion histories, but the
present six samples show no coverage advantage over the constant control.

The positive-root count changes from two to four as `t` moves from `-9` to
`-8` with the other coordinates fixed. Thus a single delineable positive-root
inventory cannot span that whole parameter segment. An endpoint crossing or
a positive repeated root must intervene; this measurement does not certify
which. It does not assert that the six prefixes share any final CAD cell.

Evidence is retained in
`artifacts/profile/a446-scaled-intervals-01/nearby-prefixes.json`, SHA-256
`f8b00ab5df773829aa75d15b46fea2c87ad2ec0df8bf253aa6dddadff2e01674`.
The frozen helper SHA-256 is
`261a946f162040124aa48092f2428b9a00a9cb1ded6ae6c5a81530cb49b6d4d6`;
its source snapshot SHA-256 is
`634269eb6b797a18dcab6dee9458c5f6fbc8b30115c59075677c4e7a0beb5a4b`.
These are individual debug diagnostic timings, not solver speedups or
original-domain completion evidence.
