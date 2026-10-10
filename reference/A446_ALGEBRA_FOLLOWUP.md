# A446 algebra and traversal after sum-dependent factor preconditioning

The 300-second `b8139e0` debug trial passed the former 41-term factorization
obstruction but remained incomplete. Its last saved progress reports 103,588
speculative nodes and 15,432 refinements. The final captured operation is
**already complete**: a seven-term factorization taking 53 microseconds. It is
not evidence of a final backend stall.

## Completed operation costs and exact operand recovery

The checkpoint inventory selects files written between the trial input and
final capture timestamps. It contains 14,115 completed operations; the final
progress snapshot precedes the capture by approximately two seconds, so these
counts need not equal that earlier snapshot's counters.

| Operation | New checkpoints | Recorded kernel seconds | Largest kernel seconds |
| --- | ---: | ---: | ---: |
| Factorization | 14,098 | 31.628 | 7.739 |
| Resultant | 9 | 34.294 | 33.896 |
| Discriminant | 8 | 0.00114 | 0.00046 |

The factorization median is 1.26 milliseconds; 14,037 calls are below ten
milliseconds, and 13,415 return one irreducible factor with multiplicity one.
The earlier progress snapshot's factor phase is 98.320 seconds. That phase
also includes canonicalization, hashing, cache handling and diagnostic/cache
file operations. The difference cannot be attributed entirely to native
factorization or entirely to any single overhead. In particular, capture
updates currently use durable atomic writes; this experiment does not isolate
their cost from checkpoint writes or polynomial processing.

The largest resultant eliminates `x3`, producing a 1.70 MB polynomial. The
SHA256 of its compact JSON output array exactly matches the next expensive
factorization's input hash:
`f1ee2a7f91218f5bd59df2f6b09c280b0980abeb73858dd7f74c125eb6a8ec66`.
Thus that factor operand is recovered exactly from completed checkpoint data,
without inferring it from an overwritten active capture. It has 32,905 terms,
degrees `(20,20,20,24)` in `(s,t,x1,x2)` and 115-bit coefficients. Native
Symbolica factorization, with full product reconstruction, takes 7.753 seconds
in a bounded standalone debug replay and returns five nonconstant factors with
multiplicities `2,4,4,4,2`. This agrees with the recorded 7.739-second kernel.

The exact equal-gradient sum test rejects this operand. A broader research
check evaluates its four gradient polynomials at 16 deterministic rational
points and uses Symbolica's exact rational matrix rank. The resulting rank is
four. Any constant rational relation among the gradient polynomials would
also hold in this matrix, so full rank proves that no invertible rational
linear coordinate change can reduce the number of active coordinates.
Translations do not alter this obstruction. Invertible shears that improve
sparsity remain possible.

The same check on the twelve slowest newly completed single irreducible
outputs finds eleven with full gradient rank. The remaining 18-term polynomial
has the already supported equal `s`/`t` derivatives. These operands therefore
do not justify a broader dependency preconditioner. A rank-deficient sample
alone never authorizes a reduction; the helper reports it only as inconclusive.
Exact proportional-gradient identities are checked separately. Two helper
tests cover full rank, sum dependence, a non-unit rational ratio and a
non-pairwise gradient relation. No production code changed for this analysis.

## Warm, no-capture release traversal profile

A separate 60-second full-input diagnostic uses immutable release
`symgcad-affine-sum-b8139e0`, SHA256
`ba0f2a3a8fea50da4d8a10020a870a48ecda64aac595ae5567b97382a7794ed0`.
It starts from an isolated copy of the approximately 2.1 GiB checkpoint tree
after the preceding trial, disables operation capture, retains ordinary
progress reporting, and runs on CPU 8 with a 4 GiB address-space bound. It
returns `incomplete` with `local projection wall time limit`, after 121,994
nodes and 16,126 refinements. This warmer release diagnostic is **not a timing
comparison** to the captured debug run.

Factor phase time is 10.736 seconds, with 370,488 memory hits, 19,728 disk hits,
175 misses and two whole-cache evictions. Other recorded phases are 7.272
seconds in domain certificates, 7.785 in pair separation and 1.246 in
resultants. The profiled process uses 58.15 user and 2.31 system CPU seconds,
61.08 seconds wall time and 371,732 KiB peak RSS.

`perf` records 5,813 samples at 99 Hz with no lost samples. Self-time sampling
identifies the following costs:

| Symbol | Self samples |
| --- | ---: |
| Symbolica/numerica integer GCD | 11.60% |
| Rational cloning | 10.21% |
| Rational multiplication | 7.52% |
| Polynomial `replace_all` | 5.86% |
| Integer quotient/remainder | 4.49% |
| Rational addition | 4.43% |
| Hash writes | 3.44% |

These are self samples, not inclusive caller attribution. Full DWARF report
processing was stopped after excessive processing time; subsequent inclusive
attempts reached their 60- and 45-second analysis limits. Their resource logs
are retained, and no inclusive percentages are inferred from them. The
self-time report is complete. Rational evaluation, cloning and cache work
remain measured targets; no production optimization is claimed here.

Evidence is in `artifacts/profile/a446-affine-sum-factor-followup-01/`:

| Artifact | SHA256 |
| --- | --- |
| Final completed capture snapshot | `13d569933fbb1bb2fe4b31fe04afb9054df774dd45f8c8075a8a87737c2a4a5a` |
| Operation metadata summary | `973ea763ec3d4ba8b289f29c0d57b8937ff8c578a1a921c77e5ca0e64d21bbae` |
| Recovered factor input | `b9b3a6bf290621d42628a68d6a1a22ba736b8b55abcb699570c9af2328f6b590` |
| Native factor replay | `74f3e8c57fe720552d6b630b011308dec6a54d57e2fe54055a63a6e837a3a5b2` |
| Frozen gradient helper | `da932891526f004900a662604f9fb0a73a60227a1445b19585313a3dac66ffeb` |
| Large-operand rank evidence | `cfa6998bc608adb9122119d95c01f27d80b082b6a006b20803fe1ccfe40beed9` |
| Twelve-factor rank evidence | `87d699725dcecd91b040b4afc1df518d0a5999dff719421557d4503a236dfe5a` |
| Full-input profile data | `4f22ec7658704b8c6a34880de2d67b560edf2301f72c3fab200434d250fa0753` |
| Complete self-time report | `18e31120e9036ee27ade3b65089271b46b4d2de9e6be5cf782c5564c283eec78` |

## Shifted-coordinate resultant alternatives

The separate ongoing kinematic-shear experiment supplied a frozen incomplete
resultant capture, SHA256
`5dbe4d4ebe6aeeeb9fa58525f780989cd5ce48d5e9db5daf02f9ea44c7a4840f`.
It eliminates `x3` from degree-six and degree-eight polynomials, with 367 and
800 terms and 22-/37-bit coefficients. Four other active variables remain,
so the bivariate interpolation dispatch does not apply.

Existing deterministic Brown and primitive resultant methods were each tried
with the same frozen debug profile helper, on CPU 9 with 4 GiB and a 60-second
limit. Both timed out without an output. Brown used 59.55 user seconds and
116,856 KiB RSS; primitive used 59.51 user seconds and 118,368 KiB RSS. These
are exploratory failures, not comparisons to the concurrently running release
solver or a reason to change the production resultant strategy. Evidence is
under `artifacts/profile/a446-shift-resultant-01/`, including the immutable
operand snapshot, inspection, commands, stdout/stderr and resource logs.
