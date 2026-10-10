# Cross-order algebra cache: proposal and bounded Ax318 audit

Exact operation reuse across different ordered variable maps is currently blocked. Reuse **within one map**, including alternative MODS eliminations through one `ProjectionCache`, already works. No meaningful solver speedup has been demonstrated for extending this behavior.

## Current separation

`src/optimized_projection.rs::operation_request` hashes the ordered variable names, numeric elimination axis, formatted operands and domain fingerprint. The loader requires the identical names/domain. Local projection's fingerprint also includes its selected order. Memory-cache polynomials likewise carry their ordered maps.

Pure factor, resultant and discriminant operations depend on their exact operands, not the CAD domain. Completed projection checkpoints, positivity/removal certificates, root owners, subtrees, learned hints and lifting cells have additional order/domain obligations. Sharing those proof objects by renaming is outside this proposal.

## Bounded evidence

The Ax318 warm-preparation manifest records 166,628 completed operation files: 38,713 retained for the final map, 127,915 excluded for another map, and no further same-map domain mismatches. The retained cache is 216,634,922 bytes: 28,375 factor, 5,161 resultant and 5,177 discriminant records. Excluded files are not necessarily duplicates or future hits.

One metadata-only sample inspected the first 8,192 lexicographic SHA filenames in that **single original Ax318 cache directory**. It skipped one file above 1 MiB and validated key/output checksums for the remaining 8,191. It read 29,933,188 bytes in 17.413 seconds under a 30-second / 1-GiB CPU10 limit. No polynomial algebra, build, cache mutation or solver run occurred.

The sample contains 88 ordered maps, all with the same named variable set/backend: 1,877 target-map records and 6,314 other-map records. Grouping by algorithm/backend, operation, **axis name** and existing input digest found 186 cross-map groups containing 442 records: 168 factor, 10 discriminant and eight resultant groups. All 186 also have identical output checksums. Eleven groups include the target map, with 15 other-map records.

Repeated recorded kernel time beyond the fastest record per group totals only **0.006385549 seconds**. The observed duplicates are cheap. This excludes formatting, hashing, disk, permutation and product-replay overhead, and establishes neither a future hit rate nor a speedup. General mathematical overlap remains unknown: v1 records omit operands, and monomial order, leading sign or pair order can change the input serialization even when the underlying operation is equivalent.

Provenance, all SHA-256:

- Warm manifest: `artifacts/research/ax318-segmentation-warm-hour-01/initial-cache-manifest.json`, `3e903c2ea9a51c4f7029382e64c085b26059cdfe7aa305d4011c7b78e4377ad5`.
- Sample report, including each read file's hash: `artifacts/research/ax318-cache-map-audit-01/report.json`, `a6b66965df102c1b804db2ea5bea1de24876f5baf17e00b258fff2ac5b885fec`.
- Bounded non-algebra inspector: the same directory's `sample.pl`, `a621a67168d943ae5500664c65cfa2f1ed124a72f94d75e53105e6966c2477bc`.
- Inspected cache source: `src/optimized_projection.rs`, `2bc745b640d7bdb7ccc0fe9a2e36839c570cfc62967d9582e7bf2af7d93c597b`.

## Proposed narrow design; not implemented

1. Introduce a separately versioned pure-operation namespace using a deterministic name-sorted **full** map and named elimination axis. Validate a bijection of identical symbols/namespaces, preserving inactive axes. Changes in points or substitutions must be reflected in the operands; matching names is insufficient.
2. Permute each exponent vector exactly, preserve rational coefficients and verify round-trip identity. Re-canonicalize in the destination map because leading signs and internal order can change. Sort resultant operands and factor/multiplicity pairs there, preserving associations.
3. Share only the existing canonical zero-set operation contract. For nonzero inputs, rational input scaling changes a resultant by `a^deg(q)b^deg(p)`; pair exchange contributes `(-1)^(deg(p)deg(q))`. A degree-at-least-two discriminant scales by `a^(2deg(p)-2)`. Signed-value APIs require explicit unit replay. Zero/constant/linear cases need their existing explicit conventions.
4. Keep domain fingerprints on projection/proof caches, while pure-operation keys include semantic algorithm/backend, coefficient domain, symbol namespace, full map, axis and exact canonical operands. Store bounded operands or sparse representations in v2 so future migration is auditable.
5. On return, validate checksums and support, reattach constants/zero to the full map, permute back and re-canonicalize. Factor hits must replay positive multiplicities, weighted degrees and the exact product. Product identity alone does not prove irreducibility; the unchanged independent checker remains authoritative. No domain certificate or interior boundary is omitted by cache sharing.

Do not rekey old JSON by editing its map. Safest migration is new v2 writes plus existing exact-map v1 fallback, promoting an old hit only when actual request operands are available and all old checks pass. A bounded factor-only importer could reconstruct its old canonical input from factors/multiplicities and require the original serialized input digest before transport. General old resultant/discriminant records need their actual operands; the output cannot recover them. No per-operation directory scan should be introduced.

Tests should cover all small-map permutations, inactive axes, changed axis indices, leading-sign changes, rational units, odd-degree pair exchange, nonconstant leading coefficients/degree drops, shared-factor zero, repeated factors and tampered maps/operands/outputs/multiplicities. Changed domains may share identical raw algebra while requiring fresh domain evidence. Add legacy/eviction/resource tests and complete small solver comparisons with fresh verification and every original boundary retained.

The next measurement would be a bounded exact operand-normalization profiler on actual expensive requests, including conversion/hash/product/I/O overhead. The present cheap-duplicate sample does not justify a broad production performance claim.
