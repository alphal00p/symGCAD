# Affine preconditioning can hide cheap native structure

11 October 2026. A matched captured-operand control identifies a substantial
regression in the optional integer-translation policy. **This is an algebra
diagnostic, not a completed Gluza33 solve or a benchmark speed ratio.** No
production change or operation-cache injection was made by these probes.

## Matched result

The original operand has 1,283 terms, degrees `[17,13,13]`, and full variable
map `[x2,x1,x7,x4,x6,x3,x5]`. Both fresh workers used the same frozen profiler,
CPU 12, 4 GiB, a 30-second limit, and the same experimental backend with Wang
tracing enabled. Only `--mode policy` versus `--mode native` changed. The helper
excludes the production dispatcher's earlier primitive-linear/sum shortcuts.

| Captured operand | Translation → shear → native policy | Native bypass |
|---|---:|---:|
| Factor operation | incomplete | 0.042986 s |
| Independent full signed replay | not reached | 0.003187 s |
| Worker wall time | timeout, 30.039220 s | 0.062342 s |
| Peak RSS | 14,040 KiB | 12,872 KiB |

The policy admitted `x2→x2+1, x1→x1+2, x7→x7+3`, reducing its all-one fiber
degree-drop count from one to zero. It stayed inside that translation attempt;
the later shear policy was not reached. The trace reports an internal
330-term operand, eight LC terms, and order `[x1,x2,x7]`. It records 1,237
sampling entries and 3,709/3,708 bivariate factor starts/ends, with no Wang
candidate or assignment. Every admission says `small=false`; the log does not
identify which gate failed or export the internal polynomial's exact identity.
Thus it establishes repeated fallback work, not one expensive image factor.

Native returned five factors with term counts `[2,16,63,3,3]` and multiplicities
`[3,1,1,2,3]`. Every factor retained the full map. Their **complete signed exact
product** reconstructs the original source. No Wang trace entry was emitted:
this success comes before that path and must not be credited to the experimental
small-LC patch. One trial does not establish reproducible timing ratios, but the
completed exact factorization supplies a concrete control for this policy failure.

## Square-free dispatch: sound, but insufficient by itself

A separate frozen Symbolica diagnostic decomposed the original source in
0.036021 s and checked its signed product in 0.004170 s. Its square-free/separable
blocks have term counts `[2,228,3,3]` and multiplicities `[3,1,2,3]`. This is the
public Symbolica decomposition, including its separable-content extraction;
blocks need not be irreducible.

The existing translation admission was then evaluated on those exact exported
blocks, without factoring them:

| Block | Degrees in `[x2,x1,x7]` | Translation admission |
|---|---|---|
| 2 terms, multiplicity 3 | `[1,0,0]` | Declined |
| 228 terms, multiplicity 1 | `[9,8,5]` | Same shift admitted |
| 3 terms, multiplicity 2 | `[1,1,1]` | Declined |
| 3 terms, multiplicity 3 | `[1,1,2]` | Declined |

Admission-only checks of the independently reconstructed irreducible factors
also retain the same drop in the 63-term factor; the 16-term factor declines.
Consequently **square-free decomposition before preconditioning alone does not
remove this bad admission**. The drop cannot be attributed solely to an easy
repeated factor.

An exact follow-up exported that 228-term block after the same translation,
checked the inverse in Symbolica, and obtained a 330-term polynomial. On this
bound operand, both automatic and explicit univariate native factorization
timed out at 30 seconds (13,436 and 12,748 KiB). Automatic mode recorded 1,318
sampling entries and 3,952/3,951 bivariate factor starts/ends; univariate mode
emitted no Wang trace. Neither returned a factorization. This independently
reproduces the problematic shape and retry pattern, but the earlier trace did
not export its component, so byte identity to that internal operand is unproved.
The initial automatic order has box density `330/(9*10) < 4`, which explains
the small-LC density gate on this newly exported operand.

A main-axis-only degree-drop gate does not separate the old win from this
regression. Inventory of the existing exact monomial exports gives initial LC
term counts `[2,3,16]` for the historical 158-term block and `[4,6,19]` for the
new 228-term block. Both initially prefer `x2`; their all-one drops occur in
`x7` and `x1`, respectively. Both are below the 256-term sparse-reordering gate.
These are support/admission facts, not traced native execution paths. Translation
changes the new block's LC counts to `[9,8,27]`, making `x1` preferred. A diagonal
nonzero rational scaling would preserve the support and this ranking while
reaching a regular evaluation point. The separate
[diagonal-scaling follow-up](DIAGONAL_FACTORIZATION_RESEARCH.md) tests this
mechanism and records its default-off implementation, exact checks and remaining
overhead on the cheap native control.

There is still a principled architectural improvement to investigate: extract
exact separable/square-free structure before applying expensive heuristics to
each remaining block. Preserve the signed scalar, full maps and block
multiplicities; multiply nested multiplicities with overflow checks and replay
the complete original product. Components require complete factoring, not a
claim that a square-free block is irreducible. Use a staged dispatcher so a
one-block unchanged decomposition does not recurse indefinitely. Bound block
count and cumulative translation/reconstruction work across the original call,
and retain its killable deadline: GCD/SQF itself is not bounded by shape alone.
Calling ordinary `factor()` on every block may repeat SQF work, so overhead must
be measured rather than assumed negligible.

The historical 224-term success already splits into a three-term component and
a 158-term block. That block independently retained the translation benefit
(about 1.67 s against a native 30-second timeout), so componentization can retain
the old useful admission. Those measurements are related controls, not evidence
that every block benefits. See [the original policy measurements](AFFINE_FACTORIZATION_POLICY.md).

For this frontier, preserve a cheap native opportunity before preconditioning.
Bounded campaign-level native/affine portfolios can already share exactly
validated factor caches. A finer native-first operation trial would need a
killable operation protocol, not an unreliable timer around a blocking call.
Alternatively, instrument the existing native fast paths and require a more
relevant block-level heuristic than any all-one axis drop. Test against both
historical successes and unrelated cheap repeated products before changing
dispatch. The simple SQF-first fix is not supported by this inventory and was
not implemented. A fixture identifier, graph name, or captured hash must never
select the production policy.

## Non-divisors are a separate research direction

All three raw leading coefficients were independently reconstructed. The
cheapest raw main axis already has a passing finite Wang image; parity prevents
pairwise-coprime images for the other two axes. These raw facts do not describe
the translated internal 330-term operand.

Wang's §3 Algorithm N permits shared image primes by constructing ordered
separating divisors, followed by reverse-order assignment of the **full** LC
images and exact content handling. That is a defensible future extension, but
simply dropping coprimality checks is insufficient. It is not the diagnosed
cause of this native/policy contrast. [Wang's author-posted paper, pp.1218–1219](https://www.researchgate.net/publication/242921585_An_improved_multivariate_polynomial_factoring_algorithm),
[version-pinned SymPy implementation](https://github.com/sympy/sympy/blob/1.14.0/sympy/polys/factortools.py#L683-L775).

## Reproduction and evidence

Use `profile_affine_factorization CAPTURE --capture-sha256 HASH --mode MODE
--seconds 30 --cpu 12 --output FRESH_DIR`, with `MODE=policy` or `native` and
both `SYMBOLICA_RESEARCH_SMALL_LC_WANG=1` and
`SYMBOLICA_RESEARCH_WANG_TRACE=1`. The frozen executable links the experimental
backend described in [GLUZA33_WANG_PREFLIGHT.md](GLUZA33_WANG_PREFLIGHT.md).

Ignored evidence is under `artifacts/research/wang-new-frontier-01/`: full traces,
process reports, factor exports, raw LC replays, SQF inventory, derived-component
bindings, admission-only reports and the translated-block controls. Capture, executable and patch hashes were
unchanged after both factor trials. The SQF supervisor first failed under the
small address-space cap before producing a worker report; the retained successful
rerun used the direct worker under an external 10-second/512-MiB bound. No failed
or skipped probe is counted as a factorization.

| Binding | SHA-256 |
|---|---|
| Original capture | `22f92bf277215836d66a4ef034e38be593eb87e34ec73bb7fbb0254234985cf9` |
| Frozen matched factor profiler | `a53b76e708fe329fae8aa735ac62344f1acce0e15ebd03369ee4d01f9631232f` |
| Experimental backend patch | `e50211910edfa513accfd7cec75718e1a464538bb5fbcd60a10d9898b9c36ce1` |
| Native exact factor export | `da010ac7bbe4c056f8606f34ba8c072ee17ac9139455cf0c199ea65faa52e387` |
| Exact translated 228-term block capture | `d52770f5e117f88f0cc586976bc2471d07ed487f3cb52a70fcac96d91d3541c9` |
| Frozen auto/univariate operation profiler | `c07bc81be030f88444e65882070b99826532b0f3ccfe43588cf4f7f06864bfab` |

The backend revision is `75f8350094b90254ee71dc2a391fde0d14b0204a`; the frozen
translation module is `b5a75d83c1f2beaf84523fbbf0f9bd4461c7caf34ff9161ad5c804d3bd656bd7`.
Original domains, kinematic chambers and all interior zero sheets are untouched.
