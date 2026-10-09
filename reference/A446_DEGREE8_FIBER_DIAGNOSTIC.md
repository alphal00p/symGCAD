# A446 degree-eight positive-fiber diagnostic

The captured A446 discriminant operand has **four distinct positive roots** at
the recorded prefix. Multiplying it by `(1+x3)^k` cannot reduce its Descartes
variation bound to zero or one, for any nonnegative integer `k`: the multiplier
has no positive roots, and Descartes' rule must still bound the four roots.
There is consequently no evidence here for replacing this discriminant with
a zero-root or one-root coefficient-sign certificate.

The original capture is
`artifacts/hard-campaigns/a446-adjacent-capture-debug-01/initial-x5-0/capture/active-1389059.json`.
It records a degree-eight operand in `x3`, with 1,232 terms and five active
variables. Specializing `(s,t,x1,x2)=(13/3,-1505,1,1)` and clearing positive
rational content gives this primitive polynomial:

```text
3015*x3^8 - 19432080*x3^7 - 180323165076*x3^6
+ 1254991943643984*x3^5 - 933792029036466838*x3^4
- 3745330724354232176*x3^3 - 1584232568212785108*x3^2
+ 8097711397050264112*x3 + 7795744343359479143
```

Symbolica's baseline factorization retained this as one degree-eight factor.
Solving only this specialized polynomial on `x3>0` produced five open cells
with signs `+,-,+,-,+`; independent result verification passed. This is a
diagnostic of one fiber, **not a complete decomposition of A446**.

The research helper independently cross-checks the positive-root count using
Sturm counting on a strict rational Cauchy interval and certified root
isolation. Both return four. For every `k=0,...,16`, its exact Symbolica
coefficient calculation reports four sign variations. The original degree,
nonzero constant endpoint, and unchanged multiplier leading coefficient are
checked. The calculation took 0.743 seconds in the frozen debug helper.

The helper also records nonzero symbolic coefficients that vanish at the
prefix. Such a specialization must not silently become a symbolic-domain
sign certificate. Its three regression tests cover repeated roots, a root at
zero, specialization-induced degree loss, zero fibers, and multiplier
coefficients vanishing only at the sample. These tests passed before freezing.

Reproduce under an external deadline and memory cap:

```sh
nix develop --command bash -c 'ulimit -v 4194304; timeout 30s taskset -c 8 \
  artifacts/hard-campaigns/root-reuse-e43a2c4-debug/profile_operation \
  artifacts/hard-campaigns/a446-adjacent-capture-debug-01/initial-x5-0/capture/active-1389059.json \
  --inspect-only --prefix=13/3,-1505,1,1 --positive-root-diagnostics \
  --output artifacts/profile/a446-degree8-positive-fiber/multiplier-diagnostics.json'
```

The output contains timing, so a replay need not have the same file hash.
These hashes identify the retained evidence:

| Artifact | SHA-256 |
|---|---|
| Capture | `9cf0933ab7e5d259ba92d1ae463bb3ce88dd4e543e30eb42228d8df3efc38cbe` |
| Frozen `profile_operation` helper | `0c604e1926e518d52de0bf75ad6791030b566690c13f4010604028341cafe561` |
| Helper source | `8d2d15e4744566a97629baf23ad7d214d51a7a044737bb81ab6f55b8994725f3` |
| `artifacts/profile/a446-degree8-positive-fiber/multiplier-diagnostics.json` | `366345af66c95839a17216584dae20d869a0d359f38a12f698ac6267e34e50f8` |
| Specialized `input.json` in the same directory | `738f0c5b7b80969d7829464a93b1a8f3e28f75ef016f693153bd804b74292993` |
| Specialized `result.json` | `6945c0c17ca6da4778eca3ef094831e56cebaa8a3182a324b18f694449eb4b93` |
| Specialized `verification.json` | `7112e566fbaa0d885cbebb8e879a8de1ef7db4652c50629c9fb3c7b0c6ee503a` |

The specialized solve used immutable release `7629a91`, SHA-256
`f3479621ada2731221775dea3c04f5552fb0c683af06a67262ce160872132805`.
No production projection rule was changed for this experiment.
