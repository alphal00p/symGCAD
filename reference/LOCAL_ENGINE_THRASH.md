# Local-engine refinement thrash: pair witnesses, sibling retention, admission

Date: 2026-10-12.

**Code** (`src/local_projection.rs`, `src/local_projection/sibling_pool.rs`,
`src/local_projection/thrash_diag.rs`, `src/pair_witnesses.rs`):

| Commit | Change |
|---|---|
| `3988f53`, `af3577f` | retention, admission gate, diagnostics |
| `3aff953`, `40d1965`, `1600619` | retention memory budget |
| `c702742`, `8b2173a` | review fixes; the gate default reverts to 0 |
| `493e6b3` | measurements |

**Tests:** `tests/local_thrash.rs`, `tests/local_thrash_diag.rs`, and unit
tests in `sibling_pool`.

**Evidence** (machine-local): `artifacts/research/claude-2026-10-12/arch-local/`
for the diagnosis, and `local-thrash/` plus `local-thrash/fix/` for the
implementation measurements. The long-form mechanism and measurements are in
[LOCAL_PROJECTION_LIVENESS.md](LOCAL_PROJECTION_LIVENESS.md), section
"Refinement thrash". This note is the summary and handoff view.

**Status.**

* **Thrash fixed on the cheap problem:** the Gluza33 base piece `{a>0}` and 9
  corpus cases are fixed by turning pair witnesses off, or by gating them.
* **No Gluza33 region piece completes**, with or without retention. The pieces
  are now bound by cut-set inflation and expensive algebra, not by traversal.

## The pair-witness pathology (measured)

**How pair witnesses work.** `local_positive_pair_witnesses` replaces a pair
resultant by a rational separator `s` between the two polynomials' positive
roots, plus nonvanishing guards `p(s)` and `q(s)`.

**Why they inflate the cut set.** These guards depend on the separator chosen
at the current sample. They become new cuts at lower levels, and pair with
further cuts there. Each pair has two templates for the whole solve; after
that the pair falls back to the resultant. Termination with sample-dependent
separator templates is not proved
([LOCAL_PROJECTION_LIVENESS.md](LOCAL_PROJECTION_LIVENESS.md)).

### Runs on S1 = `{a>0}`

The Gluza33 base piece, anchor `x4`, order `x7,x1,x2,x6,x0`; one core, 4 GiB;
`arch-local/`:

| Configuration | Outcome |
|---|---|
| Gluza options (Descartes witnesses and intervals, pair witnesses, adjacent pairs), 570 s | **incomplete**: 20.4M nodes, 2.07M refinements, base stack up to 416 cuts. Only about 0.04 s of recorded algebra |
| same, pair witnesses off | **complete**: 178 cells, 0.026 s, verified in 0.04 s |
| plain local | complete, 205 cells |
| pair witnesses only, 60 s | incomplete, 1.59M nodes, 1,327 base cuts |
| `engine = "optimized"` | complete: 45 cells, 10 polynomials, 0.016 s |

### Diagnostic counts

From `SYMGCAD_LOCAL_THRASH_DIAG=1`, over 60 s of S1 with witnesses:

* **H1 (re-exploration) and H3 (cache thrash):**
  * 337,116 completed children (1,854,802 cells) were discarded by refinements;
  * 297,891 restart samples repeated already-completed samples;
  * subtree-cache misses: 370,162 after FIFO eviction and 190,911 never
    admitted (entries above 128 nodes).
* **H2 (separators):** distinct new cuts were
  * 281 separator guards;
  * 53 coefficients of separator-produced cuts;
  * 24 pair resultants.

  Every pair there had a tiny resultant (monomial bound below 200). The
  witnesses therefore only added sample-dependent polynomials.

The same mechanism dominated the final production Gluza33 campaign: 12,343 of
the 12,458 projected pairs were witness-stock fallbacks
([GLUZA33_STRUCTURE_2026_10_12.md](GLUZA33_STRUCTURE_2026_10_12.md)).

## Changes (all checked by the unchanged independent replay)

### `local_retain_siblings`

Opt-in; requires `local_stable_samples`.

* A stack visit keeps every completed child across its refinement rounds.
* Cuts only grow at a fixed ancestor prefix. So each new sector lies inside one
  old sector, and a retained child moves into the new sector that strictly
  contains its unchanged sample.
* A sample inside a coarse isolating interval is compared exactly with the root
  before reuse. Tests fail if that check is weakened to the interval hull.
* Obligations are merged exactly as for a fresh child. The deferred discovery
  loop retains in the same way.
* **Budget.**
  * Retained pools share a quarter of `memory_mib`, counted at 4 KiB per node:
    524,288 nodes at 8 GiB.
  * The measured cost is about 5 KiB resident per node.
  * After a budget or count refusal, the round reverts to the ordinary early
    break.
* **Why it stays opt-in.** It changes samples and therefore cell counts. All
  results are verified, but corpus runs were not compared by exact common
  refinement.

### Pair-witness admission

Opt-in; both options default to 0, the historical policy.

* `local_pair_witness_min_resultant_terms` admits a new template only when a
  dense monomial bound of the replaced resultant exceeds the value.
* `local_pair_witness_axis_budget` caps new templates per lifting axis.
* A decline uses the resultant, which is always admissible.
* The review reverted the first default (4096) to 0, so old inputs run and
  serialize unchanged.

### Read-only diagnostic

* `SYMGCAD_LOCAL_THRASH_DIAG=1` adds a `thrash` section to the local report.
* It never factors, interns or changes cache counters.
* `tests/local_thrash_diag.rs` checks that the proof and all counters are
  identical with the diagnostic on and off.

## Measurements

### Corpus

20 cases, slow-suite local recipes with witnesses, 60 s each. All completed
results verified.

**Gate at 4096** (first build):

* All 12 previously solved cases are still solved.
  * 9 are faster, by 1.19× to 1,796×. Examples:
    * A5_93 fixed: 15.73 s → 0.0088 s (37,721 → 67 cells);
    * D6_219 fixed: 31.53 s → 0.098 s.
  * 3 are unchanged at 1–8 ms.
* 6 of 8 previously incomplete cases complete (for example D6_219 symbolic in
  2.52 s).
* On 19 of 20 cases the gate admitted no template. It behaved exactly like
  witnesses off.

**Where the gate admits templates, it is not better than witnesses off.**
300 s runs (`local-thrash/fix/`):

| Case | Witnesses off | Gate 0 | Gate 4096 |
|---|---|---|---|
| Ax12_6_190 symbolic | complete, 0.03 s, 67 cells | incomplete | incomplete; 2 templates reused 62,339 times |
| A446 symbolic | incomplete | incomplete | incomplete; 2,164 of 2,225 templates admitted |
| Gluza33 original (campaign order) | incomplete | incomplete; 5,557 templates | incomplete; 37 templates, same progress as witnesses off |

**Retention:**

* **B6_469 fixed** completes only with retention: 112,124 cells in 9.91 s,
  verified in 40.2 s, 848 MB RSS. With a binding budget (`memory_mib = 4096`)
  it takes 308,993 nodes and 13.5 s; before the early break was restored, it
  took 616,404 nodes and 24.4 s.
* **Random problems.** On 200 random problems with a binding 64-node budget,
  retention used fewer nodes than stable samples alone on 181 seeds, and more
  on 2 (at most 1.10×).
* **The early break is a trade-off.**
  * On a randomized-options fuzz set, the restored early break needs 297,736
    nodes, against 180,591 for finishing every round.
  * But finishing every round is 2.07× slower on B6_469.

### Gluza33 region pieces

30 min each, one core. All incomplete. The two pairs of rows used different
controls:

* **Ppm and Ppp** come from the review round with the final budget. The control
  is stable samples alone.
* **Pmp and Pmm** come from the first round. The control is plain options, and
  the retention run predates the budget.

| Piece | Without retention | With retention |
|---|---|---|
| Ppm (R+−) | 12,798,020 nodes | 1,381,853 nodes, 322,104 reuses, budget full (peak 524,288, 4,161 refusals), 1.11 GB RSS; same `x7` sample with about 1,670 base cuts |
| Ppp (R++) | 5,804,029 nodes, reached `x7 = 3/2` | 774,046 nodes by 202 s, then one long projection |
| Pmp (R−+) | 11,063 nodes; algebra-bound from 30 s | 12,965 nodes |
| Pmm (R−−) | 10,313 nodes; 733 s in discriminants | 9,838 nodes |

* Pmp and Pmm were algebra-bound from the start, not bound by traversal.
* The stuck operations in those runs were discriminants in `x2` of two-variable
  cuts:
  * about 925 terms, degrees 39/42;
  * about 1,190 terms, degrees 42/47.

## Remaining root cause (open)

With unbudgeted retention, R+− left its first `x7` sector. But:

* the level-1 (`x2`) stacks hold 263–355 cuts, or 777 with Descartes witnesses
  or intervals;
* one level-1 projection returned 2,580 new base cuts;
* single level-1 discriminants of bivariate cuts (degree about 40, about 1,000
  terms) run for more than 25 min.

**Cause.** Siblings pick different but equally valid rules for the same cut:

* coefficient guards versus the discriminant;
* separation remainders versus resultants;
* interval covers.

The union of those choices inflates the ancestor's cut set.

**Proposed fix (not implemented).** Prefer guards already present in the
ancestor's cut set (consistent rule selection). This needs verifier support for
alternative rule choices.

**Separate gap: no region relevance.** Splits are cuts in every stack, so `b`
and `c` are projected against `a` and `d` over the whole orthant, not only
inside `{a<0, d>0}`. A relevance-aware local engine is the recommended next
step; see the HANDOFF October 12 section.

## Recommendations

* Turn pair witnesses off for new recipes unless a measurement shows a gain.
* Use the optimized engine for small pieces such as S1.
* The unresolved-campaign recipes already do both (witnesses off, retention
  on); see [UNRESOLVED_CAMPAIGN_2026_10_12.md](UNRESOLVED_CAMPAIGN_2026_10_12.md).
