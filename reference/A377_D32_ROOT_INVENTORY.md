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
This launch record claims no D32 root-isolation outcome before the worker
finishes and its output is audited.
