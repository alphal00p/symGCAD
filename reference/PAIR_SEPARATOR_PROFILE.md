# Positive-root pair separators: proof and A446 operand profile

This research diagnostic replaces one pair-resultant obligation by rational
evaluation guards, conditional on separately proved positive-ray delineability
of both polynomials. It does not prove that prerequisite or solve the original
GCAD problem. All reported timings are from one frozen **debug** helper.

## Certificate and scope

Suppose two polynomials `p(u,z)` and `q(u,z)` have continuous, individually
disjoint simple positive-root graphs, with constant counts, on a connected
ancestor cell. Isolate their complete positive-root union at its rational sample.
If different polynomials share a positive root, decline this certificate and
use the existing fallback/refinement rule.

For **every adjacent cross-owner edge** in the ordered union, choose a fixed
positive rational `c` strictly between its two branches. Require both `p(u,c)`
and `q(u,c)` to be nonzero throughout the ancestor cell. Neither neighboring
continuous root graph can cross `c`, so that edge's strict order persists.
Same-owner edges are already strictly ordered by the individual delineability
proofs. Transitivity preserves the whole union order; no positive root of `p`
can coincide with a positive root of `q`.

The evaluations become recursively checked ancestor obligations, not merely
sample tests. Both polynomials' evaluations are necessary. Every cross-owner
edge is necessary: for example, at `u=4`, `p=(z-1)(z-3)` and
`q=(z-2)(z-u)` have three such edges. Protecting only the first two leaves the
collision at `u=3` possible while both polynomials remain individually simple.

The rule concerns positive roots only and must remain restricted to a proved
positive-ray stack. Common negative roots and negative-root collisions are
irrelevant. Witness branch ordinals are positive ordinals; individual exported
bounds retain the index scope justified by their own delineability proof.

An independent checker can avoid repeating union isolation. It must check a
complete permutation of each owner's positive ordinals in their internal order.
For an edge `(p,i) < (q,j)` with supplied rational separator `c`, exact Sturm
counts must give `N_p(0,c)=i+1` and `N_q(0,c)=j`, and both evaluations must be
nonzero. These facts locate `c` strictly between exactly the claimed branches.
All cross-owner edges must appear; missing edges are not justified by checking
only the supplied ones.

Discovery should retain at most two complete templates per canonical pair and
axis, without resetting or evicting that stock during a solve. A template includes
counts, owner/ordinal order, and all separators, not just a list of rationals.
Reuse requires checking it at the new sample. On stock exhaustion, fall back to
the existing pair resultant. The stock is heuristic state, never proof authority;
the checker validates the supplied template directly. This bounds this source
of generated lower-variable guards, avoiding the unbounded separator discovery
described in [the individual witness audit](DESCARTES_SEPARATOR_PROFILE.md).

## Exact capture and measurements

Frozen capture:
`artifacts/profile/a446-pair-separators-01/capture-observed.json`.

- SHA256: `04c6e1688a240baf763a2108fc58091e287989aa2c3759d72a076a0f9ed91ecb`.
- Operand hash: `64f968bfe1382077f1636288ff9c4e332753e6cdcef02a5e01115e5f948e64ea`.
- Variables: `[s,t,x1,x2,x3,x4,x7,x8]`, main variable `x3`.
- Sample: `(s,t,x1,x2)=(13/3,-1505,1,1)`.

The active operation file in the original worker changed while the helper was
built. The report's own input checksum identifies this later pair, which was
then copied unchanged to the frozen path above. It is **not** the initially
requested capture with SHA256 `26fd76981a527e7a837c0df79051ac442e4b36f3ff363b2513d21f90fd0d2338`.
The quoted measurements apply only to the frozen input matching the report.

The helper parses these exact operands afresh: the first is a 58-term quartic
with degree vector `[2,2,3,4,4,0,0,0]`; the second is a 1,232-term octic with
degree vector `[4,4,8,8,8,0,0,0]`. In this final frozen pair, the operand strings
have lengths 967 and 35,149 bytes. The octic string is **exactly identical** to
the old discriminant operand in capture `9cf0933ab7e5d259ba92d1ae463bb3ce88dd4e543e30eb42228d8df3efc38cbe`.
This was checked by direct equality of the decoded JSON operand strings. The
initial, unprofiled `26fd...` pair instead had lengths 1,662 and 29,326 and a
different octic string; those initial-pair details do not describe this profile.

Each operand has four simple positive roots. The distinct union has eight,
with owner sequence `[0,0,1,0,1,1,1,0]`. Its four cross-owner edges yield:

| Edge, using positive ordinals | Separator | Signs `p(c),q(c)` | Guard terms | Maximum coefficient bits |
| --- | ---: | --- | --- | --- |
| `(0,1) < (1,0)` | `1/2` | `+,+` | `41,582` | `13,38` |
| `(1,0) < (0,2)` | `2` | `+,-` | `41,582` | `14,41` |
| `(0,2) < (1,1)` | `698` | `-,-` | `41,582` | `46,102` |
| `(1,3) < (0,3)` | `8192` | `-,+` | `41,582` | `60,131` |

Coefficient size here is numerator bits plus denominator bits. Every evaluation
guard remains one factor. `factors_checked` reconstructs each exact original
guard, including scalar content and multiplicities.

| Work | Seconds |
| --- | ---: |
| Exact individual and merged positive-root isolation, including simplicity checks | 0.00483 |
| Symbolic construction of eight evaluation guards | 0.00267 |
| Complete discovery | 0.00923 |
| Checked factorization of all eight guards | 0.04662 |
| Supervised worker wall time, including setup/output | 0.18408 |

The worker used 0.14075 user CPU seconds, 0.02481 system CPU seconds, and
13,528 KiB peak RSS on CPU 7, under a 4 GiB/60-second bound. This establishes
cheap construction and factoring for this operand. Propagating the guards can
still produce expensive ancestor obligations; no whole-solve improvement or
completion follows from this microprofile alone.

## Reproduction

```
nix develop --command cargo test --example profile_pair_separators -- --test-threads=1
nix develop --command cargo build --example profile_pair_separators
nix develop --command target/debug/examples/profile_pair_separators \
  artifacts/profile/a446-pair-separators-01/capture-observed.json \
  --prefix '13/3,-1505,1,1' --factor-guards --seconds 60 --cpu 7 \
  --output artifacts/profile/a446-pair-separators-01/cost.json
```

The helper uses Rust `benchmark::run_bounded` for process limits and Symbolica
for algebra. It checks operand hashes, equality of symbolic and specialized
evaluations, degree preservation, nonzero endpoints, positive simplicity, and
individual-count versus distinct-union count. Atomic progress preserves every
completed factorization under interruption. Six tests passed, covering every
cross-owner edge, same-owner edges, shared positive roots, common/colliding
negative roots, repeated positive roots, degree loss, zero endpoints, and a
positive-rootless member.

- Frozen helper: `artifacts/profile/a446-pair-separators-01/profile_pair_separators`.
- Binary SHA256: `cd53c30126844b8dac9eac8df536345c52ecf288c657ce26dead6945c686069b`.
- Source SHA256: `d15ac84a3604fd7cc1de07380f4284bc0c515e251a23487a8024ac7f8ff12af9`.
- Report and raw test log: `artifacts/profile/a446-pair-separators-01/cost.json` and `tests.log`.
- Report SHA256: `4f7207d1e730715e6a3c26ded5e82e52551a216d7a473bf5816bddc6e941a5db`.
- Test-log SHA256: `e6939743400d0fa4b1f3a4534e7f981c0ef74971a61708d5b8fda7803b9cfaf6`.
