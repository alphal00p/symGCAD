# Tropical follow-up: sparse certificates for signed graph polynomials

Research proposal, 11 October 2026. **The next small experiment should be a
bounded, exact weighted-AM–GM certificate for sparse positive-orthant
polynomials.** Newton supports suggest candidate inequalities; exact
coefficient margins decide whether they prove strict positivity. This extends
the existing quadratic and binomial-square ideas without changing projection
or the exported cell contract. No implementation, new corpus completion, or
measured speedup is claimed here.

## What is new relative to the previous review

The [SubTropica audit](TROPICAL_SUBTROPICA_RESEARCH.md) already covers the
package's linear-reducibility search, joint gauge/order proposals, tropical
subset recurrences, graph-informed coordinates and their limitations. The
[site](https://subtropi.ca/) and
[primary paper, §3.1](https://arxiv.org/html/2604.20954v1#S3.SS1) do not supply
a missing interior-root certificate: the paper identifies hidden singularities
as requiring additional Landau analysis. Likewise,
[Borinsky's Theorem 8.B](https://arxiv.org/html/2008.12310#S3.SS1) assumes
complete nonvanishing to obtain a lower bound from the tropical approximation.
Using that bound to establish its own hypothesis would be circular.

The new connection is to positive polynomials with sparse Newton support:

| Primary source | Precise relevance and limitation |
| --- | --- |
| [Iliman–de Wolff, §3.3 Theorem 3.8 and §3.4 Corollary 3.11](https://arxiv.org/pdf/1402.0462) | Circuit coefficients control nonnegativity and the discriminant boundary in the specified circuit class. Their all-real theorem assumes even vertex exponents. Our proposed positive-orthant sufficient certificate uses elementary AM–GM and does not require that parity assumption. |
| [Chandrasekaran–Shah, §2.1, equations (3)–(4), Lemma 2; §2.2](https://arxiv.org/html/1409.7640#S2.SS1) | A single negative exponential term admits an AM–GM certificate; sums of these give SAGE certificates. Substituting positive coordinates by exponentials connects this to polynomials. We propose only a bounded rational subset, not a floating-point entropy solver or a completeness claim. |
| [Sturmfels–Telek, Theorem 6.1 and §7](https://arxiv.org/html/2504.01628v2#S6) | For bridgeless, fully massive graphs, interior copositive kinematics admit a Pólya multiplier. Near the boundary its degree can be large. The proof does not automatically cover mixed-mass ZZ inputs or projected factors. Their computational discussion explicitly sets aside boundary cases; symGCAD must retain them. |

The current `domain/weighted_squares.rs` searches for two positive support
exponents whose average is a negative exponent. A three- or four-point
barycentric relation is a genuine addition to that discovery rule. Existing
quadratic-component and low-degree methods should remain the cheaper first
attempts. No claim is made that the entire existing portfolio fails on every
example below.

## Exact certificate and its proof boundary

On proven positive coordinates `x`, consider a component

```text
C(x;k) = sum_i c_i(k) * x^alpha_i - b(k) * x^beta,
lambda_i = m_i/N > 0,
sum_i m_i = N,
sum_i m_i*alpha_i = N*beta.
```

Assume `c_i>0` and `b>0` throughout a target-independent domain in the other
coordinates `k`. Weighted AM–GM gives

```text
sum_i c_i*x^alpha_i >= Theta*x^beta,
Theta = product_i (c_i/lambda_i)^lambda_i.
```

Since both sides of `Theta>b` are positive, it is equivalent to the exact
polynomial inequality

```text
Delta = N^N * product_i c_i^m_i
        - b^N * product_i m_i^m_i > 0.
```

Thus `C>0` on the positive orthant. Integer powers and rational comparisons
suffice; no logarithms, approximate radicals, or sampled coefficient signs
enter the proof. If coefficients are rational constants, this is especially
cheap. For parameter coefficients, existing leaf certificates must prove the
coefficient signs and strict margin over the whole admitted domain. Decline
when they cannot; do not launch an unbounded auxiliary parameter CAD.

For several negative terms, allocate positive coefficients to components and
reconstruct the original signed polynomial exactly. Never spend a positive
coefficient twice. A sum of weakly nonnegative components is insufficient:
require a strict component or a separately certified strictly positive
residual everywhere. `Delta=0` proves only nonnegativity, and the conservative
first implementation should decline it. This also avoids introducing a new
recursive weak-positivity proof type.

The candidate discovery may be incomplete. Independent verification needs
only the original polynomial and domain, the exact allocated coefficients,
the exponent and integer-weight relations, exact reconstruction, and the
strict leaf evidence. Bind full variable maps and coefficient signs before
clearing denominators. A domain assumption must not come from the target
factor being removed. Coordinates not proved positive cannot participate as
ordinary positive monomials; a pre-existing verified shift is permissible.

## Small exact regression with a preserved interior threshold

For `x,y,z>0`, let

```text
P_k = x^2*y + y^2*z + z^2*x - k*x*y*z.
```

The three positive exponents average to `(1,1,1)`. Equal weights `1/3`
give `Delta=27-k^3` for `k>0`, and

```text
P_k >= (3-k)*x*y*z.
```

At `k=2`, the exact margin is `19>0`. No two positive exponents average to
the negative exponent, so the current direct weighted-square matcher cannot
extract that negative term. The following negative coefficients also show
why simply using the current Pólya orders `0..4` does not certify this input:

| Multiplier `(x+y+z)^r` | A coefficient of the product with `P_2` |
| --- | --- |
| `r=0` | `[xyz] = -2` |
| `r=1` | `[x^2*y*z] = -1` |
| `r=2` | `[x^2*y^2*z] = -1` |
| `r=3` | `[x^2*y^2*z^2] = -3` |
| `r=4` | `[x^3*y^2*z^2] = -2` |

These are hand-derived exact multinomial coefficients, not executed solver
results. A future Symbolica regression should replay them and compare every
existing certificate arm before assigning performance credit.

At `k=3`, AM–GM equality gives the interior zero line `x=y=z`; the proposed
strict gate rejects it. For `k>3`, the polynomial is negative there and
positive for sufficiently large `x` with `y=z=1`, so interior zero sheets
exist. The support and coefficient-sign pattern stay unchanged across all
three regimes. Neither a Newton polytope nor a tropical ordering heuristic
distinguishes them. The coefficient margin does, without deleting the
equality slice or changing the original polynomial.

## How the graph can guide discovery

Authenticated spanning-tree and two-forest data can suggest candidate support
relations much more cheaply than enumerating arbitrary support subsets.
The signed forest convention and mass terms must be checked against the
complete exported U/F; topology names and the matroid alone are insufficient.
The standard tree/forest construction is described in
[Bogner–Weinzierl, §4](https://arxiv.org/pdf/1002.3458).

One useful hand-derived pattern is a cut block. Fix a spanning two-forest
`H`, let `C` be its complementary edge set, and let `I` contain edges joining
its two components. For each `e in I`, `H+e` is a spanning tree. Consequently
the mass part of F contains the block

```text
B_H = x^C * (sum_(e in I) 1/x_e) * (sum_(f in I) M_f*x_f),
```

where `M_f` is squared mass. For rational `q_f>=0` with `q_f^2<=M_f`,

```text
B_H >= (sum_(f in I) q_f)^2 * x^C.
```

This follows either from Cauchy–Schwarz or the exact identity

```text
x^C*((sum_e 1/x_e)*(sum_e q_e^2*x_e) - (sum_e q_e)^2)
  = sum_(e<f) x^(C\{e,f}) * (q_e*x_e-q_f*x_f)^2.
```

It turns a channel coefficient into a candidate mass-threshold margin.
This particular identity fits the existing weighted-square proof language;
the contribution here is graph-guided discovery, not a new square theorem.
Different forests can reuse the same mass monomial, so summing such bounds
requires explicit coefficient allocation and full signed reconstruction.
Checking each cut threshold independently is not a global positivity proof.
Massless edges, exceptional coefficient cancellations and other negative
channels remain in the residual. Rational mass lower bounds are sufficient
but may be too weak near a threshold.

The broader circuit proposal supplies relations involving three or more
positive support points. Use cut blocks, small graph minors and basis
exchanges to propose them, then let exact support and coefficient checks
decide acceptance. This is compatible with the existing graph export and
quadratic-block experiment: coordinates can expose sparse components, but
transporting cells still needs its own proof. A positivity certificate on a
genuine superset of the projected domain requires no new cell transformation.

## Proposed bounded experiment and priorities

1. **Rational sparse components first.** Try at most 32 candidate circuits,
   each with 2–4 positive support points, weight denominator `N<=8`, and at
   most 128 source terms. Cap coefficient bits, expanded margin size and
   total exact operations. Use deterministic support-distance or graph
   ranking. On any decline, preserve the factor and resume existing logic.
   These are proposed engineering caps, not mathematical restrictions.
2. **Measure actual retained guards.** Compare against all current positivity
   arms on a frozen collection of retained factors, including the small
   factors from the Gluza33 witness's 18 guards and symbolic ZZ projections.
   Dense factors may simply exceed the first cap. The 6,938-term Gluza33
   source has two certified positive roots at its recorded prefix: proving
   that source positive is not the task. Its coefficient/separator guards
   are the possible target, subject to their own exact domains.
3. **Parameter margins only after a useful hit.** Attempt a graph-informed
   decomposition with parameter-dependent coefficients, using existing
   strict leaf certificates. A failed margin may rank a threshold chamber
   proposal, but cannot remove a guard. Keep all parameter chambers and
   ordinary root-ordering obligations. Do not first compute a global Landau
   discriminant or global coefficient-sign CAD.

Report attempted and new accepted factors, discovery and independent replay
times, certificate sizes, and subsequently avoided projection work. Compare
complete solves only when original decomposition verification finishes;
operand-level savings are not solve-time ratios. Include the threshold toy,
missing positive-axis assumptions, coefficient-budget reuse, altered integer
weights, a zero margin, and mismatched parameter signs as negative tests.

An inexpensive additional *rejection* diagnostic is an exposed negative
Newton vertex: a rational exposing vector produces a positive monomial ray
where that negative term dominates. This can avoid futile global-positive
certificate searches, provided the ray belongs to the certified domain.
It never removes a polynomial, and on a bounded or restricted chamber the
ray may be inadmissible. Absence of such a vertex proves nothing, as `P_k`
demonstrates.

The primary literature supports these proof mechanisms, not a prediction
that they solve Gluza33 or the remaining symbolic problems. This is a small
extension with independently checkable evidence and a clear failure path;
the exact positive-root witness and ordinary projection remain responsible
for all retained interior boundaries.
