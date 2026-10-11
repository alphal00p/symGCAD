# Bounded source admission for interval root certificates

The local engine now accepts `local_descartes_interval_max_terms` independently
of its guard-size limits. The default remains 4,096 terms, and explicit values
must lie between 1 and 65,536. This is an opt-in resource policy for a general
class of positive-root certificates, with no fixture or topology dispatch.

The motivating [captured-operand measurement](GLUZA33_INTERVAL_BUDGET.md)
found an affordable two-interval certificate for a 19,942-term source whose
global Descartes bound was unsaturated. A larger source does not necessarily
have large coefficient guards. Admission and resulting proof size therefore
need separate limits.

## Exact obligations and resource bounds

Every admitted interval cover must span the complete positive ray, use exact
rational seams, and have zero or one sign variation on each transformed
coefficient sequence. Leading coefficients, the coordinate endpoint, every
finite seam and every nonzero transformed coefficient remain projection
guards. A repeated positive root or an inconclusive cover falls back to the
ordinary discriminant. No interior root boundary is deleted.

Discovery, stored-template reuse and independent verification use the same
serialized source cap. Missing historical fields default to 4,096 and default
serialization remains unchanged. Invalid caps are rejected even when the
interval option is disabled. Source-budget declines have their own diagnostic
counter, separate from exhausting the finite template stock.

The downstream limits remain unchanged: elimination degree 3–32; at most
4,096 terms per guard, 65,536 terms over all guards, 256 distinct guards and
256 coefficient slots; coefficient height at most 512 bits and separator
height at most 256 bits; at most eight positive roots and 64 intervals, with
discovery depth at most 32. The lifetime template stock remains two by
default, configurable only within the existing range two through eight.

These are resource limits, not new mathematical hypotheses. An individual
native algebra call still requires the worker process deadline.

## Validation

All **633 tracked fast tests across 47 targets passed**, with zero failures
and two deliberately expensive tests ignored. Input hashes were unchanged
throughout the run. The 34 focused checks also passed. See the exact commands
and hashes in [the validation record](COMBINED_FAST_VALIDATION.md).

The new tests include a generic 5,445-term primitive source admitted by an
8,192-term cap, discovery and reuse, exact reconstruction of every raw guard,
full projection replay, altered cover/sign/cap rejection, and JSON/TOML
round trips. A separate 21,125-term source passes the larger source gate but
is rejected because its endpoint guard has 4,225 terms. Thus increasing source
admission cannot silently increase the cost limits on the resulting proof.

The changes are recorded in local milestone `b135d55`; source snapshots and
focused/release artifacts are retained outside Git under
`artifacts/research/interval-source-budget-production-01/`.

The locked release build completed in 8 minutes 25 seconds. A fresh public
CLI solve of `(z-1)*(z-2)*(z^2-z+a)+1/1000`, with `17/8<a<5/2`, `z>0`
and interval source cap 8,192, produced three cells and one interval-based
discriminant omission. Fresh CLI verification accepted all three cells,
two stacks and ten rejected branches. All production/test source hashes
still matched the regression snapshot after the release check.

Release evidence in that directory's `release/` subdirectory:

| Record | SHA-256 |
| --- | --- |
| Frozen `symgcad` | `75657a38880452444f121e2692c4e871ce868f978a30128f607959004621eaa6` |
| `smoke-result.json` | `a1701578106425c6d1404b4e7baee5c92595bdf673805b58c5f5f3e08b49b6ec` |
| `verify.log` | `16e9a1fb13a13c5824c594f39011f0b7159e05c45fd056cb45efff163e2a63bb` |
| `source-check.log` | `0419b0862c0b4aae1c31e80964b02865aa1b62d9a5d1282dab1217d2cbe76cf5` |

## Original Gluza33 continuation

A fresh development worker used a 32,768-term interval source cap, the
previous 8,192-term saturated-witness cap, opt-in affine factorization,
positive-pair witnesses and adjacent-pair projection. It retained the original
eight positivity constraints and positive F constraint, original variable
groups, anchor `x0`, and order `[x0,x2,x1,x7,x4,x6,x3,x5]`. No research
factorization was inserted into the cache.

The worker ran on CPU 13 with a 600-second solve deadline, a separate
600-second verification allowance if it completed, and a 50-GiB memory
ceiling. It inherited 10,115 completed native algebra records. These are
warm-cache development measurements; earlier discovery and campaign costs
are excluded and they are not release benchmark ratios.

The trial **timed out at 600.054950 seconds**, with **372,892 KiB (364.2 MiB)**
peak RSS. It saved **6,422 additional completed operations**: 6,292
factorizations, 77 discriminants and 53 resultants. All 10,115 inherited
records remained byte-identical; all 16,537 final records passed independent
key and output checksum checks. This metadata audit is not an algebraic
coverage verification. No complete result was exported, so verification did
not run and the original completion count remains unchanged.

The previously captured degree-24 source passed its interval proof and reached
downstream pair projection. One quartic-by-degree-24 resultant completed in
104.648949 seconds; a later quadratic-by-degree-24 resultant took 3.090709
seconds. These are recorded individual native operation times, not estimates
from progress snapshots. By the final published progress snapshot at
415.018516 seconds, traversal had reached level 3 at prefix `[2,2/3,1]`.
That snapshot is not the pending operation's start time.

The final unresolved native operation is factorization
`5b9185fc073a9f5e3d402596fc271650fafa18cc8c9b9a01a206b3273e723066`:
2,410 terms with degrees `[20,18,8,0,0,0,0]` in map
`[x2,x1,x7,x4,x6,x3,x5]`. It exceeds the affine preconditioner's current
2,048-term admission cap. This observation alone does not justify raising
that cap: degree-drop admission and shifted-factor cost require measurement.

Evidence is retained under `artifacts/research/gluza33-interval-production-01/`:

| Record | SHA-256 |
| --- | --- |
| `final-audit.json` | `6b367e7fd90673d5618bea21c13972012050609c68916721bb094c12dc4a1445` |
| Final pending capture | `aa44f4fb3da0145236f42254989933015ad6d6eda25cada5b49e65d83f60be43` |
| Final published progress | `6ec7637e1ef6e2f8480210ffd9fe9247cfc3a484f7361a3fbd250a6ee3402dab` |
| Frozen solver | `7a54aa6afab8ee73c9b236e24a09cfa2f84baa790e03d33bf26a619e9b930d90` |
| Frozen supervisor | `dcdff19056efe83c1bffc2af302968db35636a426075d92759e9b002105cf16a` |

The audit binds the actual worker input to the original mathematical fields,
the prepared TOML, launcher, binaries, process record and both cache manifests.
Completed work remains available for a later bounded continuation.
