# Symbolic Ax12/A446 on the negative-t subdomain

The unchanged symbolic Ax12 fixture, with the single additional constraint
`t<0`, has an independently verified eight-cell generic decomposition: seven
positive cells and one negative cell. This is a strict subdomain experiment.
It does not complete either unrestricted symbolic A446 or Ax12 corpus problem.

Source: `artifacts/corpus/symbolic/0152_finiteAx12_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml`.
All original polynomial expressions, exact substitutions, positive-coordinate
constraints and variable groups are retained. The order is
`[s,t,x2,x4,x8,x7,x5,x3,x1]`, with positive scale anchor `x2`, the certified
monotone engine and the local engine for its endpoint problem. The input keeps
`s>4` symbolic. Neither kinematic variable is fixed numerically.

The grouped identity in `examples/symbolic_structure_audit.rs` explains this
region's simpler geometry: the crossed invariant multiplies a polynomial with
positive coefficients, and its negative sign makes that contribution positive
when `t<0`. The existing monotone certificates suffice. The checked crossing
identity in `reference/CORPUS_SYMMETRY_AUDIT.md` maps this region to
`t_A>5/9-s` in A446; no transported A446 cell artifact is claimed here.

One fresh release process (`e43a2c4`, executable SHA-256
`cccff9b56170769fc4b7df64bf8a5673c31bfb51752dcaaeb926b39f14a465e3`)
took 0.0222 seconds under a 60-second limit, CPU 1 and 4 GiB memory cap. The
campaign, including preparation and independent verification, took 0.306 seconds.
This is an exploratory single measurement, not a three-repeat benchmark.
The later immutable `a551f36` debug verifier independently replayed the result.

Artifacts are in `artifacts/research/ax12-446-negative-t-01/`.
The `campaign/initial-x2-0/` hashes are:

| File | SHA-256 |
| --- | --- |
| `input.json` | `b69a9326a0d7a3281911e0c8653f3b609f61a7c2f9ecbc4fc171eb19e02da845` |
| `result.json` | `13713c531fbb56cad88c53db543ae44bad258a2d93c23f39b1389550f9253300` |
| `verification.json` | `e022eea311d3c752c4e8336d29af2a81ab10b2761b84644fb059dd3a593a9214` |
| `record.json` | `5d74d22a2bde4513040077d108efb7e03fce3171cde14d6c156eabe6fce85332` |

The complementary positive-t region remains unresolved. This subdomain result
does not change the original-corpus completion count.
