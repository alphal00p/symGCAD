# Joint homogeneous anchors and exact short projection prefixes

This research experiment evaluates legal variable orders by executing two ordinary
projection stages after a certified homogeneous anchor. It predicts which orders
are worth trying; it does not construct a decomposition, certify coverage, or
remove any region or projection guard. The implementation is
`examples/profile_linear_prefix.rs`.

## Scope and admission

The helper parses the original TOML with the production `Problem` parser,
substitutes exact parameters, and checks the explicitly supplied homogeneous
block with `domain::homogeneous_anchor`. Every normalized original constraint and
split polynomial participates in admission. An anchor must be strictly positive,
and its placement must respect the original groups and any fixed order. An
explicit anchor allowlist cannot override an input's fixed anchor. Kinematic
variables are not inferred from their names or placed in the scale block.

For each candidate it factors the initial basis and invokes
`ProjectionCache::project_one` twice, including leading coefficients,
discriminants and pairwise resultants. Factor-first rebasing prevents a shared
factor's zero resultant from being mistaken for a completed projection stage.
There is no positivity-based removal in this profiler. An initial linear stage
does not guarantee that the next projected basis is linear.

Candidate expansion round-robins the selected anchors and first/second-axis
ranks, with at most eight candidates and seed 0. Choices stay in the last legal
group; active variables precede inactive choices. A forced inactive stage earns
no linear-progress credit. Rankings use consecutive nontrivial all-linear stages,
then exact factor-count, term-count and coefficient-bit growth ratios. The
unused lifting-order tail remains in input/group order, **not** a Brown-ranked
order. All reported scores remain predictions of full-solve cost.

The profiler retains the original full variable map even after fixing its anchor
to one. Its hashes bind that map and the exact canonical basis. Production solves
remove/reorder axes, so their operation keys differ: these checkpoints cannot be
copied to a solve without checked variable-map transport.

## Bounded pilot and results

The pilot used the original symbolic fixtures, retaining `s > 4`, unrestricted
`t`, `mt2 = 1`, `mz2 = 5/18`, all positive Feynman coordinates, and the original
split polynomial. Original inputs were not edited. All selected restarts retain
the kinematic lifting prefix `[s,t]`.

The CFF history audit in
`artifacts/research/joint-anchor-prefix-plan-01/` identified these anchors as new
for projection/order trials, not for homogeneity or monotonicity inspection.
All-anchor monotonicity inspections had already found no peel. Merely counting
initially linear variables is also not new; the informative measurement here is
the exact second projected basis.

| Family | Anchors tried | Elimination prefix | Final factors / terms | Coefficient bits | Interpretation |
|---|---|---|---:|---:|---|
| B471 / 0174 | x6, x7 | x0, x1 | 8 / 56 | 6 | Best tied prediction |
| B471 / 0174 | x6, x7 | x0, x2 | 11 / 141 | 13 | Larger collision obligation |
| B471 / 0174 | x6, x7 | x1, x2 | 8 / 56 | 6 | Tied count, different retained basis |
| C470 / 0211 | x1, x6 | x7, x8 | 7 / 41 | 6 | Best tied prediction |
| C470 / 0211 | x1, x6 | linear axis, nonlinear axis | 10 / 199–267 | 16–17 | Loses the second all-linear stage |
| D375 / 0260 | x5, x6 | x0, x1 | 8 / 57 | 6 | Best tied prediction |
| D375 / 0260 | x5, x6 | x0, x2 | 11 / 145 | 13 | Larger collision obligation |
| D375 / 0260 | x5, x6 | x1, x2 | 8 / 57 | 6 | Tied count, different retained basis |

All 24 candidates completed their two stages. Reversing `(x0,x1)` for B/D, or
`(x7,x8)` for C, produces the same final full-map basis hash within the same
anchor. Different anchors have different hashes; no equivalence or cache reuse
between them is claimed. The two new anchors per family tie the best count/bit
scores, an informative **negative gauge result**: this short lookahead does not
distinguish them. The first anchor wins only by deterministic tie-breaking.

Controller times, including source/binary hashing and report processing, were
1.368423494 s (B471), 1.280762242 s (C470), and 1.295634283 s (D375), totaling
3.944820019 s. Peak worker RSS was respectively 11,412, 12,492 and 12,560 KiB.
The sum of measured child wall times was 0.998632042 s; the remainder includes
controller startup, hashing, process supervision and reporting. Child CPU totals
were 0.301169 s user and 0.248859 s system.
These are fresh-process diagnostic measurements, not release parity benchmarks
or solver speedups. All original-input and frozen-executable hashes matched
before and after each campaign.

Each profile used one CPU (CPU 6), a 4 GiB worker address-space cap, an internal
28-second shared budget and an **outer 30-second process-group timeout**. The
outer timeout bounds controller postprocessing as well as algebra workers;
internal worker deadlines alone do not hard-bound final hashing/report writes.
Partial candidate reports, standard operation checkpoints and pending operands
are retained if interrupted. Input, polynomial and report-size caps are explicit
in the source. No worker timed out in this pilot.

## Reproduction and evidence

Build/test commands are recorded in
`artifacts/research/linear-prefix-01/build.sh`; seven focused tests passed. They
cover groups/fixed orders/anchors, exact parameters before homogeneity admission,
linear-to-nonlinear growth, shared-factor zero resultants, inactive variables,
anchor-menu diversity and distinct second-axis collision obligations. The
default production solver is unchanged.

To build and test the committed helper without the ignored build script, enter
the locked development environment, set `SYMBOLICA_LICENSE` if required, and run:

```sh
cargo --config profile.test.package.symgcad.debug=0 \
  --config profile.dev.package.symgcad.debug=0 \
  test --locked -j2 --example profile_linear_prefix -- --test-threads=1
cargo --config profile.dev.package.symgcad.debug=0 \
  build --locked -j2 --example profile_linear_prefix
```

The resulting binary is `target/debug/examples/profile_linear_prefix`. Use a
copied/frozen binary for repeatable profiles; replace the artifact binary path in
the command below with that path when reproducing from a clean checkout.

The three commands in `artifacts/research/linear-prefix-01/run.sh` reproduce the
profiles into fresh output directories. For example, after sourcing the local
Symbolica environment:

```sh
timeout --kill-after=1s 30s taskset -c 6 \
  artifacts/research/linear-prefix-01/bin/profile_linear_prefix \
  artifacts/corpus/symbolic/0174_finiteB_7_471_13_0__7_1_1_0_1_0_1_1_1_symbolic.toml \
  --block x0,x1,x2,x4,x6,x7,x8 --anchors x6,x7 \
  --seconds 28 --cpu 6 --output FRESH_OUTPUT_DIRECTORY
```

`selected-summary.json` retains both tied anchor choices per family, their full
maps, legal restart orders and full basis hashes. The source/executable hashes
are:

| Artifact | SHA-256 |
|---|---|
| Frozen helper source | `b8534090393276ef3a66107961ab62c93bf8a18755691d8fbc9fb93aab0c742e` |
| Frozen helper binary | `087a5a745052a9e73fef356afb2a5e39b7bc6c67b2a7caa545348b4adcc29617` |
| B471 original input | `23d1b9942acdc37bfe8624e687ec9bef1f06fb5382de5834d2bb626a0eb029b0` |
| C470 original input | `51b29016f5608fbf22490386fd2ab01e3c20007e692efd8729a05848e22a560a` |
| D375 original input | `6f762b9fcae35e65a3f0bfc9f44c10830fafe10947ae334182e119d3539b9673` |
| B471 profile report | `a2a74482346c358db67ad9643145b0b3cd712c943885b597462ea5a7be63df9e` |
| C470 profile report | `e65d345748bf77c46403f405169706156d5dc7e8ec6022bb7a23969d2a6a5998` |
| D375 profile report | `35a400dfb2ddd038cb6c36fb1f447c9066b65591737df1cb2f31495af83b8b9c` |

## Exact-solve promotion protocol

`artifacts/research/linear-prefix-01/promote.sh FAMILY [ANCHOR]` selects the
corresponding exact original fixture and legal order from the saved report.
Available pairs are `b471 x6`, `b471 x7`, `c470 x1`, `c470 x6`, `d375 x5`, and
`d375 x6`. The default selects the first tied anchor per family. Each command
requires a fresh output directory and checks the original fixture hash.

Promotions use the frozen release solver and `hard_probe`, the local engine,
Descartes witnesses, interval covers, positive-pair witnesses and adjacent-pair
projection. Each has exactly one order, zero further promotions, seed 0, CPU 12,
16 GiB memory, 60 seconds solving, and up to 60 seconds independent verification
if solving completes. A 125-second campaign is enclosed in a 130-second timeout.
The follow-up probes explicitly permit 1,000,000 refinements and 10,000,000 nodes
while retaining the same wall/memory bounds; no production defaults change.
Fresh production operation caches are saved; no helper cache is imported.
Generated input JSON is checked against the source-bound original mathematical
fields. Coverage may be reported only after the independent verifier succeeds.

Solver SHA-256:
`75657a38880452444f121e2692c4e871ce868f978a30128f607959004621eaa6`.
Supervisor SHA-256:
`dcdff19056efe83c1bffc2af302968db35636a426075d92759e9b002105cf16a`.

The initial B471/x6 probe stopped incomplete in 5.797885068 s at the default
10,000-refinement limit (10,001 reported), rather than exhausting its 60-second
wall budget. It used 48,332 KiB RSS and wrote 868 operation checkpoints. Its
evidence remains under `promotions/b471-x6/`. The explicitly raised research caps
above were then used for one fresh 60-second trial per family; those trials have
the `-raised` output suffix. They do not reuse this initial probe's cache.

| Raised-cap trial | Worker wall time | Peak RSS, KiB | Last refinements / nodes | Saved operation checkpoints | Outcome |
|---|---:|---:|---:|---:|---|
| B471 / x6 | 60.040390533 s | 105,400 | 128,866 / 571,782 | 5,167 | Timeout; continued refinement |
| C470 / x1 | 60.020448192 s | 32,932 | 3,917 / 18,770 | 634 | Timeout; factor call pending |
| D375 / x5 | 60.029478889 s | 129,700 | 297,442 / 1,096,977 | 7,714 | Timeout; continued refinement |

B471's last progress reached level 3 and recorded substantial refinement/cache
turnover. C470's last progress was at 2.168713460 s, also level 3; its termination
capture identifies an unfinished factor call with a 5,632-byte exact polynomial
expression (input hash
`d5e272fdd1954174db7977330694a6cfe1b9b69507e6c7f1050734cd26c9f775`).
Completed-operation phase timers exclude that unfinished call.

D375's last progress reached level 5. The three raised-cap supervisors reported
181.699153195 s total, including their own preparation and reporting. All ran on
CPU 12 sequentially. None exhausted its raised refinement/node cap; the wall
deadline was binding. No result completed, so independent verification was not
invoked. These experiments add **zero verified complete cases**. Cheap two-stage
projections do not establish that later local refinement or factorization will
be tractable.

Every generated JSON's variables, exact parameters, constraints, split and
variable groups are identical to the source-bound original `Problem`; its order
matches the chosen restart and its research caps are checked. Those JSON-field
audits are input-preservation checks, not decomposition verification. Original
fixture hashes and both frozen executable hashes still match after the runs.
Audit JSON, source checks, full reports, exact pending operands and operation
caches are retained. `promotions/selected-summary.json` is the compact result;
`promotions/report-hashes.txt` records all summary hashes.

| Promotion summary | SHA-256 |
|---|---|
| Initial default-cap B471 | `660a3772ebd2900c70ec6b07aade3a3d6191c52d774baf2db32c8c87f1d25d42` |
| Raised-cap B471 | `497f21009f5366d35dd0a7fa2fef3262219d5f654f195c60413979fc07537114` |
| Raised-cap C470 | `240c614b7b422822daaa294c67c163f5a03a4f96d47988db6b8ebdb3e2e9c3e2` |
| Raised-cap D375 | `083ceb93560ac328dff1696477fee765c67592cf4330076995197edb84055c52` |

The useful next distinction is therefore between refinement-heavy B/D orders
and C470's specific retained factor operand. Blindly extending these prefix
scores into a solve-success prediction, or treating the tied gauges as a
demonstrated improvement, would be unsupported.

## Alternate C470 gauge after the algebra and reporting improvements

The tied x6 candidate was subsequently promoted with original order
`[s,t,x6,x1,x2,x4,x8,x7]`. Original F, constraints, exact parameters and variable
groups are unchanged; x6 is explicitly positive and passes the exact homogeneous
chart admission. Kinematics remain outermost. The frozen release was
`571f37a5c868aaa2055fb1c2a667332d0f7ff1df0a7e26d87a9edab493ee2c30`.
Its checkpoint namespace began empty: no algebra or proof was transferred from
the x1 gauge. The complete earlier C470 prefix-search cost, **1.280762242 s**,
remains separate from this solve and is not attributed to this candidate alone.

CPU 10, 8 GiB, a 300-second solve limit, existing 256/64 MiB conditional-cache
budgets, and the unchanged one-million-refinement/ten-million-node caps produced
an explicit **incomplete: local projection refinement limit** result after
**231.607739 s**, with **504,136 KiB** peak RSS. User/system CPU were
215.126596/9.130234 s. It reached 1,000,001 refinements and 4,258,270 speculative
nodes; these are activity, not final cells or certified coverage. No independent
CAD verification ran. An empty cell list in this incomplete result is not an
emptiness proof. No additional campaign followed this comparison.

The x1 release continuation also reached the same refinement cap, after
343.614492 s, but used newer warm algebra state. Gauge, traversal and warm state
differ, so the shorter x6 stopping time is not an isolated heuristic gain or a
time-to-solve ratio. See [BASE_PROGRESS_THROTTLE.md](BASE_PROGRESS_THROTTLE.md)
for that run and the separately controlled diagnostic-I/O improvement.

The x6 registry retained 3,164 sources with **zero source-limit declines**,
while its conditional caches recorded 1,861,475 subtree and 933,855 learned-cut
evictions. Its fresh inventory preserves 4,985 completed ordinary operations
(1,406 factors, 2,398 resultants, 1,181 discriminants) and 3,164 support proposals.
No invalid checkpoints, rejected support proposals or post-entry failures were
reported. This shows that raising the 4,096-source registry ceiling alone is
not the demonstrated missing lever. Both legal gauges remain unresolved.

Exact inputs, commands, cold-cache provenance, snapshots and result are under
`artifacts/research/c470-gauge-x6-01/`; result SHA-256
`d9866aba78232b15ad9f8ad2b733bb5aef4f5ca164ee35f1389288c4cc9da40c`,
final inventory SHA-256
`e9028734a9cc508dd707c2eef5a1f308b36d0d869a769928cd9bc7b6ba4e1f3c`.
The full comparison is preserved there in `REPORT.md`.

### Next diagnostic: completed-child obligation rediscovery

Cache pressure does not show whether evicted entries are requested again.
Repeated outer prefixes likewise do not establish an identical full-prefix or
cut-set revisit. Each refinement within one invocation strictly adds a new cut;
successful children can introduce obligations that require rebuilding sectors.
No exact cycle is demonstrated. A bounded, read-only diagnostic should:

- Scope a shadow ledger to each active parent invocation and exact rational
  prefix; record completed child prefixes and their transitive requirements.
- Observe actual ordinary-cache misses at previously completed child prefixes,
  newly added obligations, discarded sibling roots, and new versus retained
  sample choices. Do not sum nested descendant work as independent savings.
- Bound metadata and disable further recording on overflow; preserve skipped
  counts. Never change cache admission, cut selection, samples, proof acceptance,
  configured solver limits or failure routing. Throttle I/O; retain final output.
- Validate on/off behavior with fixed-work exact-result/operation comparisons
  before a short C470/B471 profile. Every retained child still owes all original
  projection, root-order and exhaustive-sector obligations.

Instrumentation still consumes time; wall-limited activity can change even when
mathematical choices do not. Fixed-work checks must precede timing profiles.

The detailed unimplemented design is retained in
`artifacts/research/c470-gauge-x6-01/OBLIGATION_REDISCOVERY_DESIGN.md`.
It follows the earlier [local refinement analysis](LOCAL_REFINEMENT_PROFILE.md)
and is more informative than another blind increase of the refinement cap.
