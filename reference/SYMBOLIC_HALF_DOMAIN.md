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

Two complementary `t>0` trials with the same release worker and Feynman order
both timed out at 60 seconds: one used outer order `[s,t]`, the other `[t,s]`.
A later `e8e1cd4` debug trial with interval and pair witnesses enabled also
timed out at 300.065 seconds. Its final captured discriminant operand has
degree eight and 4,467 terms, exceeding the interval rule's 4,096-term input
cap. The bounded research interval probe correctly declined at that cap;
no enlarged-cap certificate or speedup is claimed.

Records are under `artifacts/research/ax12-446-positive-t-01`, in
`s-t-order`, `t-s-order`, and `interval-s-t-order`. Their `initial-x2-0/record.json`
SHA-256 values are respectively
`6811cddc1d9feef2de29b3e3fbea4901740b4805182acde7799577842c1593a3`,
`fbf2962130218d6ff5a87209e1237b45edffd831410ed12ff77264f576e09e5e`, and
`58f0b44c7ef74fe4a9edd4f6abb26d625170fc485234834a29ccc7b626a6e529`.
The immutable last operand is retained at
`artifacts/profile/ax12-positive-t-interval-capture-01/capture.json`, SHA-256
`e3cd17f95b37630962b525579d51c3e4f980f2970d22bbff8ad12c677510a0d8`.
