# Exact guide audit for the remaining symbolic ZZ inputs

The negative linear guide reduction that solves the negative Gluza examples
does not apply globally to any of the 39 linear Feynman-parameter axes in the
27 unresolved symbolic ZZ originals screened here. This is an exact
counterexample result for this particular reduction, not an impossibility
result for their GCAD problems. No new original completion is claimed.

## Original data and exact checks

The upstream files in `DO_NOT_PUSH_REFERENCES/gcad/tests/ZZ_integrals/` contain
both first and second Symanzik polynomials as literal `ExponentiatedPolynomial`
arrays. `examples/zz_rayleigh_guides.rs` reads these arrays as data without
executing Python. For each input it substitutes the original exact parameters
and checks that the normalized upstream F equals the imported original split.
It retains the original variable map and constraints, including `s > 4` and
unrestricted `t`.

The 27 input identities and aliases come from
`artifacts/research/monotone-inspection-01/manifest.json`. All 27 identities
passed; the complete input/source hashes and 39-axis inventory are in
`artifacts/research/zz-rayleigh-guides-01/summary.json`.

For every physical axis z in which F is linear, write

\[
F=A z+B,\qquad U=C z+D,\qquad R=BC-AD.
\]

Both C and D have independently replayed strict positivity certificates for
all 39 axes. Nevertheless, R takes both signs inside the original domain for
all 39 axes. Each report row stores two exact rational interior points and
their exact R values; every original constraint is evaluated at both points.

## Stronger obstruction to every positive guide

The helper also searches exact open intervals in t that satisfy
`orientation*A > 0` and `orientation*B < 0`. It first fixes positive rational
Feynman parameters and `s > 4`, then solves the two strict affine inequalities
in t exactly. A returned point is checked against all original constraints
and both inequalities. Failure to find a point would be inconclusive.

It found such points for **both orientations on all 39 axes**. At any one of
these points, every guide with C > 0 and D > 0 satisfies

\[
(\mathrm{orientation}\,B)C-(\mathrm{orientation}\,A)D<0.
\]

Consequently, neither a different positive polynomial guide nor a stronger
nonnegativity certificate can establish the current global guide hypothesis
for that axis/orientation. The obstruction also explains the geometry: a
positive slope and negative endpoint produce a bounded negative interval in
z, whereas the current negative-guide reduction restores an upper ray over
a negative-slope base. A partition that retains both slope cases, or a
different coordinate reduction, is still possible.

## A446 and Ax446 explicitly

Let

\[
a=x_1+x_2+x_3,\quad b=x_4+x_5+x_7,\quad z=x_8.
\]

Their exact upstream first Symanzik polynomial is
`U = a*b + (a+b)*z`. The original fixed mass parameters are `mt2=1` and
`mz2=5/18`; s and t remain symbolic. Only x8 is linear in F. With the primitive integer
normalization used in the report, the U comparison satisfies

\[
\partial_t R_A=18(bx_1-ax_5)(bx_3-ax_7),\qquad
\partial_t R_{Ax}=-18(bx_1-ax_5)(bx_3-ax_7).
\]

These factorizations are reconstructed exactly, including the scalar.
At `s=5`, `x2=8`, and every other x coordinate equal to 1, take

| Original | t | A | B |
| --- | ---: | ---: | ---: |
| A446 | −4046/117 | 808/13 | −202 |
| Ax446 | 3526/117 | 808/13 | −202 |

Both points lie in the full original domain. Thus every positive linear
guide fails the required comparison there. Separate exact counterexamples
for the reversed orientation are stored in the same report.

An additional candidate was tested: `guide = F_t` for A446 and `guide = -F_t`
for Ax446. Its coefficients are

\[
C=18(x_1+x_5)(x_3+x_7),\qquad
D=18(bx_1x_3+ax_5x_7),
\]

and both certify strictly positive. The comparison cancels t identically and
is exactly the same 70-term polynomial for A446 and Ax446. Its s derivative is
`324*x2*x4*(b*x1*x3+a*x5*x7)`, which is positive, but this does not establish
positivity of the entire comparison. The exact points above refute it. This
candidate therefore does not justify a production extension.

## Reproducibility and bounds

The Rust helper supervises its worker with a hard wall limit and a 2 GiB
memory limit, pins the worker to CPU 9, and checkpoints completed rows with
atomic replacement. Polynomial arithmetic, factorization, exact evaluation,
and certificates use Symbolica. Every reported factorization reconstructs
its full input, including scalar and multiplicities. The final helper's three
tests cover literal parsing (including exact integer coefficients and rejected
floating literals), exact opposite-sign witnesses, and strict affine-interval
feasibility/orientation.

The final 27-original screen completed in 0.366 s inside the debug worker
(0.406 s supervised wall time, 14,404 KiB peak RSS) under a 120 s limit.
These are exploratory diagnostic timings, not solver or release benchmarks.
The earlier parser attempt retained 12 valid rows before encountering an
unquoted integer coefficient; its failure record remains separate. The final
run completed all 27 originals without missing rows.

Artifacts:

- Final source SHA256:
  `c92894ba416a26f078bf81df15b1dc883a20cbdfe33ae4b93a53c00fc6a14f91`.
- Frozen helper:
  `artifacts/research/zz-rayleigh-guides-01/bin/zz_rayleigh_guides-v4`, SHA256
  `5177a64ebfd45f25ca5d46a2dcf4e017f9da9860fdb1fff570af20eba58b10bc`.
- Complete report:
  `artifacts/research/zz-rayleigh-guides-01/all27-v4.json`, SHA256
  `804f9d08e459cad208845d0c3ebd9bf033ea62045983e76828102faa9be5ba6e`.
- Alternative kinematic-guide report:
  `artifacts/research/zz-rayleigh-guides-01/kinematic-a446-ax446-v3.json`, SHA256
  `b8dd6dfd04fe18fe3e459dabbef3bb447e878d78bf97110469a49f296bb67264`;
  its separately frozen v3 helper/source hashes are recorded beside it.
- Final tests/build: `artifacts/research/zz-rayleigh-guides-01/build-v4.log`.

The helper is research-only. It does not change solver behavior, transport a
decomposition, restrict physical kinematics, or assert coverage.
