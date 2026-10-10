# Prefix evaluation: negative power-cache experiment

The full A381 traversal profile attributes 26.66% inclusive samples to
`prefix_value` and 26.35% to Symbolica `replace_all`. Symbolica's implementation
computes rational powers while traversing terms. This motivated testing the
existing `replace_except` power-cache API before changing production.

The research-only Rust helper `examples/profile_prefix_values.rs` compares:

1. Existing exact `replace_all` evaluation.
2. Existing `Algebra::specialize_univariate_batch`, sharing powers within a
   fresh batch and extracting constant coefficients afterward.
3. A retained, bounded prefix cache passed to Symbolica `replace_except`.

All three paths check variable maps, prefix length, absence of unsupported
later variables and Symbolica's signed exponent range. The retained cache
charges at most 64 KiB and keeps at most 64 powers per axis. Larger degrees or
exhausted byte budgets use Symbolica's exact uncached fallback. All arithmetic
remains in Symbolica. Exact outputs are compared before timing. Four helper
tests passed, covering zero, negative and rational prefix values, inactive
variables, constants, full and empty prefixes, zero-dimensional maps, large
exponents with bounded-cache fallback, mismatched maps and unsupported later
variables even when an earlier zero would nullify the expression.

## Captured operands and measurement

The A446 input contains the actual 12 coefficient/separator guards from
`artifacts/profile/a446-descartes-witness-01/cost.json`: 2,978 terms at prefix
`[13/3,-1505,1,1]`. The A381 input is an inventory of actual cached projection
guards from the fixed-work controls. Of 4,777 polynomials, 4,775 are supported
at the observed prefix `[1,16/29,14,1,1]`; the helper selects the largest 64,
containing 4,535 terms. Those 64 are **not claimed to have been jointly visited
at that prefix**. This is an operand microbenchmark, not a full traversal.

The same frozen debug executable evaluates each selected batch 2,000 times on
CPU 8, with one thread, a 4 GiB address-space limit and a 60-second external
deadline. Both processes exit successfully and exact outputs agree.

| Batch | `replace_all` seconds | Fresh shared batch | Retained power cache | Charged cache bytes |
| --- | ---: | ---: | ---: | ---: |
| A446, 12 guards | 3.188 | 3.966 | 3.623 | 3,624 |
| A381, 64 guards | 8.303 | 9.889 | 9.460 | 14,357 |

The retained-cache path is about 14% slower on both batches, and the fresh
batch path is slower still. An initial 50-round measurement has the same
direction. Constant polynomial construction and repeated degree checks are
plausible overheads, but were not separately attributed. These measurements
do not justify a production prefix-evaluation change. They do not exclude a
different implementation that avoids polynomial construction, nor establish
release performance. No solver code was changed for this experiment.

Evidence is in `artifacts/profile/prefix-values-01/`. The `.long.result.json`
files contain operand-selection hashes, exact-equality results and timings;
resource logs retain commands and limits. Total process wall times are 10.87
seconds (A446) and 28.03 seconds (A381), with 3,120 and 9,264 KiB peak RSS.
The compiled source snapshot is retained beside the binary; the committed
source differs from that snapshot only by formatting.

| Artifact | SHA256 |
| --- | --- |
| Frozen debug helper | `b27c9cec731f05c183378555bba1fa79ca58b0639a8ade7154e5b0f7a0adadb5` |
| Compiled source snapshot | `b41eb07bf84d433fceb74b0d36e57bbe8e3828df1cb8f159f0fa4fcd1e9da2a0` |
| A446 input | `0bbdd34968751cb1cab983d66499affdf7a2d9455105b3b4ed919d11bce2db66` |
| A381 input | `7acda8c7455eb5c28193c706f702e0e0d6ce7d85d5f8503c1981b2b86ba1fc86` |
| A446 2,000-round result | `8b272353d32e5a4d088a051c030dbf660d6d50e883aae2e274f299c221a09082` |
| A381 2,000-round result | `ff1644808300fbc7c220a1f6dc0f20a472b80dc447b75457c68ba65f048187ee` |
