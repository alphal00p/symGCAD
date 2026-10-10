# Bounded recursive quadratic positivity screen

The research prototype extends the existing strict quadratic nonvanishing
argument to a bounded tree of such arguments. It proves a nested test that
depth one cannot prove, but found **no additional certificates** in two
completed scans of authentic projection outputs. There is no production
change, factor removal, new decomposition, or claimed solver speedup.

For `p=a*z²+b*z+c`, the identity
`4*a*p=(2*a*z+b)²+(4*a*c-b²)` gives strict nonvanishing if `a` has a
certified strict sign and `4*a*c-b²>0` throughout the target-free domain.
Either coefficient obligation may itself be established by another quadratic
step in an earlier-independent coordinate. Each child loses its parent's
axis; no sampled value or target sign is used as a premise. Repeated roots
and same-sign interior zeros are not omitted: a merely nonnegative
discriminant complement or square does not satisfy this rule.

[`profile_recursive_quadratic.rs`](../examples/profile_recursive_quadratic.rs)
uses a separate research evidence tree. Replay extracts the coefficients,
reconstructs the polynomial exactly, checks the supplied child proofs, and
uses the production independent verifier only for nonrecursive leaves.
It does not rediscover candidate axes or accept its evidence as a production
`DomainCertificate`. The current production depth-one semantics remain intact.

## Experimental envelope

Inputs and every recursive polynomial are bounded to 128 terms, 16 map
coordinates, axis degree 32, and 512-bit combined rational numerator/denominator
height. Coefficient products have a 4,096-pair estimate cap before multiplication.
Each discovery trial uses at most four quadratic axes per node, depth at most
three, 128 visited nodes, and a one-million sum-of-squared-term-count estimate.
The latter bounds this discovery traversal; it is not an exact count of all
arithmetic inside the existing leaf portfolio. Leaf attempts retain that
portfolio's own limits. Parsing and native algebra also run inside a killable
worker with a 60-second deadline and 2 GiB memory limit.

The worker accepts at most 4,096 checkpoint files, 32 MiB per file, and
128 MiB total input. It checks each operation's output checksum over
`(output, multiplicities)`, retains source hashes, and requires one explicit
variable map. These are provenance checks, not proofs of the cached operation.
The independently replayed candidate polynomial identity and domain evidence
are the mathematical claims made by this experiment. Large expressions and
directory enumeration remain bounded by the external worker, rather than by
a separate raw-expression preflight.

The original target-free constraints are supplied explicitly. Separate input
audits bind them to the recorded problem and every checkpoint's domain
fingerprint; the helper itself does not import a `Problem` TOML. All eligible
outputs first receive the unchanged production portfolio. Only its declines
are attempted at depths zero through three, stopping after a checked success.
Skipped, oversized and nonquadratic polynomials are not counted as checked.

## Authentic input results

Both fresh development-build processes used CPU11, one worker, the same
immutable executable, and no injected algebra cache or changed domain.

| Input inventory | Unique outputs | Eligible / checked | Already certified | Additional | Process wall | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| A414 fixed-work traversal, all 212 completed operations | 198 | 175 / 175 | 2 | 0 | 0.590241 s | 9,244 KiB |
| Physical elliptic traversal, all 2,655 completed factor operations | 1,904 | 184 / 184 | 76 | 0 | 0.609767 s | 14,488 KiB |

The A414 domain is the original homogeneous-ratio domain `s>4`, unrestricted
`t`, positive `x1,x2,x3,x4,x7`, and the constant positive anchor. The physical
elliptic domain retains `s>4`, `t<0`, `0<pp4<4`, positive
`x0,x1,x2,x3,x5`, and its constant positive anchor. Neither scan assumes a
sign of the original `F`. The physical example is already a solved corpus
problem; it supplies a useful coefficient test set, not a new completion.

The unchanged portfolio therefore declined 173 and 108 eligible polynomials,
respectively, and bounded nesting also declined all of those. This does not
prove nesting useless on other operands, larger envelopes, or differently
conditioned exact domains. It does mean these two screens provide no measured
reason to incur production discovery or proof-format costs.

Evidence is retained under `artifacts/research/recursive-quadratic-a414-02/`
and `artifacts/research/recursive-quadratic-physical-01/`:

| Record | SHA-256 |
|---|---|
| A414 scan | `ba58651d94563cd70e800c4df7583b80b86dc45f6cb933be7543d1637e8b2a1c` |
| A414 process | `25045e85670d2738de619b0b38a6d972efa06c395150551e84788e68441481ca` |
| A414 input audit | `ce33f89a740e350cfd502204d4b61388488bf11c0b475140fd3e0894f4542568` |
| Physical scan | `ba5ac907cc48b24456b30dbcca6abe4227516465ba44cd7de42d11253f4c51c9` |
| Physical process | `4812c556b620d90fb0b1ee9106c0c08b3907017f249495092684c729363b3903` |
| Physical input audit | `0f2d4090446896081c4f22cde1a5fdf600d4ea8ee4f58a762dec64096cdda4a7` |
| Frozen helper | `bbaf53c15f0f7cec8a1ec7455635d2969a3b7ae421c574c741901c5079ceff0f` |
| Helper source | `fc4e5c73dffa7c6bff67cd0c2acaf579883483c7e26270ad9e7a9e95bbd6cf21` |

The first A414 attempt, `recursive-quadratic-a414-01`, failed immediately
because its checksum reader omitted multiplicities. Its failed process record
is preserved. The corrected reader matches `OperationFile` and has a regression
covering null multiplicities, factor multiplicities, and tampering.

Four focused tests pass. They include a genuine nested example
`(z+x)²+(x+y+1)²-m*(x+y)` on `0<m<4,x>0,y>0,z∈R`, rejection at depth one,
successful deeper independent replay, forged polynomial/domain/depth rejection,
resource limits, and rejection of interior zeros, degree drops and sign changes.
Tests and helper builds are recorded in
`artifacts/research/recursive-quadratic-build-02.log`, SHA-256
`4b7c91353d7a7e584552dbba477381b2ed9cc77210c541faa37dedc844493346`.

The wider research directions and their interior-boundary obligations are in
[the positivity generalization review](QUADRATIC_POSITIVITY_GENERALIZATIONS.md).
