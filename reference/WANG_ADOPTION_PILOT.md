# Broader pilot of the optional Wang policy

11 October 2026. **Keep the small-leading-coefficient Wang patch experimental.**
Its captured Gluza33 benefit is documented in
[GLUZA33_WANG_PREFLIGHT.md](GLUZA33_WANG_PREFLIGHT.md). This independent pilot
finds no additional completed speedup or original-problem completion.

The same frozen experimental executable ran four original captured operands,
with only `SYMBOLICA_RESEARCH_SMALL_LC_WANG` enabled/disabled. Both arms used
automatic native factorization, tracing, CPU 12, 4 GiB and a 12-second killable
worker. Two additional LC-only checks had 10-second limits. Total actual worker
wall time was **72.307 s**, within 116 seconds of allocated worker budgets.
No existing backend tests or Gluza2410 timing were repeated.

| Original capture | Terms | Enabled | Disabled | Observed path |
|---|---:|---|---|---|
| B471 `active-2264304` | 263 | 7.085 ms algebra | 6.972 ms algebra | Earlier native path; no Wang admission. |
| A318 `active-4068322` | 1,029 | timeout, 12.037 s | timeout, 12.035 s | All 12 new LC-image candidates rejected; fallback. |
| A317 `active-4137968` | 1,936 | timeout, 12.020 s | timeout, 12.037 s | All 12 new candidates rejected on a 1,847-term internal component; fallback. |
| C470 `active-2289959` | 204 | timeout, 12.035 s | timeout, 12.036 s | Density gate declines the new path. |

Both B471 completions passed the profiler's mandatory **exact full signed
factor-product reconstruction**, including multiplicities, in the original
map. Their canonical output hashes agree. The six timeouts supply no completed
factorization. These are single diagnostics: the historical fallback samples
randomly, and the small B471 timings do not justify a speed ratio. No cache
entry or CAD coverage is produced by the profiler.

## Why extending the integer menu is insufficient

The frozen LC helper independently checked the primitive-source identity and
full signed LC factor product. For A318, writing `v=x8`, it obtained

```
81714368160000 * v^4 * (72+77*v) * (72+67*v) * (1+v)^2 * (13+31*v)^2.
```

This took 0.00135 s. No integer `v` can satisfy the current necessary image
gates: even `v` shares 2 with the LC content through its primitive-variable
factor; odd `v` makes the two distinct factors `1+v` and `13+31*v` both even.
Zero or unit images also fail the existing rules. This exact parity argument
concerns factor reconstruction, not physical thresholds or polynomial
nonvanishing. It permits no removal of an interior boundary.

The original A317 LC similarly has even content and factors `v`, `v-1`, `v+1`.
Its exact replay took 0.00066 s. That identity is bound to the original source;
the smaller internal component was not exported, so its identity is not
silently inferred from the trace.

A useful next experiment is a **bounded modular obstruction preflight** after
checked LC factorization: enumerate all residues modulo 2 in at most two LC
variables. Skip the optional integer sampler only if every residue violates a
necessary image condition; otherwise retain all existing checks. This skips
futile algebra work only. It neither removes guards nor relaxes assignment.

Before wider adoption, collect completed new-path and completed fallback cases
from other families, with three fresh runs per arm and preprocessing included.
Add public-path regressions for rational signs/repeated powers, permuted full
maps, static-cap declines and irreducible sources with reducible specializations.
Keep exact degree, square-free, primitive-content, multiplicity-assignment and
signed-product checks. This pilot supplies no evidence for changing defaults.

## Evidence

Untracked evidence is under `artifacts/research/wang-adoption-01/`: `REPORT.md`,
`summary.json`, original capture pins, on/off process reports, exact LC exports,
and reproduction scripts. A metadata-only inventory screened 1,142 named
captures, excluded files above 1 MiB, and deduplicated exact operand text. It
found 80 three-axis expressions and 13 rough candidate shapes; backend content
removal and reordering can change actual admission. This is not a corpus-wide
algebra check. Input/helper hashes were verified unchanged after the runs.

| Artifact | SHA-256 |
|---|---|
| Frozen experimental factor helper | `c07bc81be030f88444e65882070b99826532b0f3ccfe43588cf4f7f06864bfab` |
| Summary | `2d913693d03137c36d90573594c73166740884094a49628fd4b9d750b668509f` |
| Metadata inventory | `cec04a12fa1950b6c1ae19eab242c78332d64c50640645914f316d2804877cd8` |
| A318 exact LC report | `38764d071d2394f1e9d03f9c820334b9dae925f47fd9d9f1b89bff9110c5b18c` |
| A317 exact LC report | `cf9bf0bc61467b31ac81c703f4a570be2505b3939b8e7bbec079385caada9f1f` |

The pinned backend is `75f8350094b90254ee71dc2a391fde0d14b0204a`, built at
opt-level 2 with the separately replayable experimental patch. The main
dependency and production source remain unchanged by this pilot.
