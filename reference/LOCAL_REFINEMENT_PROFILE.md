# B471 local refinement and exact sample reuse

The measured refinement work comes predominantly from **successful conditional
children returning additional obligations**, not repeated failed guards. A
matched 12-second experiment provides no additional completed problem and no
solver speedup claim.

The original symbolic B471 fixture is
`0174_finiteB_7_471_13_0__7_1_1_0_1_0_1_1_1_symbolic.toml`.
Both fresh-cache trials use anchor x6 and the legal lifting order
`[s,t,x6,x2,x4,x7,x8,x1,x0]`. Exact input comparison preserves the original F,
constraints, variable groups, mt2=1, mz2=5/18, s>4 and unrestricted t. Both retain
the local engine, positivity, Descartes witnesses/intervals, positive-pair
witnesses, adjacent pairs and sibling batching. The only algorithmic difference
is `local_stable_samples=false/true`.

Each trial has one worker on CPU 12, 8 GiB, 12 seconds, a million refinements
and ten million nodes. The controller has a separate 15-second outer timeout.
Both enable the existing read-only
[obstruction observer](LOCAL_OBSTRUCTION_OBSERVER.md). The two controllers
together take 24.586942761 seconds. Neither worker completes, so no independent
decomposition verification is invoked.

| Measurement | Stable samples off | Stable samples on |
|---|---:|---:|
| Worker wall time, s | 12.026684 | 12.036876 |
| Peak RSS, KiB | 63,416 | 63,628 |
| Last snapshot time, s | 11.901112 | 12.002834 |
| Refinements | 37,087 | 35,693 |
| Added cut occurrences | 63,216 | 60,874 |
| Speculative nodes | 184,358 | 172,348 |
| Subtree hits / misses | 43,680 / 82,561 | 47,948 / 75,597 |
| Subtree evictions | 77,220 | 70,229 |
| Learned-cut hits / misses | 7,253 / 75,308 | 6,852 / 68,745 |
| Retained-sample hits | 0 | 47,747 |
| Completed operation files | 1,647 | 1,740 |

Both observers record 15 failed `Stop::Guard` visits, including propagation,
with **zero repeated exact keys**, zero evictions and zero skipped observations.
The retained samples cover seven local guard IDs and 15 distinct rational
prefixes. Owner-axis counts are s:5, t:6 and x2:4 in reduced order
`[s,t,x2,x4,x7,x8,x1,x0]`. Numeric polynomial IDs are scoped to their Builder.

In the eager visitor, a refinement round needs a failed Guard return or a
successful child's missing current-axis requirement. At most 15 recorded rounds
could contain a failed return; therefore over 99.95% arise without one. This
rules out repeated-failure caching as the primary opportunity in this measured
prefix. It does not quantify discarded descendant work.

All 1,647 off-arm operation records occur in the on arm, with exact equality
after removing only elapsed time; the on arm adds 93 records. Distinct cached
factor-output strings number 1,570/1,662, principally at highest variable x2
(1,045/1,136); later-axis counts agree. This inventories exact serialized algebra
outputs, **not final live cuts**: primitive-linear bypasses do not all write
operation files, and later domain certificates can discard outputs. Final file
counts can exceed a stale progress counter.

Stable samples change traversal. The last prefixes are `[6,-1,2]` off and
`[6,-1,2,1/24]` on. More completed algebra and fewer visits in this pair establish
neither more verified coverage nor a timing ratio. A separately started progress
collector saw only the final snapshots; no per-level runtime distribution or
lifetime distinct-prefix count is claimed.

## Capacity pressure and the next diagnostic

Both subtree caches reach 32,768 retained nodes while charging only
12,777,587/12,631,299 bytes of a 32 MiB budget. Learned-cut caches reach their
4,096-entry ceiling at about 4.15 MB of an 8 MiB budget. The earlier 60-second
B471/D375 runs show the same count pressure. Evictions alone do not prove that
useful work will be repeated.

These limits already scale with configuration: `local_subtree_cache_mib`
increases the retained-node ceiling, and `local_learned_cut_cache_mib` increases
entry/ID ceilings. Effective budgets also respect total worker-memory fractions;
per-entry proof bounds remain unchanged. The earlier A414 cache-budget study
changed traversal, so unused byte headroom does not justify a performance claim
or an automatic default change.

The next useful diagnostic is a bounded shadow of **completed children within
one still-active parent visit**. At an actual global subtree-cache miss, record
whether that exact child prefix was already completed in the same invocation.
Also record admission declines, byte-versus-node evictions, successful-child
cut additions and discarded completed sibling roots. Count roots, not nested
descendant work, to avoid double counting. This would distinguish missed local
reuse from genuinely new fibers; the current counters cannot.

The existing cache's mathematical key remains valid when learned cuts change:
the entire exact rational prefix, within an immutable Builder/problem/map and
append-only proof registry. A `Built` payload includes the whole child proof,
cell count and transitive requirements. Parent cuts are discovery state, not a
hidden validity premise. A future per-visit pool may equivalently key by the
child rational while binding the fixed ancestor prefix. Every hit must preserve
all obligations, exact sample membership, final parent projection, root union
and exhaustive sectors. Samples cannot be moved across new boundaries or copied
to both sides; all interior F=0 sheets remain protected.

The proposed observer must change no solver choice and first pass exact
fixed-work off/on proof and operation comparisons. No new pool or production
instrumentation was implemented. The durable hint archive was not tested:
its current source declines these sample-dependent witness modes without a
persistent admission ledger.

## Reproduction and provenance

The ignored `artifacts/refinement-profile-01/` directory contains `run.sh`,
actual inputs, observer sidecars, summaries, operation inventories, hash checks,
`REPORT.md`, and the precise `ACTIVE_STACK_DIAGNOSTIC_PLAN.md`.
`input-audit.json` proves matched fields; `operation-comparison.json` records the
shared-record equality. No new binary was compiled. Frozen executable hashes
were checked before and after the pair.

| Artifact | SHA-256 |
|---|---|
| Original B471 input | `23d1b9942acdc37bfe8624e687ec9bef1f06fb5382de5834d2bb626a0eb029b0` |
| Release solver | `75657a38880452444f121e2692c4e871ce868f978a30128f607959004621eaa6` |
| Rust supervisor | `dcdff19056efe83c1bffc2af302968db35636a426075d92759e9b002105cf16a` |
| Run script | `92e34897dacf434839ed606acbed08213d011c87fa409967762abccd4226d769` |
| Input audit | `91f15ede5789410c6d073d7ed8dee05fb3d62e7b86d5d820907172a4f710a5ff` |
| Operation comparison | `433350d4ddf0a228ed8b60191d0b2aa13cd92ef33e311025ab41f03d57226c79` |
| Off summary | `5533e9ef81129e277a22c2f5850b490c132e8695f0b319d16f5f8a14a481dc53` |
| On summary | `ba8af62d4588ee99819af90209c0c173fd7ad20be7c7751c8ba3102809f7fa63` |
