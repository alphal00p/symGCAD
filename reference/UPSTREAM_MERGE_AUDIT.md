# Original aggressive merging: defects and compatibility

This audit invokes the unchanged native reference at revision
`fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`. Its `gcad/gcad.py` has SHA-256
`b5d2be77de0e87ae3b311fb168774edf58889dcfaba4bbf5918e792c86049619`.
The research adapter constructs exact small cells and calls the original
`merge`; it does not implement a replacement merger.

```sh
timeout 20s nix develop .#reference --command python \
  reference/reproduce_merge_defects.py \
  --output artifacts/upstream-merge-audit-01/with-order-control.json
```

The report SHA-256 is
`37b4547e4aa0a7508b839e53ac1cf78fc48bab21014a12c22a788c0ce9927a7a`;
the helper SHA-256 is
`413f22155b0ba54797e450dc218980f3df44ad8cb08c624fd890b08c70c6d61c`.

## Stale upper-face entries can crash a valid merge

Use three adjacent one-dimensional cells A=(-infinity,0), B=(0,1), and
C=(1,infinity), with both finite roots owned by the same exact polynomial
`x*(x-1)`. Input order `[C,A,B]` raises
`ValueError: list.remove(x): x not in list` at original line 555. The other five
permutations return one merged cell.

The LIFO queue first stores B as entry 0. A absorbs B, removes its lower-face
entry, but fails to remove its upper-face entry. The combined cell is stored as
entry 1. C then selects the stale upper-face entry 0; cleanup tries to remove
entry 0 from its already-cleared lower-face list.

Original lines 556–558 have two errors in upper-face cleanup: the key omits the
other-axis key `kkey`, and the code appends the absorbed index instead of
removing it. Correct cleanup would remove from the same complete face key used
when inserting. No source patch was applied in this experiment.

## Polynomial allocation changes the result

Two adjacent half-rays separated by the root of `x` merge into one cell when
the two root objects share the same `sympy.Poly` object. They remain two cells
when their polynomials compare equal but are distinct objects. The report
checks both equality and distinct object identity before invoking `merge`.

The face dictionaries use `id(poly)` (original lines 494–496, 517 and 531),
although the later root-equality predicate uses exact polynomial equality.
Thus the public merger depends on object aliasing, not only exact boundaries.
Interned reference projection output may hide this defect in ordinary solves.

## Valid greedy results depend on scheduling

A separate control uses four unit squares forming a T: A=(0,1)x(0,1),
B=(1,2)x(0,1), C=(2,3)x(0,1), D=(1,2)x(1,2). Both native invocations return
normally and use shared polynomial objects:

| Input order | Merged rectangles |
| --- | ---: |
| A,B,C,D | 2 |
| A,C,B,D | 3 |

The original policy pops input cells LIFO, tries axes in ascending order, and
tries a neighbor below before one above. A combined cell is requeued
immediately. The historical Rust policy chooses the earliest live `(i,j,axis)`
pair. The indexed Rust implementation preserves that historical policy exactly;
it does not implement the original queue schedule. For example, on A,C,B,D its
first pair joins A to B, leading to the two-rectangle horizontal cover.

Consequently merged counts are not canonical. Exact source-policy count parity
also requires matching the raw-cell enumeration and boundary interning. Equal
raw counts alone do not establish either condition.

## Disabled-example disposition

The frozen campaign `artifacts/bench/disabled-aggressive-256527b/results/report.json`
used three fresh matched repetitions, the explicit original orders, a 120-second
limit per worker, and a 16 GiB cap. Its observations are retained unchanged:

| Original example | Source merged assertion | Native raw / merged | Rust raw / merged |
| --- | ---: | ---: | ---: |
| ex38 | 29 | 131256 / 1 | three 120-second timeouts |
| ex43 | 29 | 66 / 23 | 66 / 20 |
| ex44 T/C1/B2 | 28 | 224 / 19 | 224 / 20 |

The ex44 source itself comments that it obtains 19. All three old assertions
therefore disagree with the current unchanged native oracle. Successful native
ex43/ex44 results and different Rust counts are compatible with scheduling,
enumeration or object-identity differences; this audit has not transported
identical canonical raw inventories through both policies, so it does not
attribute those specific differences to one cause. In particular, the stale
deletion defect was demonstrated by a crash, not by these successful counts.

The benchmark skipped proof verification after detecting merged-count
differences. A separate bounded replay now verifies the first saved proof for
each of ex43 and ex44 with the same immutable Rust worker. Both return
`verified: true`, for 66 and 224 raw cells respectively. Verification checks the
raw decomposition and reconstructs the declared historical aggressive merge,
including its retained exceptional-seam convention. It does not claim equality
with the native merged cover.

```sh
nix develop --command bash -c 'ulimit -v 4194304 && timeout 30s taskset -c 5 \
  artifacts/bin/symgcad-linear-negative-256527b _worker verify \
  artifacts/bench/disabled-aggressive-256527b/results/0001-rust-0-result.json \
  --output artifacts/upstream-merge-audit-01/ex43-verification.json'
# Repeat with 0002-rust-0-result.json and ex44-verification.json.
```

| Artifact | SHA-256 |
| --- | --- |
| Frozen worker | `c9018af88c284ee7de2e6886fe5c3c21d937af16258ee045f33b41126f869c02` |
| ex43 proof | `1f4167fc895c7979e1a102a26a6ea73fdc116dfc13d54b93e65c6c3fd96f6456` |
| ex44 proof | `116fa9f21e89c6ce4c8f4438c2152e6c563153ca2855cac0d7caced7cba9894f` |
| ex43 verification | `a1c6ab4b8497df190955a2d2638eaf2f78c7dd3b0bcfedb82fc8b4642510988b` |
| ex44 verification | `6a8cdd720a46fdf138559600254ecfe4412072b3f0e488bd5ee0e8b335be75d9` |

## Minimal acceptance and any future compatibility mode

The meaningful current acceptance is exact raw-decomposition verification plus
exact reconstruction of the explicitly declared Rust merge policy, retaining
all exceptional seams. The count mismatches above remain recorded compatibility
differences; they are not a merged-parity pass. The native defects and stale
assertions are explicit upstream dispositions. Neither the raw M2 benchmark
denominator nor the 302/331 mathematical-completion audit changes here.

If reproducing the source queue schedule becomes an additional requirement,
introduce a separately named/versioned merge mode. Keep existing `aggressive`
semantics for historical proof replay. A corrected source-style mode would use
the original LIFO/axis/below-before-above schedule and insertion order, delete
absorbed indices from both complete face keys, and replace pointer addresses by
canonical polynomial IDs including root-index domains. The stored input's merge
mode can select the corresponding deterministic verifier reconstruction.
Compare both policies on identical exported raw inventories before claiming
schedule parity. This document proposes that path; it makes no production change.
