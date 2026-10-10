# A377 degree-32 discriminant: complete signed identity verification

The captured A377 discriminant is now independently verified as an exact
polynomial identity. A fresh grid process finished on **2026-10-10 at
13:58:06 UTC**, checking all **4,773 required valid points**. This verifies
one algebraic operation. It does not provide an A377 decomposition, root
inventory, cache admission or permission to omit a zero locus; the original
completion count remains **303/331**.

## Identity and complete grid

The input has full map `[s,t,x0,x3,x5,x6,x8]`, with only `s,t` active,
degree 32 in `t` and 2,502 terms. The signed candidate `D(s)` has degree
4,058 and 3,261 terms. The adapter constructs

```text
q = derivative_t(p)
R_candidate = lc_t(p) * D_candidate
```

The discriminant sign is positive because `32*31/2 = 496` is even.
An independent bounded sparse-integer audit checked the complete derivative
and every coefficient of this signed product, without primitive
normalization. The derivative has 2,419 terms; `R_candidate` has degree
4,118 and 3,321 terms. The leading coefficient is a nonzero, 61-term
polynomial, not an assumed nonvanishing function throughout the domain.

The unknown true resultant has rectangular parameter-degree bound 5,292.
Its Sylvester weighted-degree bound is tighter:
`31*92 + 32*91 - 32*31 = 4772`. Both were independently recomputed from
the captured monomial support. The candidate also lies within this bound.
Consequently 4,773 distinct valid parameter values suffice. Candidate
degree alone was not used to shorten verification.

The frozen verifier enumerates distinct rational values, skips every value
where either input loses its eliminated-variable degree, and compares each
candidate value with an ordinary exact numeric Ducos resultant. It reports
success only after the entire required grid. The completed report records
`verified:true`, 4,773 checked points and degree bounds
`[4772,0,0,0,0,0,0]`. Its process exited zero without truncated output.
This proves the global resultant identity. Cancellation of the nonzero
polynomial `lc_t(p)` then proves the signed discriminant identity, including
its specializations where that leading coefficient vanishes. No such
boundary is discarded.

## Measurements and immutable bindings

The fresh verification used CPU 9, one worker, 4 GiB and a 3,600-second
limit: **2,093.930808 seconds process wall**, **187,400 KiB peak RSS**;
the exact-grid stage reports 2,091.859583 seconds. All source, input,
candidate and executable hashes were checked again after completion.
The earlier partial grid, stopped at its published count 4,608, remains
preserved and contributed no points to this run.

The candidate was computed separately in 1,538.878302 seconds with
1,364,132 KiB peak RSS. These are individual research stages with different
limits, not matched solver benchmarks or a completed-original timing.

Under `artifacts/research/a377-degree32-verification-03/`:

| Artifact | SHA-256 |
|---|---|
| `grid-verification.json` | `580bd7e8eb9feed223453f1b681a39bbf7b55fcf880a84a8d50ecc58b130b4d2` |
| `grid-verification.process.json` | `75495973c99b4089c18b94225bcf55d76d243b667ecee37ac621e21fe7d8d613` |
| `independent-adapter-audit.json` | `0242e5e278b773a8b7b654c445c5b6ce8c914f523b7b0df4192c349ab3ef8d5c` |
| `independent-final-audit.json` | `a181e60a1e2a7ec26399839ac13f536f82e3261128981e644a8ab813660bcddc` |

Inputs remain under `artifacts/research/a377-degree32-segmentation-02/`:

| Artifact | SHA-256 |
|---|---|
| `capture.json` | `bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75` |
| `discriminant.json` | `b798275222aa98597a4261224e714272d0d21dafcd805937e57b7397d337b2c1` |
| `grid-input/pair-capture.json` | `cabb824f41f8a5acb35d19821c5a3c0de01d6c04f117118d877a85b62cd192c8` |
| `grid-input/resultant-candidate.json` | `5490aa8e794f5480de3148be56e77561f59d0cdcf895d58aec835fc260833b08` |
| Frozen `profile_kronecker_resultant` | `44428e764331e44d295eb4b8dae17201d39fe7b1a8573a240a499b1fea95ec09` |

Reproduction uses the saved original files with
[`prepare_discriminant_grid`](../examples/prepare_discriminant_grid.rs)
and [`profile_kronecker_resultant`](../examples/profile_kronecker_resultant.rs)
in `--verify-candidate` mode; the launch manifest records the exact frozen
inputs and limits. Large operands remain ignored artifacts. The separate
[content and basis measurements](A377_D32_CONTENT_AND_BASIS_LIMITS.md)
still apply: identity verification does not make the candidate fit the
current production resource envelope or finish square-free decomposition.
