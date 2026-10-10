# Deterministic CRT feasibility and a one-prime diagnostic

This is research-only. The production resultant/discriminant dispatch, operation
cache, proof schema and verifier are unchanged. A finite-field image is not a
rational resultant, a discriminant, or CAD coverage.

## Public API and exact completion criterion

The pinned Symbolica `poly/resultant.rs:628` implementation of `resultant_crt`
can return either at a deterministic coefficient bound or after stabilization
and two fresh checks. The latter is explicitly Monte Carlo. Its public APIs
suffice to write a deterministic wrapper without modifying the dependency:
`map_coeff`, `to_univariate(...).resultant(...)` over `Zp`, and integer-polynomial
`chinese_remainder` (`poly/polynomial.rs:7256`). The private normalization and
prime-loop helpers would be replaced by bounded caller code.

For nonzero rational inputs, construct positive rational scalars and signed
primitive integer polynomials

\[
p=\alpha P,\quad q=\beta Q,\qquad
m=\deg_x p,\ n=\deg_x q.
\]

Replay both identities coefficient by coefficient in the complete original
variable map. Positive denominator LCM and positive integer content leave the
signs in `P,Q`. Then

\[
\operatorname{Res}_x(p,q)=\alpha^n\beta^m\operatorname{Res}_x(P,Q),\qquad
B=\|P\|_1^n\|Q\|_1^m.
\]

Expanding the Sylvester determinant and bounding the unsigned sum by the product
of row 1-norms proves that every integer resultant coefficient has magnitude at
most `B`. CRT over distinct accepted primes is conclusive only once its modulus
`M > 2B`. Symbolica's integer CRT uses centered representatives. Missing monomials
must be merged as zero coefficients; with that complete union and `M > 2B`, an
unseen nonzero coefficient cannot have vanished modulo every accepted prime.
Stabilization, several matching primes, and a zero image are not substitutes for
this inequality.

Reduce the **integer** associates before each exact image. Skip primes that
reduce either eliminated-axis degree. Parameter support or resultant degree may
drop modulo a valid prime, and a zero resultant image is valid: these must not
be discarded merely because they differ from earlier images. Preserve the
ordered full map, restore mapless native constants/zero to that map, and check
absence of the eliminated variable and all parameter degree bounds. No
parameter specialization or domain assumption is involved.

For a discriminant, derive `q=p'` exactly and finally apply

\[
\operatorname{Disc}_x(p)=(-1)^{m(m-1)/2}
\operatorname{Res}_x(p,p')/\operatorname{lc}_x(p).
\]

The quotient must have zero polynomial remainder; retain its exact sign and
rational scale. This polynomial identity remains valid on leading-coefficient
and repeated-root loci. No boundary can be removed by treating the division as
a nonzero-domain assumption. The existing production adapter already enforces
these obligations.

## Bounded one-prime helper

`examples/profile_resultant_prime.rs` accepts only an authenticated discriminant
capture and computes one image of `Res(P,Q)` for the independently normalized
pair `p,p'`. It performs no CRT reconstruction or leading-coefficient division.
The accompanying positive scales describe the original rational inputs; the
saved image is not mislabeled as their modular reduction if a scale denominator
is divisible by the prime.

The helper reuses only the research profiler's bounded capture reader and
literal sparse parser. Input limits include an 8 MiB capture, 128 axes, three
active remaining parameters, eliminated degree 2–64, other degrees at most
4096, 8192 combined input terms, 4096-bit input coefficient parts, 16384-bit
LCM and 32768-bit cleared coefficients. The norm-bound estimate is capped at
one million bits before exponentiation. Output permits at most one million
terms and 32 MiB of exact expression text. These caps do not bound native
intermediate allocations: the worker is externally limited to **300 seconds
and 4 GiB**.

The deterministic prime stock starts above `2^30` and allows at most 16 skipped
degree-dropping primes before the first complete image. `PrimeIteratorU64`
uses `Integer::is_prime(0)`; the pinned numerica implementation dispatches `u64`
inputs to fixed-witness deterministic Miller–Rabin tests
(`domains/integer.rs:1665`, `domains/finite_field.rs:3090`).

Atomic progress records identify normalization, modularization, and entry into
the finite-field resultant, with input hashes, scales, degree/term counts,
prime and bound metadata. The process outcome is persisted **before** fallible
post-run hashes or worker-report reads; a separate audit binds any completed
image. Output directories must be fresh. No saved image or diagnostic is added
to production caches or treated as proof authority.

Four focused regressions compare modular images with signed deterministic
integer-Ducos resultants: negative rational scales and inactive axes; rejected
elimination-degree drops versus valid parameter-support drops; repeated-factor
zero; and authenticated sparse input/cap failures. Together with ten reused
profiler/parser tests they pass (24 executions across the two example targets,
14 distinct tests). The final test/build log is
`artifacts/resultant-prime-helper-tests-03.log`, SHA-256
`c34a0e10ece9b27df072cfe1d0d8dee15e67502d76d2588180e612857620cfa5`.

## Measured operand and interpretation

The authentic Gluza33 capture has input SHA-256
`73189be967be55624321db0c39697b0d0d5cfeb8e8a49aa17869e21e477fb4b0`
and full-file SHA-256
`aab900ab332f90a1d0c9d32ec73cdd65b88d64b22a45d20e23ef77e33aa70289`.
Its original map is `[x2,x1,x7,x4,x6,x0,x5]`, with elimination axis `x4`.
The existing native CRT diagnostic timed out after **300.032155 seconds**,
111500 KiB RSS, with no result or internal prime telemetry
(`artifacts/research/gluza33-x4-discriminant-crt-01/final-audit.json`, SHA-256
`f90a485fb37def6fe4ea7703a2b602d3cb2e4ba844e16382b1024fab07212d5e`).
That alone does not identify the expensive stage or justify a deterministic
wrapper's performance.

The separate one-prime development build is frozen at
`artifacts/research/gluza33-one-prime-01/bin/profile_resultant_prime`, SHA-256
`0eabe8ace87875f36ff201c9919fe6cd35284f908578b622663c66b1cddc6eb9`.
It uses the same capture, CPU 2, and the stated worker limits. Exact preparation
found primitive scales `[1,2]`, degrees `[10,9]`, and term counts `[2209,1795]`.
The first accepted prime is `1073741827`; both degrees and term counts survive.
The primitive-resultant coefficient bound has 649 bits, so 22 accepted primes
at least `2^30` would suffice for the deterministic modulus criterion. This
count is not a runtime prediction.

A future deterministic wrapper should remain bounded and experimental until
complete prime-image costs are known. Static declines may choose an ordinary
backend before work starts; entered native errors/timeouts must not silently
restart a second expensive backend. Checkpointed partial reconstructions remain
proposals unless their image obligations are independently replayed. Fresh
recomputation through an exact deterministic wrapper can preserve the existing
proof schema, but no such production implementation is included here.

The first-image worker timed out after **300.020603 seconds**, 108512 KiB RSS,
with no image or completed worker result. The final checkpoint is
`finite_field_resultant` for prime `1073741827`: preparation took
0.018527 seconds and modularization 0.000448 seconds. This stage includes the
native call and its bounded result conversion/map/degree checks; it is not an
instruction-level profile. The input and executable hashes remained unchanged.
The raw process record was saved before the subsequent audit. Final artifacts
are `artifacts/research/gluza33-one-prime-01/{run/process.json,run/audit.json,
run/progress.json,final-audit.json}`.

This confirms that even the **first complete image** is unresolved at this
budget in this development build. It does not show which prime or stage the
older opaque CRT run reached, nor establish a release-build cost. Replacing
Monte Carlo termination with the deterministic modulus criterion is
mathematically feasible, but this measurement supplies no performance case
for deploying that wrapper. A faster exact image algorithm or further
instrumented evidence is needed before scheduling a full reconstruction.
