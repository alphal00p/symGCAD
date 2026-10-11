# Positive Gluza33: the next algebra gate

Research follow-up, 11 October 2026. This investigation does not add an
original-problem completion. It preserves the original strict positive domain
and every interior zero of the original polynomial.

## The two observed obstacles are different

The two-hour, 50-GiB original attempt used the `x3` scale anchor. Its final
captured operation was a degree-ten discriminant with 2,209 terms and three
surviving parameters. Zero new completed algebra records were produced after
the inherited warm cache. Increasing memory did not resolve that frontier.
The separate packed-resultant experiment was interrupted after 2,580.9 seconds;
that is not a completed measurement or a two-hour timeout. The earlier
[campaign report](GLUZA33_AFFINE_ORDER_CONTINUATION.md) records both outcomes.

The legal `x0`-anchor order reached a different obstacle: factorization of a
224-term polynomial in `[x2,x1,x7]`, with degrees `[9,9,6]`. Its complete map
also contains the inactive variables `[x4,x6,x3,x5]`. The earlier
[square-free diagnostic](MULTIVARIATE_SQF_DIAGNOSTIC.md) reconstructed its full
signed product in under a millisecond after 6.6 ms of native decomposition:

```text
p = (1+x2+x7) * q,
degree(q) = [8,9,5], 158 terms.
```

This smaller frontier is the priority for the present investigation. Avoiding
an irreducible-factor call is useful only if the ensuing exact projection and
ordinary factoring of its guards remain manageable.

## Completed inexpensive measurements

These are separate, fresh, one-core algebra processes, capped at 4 GiB, outside
original-corpus benchmark timings. The frozen operation profiler has SHA-256
`cf4148ef6ea10d14f99055586026a0a5313802ba8f8e0c3f98305622426158f8`.

| Trial | Outcome |
| --- | --- |
| Native factorization, automatic start | 30-second timeout |
| Native factorization, univariate start | 30-second timeout |
| Native factorization, bivariate start | 30-second timeout |
| All six permutations of the three active coordinates | Six 10-second timeouts |
| Exact `Disc_x7(q)`, integer Ducos | **0.149522 s algebra**, 0.162245 s process, 7,636 KiB peak RSS |
| Gradient-rank diagnostic for `q` | Exact rank three; no reduction to fewer constant rational linear coordinates |

The discriminant has 1,341 terms and maximum coefficient height 117 bits.
The exact rational calculation, including division by the leading coefficient
with zero remainder, completed. A binary-segmentation probe correctly declined
this small degree because its degree product is 20, below its default admission
threshold of 60; that decline performed no resultant algebra.

Variable-map permutations do not necessarily change the backend's internally
selected main variable, so these six failures do not exhaust factorization
orderings. Likewise, a full-rank gradient matrix rules out a reduction in the
number of linear coordinates; it does not rule out useful invertible shears.

The quintic's exact fiber at `(x2,x1)=(1,1)` has degree three; at `(2,2)` it
has degree four. The fibers at `(2,3)` and `(3,2)` retain degree five and have
reducible univariate factor inventories. These facts explain why a sole
all-one, degree-preserving irreducibility scout cannot certify this polynomial.
They motivate testing generic small-integer translation preconditioning, with
an exact inverse and full source-product check. They do not identify the
internal phase responsible for a historical timeout.

All inputs, process records and diagnostics are retained beneath
`artifacts/research/gluza33-next-01/`. The derived quintic discriminant capture
has SHA-256
`b3fe7c9adb31be97261668852bd1a8a8b90c8cb0993f6590f38cd7967cb73beb`.
Its expression is copied from the authenticated native SQF proposal, whose
digest is bound to the earlier successful signed-product report. Metadata
preparation does not perform polynomial algebra.

## A cheap certificate also exists at the larger frontier's saved prefix

The earlier `x3`-anchor progress snapshot records prefix
`(x2,x1,x7)=(1/2,1,1)`. At that prefix, the captured degree-ten polynomial
has exactly two positive roots. Exact Sturm counting and native isolation
agree; its nonzero coefficient signs have exactly two Descartes variations.
The existing saturated-witness rule applies at this particular point.

Replaying the existing research helper with every coefficient and separator
guard retained and factored took **0.224700 s process time**, 13,064 KiB peak
RSS. Discovery took 0.006517 s and the twelve guard factorizations together
0.048049 s. The exact separator is `x4=10`; the signs at zero, ten and positive
infinity are `(+,-,+)`. The extra separator guard has 584 terms. No bound on
the subsequent lower-dimensional projection has been established.

Over any connected parent cell where these exact coefficient and evaluation
signs remain strict, the intermediate value theorem gives one root in each
of `(0,10)` and `(10,infinity)`. Descartes' upper bound, counting
multiplicity, is two; hence these are the only positive roots and both are
simple. Keeping all guard obligations establishes the required continuous,
separated sections there. It does not remove any positive interior root or
the parent-cell boundaries where this certificate ceases to hold.

The production saturated-witness stock currently permits two templates per
polynomial for the Builder's entire lifetime. The saved progress records
35 budget fallbacks, 203 admitted templates and 628 reuses. Exhausting the
two templates can force an expensive ordinary discriminant when neither
template works at a new prefix. This is a **candidate explanation**, not an
identified cause of this captured call: the progress snapshot is stale and
does not bind that request to its exact contemporaneous prefix or stock.

A useful next original-order experiment should capture the pending
polynomial's actual prefix, the attempted templates and decline reasons.
If template exhaustion is confirmed, compare a finitely bounded larger stock
or cost-aware preselection, retaining every supplied certificate and replaying
every guard. The existing separate interval-template budget is not the same
setting. Increasing a cap without identifying the relevant decline would be
uninformative. Evidence is in `large-discriminant-fiber.json` and
`large-witness/report.json` beneath the same research directory.

## Full-guard experiment

The research helper
[`profile_support_projection`](../examples/profile_support_projection.rs)
repeats the signed SQF reconstruction and retains **all** contents, leading
coefficients, discriminants and distinct-pair resultants for its proposed
blocks. It saves each completed signed guard before optional ordinary
factorization. A zero discriminant or pair resultant is retained and causes an
explicit failure requiring rebasing; it never becomes an omitted obligation.
Several lower projection levels can be requested under the same killable
deadline. All roots and same-sign seams remain mathematically present.

This is a projection of the captured source's block subset. The contemporaneous
local stack's companion cuts are absent from that single operation capture.
Consequently even a completed gate cannot certify that entire stack or the
original eight-variable decomposition. It also does not establish a global
production policy for overlapping composite blocks. The immutable basis,
cross-GCD, content, nullification and proof-transport obligations remain those
in [the support-basis assessment](GENERIC_SUPPORT_AND_AFFINE_GATES.md).

An optional, separate `--factor-translation k` experiment applies the algebra
automorphism `x_j -> x_j+k*(j+1)` on active coordinates, checks its exact inverse,
factors once, translates every factor back, and reconstructs the full signed
original source. This changes neither original CAD coordinates nor domain
assumptions. No case identifier, polynomial hash or graph-specific constant
selects a mathematical rule. The trial menu and all resource limits are finite.

Both experiments completed. All thirteen helper/module tests passed: three
new tests exercise repeated seams and signed reconstruction, zero pair
collisions with content preservation, and inverse translation with rational
coefficients and inactive coordinates; ten tests come from the reused parser
and operation profiler.

| Bounded experiment | Process time | Peak RSS | Outcome |
| --- | ---: | ---: | --- |
| Full subset projection, two levels, all guards factored | **3.692678 s** | 14,144 KiB | Both levels complete |
| Internal translation `(1,2,3)`, native factorization, exact inverse replay | **1.698483 s** | 12,600 KiB | Full signed source reconstructed |
| Fresh unshifted native control, matching backend/build profile/CPU | **30.023801 s** | 12,264 KiB | Timeout |

The projection trial used `--factor-guards --levels 2 --seconds 120 --cpu 7`.
The first level projects `x7`: two source blocks produce four raw guards,
whose ordinary factorizations produce fifteen unique nonconstant factors.
All guard algebra took 0.672979 s and their factoring 0.153219 s. The second
level projects `x1`: fifteen factors produce 117 raw guards, all computed and
factored, taking 0.102382 s and 0.269561 s respectively. The largest first-level
guard has 1,341 terms; the largest second-level guard has 81 terms and the
maximum resulting degree in `x2` is 92. The remaining process time includes
parsing, source replay, formatting and durable progress records. No lifting
or missing companion-cut projection is included.

The translation trial used `--factor-translation 1 --seconds 30 --cpu 8`.
The full original source is changed internally by
`x2 -> x2+1`, `x1 -> x1+2`, `x7 -> x7+3`. Forward substitution and exact inverse
checking took 0.053634 s; the translated source has 345 terms. Native
factorization took **1.632485 s**. Translating every factor back and checking
the full signed original product gave three multiplicity-one factors:

| Restored factor | Terms | Degrees in `[x2,x1,x7]` |
| --- | ---: | --- |
| `1+x2+x7` | 3 | `[1,0,1]` |
| Linear-in-`x7` factor | 10 | `[4,3,1]` |
| Remaining factor | 50 | `[4,6,4]` |

All three retain the full seven-variable map, including four inactive entries.
The report binds capture
`162d5b9b98efeed85cadf3f375ad914fa902682a58764d927505927e66100646`
and operand-array digest
`692447d2758a0dc4af4c3cb0ef38f2a17deb480ab1589f00a1d2e40a9d8421a1`.
Thus this is the exact 224-term historical obstruction, not a specialized
fiber or different problem. The matching unshifted control was rebuilt from
the existing operation profiler using the same freshly compiled backend and
development profile, then ran on the same CPU8 with the same 4-GiB/30-second
limits. It timed out. The old profiler's possibly different patch state is
therefore not needed to establish this contrast.

These are single-run operand experiments. No end-to-end speed ratio or Gluza33
completion follows. The result justifies prioritizing **generic bounded
affine preconditioning of hard factorization** ahead of the more invasive
composite-basis extension. A production policy should select a finite shift
menu using measured operand features, include preparation and replay costs,
and benchmark other captured hard and easy operands. It must preserve signed
scalar content, all multiplicities and the original variable map. An internal
polynomial-ring automorphism does not move or remove any original boundary
after exact inverse reconstruction. It should not become example-ID dispatch
or a retry loop following unbounded native calls.

The helper was built against the existing isolated positive-axis research tree
with the two optional positive-root/unit-Taylor-shift patches. Its factor
implementation is byte-identical to the ordinary dependency checkout:
`d5eaea5bc76e04aa29a0f481f94ba43580230d314fb0b182faebd9fe469e3fa1`.
The matching control uses that same isolated backend. These optional root
patches were not added to the production dependency setup by this investigation.
The clean build required 10m56s for tests plus 8.34s for the helper; build time
is outside all algebra timings. Earlier bounded build and stale-incremental
link failures are retained separately. A mismatched input hash was also
rejected before any worker launch; it is not an algebra trial.

| Evidence beneath `artifacts/research/gluza33-next-01/` | SHA-256 |
| --- | --- |
| `bin/profile_support_projection` | `44838d13593fcdea4ca10e7fc2c55536c4b8b4b877635c0c97e2f2c69155f801` |
| Helper source | `b05b9e83b5ef1b0ea0bd14da0d444d490b3781a00c03af8cb9025754c10233d9` |
| `projection-02/worker.json` | `13917f104ce73c06ab6f9e5948a9e1def216c5c44543ad024fb54c1e899243aa` |
| `projection-02/process.json` | `30595370a5bd1be592162feac21624aa3afdcb0edda46e967d85b1b4794767b3` |
| `translation-01/worker.json` | `468f7d11f8b187bb51b4f8087f90e9bbe45bcc0386176601ed35c5fc32772cd9` |
| `translation-01/process.json` | `1834561d31f4f7ab40b0b3f97b96462f7a425ab68ac61452883c318bac8c40ba` |
| `bin/profile_operation_control` | `077221caeec2633cd7bbc44800a85dc22225f2a8461bfb42e03bbb68074e11a3` |
| `large-witness/report.json` | `b51780bdf5feb3fac8a67fe808b86ee6097993eb357548ef044af8ee0285d6a9` |

No production source, dependency source or operation cache was modified. All
bounded workers have stopped. Original coverage remains unchanged.
