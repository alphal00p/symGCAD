# Opt-in weighted-square certificates

The bounded algebraic rule from the [connected-block diagnostic](CFF_ENDPOINT_COMPOSITION_DIAGNOSTIC.md)
is available through `solver.weighted_square_positivity=true`, together with
`positivity=true`. It applies to optimized projection, ordering trials, and
local guard pruning. `input_positivity=true` additionally allows the optimized
engine to certify whole input factors. Monotone premise discovery keeps its
existing portfolio. The rule is disabled by default for separate ablations.

There is no graph or topology dispatch: discovery consumes only a sparse
polynomial and exact coordinate bounds derived from the original constraints.
Connected cuts motivated a useful chamber experiment, but confer no proof
authority. The original symbolic ZZ domains still include the entire
complement of that chamber.

## Exact theorem and replay

After a recorded invertible bound shift or reflection, every active coordinate
must be strictly positive. The evidence reconstructs the complete signed
polynomial as

```text
sign * shifted(P) = sum_j w_j x^c_j (x^a_j - r_j x^b_j)^2 + R(x),
```

where each `w_j,r_j` is a positive rational, all exponents are nonnegative,
and `R` is a nonzero polynomial with strictly positive rational coefficients.
Every square term is nonnegative throughout a superset of the original
domain, and the residual is strictly positive there. Thus the signed original
polynomial never vanishes on that domain. Pure squares and zero residuals
cannot receive this certificate; an inconclusive search retains the polynomial
and its exact root obligations.

Fresh verification rederives the domain from the original constraints, checks
the bound orientations and variable map, and reconstructs the full identity
with Symbolica. It does not trust the discovery coefficient map or merely
evaluate samples. New residual evidence uses typed sparse coefficient/exponent
records. Before algebra, replay bounds their counts, dimensions and exponents,
and accepts only short literal rational coefficients, weights and ratios.
Expression powers/towers, zero denominators and oversized integers are rejected
before arithmetic. The outer polynomial spelling is compared with canonical
formatting of the bounded expected polynomial, rather than parsed again.
This hardening is specific to the new method; it does not claim to redesign
all historical proof formats.

Discovery cancels one negative monomial at a time using two available positive
diagonals. If the negative coefficient is `-c`, diagonal coefficients are
`a,b`, and their exponent midpoint matches that monomial, it requires
`c² <= 4ab`. A rational weight in `[c²/(4b),a]` leaves both diagonal
coefficients nonnegative. Four negative-term orders, three pair rankings, and
three rational weight choices provide a deterministic bounded portfolio.
Every final proposal must pass the independent signed reconstruction.

Hard caps are 256 terms, 16 mapped variables, exponent 32, combined rational
numerator/denominator height 512 bits, 32 squares per trial, 16 bound
orientations, and 72 trials shared across signs/orientations. Existing
`work_limit` bounds scans, shifts and replay work. External process limits
remain necessary for individual noninterruptible algebra calls.

Default serialization omits the new false/zero fields. Disabled optimized
projection fingerprints retain their previous spelling. Enabled optimized
trials append a versioned fingerprint suffix, conservatively separating their
projection and raw operation cache namespace. Local raw algebra identities
remain shared; their guard certificates are independently generated/replayed.

## Scope and limitations

The authentic expanded endpoint
`B=(x4+x5+x7)*F1+(x1+x2+x3)*F2`, with the exact `F1,F2` defined in the
connected-block note, gains a six-square certificate on `u<4`. Its graph
contraction, original signed `F`, parameter map and prefactor were verified
separately in that note. This does not solve original A446/Ax12 or close
`u>=4`; in fact B changes sign on the positive orthant for every `u>4`.

The unrelated family
`z*(x-y)^2+x^2*(y-z)^2+y^3*(z-x)^2+1` gains three squares on `x,y,z>0`.
The version without `+1` retains its interior zero seam. Its local lifting
example `P*w-1` retains the ordinary interior boundary `w=1/P` and both signs.

Partial cancellation across several diagonal pairs is deliberately absent.
For example the positive cyclic cubic
`z*(x-y)^2+x*(y-z)^2+y*(z-x)^2+1` merges its negative coefficient to
`-6xyz`; the current single-pair capacity is insufficient and discovery
declines. The larger-support PSD rule for C0 and the quadratic constant remains
[a separate research prototype](QUADRATIC_COMPONENT_CERTIFICATE.md).

## Validation and bounded diagnostics

The initial combined gate passed 31 existing domain tests, five quadratic
tests, and seven weighted tests. After adding explicit expected-polynomial
variable-map validation, the final weighted target passed all eight tests.
These include positive/negative orientations, simultaneous rational bounds,
serialized replay, a foreign-name map with identical formatted coefficients,
forged proof fields, work exhaustion, zero seams, and the exact local interior
root. An independent code review checked the theorem and replay path.

All six matched off/on solves completed and independently verified. Each pair
used the same frozen binary, input apart from the one option, fixed order,
cold process, one worker on CPU 3, 2 GiB limit, and ten-second solve limit.
The host reports AMD EPYC 9754 processors. No reference GCAD run or benchmark
ratio is claimed. These are single development-profile diagnostics, outside
the published benchmark timings; process wall times have coarse 0.01-second
resolution and include CLI/worker startup. Verification is excluded.

| Problem | Option | Solver seconds | Process wall seconds | Peak process RSS, KiB | Cells | Polynomial table | New weighted proofs |
|---|---|---:|---:|---:|---:|---:|---:|
| Contracted endpoint B, `u<4` | Off | 0.138886 | 0.36 | 12,308 | 20 | 14 | 0 |
| Same endpoint | On | 0.025870 | 0.07 | 12,352 | 1 | 7 | 1 |
| Three-variable family P | Off | 0.023832 | 0.07 | 15,504 | 6 | 7 | 0 |
| Same family | On | 0.005922 | 0.04 | 15,396 | 1 | 3 | 1 |
| Local `P*w-1` | Off | 0.031929 | 0.07 | 15,480 | 28 | 18 | 0 |
| Same local problem | On | 0.002726 | 0.05 | 15,416 | 2 | 6 | 1 |

The first two enabled examples record an input-factor certificate; the last
records a local guard certificate. The local result retains both signs at
`w=1/P`. The smaller raw cell counts remove unnecessary projection seams, not
interior zeros of the original split. The endpoint result completes only that
contracted chamber problem, not the original symbolic A446/Ax12 domains.

Artifacts are under `artifacts/research/weighted-production-01/`, including
all six TOMLs/results, independent verification reports, process measurements,
the runner, and complete source hash manifests. Frozen binary and input
hashes were checked unchanged after all runs. These generated artifacts remain
outside commits.

| Artifact | SHA-256 |
|---|---|
| Frozen final development CLI | `7fe8698b44671f051ad8c0d6afad20ff31ca78722dd2dcfde493b23e4c9c67bd` |
| `ablation-summary.json` | `6feb51bf5b882a9af7b385fe66348e6f111d4329fc027cf5ec5b3ca7a03149ee` |
| Full source hash manifest | `f3ed2e21bdf319600c86b30314c8e6531897bedb8d15b6952604693ad31d39c7` |
| `endpoint-off.toml` | `902b6f4a75b32ac44e262190b116d8fbe0fb04323b3d39e079b0135ef226d7a1` |
| `endpoint-on.toml` | `17e92e281bfbbe1a03145cb940c707cc24a28cde9e56b9ab2d982014d7fdf565` |
| `family-off.toml` | `da10577c1e078cdda92dd56087ca77c22d94a63dc9f4c5aab1370f621e1f0df9` |
| `family-on.toml` | `d41241a09eee5e3e9f3ff7523c959c7c24420872632ad1b323a64692e27823cd` |
| `local-off.toml` | `ecc485767d4ec068ab8bb7a29b742e2b9158d59b002a73e339745683522f2fc5` |
| `local-on.toml` | `c9ee1e08188c06fe2d4177b9bd5b086b6c062eb2e8fce931038047844ece3642` |
| Enabled endpoint result | `18e6a18a92a7bab4ee1abd6b6b3510abd9e3521307b20df0359bddf7021ada7b` |
| Enabled local result | `7c2db769bb9e5798bc25973c1cd340653a8ae7b85aaf6c63147a680f7b2728fe` |

## Full original-F conditional chamber: still incomplete

The next gate used the exact original symbolic crossed fixture
`0152_finiteAx12_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml`, SHA-256
`ca89810d876b2bcde20ffc86b18928380eb4b26a418b6ab0f1accbb382bf1e3d`.
The full F split expression, both signs, original constraints, groups and
`mt2=1,mz2=5/18` substitutions were preserved; only `t<4` was added.
The authenticated crossed channel is already `u=t`, so there is no kinematic
coordinate change. No endpoint B, C0 or other derived positivity premise was
inserted. The `t>4` complement remains unresolved and is retained in its own
TOML; `t-4` needs explicit exceptional-set accounting or specialization before
any future original-domain composition.

Both configurations use the local engine, fixed lifting order
`s,t,x2,x1,x3,x5,x7,x4,x8`, and the existing exact homogeneity transform with
positive anchor `x2` from the original domain. The scale block contains only
Feynman parameters and its anchor precedes the other members. Existing
homogeneity validation and cylindrical reconstruction are responsible for
that reduction; no manual `x2=1` substitution is assumed. The inputs differ
only by `weighted_square_positivity`, apart from the matched time limits.

Each run is cold, uses the same frozen development CLI as above, CPU 3, one
worker and a 2 GiB cap. The first stage allows ten seconds and the second
sixty seconds. All four results are **incomplete**, so there is no complete
chamber, final cell count, upstream ratio or original corpus credit.

| Option | Limit, s | Wall, s | Peak process RSS, KiB | Outcome |
|---|---:|---:|---:|---|
| Off | 10 | 10.04 | 67,892 | Worker timeout |
| On | 10 | 10.05 | 55,392 | Worker timeout |
| Off | 60 | 60.04 | 150,532 | Worker timeout |
| On | 60 | 14.14 | 55,476 | Local projection refinement limit |

The baseline's final progress snapshot is stale at 2.20 seconds in the longer
run: 389 speculative nodes, 133 refinements, 278 projected pairs, and prefix
`(s,t,x1)=(5,7/2,1)`. It does not identify the subsequent algebra call; no
specific resultant/discriminant diagnosis is inferred without an operation
capture. The enabled ten-second snapshot is at 8.90 seconds: 18,322 nodes,
6,616 refinements, 12,478 refinement cuts, 7,901 certified guard removals and
5,997 projected pairs. Its prefix is `(s,t,x1,x3)=(5,13/6,1,1)` after scale
reduction. Thus guards and traversal are active, but this count is not a
fraction-of-domain completion measure.

The enabled sixty-second run reaches the default 10,000-refinement cap and
returns normally with an incomplete diagnostic. This path does not retain
the final tree/progress report, so its final cut distribution cannot be
reconstructed from the output. The ten-second snapshot already has 4,600
subtree-cache evictions; neither that fact nor the higher node count predicts
completion under a larger cap. No blind cap increase was run. The previously
measured larger-support C0/A-constant proof gap remains a separate candidate
for the proposed quadratic-component certificate.

Inputs, unchanged split-line comparison, explicit complement, provenance,
runner, full incomplete results and timing records are under
`artifacts/research/full-f-chamber-01/`. Input and binary hashes match before
and after all four runs. The compact `ablation-summary.json` SHA-256 is
`90231fabbdeadbdd598a344574377298f4c4eb95832a9c63c40bb42c4814fb62`;
`provenance.json` is
`35f3c16a3e2384a8a5b72eea525e821ce690ea5c173d788b2cac430c9e98c3f3`.
The sixty-second enabled result SHA-256 is
`4dbc11edac878b6517aebdf5d5dad1319be6a9cd78893e1c9dcaf66bc16ebc18`.

### Bounded cache-only follow-up

Read-only inspection showed that the enabled run reached cache **count** caps:
32,767/32,768 retained subtree nodes at only 11.99/33.55 MB, and 4,096/4,096
learned-cut entries at 3.83/8.39 MB. Counters do not record whether an evicted
prefix is requested again. Learned-cut IDs are summed across entries, so they
do not measure globally distinct mathematical refinements. Every refinement
adds a cut absent from its current stack, but restarting that stack can still
rediscover a previously evicted hint.

A matched cache-only diagnostic therefore repeated the enabled ten-second
input with default 32/8 MiB subtree/learned caches versus 128/16 MiB. The
existing proportional count caps rise from 32,768 to 131,072 subtree nodes
and from 4,096 to 8,192 learned entries. Both configurations keep the same
frozen binary, 2 GiB worker limit, mathematical options, order and 10,000
refinement cap. No production code changed.

| Cache MiB, subtree/learned | Wall, s | Peak process RSS, KiB | Snapshot age, s | Refinements | Subtree evictions | Learned evictions |
|---|---:|---:|---:|---:|---:|---:|
| 32/8 | 10.04 | 55,420 | 8.84 | 6,609 | 4,591 | 1,558 |
| 128/16 | 10.04 | 67,724 | 9.32 | 7,156 | 0 | 0 |

Both results time out and remain incomplete. The expanded cache retains
73,349 subtree nodes and 6,099 learned entries. Its subtree hits are 7,883
versus 7,171, and learned hits 11 versus four, seeding 121 versus 44 cuts.
Snapshots have different ages and the traversals are not synchronized, so
these counts establish removal of the measured eviction pressure, not a
completion speedup or a proof that most refinements were duplicate work.
No larger-cap or longer repeat follows from this diagnostic alone.

Artifacts and checked input/binary hashes are under
`artifacts/research/full-f-cache-01/`. The summary SHA-256 is
`060071d54bb70a2c4caccac96c7496e38e15de0a5ece30e56758f7a4fed53ab6`.
