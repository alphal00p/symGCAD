# Exact peel inspection without projection

`monotone::inspect_peels(&Problem)` exposes the existing bounded discovery and
independent coefficient-certificate replay. It returns verified peel steps,
the exact endpoint problem, and a provisional order. It does not run projection,
choose a projected variable order, solve the endpoint, or claim coverage.
An empty step list means that this bounded portfolio found no admissible peel;
it is not a proof that every possible reduction fails.

The API requires one split polynomial and an already reduced scale configuration.
A caller transferring a strict target must remove that target from the domain
used to certify coefficients. Explicit fixed orders are retained. Otherwise the
remaining prefix follows the original variable groups, followed by the reversed
peel suffix. Every returned step is replayed against those order obligations.

The research helper supports, for example:

```sh
cargo run --example monotone_candidates -- \
  --input problem.toml --inspect --all-anchors --ordering brown \
  --seconds 10 --output inspection.json
```

Its heuristic override preserves an explicit original order. Each scale anchor
uses the existing exact homogeneous certificate, including positive scale and
anchor-before-ratio order requirements. Both polynomial orientations are checked
separately. Partial reports are replaced atomically after completed inspections.
The `--seconds` budget is checked between rows; the bounded worker supplies the
hard wall and memory limit.

## Remaining symbolic cohort, 2026-10-10

The projection-free screen inspected all 27 unresolved symbolic ZZ originals
and four Gluza originals from `artifacts/completion-aggregate-300.json`. The
manifest retains original paths, SHA-256 hashes, aliases and fixed orders.
No domain restriction was added. Both orientations and every requested positive
scale anchor were inspected, including the unscaled problem.

All **470 distinct input/anchor/orientation requests** have a recorded outcome:

- 442 exact inspections returned no peel.
- 28 Gluza30 anchor requests rejected the nonhomogeneous scale reduction.
- No nonempty peel plan was found. All original hashes matched, and repeated
  inspections returned identical evidence.

This is a negative result for the bounded direct/conditional coefficient
portfolio on these complete original domains, not a new solve or an impossibility
theorem. The fixed A381/Ax381 regression remains a positive control: the same
discovery finds its two conditional peels after the checked `x8=1` reduction.

Workers ran on CPU9 with a 2GiB address-space cap. Initial workers had a 15s hard
limit and 10s soft row budget; targeted follow-ups filled missing rows. One 12s
follow-up of unscaled Gluza33 timed out and was completed under a separate 25s
hard/20s soft limit. No projection, lifting or full-case timing comparison was
performed.

Evidence is under `artifacts/research/monotone-inspection-01/`:

- `manifest.json`: SHA-256
  `82d9badbc68097f91124c85a33020f1732d705280f8b4aa7e90b2c0ebea2f323`.
- `summary.json`: SHA-256
  `1608ecab3542276f6901783cfba73025332db75bc86bba64de3c9e47f9b868e8`.
- Frozen debug helper: SHA-256
  `052fb76c1c770f42cbe243e65689957daa64726908300600aafb6aa5c9f064f4`.
- `run.sh`, `fill-missing.sh`, process ledgers, original and supplemental
  reports retain the bounded research provenance.

The API/helper were committed as `fdb91db`. Four API regressions and three helper
regressions passed; the independent review checked fixed/grouped order and scale
identity obligations. The actual solver dispatch and solve behavior are unchanged.
