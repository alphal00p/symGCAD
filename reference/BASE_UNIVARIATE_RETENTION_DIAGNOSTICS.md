# First-axis support-basis retention diagnostics

The optional `local_base_univariate_basis` report now exposes the existing
registry's logical coefficient costs. Admission limits, source products,
root inventories, fallback decisions and the independent checker are unchanged.
These observations support later resource-policy decisions; they do not
establish a solver speedup or authorize larger bounds.

`retained_source_bits`, `retained_block_bits` and `retained_total_bits`
sum numerator and denominator bit lengths using exactly the admission
accounting. Blocks are charged again for each source record that references
them, even when the polynomial registry shares an ID. Multiplicity does not
multiply the stored block cost. These are conservative logical costs, **not
allocated memory or RSS**. The immutable registry only grows, so the retained
totals are also its lifetime peaks.

The `peak_source_*` fields record the largest individually admitted-size
canonical candidate's coefficient bits, height and term count. The text-byte
peak is measured only from an already constructed evidence string; no extra
formatting is performed. `peak_proposed_block_bits` is the largest proposed
block inventory's cost, while `peak_proposed_total_bits` includes the registry
already retained and the current candidate source and, once available, its
blocks. A proposal peak can exceed the retention limit even though the
candidate is never admitted.

`declines` counts the first existing failed check per static decline: map or
support, degree, term count, coefficient height, coefficient storage,
denominator storage, source count, or registry storage. Hits remain separate.
The existing `static_declines` counter is their total. There is no fabricated
builder text-size decline: the current builder has no separate text gate,
and the existing polynomial/name caps imply its text bound. The independent
source parser still enforces its existing 32-MiB and decimal-literal limits.

`last_attempt` contains only raw degree/term metadata, known canonical source
cost, proposed block/total costs, retention before the attempt, a fixed phase
and an optional decline reason. Unknown costs remain absent. No polynomial,
coefficient, environment value or credential is included. The report also
counts resource stops and algebra failures after entry into the optional
stage; it preserves their existing incomplete/computation-failure routing.

When `SYMGCAD_PROGRESS_PATH` is configured, ordinary atomic local-progress
snapshots carry these fields. Additional best-effort snapshots precede
ordinary fallback, follow native SQF return, precede product reconstruction,
and record successful admission. The existing pre-SQF snapshot remains.
Stored capture context is refreshed so a later operation snapshot retains
the observations, without forcing a new same-prefix capture write.
Diagnostic write failure cannot change algebra or status. No callback can
interrupt a native call: after a hard kill, the last snapshot is a historical
checkpoint, not proof that its recorded phase was still active at termination.

The report is diagnostic and uses serde defaults. Older reports still load;
editing these counters cannot make an invalid proof verify. The option-off
path performs none of this registry accounting or publication.

Focused regressions compare costs against complete source/block evidence,
including repeated references to the same block; check historical reports;
observe the pre-fallback cache counter before factorization; preserve metadata
through later capture; and exercise diagnostic I/O failure, native return
followed by a resource stop, and unchanged static decline classifications.

Validation: 31 focused tests passed, with the expensive authenticated replay
left ignored. The run included 10 library tests, 13 existing basis integrations,
two reporting integrations and six deferred-projection integrations. The log is
`artifacts/base-univariate-report-focused-02.log`, SHA-256
`8cd2b051c7c75fbc45df39de95075e6b6bfc3599e13399475d7455846ff7546e`.
Independent read-only review also passed after adding the same-prefix,
65-term fallback regression for a failing diagnostic path.

The [basis proposal](BASE_UNIVARIATE_BASIS_PROPOSAL.md) documents the exact
mathematical contract. The [D32 measurements](A377_D32_CONTENT_AND_BASIS_LIMITS.md)
and [completed native product replay](A377_SQF_GCD_AUDIT.md) explain why
per-source and cumulative registry costs must be distinguished. No limits
were raised by this instrumentation.
