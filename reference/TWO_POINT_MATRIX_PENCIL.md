# A matrix-pencil route for two-point domains

This is a research deduction from the already checked graph identities, not
a new solver rule or a completed positive Gluza33 decomposition. The matrix
identities have a bounded exact Symbolica replay, described below. They extend the
[two-point convexity proposal](TWO_POINT_CONVEXITY.md).

## Exact sign representation

For a connected two-terminal graph with positive parameters, fixed rational
s>0 and the authenticated convention

```text
U=det(M)>0,   V=c*U-j^T*adj(M)*j,   L=sum(m_e^2*x_e),
F=L*U-s*V,
```

form the symmetric rational pencil

```text
S(x) = [[s*M, s*j],
        [s*j^T, s*c-L]].
```

M is the positive definite cycle Gram matrix. Its Schur complement gives
`det(S)=-s^ell*F`, where ell is the cycle rank, and

```text
F<0 iff S is positive definite;
F=0 iff S is positive semidefinite with one-dimensional kernel;
F>0 iff S has exactly one negative eigenvalue.
```

These are consequences of the block congruence and M>0, not numerical
eigenvalue tests. They hold on the complete original positive orthant under
the checked two-point hypotheses. They do not extend automatically to
multi-leg indefinite Minkowski Gram data. For varying s the matrix has
bilinear s*x entries; convexity is in x at each fixed s, not jointly in all
kinematics and parameters.

With checked edge vectors w_e, each edge contribution has the form
`s*x_e*w_e*w_e^T - m_e^2*x_e*e_last*e_last^T`. Massless directions are PSD
rank one. Massive directions are generally indefinite rank two. This is
graph-derived structure useful for choosing and certifying elimination axes.

## A strict rank-one projection theorem

Suppose an eliminated positive coordinate z enters as `T(v)+z*w*w^T`, with
nonzero fixed rational w. Choose an invertible rational basis Q whose first
columns span ker(w^T). A congruence gives

```text
Q^T(T+z*w*w^T)Q = [[H(v), k(v)],
                   [k(v)^T, d(v)+gamma*z]],   gamma>0.
```

Then a positive z making this matrix strictly positive definite exists
exactly when H(v) is positive definite. Necessity follows by restriction;
sufficiency follows by taking z larger than the finite Schur-complement
threshold. The exact negative-F fiber is above
`max(0,(k^T H^-1 k-d)/gamma)`.

This theorem recovers the logic of existing massless Rayleigh peels in a
matrix form. It does not remove the original positive-F fiber's switch
between finite and infinite endpoints, nor the exceptional sections of H.
Subsequent massive-coordinate projection is a different problem; one cannot
assume that an arbitrary projection of a spectrahedron is again a single
matrix inequality.

## Gluza33 illustration

Using the authenticated 3-by-3 M,j,c and s=10 from the
[Rayleigh report](GLUZA33_RAYLEIGH_REDUCTION.md), the two massless updates
have directions `(0,1,-1,0)` and `(1,0,0,1)`. Choose the first two columns of
Q as `(1,0,0,-1)` and `(0,1,1,0)`, followed by those two update directions.
Then det(Q)^2=16 and the eliminated coordinates add only `40*x3,40*x5` to
the last two diagonal entries. The remaining principal block is

```text
A=x0+x2+x7,   B=x1+x4+x6,   L=A+B,
H = [[10*A-L, -10*A],
     [-10*A,    10*L]],
det(H)=10*(10*A*B-(A+B)^2).
```

This reproduces the known quadratic derivative base. Its second diagonal
entry is strictly positive. Thus, away from det(H)=0, det(H)<0 already
excludes positive definiteness of S for every positive massless pair.
By the one-negative-eigenvalue identity above it gives F>0 on that whole
fiber. For det(H)>0, a 2-by-2 Schur complement retains both massless
coordinates and all their finite-boundary switches. Its off-diagonal square
can vanish; the known interior pinch in the positive-bilinear report remains
an explicit obligation. This recasting does not solve the remaining base.

## Literature and next measurement

[Anai–Parrilo, *Convex Quantifier Elimination for Semidefinite Programming*](https://www.mit.edu/~parrilo/pubs/files/AnaiParrilo-CASC03.pdf)
uses convexity and a feasible interior point to specialize CAD lifting and
reduce the polynomial description of matrix feasibility. Their determinant
boundary argument does not authorize replacing every matrix condition by a
determinant sign. In this proposed application that equivalence comes from
the separately proved positive Gram block and its single scalar Schur
complement. Since symGCAD already starts from F, their reduction from many
principal minors to a determinant is not itself a new projection saving here.

The next useful gate is a comparison of conditional projection costs before
and after the certified rank-one peels. In particular, compare
an adaptive split on det(H) against the existing full coefficient-sign base;
preserve and solve its entire det(H)>0 complement. Tests must reject a changed
signed F, a missing Gram-positivity premise, a massive direction incorrectly
treated as rank one, and a proof that erases an interior zero or endpoint
switch. No numerical SDP feasibility result supplies any of these proofs.

## Exact computational replay

The generic research helper [`two_point_pencil.rs`](../examples/two_point_pencil.rs)
accepts an original TOML problem and separate rational routing data: fixed s,
one augmented vector and squared mass per original variable, proposed massless
axes, and an optional rational joint basis. It checks all strict coordinate
bounds, nonnegative masses, full rank of the loop Gram vectors, the entire
signed original determinant identity, and every congruence and rank-one
update. Routing metadata alone is not authority: the complete input F must
match exactly. The helper does not claim to reconstruct graph connectivity.

Six focused tests passed, including a different one-loop graph, changed F
sign, rank deficiency, missing coordinate bounds, a massive peel, negative
mass, and an invalid joint basis. The joint-basis test compares exact
polynomials rather than their display spelling. The two fresh debug-helper
runs in `artifacts/research/two-point-pencil-03/` completed as follows:

| Input | Exact checks | Process wall time | Reported peak RSS |
| --- | --- | ---: | ---: |
| Original positive Gluza33 | Signed F, Gram rank, both individual massless peels, joint H with updates 40*x3 and 40*x5 | 0.042298 s | 3,088 KiB |
| One-loop bubble with one massive and one massless edge | F=a²−4ab at s=5, individual and joint-basis peel | 0.020841 s | 3,076 KiB |

These are identity diagnostics, not solve timings or speedup measurements.
Each runtime had a killable 60-second/2-GiB worker on CPU 3. The helper admits
at most 12 edges and three loops, small integer routing vectors and bounded
rational literal data. Original-expression parsing and normalization are
bounded by that worker, not by a claimed pre-parsing sparse-support limit.
The two bounded compilations took five and six seconds and stayed below
747,000 KiB sampled aggregate RSS. Large generated reports remain untracked.

Reproducibility hashes:

```text
source: e2f9de1b50d35f9f4cc34c4a8e0f533a4861e801097f4f258f8bde99ed78b157
helper: 04d8de40c19619afdb8cf3e28912a071a2313f090286d74c491d6fb846148a23
Gluza report: a64dfc061ace8536fcd89a6d427eb74e975c1a311462353bab9e7719287ce447
bubble report: 29b7552db75011812aec1c28982b6f9c595e1dc7554b72eb522dd3cc87fe53f2
```

This completes the identity gate for two graph families. A joint positive
diagonal update has a feasible positive fiber exactly when its retained H
is positive definite, by the same Schur-complement argument. Exporting all
original positive-F cells still needs the finite endpoint switches, pinch
seams and full base decomposition. No original completion credit follows.
