# Proposed positive-endpoint quadratic-ladder wrapper

This is a mathematical design, not an implemented engine or a completed
conditional problem. Its generic coefficient conditions include the exact
[A446/Ax446 research ladder](A446_KINEMATIC_QUADRATIC_LADDER.md), but discovery
must use degrees and identities, never graph names or fixture identifiers.

## Sufficient theorem in original coordinates

Let v denote any preceding base coordinates, followed in lifting order by
y,x,z. The original domain has x,y,z>0. Suppose exact original-map identities
and full-domain certificates establish

```text
F = q*z+B,       B(v,y,x)>0,
q = a(v,y)*x²+b(v,y)*x+c(v,y),
a(v,y)>0,       c(v,y)>0,
b = b0(v)-b1(v)*y,       b0(v)>0, b1(v)>0,
D = b²-4*a*c = d2(v)*y²+d1(v)*y+d0(v),       d2(v)>0.
```

All positivity claims hold throughout the appropriate original positive
coordinate domains above the base cell. In particular, a and c are certified
for **every** y>0, not only a rational sample. Put theta=b0/b1>0. The exact
discriminant identity and b(theta)=0 imply

```text
D(theta) = -4*a(theta)*c(theta) < 0.
```

This is a substitution consequence of the whole-domain a,c certificates.
It does not require the positivity search to rediscover the expanded
polynomial −b1²D(theta). A verifier can alternatively check that polynomial
and a separate strict certificate; either route needs exact positive
denominators. If a or c has degree greater than one in y, use the actual
positive denominator power in their individual substitutions, rather than
assuming b1² clears each substituted factor.

Since d2>0, D has two distinct real roots lambda<theta<rho. Thus rho>0 and is
the larger real root of D. For 0<y≤theta, b≥0 and q>0 for every x>0. For
theta<y<rho, D<0 and again q>0. For y>rho, b<0, D>0, and a,c>0, so q has
exactly two distinct positive roots r0<r1. All these root branches are simple
on their parent cells. Neither theta nor lambda needs a CAD cut under this
theorem: their possible variations do not change the positive q-fiber pattern.

## Exhaustive generic restoration

For each certified base cell, emit exactly these five open cells:

| y | x | z | sign F |
|---|---|---|---|
| `(0,rho)` | `(0,infinity)` | `(0,infinity)` | + |
| `(rho,infinity)` | `(0,r0)` | `(0,infinity)` | + |
| `(rho,infinity)` | `(r0,r1)` | `(0,-B/q)` | + |
| `(rho,infinity)` | `(r0,r1)` | `(-B/q,infinity)` | - |
| `(rho,infinity)` | `(r1,infinity)` | `(0,infinity)` | + |

The final boundary is the sole real root of F as a polynomial in z. Its
positivity follows from B>0,q<0, and its simplicity from q≠0. Export the
original polynomial and an exact root selector, not merely a formatted
rational quotient. Use the existing convention: rho is real ordinal 1 of D;
r0/r1 are ordinals 0/1 of q; the F root has ordinal 0.

The actual F=0 interior sheet is retained as the common boundary of the two
middle z sectors. Sections y=rho and q=0 are additional omitted auxiliary
sections, even though F may be positive there. Include D and q explicitly in
the exceptional set, alongside F and inherited base exceptions. This is a
generic decomposition, with no silent claim of coverage of those sections.
The regions are disjoint, and the case analysis proves exhaustive coverage
outside those explicitly listed zero sets. A timeout, incomplete base, or a
missing one of the five branches prevents complete status.

## Conservative first discovery scope

- Start with one original split polynomial and base-only strict constraints,
  plus the exact coordinate positivity constraints for x,y,z. Extra upper,
  coupled, or sign constraints involving these three axes cause decline in
  the first version; do not silently drop them. Multiple original sign
  polynomials need a separate arrangement/composition proof.
- The first explicit `engine="quadratic_ladder"` accepts a fixed/given order
  only, respecting variable groups, and inspects its final triple y,x,z.
  No triple search or homogeneity reduction is performed. Require degree_z(F)=1 and
  degree_x(q)=2, b affine in y with the stated independent b0,b1, and
  degree_y(D)=2. Verify support independence for every coefficient.
- Normalize only with a recorded strictly positive constant factor. Bind the
  exact signed F to the original normalized input. A negative content or
  target sign must be handled explicitly, not assumed to preserve labels.
- Discover full-domain B,a,c,b0,b1,d2 certificates with the enabled generic
  certificate portfolio. Their proofs may use original strict coordinate
  assumptions, but must not use the conclusion F>0 or an invented C0>0
  premise. Every inconclusive premise produces a clear applicability error;
  this explicit engine does not silently fall back. The caller can select an
  ordinary engine separately.
- The reduced base carries precisely the original base constraints. It must
  be solved and verified completely before restoring cells. Any certificate
  specialized to a base cell needs an exact proof over its whole domain;
  this is a future extension, not permission to use its sample.

This class includes general positive-endpoint polynomials with a positive
quadratic fiber whose linear coefficient crosses once, and a discriminant
opening two positive roots. The number of preceding variables, origin of the
graph and masses are unrestricted subject to bounded discovery and the exact
certificate conditions.

## Independent replay and serialization

Store a typed versioned wrapper containing the selected original axes, exact
coefficient polynomials or references, domain certificates, base result and
complete restored cells. Apply sparse/literal size bounds before parsing new
proof data, as for the weighted and quadratic-component certificates.

The checker independently extracts q and B from the original F, extracts
a,b,c, checks all support/degree/affine identities, and recomputes D. It
rederives original domains and replays B,a,c,b0,b1,d2 positivity. For the
substitution route, prove theta>0 from b0,b1 and that substituting it satisfies
all y premises; then D(theta)<0 follows from the checked discriminant identity.
This avoids trusting a stored discriminant sign or an unproved rational
evaluation.

Verify the base independently. Reconstruct the expected five symbolic bound
patterns for each base cell and require exact agreement with the exported
axes, root ordinals, labels and exceptional-set references. Use Symbolica
isolation to choose and check rational interior samples and isolating
intervals; those sample checks supplement the global root-inventory proof.
Reject missing or duplicated branches, swapped selectors, altered input maps,
uncovered auxiliary sections, or a purported full result with an incomplete
base. Existing merging remains opt-in and cannot erase the F=0 boundary.

## Relation to the current engines and next gate

The monotone wrapper handles a zero/one-positive-root pattern controlled by
an endpoint; it cannot directly represent this zero/two-root opening. The
bilinear wrapper uses different degree and coefficient identities. Ordinary
local projection still discovers many auxiliary guards even after the
weighted B proof, as the incomplete full-F t<4 diagnostic shows. A separate
typed wrapper would therefore provide additional mathematical information,
rather than merely a longer projection timeout.

For the authentic crossed t<4 conditional problem, use the legal original
order `[s,t,x1,x3,x5,x7,x4,x2,x8]` initially, **without homogeneity**. Taking x2
as an early scale conflicts with its late restored fiber; taking x1 as scale
turns the four-coordinate homogeneous C0 square into an affine form beyond
the initial PSD-component kernel. Working in original coordinates lets the
weighted certificate prove B and the proposed PSD certificate prove c directly.
The remaining base constraints are linear and positive-coordinate bounds.

First validate the theorem with unrelated rational-coefficient families and
mutations of every premise. Then perform a bounded whole conditional-domain
solve and fresh independent verification. Even five verified conditional cells
would leave the original t>4 complement unresolved and require explicit t=4
exception/partition composition before original-corpus completion. No current
benchmark or completion credit is implied by this proposed wrapper.
