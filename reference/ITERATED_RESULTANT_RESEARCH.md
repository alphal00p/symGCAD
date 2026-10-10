# A377: an exact discriminant-of-resultant reduction

Research, 2026-10-10. The captured degree-32 polynomial really is the whole
canonical resultant of much smaller ancestors. Its first ancestor is linear
in `t`. This supports a concrete algebra-only experiment: change the primitive
element of a finite algebra before computing the discriminant, retaining the
complete collision-square and all normalization factors. The bounded first
gate now proves the polynomial substitution and normalization in 0.990 s;
the new polynomial has parameter degree 25. **Its discriminant and collision
determinant remain uncomputed.** This is not a completed discriminant
replacement, speed comparison, or solver completion. Production is unchanged.

## Recovered, hash-bound ancestry

The operand of `Disc_t(p)` in
`artifacts/research/a377-warm03-progress-01/capture.json` has 2,502 terms,
degrees `(s,t)=(84,32)`, and expression SHA-256
`59b41f78e9e212a5c4237595ed766b15f91922e6f631696ba9e0c7b3fa870948`.
The capture SHA is
`bd8bb1416f3154a312f1849dad4a9ceba0f41761f1ac9bfcbd1c36af14ef0d75`.

A bounded metadata scan rehashed all 2,048 files in warm03's final cache
manifest, checked every `SHA256(JSON([output,multiplicities]))`, and matched
ordered operand-array hashes. It found:

| Saved operation | Exact serialized relationship | Recorded native time |
|---|---|---:|
| `33ce3ba6…` | `p = canonical(Res_x0(f,g))` | 0.536075 s |
| `d9717a4d…` | factor input `p`, sole output `p`, multiplicity 1 | 0.163894 s |
| `684fca18…` | raw `Res_x3(h,k)` supplies the input to the next factor record | 0.132624 s |
| `66316810…` | six factors; output index 4 is `g`, multiplicity 1 | 0.004701 s |

These are historic operation records, not independently repeated timings or
a fresh algebraic proof. The first pair was uniquely recovered among 238
candidate `Q[s,t,x0]` outputs; the earlier pair among 25 outputs containing
`x3` and no later axis. No guessed topology or numerical matching was used.

| Operand | Terms | Degrees `(s,t,x0,x3)` | Total degree in `(t,x0)` |
|---|---:|---|---:|
| `f` | 31 | `(3,1,4,0)` | 5 |
| `g` | 770 | `(15,6,8,0)` | 14 |
| `h` | 44 | `(3,1,4,4)` | 4 |
| `k` | 45 | `(3,1,5,5)` | 5 |

The small `f` and `k` have only singleton self-factor records. Their earlier
coefficient/evaluation/guard-producing steps are not retained. This does not
prevent using their exact strings in an operation-level identity, but it
does prevent claiming complete provenance back to the original integral.
The earlier factorization of `g` is not interchangeable with the whole
`Res_x3(h,k)` in a discriminant formula: the other five factors and their
cross-resultants cannot be silently erased.

Evidence: `artifacts/research/a377-resultant-ancestry-01/report.json`, SHA
`58a5a39c21c2ffa691d9663f76681718f284e57bc97c26d992f9bf42ae5f9a55`.
The directory retains the metadata-only audit script and exact copied
`p,f,g,h,k,g_raw_resultant` JSONs with source-file and expression hashes.
The script caps the inventory at 4,096 files / 256 MiB and each pair search
at 512 candidates / 16 MiB; the audit finished under a 30-second deadline.
It does not modify the completed cache or interpret these hashes as proof.

## What the primary literature permits

[Busé–Mourrain, *Explicit factors of some iterated resultants and
discriminants*, Math. Comp. 78 (2009), 345–386](https://arxiv.org/pdf/cs/0612050)
give the relevant universal formulas. Section 5, Proposition 5.2 and
Theorem 5.6 express a resultant's discriminant as a complete-intersection
discriminant times a collision factor squared, with an explicit sign.
Lemma 5.3 constructs that square using a principal subresultant or divided
differences. Theorem 6.8 similarly retains multiplicity-two and
multiplicity-three factors for an iterated discriminant. These factors are
part of the complete result, not a license for filtered projection.
Remark 2.1 fixes **expected degrees** before specialization.

That last qualification matters here: total-degree homogenizations of `f,g`
have degrees 5 and 14, whereas their actual `x0` degrees are 4 and 8.
Padding both univariate degrees to 5 and 14 introduces a common infinity
root and makes that padded resultant zero. Directly substituting these
ancestors into the generic formula therefore does not compute our actual
degree-32 resultant. A separate degree/infinity calculation is necessary.
The following derivation uses the actual support instead.

## Exact linear-ancestor gate, derived for this support

Work over `K=Q(s)` and write `z=x0`,

```text
f = a(z)*t+b(z),       g = Σ(i=0..6) g_i(z)*t^i,
H(z) = a(z)^6*g(-b(z)/a(z),z)
     = Σ(i=0..6) g_i(z)*(-b(z))^i*a(z)^(6-i).
```

This definition is polynomial; it performs no domain restriction at
`a=0`. Metadata gives `a`: 13 terms, degrees `(s,z)=(2,4)`;
`b`: 18 terms, `(3,4)`; and `deg_z(g)=deg_z(g_6)=8`.
Consequently `deg_z(H)≤32`, `deg_s(H)≤29`, and there are at most
`33*30=990` possible bivariate monomials before cancellations. These are
**support bounds**, not observed degrees or a completed construction.
The parameter-degree reduction from 84 to at most 29 motivates the test.

Let `N=deg_z(H)`, `ell=LC_z(f)=a_4*t+b_4`,
`delta=Res_z(a,b)`, and `rho0=Res_z(a,g_6)`. The exact polynomial
`a^6*g-H` is divisible by `f`. Evaluating at the four roots of `f`, with
leading powers retained, gives

```text
ell^(32-N) * Res_z(H,f) = delta^6 * Res_z(f,g).              (1)
```

Here `Res_z(f,a)=Res_z(a,b)` and reversing `f,H` has sign
`(-1)^(4N)=1`. Evaluating `H` at the four roots of `a` gives

```text
a_4^(32-N) * Res_z(H,a) = delta^6 * rho0.                   (2)
```

The sign from `(-b)^6` is positive. Both identities retain the degree
correction explicitly. A first prototype should require `N=32` and
`delta*rho0 != 0` as polynomials over `Q(s)`; otherwise decline. For
`N<32`, the `ell` factor in (1) is a real algebraic contribution from
infinity, not an ignorable normalization.

Under those first-prototype conditions, `a` is invertible in
`A=K[z]/(H)`. Set `theta=-b/a` there and let `chi_theta(t)` be the monic
characteristic polynomial of multiplication by `theta`. The root-product
formula, including `LC_z(H)^4`, gives

```text
Res_z(H,f) = Res_z(H,a)*chi_theta(t).
```

Combining with (1)–(2), without a generic 32-by-32 characteristic-polynomial
calculation,

```text
Res_z(f,g) = rho0*chi_theta(t).
```

A small-parent native resultant replay must bind the signed raw result to
the captured canonical `p`, say `Res_z(f,g)=kappa*p`, with nonzero rational
`kappa`. Then `rho=LC_t(p)=rho0/kappa` and `p=rho*chi_theta` exactly.
Alternatively, directly checking `p=rho*chi_theta` bypasses recovery of
`kappa`, but may be more expensive.

## The indispensable collision determinant

Let `L=LC_z(H)` and let `V` have columns
`1,theta,...,theta^(N-1)` expressed in the basis `1,z,...,z^(N-1)` of `A`.
The trace-pairing matrix changes by `V^T*T*V`. Its determinant in the
`z` basis is `Disc(H)/L^(2N-2)`; in the power sequence of `theta` it is
`Disc(chi_theta)`. Therefore, with the usual signed discriminant,

```text
Disc_t(p) = (rho/L)^(2N-2) * det(V)^2 * Disc_z(H).           (3)
```

Here `2N-2=62`. This identity also holds for nonseparable finite algebras:
both trace determinants may vanish. A zero determinant never proves
nonvanishing or licenses deletion of roots. Independent theorem review
confirmed the trace argument and the normalization exponent.

For example, `H=z²-1`, `theta=s*z` yields `p=t²-s²` and `V=diag(1,s)`.
The factor `s²` in `Disc(p)=4*s²` describes two **distinct** `z` roots
mapping to the same `t` root. It is absent from `Disc(H)=4` and remains
mandatory even though its sign does not change.

Every rational-function denominator must be kept through exact polynomial
division. If `det(V)=v_num/v_den`, (3) means

```text
L^62 * v_den² * Disc_t(p) = rho^62 * v_num² * Disc_z(H).
```

The proposed result must be the **exact polynomial quotient**, with the
division reconstructed; its surviving square factors remain in the guard
basis. A denominator zero is not a restriction of the original parameter
domain. For example `f=(z-s)t+1`, `g=z²-1` has resultant
`(s²-1)t²-2s*t+1` and discriminant 4: rational poles cancel from its
discriminant, while the existing leading-coefficient guard still preserves
the degree-drop loci `s=±1`. Checking only on denominator-nonzero samples
would be inadequate.

## Small prototype and decision gate

First, in one bounded research worker, parse the copied ancestors, construct
`H`, reconstruct its defining identity and exact division by `f`, and record
actual terms/degrees/coefficient heights. Require `N=32`. Compute the small
`delta`, `rho0` and raw `Res(f,g)`, verify the canonical input and its signed
scale, and decline on any zero or failed identity. Do **not** start `Disc(H)`
or construct `V` during this first gate. The 990-support bound makes this
much smaller than blindly expanding another discriminant, though native
arithmetic still needs a hard time/memory limit.

Only if that gate succeeds, separately bound and measure fraction-free
inversion/change-of-basis, `det(V)`, `Disc(H)`, exact cancellation, and full
signed reconstruction. A final candidate needs an independent identity
check, for example all exact univariate images under a proven degree bound;
isolated samples are insufficient. Preserve the ordinary fallback and
every original coefficient, endpoint, discriminant and pair obligation.
The current cache discards signed scalar content, so a cached canonical
answer alone does not check (1)–(3).

The likely benefit is a smaller parameter degree for the hard discriminant.
The likely cost is coefficient growth in a rational 32-dimensional basis
change, and the collision determinant may itself be large. The existing
center test declined, while the first resultant already costs only about
half a second: recomputing that resultant faster is not the objective.
No general multivariate resultant engine, filtered Landau rule, positive-ray
root omission, or new production option is justified by this note.

Primary source accessed 2026-10-10. Related local context:
[center experiment](CENTERED_DISCRIMINANT_RESEARCH.md),
[binary discriminant diagnostics](A377_BINARY_DISCRIMINANT_DIAGNOSTIC.md),
and [support-basis audit](HIGHER_DIMENSIONAL_SUPPORT_BASES.md).

## Completed bounded first gate

`examples/profile_resultant_linear_gate.rs` implements only the first gate.
It binds the ancestry report, exact copied `p,f,g`, full seven-variable map,
ordered pair digest and original capture. Its input parser accepts bounded
signed integer sparse terms only. Before multiplication it checks per-axis
degree, dense support, integer coefficient-height and term-pair bounds; the
complete construction is limited to 30 million term pairs. Small resultant
calls additionally check Sylvester degree/support/coefficient bounds. A
killable single worker enforces 120 s and 4 GiB. Progress and result files
are fresh diagnostic artifacts, never solver checkpoint entries.

The actual CPU12 worker passed:

| Quantity | Exact result |
|---|---|
| `H` | 644 terms, degrees `(s,z)=(25,32)`, maximum 378 coefficient bits |
| Geometric quotient | 3,872 terms; `a^6*g-H=f*quotient` checked exactly |
| `delta` | nonzero polynomial, degree 14 in `s`, 11 terms |
| `rho0` | nonzero polynomial, degree 60 in `s`, 61 terms |
| Parent resultant | `Res_z(f,g)=6561*p` exactly |
| Leading identity | `LC_t(Res_z(f,g))=rho0` exactly |
| Infinity correction | `32-N=0` |

"Nonzero" here is a polynomial identity statement over `Q(s)`; it does not
claim these polynomials never vanish on the physical domain. All are retained
in the candidate. The five explicit source hashes were not changed by the
worker; no physical parameter point or root locus was removed.

Construction/remainder checking took 0.235129 s, the two small resultants
0.001465 and 0.005489 s, and parent resultant plus scale checks 0.619504 s.
The complete worker took 0.990003439 s, 22,444 KiB peak RSS. These are one
diagnostic's timings, with no matched solver comparison.

Evidence under `artifacts/research/a377-resultant-linear-gate-01/`:

* `report.json`, SHA
  `0f6f1ce5c6f7f5c46b95f08ed82da47679427e0694208ef76ee60809ef2b02e2`.
* Signed `report.candidate.json`, SHA
  `c6fcd3773d4671a56db5b7e9a511b4e722a836b82410f7a017837d81f71cbba5`.
* `H` expression SHA
  `4bd958543ac38a64aba19ca56302b178ec26eddf6031e9405ded33be402cdda8`.

Frozen helper SHA
`62f2a4942b475452815f7430785d10b1e7265197edc7e1e41e3d455ea2771e67`,
source SHA
`96c487fe97bcb2d1862052b1ffe3626953b558aed51b950b371b8b9c44703910`;
`artifacts/research/resultant-linear-gate-build-01/build-provenance.json`
records HEAD `44d46ef`, the complete library source manifest and build log.
Six focused tests passed: exact small-fixture norm identities, degree loss,
forged target, both zero-norm branches, scalar reconstruction, and parser/
product caps. The scalar unit test covers a negative rational multiplier;
the actual input interface remains integer-only. The degree-4/8 trap has
`delta=1,rho0=16` but `N=28`, and is explicitly declined.

Tests/build used `taskset -c 14,15`, `cargo -j2`, 4 GiB and a 300-second
external deadline. The worker used the frozen executable with
`--cpu 12 --seconds 120` under a separate 135-second external deadline.
Independent source/theorem review found no blocker. Further work must
measure **both** `Disc(H)` and the complete change-of-basis contribution
before making an algebra-cost claim.
