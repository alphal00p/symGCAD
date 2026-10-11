# Bounded quadratic-face projection experiment

Research diagnostic, 11 October 2026. This continues the
[graph-coordinate investigation](CFF_PROJECTION_NEXT_INVESTIGATION.md).
The executable is [`profile_quadratic_faces`](../examples/profile_quadratic_faces.rs).
It constructs and independently replays exact stationary-candidate identities
on every face of a quadratic unit cube. It also measures ordinary projection
through the same coordinate block. **The face inventory is not a substitute
for cylindrical projection and does not certify a new corpus decomposition.**

## Mathematical contract

The input declares a final block of one to three coordinates with conditional
domain `(0,1)^n`; all earlier variables are parameters for this experiment.
Any listed strict prefix constraints must depend only on those earlier
variables. The program accepts a polynomial only after exact checking that
its joint degree in the block is at most two. It does not infer a graph map,
authenticate a supplied provenance label, or prove feasibility of the prefix.

For every relatively open face of the closed cube, restrict coordinates to
zero, one, or free. This enumerates all `3^n` faces, including vertices. Write
the restricted polynomial as

```text
Q(v) = v^T H v + l^T v + c,
d = det(H), n = -adj(H) l,
g = 4 d c - l^T adj(H) l.
```

The executable checks reconstruction, original derivative binding, and the
identities

```text
2 H n + 2 d l = 0,
4 d Q = (2 H v+l)^T adj(H) (2 H v+l) + g.
```

When `d != 0`, the sole stationary candidate is `v=n/(2d)`. Its membership
in this relatively open face is exactly `n_i*d > 0` and
`(2d-n_i)*d > 0` for every free coordinate. Its value has sign `g*d`;
using `g` alone would be wrong for negative `d`. At a vertex use the
zero-dimensional determinant convention `d=1`, an empty numerator vector,
and `g=4c`.

Every specialization `d=0` remains an explicit fallback obligation, including
when `d` is a nonzero polynomial generically. For an identically singular
face the output also lists every nonempty square minor of `2H` and
`[2H|-l]`, with row and column indices. These allow subsequent rank and
consistency stratification; the present helper does not solve the resulting
stationary families or declare them empty. It never removes an `F=0` sheet.

Discovery extracts coefficients using Symbolica differentiation. Replay
deserializes the evidence and extracts them independently from sparse
monomial exponents. Both paths reconstruct and compare the original
polynomial exactly with Symbolica rational arithmetic. Evidence binds the
entire input, variable map, ordered face inventory, guards, and singular
minors; modified identities, missing faces, changed inputs and altered minors
are rejected. This is separate extraction/replay within the same algebra
backend, not an independent computer algebra implementation.

## Bounded measurement protocol

Both modes run as killable subprocesses with the same selected CPU, a
60-second wall-clock allowance and a 2-GiB diagnostic memory allowance.
This small research bound is independent of the larger authorized solver
campaign bounds. The helper additionally bounds input bytes, token nesting,
raw expansion, joint degree, polynomial support, coefficient height,
matrix dimension, intermediate product work and output size. It retains
process information and ordinary-projection progress when incomplete.

Face timings include admission, differentiation, all face identities and
guard construction. Serialization/replay and guard-statistics times are
reported separately; process wall time includes all of them. Ordinary mode
starts from the same polynomial, prefix constraints, and `x,1-x` block
boundaries. It runs the existing leading-coefficient/discriminant/resultant
projection with exact checked factor reconstruction, from the final axis
backward, and stops at the common prefix. It performs no prefix CAD or
lifting. Parsing, initial factorization and every completed block level
are included in its process time.

The two outputs have different proof obligations: the raw face guard
inventory is not a factored complete projection basis. Thus their times and
counts assess the cost of a proposed certificate source, not a solver
speedup. Raw face counts include determinant/value/interiority guards,
singular rank minors and prefix constraints. Ordinary counts refer to the
checked, factored basis.

## Executed measurements

The first frozen executable checked all 27 authentic faces in **0.406 s**
process wall time, with **11,864 KiB** peak RSS. This includes 0.136 s for
discovery, 0.135 s for serialization/replay and 0.108 s for parsing and
canonicalizing guard statistics. There were 51 unique raw nonconstant guards
with 2,320 terms, including prefix constraints. None of the 19 positive-dimensional
faces had an identically zero determinant; their determinant-zero parameter
specializations remain explicit obligations.

The same executable's ordinary projection finished gamma in 0.0096 s and
beta in 0.4243 s, reaching 22 factors and 1,077 terms. The raw alpha output
then exceeded the initial 8,192-term per-polynomial admission bound. The
worker stopped at **1.779 s**, with **38,324 KiB** peak RSS. This is an
admission-limited result, not a completed projection or a 60-second timeout.
The original failure and progress record are preserved. The instrumented
rerun records raw shape before admission and permits at most 65,536 terms
in an ordinary-projection polynomial, within the unchanged process bounds.

That rerun **completed all three ordinary block levels in 8.643 s**, with
**46,812 KiB** peak RSS. It leaves **71 checked factors with 41,150 terms**
over the six-variable prefix `(s,u,X,Y,R,z)`; its largest factor has 6,073
terms. The individual gamma, beta and alpha levels took 0.0100, 0.4250 and
8.0996 s respectively. The alpha raw output has 75 polynomials with 109,635
terms. Exactly two exceeded the initial cap: 10,810 and 9,916 terms, with
full degree vectors `(6,10,20,16,16,6,0,0,0)` and
`(4,10,20,16,16,4,0,0,0)` in the declared order. Their exact polynomial
hashes and raw indices are preserved in the progress sidecar.

The same revised binary replayed all authentic face identities in 0.446 s,
with 12,016 KiB peak RSS. Its serialized evidence is exactly equal to the
first run's evidence; only projection instrumentation and admission changed.
The six-variable prefix still needs decomposition and lifting. Neither this
completed block projection nor the cheaper face inventory restores the
original cylindrical bounds, and no additional corpus case is counted as
solved. In particular, the ratio of their process times is **not** a solver
speedup measurement.

Five unrelated three-dimensional quadratics completed both modes:

| Polynomial | Faces / identically singular faces | Unique raw guards | Face wall time | Ordinary projection wall time |
|---|---:|---:|---:|---:|
| centered positive quadratic minus t | 27 / 0 | 4 | 0.0217 s | 0.0216 s |
| centered indefinite quadratic plus t | 27 / 0 | 4 | 0.0219 s | 0.0218 s |
| `(x+y-1)^2+t*z` | 27 / 11 | 3 | 0.0217 s | 0.0225 s |
| `t*x^2+y^2+z^2-1/4` | 27 / 0 | 5 | 0.0219 s | 0.0233 s |
| zero | 27 / 19 | 0 | 0.0219 s | 0.0214 s |

These tiny process times are dominated by startup and supervisor polling;
they do not resolve an algorithmic timing ratio. All six example unit tests
passed, including malformed input/resource admission, negative determinants,
parameter-dependent determinant zeros, identically singular families, altered evidence,
missing faces, vertices, and the curvature-only counterexample.

Measurements used an AMD EPYC 9754, pinned to logical CPU 3 with one worker.
The helper was a Cargo development build using the repository's optimized
dependency profile; it is not a release benchmark. It used the existing
isolated cached Symbolica checkout based on revision
`75f8350094b90254ee71dc2a391fde0d14b0204a`, with the separately recorded
[positive-axis and unit-shift patches](POSITIVE_AXIS_ISOLATION_RESEARCH.md).
This diagnostic calls no root-isolation API. Ordinary mode leaves the new
opt-in affine-factorization policy disabled. Timing covers only this block,
not the full original graph problem or a reference GCAD run.

Execution evidence is retained in the ignored directory
`artifacts/research/quadratic-faces-01/`; the final manifest records input,
source, executable and report hashes. The initial executable SHA256 is
`eeef39a4384da7fe730a46a284c27eb58c32380a4ee19eeded83e8c9547d1f59`;
the authentic cube input SHA256 is
`fa6b0f762f6fedfc868572e0e42e85955594b9693574d706a423698c7f26812b`.
The final instrumented executable SHA256 is
`68b88f12eaa315afeb88214224c91951b3fefe3ec0c0dd374fd8f6550e59a5af`;
the audited evidence manifest SHA256 is
`7e80ab830aae0d01a2058623b3160cbbedbafec0e836aeadae7a076afc77d364`.

To reproduce the diagnostic interface, save for example

```json
{
  "variables": ["t", "x", "y", "z"],
  "block": ["x", "y", "z"],
  "polynomial": "(x-1/2)^2+(y-1/2)^2+(z-1/2)^2-t",
  "prefix_constraints": []
}
```

and run with fresh output paths:

```sh
nix develop -c cargo run --locked --example profile_quadratic_faces -- \
  quadratic.json --mode faces --output quadratic-faces.json --seconds 60 --cpu 3
nix develop -c cargo run --locked --example profile_quadratic_faces -- \
  quadratic.json --mode projection --output quadratic-projection.json --seconds 60 --cpu 3
```

For the graph trial replace the variable map, block, expression and prefix
constraints by the complete cube formula and domain below, using exact
`m=5/18`. The preceding coordinate note provides the full expression and
both original graph-polynomial bindings. The source has no fixture-ID
branch or special polynomial recognition.

## Authentic graph input and limits

The first input uses the previously replayed A446/Ax12 positive-sum chart,
with lifting order

```text
s, u, X, Y, R, z, alpha, beta, gamma,
s>4, X>0, Y>0, R>0, z>0, u unrestricted,
0<alpha,beta,gamma<1.
```

The polynomial is the complete `compact` expression from the earlier exact
cube replay, bound to both signed original fixtures in the input provenance.
Its transformation covers the positive orthant with positive absolute
Jacobian `X*Y*R`; it is not triangular in the original Feynman order.
Consequently even a complete transformed solve would still require a new
mapped-cell contract or exact restoration to the original cylindrical
bounds before receiving original-corpus credit. No claim depends on
restricting `u` to the physical scattering strip or to `u>4`.

Unrelated convex, indefinite, affine/singular, parameter-degenerate and
zero quadratics exercise the same implementation. The convex example
`(x-1/2)^2+(y-1/2)^2+(z-1/2)^2-t` has constant positive Hessian while its
interior zero set changes at `t=0`: curvature alone cannot delete this
critical-value guard or decide the sign geometry.

## What this could enable next

One sound, narrower application is strict positivity on a **closed** rational
box that contains the relevant projected domain or block fiber. A quadratic attains
its minimum there. Choose a minimizer on a face of minimal dimension. If its
restricted Hessian were singular, a stationary null direction would keep
the value constant until reaching a lower-dimensional face, a contradiction.
Therefore a vertex or nonsingular stationary candidate witnesses the minimum.
If every feasible such candidate is exactly proved strictly positive at a
prefix, strict positivity on the whole closed cube follows. Turning this
observation into a parameter-domain certificate still requires exhaustive
feasibility and sign proofs for all candidates. It is not a free elimination
of singular specializations from a cylindrical decomposition, an assertion
about open-box boundary zeros, or a location theorem for interior roots.

For actual cylindrical projection, the next experiment should target the
ordered sequence of endpoint and discriminant obligations. Joint quadratic
structure gives linear differences between opposite endpoint restrictions
and can relate endpoint/discriminant resultants to squares of smaller
resultants, subject to degree-drop and leading-coefficient guards. Every
such identity needs exact replay and a complete list of retained guards.
The current all-face inventory does not prove those ordered obligations.

Graph-derived worldline or positive-sum coordinates are therefore a promising
way to expose small matrices and parameter-dependent certificates. CFF normal
thresholds can organize candidate prefix chambers, but momentum-space
threshold guidance alone still does not certify Feynman-parameter delineability.
Worldline insertion positions generally lie in products of ordered simplices,
not this unit cube. Extending the method there requires their actual affine
face restrictions and exact domain coverage; a nonlinear simplex-to-cube
substitution must recheck joint degree and cannot inherit this admission test.
