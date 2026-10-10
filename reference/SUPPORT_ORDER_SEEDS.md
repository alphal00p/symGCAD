# Bounded support-informed order proposals

`examples/support_order_seeds.rs` is a research helper, not a solver or a
graph certificate. It reads an original problem TOML and the corresponding
upstream fixture's literal `U` and `F` arrays, verifies exact normalized `F`
identity after the supplied rational parameter substitutions, and proposes
legal lifting orders. It neither invokes CAD nor changes production ordering.

Build and run after the documented dependency/corpus setup:

```sh
nix develop -c cargo build --example support_order_seeds
nix develop -c target/debug/examples/support_order_seeds \
  artifacts/corpus/symbolic/0081_finiteA_6_377_10_0__5_0_0_1_1_1_1_0_1_symbolic.toml \
  --fixture DO_NOT_PUSH_REFERENCES/gcad/tests/ZZ_integrals/finiteA_6_377_10_0__5_0_0_1_1_1_1_0_1.py \
  --output artifacts/support-seeds/a377.json --cpu 8
```

Choose a free CPU explicitly. Default worker limits are 30 seconds and
512 MiB; accepted maxima are 60 seconds and 2 GiB. The separate
`a377.process.json` records the enforced worker's wall time, peak RSS and
failure status. Existing output or process reports are refused. Report
publication is atomic and never replaces earlier evidence.

The shared parser in `src/corpus.rs` accepts literal data only. Its stricter
research entry point does not change the existing corpus importer. Generated
one-loop scripts are rejected rather than executed. Input files must be regular
and remain below their byte caps while read: 2 MiB for TOML, 4 MiB for the
fixture. Additional limits bound literal nesting, coefficients, powers, terms,
symbols and polynomial expansion before exact conversion. At most 12 Feynman
axes and 4,096 subsets are admitted; a 64-million support-visit estimate caps
the table work. Publication is capped at 16 MiB while serializing.
The expression preflight uses unnormalized Symbolica tokens, including for
parameter values: bounded integer exponents, traversal, expansion and rational
coefficient-height estimates are checked before any numeric power is evaluated.
Aggregate normalization still runs inside the killable memory-limited worker.

The `U/F` Feynman-variable maps must agree exactly and bind to declared
original variables. Auxiliary `eps,U,F` axes must be inactive. `U` must have
positive coefficients and homogeneous multiaffine Feynman support without
remaining kinematic dependence. These are consistency checks, not proof that
the fixture describes a particular graph. The report binds original file,
fixture, executable and normalized-math hashes, the exact parameter map,
constraints, splits, groups and fixed order.

For each subset `S` of Feynman axes, exact normalization and support extraction
provide:

- `valuation`: the minimum total exponent on `S`;
- `width`: maximum minus minimum total exponent on `S`;
- `interaction`: `sum_b n_b^2`, where terms are grouped by their exponent
  vector on `S`, and `n_b` counts remaining Feynman monomials in that block.

Terms sharing a Feynman monomial but differing in symbolic kinematic powers
are one support point. Exact fixed-parameter cancellations are applied first.
Symbolic coefficient-zero loci are not stratified: this is a formal support
heuristic, not a nonvanishing test.

Three projection-order policies are compared without fitted weights:
lexicographically smallest `(delta_U,delta_F,interaction_F,interaction_U)`;
largest first two increments with the same interaction tie-break; and
smallest `(interaction_F,interaction_U,width_F,width_U)`. Original variable
position breaks remaining ties. The report records every legal candidate's
features and each selected action, then reverses projection to lifting order.
Identical final orders are deduplicated while their policy traces remain.

Variable groups remain in their original order. Kinematic slots and their
relative order stay fixed, including mixed groups. A nonempty original
`solver.order` produces only that exact order. In particular, this helper
cannot replace an asserted order with a seemingly better heuristic.
The requested homogeneous anchor must precede its block ratios in every
proposed order. This is an ordering check; the helper does not certify the
applicability of a homogeneity or another solver transformation.

`metrics.total_preprocessing_seconds` measures the exact preparation function;
the worker metric also includes file reading/hashing before publication.
Use the enclosing process wall for end-to-end overhead, including startup and
report writing. A future solve experiment must include this overhead and
independently verify the original problem. The scores predict neither an
operation-count bound nor a speedup, and never justify omitting projection
polynomials or treating a partial result as complete. The rationale and
proposed ablations are in [the tropical literature review](LITERATURE_TROPICAL_SAMPLING.md).

## Exact preparation snapshot (2026-10-10)

The three symbolic originals below passed literal-map and normalized signed-F
identity checks. Sequential metadata-only workers used CPU 8, a 30-second wall
cap and 512 MiB each. These are single observations, not solver benchmarks or
cold-cache claims. Worker wall includes executable hashing and publication;
future portfolio totals must charge this full preparation overhead.

| Original | U/F support points | Subsets | Distinct seeds | Exact preparation (s) | Worker wall (s) | Peak RSS (KiB) |
|---|---:|---:|---:|---:|---:|---:|
| A377 | 11 / 36 | 64 | 3 | 0.00614 | 1.3974 | 8,004 |
| A414 | 11 / 36 | 64 | 3 | 0.00629 | 0.6475 | 8,008 |
| A446 | 15 / 57 | 128 | 2 | 0.01197 | 0.8095 | 8,288 |

All proposed lifting orders start with the unchanged kinematic prefix `[s,t]`.
Their remaining axes are:

| Original | Delay valuation | Expose valuation | Coefficient interaction |
|---|---|---|---|
| A377 | x6,x8,x5,x4,x3,x0 | x6,x5,x4,x8,x3,x0 | x8,x6,x5,x4,x3,x0 |
| A414 | x3,x8,x2,x1,x7,x4 | x3,x2,x1,x8,x7,x4 | x8,x3,x2,x1,x7,x4 |
| A446 | x7,x8,x3,x5,x4,x2,x1 | x8,x7,x3,x5,x4,x2,x1 | same as expose |

These inputs request no homogeneous anchor. A portfolio using a particular
anchor must regenerate proposals from that complete prepared configuration;
it must not silently move the anchor or copy an incompatible raw seed.

Reports, every candidate score, input/fixture hashes and exact parameter maps
are retained under `artifacts/research/support-order-seeds-03/`. The summary
also binds process reports and source hashes. Report SHA-256 values:

- `a377.json`: `0293e65d21587ca34a6b307de320d5a0c9f2dfeec1a0e9347ceddc9019fc2008`
- `a414.json`: `a347db9fc790ee0fa88975c92d0b7a5ceda245d26a20a328b797ef243aca44b3`
- `a446.json`: `3a868b374cc9bcb4248c84b976540c5653cdd4d9867a18a4e19bb1058c6059a9`

Frozen helper: `artifacts/research/support-order-seeds-02/bin/support_order_seeds`,
SHA-256 `01e34fdd4dd163c66b0a634030b10db517ce049a300e120e4250af3cccd40c4b`.
Example source: `12454f04f8b868dfac4881949d36bb82ac5d0312e9092a2efc7ef736fd6f1c6b`;
parser source: `6705273168a9822d786565840ca07426ccf0fd32d38dde0bc02bcdd1b1f4e5b4`.
The earlier `support-order-seeds-01` A377 worker completed but its supervisor
incorrectly expected status `success` instead of `completed`; both records are
preserved. The corrected status gate has a regression rejecting incomplete,
failed, truncated and missing-output workers. Snapshot `-02` remains retained;
its support tables, scores and seeds exactly equal the sequential `-03` replay.

Validation: eight helper tests, three strict literal-parser tests and four
existing corpus/importer tests pass. The unchanged importer also passed the
parent's full fast suite. Tests cover parameter cancellation, mismatched F and
maps, generated/executable input rejection, fixed/group/anchor ordering,
determinism, resource preflights and evidence preservation. In particular,
large numeric powers and exponent towers are rejected by raw tokens before
normalizing arithmetic. No original CAD completion or ordering speedup is
claimed by this snapshot.

## First bounded order screen

Proposals were regenerated with the existing homogeneous anchors (`x4` for
A377, `x8` for A414), preserving the original mathematical inputs, groups and
all other engine options. Each proposal and its prior control received a fresh
60-second cold-cache solve with the same frozen release on CPU0 (A377) or CPU3
(A414), one worker and 4 GiB. Neither order completed in either case:

| Original | Order after the fixed `[s,t]` prefix | Worker wall (s) | Peak RSS (KiB) | Status |
|---|---|---:|---:|---|
| A377 control | x4,x0,x3,x5,x6,x8 | 60.029 | 90,260 | timeout |
| A377 support seed | x4,x8,x6,x5,x3,x0 | 60.038 | 190,112 | timeout |
| A414 control | x8,x1,x2,x3,x4,x7 | 60.029 | 85,372 | timeout |
| A414 support seed | x8,x3,x2,x1,x7,x4 | 60.026 | 118,492 | timeout |

This single bounded screen establishes no speed ratio, winner or new coverage.
It does not include seed-discovery overhead in the worker times; that overhead
must be charged in a future complete portfolio comparison. Actual supervised
inputs were checked against the original constraints, splits, parameters,
variables and groups, and no result files were produced.

Evidence is in `artifacts/research/support-seed-screen-01/`. The independently
checked summary SHA-256 is
`ea20078c99802a55d61a9a6dfd8bc17f5dda8aefab58decdfd4a3a125f544cd4`.
The A377/A414 ledger hashes are respectively
`cefbbeedf562ad83ac90b7a84b60b2f44b0121e6cf46f6919f781d28269b24a4` and
`cc9164f57b04d67d10f77b6372494041bbb0e36723a629b2c23bbb4c385ad7db`.
All four workers used release SHA-256
`578d31e639230854c6a7de3515161aac98aaf4ee0794ac490633c86fb28dd6bb`.
