# Gluza33: bounded interval-Descartes guard measurement

The configurable saturated-witness source budget moved the original positive
Gluza33 solve beyond its previous 6,938-term discriminant. A later captured
19,942-term discriminant cannot use that saturated proof, but an exact two-piece
interval proof has affordable guard construction and factorization. This is a
research result about one captured projection obligation, not a completed CAD.

## Captured obligation and provenance

The pending operation is discriminant key
`4fa9b80565798fa8d080fd6ac8b7a80d24ebcc05ce1f267fde85b71be220b10a`,
eliminating `x7` from a degree-24 source with full variable map
`[x2,x1,x7,x4,x6,x3,x5]` and degrees `[41,33,24,0,0,0,0]`.
The frozen capture SHA-256 is
`5c15531346bf5a4a4e247db0f23fdbb38031d1bdf0485f7e08e7af7b1c2ad3d6`.
Its same-time last published progress records level 2 and prefix
`[x2,x1]=[2,1/2]` at 177.595117578 seconds. That progress file has SHA-256
`55d0d86415576022526d8e80d0bf86e53ffc5b21b3ae2baf46f1568cfe9290c5`.
This binds the diagnostic to a recorded prefix; it does not certify original
coverage, nor give the start time of the pending algebra call.

The existing frozen saturated-witness helper, with research source admission
32,768, found four coefficient-sign variations and two distinct positive roots.
It declined in 0.385917 seconds using 19,712 KiB peak RSS. Raising that witness's
source cap alone cannot help this polynomial. The report is retained under
`artifacts/research/gluza33-witness-budget-02/`.

## Exact interval proof and measured guards

`examples/profile_interval_descartes.rs` now accepts a research-only
`--max-terms` between 1 and 65,536, defaulting to 4,096. Production limits and
algorithms are unchanged. The helper preserves the full variable map and
reconstructs every transformed coefficient, the leading coefficient, the
coordinate endpoint and all finite seams using exact Symbolica arithmetic.
It separately reports exact witness applicability and current production
resource eligibility; factorization is enabled only when all production limits
other than source admission pass.

Reported coefficient signs belong to the original transformed coefficients;
guard indices refer to primitive positive-leading canonical polynomials, whose
signs can differ by a scalar unit. Production replay reconstructs the original
coefficients. An early declined discovery may omit inventory and eligibility
fields; missing fields do not authorize factorization.

At the recorded prefix, the complete positive-ray partition is `(0,1)` and
`(1,+infinity)`, with the seam `x7=1` explicitly checked as a nonroot. The finite
transform is `(1+u)^24 p(u/(1+u))`, and the tail transform is `p(1+u)`.
Each coefficient sequence has exactly one variation: the finite sequence has
four positive then twenty-one negative coefficients; the tail has twenty-four
negative then one positive coefficient. All coefficients are nonzero at this
prefix. Each interval therefore contains exactly one positive simple root;
the endpoint, seam and leading guards preserve the coverage and degree.

These identities retain both interior root branches of the original polynomial.
They do not discard an `F=0` sheet or use a physical threshold to erase a root.
Their use throughout a parameter cell remains conditional on proving all
listed guard signs constant and nonzero there.

| Quantity | Measured | Current production bound |
|---|---:|---:|
| Source terms | 19,942 | 4,096 (fails) |
| Elimination degree | 24 | 3–32 |
| Intervals | 2 | 64 |
| Transformed coefficient slots | 50 | 256 |
| Distinct canonical guards | 49 | 256 |
| Largest guard | 1,247 terms | 4,096 terms |
| Total distinct guard terms | 49,870 | 65,536 |
| Largest coefficient height | 133 bits | 512 bits |
| Largest separator height | 2 bits | 256 bits |
| Positive roots | 2 | 8 |

Height is numerator bits plus denominator bits, matching production checks.
Guard term counts range from 501 to 1,247; their remaining variable degrees
reach `[41,33]`. All non-source production limits pass.

Both measurements used a fresh supervised process on CPU 8, a 60-second wall
limit, a 4-GiB memory limit and one worker. The ordinary production Symbolica
backend, without the optional root-isolation patches, was used with a debug
helper; these are research timings,
outside benchmark solve timings.

| Trial | Wall time | Peak RSS | Result |
|---|---:|---:|---|
| Construction only | 0.809820 s | 30,352 KiB | Full cover and 49 guards constructed |
| Construction and checked factorization | 9.271372 s | 32,848 KiB | All 49 guards factored and reconstructed |

The factor calls plus their exact signed reconstruction total 3.274083 seconds;
the slowest guard takes 0.134085 seconds. There are 69 canonical factor
occurrences. Remaining wall time includes parsing and repeated durable JSON
progress writes. Factor counts are exported, while scalar units and
multiplicities are checked inside `factors_checked`; the report is not a
standalone serialized signed-factor certificate. No lower guard projection,
pair separation campaign or original complete decomposition was measured here.

## Reproduction and next step

The ignored artifact directory is
`artifacts/research/gluza33-interval-budget-01/`. Its `SHA256SUMS` binds the helper,
source, capture, progress, reports, tests and invocation manifest. The source
base is `8e28c21fac8c96348560efbe79c26da5dbfe06f5`; `helper.patch` records the
uncommitted research-helper change used for the measurements. The frozen helper
SHA-256 is `eb79ac98194a58ec14667aecde579cedd83496fb24d493f9d04778f5da1e332c`.

```sh
profile_interval_descartes source-capture.json --prefix 2,1/2 \
  --max-terms 32768 --seconds 60 --cpu 8 --output construction.json
profile_interval_descartes source-capture.json --prefix 2,1/2 \
  --max-terms 32768 --factor-guards --seconds 60 --cpu 8 \
  --output factorization.json
```

All seven helper tests pass, including a generic 5,445-term primitive source,
invalid source budgets and maps, exact symbolic transformation identities,
full-cover gap/overlap rejection, and rejection of repeated positive roots.

The next justified production experiment is a general, explicit interval-source
admission budget, independent of the existing downstream guard budgets. Its
default should remain 4,096, with the same explicit budget in discovery,
template reuse and independent verification. Permanent template quotas and
all degree, endpoint, seam and transformed-coefficient obligations must remain
unchanged. A warm original solve would then determine whether the much larger
two-parameter guard problem actually improves completion time. No fixture-name
dispatch or automatic cap increase is justified by this operand measurement.
