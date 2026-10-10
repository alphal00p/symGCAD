# Exact sum reduction for a stalled A446 factorization

The final captured factorization in the interval-Descartes A446 campaign is
small: 41 terms, active degrees `(3,3,6)` in `(s,t,x1)`, and coefficient
height 17 bits. Nevertheless, separate fresh debug processes using Symbolica's
automatic, univariate-start and bivariate-start policies each reached a
60-second limit. A separate older automatic-policy replay also timed out.

The polynomial depends on s and t only through their sum. Symbolica checks
`derivative(p,s)==derivative(p,t)` exactly. Let `q=p|s=0`; then the additional
exact identity `q(t=s+t)==p` verifies the reduction independently. The helper
factors q in two active variables and applies the inverse substitution to
every factor. It checks scalar content and multiplicities in the factor
product before transport. The inverse identity then reconstructs the original
polynomial as well.

This changes only the algebra used to obtain factors. It does not transform
the CAD domain, order, coordinates, cells or boundary semantics. Over Q,
adjoining the unused s preserves irreducibility of a factor in the smaller
polynomial ring, and the invertible shear `t -> t+s` preserves irreducibility
and multiplicities. A production precheck can therefore use this reduction
with exact reconstruction and retain ordinary factorization whenever its
bounded discovery is inconclusive.

Three fresh reduced-factor processes on CPU2 with a 4-GiB/60-second cap took
0.025353, 0.003415 and 0.003169 seconds in the measured algebra phase. All
returned the same canonical factor-set hash, with 26 total output terms.
These are operand diagnostics, not a whole-solve speedup or completed A446
decomposition. Native-policy trials used CPUs1,2,3 respectively, each with
the same memory and time cap. Two additional unaffined reduced-factor
diagnostics are retained separately and excluded from the listed timings.

The `disabled` start-policy negative control returns the unsplit input
immediately: Symbolica explicitly returns `self` in that mode. It is not a
complete factorization result or an irreducibility certificate and must not
be counted as a fast successful trial. Production does not use this mode.

## Evidence

The input capture SHA-256 is
`caeb1e6c55cdb75dff0b31bfe989c1ca7f09b25db3ac1651e2905a4b3570baf5`.
Native-policy outcomes are retained under `artifacts/profile/factor-start-01`;
the reduced trials and immutable helper are under
`artifacts/profile/factor-sum-collapse-01`.

| Artifact | SHA-256 |
| --- | --- |
| Reduced-factor helper | `5ac1762e814fb2f81bbe8fc67bdd6a3d2bb40ebc5e404163870922b67a94f0b7` |
| Helper source | `2ec2bcd962cdc8e651203a3b114a01629f016e048970c9bc07fcccbc0bcebf87` |
| Initial reduced-factor report | `11de0ca2413d2966ae2093f1c4d5bc86b5fa0605a8c641e43b6e4e635f78a8db` |
| Repeat 2 report | `291101274bb2b896e518dbe8247abfd70a05230e328c0e92461d26eaaefc1d30` |
| Repeat 3 report | `4cf19ace4d4d2448fca619fb9d7b02f9d9c6e1b37e79227bf64dd1bd6f702b6b` |
| Canonical transported factor set | `8f5d26f563819cb8b6a3076e919f3bea349df6fefc5baa163e418cc717f84843` |

The research helper's five tests cover factor content and multiplicities,
the exact sum substitution and rejection of a false invariance assertion,
alongside its existing root-diagnostic regressions. All five passed before
the helper was frozen. The Symbolica checkout remains unchanged.
