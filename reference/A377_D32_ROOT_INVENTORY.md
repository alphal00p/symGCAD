# A377 D32: checkpointed per-block root isolation

This research diagnostic consumes the [authenticated SQF proposal and complete
signed product](A377_SQF_GCD_AUDIT.md). It does not rerun decomposition of the
whole source. It first replays the bounded proposal against the original
capture, discriminant candidate, full variable map, signed scalar and every
multiplicity. Only after exact canonical and signed raw product equalities
pass does it isolate any block.

The blocks have degrees/multiplicities `1/798`, `268/1`, `1478/2` and `3/12`.
Their degree-weighted sum is 4058. They are visited in degree order
`1, 3, 268, 1478`, retaining a completed checkpoint before starting the next
block. All roots of the coordinate factor, the cubic and the squared block
remain represented, including same-sign repeated-root seams.

The new example is
[`profile_support_block_roots`](../examples/profile_support_block_roots.rs).
It reuses a narrow example-only authenticated replay wrapper in
[`profile_univariate_content`](../examples/profile_univariate_content.rs).
No production source, source-admission limit, cache or root-selection rule
is changed.

## Meaning of the output

Each nonconstant block is passed separately to the existing exact
`isolate_union_observed` API. With one input, the returned labels are that
block's ascending full-real distinct-root ordinals. The API includes its
normal native root-isolation and repeated-root handling; the native isolator
may internally perform its own square-free work on that block. No supplied
square-free or irreducibility assertion is trusted.

Every completed record stores all exact rational isolating intervals, the
block expression digest and multiplicity, full map/source bindings, native
executable digest and measured isolation time. Counts below, at and above
four are exact. If an interval straddles four, exact evaluation detects an
actual root there or exact bisection separates it. Endpoints at four are
handled using the API's open-or-exact interval contract. At most 1024
bisections per root and 8192 total are allowed; exhaustion is incomplete,
not permission to assign a side.

**This is a per-block inventory.** Common roots between different blocks
are not identified. A sum of these counts is never reported as the number
of distinct roots of the original source. The exact product ensures every
source root is represented somewhere, but an exact union still requires
its own shared-root tests and interval separation. No ordinary CAD
completion or nonvanishing certificate follows from these inventories.

## Bounds, checkpoints and restart

One killable worker has at most 3600 seconds and 4 GiB, including source
parsing, exact product replay, native isolation, threshold classification
and checkpoint I/O. Inputs retain the content helper's bounded sparse
integer grammar and product limits. Roots are capped by each block's
degree, at most 4096; rational endpoint numerator/denominator strings have
at most 20,000 digits each. Each output record is at most 16 MiB. There are
at most 64 blocks, and a resumed history must total at most 64 MiB.

Each completed block is published atomically without overwriting an
existing record. An atomic manifest then lists its file name and digest.
An interruption can leave a completed block not yet listed by the
manifest; such an orphan is not automatically reused. A fresh output
directory is required even on resume.

`--resume-manifest PATH --resume-sha256 SHA` authenticates the previous
manifest and all its immutable block files, checks their complete source
and block bindings, bounds and interval shape, and replays the source
product anew. Reused native intervals are explicitly labeled historical
output, **not independently re-isolated or proved by their checksums**.
They are research evidence, never verifier authority. The resume does not
mutate old output. Before/after input hashes and the complete process
outcome remain in the new report; a killed or failed worker is incomplete.

Whole-process wall time is authoritative. Per-block native isolation and
threshold times are recorded separately; source-product replay and
publication are not silently charged to one root block. A subsequent
timing comparison would require matched builds and equivalent inputs.

## Focused validation

Three new tests cover a repeated root exactly at four, shared roots between
two blocks whose separate counts exceed their distinct union, and bounded
endpoint/ordinal/threshold rejection. All pass, together with the ten
included content/proposal tests; the ten standalone content tests also
pass. Builds used CPUs 14–15, two jobs, 4 GiB and a 300-second command cap.

An actual fresh/resume CLI smoke reconstructed signed `-4*s^2` twice. The
fresh singleton isolation completed; the resumed run retained the exact
same block bytes and labeled them prior native output. The respective
process walls were 0.609150421 and 0.708609180 seconds. These tiny fixture
times are not D32 measurements.

Frozen build evidence is under
`artifacts/research/support-block-roots-build-01/`:

| Item | SHA-256 |
|---|---|
| Frozen executable | `f341a6c64cfc43d218a2a19f2a874787b8859107f30fb3469246afb456791ac2` |
| Source/library/build manifest | `9bfe57dddcbfdbb4ea7e81c11f4d03244a19635353d466407b6577332b437141` |
| Root helper source | `a7af44023e17388511b007d534c2e1ec92f41632abb3749b1f5c044237301fe0` |
| Included content helper source | `c220079b1dee000c68e1987820fd49e35c1f02dc9735b9cf608ff7e85f639303` |
| Shared smoke block, fresh and resumed | `08d789de4fade7eb4ee598e7e73c4ce94be2c37625b1bb961dfdb84a3006bfee` |

Independent source review found no blocker in source/product binding,
singleton ordinals, threshold handling or the explicitly historical resume
path. The full D32 worker started on 2026-10-10 at 14:29:34 UTC on CPU12,
with 4 GiB and a 3600-second total worker limit. The external 3675-second
guard permits bounded supervisory bookkeeping after termination. Output is
under `artifacts/research/a377-d32-block-roots-01/`; the adjacent launch
script has SHA-256
`5e55f41ada089369bfbb26361c4a04e7f3d51cc7def179cd09841a9912d77e63`.

## Completed per-block measurement

The bounded worker completed in **1187.682187414 seconds**, with peak RSS
820,916 KiB, exit zero and untruncated output. All authenticated inputs were
unchanged. Exact canonical and signed raw source-product replay took
42.333512985 seconds; preparation including parsing took 46.085158211 seconds.
It did not rerun the source's native square-free decomposition.

| Block degree | Source multiplicity | Distinct roots in this block | Below 4 | At 4 | Above 4 | Native isolation seconds |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 798 | 1 | 1 | 0 | 0 | 0.003905902 |
| 3 | 12 | 1 | 1 | 0 | 0 | 0.001596405 |
| 268 | 1 | 56 | 41 | 0 | 15 | 1.447824966 |
| 1478 | 2 | 294 | 198 | 0 | 96 | 1138.558141164 |

The largest block's threshold classification took another 0.012461567 seconds
and required no bisections. Its 96 roots above four are retained despite the
block appearing squared in the original discriminant. A sign-change-only
inventory would not account for these repeated-root boundary possibilities.
This does not establish that any particular discriminant zero is a positive
root collision of the original higher polynomial.

All intervals and full-real ordinals are saved. Cross-block coincidences remain
unresolved; the table deliberately has no summed distinct-source root count.
This is one immutable debug-helper measurement, not a comparison with a
different algorithm or build. It shows that the dominant isolated cost in
this run was the degree-1478 native call, rather than product replay or the
other three blocks.

| Completed evidence | SHA-256 |
|---|---|
| Process outcome | `0ae239ac86bad1660eef9bd83887abb0a564dbaf54b97cb1bc74c8be6240a1f9` |
| Final report | `4529e6e86da89c8cd73320300e7bcd5850f234fa8e735c20182da5b98a32be62` |
| Final checkpoint manifest | `99efcd50bc0bfb49c70543973889f1d4d4cae9e075712f01d4fa2f87b5dc5bd0` |
| Degree-1478 block record | `ddc4bc009730e017b5d04cea3fa46ef5ad8f24c4e201b6c149fb61280c7f679a` |

The independent audit of the first three checkpoints checked their hashes,
source/map/product binding, consecutive ordinals, rational interval ordering
and exact threshold classifications. Its digest is
`263f0a09080eca8f3d4bc53373756c088a943e6ebfd62b7b9dd908e80139b558`.
The completed four-record audit also passes. It rechecked current input hashes,
the final manifest and each block digest, source/product/full-map bindings,
contiguous per-block full-real ordinals, strictly ordered rational intervals
and all threshold counts. Its path is `completed-independent-audit.json` in
the run directory, SHA-256
`fd6fb57fefe850370aa45ba7f314f641308d6009a003d54fd814d1026dc71e3f`.
Neither metadata audit reran native algebra or proves a cross-block union.

A separate exact endpoint comparison finds 50 closed-interval overlap pairs
between the degree-268 and degree-1478 blocks. The cubic's broad interval also
overlaps many other block intervals. These overlaps do not prove shared roots;
they show why concatenating the current intervals is not yet a separated union.
The metadata-only comparison is saved as
`cross-block-interval-overlap-metadata.json`, SHA-256
`b2ca973003fca67785c4b0ef5c3a4df89b6c5028d23f07e11359c09b86c7f13a`.

Source inspection identifies a concrete profiling target without assigning
measured blame: Symbolica `univariate.rs:1327` performs its own SQF before
VAS-CF on the positive and negative rays (`:1429`). Its `shift_var` (`:1046`)
uses `d(d+1)/2` coefficient add-multiplies for each Taylor shift. At degree
1478 that is 1,092,981 coefficient add-multiplies per shift. VAS repeatedly calls
this routine, but the current single native timing does not separate internal
SQF, Taylor shifts and other isolation work. A future bounded profile should
measure that split before changing the isolator.
