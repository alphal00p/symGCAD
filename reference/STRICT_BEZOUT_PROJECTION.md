# Strict Bézout certificates for positive pair projection

The local engine can now omit a pair resultant when an exact polynomial
combination proves that the two cuts have no common positive root. This is
an opt-in extension of the existing positive-ray projection rule. It does
not replace the individual leading-coefficient, discriminant, endpoint or
root-ordering obligations.

## Certificate and domain

For the two canonical cut polynomials `p,q`, a proposal supplies

```text
p*L + q*M = k*H^2,       k > 0.
```

Version 1 accepts a nonzero `H` only when all its nonzero coefficients have
the same sign and every coordinate in its support has an original literal
strict positive-coordinate constraint. Consequently `H` is nonzero on the
positive domain. At a hypothetical common zero of `p,q`, the left side
would vanish and the right side would be strictly positive, a contradiction.
The multipliers `L,M` need no sign assumption. Neither they nor `H` may
depend on a later variable than the pair's current projection axis.

The current coordinate's positivity comes from the existing positive-ray
rule. For every earlier coordinate in `H`, the certificate contributes its
original positive constraint as an ancestor guard. The exact positive
sample and guard nonvanishing then preserve positivity throughout the
connected ancestor cell. No target-polynomial sign is used as a premise.
The full-real projection path does not use these certificates.

The strict premise matters: `H^2 >= 0` alone cannot exclude a common zero.
For example, `p=z^2-a`, `q=p+(a-1)^2` satisfy `q-p=(a-1)^2`, but both
vanish at the positive point `a=z=1`. The independent integration tests
inject this exact identity into a proof and require kernel rejection.

## Proposals and independent checking

The TOML field `solver.local_strict_bezout_hints` contains a list of records
with `schema_version = 1`, two `polynomials`, two `multipliers`, a `square`
and a positive rational `multiplier`. These records are discovery proposals,
not additional input assumptions. Invalid, out-of-scope, unmatched or
resource-limited proposals leave the ordinary pair projection available.

Production preparation checks exact canonical factors, support, coordinate
premises and the expanded polynomial identity using Symbolica. The verifier
starts with an empty certificate registry, repeats those checks, reconstructs
the entire positive-ray witness and checks the exhaustive lifting proof.
It does not trust the proposal acceptance counters or a producer cache.
Historical evidence without this optional field retains its previous
semantics. Input proposals need not remain attached to independently
checkable result evidence.

Preparation is bounded by 128 proposals, 4 MiB of expression text, 64 KiB
per expression, 64 variables, 4,096 terms per polynomial, total degree 32,
4,096-bit numerator/denominator sizes and 65,536 candidate term products
per identity multiplication. Parsing and algebra run inside the existing
killable worker. Deadline exits remain incomplete. The preparation time
is included in reported projection time.

The report separates accepted/rejected preparation proposals, resultants
omitted and ordinary resultants still computed. The latter counter,
`strict_bezout_resultant_fallbacks`, includes pairs with no matching proposal;
it is not a count of false identities.

## Graph-derived proposals

For `F=A*z+B` and a graph guide `U=C*z+D`, an exact strict Rayleigh identity

```text
C*B-D*A = k*H^2
```

gives proposals for canonical factors `p` of `A` and `q` of `B` by choosing
`L=-D*(A/p)` and `M=C*(B/q)`. The Rust research helper
`examples/gluza_bezout_hints.rs` checks the divisions, factor reconstruction,
strict square and final identity. It explores a bounded lattice of
derivatives and zero endpoints, preserving the original mathematical input
and a legal lifting order. Uniform-coefficient factors are omitted from the
proposal inventory because existing positive-ray reasoning already proves
them rootless.

This research generator is not trusted by production. Its discovery time
must be reported separately or included in any end-to-end optimization
comparison. An exact proposal inventory by itself is not a decomposition
and earns no corpus completion credit.

## Validation

The focused checks cover exact common refinements against baseline and
ordinary local projection, JSON/TOML transport, false-proposal fallback,
coherent equivalent identities, incorrect polynomial/identity/pair/support
evidence, current and ancestor positivity, forbidden later-axis support,
isolated ancestor-guard deletion, retained individual delineability guards
and historical evidence. The focused bundle passed 217 library/integration
tests and two generator tests. Logs are
`artifacts/strict-bezout-focused-tests-{01,02}.log`; the second log corrects
one test-only expectation that had mistakenly rejected an equivalent exact
identity in the first run.

The subsequent full `cargo test --all-targets` run passed 529 tests across
76 targets, with no failures or ignored tests. Documentation tests and
`cargo fmt --all --check` also passed. The full logs are
`artifacts/strict-bezout-all-targets-tests-01.log` and
`artifacts/strict-bezout-doc-tests-01.log`.
