# Exact full-real root covers from untrusted hints

`examples/profile_root_cover.rs` is a bounded research experiment for rechecking
saved root intervals. It has no production dispatch, cache or proof-schema
effect. A successful run certifies the roots of one supplied integer polynomial;
it does not establish that polynomial's projection provenance, reconstruct a
source product, resolve common roots between different blocks, or complete a CAD.

The mathematical tool is Descartes' rule, including its parity statement and
interval substitution; see [Sagraloff–Mehlhorn, §2.3](https://arxiv.org/html/1308.4088#S2.SS3).
Zero sign variations prove absence of positive roots, and one proves exactly
one, counting multiplicity. The recent formally verified implementations of
[Sheng–Li–Jackson](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ITP.2026.32)
also support this direction. The Rust helper is independently reviewed and
tested, not formally verified, and does not implement their Newton algorithm.

## Complete partition and replay

Interval hints contribute only rational cut points. The helper inserts zero,
sorts and deduplicates all cuts, and covers **every** intervening open interval
and both infinite tails. At every cut it independently evaluates the original
polynomial exactly. An endpoint root is counted once, including repeated
rational roots; no square-free assertion is taken from the input.

For finite endpoints `a=A/D < b=B/E`, with positive denominators, homogeneous
Horner evaluation constructs

```text
(DE)^n (1+y)^n p((AE+BD*y)/(DE*(1+y))).
```

This maps `y>0` bijectively onto `(a,b)` and preserves root multiplicities.
For each tail it uses denominator-cleared `p(a+y)` or `p(b-y)`. All coefficients
and sign tests use Symbolica integers and rationals. Zero leading or constant
coefficients merely represent endpoint roots outside the open interval.

Variation greater than one triggers bounded subdivision. The completed witness
contains the ordered cuts, exact endpoint-root flags and interval variations.
After a JSON round-trip, a checker rebuilds every transform and re-evaluates all
endpoints without consulting the hints. Missing gaps, forged counts, reordered
cuts and false endpoint claims fail verification. Missing hints may cause extra
discovery work; they never remove a region from the cover.

Repeated irrational roots can prevent a zero/one-variation certificate. Such
cases conservatively decline at the work/depth limit. A one-root tail is a
count certificate, not yet a finite isolating interval suitable for ordinary
cell export. A production restart facility would need to refine those tails,
bind the original projection proof, resolve shared roots and preserve every
full-real owner ordinal and interior seam.

## Resource envelope

The first experiment is capped at degree 512, 513 integer terms, 20,000 bits
per coefficient, 256 bits per endpoint numerator and denominator, 2,048 cut
points and 64 levels of subdivision. Input is at most 4 MiB. Decimal literal
grammar and length are checked before number parsing. There is no expression
parser for untrusted input.

Each transform has a conservative coefficient-height preflight and an actual
131,072-bit intermediate cap. Discovery and replay separately allow at most
128 million conservative transform work units. This counter excludes endpoint
Horner evaluations, which are bounded by degree, cut count and endpoint height.
The 128 million bound allows the degree-268 control's 56 hints even when discovery
recomputes each accepted interval to assemble its final witness. It is a
research limit, not a change to existing production Descartes policies.

An external worker supervisor enforces 60 seconds and 2 GiB on one CPU. Progress
identifies discovery versus independent replay. Terminal process evidence is
written before post-run input checks. No larger-degree trial is authorized by
this helper's existence.

## Validation and measurement

Six focused tests pass: full-real coverage including negative roots and empty
gaps; repeated rational endpoint roots; narrow roots with omitted hints;
altered counts, missing intervals and false endpoint claims; conservative
decline on repeated irrational roots, malformed literals and exhausted work;
and homogeneous-Horner signs compared with a separate Symbolica polynomial
expansion. Three earlier compilation attempts caught example-only ownership,
borrowing and univariate-API mistakes; those were corrected before the passing
gate. No production source was changed for this experiment.

The authenticated degree-268 block completed within its 60-second worker limit:

| Stage | Time | Work |
|---|---:|---|
| Cover discovery | 35.2623 s | 295 transforms, 23 subdivisions |
| Independent serialized replay | 16.4305 s | 136 transforms |
| Supervised complete worker | 51.7361 s | 10,992 KiB peak RSS |

The witness has 135 rational cuts, 136 open intervals including both tails,
56 one-root intervals, 80 root-free intervals and no rational endpoint roots.
Its independently established count agrees with the native inventory's 56
roots. Peak transformed coefficient height was 26,452 bits. The prior native
isolation took 1.4478 s; these are development-build diagnostic measurements,
not a matched release benchmark. The checker alone is slower in this control,
so this experiment supplies **no performance case for production promotion**.
No larger-degree experiment follows from a successful count certificate.

The selected block and untrusted native intervals are prepared under
`artifacts/research/full-real-root-cover-01/`; preparation is metadata
extraction, not algebra or proof. The input SHA256 is
`af8fc2b17993604d6a86903b38d87596ba188a2e9bc32453cf37d88115027463`.
The degree-1478 block remains outside this experiment's envelope.

Reproducible evidence hashes:

- Frozen helper: `b4f671bbe92526f25d8db5d8fc4207faae1179b6d75586c3bdbde983b3dbcf68`.
- `result.json`: `7ad0be4eb8384f6ad622f6235963179a50401f6a0674cef9052ed73586e707a6`.
- `result.process.json`: `c68379250a6cfc1102a683a33533cadc835afc8dd375bfe1b7225bdd36e2c1ca`.
- Source manifest (checked unchanged after build): `8c3d3860844b03d7ab21d5f010c1eb0d22f75358944aa86d773a25d51979a796`.
- Passing test log `artifacts/root-cover-tests-04.log`: `925a6d4317701e05f23fa8ac22a6ec6798c735c3af30903a57bc95b715c5384c`.

The builder/test supervisors stayed below 2 GiB aggregate RSS (880,768/860,648 KiB).
All original-input hard campaigns continued under their separate frozen binaries.

## Exact shift/reversal control

The research helper's optional `--shift-transform` retains the same full-real
cover theorem and resource limits. It removes a common positive projective
scale and evaluates each finite-interval substitution using two exact
Symbolica integer Taylor shifts, a diagonal coefficient scaling and reversal.
For `P(X)=d^n p(X/d)`, shifting by b, scaling degree i by `(a-b)^i`, reversing
all n+1 coefficients and shifting by one gives exactly
`(1+y)^n P((a+b*y)/(1+y))`. Padding before reversal preserves endpoint zeros.
Tails require only `P(a+b*y)`. No approximate root or polynomial operation
enters the proof. The alternative has a conservative three-stage height
preflight and checks actual intermediate coefficient heights; the original
Horner kernel remains available as a control.

Eight tests passed, including full coefficient equality against independent
Symbolica polynomial expansion, degree drops at either endpoint, repeated
rational endpoint roots, both tails and replay of shift-generated evidence
using the original Horner checker. Test/build runs took 7/7 seconds with
sampled aggregate peaks 766,704/784,900 KiB. Production code is unchanged.

On the identical authenticated degree-268 input and untrusted hints, the
bounded single development-build trial completed in **4.454089098 seconds**
with **9,036 KiB** peak RSS. Discovery took 2.986022183 seconds and fresh
independent replay 1.421489719 seconds. The complete exported cover is the
same 135 cuts, 136 intervals and 56 real roots; discovery still performs
295 transforms and 23 subdivisions, with 136 replay transforms. Maximum
intermediate height fell to 20,259 bits. Source hashes remained unchanged.
Replay recomputes every claim from the original coefficients and serialized
partition; it uses the same exact backend, not a second independent isolator.

This improves the previous research helper's 51.736-second end-to-end trial.
Fresh replay alone is comparable to the previously measured native isolation
(1.4478 seconds), so it does **not** yet establish a production speedup or a
reason to raise the degree envelope. Profiling the degree-1478 native call's
SQF-versus-isolation costs remains a separate proposed measurement. Neither
root-cover experiment verifies factor products, cross-block coincidences or
an original decomposition.

Evidence under `artifacts/research/full-real-root-cover-shifts-02/`:

| Record | SHA-256 |
| --- | --- |
| Frozen helper | `73d43b5edeebbb630a436b18ded53ad6c71e6093284dc8bae45a42e2e42b9869` |
| `result.json` | `68c3d5e8bf3c8c267cf7f1c67d2aaa4836d33548ae7268aa5730af6779f6854e` |
| `result.process.json` | `e6881bca7930c47a1d357de94f194ff8954a6ece11632a12beb8fcc911a67d58` |
| `artifacts/root-cover-shift-tests-01.log` | `857b38637c00279c0c9538e433ad516de751db5014570129764c20aab796903e` |
