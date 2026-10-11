# Gluza33: the source-specific cause of the discriminant fallback

The full original solve now directly explains why a successful fresh interval
certificate was not used: **this polynomial's two permanent interval-template
slots had already been consumed at earlier prefixes.** Both stored templates
fail at the quarter prefix. Discovery is therefore skipped and the solver
requests the large discriminant. This is an observed resource-policy gate,
not a failure of the exact certificate theorem or an original-problem solve.

The source is the same 19,942-term polynomial, degree 24 in x7, with full map
`[x2,x1,x7,x4,x6,x3,x5]`. Its operand hash is
`eba809f23b0ad1faa65f33832b6d786d63df851a0c4a4888781e4987430bad5e`.
The isolated trace selects this hash only for diagnostic output; it changes
no solver choice, algebra operation, cache, proof guard or acceptance test.
It records exact prefixes, the positive-ray/ordinary route, actual retained
templates and each existing admission/reuse branch.

## Observed lifetime history

The original order, anchor and mathematical input are unchanged. Global and
interval source allowances are 32,768 terms, aggregate interval guards are
limited to 131,072 terms, and interval stock capacity remains two.

| Exact prefix | Global Descartes path | Interval path |
| --- | --- | --- |
| `[2,1/2]` | Two positive roots, four variations: not saturated | Admits `(0,1),(1,infinity)`; slot 1 |
| `[2,2/3]` | Two positive roots, four variations: not saturated | First template fails; admits `(0,1/4),(1/4,1/2),(1/2,infinity)`; slot 2 |
| `[2,3/2]` | Three roots, three variations: accepts separators `1/2,1` | Skipped after successful global certificate |
| `[2,1/4]` | Four roots, eight variations: fresh global discovery fails | Both stored covers fail; stock 2/2 prevents fresh discovery |

At the quarter prefix the first interval cover has three variations on
`(0,1)`, and the second has three on `(1/2,infinity)`. The interval rule
requires at most one variation per piece. The trace records
`permanent_stock_limit` and `discovery_attempted=false`, followed by the
discriminant request. This is the positive-ray producer, with no full-real
replay or ordinary-route fallback. The global stock is only 1/2 full: raising
that quota would not fix its unsaturated root bound.

The operation capture then matches the historical incomplete discriminant
byte for byte, SHA-256
`5c15531346bf5a4a4e247db0f23fdbb38031d1bdf0485f7e08e7af7b1c2ad3d6`.
An external observer stopped only the identified owned worker group after
matching its executable, PID/group, source, prefix and pending operation.
This deliberately interrupted diagnostic cannot be reported as a solve time.

The independent fresh-stock construction already documented in
[the interval frontier investigation](GLUZA33_FINAL_INTERVAL_FRONTIER.md)
certifies four positive roots at this same prefix with four pieces,
97 guards and 109,726 terms. The trace connects that earlier operand-level
success to the actual solver state; it does not itself discharge the guards
throughout the original ancestor cells.

## Smallest justified follow-up

The existing `local_descartes_interval_template_limit` option can be raised
from two to four without changing any certificate or verifier. Slots remain
permanent, finite and bounded, even after discarded tentative stacks. Thus
the existing downward-variable finiteness argument still applies. The next
original-problem ablation changes only this option and retains all source,
guard, order and domain obligations. It may encounter another obstruction;
bypassing this call is not evidence of complete coverage.

Smaller interval covers also have a reuse tradeoff. For the hand-derived
example `p_t(z)=(z+1)(z^2-t*z+1)`, a whole-ray cover works at `t=0`. At
`t=3/2`, its coefficients have signs `+,-,-,+`, whereas the pieces `(0,1)`
and `(1,infinity)` have positive transformed coefficients. Thus coalescing
can reduce future reuse even while reducing today's guards. This example
does not claim that production discovery generated those fine pieces, or
that coalescence caused the observed Gluza history.

Refining only at already stored rational knots would preserve a finite set
of possible guards, but new seam roots, sample-zero coefficients and guard
budgets can still make replay fail. The actual retained knots here do not
include the new separators 11 and 19 from the fresh four-root cover. No
derived-cover policy, eviction/reset policy or weaker proof is introduced.

## Validation and provenance

Twenty-four focused tests and independent three-cell smoke verification pass.
The first unoptimized diagnostic exhausted its 180-second limit after only
the first two source events; it did not establish the final fallback. A
second build changes only the Rust optimization level to two, without LTO
or a backend rebuild, and reaches the definitive event. Both start from
fresh copies of the same 24,406 completed operations; no research factors
are injected. CPU 8 and a 50-GiB ceiling bound the original workers.

Artifacts are `artifacts/research/gluza33-witness-route-trace-01/` and `-02/`.
The optimized trace binary is
`b8d082556f788815c61d12291361027db200edb4a96605a6966cb3f64e7f4005`;
the definitive trace is
`37a1e67f92cba91223dd6744d07193832a558c3512a42536d8f7c9a2aada39b2`.
The separately optional Wang backend is unchanged and remains experimental.
The production dependency, completion count and benchmark ratios are unchanged.

## Four-slot original ablation

The same frozen optimized diagnostic binary repeats the original problem with
only `local_descartes_interval_template_limit=4` changed, starting again from
the untouched 24,406-operation snapshot. At `[2,1/4]` the old covers fail as
before, but fresh discovery now admits the third permanent template:
`(0,1/2),(1/2,11),(11,19),(19,infinity)`, with 97 guards and 109,726 terms.
The former discriminant is bypassed. Native processing subsequently completes
at least one new pair resultant, so the run moves beyond that single witness.

The worker nevertheless times out after **180.078399 s**, with **609,392 KiB**
peak RSS (180.514 s total campaign). There is no complete result or verification.
It retains **1,094 new operations**: 998 factors, 33 discriminants and 63
resultants, for **25,500 total records**. All inherited files, operation keys
and output hashes pass the final metadata audit; this is not an independent
algebraic replay of every checkpoint. The first new resultant takes 34.082845 s.
The final pending operation is a factor, key
`92a09155bfc5be6acccdce3af49ab6ba87d6042234d8f5799f711bbc2087833a`;
its capture alone does not establish that this call is a long-running bottleneck.
The final capture hash is
`6c07529df9fa481220f3de903d79c687f532d88aa773cf112a4fa718b69a0aac`.
The final audit hash is
`538dd0f8436d597fbe3ecee715635b101fcae2f573492a345fff9146e00082ca`.
Artifacts are `artifacts/research/gluza33-interval-stock4-01/`. This is a warm,
instrumented configuration ablation, not a release benchmark or solved-time
comparison. All interior roots and pair-order obligations remain required.
