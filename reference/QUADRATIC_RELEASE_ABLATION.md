# Quadratic certificates and ladder: release measurements

All **66 fresh solve trials and 66 separate verification trials completed**.
The experiments measure the two opt-in positivity certificates and the
quadratic ladder. They contain synthetic and conditional problems; they add
no original corpus completions and no new comparison with native GCAD.

Three cold repetitions used one process at a time, pinned to CPU 3 of an
AMD EPYC 9754, with a 2 GiB worker limit. The same frozen release binary,
given order and unmerged output were used throughout. The certificates are
disabled by default; the measurements do not support enabling every
certificate on every input.

Cold means fresh application checkpoint namespaces; the OS page cache was
not flushed. Variants ran in a fixed order. Reported maximum peak RSS includes
both solve and verification processes across the three repetitions.

## Certificate ablation

Each entry below gives the median **internal solver milliseconds / raw
cells**. The four configurations differ only in the two certificate flags.
Every cell count was the same in all three repetitions. All five original
case definitions, orders and domains remain identical between variants.

| Case | Both off | Weighted squares | Quadratic components | Both on |
|---|---:|---:|---:|---:|
| Positive endpoint | 37.160 / 20 | 9.305 / 1 | 39.040 / 20 | 9.553 / 1 |
| Constant coefficient C0 | 10.304 / 2 | 12.430 / 2 | 6.412 / 1 | 6.951 / 1 |
| Full quadratic constant | 18.342 / 8 | 24.864 / 8 | 7.629 / 1 | 8.337 / 1 |
| Unrelated square family | 10.147 / 6 | 10.939 / 6 | 4.873 / 1 | 5.109 / 1 |
| Local linear split | 2.374 / 12 | 3.848 / 12 | 0.823 / 2 | 1.159 / 2 |

The weighted certificate recognizes the endpoint while the quadratic
component certificate recognizes C0 and its extensions. The local example
keeps its two sign regions and their interior linear zero sheet. A smaller
cell count here comes from verified nonvanishing of auxiliary cuts, not
merging across an interior singular seam.

For public process timing and RAM, the matching measurements are:

| Case | Configuration | Median process seconds | Maximum peak RSS, KiB |
|---|---|---:|---:|
| Positive endpoint | Off / weighted / components / both | 0.102508 / 0.061243 / 0.101759 / 0.061257 | 13684 / 6196 / 13724 / 6180 |
| C0 | Off / weighted / components / both | 0.061277 / 0.061405 / 0.061325 / 0.061336 | 6192 / 6208 / 6196 / 6220 |
| Full quadratic constant | Off / weighted / components / both | 0.081800 / 0.081619 / 0.061391 / 0.061378 | 12700 / 12648 / 7076 / 7620 |
| Unrelated square family | Off / weighted / components / both | 0.061459 / 0.061402 / 0.061356 / 0.061469 | 7648 / 7648 / 7648 / 7648 |
| Local linear split | Off / weighted / components / both | 0.061425 / 0.061451 / 0.061435 / 0.061511 | 10432 / 7648 / 10844 / 7648 |

These small process timings are strongly affected by startup and the
supervisor's polling intervals. Differences within the approximately
61 ms group are not evidence of meaningful end-to-end speed differences.
The internal phases and exact projection/cell reductions show what changed;
they must not be substituted for the public time-to-solve column. Verification
is excluded from solve time and took about 21 ms per fresh process.

## Exact input definitions

For all cases, each named x, y, z, h and v coordinate is strictly positive;
u has the sole bound u<4. The order is as written below. Each displayed
polynomial is a sign split, not an extra strict constraint.

Let A=x1+x2+x3, B=x4+x5+x7, X=x1+x5, and Y=x3+x7.

| Case | Split polynomial | Order | Engine |
|---|---|---|---|
| Positive endpoint | B*(A^2-u*x1*x3-(5/18)*x2*x3)+A*(B^2-u*x5*x7-(5/18)*x4*x7) | u,x1,x2,x3,x4,x5,x7 | optimized |
| C0 | (X+Y)^2-u*X*Y | u,x1,x3,x5,x7 | optimized |
| Full quadratic constant | (X+Y)^2-u*X*Y+x4^2+(2*X+(31/18)*Y)*x4 | u,x1,x3,x5,x7,x4 | optimized |
| Unrelated square family | (2*x-3*y+z)^2+h*x*y*z+1 | h,x,y,z | optimized |
| Local linear split | ((X+Y)^2-u*X*Y)*v-1 | u,x1,x3,x5,x7,v | local |

All optimized cases enable ordinary positivity, input positivity and
operation pruning. Only `weighted_square_positivity` and
`quadratic_component_positivity` vary. The independent verifier replays the
selected certificate identities and complete decomposition evidence.

## Ladder validation

| Input | Median process seconds | Median internal solver seconds | Peak RSS, KiB | Cells | Fresh verification seconds |
|---|---:|---:|---:|---:|---:|
| [Generic four-variable example](../examples/quadratic_ladder.toml) | 0.061230 | 0.001577 | 6208 | 5 | 0.020871 |
| Crossed A446, original F restricted to t<4 | 0.061135 | 0.005268 | 6196 | 5 | 0.020867 |

The second row uses exactly the conditional problem and given order in the
[ladder validation](QUADRATIC_LADDER_VALIDATION.md). It retains all original
Feynman parameters and the original F boundary. The unrestricted symbolic
problem still requires t>4 and a valid composition with recorded threshold
sections. Its earlier two-fresh-points-per-cell supplemental sanity check
remains separately reported and is not part of these timings.

## Reproducibility

The immutable executable was built from source commit `ca0eb5f`, with
production code unchanged from `0adcf97`. The release build took 482 seconds
and peaked at 4,270,244 KiB sampled aggregate RSS under an 8 GiB cap. Source
hashes were checked before and after compilation.

| Artifact | SHA256 |
|---|---|
| Frozen release executable | `2abcfd14ec80973489fe803efa787ff22cff21b5c36dc9fb6bc1bc4660afd15c` |
| Production source manifest | `91e70f89c7af2561926918fe90ddf440d4b3d9587b9f7877455fff56f2a34bce` |
| Certificate campaign config | `83ea65a4196fd2203dd5b1ccb3531a7d845095fa0602527b3002d8d0fbf7e47a` |
| Certificate campaign ledger | `d960cac12f9b03e00f9e2a58ad9ea60283c683cea6500fa10414bb64f12df136` |
| Ladder campaign config | `b07c83390efaddbf682d3e1e281095089d9ae2d3d6df60a93dc0fc49c52238ec` |
| Ladder campaign ledger | `51b1719ac4711a12ab1405580dca3003df10efa0beb15e910d2752cbe4760049` |
| Compact numerical summary | `b49db11da174de35958e084a1323bcc417121e17128966cf4779eba6386d4d82` |

Generated inputs, cold per-trial caches, result/verification hashes and
process records are retained outside Git under
`artifacts/research/quadratic-release-ablation-01/` and
`artifacts/research/ladder-release-validation-01/`. Each campaign has three
repeats and a 300-second campaign budget; solve/verify caps are 10 seconds
for the certificate cases and 30 seconds for the ladder cases. No trial
timed out, resumed an earlier result or used a warm operation cache.
