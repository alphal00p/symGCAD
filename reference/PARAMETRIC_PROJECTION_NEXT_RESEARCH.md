# A next step for threshold-informed parametric projection

Research assessment, 11 October 2026. No new production behavior, performance
comparison, or completed corpus problem is claimed. A small exact identity
replay is recorded below. This complements
[CFF kinematic projection](CFF_KINEMATIC_PROJECTION.md),
[selected positive roots](SELECTED_POSITIVE_ROOT_DELINEABILITY.md), and
[the critical-certificate gate](SELECTED_CRITICAL_CERTIFICATE_GATE.md).

The most relevant additional literature is **real root classification of
parameter-dependent equations together with inequalities using weighted
Hermite matrices**. It suggests a bounded, equality-aware checker for the
selected roots of a small resultant, rather than a replacement of the whole
GCAD engine. CFF thresholds would propose useful parameter chambers and
coordinate groupings; exact polynomial certificates would decide which
projection obligations can actually be removed.

## What the additional literature does and does not provide

| Primary source | Result useful here | Limitation for symGCAD |
|---|---|---|
| [Gaillard–Safey El Din, *Solving parameter-dependent semi-algebraic systems*, ISSAC 2024, revised 2025](https://arxiv.org/pdf/2402.07782), §2.2/Theorem 2.4, §3.1/Proposition 3.1 and Corollary 3.2, §4 | Weighted Hermite signatures give Tarski queries; adapted sign determination counts roots satisfying strict inequalities. The specialization theorem states the excluded basis locus. The method permits nonradical input ideals. | The output is a dense parameter-space root-count classification, not cylindrical root functions. Its exceptional basis locus cannot silently be crossed. Complexity bounds require additional hypotheses. |
| [Le–Safey El Din, *Solving parametric systems of polynomial equations over the reals through Hermite matrices*](https://arxiv.org/pdf/2011.14136), §§4–5 | A parametric quotient algebra and Hermite matrix organize real root classification without applying ordinary CAD to all unknowns. | This earlier equation-only result does not by itself select positive common roots or provide exported cells. |
| [Lazard–Rouillier, *Solving parametric polynomial systems*](https://doi.org/10.1016/j.jsc.2007.01.007), covering-property formulation | Discriminant varieties identify a parameter set outside which finite solution sheets form a covering. | A complex discriminant variety still sees complex and negative-coordinate events; constructing it is not automatically a reduction of the relevant real problem. |
| [Chen et al., *Computing with semi-algebraic sets: relaxation techniques and effective boundaries*](https://www.csd.uwo.ca/~mmorenom/Publications/Computing_with_Semi-Algebraic_Sets_Relaxation_Techniques_and_Effective_Boundaries.pdf), §3/Theorems 1–2 | Effective boundaries concern changes of real solution count and can be smaller than a border-polynomial set. | Equal counts on neighboring open regions do not certify continuity, simplicity, or the omitted parameter section required for a cell spanning them. |
| [Chen et al., *Triangular decomposition of semi-algebraic systems*](https://arxiv.org/pdf/1002.4784), §5/Theorems 3–4 | Fingerprint polynomials address the gap between parameter sign conditions and connected components, allowing real root classification over regular systems. | This is a substantial alternative representation and does not remove the need for cylindrical export or specialization evidence. |
| [Han–Dai–Hong–Xia, *Open weak CAD and its applications*](https://arxiv.org/pdf/1507.03834), Definition 1 | Open weak delineability preserves the projection incidence of open connected components. | It is weaker than ordered boundary functions. It can aid exploration/positivity proposals but is not a substitute for the current cell certificate. |
| [Michel et al., *On projective delineability*](https://arxiv.org/pdf/2411.13300), Theorems 1–3 | Projective roots can remain delineable when ordinary leading coefficients vanish; the global theorem requires simple connectedness as well as its other hypotheses. | This can remove some leading-coefficient guards for bounded sectors. It retains discriminant/resultant obligations and needs explicit infinity/sector exclusion; it is not the first attack on the large resultant discriminants. |

These are narrower conclusions than “the physical thresholds are the CAD
projection set.” The common useful principle is to classify the **selected
real system** and the particular map being used. In the quadratic-pair
problem that system is `q=B=0` with positive coordinates, and the map forgets
the last pair coordinate. Its parameters are the **entire earlier CAD
prefix**, including remaining Feynman parameters, not only `s,t`. A
two-kinematic-variable complexity claim would therefore be incorrect without
an independently verified reduction of the other variables.

## Recommended bounded prototype

First implement an examples-only **filtered univariate count checker**. This
is a concrete continuation of the earlier selected-root design, not a claim
that Hermite matrices will necessarily be cheaper on the authentic input.

1. Take a nonzero low-degree resultant `R(v,y)` reconstructed from its actual
   operands, and the regenerated selection predicates. For the existing
   quadratic chart these are `y>0`, `-C>0`, and `-J*K>0`.
2. Start with degree at most 8, at most two symbolic prefix variables, and
   explicit coefficient/term/time caps. A first counterexample suite can use
   one parameter. Decline larger cases before expensive expansion. This
   does not yet admit the full authentic prefix.
3. Require the leading coefficient of `R` to be nonzero on the claimed
   chamber. Form the companion multiplication matrix over rational functions
   in the prefix using Symbolica. The basis `1,y,...,y^(d-1)` then specializes
   correctly everywhere on that chamber, including multiple-root fibers.
   There is no generic square-free reduction or Gröbner basis in this first
   version. Retain every denominator and prove it nonzero.
4. Construct the weighted trace matrices for the few Tarski queries needed
   for the strict selection. Reuse multiplication matrices and traces.
   Recompute all identities in verification. At a rational prefix, compare
   the exact selected count against an independent complete root isolation
   and sign determination, including zero predicates and repeated roots.
5. For a whole chamber, accept only a replayable exact signature proof:
   bounded rational congruences and their diagonal/block signs, or a fully
   checked subdivision whose boundary cases are retained. A matrix evaluated
   at one sample proves no parametric count. A zero pivot must trigger an
   explicit rank/branch treatment or decline.
6. Use the counts only with the selected-root regularity and properness
   obligations in the earlier theorem. Define a selected-ordinal boundary
   type before using them in exported cells. Keep the ordinary exact path
   whenever any obligation is missing.

For strict predicates `g_1,...,g_m`, the elementary identity

```text
N_selected = 2^(-m) * sum_{e in {1,2}^m} TaQ(product_i g_i^e_i ; R)
```

counts distinct selected real roots. Multiplying every weight by `R_y^2`
counts the selected roots where `R_y` is nonzero. Equality of the two counts
is therefore an exact simplicity check. It does **not** establish properness
or exclude a root escaping through a selection endpoint. For a direct
finite system `q=B=0`, the analogous weight is `Delta^2`, with the Jacobian
`Delta=q_y*B_x-q_x*B_y` reconstructed from the inputs.
That system version additionally needs a proof that each selected `y` has
at most one selected `x`, or a separate proof that the projected `y`
branches do not cross. Nonsingularity of the two-variable system alone
does not give ordered, disjoint `y` boundaries.

The important optimization opportunity is **cancellation between queries**.
Do not insist that every individual matrix signature stay constant if only
the selected count needs to stay constant. Requiring every minor to be
sign-invariant can recreate the unwanted projection set. Conversely, a
claimed cancellation must be proved for the complete chamber, including
rank drops; observing it on a few samples is insufficient.

The first checker can accept a supplied small congruence/block identity and
replay it. Discovery remains bounded and optional. This is a sufficient
certificate path, not an unbounded general real-root-classification solver.
In particular, constructing every Hermite principal minor and performing a
sign-invariant CAD of them may simply reproduce the costly splits. The
example below demonstrates that a compact cancellation certificate can
avoid a split; it does not establish that such a certificate will be found
for an irreducible authentic system.

## An exact matrix example with an irrelevant collision

The following extends the unrelated fixture in the selected-root note. Its
polynomial and matrix identities were replayed with Symbolica as recorded
below. For `-1<u<1`, set

```text
f=y-4,  g=(y-1)^2-u,
B=x^2-1,
q=x^2+(f-g)*x/2+(f+g)/2-1.
```

Since `q(1)=f` and `q(-1)=g`, the only common solution with `x>0` is
`(x,y)=(1,4)` for every `u`. Nevertheless, the negative-`x` solutions
coalesce at `u=0`. Their `y` coordinates are positive when real, so the
ordinary positive-`y` index of the selected root changes across that event.

Put `e_+=(1+x)/2`, `e_-=(1-x)/2`, and `w=y-1`. Modulo `(B,q)`, the ordered
basis `(e_+,e_-,w*e_-)` exhibits the quotient as

```text
Q[u]  direct-sum  Q[u,w]/(w^2-u).
```

This three-dimensional basis is valid also at `u=0`; the second component
then becomes nonreduced. Its weighted Hermite matrices are diagonal:

```text
H_1 = H_(x^2) = diag(1, 2, 2*u),
H_x             = diag(1,-2,-2*u).
```

Using `sign(0)=0`, their signatures are `2+sign(u)` and `-sign(u)`.
Consequently

```text
N_(x>0) = (signature(H_x) + signature(H_(x^2)))/2 = 1
```

on the entire parameter interval, including zero. Each signature changes;
the selected count does not. A checker proving the opposite second blocks
need not classify their signs independently.

Here

```text
Delta=(3-2*y)*x^2+(2*y-1)*x.
```

It equals `2` on the positive component and `-4*w` on the negative one.
Thus

```text
H_(x^2*Delta^2) = diag(4, 32*u, 32*u^2),
H_(x*Delta^2)   = diag(4,-32*u,-32*u^2).
```

The same cancellation proves that exactly one selected root has nonzero
`Delta` for every `u`. Properness is explicit: the selected pair is fixed
at `(1,4)`. This supplies a complete small selected-system example, not just
a comparison of off-boundary samples. Any native replay must independently
verify the quotient basis, multiplication relations, trace matrices and
selection; accepting the proposed matrices themselves would be unsound.

The necessary negative test replaces `f` by `(y-4)^2-u` and `g` by `1`.
There are zero, one, and two selected positive pairs for `u<0`, `u=0`, and
`u>0` respectively, and the middle pair has `Delta=0`. Its critical cut
must remain. Also test `((y-1)^2-u^2)=0`: equal root counts on the two open
sides of zero do not remove the collision on the intervening section.

### Executed algebra replay

Ignored artifacts are under
`artifacts/research/selected-hermite-replay-01/`. The existing frozen
`endpoint_identity` executable checked **146 independent packed coefficient
identities**, and the same 146 after exact specialization `u=0`. A fresh
packing variable and a constant coefficient of one protect the signed
identities from the helper's primitive-polynomial normalization. No build
or new backend was needed.

The preparation script constructs expression syntax, including literal
matrix products and traces; Symbolica performs the exact expansions and
rational arithmetic. Checks bind the original `q,B`, idempotent identities,
both coordinate multiplication matrices modulo the input ideal, component
maps, all six weighted trace matrices, and `Delta`. Finite Taylor identities
bind the derivatives used in the Jacobian to the original polynomials.
The negative component maps to the monic relation `v^2-u`, including at
`u=0`; this is the basis-independence argument's explicit algebraic input.
Changing one proposed trace entry from one to two was rejected, as expected.

Both successful runs and the corrupted-input rejection took 0.03 seconds
each at the timer's precision, under a 20-second, 2-GiB, single-CPU bound.
Peak RSS was 6,160 KiB for the generic replay. These tiny validation timings
are not a GCAD speed measurement. The signature cancellation, basis
independence from the monic relation, and properness argument are reviewed
mathematical deductions from the checked identities; the helper does not
implement a general Hermite-signature or selected-root CAD verifier.

| Artifact | SHA-256 |
|---|---|
| Frozen helper | `addc74a94abc1762c02822c4a73ab619fba97603ec9e70e502d5831492d22381` |
| Identity inventory | `37df4902a12f61fdf86bc5fa52a8ccd50c431802e91456401938c9ef5533d0d7` |
| Generic replay report | `cf644eed7bf901a357a9c770e7d3be9370796913c57d08f427370b87e4e29337` |
| `u=0` replay report | `9cea90b579deb0d7264e4c5fce55bad87b461e5451e6a5dcac6c73a01fe44669` |
| Audit manifest | `7638f87f9c22c7bc9f01ede2e5719002b81ef2b5975e9865d3481fd3f4940fd5` |

The genuine-critical replacement above remains a mathematical proposed
regression, not a newly executed selected-root counting test. No original
problem, interior boundary, or exceptional parameter region was removed.

## How CFF chamber information becomes usable evidence

There is a smaller complementary implementation task: make verified
**affine chamber predicates** usable by existing positivity and root rules.
`src/domain.rs::derive_domain` currently admits only strict inequalities
supported on one coordinate. A threshold such as `c-s-t>0` is consequently
not a domain bound, even though it is affine in kinematics.

A narrow checker can prove an affine expression positive by reconstructing

```text
h = a_0 + sum_i a_i*l_i,
a_i >= 0,
```

where the `l_i>0` predicates are already bound to the verified ancestor
chamber. At least one strictly positive constant or positive-weight strict
predicate must occur. All coefficients are exact rationals; discovery can
use a small linear program but its output is accepted only by exact
identity/sign replay. A later polynomial version can use bounded products
of these strictly positive predicates and squares. This is a proposed
sufficient certificate, with the proof given by the displayed identity.

Use those signs to establish actual quadratic/Descartes/endpoint or selected
matrix premises. Do not treat CFF metadata as a new domain constraint.
The outer chamber decomposition must cover every original parameter
region, including the nonphysical complements of the `s>4`, unrestricted
`t` corpus. Added seams belong to the explicit exceptional set when omitted
by generic coverage, and no accepted cell may span an unproved seam.

## Required interfaces and acceptance measurements

| Location | Missing capability |
|---|---|
| `src/domain.rs` and `src/local_projection.rs::guard_domain` | A typed ancestor-chamber premise set, with exact replay and no borrowing of later variables or the target sign. The current rectangular proof context must not be silently widened. |
| A new research count module | Symbolica trace/multiplication matrices, exact congruence replay and bounded weighted-count evidence; all specializations and nullifications explicit. |
| `src/output.rs::RootIndexDomain` / `RootBound` | A predicate-bound selected root ordinal. `Positive` counts positive roots of one polynomial and cannot express positive common-pair selection. |
| Local lifting, saved-result verification and sanity-point reconstruction | Reconstruct the selected inventory at every new prefix, prove its whole-cell continuity/order, and preserve every original interior `F=0` sheet. |
| Ordering/caches | Bind assumptions, selector, complete prefix, transformations and proof version. Identical polynomial operands do not imply identical conditional proofs. |

Unlike the already implemented local-projection approach, this proposal
changes the object whose regularity is certified: a selected real subset,
rather than every real root of a retained cut. The existing local engine is
the fallback and the comparison baseline. The earlier positive-ideal
identity gate should be tried first when it can prove the same critical
exclusion more cheaply; Hermite counting is useful when that sufficient
positivity search fails or selected counts themselves are needed.

Measure matrix size, term/degree/height growth, discovery and replay time,
number of avoided discriminants and ancestor factors, and total time across
**all** original chambers. Compare: existing local projection; chamber
premises alone; critical identity certificates; and bounded filtered counts.
Small exact common refinements and malformed-evidence tests precede any
claim of a new complete decomposition. No benefit on the unresolved ZZ
corpus is established by this note.
