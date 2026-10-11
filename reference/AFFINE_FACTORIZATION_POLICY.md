# Bounded affine factor preconditioning

Follow-up to [the Gluza33 operand investigation](GLUZA33_NEXT_INVESTIGATION.md),
11 October 2026. This is an opt-in algebra optimization, enabled with
`[solver] affine_factorization = true`. It does not alter CAD coordinates or
remove any original interior boundary. The operand measurements below do not
constitute a new completed Gluza33 decomposition.

## Admission and exactness

The policy admits at most 2,048 terms, 2–4 active variables in a full map of at
most 64 variables, axis degree at most 32, total degree at most 48, and
4,096-bit input coefficients. It first tests every active variable's leading
coefficient at the all-one point. A zero value certifies a degree drop on that
fiber. At least one such drop is required.

For active coordinates indexed by `j`, try the finite shifts
`x_j -> x_j + k*(j+1)` with `k = 1, -1, 2`. Choose the first shift that reduces
the number of exact fiber degree drops. All active axes participate in this
score: fixing one drop while introducing another is not counted as an
improvement. This heuristic predicts useful evaluation points; it makes no
claim about the internal cause of a native factorization delay.

Before native factoring, Symbolica expands the translation under term, degree,
coefficient and cumulative work caps and checks the exact inverse identity.
There is one native factor call. Its complete signed product is checked,
every factor is translated back, and the original signed product is checked
again, retaining units, multiplicities and inactive map entries. The inverse
polynomial-ring automorphism preserves irreducibility. Pre-native resource
declines retain the ordinary path; failed identities and post-native resource
failures are errors and cannot start a second native factorization. The
caller's killable worker remains responsible for the native call's deadline.

The immutable option belongs to the normalized problem's algebra context;
default behavior remains unchanged. Projection, ordering and verification use
the same context. No graph or fixture identifier selects the policy.

## Matched operand measurements

All trials used one frozen profiler binary, CPU 8, fresh processes, a 4-GiB
limit and a 30-second cap. Native control means a direct Symbolica factor call,
not the full production dispatcher with its existing linear/sum shortcuts.
The timed policy operation includes discovery, substitution and both signed
reconstruction checks. Both modes also perform the same admission inspection
outside the timed operation to export diagnostic metadata.

| Operand | Admission | Policy operation | Native operation |
| --- | --- | ---: | ---: |
| Historical Gluza33 224-term capture | 1 degree drop → 0 | 1.691637 s | 30-second process timeout |
| Its 158-term square-free block | 1 degree drop → 0 | 1.669261 s | 30-second process timeout |
| Captured A377 resultant operand, factored separately | Declined | 0.394 ms | 0.347 ms |
| Captured A446 seven-term operand | Declined | 0.381 ms | 0.313 ms |
| Synthetic signed repeated factors | Admitted | 13.681 ms | 3.027 ms |
| Synthetic bivariate product | Admitted | 2.842 ms | 1.626 ms |
| Synthetic four-variable product | Admitted | 2.663 ms | 0.924 ms |
| Synthetic negative-shift selection | Admitted | 0.628 ms | 0.434 ms |
| Synthetic exhausted shift menu | Declined | 0.227 ms | 0.363 ms |
| Synthetic ordinary nondegenerate product | Declined | 1.875 ms | 1.704 ms |

The complete 224-term trial took **1.718989 s**, peaking at **12,288 KiB**;
the matched unshifted process timed out at **30.026306 s**, 11,956 KiB. Its
158-term subproblem took 1.678588 s and 11,776 KiB, against a 30.036021-second
timeout. These two successes are related operands, not independent families.
The restored 224-term factor inventory has 3, 10 and 50 terms, each with
multiplicity one; every factor retains the original seven-variable map.

Three additional admission-only trials correctly declined an oversized A446
capture and synthetic axis-degree/coefficient-size limits. Across 23 workers,
18 completed factorizations passed exact signed replay, three completed
admission-only diagnostics, and two native controls timed out. No skipped
factorization is counted as checked. All workers stopped.

The cheap synthetic regressions are why this policy remains **opt-in**. These
single-run measurements establish a useful escape from one observed algebraic
obstruction; they do not establish a corpus-wide runtime improvement.

## Reproduction and evidence

The example [`profile_affine_factorization`](../examples/profile_affine_factorization.rs)
accepts authenticated operation captures or its fixed generic synthetic menu,
and `--mode policy`, `--mode native`, or `--mode admission`. All trials and
their exact arguments are retained under
`artifacts/research/affine-factor-policy-01/`, particularly `run-trials.sh`,
`bin/frozen.sha256`, `audit.json`, and each trial's `worker.json`/`process.json`.
The A377 input was copied verbatim from a captured resultant operand; the
metadata-only conversion is bound in `inputs/a377-binding.json`.

The frozen profiler has SHA-256
`0f2dfd26e593d2a2bc567b74ec652fa16b74289ed9d2818ede01f198753e4cd9`;
the module source is
`b5a75d83c1f2beaf84523fbbf0f9bd4461c7caf34ff9161ad5c804d3bd656bd7`.
The historical input is unchanged at
`162d5b9b98efeed85cadf3f375ad914fa902682a58764d927505927e66100646`.

This operand profiler used the existing isolated development backend with the
optional positive-root and unit-Taylor-shift research patches. Both arms used
that exact same backend. Its factor implementation is identical to the normal
checkout (`d5eaea5bc76e04aa29a0f481f94ba43580230d314fb0b182faebd9fe469e3fa1`).
No new dependency patch was introduced. Dependency optimization was level two;
the symGCAD driver used the development profile. Production release campaigns
are separate measurements.

Five module tests, three solve/verify integration tests and fifteen profiler
tests passed (the profiler repeats the five module tests and includes ten
existing parser/algebra tests). They cover signed repeated factors, inactive
maps, negative shifts, menu exhaustion, resource declines, exact replay,
policy serialization, context isolation and preservation of a same-sign
interior singular seam. An initial incorrect test expectation about a
negative shift was corrected after the policy detected a new degree drop on
another axis; the policy itself did not change.

The subsequent [full production gate](COMBINED_FAST_VALIDATION.md) passed
622 tracked tests with zero failures and two ignored against the normal
unpatched Symbolica checkout. The [original Gluza33 continuation](AFFINE_GLUZA33_CONTINUATION.md)
passed the captured factor obstruction, saved further operations and reached
a different unresolved discriminant; it supplies no completion credit.
