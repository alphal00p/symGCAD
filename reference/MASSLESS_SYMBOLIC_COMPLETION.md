# Symbolic massless core completion

The full original problem `test_massless_planar_elliptic_symb/order/1`
now has a 12-cell generic decomposition, independently verified by both
release `e43a2c4` and the frozen `a615d59` debug worker. The original `x0`
coordinate remains unrestricted. All thirteen positive kinematic parameters,
all six positive parameter coordinates `x1` through `x6`, the original strict
inequality, variable groups and split list are retained exactly. No kinematic
specialization or subdomain restriction is used.

The successful lifting order keeps the original outer parameter order, then
uses `[x1,x6,x2,x3,x4,x5,x0]`. The fixture requests heuristic ordering and has
no explicit fixed order, so this order respects its requirements. The existing
homogeneous wrapper fixes positive `x1=1`, and the existing local engine
certifies the reduced problem and reconstructs the original coordinates.
No additional transformation or new proof rule was needed.

## Why this order helps

Write the original strict polynomial as `F=A*x0+B0`. Since `x0` is
unrestricted, a final linear lift needs only the nonzero coefficient A below:
its sign selects the appropriate half-line bounded by `-B0/A`. The generic
seam `A=0` is retained explicitly; projecting the endpoint B0 is unnecessary.

Set `S=x2+x3+x4+x6` and

```text
B = -ss4*x2*x3 + ss45*x2*x4 + ss5*x3*x4
    +ss23*x2*x6 - ss234*x3*x6 + ss61*x4*x6.
A = (ss2*x1-ss1*x5)*S + B.
```

After `x1=1`, A is linear in `x5`, with strictly negative coefficient
`-ss1*S`. Its endpoint is `C=ss2*S+B`. C is linear in `x4`, with strictly
positive coefficient `ss2+ss45*x2+ss5*x3+ss61*x6`. The next endpoint is

```text
E = H*x3 + ss2*(x2+x6) + ss23*x2*x6,
H = ss2-ss4*x2-ss234*x6.
```

The x3 endpoint is strictly positive, and H is linear in x2 with a simple
positive-parameter threshold in the preceding x6 coordinate. These facts
explain the short chain of conditional linear projections. Four exact
Symbolica identity checks reconstruct A, C, its x4 coefficient and E; all
four differences normalize to zero. The full verifier establishes coverage
independently of this explanatory derivation.

## Retained evidence

The initial solve and both independent checks are in
`artifacts/research/massless-free-linear-01`. Its `identity-audit.json` compares
every original mathematical field, checks group/order compatibility and
records all source, input, result, verification and identity-check hashes.
The identity report SHA-256 is
`f47994ca6aed554d22e63a2ffc1a9d095d8dc654497a3794fff41970e071a9ca`.

A separate cold one-case completion campaign read the untouched original
fixture and applied only solver/order options. It also solved and verified
12 cells. Its retained ledger is
`artifacts/completion-massless-structural-01/results/ledger.json`, SHA-256
`67b297074fda0cdcec2f0231f7ebc01a876ef188e50b7ab68ccb1d880b6a9ce7`.
The process solve/verification times were 0.183/0.021 seconds; these are
single completion measurements, not a repeated matched runtime comparison.
The immutable release worker SHA-256 is
`cccff9b56170769fc4b7df64bf8a5673c31bfb51752dcaaeb926b39f14a465e3`.

The research helper `examples/core_linear_probe.rs` checks the general exact
free-axis identity and prepares its coefficient problem. Its three tests
cover rational normalization and rejection of a second axis constraint or
nonlinearity. The successful full-problem solve uses the ordinary solver;
the helper is not a new production elimination wrapper.
