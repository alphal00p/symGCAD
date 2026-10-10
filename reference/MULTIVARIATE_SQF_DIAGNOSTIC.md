# Bounded multivariate square-free product diagnostic

This research-only gate measures native square-free decomposition separately
from full signed product reconstruction. It is motivated by the pending
three-variable factor operation described in
[the generic support-basis design](GENERIC_SUPPORT_AND_AFFINE_GATES.md), but
its admission rules contain no case identity, graph topology or polynomial
fingerprint. It does not replace any projection basis.

[`profile_multivariate_square_free`](../examples/profile_multivariate_square_free.rs)
accepts a SHA-256-pinned captured factor operand and its complete variable map.
It calls native `square_free_factorization` once. All proposed constants,
nonconstant blocks and multiplicities are retained, serialized and parsed
back before reconstruction. The proposal is saved atomically **before** the
possibly expensive product replay, with its verified flags false. Only a
successful final report, bound to that proposal digest, asserts exact equality
of the complete signed source and product. A timeout after proposal publication
therefore preserves reusable research data without labelling it checked.

The exact equality retains every source zero, including even-multiplicity
interior sheets. It does **not** independently establish square-freeness,
coprimality, irreducibility or delineability. A multivariate CAD basis would
still need all content, LC/discriminant/pair and nullification obligations
identified in the design note. No solver cache is read or written; no root
inventory or original-case completion follows from this diagnostic.

## Generic resource envelope

- At most 64 map entries, three active variables, 2048 source terms, degree
  32 in each variable and total degree 64.
- Literal sparse rational grammar only: no function calls, nested powers or
  arbitrary expression normalization. Numerators and denominators are at most
  4096 bits; input coefficient storage is at most two million bits, and the
  summed denominator-bit bound is at most 4096.
- At most 64 native blocks, 4096 total block terms and two million coefficient
  bits. Nonconstant native blocks must be integral; rational scalar blocks
  remain explicit. Every factor has positive multiplicity at most 64, with
  summed multiplicities at most 192.
- Weighted individual and total degrees must match the source before product
  work. Product numerator norm is bounded by 16,384 bits, and scalar denominator
  growth by 4096 bits. Every multiplication has at most eight million sparse
  term pairs; the full reconstruction has at most 64 million. Intermediate
  dense support is at most 40,000 terms.
- Captures and aggregate proposed expressions are at most 4 MiB; each output
  record is at most 16 MiB. A fresh output directory is mandatory. The proposal
  is never overwritten; progress is replaced atomically.

The whole worker, including parsing, native decomposition, proposal I/O and
reconstruction, runs under one explicit deadline of at most 3600 seconds and
4 GiB. Native internal allocation is contained by that external limit; static
output bounds alone do not bound the native algorithm. An interrupted stage
is incomplete, and the helper does not launch an alternative factor backend.
The supervisor retains the process outcome before final evidence checks and
checks input and executable hashes again afterward.

## Validation and interpretation

Focused tests exercise multivariate signed rational content, repeated factors,
source-sign and multiplicity tampering, bounded sparse grammar, immutable
unverified checkpoints and source-byte changes. An explicit fixture also
accepts a correct product whose blocks are not square-free or pairwise coprime:
that is intentional evidence that product equality does not silently grant
those stronger claims.

Each completed report records native SQF, proposal serialization and signed
reconstruction time separately. Whole-process wall time remains authoritative.
Before recommending a production representation, subsequent measurements
would have to include the next full LC/discriminant/pair projection and root
union costs with the actual companion cuts. Fast standalone SQF is insufficient
if retaining composite blocks makes downstream projection much larger.

All four focused tests passed in the coordinated test/build bundle, with
independent source review before execution. The immutable executable and source
manifest are under `artifacts/research/multivariate-sqf-build-01/`:

| Evidence | SHA-256 |
|---|---|
| Frozen executable | `d15d873cde349179d4f02ad99e0ba76aec9a9f1efc0465c3de367fedb64f1319` |
| Helper source | `3337626373512878e725a6e5ed93c631d6b6a510cae246ab0c68e849b1d21b98` |
| Library/source/Cargo/build manifest | `ba33e2fb020b4405e1476d29a35c695303853d7f71547823f135c2f107cd3308` |

## Authentic three-variable result

The frozen helper ran on CPU1 with 4 GiB and a 3600-second worker ceiling,
inside a 3675-second external deadline. It completed in **0.042473487 seconds**,
peak RSS **9644 KiB**, exit zero and untruncated output. Input and executable
hashes remained unchanged. The reviewed sparse parser preserved the full map
`[x2,x1,x7,x4,x6,x3,x5]`.

| Stage | Seconds |
|---|---:|
| Native square-free decomposition | 0.006578250 |
| Proposal serialization and roundtrip | 0.001959256 |
| Full signed product reconstruction | 0.000682642 |

The original 224-term, degree `[9,9,6]`, 17-bit polynomial reconstructs exactly
as the product of two multiplicity-one blocks:

| Block | Terms | Degrees in `[x2,x1,x7]` | Total degree | Maximum coefficient bits |
|---|---:|---|---:|---:|
| `1+x7+x2` | 3 | `[1,0,1]` | 1 | 1 |
| Retained composite proposal | 158 | `[8,9,5]` | 11 | 16 |

There are 1866 coefficient-storage bits across the blocks, including denominator
bookkeeping. Reconstruction consumed 477 sparse term-pair products. All signed
scales and factors remain in the proposal; the positive-looking linear factor
is not silently removed by this helper. Neither block's irreducibility or
coprimality with future stack cuts has been established independently.

This cheap SQF call shows that the standalone decomposition stage is feasible
for this authentic operand. It does not locate the historical pending factor
call's internal phase, give its exact duration, or measure a whole-case speedup.
The retained highest-axis degree falls from six to five, but even its ordinary
discriminant has conservative lower-coordinate degree bounds `[64,72]`.
Full projection and interactions with the other active cuts remain unmeasured.

Run evidence is under `artifacts/research/gluza33-multivariate-sqf-01/`:

| Evidence | SHA-256 |
|---|---|
| Copied original capture | `162d5b9b98efeed85cadf3f375ad914fa902682a58764d927505927e66100646` |
| Launch record | `fefa9adcf58094e2de5195945c30bcd1953da02018f293e46d54ac1654588fd7` |
| Process outcome | `3975ddb7476c3da2b36ba80fbc6c10d3f2f2d4ec53b23ce4d37b491e0b0bde93` |
| Immutable unverified proposal | `334f7737e506cd10771080ef7eb4b8b1c3a3fb3f290d34bf5e21490f19745f1d` |
| Completed worker/product report | `2188a8841b14f2d21ebe2cbaf5838b06b04240312a6dd6e3d7c1558ce00ea164` |
| Final supervisor report | `4ea8250c2bb293ff4500c6ddcfab38ab7200b99a6bc06b4cec864384a9c73724` |
| Independent metadata/provenance audit | `fee8a0c6f4d2152c9d3659fc32859c5a46281b5db45fd587ed906e9d1a830562` |

The proposal intentionally keeps `exact_signed_product_verified=false`;
the successful report binds its digest and records the completed replay.
The independent audit rehashed the original and copied capture, ordered
operand/full-map binding, frozen source/binary/build/launch, and complete
process/worker/proposal records. It checked all reported degrees and
multiplicities without rerunning native algebra. Its corrected v2 audit passed;
the initial audit script's status-key schema mismatch remains retained.
No original CAD coverage was gained or claimed.
