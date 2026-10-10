# Unresolved symbolic kinematics and ladder scope

The 27 unresolved symbolic original representatives all depend genuinely on
**both `s` and `t`**. None is an `s`-only problem. Fifteen have eight variables
(six Feynman parameters), and twelve have nine (seven Feynman parameters).
Every input has the domain `s > 4`, unrestricted real `t`, and strict
positivity of each Feynman parameter, with exact `mt2=1`, `mz2=5/18`.
The separate unresolved Gluza33 input has fixed kinematics and is not part
of this inventory. Original completion coverage remains 303/331.

Each symbolic split is homogeneous of degree three in its Feynman
parameters and affine in each kinematic variable. Its individual Feynman
degrees are one or two. Original groups place `s,t` before the Feynman group;
there is no additional fixed order. Consequently the following final linear
axes and preceding quadratic axes offer legal support patterns, but support
alone establishes none of the ladder's positivity premises.

| Original families | Representatives | Possible final positive linear axes |
| --- | ---: | --- |
| A/Ax12: 317, 318, 377, 414, 381, 446 | 12 | `x8` |
| B/Bx12: 471 | 2 | `x0`, `x1`, `x2` |
| C/Cx12: 215, 231, 247 | 6 | `x7` |
| C470; C/Cx12: 476, 478 | 5 | `x7`, `x8` |
| D251 | 1 | `x0`, `x1` |
| D375 | 1 | `x0`, `x1`, `x2` |

## Why the present ladder cannot cover these full domains

The [quadratic-ladder rule](POSITIVE_ENDPOINT_QUADRATIC_LADDER_PLAN.md)
extracts the signed identity

```text
F = B + z [a x^2 + (b0 - b1 y) x + c]
```

and requires `B,a,c,b0,b1` (and the outer discriminant's leading coefficient)
to be strictly positive on the entire original domain. Its three fiber axes
are positive Feynman parameters; the group rules leave unrestricted `t` in
the base. Coefficient extraction preserves affine dependence on `t`.

Because `F` genuinely depends on `t`, at least one of `B,a,c,b0,b1` also
does. A nonzero coefficient of `t` cannot vanish on the entire open domain
of the other variables. Fixing a point where it is nonzero makes that
premise an affine function with nonzero slope on all real `t`; it takes
both signs. Thus at least one required global strict premise fails. This
is an obstruction to this specific theorem, not evidence that the original
CAD is impossible or that no other exact reduction exists.

The [crossed-A446 result on `t<4`](QUADRATIC_LADDER_VALIDATION.md) does not
contradict this argument: it explicitly bounds `t`. Its five verified cells
retain the whole original F and its interior zero sheet. The unrestricted
original and the open `t>4` complement remain unresolved; a future full-domain
composition must also account for the threshold section. No conditional
result earns unrestricted-original completion credit.

## Inventory evidence

The read-only inventory is
`artifacts/research/unresolved-ladder-inventory-01/inventory.json`, SHA-256
`e0f6aff3575f2b02cfbdbeaf92b696e335dbebaa28a08d53eb07a693be3073a4`.
It binds all original file hashes to the retained 28-case slow-suite
preparation manifest, excludes its fixed-kinematics Gluza entry, and records
every variable degree, linear/quadratic support pair and legal fiber triple.

This was a bounded literal inspection, with no algebra worker or solver run.
The parser reconstructed every split string from its coefficient/monomial
records and checked distinct Feynman supports. All coefficients belong to
seven explicit spellings: `mt2`, `2*mt2`, `2*mt2-mz2`, `2*mt2-s`,
`2*mt2-t`, `2*mt2-2*mz2+s+t`, and `-s`, ignoring whitespace. At the supplied
exact masses each is a nonzero polynomial; each occurrence of `s` or `t`
has literal coefficient `+1` or `-1`. Therefore there is no hidden support
cancellation in this degree inventory. The recorded fiber triples are
untrusted ordering candidates, not certificates or proposed domain changes.
