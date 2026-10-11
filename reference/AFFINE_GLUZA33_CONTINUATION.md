# Original Gluza33 continuation with affine factor preconditioning

11 October 2026. **The original positive fixed-kinematics problem remains
unresolved.** The new opt-in policy passed the previously captured factor
obstruction and saved 384 more exact operations, then reached a larger
discriminant. No complete decomposition, independent verification or new
corpus completion resulted.

## Scope and measured outcome

The release binary at source commit `8de480f` used the ordinary, unpatched
Symbolica checkout at `75f8350094b90254ee71dc2a391fde0d14b0204a`.
The original constraints, parameters and variable groups were unchanged.
The legal order was `x0,x2,x1,x7,x4,x6,x3,x5`, with positive homogeneous
anchor `x0`, the existing local-projection settings and unmerged output.
The only algorithm change from the prior x0-anchor attempt was
`affine_factorization = true`; checkpoint location and resource limits changed.

One worker on logical CPU 13 of the AMD EPYC 9754 had a 300-second campaign
budget and 50 GiB allowance. It timed out after **300.020938 seconds**, with
**462,972 KiB (452.12 MiB)** reported peak RSS. The parent process's CPU
counters are not interpreted as aggregate descendant CPU time. The five-minute
budget was a diagnostic continuation, not a replacement for the earlier
one-hour and two-hour attempts.

The warm cache contained 5,177 prior completed native operations, copied with
every source/destination hash checked. It ended with 5,561: **362 additional
factorizations, 20 resultants and two discriminants**. All inherited files
remained byte-identical. No research factorization was injected into the
cache. Metadata audits checked every operation key and output checksum;
these checks are not independent algebra replay or a coverage proof.

The old 224-term factor request completed in **0.151788 seconds** through
the production policy, retaining all three factors and their multiplicities.
This release operation timing is separate from the earlier matched
[development-profile operand experiment](AFFINE_FACTORIZATION_POLICY.md).
Neither number is a full Gluza33 solve time or a native-GCAD speed ratio.

## New obstruction and next experiment

The final request is the discriminant in `x7` of a **6,938-term** polynomial,
with degree vector `(30,32,16,0,0,0,0)` in the reduced map
`x2,x1,x7,x4,x6,x3,x5` and maximum coefficient height 80 bits. Its 17
coefficients contain 249–481 terms. It comes from a completed `x4` resultant
(0.123561 seconds) and is already an irreducible multiplicity-one cached
factor (0.007012 seconds). Further affine factor tuning therefore does not
address this particular obstruction.

This degree-16 discriminant has two remaining parameters, `x2,x1`.
The earlier x3-anchor degree-10 obstruction had three; its saved local root
witnesses cannot be transferred. This run enabled operation capture but not
the contemporaneous local-prefix snapshot. Consequently no rational sample
or root-exclusion certificate is asserted for the new request.

The next diagnostic should capture the exact local prefix and certificate
rejection reason, then test positive-root separator certificates there.
Any accepted certificate must retain all necessary coefficient, degree-drop,
endpoint and separator guards, and preserve every positive interior root.
This is a proposed experiment, not an implemented discriminant shortcut.

## Reproduction and provenance

Set the documented opt-in flag on the original Gluza33 problem with the order
and local-projection settings above. A fresh run includes all prior work;
reproducing this warm timing additionally requires the retained operation
cache. Resource limits alone do not reproduce it. For further diagnostics,
set both `SYMGCAD_CAPTURE_DIR` and `SYMGCAD_PROGRESS_PATH` before launching.

Ignored evidence is under
`artifacts/research/affine-gluza33-continuation-01/`: campaign configuration,
frozen executable, input, ledger, captures, exact operand inspection,
cache manifest, copy audit and metadata audit. The Rust benchmark generator
separately checks the original mathematical TOML fields, legal order,
input/executable hashes and censored outcome before incorporating the record.

| Evidence | SHA-256 |
| --- | --- |
| Release executable | `18b916fd4e2b09898779b2f4446af7df5855ae29a67ebb7cad7b582fe8f83730` |
| Original corpus input | `63247029fd6de5073d0b3b3587689e2235b987d6bfdd2ec9fe8139c5ffa20574` |
| Actual trial input | `bc874cbe1b01272274775c740054b23231a068522907b479e239ceb6400b6001` |
| Ledger | `ebf1fa6945a1ebb5de1b4b842ca4e42a8fe02825f535a02bc5d15af5e621809d` |
| Final capture | `2c5be2b92053d1c85fba321eb5a67e5204c3954c926387d6a57d678a00926d6a` |
| Metadata audit | `9d86e6d967657287a963db579cc479dc03edbe051145a9e40e0b6dce3e2c054a` |

The benchmark retains the longest observed incomplete attempt and highest
retained RAM for the original row. This shorter diagnostic is recorded in
its follow-up provenance and notes; it does not replace a timeout with a
solved time. The audited total remains **303/331**.
