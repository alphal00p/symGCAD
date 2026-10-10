# Direct certification of a negative monotone inequality

The optional `solver.monotone_negative_target = true` mode applies to an
unsplit problem using the `monotone` engine. It solves the requested strict
inequality directly, avoiding a full sign arrangement whose positive cells
would subsequently be discarded. Commit `529e708` retains the historical
default and proof format through an optional, absent-by-default target field.

## Exact applicability and coverage

Choose an original normalized positive constraint `Q > 0` and set `F = -Q`.
Remove that constraint from the domain used to certify monotonicity. A peeled
variable must range over its complete positive ray: every remaining constraint
that depends on it must be a positive scalar multiple of that coordinate.
Peels form a legal suffix of the cylindrical order and respect variable groups.

For a linear polynomial `F(z) = a*z+c`, certify `a > 0` on the full remaining
domain. For a quadratic `F(z) = a*z^2+b*z+c`, certify `a > 0` and either `b > 0`
or the exact identity `b = 0`. These are existing exact domain certificates;
the removed inequality cannot supply a premise. Thus `F` is strictly
increasing and unbounded on `z > 0`, and

```text
F(z) < 0  iff  c < 0 and 0 < z < the unique positive root of F.
```

Apply the same argument to successive endpoints. The terminal base problem
contains every remaining original constraint and the strict negative endpoint
inequality. It has no requested split polynomial. Restore one lower interval
per peel in reverse order. A negative endpoint implies real-root index zero
for a linear polynomial, and index one for a quadratic with positive leading
coefficient. Rational samples and certified isolating intervals are generated
by the existing exact root facilities. Final cells carry the original unsplit
problem's empty split-sign annotation.

A zero or positive constant endpoint gives an empty fiber. In particular,
zero is never inserted into the exceptional set to conceal an unsolved region.
Nonconstant endpoint, coordinate, peeled-polynomial and base seams remain
explicit. The endpoint-negative base and the unique restored fibers establish
generic coverage of the original inequality; no claim is made to cover the
discarded positive side of a sign arrangement.

If positive homogeneous reduction was requested, apply it to the original
constrained problem first. Its independently checked scale transformation
then wraps the direct inequality proof. If no admissible peel is found, the
existing complete-arrangement/sign-filter path remains the fallback.

## Independent verification and resource behavior

Evidence records the original constraint index, exact normalized target,
orientation `-1`, selected sign `-1`, coefficient certificates, peeling steps
and complete terminal proof. Verification reconstructs the target-free domain
and exact restricted base, checks the step certificates without rediscovering
them heuristically, verifies the base, and compares every restored cell and
all boundary/exceptional metadata. Missing, duplicated, altered or extra cells
cannot acquire complete status. The verifier accepts historical arrangement
evidence with the target field absent using its original semantics.

Planning consumes the original time budget; the delegated base receives only
the remaining time, and reconstruction checks the cumulative deadline. A
resource-limited base remains incomplete. Hard worker limits still bound an
individual algebra operation and serialization. Requested aggressive merging
remains a separate compatibility view; raw cells and excluded seams persist.

## Validation

Ten direct-mode regressions and three option/dispatch regressions cover linear
and quadratic fibers, multiple peels, exact baseline common refinements,
constant and contradictory endpoints, fallback applicability, parameter
substitution, fixed orders/groups, local bases, original-domain homogeneity,
serialization, historical evidence, tampering and resource limits. The full
source passed 367 library/integration tests in 23 targets, two campaign-helper
tests, documentation tests and formatting checks. Historical 5-, 8- and
12-cell proofs independently replay with the frozen worker.

Build and replay evidence is retained under
`artifacts/hard-campaigns/negative-target-debug-01/`. The main executable SHA256
is `b5f3742b4b4faa68f0d02920ee43e033be4021e28bd38109ce4f817dd1c64ae2`.
This algorithmic validation alone does not add corpus completion credit;
individual hard-case results require separate full-proof verification.
