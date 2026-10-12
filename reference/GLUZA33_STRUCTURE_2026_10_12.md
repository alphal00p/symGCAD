# Positive Gluza33: graph structure, projection sizes and why the final campaign stalled

Date: 2026-10-12. This note collects the structural findings of the October 12
continuation about the original positive problem
`artifacts/corpus/symbolic/0338_test_gluza_ex33_solve_3.toml`:

* `F > 0` with `x0..x7 > 0`;
* homogeneous of degree 4;
* 143 terms.

**How the evidence was obtained.** Unless noted, these results are exploration
with Singular 4.4.1 and sympy, never production algebra. The tracks:

* `artifacts/research/claude-2026-10-12/g33-structure/` for the structure;
* `convex-theory/` for convexity and contract variants;
* `symbolica-audit/` for the backend;
* `arch-local/` for the stall;
* `shadow-prototype/` and `order-screen/` for estimates;
* `shared/` (`CONTEXT.md`, `F.txt`, `poly_*.txt`, `fp_*.log`).

All of these are machine-local; the track reports are the
`agent-*.final.txt` files there.

**What is proved where.**

* Identities marked *exact* were checked by exact expansion in the exploration
  CAS.
* Several identities were also replayed in Symbolica by the production
  `bilinear_general` planner, which discovers and checks the ratio-order
  identities and `b·c − a·d = H²`.
* Numerical screens are evidence only.

## Graph, symmetries, Symanzik polynomials

The graph is K4 on vertices Va, Vb, Vc, Vd:

* **Cycle edges:**
  * `x5` (Va–Vb), massless;
  * `x2` (Vb–Vd);
  * `x3` (Vd–Vc), massless;
  * `x1` (Vc–Va).
* **Subdivided diagonals:** two diagonals, each split by an external vertex:
  * `Va –x6– Pin –x0– Vd`;
  * `Vb –x7– Pout –x4– Vc`.
* **Kinematics:** massive edges `x0, x1, x2, x4, x6, x7` with `m = 1`; `s = 10`.
* **Symmetries:**
  * `σ = (x5 x3)(x2 x1)(x6 x0)(x7 x4)`;
  * `τ = (x2 x1)(x6 x7)(x0 x4)`.

  Graphica's canonization gives an automorphism group of size 4 with exactly
  these generators.

**Exact reconstruction.** `F = L·U − 10·V`, with `U = det M` (36 terms) and `V`
from a bordered determinant (45 terms). This reproduces `F.txt` exactly. It was
checked both in Singular and with Numerica `Matrix::det` (Bareiss).

## Convexity of F/U

`F/U = L − s·R`, where `R` is the effective resistance (Thomson's principle). `R`
is concave, so:

* `F/U` is convex and homogeneous of degree 1;
* `C = {F<0}` is a convex (spectrahedral) cone;
* `S = {F>0}` is the orthant minus `closure(C)`.

This is a deduction from Thomson's principle and the
[two-point matrix pencil](TWO_POINT_MATRIX_PENCIL.md). Here `F ∝ det S(x)` is
hyperbolic with hyperbolicity cone `C`. Convexity is used only as structure: no
production certificate relies on it.

**Limits of the convexity argument:**

* Massive coordinate directions are not in `closure(C)`, so no delineability
  comes for free.
* For multi-leg kinematics (the ZZ problems), joint convexity needs a PSD
  kinematic Gram matrix. That fails; there is an exact counterexample in
  `convex-theory/` and in [KINEMATIC_PORTFOLIO.md](KINEMATIC_PORTFOLIO.md).

**Massive edge `e`** (exact): `F` is quadratic in `x_e`,

* `F = m²·U_{G\e}·x_e² + (F_{G\e} + m²·U_{G/e})·x_e + F_{G/e}`;
* the leading coefficient is positive;
* `disc_e F = (F_{G\e} − m²·U_{G/e})² − 4·s·m²·H_e²`;
* every Q-irreducible discriminant factor of `F, b, c, d` in a massive variable
  has the form `P² − 40·Q²` and splits over `Q(√10)`. This was measured with
  Symbolica's `Q(α)` factorization; see `symbolica-audit/`.

## The massless pair and the Dodgson chain

**Bilinear form.** In CONTEXT naming, with `x5` last: `F = a·x3·x5 + b·x3 + c·x5 + d`.

| Atom | Terms | Exact form |
|---|---:|---|
| `a` | 21 | `β(A,B)`, with `β(p,q) = p² − 8pq + q²`, `A = x0+x2+x7`, `B = x1+x4+x6` |
| `b` | 36 | `(x1+x4+x7)·β(x6, x0+x2) + (x0+x2+x6)·β(x7, x1+x4)` |
| `c` | 36 | `(x2+x4+x7)·β(x0, x1+x6) + (x0+x1+x6)·β(x4, x2+x7)` |
| `d` | 50 | `F_{G/3/5}` |

**Core identities** (exact):

* `b·c − a·d = H²`, where `H` has 26 terms and degree 3.
* Monotonicity: `F_i·U − F·U_i = −10·H_i²` for `i = 3, 5`; `H_3` and `H_5` have
  11 terms each. So `F/U` is non-increasing in `x3` and `x5`, and the S-fiber in
  `(x3, x5)` is a down-set.

**Corner minors of U** (exact):

* `U11 = A + B`;
* `U10 = (x0+x2+x6)(x1+x4+x7)`;
* `U01 = (x0+x1+x6)(x2+x4+x7)`;
* `U10·U01 − U11·U00 = ψ²`, with `ψ = (x0+x6)(x4+x7) − x1·x2`.

**Chain identities** (exact; the planner discovers and replays them as
`RatioOrder` premises):

* `b·U11 − a·U10 = 10·hab²`, with `hab = x6·x7 − (x0+x2)(x1+x4)`;
* `c·U11 − a·U01 = 10·hac²`;
* `d·U10 − b·U00 = 10·hbd²` and `d·U01 − c·U00 = 10·hcd²`. Both `hbd` and `hcd`
  have only positive coefficients.

**Plücker-type identities** (exact):

* `H·U11 = a·ψ + 10·hab·hac`, and three companions;
* equivalently, `b·U01 + c·U10 − a·U00 − d·U11 = 2·H·ψ`.

**Implications** (exact, everywhere on the orthant):

* `a > 0 ⇒ b, c, d > 0`, so `F > 0` on the whole quadrant;
* `d ≤ 0 ⇒ a, b, c < 0`;
* `b ≥ 0` or `c ≥ 0` ⇒ `d > 0`;
* `H < 0` on `{d = 0}`.

**Fiber.** The fiber table was checked at 4,000 exact base points with 0
violations. All four `(b, c)` sign patterns occur inside `{a<0, d>0}`, and both
signs of H occur in `{b<0, c>0}`.

**Pinch witness.** At `x6 = 15` with all other coordinates 1, `(a,b,c,d,H) = (−110, −220, 200, 400, 0)`.
This point is why the `H = 0` seam is a strict-contract cut.

## Region pieces

The regions are governed by `sign(b)` and `sign(c)`:

| Region | Atoms needed in the base |
|---|---|
| R++ `{b>0, c>0}` | `a` |
| R+− | `b` and `c` only |
| R−+ | `H` |
| R−− `{b<0, c<0}` | `d` |

* The empty claim is `{d < 0}`.
* These are the four strict `bilinear_general` pieces. That engine uses the
  swapped naming; see [BILINEAR_GENERAL.md](BILINEAR_GENERAL.md).
* Under the opt-in projective contract the whole base collapses to the single
  piece `{d > 0}`; see [PROJECTIVE_SECTIONS.md](PROJECTIVE_SECTIONS.md).

## Projection sizes: full versus relevance-restricted

### Level 1

Eliminating `x0` from the base `{a, b, c, d, H}`; `fp_*.log` and `g33-structure/logs/`:

| Variant | Factors | Terms | Notes |
|---|---:|---:|---|
| Full | 15 | 750 | max factor `Res(a,d)`, degree 8, 237 terms |
| Exact level-1 drop (structure track) | 12 | 465 | drops `Res(a,d)`, `Res(b,d)`, `Res(c,d)`, which never vanish together with `y ≥ 0` |
| Strict cells (convex-theory track) | 14 | 513 | drops `Res(a,d)` |
| Generalized sectors (not implemented) | 8 | 398 | |

* Many resultants are perfect squares (Dodgson): `Res(b,d) = (5/16)²` and
  `Res(c,d) = (5/32)²`, written as (degree/terms)².
* The only genuinely new factors are `Res(a,d)` (8/237) and `Res(b,c)` (7/137,
  a difference of squares: the b/c crossing).

### Level 2

Eliminating the Pin chain, `x0` then `x6`:

| Variant | Factors | Total terms | Max degree / terms |
|---|---:|---:|---|
| Full | 86 | 15,327 | 20 / 1,339 |
| Exact level-1 drop | 56 | 8,091 | 16 / 707 |
| plus exact level-2 drops | 51 | 7,311 | 16 / 707 |
| Strict cells (convex-theory rules) | 71 | 9,180 | 16 / 707 |
| Generalized sectors and selected roots (not implemented) | 34 | 3,709 | 16 / 630 |
| Numerical co-relevance screen (evidence only) | 30 | 4,145 | 15 / 567 |

* The two tracks used different exact drop rules, so their strict-cell rows
  differ. Both sets of rules are exact; neither is implemented as a production
  projection operator.
* **Other first pairs** (full → generalized sectors and selected roots):
  * `x6,x4` (the final campaign's pair): 109 → 33 factors, max degree 32 → 24,
    29,190 → 5,104 terms;
  * `x0,x1`: 110 → 36;
  * `x1,x2`: 115 → 34.
* **Per-region screen** (structure track): the Pin chain `{x0, x6}` is best in
  every region. The cross-chain pair `x6, x4` reaches 74 factors and degree 28.

### Level 3

Pruned sets, `x0, x6`, then a third variable. Each entry is factors / max
degree / max terms:

| Third variable | R+− | R++ | R−− | R−+ |
|---|---|---|---|---|
| `x1` | 113 / 24 / 325 | 158 / 52 / 1,421 | 297 / 54 / 1,345 | 176 / 108 / 5,548 |
| `x2` | 108 / 24 / 325 | 160 / 52 / 1,431 | 290 / 48 / 1,147 | 173 / 99 / 4,959 |

* Level-3 polynomials are dense. At level 4 the univariate degrees would be
  about 576 for R+− and about 2,300–4,400 for the others.
* The production optimized engine on the strict pieces matches this picture:
  * Ppm reaches a bottom level of 22,853 polynomials, degree 156;
  * Pmm reaches 1,033 polynomials at level 3.

  See [BILINEAR_GENERAL.md](BILINEAR_GENERAL.md).
* Estimates of the strict structured decomposition are about 3·10⁶ cells
  (shadow prototype and order screen).

## Why the final production campaign stalled

**The campaign** ([GLUZA33_FINAL_FACTOR_PUSH.md](GLUZA33_FINAL_FACTOR_PUSH.md);
`artifacts/research/gluza33-final-main-axis-campaign-01/`):

* order `[x0,x2,x1,x7,x4,x6,x3,x5]` with anchor `x0`, local engine;
* 6,603.9 s, 1 CPU, 1.92 GiB peak;
* no result.

The `arch-local` track reconstructed what happened from the cache records and
captures.

### 1. A poor elimination pair

* The order eliminates `x6` and then `x4`: a cross-chain pair, the τ-image of
  `x7, x0`.
* At level 2 its full projection has 109 factors of degree up to 32. The Pin
  chain `x0, x6` gives 86 factors, degree 20.
* At the 3-variable level it hit the earlier factor gates: degrees [24,18,11],
  2,867 terms.

### 2. Pair-witness cut inflation

* **Mechanism:**
  * Pair-witness guards `p(s)`, `q(s)` depend on the separator `s` chosen at
    each sample.
  * They become new cuts at lower levels and spawn further pairs.
  * Termination with sample-dependent separator templates is not proved
    ([LOCAL_PROJECTION_LIVENESS.md](LOCAL_PROJECTION_LIVENESS.md)).
* **Campaign accounting:**
  * 12,343 of the 12,458 projected pairs were pair-witness stock fallbacks;
  * `max_refinement_cuts` reached 754;
  * pruning still omitted 445,976 rootless pairs and 327,567 non-adjacent pairs.
* **Same mechanism on a sub-piece.** It makes the cheap base piece `{a>0}`
  thrash: incomplete after 20.4M nodes in 570 s, versus 0.026 s and 178 verified
  cells with pair witnesses off. See [LOCAL_ENGINE_THRASH.md](LOCAL_ENGINE_THRASH.md).

### 3. A resultant storm in one stack

* After the last progress snapshot at 880 s, 431 resultants took 5,672 s of
  recorded time, apparently inside one projection of the `x7` stack at prefix
  `[2, 6/13]`.
* The four largest took 678.0, 592.3, 559.8 and 549.5 s. Each is a resultant
  in `x7` with bivariate output in `(x2, x1)`:
  * degree 249–274 per variable;
  * 42.7k–48.3k terms;
  * 870–977-bit coefficients.
* **Kernels:**
  * These went through deterministic Ducos. Interpolation handles one surviving
    parameter only, and segmentation was off and capped at 8,192 input terms.
  * Symbolica `resultant_crt` gave no gain on the one captured level-2 operand:
    37.1 s against 36.1 s for Ducos, with identical output.
  * The 550–680 s operands were never captured.

### Conclusion

The cost comes from the size of the cut set, not from the speed of a single
kernel. The order, the witness policy and the missing region relevance
(`b, c` projected against `a, d` over the whole orthant) all multiply it.

## Open items

* **Numerical drops.** The structure track's numerical co-relevance drops (for
  example F61 and F67 at level 2) need exact proofs before any production use.
* **F21/F22 positivity.** The apparent positivity of F21/F22 on the open
  orthant is unproved: Pólya fails because they vanish on the face
  `x1 = x2 = 0`.
* **Missing machinery.** A strict solve would need a relevance-aware projection
  (selected-root bounds plus verifier rules). Symbolica has no public
  subresultant chain; exposing the one its Ducos loop already builds is
  estimated at about 40 lines (`symbolica-audit/`).
* **Status.** The opt-in projective contract sidesteps all of this for the
  positive problem.
