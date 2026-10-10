# Adaptive native-GCAD comparison budgets

New paired comparisons limit each native-GCAD worker to

\[
\min\bigl(\max(300\text{ s},3t_{\mathrm{Rust}}),
           T_{\mathrm{configured}},T_{\mathrm{remaining}},3600\text{ s}\bigr),
\]

where `t_Rust` is the median fresh-process solve wall time of completed
Rust repetitions for that same mathematical branch, explicit variable
order and merge policy. Projection/lifting/merging phase times, verifier
time, timeouts, failed runs and truncated output do not supply that median.
If no completed Rust measurement exists, the adaptive ceiling is 300
seconds. The configured reference ceiling defaults to 3,600 seconds;
explicit shorter limits remain binding.

`src/benchmark.rs` preserves its original qualification sequence: every
reference qualification runs before any Rust timing is observed, with
the unknown-time ceiling of 300 seconds, further bounded by the configured
and remaining campaign budgets. The qualifying cohort and orders are then
frozen. For each qualified case, all requested Rust repetitions run first,
followed by the native repetitions under the resulting adaptive ceiling.
Qualification can therefore censor a case that would have completed under
a larger adaptive measurement budget; the report retains that distinction
instead of silently expanding the frozen cohort.

`examples/frozen_replay.rs` similarly retains its previously frozen cohort,
original inputs, order and merge policy while measuring Rust repetitions
before new native repetitions. Existing manifests, measurements, aggregate
ratios and the completed M2 qualification audit are not retrospectively
changed. A new benchmark output directory is required to preserve prior
evidence.

Every new reference record saves the applied worker limits and budget
metadata. Case summaries additionally retain the stage/repetition, Rust
median, adaptive ceiling, configured ceiling, remaining campaign budget
and effective timeout. All reference workers use the existing process-group
supervisor, so their time and memory limits remain killable. Standalone
`run_reference` callers without a Rust measurement also receive the
300-second ceiling.

A timed-out reference is a censored observation. It does not receive an
exact Rust/native ratio, even if some other repetitions completed. Exact
ratios require all paired repetitions, the existing cell-count checks,
and successful independent Rust proof verification. A timeout is neither
replaced by its cap as a measured runtime nor reported as an exact speedup.

Tests cover the 300-second floor, completed-only median, configured and
campaign ceilings, the one-hour hard ceiling, invalid timing data, failed
and truncated Rust runs, and an actual supervised reference timeout with
its applied budget retained. No new broad native campaign was launched
while the one-hour unresolved-original queue was running.
