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
