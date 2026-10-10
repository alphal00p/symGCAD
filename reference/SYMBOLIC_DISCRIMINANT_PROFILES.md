# Captured symbolic A317/A318 discriminants

These are bounded research diagnostics, not completed decompositions or a
release parity benchmark. Both original symbolic problems remain unresolved.
All reference algebra and inputs remain outside commits.

## Original bounded searches

The public CLI ran the original A317 and A318 symbolic problems with their
parameter-first groups, exact masses, and verified homogeneous anchor `x4`.
Each cold search used seed 0, at most 256 trials and 300 seconds, followed by
a solve of the suggested order within the remaining shared budget. Neither
search found a completed ordering: each retained 99 unsuccessful trials.
The subsequent solves also remained incomplete.

| Original problem | Public worker wall time | Peak RSS, KiB | Status |
| --- | ---: | ---: | --- |
| A317 symbolic | 1501.081160 s | 834812 | incomplete |
| A318 symbolic | 1501.080868 s | 729428 | incomplete |

The frozen solver was `symgcad-indexed-merge-dda5980`, SHA-256
`f300617988679a167a6911b79c547f5b82c623bf368a6cc704b9a9611c3a8a67`.
Each worker used one assigned CPU and an 8-GiB memory cap. The Rust supervisor
retains the original inputs, hashes, process outcomes, search journals and
captures under `artifacts/research/symbolic-mcts-03/{a317,a318}/run`.
Its compact summary is `artifacts/research/symbolic-mcts-03/summary.json`.

The old public timeout exporter reported only search time in
`statistics.total_seconds`; these measurements use the enclosing supervisor's
wall time. A killed solve is not free. The reporting defect has a separate
regression fix; old result files are retained unchanged.

## Captured operation costs

At interruption, each final solve was computing a degree-eight discriminant
in `x2`. The operands have four active variables; the other three map entries
are inactive. Thus the existing exact interpolation helper for a single
remaining parameter does not apply directly.

| Capture | Terms | Degrees in active map order | Maximum coefficient bits |
| --- | ---: | --- | ---: |
| A317, `[s,t,x0,x2]` | 1307 | `[9,6,8,8]` | 44 |
| A318, `[s,t,x1,x2]` | 1210 | `[9,8,6,8]` | 43 |

The captured JSON hashes are respectively
`98a017485860a2c11bfa9933a9b88c07eba09ec6372752c10115c1d10d2780ce` and
`2025901ae58829c481d74d3dc6596bf5d230e9b801e7ad0506e274e313a3befb`.

Fresh 60-second workers compared existing Symbolica integer Ducos, Brown,
primitive, and CRT strategies on both operands. **All eight timed out**,
after 60.027–60.059 seconds. Peak RSS ranged from 83,716 to 296,876 KiB;
each used about 59.2–59.4 CPU seconds. There is no finite speed ratio, completed
candidate, or measured basis for a production backend change from these runs.

This is a diagnostic build with Symbolica dependencies at optimization level
2, not the release engine. The frozen helper hash is
`be636431ce37bca78e345f54cf0efd65c117293bffdd29fa7ea0340350080ae7`.
Per-worker records are in
`artifacts/profile/symbolic-mcts-03-discriminants/*-*.process.json`.
The initial eight-trial summary is `strategy-summary.json` there, SHA-256
`939700a03ba6d0194829e168b28a95cf2d6c26994e33ba1ec5c7efcb92d22be2`.
The recorded backend's CRT early-stop policy is probabilistic: any future
completed candidate needs independent exact validation before production use.

## Reproducible bounded profiling

`examples/profile_operation.rs` accepts `--seconds` and `--memory-mib` to run
the entire operation in the existing Rust killable-worker supervisor. It
requires a fresh output and process sidecar, records input and executable
hashes, and retains timeouts as process results. Omitting `--seconds` preserves
the original direct-helper interface. For example:

```sh
profile_operation capture.json --strategy integer-ducos \
  --seconds 60 --memory-mib 8192 --output trial.json
```

The optional research flag `--reverse-variable` computes the discriminant of
`x^d p(1/x)`. For a degree-preserving reversal its discriminant is exactly
that of `p`: the root differences acquire denominators cancelled by the
reversed leading coefficient. The helper checks the degree and reconstructs
the original polynomial by a second reversal. It rejects a zero constant
coefficient that would lower the actual degree. This changes only the
intermediate algebra, not the decomposition's coordinates or root selectors.
Exact low-degree parameterized regressions check the identity. No production
projection operator uses this experimental representation choice.
