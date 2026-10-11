# Exact shear preconditioning for factorization

11 October 2026. Implemented as an optional extension of
`solver.affine_factorization = true`, which remains **off by default**. This is
an algebra optimization: CAD coordinates, constraints, ordering, and interior
zero sets are unchanged.

## Generic policy and exactness

After the existing integer-translation preconditioner declines, examine the
bounded menu `x_i -> x_i ± x_j`. There are no graph, variable-name, kinematic, or
fixture-specific rules. The original minimum number of leading-layer monomials
across active axes must exceed one; an admitted image must reduce that minimum
to one without increasing its total term count. Rank all budgeted admitted
candidates by term count, maximum axis degree, coefficient height, then stable
axis/sign indices. Only the best candidate reaches native Symbolica factoring.
Leading-layer simplification is a heuristic, not a nonvanishing certificate.

Each shear is a polynomial-ring automorphism with inverse
`x_i -> x_i ∓ x_j`. Before factoring, Symbolica verifies the inverse source
identity in the full original variable map. After factoring, it verifies the
complete signed transformed product, restores every factor, and verifies the
complete signed original product, including rational units and multiplicities.
All original boundaries remain. Small solve/verify regressions explicitly retain
repeated same-sign interior seams and replay with the option disabled.

Admission limits are 4,096 source terms, at most 64 mapped coordinates and
2–4 active axes, input axis degree at most 32, total degree at most 48, and
4,096-bit input coefficients. Intermediate limits are 65,536 terms and
8,192-bit coefficients. A shared budget of 2,097,152 expanded-monomial upper
bounds covers discovery and inverse maps; discovery uses at most three quarters
of it. This estimates expansion size, not every native arithmetic operation.
Both signed-product replays share a 4,194,304 term-pair budget. Total-degree
preflights prevent exponent overflow. Pre-native inconclusive cases decline to
the ordinary path; exceeded post-native replay bounds are explicit errors.
The native factor call remains inside the caller's killable worker deadline.

Tests include unrelated coefficients, permuted names and inactive coordinates,
negative rational units, repeated factors, invalid maps, corrupt products,
degree/term/height/work caps, actual opt-in dispatch, serialization, independent
verification, and a deleted-cell rejection. No Symbolica dependency patch was
needed.

The tracked fast suite passes: 642 tests, zero failures, two ignored slow tests,
48 targets (51.48 s test body). The release build also passes a fresh
same-sign-seam solve and independent verification: four cells, all labelled
positive, with four checked stacks and ten rejected branches. Source hashes
were unchanged across validation. Release binary SHA256:
`1de04797348635f96695f303b8d5c0cc53dc1d14208078e93a4a30ae276bab60`.

## Captured operation and automatic-policy measurement

The authentic symbolic C470 capture contains 204 terms, degree vector
`(6,4,8,0,0,0,0)` in `(s,t,x2,x4,x6,x8,x7)`, and total degree 14. Native default
and bivariate-start controls each timed out after 60 seconds. A preliminary
hand-selected `t -> t-s` experiment reduced support to 175 terms, replacing a
five-term `s` leading layer by one monomial. Its transformed factors had 29 and
37 terms; inverse restoration gives original-coordinate factors with 27 and
45 terms and signed unit −1.

The subsequent **automatic optional-policy probe** supplied only the original
capture: integer translation declined and generic shear selection succeeded.
Factoring including admission/internal replay took 0.087845 s; a separate exact
signed-product check took 0.000166 s. Fresh-worker wall time was 0.103213 s,
peak RSS 9,244 KiB. This is one debug diagnostic, not a benchmark median or a
whole-solver speedup. The earlier hand-selected 0.02104 s native-factor timing
has a narrower scope. A 2,410-term Gluza capture declined both preconditioners;
its native fallback still timed out at 30 seconds.

The production C470 continuation independently completed that same captured
factor operation in **0.086409153 s**, preserving two multiplicity-one factors.
Its durable operation key is
`2cece6e84f327e49eaf3b4d1ec4ec78718499eb156d2285b65e81503776036bb`.

## Original symbolic C470 continuation

Original fixture: `0211_finiteC_6_470_10_0__0_5_1_0_1_0_1_1_1_symbolic.toml`.
The exact input audit preserves its original polynomial, `mt2=1`, `mz2=5/18`,
`s>4`, unrestricted `t`, all positive Feynman parameters, groups, and lifting
order `(s,t,x1,x2,x4,x6,x8,x7)`. Homogeneous anchor is `x1`. All previous local
solver flags are identical; only the optional factorization policy, cache path,
and worker limits change.

The first new continuation used one CPU (8), an 8 GiB worker limit, and a fresh
hash-audited copy of **635** prior completed algebra operations. The actual
count is 635, correcting the earlier proposed count of 634. It timed out at
**300.028 s**, using 224.503 s user CPU plus 34.750 s system CPU and **139,728 KiB**
peak RSS. There is no completed cell export and no verification result.
C470 remains unresolved.

The last emitted progress snapshot, at 278.057 s, records 364,266 speculative
nodes, 69,737 refinements, 134.140 s factoring and 55.674 s resultants. The run
was in level-1 kinematic projection. Capture changes and newly completed
checkpoints demonstrate continued work beyond the original factor stall.
The final cache contains **57,306** operation files, **56,671 new**; cached work
is preserved. The final active factor capture is incomplete. These are
warm-cache exploratory results; preprocessing and independent verification are
not hidden inside a reported finite solve time. A separately authorized
600-second continuation used this same frozen binary and a separately copied
cache. Cell exploration restarts; only completed algebra operations are
inherited.

That second run also timed out: **600.061 s**, 576.654 s user plus 12.721 s
system CPU, **227,048 KiB** peak RSS. Its final progress records 1,372,323
speculative nodes and 297,069 refinements, with 229.211 s factoring,
32.275 s resultants and 29.976 s root isolation. No cell export or independent
verification completed. It added **1,205** operations (641 factors, 494
resultants, 70 discriminants), bringing its cache to **58,511**; all 57,306
inherited files are byte-identical. During this run a 268-term, degree-300
univariate polynomial in `s` factored in 38.059 s into four factors, with
multiplicities `(1,1,1,33)`. Thus the shear removed one blocker, while expensive
kinematic algebra and extensive refinements remain. Neither trial contributes
a newly solved corpus problem or a finite benchmark solve time.

## Evidence and reproduction

Large operands, frozen binaries, full source archive, exact input audit,
operation manifests, and process reports remain untracked under:

- `artifacts/research/c470-factor-profile-01/` — initial native controls,
  hand-selected shear, and independent full signed inverse identities;
- `artifacts/research/shear-factor-policy-01/profiler/` — automatic policy,
  original-map factor export, independent signed replay, Gluza negative control;
- `artifacts/research/c470-shear-continuation-01/` — original 300-second run,
  `run.sh`, `input-audit.json`, source archive and before/after cache inventories;
- `artifacts/research/c470-shear-continuation-02/` — separate 600-second run,
  exact input/cache inheritance audits and captured degree-300 source.

SHA256 bindings:

| Item | SHA256 |
|---|---|
| Original captured factor | `9aa13570bccd6fd37bfb54b66e8b43a391d4d09e7efeb50e1699f1f894f06008` |
| Shear module | `66f8b6cd4c0c63a5e1f633422f9202c77ee8f05fc960339c238454c0daa01200` |
| Automatic-policy helper | `130d828ed29cd82dc5e63e7ba7bcf6ece1e989aae7c5e1a6813cf7952dd2adee` |
| Frozen debug symgcad | `315cc6393b510e9432f34a81ba55e4b68d950f67f346665fe261474443584b33` |
| Frozen hard_probe | `0372b619858c8b464580a6930fc79258631b9518f1db49cf689dd4e59cbd6ad2` |
| Original TOML fixture | `51b29016f5608fbf22490386fd2ab01e3c20007e692efd8729a05848e22a560a` |

Backend: `symbolica-v3.0.1-1-g75f83500`, supplied license enabled. The note does
not claim that coefficient sparsity is the diagnosed cause of the native
factor stall, or that this policy solves all graph families.
