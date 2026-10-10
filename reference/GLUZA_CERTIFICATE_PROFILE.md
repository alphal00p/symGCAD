# Gluza retries and identity-substitution cost

The full-original Gluza30/Gluza33 retries with immutable source `b8139e0`
did not complete any of the four remaining problems. The negative and positive
domains were retained separately; all original constraints, groups, variables
and fixed-order obligations were preserved. Both families used one CPU,
4 GiB, 20-second initial solves, 60-second promotions, independent verification
for any completed result, and a 600-second family budget including preparation.

| Family | Attempts | Timeouts | Verified completions | Total seconds | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Gluza30 | 8 | 8 | 0 | 322.391 | 202084 |
| Gluza33 | 16 | 16 | 0 | 482.281 | 97472 |

The worker SHA-256 was
`e24b80cd8a8d69bb361b7ed08737393e8fcd8161eefa69d0bc3e1b3b44bdb6e7`.
Detailed inputs, source hashes, commands, every attempt, and timeout evidence
are under `artifacts/core-affine-sum-retry-01/`; its summary hash is
`f421d483afadcca4a13e8d49a3c7e9723faa638681e79f0bc887dec03cd767b4`.
These are exploratory debug runs, separate from the frozen baseline denominator.

## What the certificate profile found

Gluza33's first local snapshot attributed 7.89 of 12.66 seconds to domain
certification. A separate 20-second replay of its original negative domain,
with the exact x0 homogeneous anchor and checkpoints enabled, collected 1,405
algebra operation checkpoints. This replay also timed out and is not a new
completion. `examples/profile_certificate_rejection.rs` checks checkpoint
output hashes, extracts distinct mixed-coefficient factor outputs, and evaluates
the full nonvanishing portfolio on their positive-orthant box. These are
candidate factors, not a trace weighted by actual certificate-call frequency.

The research probe tests at most nine low-height rational points inside the
open box. An exact zero, or opposite exact signs at two such points, disproves
strict nonvanishing on that connected box. Same-sign samples prove nothing.
This rule can only decline a certificate attempt; it cannot accept a sign,
remove a polynomial, or establish any cell. It remains research-only.

The first 128 candidates suggested a large benefit from bypassing failed
certificates, but attribution showed that ordinary positive-box conversion was
repeatedly applying the identity substitution `x_i -> x_i`. The independent
identity replay took 2.087 seconds of the 2.175-second certificate total. The
production fix returns an exact clone for offset zero and scale one, **after**
the existing term-limit check. All nonidentity transforms and proof semantics
remain unchanged.

| Same 128 candidates, one CPU | Before | After identity fast path |
| --- | ---: | ---: |
| Full certificate kernel seconds | 2.174523 | 0.022836 |
| Independent identity-substitution control seconds | 2.087224 | 2.111997 |
| Exact rejection probe seconds | 0.026861 | 0.027423 |
| Whole-call bypass estimate, including probe seconds | 0.109961 | 0.034592 |

This is about 95 times faster for this isolated certificate workload, **not**
an end-to-end GCAD speedup. All 128 portfolio attempts remained inconclusive;
74 had exact counterexamples. The canonical JSON inventory of variables,
polynomials, domains, certificate methods, rejection flags and exact observations
was identical before and after, SHA-256
`51de54dbf06c0d6cc0e02658702840ee2463e4a79a04b670bc01be9e946cdf6f`.
The 31 domain integration tests and two new identity/cap/nonidentity regressions
passed, covering successful proof discovery and independent replay as well.

After eliminating the identity work, the rejection probe would increase this
sample's certificate cost by about 51%. It therefore was not added to production.
A larger pre-fix sample reached 703 candidates and 382 exact rejections, taking
7.9613 certificate seconds and 0.1859 probe seconds; its apparent bypass benefit
also included avoided identity substitutions. Neither estimate establishes
benefit from inserting probes after coordinate conversion. Detailed-report
persistence consumed most of that experiment's 60-second wall budget and is
excluded from the isolated kernel timers.

## Reproduction and frozen artifacts

All files below are in `artifacts/profile/gluza33-certificate-rejection-01/`:

| Artifact | SHA-256 |
| --- | --- |
| `profile_certificate_rejection-v3` (before) | `ccd38843bc6f5bff055fcd871e09d835e73ae70e67057fa7e6577c16bab0eb04` |
| `profile_certificate_rejection-identity-fast` (after) | `c15e0acb8da86bf6b0594f848b9b1c85a6fd6a6daeea9041d6301119ebc720f7` |
| `profile-identity/report.json` | `a5e92372f2272d38a2d48e205b1bf62b785c768f71947262159e7916c672355a` |
| `profile-identity-fast/report.json` | `4a95a26fa73e38503cbf0f9960af8f4aaefeef0f25063bd45f1c4c8ac791582f` |
| `profile-all/report.json` | `4396b28cbc243689654b7da9f967bec3e3617e60870be22dca17088056f979b1` |

The frozen source copies, production patch, original input and operation files
are beside these reports. The earlier v1 helper rejected the checkpoint checksum
format before profiling; v2 corrected the hash to include multiplicities, and
v3 added the independent identity measurement. Superseded artifacts are retained.

From the repository root, after `nix develop`, choose either frozen executable
and a fresh output directory:

```sh
ulimit -v 4194304
timeout 75s taskset -c 1 artifacts/profile/gluza33-certificate-rejection-01/profile_certificate_rejection-identity-fast artifacts/profile/gluza33-certificate-rejection-01/checkpoints/operations --output artifacts/profile/gluza33-certificate-rejection-01/replay --max-cases 128 --max-points 9 --seconds 60 --measure-identity-shifts
```

The helper has four tests for exact counterexamples, finite/one-sided/free
domains, inconclusive probes and caps, and identity reconstruction. No changes
to Symbolica or the original reference implementation were needed.

## Matched full-original retry after the identity fix

The immutable production `ab8b9ba` worker, SHA-256
`68a9cdc90842868ec9d7c08e2b9ba8f2b6265acbf315437bfae4ec647ca58157`,
repeated the same portfolio on the same CPUs and limits. All 24 `(name,
variant, input SHA-256)` pairs are exactly equal to the earlier campaign,
including the promoted variants. Adaptive promotion execution order can differ.

| Family | Attempts/timeouts | Verified completions | Total seconds | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Gluza30 | 8/8 | 0 | 322.313 | 202072 |
| Gluza33 | 16/16 | 0 | 482.467 | 99136 |

The four original problems remain unresolved. The first Gluza33 local/x0
attempt published nearby snapshots with the same 776 certificate attempts:
certificate time changed from 7.89286 to 0.08124 seconds and elapsed time from
12.656 to 4.793 seconds; factor/resultant totals stayed near 2.57/1.08 seconds.
This supports the identified bottleneck without claiming completed-solve speedup.
Inputs, plans, complete ledgers, reproduction commands and hash comparison are
under `artifacts/core-identity-retry-01/`, summary SHA-256
`9ac77eff7827ebd5cebecbce1fd3f267d637fbd3564a252bba9d1bca91696c32`.

## Research: conditional rejection before the final variable

`examples/profile_partial_rejection.rs` observes separate direct-local solves
that retain the original nonlinear inequality. It does not use sign-filter
arrangements and does not change the solver's traversal. At an actual observed
positive prefix, the helper groups the original inequality by remaining
monomials and evaluates every earlier-coordinate coefficient exactly. If every
coefficient is strictly negative, the original strict-positive inequality is
impossible over the remaining positive coordinates, provided the recorded
coefficient signs hold throughout the parent cell. A coefficient that vanishes
only at the sample blocks the proposed rule; no sign is inferred from it.

A future proof would need the original positive-coordinate premises, exact
coefficient reconstruction, and sign-invariance of every earlier coefficient
through its guarded factors. These observations are not themselves a GCAD proof.
An opposite-sign split cell cannot be discarded by this argument; that is why
these diagnostics use the original inequality instead of the monotone wrapper.

Four independent 20-second observations, one CPU/4 GiB each, all timed out:

| Original domain | Distinct published prefixes | Candidate rejected prefixes | Candidates with two axes remaining |
| --- | ---: | ---: | ---: |
| Gluza30 negative | 30 | 1 | 0 |
| Gluza30 positive | 31 | 5 | 3 |
| Gluza33 negative, x0 anchor | 3 | 0 | 0 |
| Gluza33 positive, x0 anchor | 3 | 0 | 0 |

There were no candidates with three or more remaining axes. Each two-axis
candidate used four earlier coefficient guards. Published progress prefixes are
sparse and are not weighted by actual visit count. Offline Gluza30 coefficient
evaluation took about 10 ms over 61 prefixes; applying this roughly 0.17 ms cost
at tens of thousands of visits could add seconds. Final-axis rejection may
already be inexpensive through existing positive-ray rules. The evidence
supports only a bounded opt-in experiment, not a default rule or a speedup claim.
No production rejection heuristic was implemented.

Frozen observer, source, original-domain inputs, snapshots and reports are in
`artifacts/profile/gluza-partial-rejection-01/`. Its executable SHA-256 is
`21c8e7e7e9aca294ed4ad4ae099425763ca2f42c6b8e8a9fd18fb2550e4a00af`;
summary SHA-256 is
`f5a660f2709de2d48c5c7e8dae15b9a4b0e1a7fd1f12f7bb0c338f9e9773183e`.
The two helper tests cover exact coefficient grouping, strict-negative fibers,
and nonstructural zero-coefficient obstructions. Example replay after
`nix develop`, using a fresh output directory:

```sh
taskset -c 4 artifacts/profile/gluza-partial-rejection-01/profile_partial_rejection artifacts/core-identity-retry-01/gluza30/0002-monotone-local-unscaled-seed0/input.toml --executable artifacts/hard-campaigns/identity-fast-debug-01/bin/symgcad --output artifacts/profile/gluza-partial-rejection-01/replay --seconds 20 --memory-mib 4096
```
