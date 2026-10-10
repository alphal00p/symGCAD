# Unresolved-corpus symmetry audit

The retained completion audit still certifies **296/331** original problems:
122/124 fixed ZZ, 97/124 symbolic ZZ, and 77/83 core. Its 35 unresolved inputs
are two fixed A381 variants, 27 symbolic ZZ inputs, and six core inputs. A
read-only scan of every retained campaign ledger found no verified trial for
any unresolved alias that had been omitted from this aggregate. This scan is
an inventory check, not a new mathematical proof verification.

`examples/corpus_symmetry.rs` compared all 35 unresolved originals against all
331 original representatives. It used exact Symbolica normalization and
polynomial equality, with integer exponent permutations as a necessary support
screen. The allowed maps were propagator permutations, any subset of
coordinate reciprocals supported by original strict positive-coordinate
constraints, and the ZZ crossing `t -> 5/9-s-t`. Every candidate then had all
strict constraints and all ordered split polynomials checked. Split orientation
changes are recorded explicitly. Variable groups, explicit original orders,
verified donor proof orders, and triangularity of the crossing are checked
separately; a polynomial identity alone is insufficient.

The completed search took 4.154 seconds in one debug-helper process. It found
34 exact identities, but **zero eligible verified donors**:

* 32 directed identities connect 12 pairs of unresolved symbolic ZZ inputs.
  Some pairs admit more than one propagator permutation. None needs a
  reciprocal transformation.
* Two identities map the physical elliptic core problem to its verified
  `solve/2` input: the identity map, or inversion of the inactive coordinate
  `x2`. The existing proof's order interleaves kinematic and Feynman variables,
  violating the unresolved original's outer-kinematics groups. Neither is an
  admissible transferred cylindrical proof.
* No other reciprocal match was found. Both fixed A381 inputs, symbolic C470,
  D251 and D375, and the five other unresolved core inputs have no donor in
  this searched transformation class.

The 12 symbolic pairs are A/Ax12 sectors 317, 318, 377, 414, 381 and 446;
B/Bx12 sector 471; and C/Cx12 sectors 215, 231, 476, 247 and 478. Thus, with a
future exact transport verifier, solving 15 suitably ordered symbolic
representatives could cover the 27 symbolic inputs. This does not reduce the
331-original denominator or count any counterpart as already solved.

For symbolic A381 the simplest exact identity is
`F_A(s,t,x) = F_Ax12(s,5/9-s-t,x)`. There is also the symmetry
`(x0 x2)(x4 x6)`. The domain `s>4`, unrestricted `t`, and positive Feynman
coordinates is preserved by this affine involution. The map is triangular
when `s` precedes `t`. At the fixed kinematics `s=10,t=-3`, however, it sends
`t` to `-58/9`: that is a different fixed problem. A fixed-A381 proof at
`t=-3` cannot certify the original fixed-Ax381 input by this argument.

## Proof transport obligations

The current completion auditor intentionally compares original-coordinate
mathematical identities. It streams and hashes the executable, input, result,
and verification files; checks complete status and verified cell count;
normalizes the full strict domain and ordered split list; preserves variable
groups and explicit fixed orders; and records failed attempts. An identity
report must not be supplied as a completion ledger or substitute for a full
split-problem certificate.

A future transport wrapper should follow the existing homogeneous and
sign-filter evidence pattern: retain a complete independently verified donor,
record a bijective coordinate map and inverse, reconstruct every original
domain/split identity, deterministically transform the entire cells and
exceptional seams, then independently reproduce this reconstruction. Its
original-coordinate result must preserve the original groups and fixed order.

For a positive coordinate reciprocal, with degree `d_i` in every inverted
coordinate, the exact cleared identity is

`q(x) = (product x_i^d_i) * p(...,1/x_i,...)`.

The multiplier is strictly positive on the certified domain, so strict signs
are preserved. The map is a bijection of that positive ray and reverses its
order. Bounds swap, zero maps to positive infinity, and the index of a positive
root reverses as `n-1-i`, using the exact distinct positive-root count of that
boundary's own polynomial. Merged root ownership and all-real ordinals do not
supply this count. All ancestor coordinates, sample points, polynomial maps,
and excluded seams must also be transformed. The affine `t` reflection likewise
reverses that fiber's endpoints and all-real root ordinals, and requires `s`
earlier in the order. Permutations must map the donor order to a legal original
order rather than merely relabel a noncylindrical cell set.

The physical elliptic donor's order
`[x4,x0,x1,x3,x6,pp4,x5,s,t,x2]` illustrates why these obligations matter.
Its normalized polynomials match the unresolved input, but it does not put
`[s,t,pp4]` before all Feynman parameters. Recovering the required parameter-
outer decomposition requires actual recylindrification or another solve.

## Reproduction and provenance

```sh
nix develop -c cargo test --example corpus_symmetry
nix develop -c cargo run --example corpus_symmetry -- \
  --audit artifacts/completion-aggregate-296.json --reciprocal --seconds 120 \
  --output artifacts/corpus-symmetry-replay/identities.json
```

The retained run used:

| Artifact | SHA-256 |
| --- | --- |
| `artifacts/completion-aggregate-296.json` | `bb7bcf58440c6e364d9da2a8d1ae850f95548274fbdc2012e50c5df42e9819d9` |
| `artifacts/corpus-symmetry-02/corpus_symmetry` | `e9fcd30d1bb1fd9d9d7d7de232d65427bd6ee91620845b3de9b72a8cf4c19844` |
| `artifacts/corpus-symmetry-02/corpus_symmetry.rs` | `5732ebe75f917e863ae245bb7c23b461afd365765537145de7b6459af6afb059` |
| `artifacts/corpus-symmetry-02/identities.json` | `33da5b38b333bdcaf7004f3ef988b3d9a3301342f357c9c3cab49d693d7c1755` |

Two helper tests pass: exact reciprocal sign/inverse identities and the fact
that crossing can change expanded monomial counts before support screening.
The first `corpus-symmetry-01` report is retained but superseded: it applied a
term-count prefilter before crossing expansion and missed valid crossings.
Its positive matches remain exact identities; its negative crossing results
are not used. The corrected second search removed that prefilter.

The search does not cover arbitrary positive rescalings, translations,
projective maps, parameter relabelings, or removal of positive monomial
factors. Absence of a match proves no broader inequivalence. No original
completion is claimed by either report.
