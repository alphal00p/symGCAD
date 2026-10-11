# Bounded source admission for exact positive-root witnesses

Production change `8e28c21`, 11 October 2026. The optional
`solver.local_descartes_witness_max_terms` now controls the source-term
admission budget when `local_descartes_witnesses = true`. The default remains
4,096; explicit values must lie between 1 and 65,536. No graph, fixture or
polynomial identifier selects a different policy.

Discovery, reuse and independent proof replay use the same recorded limit.
Every coefficient, endpoint and rational-separator guard remains required;
the degree bound, positive-root count bound and permanent two-template stock
are unchanged. Historical evidence defaults to 4,096 and default output
omits the new option. Source-term declines have a separate diagnostic counter
from template exhaustion. The [earlier guard experiment](GLUZA33_WITNESS_BUDGET.md)
motivated this general resource option; it did not prove that lower projection
would be cheap.

## Original Gluza33 continuation

A fresh development-build worker resumed the original positive Gluza33 with
the same `x0` homogeneous anchor and order
`[x0,x2,x1,x7,x4,x6,x3,x5]`. All eight positivity constraints and the original
strict `F>0` polynomial were preserved. The affine factor policy remained on;
the witness source budget was 8,192. The worker used CPU 13, a 50 GiB memory
allowance and a 300-second solve limit. Verification would run separately
only after a complete result.

| Measurement | Result |
| --- | ---: |
| Process wall time | 300.057932 s, timeout |
| Peak worker RSS | 412,188 KiB (402.5 MiB) |
| Inherited exact operation records | 5,561 |
| New completed operation records | 4,554 |
| New factors / discriminants / resultants | 4,470 / 47 / 37 |
| Final operation records | 10,115 |
| Verified original decomposition | **None** |

The former 6,938-term degree-16 discriminant was bypassed at its recorded
prefix. The solver completed downstream pair resultants and traversed further
prefixes. Its new pending operation is a 19,942-term degree-24 `x7`
discriminant, with degrees `[41,33,24,0,0,0,0]` in
`[x2,x1,x7,x4,x6,x3,x5]`. The last published prefix is `[2,1/2]` at
177.595118 seconds. That timestamp is a progress snapshot, not the duration
of the final native call.

A separate bounded probe allowed 32,768 source terms at that prefix. It
completed in **0.385917 s**, using **19,712 KiB**, and declined the saturated
witness: exact isolation finds **two positive roots but four coefficient-sign
variations**. Increasing this source budget alone cannot resolve the new
frontier. No guard factorizations were attempted. The subsequent
[interval-certificate measurement](GLUZA33_INTERVAL_BUDGET.md) supplies a
different proof with 49 guards; its production integration and remaining
projection are separate work.

These are warm exploratory measurements. Prior solve/order discovery and
checkpoint generation are excluded. They establish forward progress, not a
release speed ratio, complete coverage, or new original-corpus completion.
The published table therefore remains at **303/331**.

## Validation and retained evidence

All **627 tracked fast tests passed across 46 targets**, with zero failures
and two expensive tests ignored. The separate workstream's untracked
alternate-mass fixture was excluded. The five new tests cover oversized
source admission, every raw guard, reuse and replay with the same budget,
historical serialization, invalid limits and altered evidence. See the
[combined validation record](COMBINED_FAST_VALIDATION.md).

The release build also passed a fresh CLI solve and independent verification
of the three-cell witness regression. Its frozen executable SHA-256 is
`417e7ee79590f683d41bcc8a62d1d050fd339db853f39bdcf509a3533fa81872`.

The original worker's mathematical fields were independently compared to the
source fixture. Every inherited record remained unchanged; all operation
keys and output checksums were audited. This metadata audit is not algebraic
verification of a complete CAD.

| Ignored evidence path | SHA-256 |
| --- | --- |
| `artifacts/research/gluza33-witness-production-01/final-audit.json` | `cd7833710f59f1c9b6a35e8a1d6ff9f3f0a5c411dfd86dbcfa2ac28ccb5af14a` |
| `gluza33-witness-production-01/bin/symgcad` | `a49e9532d11c6b0ac494a2888fa5dcd1ca13d066aad23d5cc428707715dc862f` |
| `gluza33-witness-budget-02/manifest.json` | `431e7d539cd07ad8c0d15d1376d5ba4e53908a9bde51fb2dac8616e8255bfe0a` |
| `gluza33-witness-budget-02/report.json` | `afd90f40d6f3b153a3193d584ddccd1c982ba2d6c4d01cde570a6c86c1a4f26d` |

The last three paths share the `artifacts/research/` prefix. Captures, exact
guards, checkpoints, executable hashes and launch commands remain outside Git.
