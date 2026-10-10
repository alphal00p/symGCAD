# Positive pair roots above a conditional quadratic endpoint

Research design, 10 October 2026. The identities below were initially
hand-derived and independently reviewed. The subsequent bounded helper now
replays the endpoint, resultant, and Jacobian identities and their ten strict
premises on the authenticated `u>4` operand, as recorded at the end of this
note. The positive-pair topology and selector remain a design requiring
additional whole-cell proofs. No solver completion, comparative benchmark,
or production implementation is claimed. The previous exact evidence remains in
[A446 complement root geometry](A446_COMPLEMENT_ROOT_GEOMETRY.md).

The proposed generic algorithm arranges a quadratic slope and a quadratic
endpoint having at most one positive root, then restores the final affine
fiber. Admission must depend on degrees, exact identities, and domain
certificates, never fixture names. The endpoint theorem already exists in
[conditional quadratic peeling](CONDITIONAL_QUADRATIC_PEEL.md); the new work is
joint arrangement and positive-common-root projection.

## Exact endpoint witnesses

Use the documented representation `F=q*z+B`, with `x=x2`, `y=x4`, `z=x8`,
`u=t` for Ax446 or `u=5/9-s-t` for A446, and define

```text
l=x1+x3, r=x5+x7, a=l+x, b=r+y, m=5/18,
P=u*x1*x3-m*l*x3, Q=u*x5*x7-m*r*x7,
B/(a*b)=a+b-m*(x3+x7)-P/a-Q/b.
```

For positive Feynman coordinates, `u>0`, and more generally `0<m<1`,

```text
a*B_x-B = b*(a^2+P),
b*B_y-B = a*(b^2+Q),
a^2+P = x^2+2*l*x+(1-m)*l^2+m*l*x1+u*x1*x3 > 0,
b^2+Q = y^2+2*r*y+(1-m)*r^2+m*r*x5+u*x5*x7 > 0.
```

Thus `B/(a*b)` increases strictly in each of `x,y`, including the coordinate
endpoints. Only positive denominators were used. Write

```text
B=b*x^2+E(y)*x+C(y),
C(y)=l*y^2+G*y+H0,
H0=l*r*(l+r)-u*(r*x1*x3+l*x5*x7).
```

Coefficient comparison gives

```text
l*E-C = b*(l^2+P) > 0,
r*G-H0 = l*(r^2+Q) > 0.
```

These are precisely the existing conditional-quadratic premises, with
multipliers `l` and `r`. Discovery can reuse bounded multiplier proposals and
ordinary positive-coefficient certificates on an explicit `u>0` chamber.
For A446, dependence `u=5/9-s-t` still needs certified triangular transport or
a certificate accepting that original coupled premise; rectangular bounds
must not silently replace it.

The endpoint inventory is a theorem from these premises:

- `H0>0`: `B>0` on the complete positive `(y,x)` fiber.
- `H0<0`: `C` has exactly one positive simple root `eta`. For `0<y<eta`,
  `B` has exactly one positive simple root `beta(y)`, with signs `-,+`.
  For `y>eta`, `B>0` for all `x>0`.
- At `H0=0`, strict increase gives `B>0` for positive `x,y`; a generic wrapper
  may instead record this seam explicitly as an auxiliary exception.

Both positive quadratic roots have full-real ordinal 1. This proves an
endpoint statement, not positivity of the complete `F`.

## Joint lift and required boundaries

For a generic class, let `q` and `B` be quadratic in `x`, both with strictly
positive leading coefficients, and require the conditional endpoint proof
above or its generic multiplier version. Classify the zero, one, or two
positive roots of `q`, arrange them with the zero or one positive root of `B`,
and restore the positive `z` fiber as follows.

| sign q | sign B | Restored sign F |
|---|---|---|
| + | + | Positive on the whole fiber |
| - | - | Negative on the whole fiber |
| + | - | Negative below `-B/q`, positive above |
| - | + | Positive below `-B/q`, negative above |

The root of the original signed `F` must be exported in both opposite-sign
cases. Away from coincident roots, a single certified `y` sector gives at most
six final cells. The `q=0`, `B=0`, endpoint, and other omitted auxiliary
sections must be explicit exceptions. In particular `q=B=0` makes the entire
`z` fiber lie in `F=0`; it is not an absent boundary.

An initial wrapper can retain ordinary exact resultant projection wherever
the stronger filtered proof below is unavailable. The existing sextic is
not uniformly nonzero, and sampled root counts do not establish its topology.
The positive-pair theorem addresses mutual collisions only: `q` still needs
its own leading-coefficient, positive repeated-root/discriminant, and
zero-endpoint obligations certified before a `y` sector has a fixed root
inventory. Excluding `Delta=0` at common roots does not discharge them.

## The positive-pair topology theorem and its unproved premises

On `H0<0`, the positive part of `B=0` is one smooth decreasing arc joining
finite axis endpoints `(alpha,0)` and `(0,eta)`. Here `alpha` and `eta` are
the positive roots of `B(x,0)` and `B(0,y)`. On the arc, `B_x,B_y>0` follow
from the displayed witnesses. Strict coordinatewise increase bounds every
positive common solution of `q=B=0` inside `[0,alpha] x [0,eta]`.
The simple endpoint roots vary continuously, giving uniform bounds on compact
subsets of an admissible base cell. Therefore these positive pair branches
cannot escape to infinity there.

Define `Delta=q_y*B_x-q_x*B_y`. On a connected base cell, suppose exact proofs
exclude all of the following throughout that cell:

1. Positive interior critical solutions `q=B=Delta=0`, with `x,y>0`.
2. Common solutions at the two positive axis endpoints of the `B=0` arc.
3. Failure of any strict endpoint premise or the chosen `H0<0` chamber.

Then the positive common solutions form finitely many continuous branches,
ordered globally by `y`. Local regularity follows from the nonzero Jacobian;
the compact arc and endpoint exclusions prevent branch loss. Distinct
positive branches cannot share the same `y`, since `B` has a unique positive
`x` root there. This is a sufficient theorem, not a proof that its exclusions
hold on the whole complement or are cheap to certify. A sampled upper bound
of two positive intersections remains unproved.

This covering formulation is consistent with
[Lazard and Rouillier, *Solving parametric polynomial systems*](https://www.sciencedirect.com/science/article/pii/S0747717107000132),
which treats analytic coverings over parameter regions. The explicit arc
bound is needed here to discharge escape to infinity; a Jacobian test alone
would not suffice. Regular semialgebraic systems are a possible exact
representation, as developed by
[Chen et al., *Triangular Decomposition of Semi-algebraic Systems*](https://arxiv.org/abs/1002.4784).
Neither reference proves the application-specific exclusions above.

## Resultant degeneracy and root selectors

In the documented monic case,

```text
q=x^2+b2*x+c2,
B=b*q+J*x+K,
R=K^2-b2*J*K+c2*J^2.
```

On `q=B=0`, the replayed identities imply

```text
Delta=J*q_y-q_x*(J_y*x+K_y),
R_y=J*Delta.
```

For `J!=0`, `x=-K/J`, so `J*K<0` selects positive common `x` and `R_y=0`
is equivalent to true pair criticality. At `J=K=0`, `R_y=0` automatically,
even when `Delta!=0`. A separate chart must distinguish this harmless
projected overlap from actual positive criticality; deleting every such
event is unsound. The previously measured negative-common-coordinate pair
collision also needs a whole-region proof before its projection cut can be
removed.

Current `RootIndexDomain` supports `Real`, `Positive`, and `RealDescending`.
None selects roots of `R` by the sign of their common `x` coordinate:
`Positive` only means positive `y`. An unwanted negative-common-`x` branch
can still have positive `y`, changing root ordinals. Consequently any reduced
projection needs a new **positive-common-solution selector**, or an exact
proof that its chosen existing ordinal stays fixed. Sampled `J*K` filtering
alone supplies neither continuity nor a valid exported selector.

## Connection to critical points of the original affine fiber

At `q=B=Delta=0`, criticality of `F=q*z+B` in the three fiber coordinates
requires `F_x=z*q_x+B_x=0`. Since `B_x>0`, necessarily `q_x!=0`, and

```text
z=-B_x/q_x > 0  if and only if  q_x<0.
```

With that value of `z`, `F_y=-Delta/q_x=0` and `F_z=q=0` as well. This
distinguishes positive-`z` critical points in these three coordinates; it
does not establish stationarity in the other Feynman coordinates or a full
physical Landau solution. In particular a pair event with `q_x>0` can still
be necessary for the cylindrical ordering of `q` and `B`. The condition
`q_x<0` provides no authority to drop such CAD projection boundaries.

## Next bounded research gate

The endpoint and Jacobian identity/premise gate has passed. Next, extend exact
positive-pair diagnostics to the documented negative-coordinate
collision and `J=K=0` event, checking positive endpoint contacts and `Delta`.
The decisive new result would be a certified positive-common selector across
an irrelevant resultant collision, with its root inventory unchanged on the
whole region. Until that proof exists, retain exact fallback projection.

No original-domain completion follows from this proposal. Every chamber and
all original interior `F=0` boundaries remain required.

## Bounded identity and certificate replay

The research-only `examples/positive_pair_replay.rs` accepts a declared
variable order, the original strict constraints, a signed polynomial, final
`y,x,z` axes, positive rational normalization, and two base-only multiplier
proposals. It extracts the coefficients itself, rejects coupled fiber
constraints, and uses the shared Symbolica algebra/resultant and production
domain-certificate kernels. It contains no graph or fixture dispatch, root
classification, filtered selector, or CAD restoration.

For `L=J*x+K`, it checks the full polynomial identity

```text
R_y-J*Delta
 = L*(2*L_y-J_y*q_x-J*q_xy)
   +q*(2*J*J_y+J*b_y*q_x).
```

Thus the common-root implication does not divide by `J` and remains valid
at `J=K=0`. Both normalized-derivative decompositions and both positive-axis
endpoint witnesses are reconstructed. All ten strict certificates are
serialized, deserialized, and replayed against the original constraints;
the checker re-extracts every polynomial and recomputes the identities.

Six focused tests passed, covering two unrelated rational-coefficient
families, a vanishing `J,K` specialization, changed evidence and assumptions,
unsupported theorem scopes, a false additional identity, raw expression
resource limits, and protection against report overwrite. The first two
compile attempts failed on Rust type/ownership mismatches; their logs are
preserved. The successful build/test process used one CPU and an aggregate
2 GiB RSS supervisor, taking 8 seconds with 809,172 KiB sampled peak RSS;
the six tests themselves took 0.03 seconds.

The authentic input is the normalized `F` from the exact source report
SHA-256 `9adaac7df7cde50989684002f8c5a93e225edfc95885c91f8f48f1cb426506b8`,
whose original A446/Ax446 bindings are copied into `preparation.json`.
The domain for this new operand check is `s>4,u>4` and every original Feynman
coordinate positive, with **no `C0` assumption**. This is conditional operand
evidence, not a new original-coordinate transport proof. Each of the ten
strict premises passed using the existing `shifted_coefficients` method.
Three additional exact identities replayed the positive-denominator endpoint
formula and its two explicitly positive derivative witnesses.

The CPU 3 / 2 GiB / 120-second supervised run completed in 0.062551 seconds,
with 7,932 KiB peak RSS. Discovery took 0.019801 seconds and separate
serialization/replay took 0.016903 seconds. These are single research-operand
measurements, not a whole-solver speedup. No critical-locus exclusion or
positive-root topology computation was performed.

Artifacts remain outside commits under
`artifacts/research/positive-pair-replay-01/`:

| Artifact | SHA-256 |
|---|---|
| Generic helper source | `71a83c4e435f5de4f6c6ec2ae3b706fcbfc189a69c6e0f6773b55743ceb689b4` |
| Frozen `helper` | `e7bb69f5101f7cffd36331cc251d815dfd9e2f7b3e30b116011ee33f2f8d5958` |
| `authentic-input.json` | `d3ce4952e7be05136c430fb9db40e6ceae7c9e2598005d7e1efbcd8334c91d96` |
| `authentic-report.json` | `6cf415de6ed62456f92a8480a5c6e5475b2c42d97f8a45568f4b3c4795afd285` |
| `authentic-report.process.json` | `290475f032c29942449b261507d632cfd464eafa44699956a7c86d7b8dfebcf0` |
| `preparation.json` | `996b0aa3a65ed7ad05fdd4f63f8502c285a1d3889fb19b982890e773f3952ac9` |

Source, input, and executable hashes were checked unchanged after the run.
The helper bounds unnormalized Symbolica tokens before Atom parsing, rejects
oversized sparse operands/products, and creates fresh output files. Native
algebra still runs inside the killable external resource supervisor.
