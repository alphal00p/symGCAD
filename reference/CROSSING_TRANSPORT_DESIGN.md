# Exact crossing transport: implementation proposal

This is a design and identity inventory, **not a new coverage certificate**.
The existing Rust symmetry helper was replayed against the 303/331 completion
aggregate on 2026-10-10. It checked all 28 unresolved originals against all 331
representatives in 3.710 seconds: 32 directed identities connect 12 unresolved
pairs, with **zero already verified donors**. No original completion can be
added immediately. The first-hour audit also reports four incomplete attempts
and 24 pending entries, not a new donor.

## Applicable originals and orders

Every pair below admits the same names and the exact involution
`T(s,t,x) = (s,5/9-s-t,x)`. All strict constraints match after normalization by
positive rational content, and the ordered split identity has orientation +1.
No Feynman permutation or reciprocal is required.

| Family pair | Sectors | Additional optional symmetries |
| --- | --- | --- |
| A / Ax12 | 317, 318, 377, 414, 381, 446 | 381: `(x0 x2)(x4 x6)`; 446: `(x1 x5)(x2 x4)(x3 x7)` |
| B / Bx12 | 471 | `(x0 x2)(x6 x7)` |
| C / Cx12 | 215, 231, 476, 247, 478 | 247: `(x0 x5)(x1 x6)(x2 x4)` |

The four unmatched entries are symbolic C470, D251, D375, and positive
`test_gluza_ex33/solve/3`. Thus there are 16 equivalence classes among these
28 originals in the searched class: 12 pairs and four singletons. The original
331 denominator and all 28 unresolved statuses remain unchanged.

The original symbolic ZZ group is `[s,t]` before the Feynman coordinates; it
permits either internal kinematic order. Transport requires the **actual donor
order to put s before t**. An `s,t,...` donor gives a legal counterpart in the
same order. A `t,s,...` donor, including the recent reversed A446 experiment,
does not meet this condition. Swapping order metadata does not repair it.
For example, the image of the donor cylinder `t<0` is
`t>5/9-s`; in t-first order its first-axis boundary depends on the later s.

There is no complete stored proof for any of the 24 paired unresolved
originals in the aggregate. Four tracked slow recipes explicitly start s,t
(A317, A318, A377, A446); the other 23 ZZ recipes leave order selection open.
A future representative campaign intended for transport should pin the legal
groups to `[s],[t],[Feynman variables]`, or check the selected actual order
before promising transfer. Original explicit/given orders remain mandatory.
Fixed-kinematics proofs do not substitute: `s=10,t=-3` crosses to `t=-58/9`,
not the counterpart's original fixed point.

## Minimal production interface

Propose a separate `crossing_transport` module, with no change to solve
heuristics or proof interpretation:

```text
transport(target: &Problem, source: SolveResult) -> Result<SolveResult>
verify_transport(result: &SolveResult) -> Result<Verification>
CrossingEvidence { schema_version: 1, source: Box<SolveResult> }
```

Version 1 fixes the map to the involution above, requires identical variable
names, requires s and t as variables, and allows no coordinate permutations.
It should derive every mapping and sign identity itself; no unverified mapping
table is authoritative. This narrow surface already covers all 12 pairs.
Introduce one optional serde-default evidence field, mutually exclusive with
all other top-level coverage fields. Preserve the complete source, including
its homogeneous/local/monotone or other nested evidence, without rewriting it.

Preparation and independent replay must:

1. Validate both problems and the source's explicit complete order. Require s
   before t; validate that order against the target's groups and any original
   explicit/given order. Require `complete_generic`, then independently verify
   the entire source proof. An identity or incomplete proof is insufficient.
2. Normalize both original inputs, including their rational parameters. Check
   `T(T(x))=x` exactly, match the full strict domain under positive rational
   scaling, and compare each split in input order. V1 can require orientation
   +1; any later signed extension must derive and apply each sign explicitly.
   Do not match only zero sets or substitute fixed kinematics silently.
3. Substitute T into every exported source polynomial using Symbolica and
   check the inverse identity. Retain stable polynomial IDs and level IDs;
   check each transformed level has positive degree in its owning axis and
   no later-axis support. Retain the exact transported exceptional inventory,
   including unused-in-cells seams. A polynomial automorphism preserves all
   zero sets and their nonzero status; no refactorization is needed.
4. Reconstruct every cell once, in source vector order, then compare each
   exported target cell exactly. Preserve cell count and split annotations.
   Check final iterator exhaustion so omission, duplication, and reordering
   fail. No full second output vector or source clone is necessary in replay.
5. Recompute optional compatibility merging from the transported raw cells
   under the target merge policy; do not copy the donor merge across a fiber
   reversal. Top-level trace and other coverage fields must be absent. The
   nested source proves exhaustion; transported samples alone never do.

Target solver heuristics need not equal donor heuristics; mathematical inputs,
actual order, explicit target order restrictions, and proof coverage do.
The transport operation needs resource checks between polynomial and cell
operations, bounded input/expansion limits, and typed incomplete/resource
failure. Exceeding them cannot yield a complete wrapper.

## Bounds and root selectors

At each stored rational prefix put `c=5/9-s_sample`. On the reflected t axis:

* sample `a` becomes `c-a`;
* the old upper bound becomes the new lower, and vice versa, with unbounded
  ends swapped;
* an isolating interval `[l,u]` becomes `[c-u,c-l]`, including exact roots;
* `Real(i)` becomes `RealDescending(i)`, and `RealDescending(i)` becomes
  `Real(i)`, with the **same index**.

The last rule is valid for distinct roots regardless of multiplicity or the
number of negative roots; it avoids computing `n-1-i` from a sample count.
It uses the existing selector precisely as specified in `output.rs`.
Reject a `Positive` selector on the reflected t axis in V1: a translated ray
does not remain the positive ray. On all other axes the coordinate, sample,
root ordinal/domain, and stored interval remain unchanged after substituting
the transformed prefix into the boundary polynomial. Positive selectors on
Feynman coordinates are therefore valid. The source proof supplies root
continuity; the triangular bijection transports it.

The ordinary global verifier must continue rejecting non-Real selectors.
Only the new wrapper justifies the reflected selector by exact transport of
an independently verified source. Existing comparison and positive-scale
transport already understand `RealDescending`.

## Shared outer refinements and limits of reuse

A **whole verified donor** maps directly to one complete counterpart. No
common refinement with that counterpart's existing attempts is needed. Since
s is fixed, the donor's first-axis s cells are unchanged by this map.

Combining independently solved parameter pieces or exporting a simultaneous
partition for both F polynomials is a separate task. Their t cuts and reflected
t cuts can cross inside an s cell. A shared outer CAD must include their exact
union and the lower-dimensional obligations establishing relative root order,
then restrict/replay each inner decomposition over the refined outer cells.
Simply concatenating piece cells or matching rational samples is insufficient.
Likewise, transporting a t-first donor requires actual recylindrification.
`comparison::compare_generic` constructs an exact full common refinement and
can test small cases; it is not a cheap production remedy for these orders.

## Tests and an existing small donor

Required tests: reflection involution with rational content; reversed bounds
and infinities; repeated roots and exact zero roots; Real/RealDescending
roundtrip; later Positive selectors; rejected Positive-on-t; wrong s/t order,
groups, fixed order, parameter value, constraint or split; changed polynomial,
sample, interval, exception, omitted/duplicate/reordered cell, or source proof;
historical deserialization; partial/capped source; nested homogeneity;
compatibility merging; exact common refinement against a fresh small solve.

An existing eight-cell symbolic A26 proof is a useful concrete regression:
its order is `[s,t,x4,x3,x1]`, its t selectors are Real, and its evidence is
homogeneous. The helper independently checked its exact identity with Ax12_26.
The already stored Ax12_26 proof uses `[t,s,x4,x3,x1]`, so the A-to-Ax direction
is directly eligible while that stored reverse direction is not. Both inputs
are already completed; this provides a wrapper test, not a new completion.

## Exact-check evidence

No new Cargo build, large solve, or wrapper implementation was used. Frozen
`corpus_symmetry` checked exact Symbolica polynomial/domain/split identities
under a 30-second, 2-GiB external limit on CPU9. Its original source and two
tests are documented in `CORPUS_SYMMETRY_AUDIT.md`.

* Current report: `artifacts/research/crossing-transport-design-01/identities.json`,
  SHA256 `127bfe7e0c1daf0d4875940c26ff701796a1aa087dd1ca9b48fa048b7326ae30`.
* Input aggregate: `artifacts/completion-aggregate-303.json`, SHA256
  `ec2e2d5fc08902740210c7a0afc5f57ed36ca2ac5eb4eeacc95e477729cfb17d`.
* Helper SHA256:
  `e9fcd30d1bb1fd9d9d7de232d65427bd6ee91620845b3de9b72a8cf4c19844`.
* Compact tracked identities and provenance: `crossing_transport_inventory.json`.
  This contains no large polynomials, exported proofs, or completion claims.

The auxiliary solved-pair scan changes only a copy of the aggregate's selection
flag so that the helper visits the already solved Ax12_26 target. It does not
modify the canonical aggregate or grant a new verified status.
