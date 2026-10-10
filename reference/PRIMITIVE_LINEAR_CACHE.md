# Avoiding persistence for exact primitive-linear identities

`ProjectionCache::factor_basis` now avoids operation-file formatting, hashing,
reading and writing when an existing exact irreducibility test proves that a
small polynomial is already its own square-free factor basis. The change does
not remove a projection factor or use a sampled sign. Independent factorization
and proof checking retain their existing exact test and reconstruction.

For a polynomial linear in one variable, a monomial leading coefficient
coprime with the remaining coefficient proves irreducibility by Gauss's lemma.
The existing exponent-minimum test checks this condition exactly over the
rationals. The cache shortcut applies only when `terms × variables <= 256`;
larger and inconclusive inputs retain the ordinary path. Warm memory lookup
comes first, and the returned identity still uses the same bounded memory
retention. This avoids an additional precheck on warm hits. Historical cache
keys, factor bases, proof formats and verification obligations are unchanged.
The new diagnostic counter `primitive_linear_identities` defaults to zero when
reading older reports.

The motivation was the symbolic A414 hour campaign, whose shared checkpoint
directory retained 303,026 completed operation records, including its ordering
trials. This count is not the final worker's polynomial inventory and does not
establish that most records were trivial. An initial bounded scan of 10,000 directory entries selected only 30
small, distinct, unit-leading-coefficient linear factors. The exact record paths
and hashes were frozen and replayed for the following comparison.

| Work on the same 30 polynomials | Before, ms | After, ms |
| --- | ---: | ---: |
| Fresh memory cache, persistence disabled | 0.697 | 0.418 |
| Fresh memory and disk caches | 15.531 | 0.415 |
| Fresh memory, previously written disk cache | 4.326 | 0.409 |
| Warm memory cache, persistence enabled | 0.206 | 0.199 |

Every returned factor basis equalled the independently selected canonical
input exactly. Each persistent cold pass previously wrote 30 operation files;
the new path wrote zero and recorded 30 exact identities. These are medians of
three fresh cache instances within each diagnostic process on CPU9, using
development builds with optimized dependencies. They are microbenchmarks,
not three fresh-process solver measurements, native GCAD ratios, or evidence
of another original-problem completion. Parsing and source selection are
outside the measured cache loops; both processes replay the same saved list.

The broader independent monomial/GCD selector found 36 eligible records in
the same bounded 10,000-entry scan. All 36 took the new identity path and wrote
no files. This small observed share does not support a large end-to-end A414
speedup claim. Its report SHA-256 is
`385846c6b146d9ae15670a449fefd056647131fe60cda02f97f7a7e808222fed`;
the process record is
`daac7db33375b4566c663b1525599c428efd759a0c923ce2c63d658cf93d78d2`.

The research helper `examples/profile_linear_cache.rs` bounds directory scanning,
record size, expression support, repetitions, worker time and memory. It refuses
to reuse cache namespaces or output records. Its final selector independently
checks monomial leading coefficients and exact Symbolica GCDs; this broader
selector does not alter the frozen 30-record comparison list. Warm hits,
nonprimitive and repeated-factor fallbacks, the exact support cap, foreign
variable-map rejection and complete proof equality have focused regressions.

Evidence under ignored `artifacts/profile/linear-cache-01/`:

| Item | SHA-256 |
| --- | --- |
| Before binary | `c10d625789f1802615c91b9610fd61c8256c58ffabe48993577701b015775876` |
| After binary | `1cc4c5e180a5dfe8b7e75e760d0795f725b84acda1b8c131ec77bd17b2e001b1` |
| Before report | `d8e66caaf719352fb6b16f8ddb421abab1e8ecbb98ed807b5524a95ccac21ba4` |
| After report | `f6515dc444ee8979e344ec05c1f74a14be9fd8642c553fdd9098e4c297b1a52c` |
| Before process record | `fb40e1158ce57b7ded3d4aebaf5806bfe2c7675bce742483050fa157d83420b0` |
| After process record | `28d58e1bb8aa036a3bf179fc21497924ca7383d651107185200898182fc41a8a` |
| Final helper source | `1362ce87c388cc937b034c0731c4111481858fe54f6217180067269dc14683ec` |

The source change is `862bf60`; the before library has the production code from
`dec27e8`. Large cache records and external source trees remain outside Git.

The full fast suite passes **507 tests across 34 targets** after this change.
Its log, `artifacts/primitive-linear-cache-fast-tests-01.log`, has SHA-256
`84b2dd57b0a0b1965b771a6f81b8654d7368e5ed27a5b32f75562bcf28cd08ed`.

The subsequent release build from source `af1d942` completed in 454 seconds.
Its frozen executable, `artifacts/bin/symgcad-primitive-linear-862bf60-release`,
has SHA-256 `18f117360567d51134ea5c257c7b4c18a7097fe1d6b5f2979ceec26431d3ec90`.
A fresh public CLI solve of the bounded square with the `x+y-1` split and
deferred local projection returned two cells. Independent `verify` accepted
both stacks and four rejected branches. The build and smoke manifest is
`artifacts/primitive-linear-release-build-01.json`, SHA-256
`2da0249f8210de124744ed70b0e123e303a834d2e633ff5d714ea04769263ec9`.
This release validation does not add a corpus completion or a timing ratio.
