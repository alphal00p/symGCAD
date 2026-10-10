# A377: exact isolation of the square-free blocks

The complete checked block union of the verified A377 discriminant has
**84 distinct real roots, including 37 strictly above `s=4`**. Its degree-881
block contributes 69 real roots, including 35 above 4. Existing exact root
isolation completed within the 180-second research limit. The block therefore cannot
be discarded as nonvanishing on the original kinematic domain, even though
its multiplicity in the discriminant is even. This is a root inventory for
a projection operand, not a completed CAD or a physical singularity claim.

**The ordinary full-factorization control also completed**, in 177.241 s
under a 300-second deadline. The earlier 60-second timeout established only
insufficient time in that diagnostic, not factorization intractability.
No production composite-basis exception follows from these measurements.

A subsequent **matched release-build pair completed both pipelines** on
the same raw D: square-free decomposition plus root union took 92.063 s;
full factorization plus root union took 185.827 s. Both reconstructed the
same signed polynomial and returned matching complete 84-root inventories.
This single operand comparison motivates further evaluation; it is not a
solver speedup estimate or a reason to weaken the existing factor checker.

The [signed discriminant](A377_BINARY_DISCRIMINANT_DIAGNOSTIC.md) was checked
against 2,480 independent exact resultant images. The
[factorization diagnostic](A377_UNIVARIATE_FACTORIZATION.md) then obtained
an exact square-free decomposition, retaining scalar and multiplicities.
The present helper independently reconstructs that entire original signed
polynomial before selecting any block. Neither the sidecar's hashes nor its
square-free labels replace this algebraic check.

## What the existing root API does

At pinned Symbolica revision
`75f8350094b90254ee71dc2a391fde0d14b0204a`, the relevant source paths are:

- `src/poly/univariate/roots.rs`, rational
  `isolate_real_root_intervals`: clear rational content and delegate to the
  integer isolator.
- `src/poly/univariate.rs`, integer `isolate_real_root_intervals`: call the
  same `square_free_factorization` used by full factorization, isolate each
  block on the positive and negative rays, then separate intervals belonging
  to different blocks. The private `isolate_real_roots_square_free` routine
  uses exact VAS continued fractions, rather than a Sturm sequence.
- `src/poly/factor.rs`: square-free decomposition extracts common monomial
  powers and uses the derivative/GCD decomposition. Full factorization
  continues with the additional irreducible-factor work.
- [symGCAD's root union](../src/roots.rs): if native isolation returns a
  repeated real root, recompute `gcd(p,p')` and retain the resulting fiber
  for exact sign refinement. Shared roots between inputs are merged exactly;
  all distinct full-real ordinals and strict interval ordering are retained.

Consequently, full-D isolation would repeat square-free work after an
external decomposition and can repeat a full-input derivative GCD afterward.
Passing checked blocks separately avoids that particular full-input GCD,
but native isolation still decomposes each block. There is no public native
entry point accepting a precomputed square-free decomposition. No dependency
patch or production root API change was made here.

## Bounded probe and exact evidence

[The research helper](../examples/profile_univariate_isolation.rs) supports
the original D, its exactly deflated residual, one checked factor block,
or the union of all checked blocks. It bounds the input grammar, degree,
coefficient height, factor multiplicities, product degree and reconstruction
size before performing algebra. The selected block may be reducible or have
repeated factors: the existing root API handles both.

The degree-881 probe used one CPU10 worker, 4 GiB, and a 180-second hard
deadline. Its exact signed original reconstruction took 2.704 s; root
isolation and interval separation took 73.830 s. Total process wall time
was 77.183 s, with peak RSS 74,200 KiB. This is one development-build
diagnostic, not a repeated timing comparison or a whole-solver speed claim.

All 69 returned intervals are strictly ordered and disjoint. Exact rational
comparison of their endpoints places 34 wholly below 4 and 35 wholly above
4, with no interval meeting or straddling 4. The first root above 4, with
full-real ordinal 34, lies in

```
16384/4077 < s < 16384/4045.
```

A second fresh worker isolated the complete union of all nine nonconstant
blocks, including the coordinate and threshold factors. Signed original
reconstruction took 2.698 s, isolation and union separation 74.011 s, total
wall time 77.316 s, and peak RSS 75,300 KiB. The result contains 84 strictly
ordered distinct roots: 46 below 4, the exact root 4, and 37 above 4. No
interval straddles 4. The helper retains each owning block's complete real
ordinal; the first root above 4 has global union index 47 and block index 34.
The degree-111 block owns seven roots, the degree-881 block owns 69, and the
remaining blocks own eight. A rootless block is retained in the product even
though it supplies no interval.

Adding the separately measured 35.971-second export/SQF process to this
77.316-second union process describes one possible two-stage research
pipeline, including repeated preparation. It is not a same-build paired
benchmark, a production solver timing, or proof that full factorization is
slower. The ordinary full-D factorization control uses a different frozen
release executable and a longer deadline.

The interval inventory is generated by the existing certified native/root
union path; this probe does not independently rerun a high-degree Sturm
sequence for every interval. Four focused helper tests check small repeated
and shared-root examples, exact interval membership, full-real ordinals,
bounded parsing and tampering with an unselected factor. All four passed;
the combined build also passed the three factor-export helper tests.

The root callback publishes entry into native isolation but does not expose
internal square-free, VAS or post-isolation GCD times. A late attempted perf
attachment found the worker had already exited, collected no samples and
provides no stage attribution. Its failed diagnostic logs are retained.

## Ordinary factorization control and next measurement

A fresh CPU9 process used the unchanged full-D factor capture, native
`factor()` with its automatic policy, 4 GiB, and a 300-second hard limit.
It completed in 177.241 s total, with 177.061 s in factorization plus exact
signed multiplicity reconstruction, and 74,072 KiB peak RSS. The frozen
release helper reconstructs the entire input from the native factors before
accepting them. Its output reports 1,023 total nonconstant factor terms,
maximum coefficient height 3,002 bits, and canonical factor-basis SHA-256
`7ffbaf4b920a749c645a02357261bf9cb000c83c50f1cc290c29c987173653a1`.

That helper exports neither factor expressions nor their degrees,
multiplicities or count. The completed report is an exact accepted-operation
measurement, not a reusable factor sidecar. Its algebra timer does not
separate factorization from product reconstruction. Ordinary irreducible
factor isolation has not been measured by this control.

The follow-up below makes a same-executable comparison starting from
the same original D: native square-free decomposition followed by exact
reconstruction and root union, against native full factorization followed
by the same reconstruction and union. Keep transient factors and stage
times, and compare complete root inventories, not just factor hashes.
The current separate SQF and union runs should not be added into a claimed
speedup over the release control. Until such evidence and the separate
proof-format work exist, ordinary production factor/checker contracts remain
unchanged; the already verified discriminant acceleration is independent.

## Matched development-build follow-up

The helper now has `native-squarefree-union` and `native-factor-union`
modes. Both start from the authentic same signed D, parsed into the same
one-variable `s` map. Neither uses a factor sidecar, predeflates a threshold,
or reads an operation cache. Each invokes its named native decomposition,
checks the complete signed multiplicity product, and calls the identical
root-union path. Constants retain exact rational content and contribute no
roots; nonconstant blocks may share roots. The input, map, factor count,
multiplicity, degree and coefficient bounds are checked, and the external
wall-time limit is explicitly capped at 300 seconds.

One fresh sequential pair used the same frozen development executable,
CPU10, and a 4 GiB limit per worker:

| Pipeline | Process wall | Decomposition | Signed reconstruction | Root union | Peak RSS | Outcome |
|---|---:|---:|---:|---:|---:|---|
| Native square-free decomposition, then union | 107.719 s | 30.524 s | 2.614 s | 73.873 s | 76,968 KiB | 84 distinct roots |
| Native full factorization, then union | 300.039 s | incomplete | not reported | not reached | 56,656 KiB | timeout |

The full-factor worker's last published phase was `native_decomposition`;
no factor list or root inventory was accepted. The square-free run again
has 46 roots below 4, the exact root 4, and 37 above 4. Every one of its 84
ordered intervals intersects the corresponding interval from the prior
checked sidecar union, using exact rational endpoint comparisons. This
checks consistency between the two completed square-free routes, **not**
a comparison against an unavailable full-factor root inventory.

These are single bounded diagnostics. The development-build timeout does
not negate the successful 177.241-second release factorization above, and
does not establish factorization intractability. It supplies no numerical
speedup claim. The optimized matched comparison below now supplies the
next operand measurement; a production decision still requires the
independent evidence-format work described in the support-basis proposal.
No production proof format or default changed.

Whole process wall time is the comparison metric. Named stage timers exclude
some metadata formatting/writes; the post-parse pipeline timer and process
wall retain that overhead. All eight helper tests passed, including the two
native modes' exact root-set agreement with repeated roots and negative
rational content, shared blocks, invalid maps/caps, constants, tampering,
the 300-second limit and refusal of leftover progress paths. Independent
read-only review found no algebraic or measurement-contract blocker.

The ignored follow-up directory is
`artifacts/research/a377-isolation-matched-01/`. Its frozen executable SHA-256
is `313236752ae3b911d56a7cce3f87bce2ad7ff1df47d872b3f732f2b79ad40e39`,
and source SHA-256 is
`3e61df29f941721a94dc0e4d9e3a302fea31423c7c14a8aa0daa2754116bff54`.
The passing eight-test log is `artifacts/a377-matched-isolation-tests-01.log`,
SHA-256 `830148b6c9ffbd36543410805a2aeabd260d979a3d3b9ee67a4bef41a46f93df`.

| Follow-up artifact | SHA-256 |
|---|---|
| Square-free process report | `81e60b5be9bbfb156a84c6f84db234a9d20204ee04901d10ea1c67f092906f96` |
| Square-free worker report | `a44798fbf92875d95489a7882d637ede3b435436c69bd08cac53a251501a5a0d` |
| Full-factor timeout report | `7bd7b39469320f92c99ee85083f5e787676ceaa134018a36a69a93edc4ddbf8c` |
| Full-factor final progress | `d9678879715abf77fcaf175742461045983a1ffa2bf718b66e6ceaddefe7527e` |
| Input bindings and exact interval comparison | `786575de6f73a3746087c620132075d1b4997ae9db13e330536d3f115b5dad09` |

```sh
profile_univariate_isolation discriminant.json \
  --operand native-squarefree-union --output squarefree.json \
  --cpu 10 --seconds 300
profile_univariate_isolation discriminant.json \
  --operand native-factor-union --output factor.json \
  --cpu 10 --seconds 300
```

## Matched release-build follow-up

The same helper source was compiled with the repository's release profile
(`opt-level=3`, thin LTO, one codegen unit, debug information level 1).
The warm default `target/release` library was reused: only the example was
built, in 6 min 5 s. There were no `RUSTFLAGS` or target-directory overrides;
the existing solver binary and Cargo manifests remained byte-identical.
The build used CPUs 0–3, two jobs, an 8 GiB address-space cap per process,
and a 20-minute outer limit. The source's eight focused tests had passed.

Two fresh processes then ran sequentially on CPU10, each with a 4 GiB
memory limit and a 300-second deadline. Both received the identical raw
degree-2169 candidate and map `[s]`, without predeflation, saved factors,
or an operation cache. Both used the same signed multiplicity-product
check and exact full-real root-union routine.

| Release pipeline | Process wall | Decomposition | Signed reconstruction | Root union | Peak RSS |
|---|---:|---:|---:|---:|---:|
| Native square-free decomposition | 92.063 s | 16.608 s | 2.544 s | 72.618 s | 71,724 KiB |
| Native full factorization | 185.827 s | 172.125 s | 3.346 s | 10.104 s | 69,828 KiB |

Both processes completed successfully without truncated output. Their
84 corresponding ordered isolating intervals intersect by exact rational
cross multiplication. Each inventory has 46 roots strictly below 4, one
exactly at 4 and 37 strictly above 4; no interval straddles 4. The release
square-free intervals also agree with both earlier completed square-free
inventories. Independent read-only replay checked the hashes, limits,
signed-reconstruction status and interval comparisons.

Full factorization splits the degree-881 block into factors of degrees
139, 250 and 492, each with multiplicity two. This explains a concrete
tradeoff in this run: avoiding that decomposition saves work, but the
larger square-free block costs more to isolate. The release completion
also confirms that the development-build timeout cannot establish full
factorization intractability. This is one sequential operand pair, not a
repeated benchmark or whole-original completion; no numerical speedup
claim or production policy change follows.

Evidence is under `artifacts/research/a377-isolation-matched-release-01/`.
The frozen source is the same `3e61df29…` recorded above, from commit
`2e9b26467f4c4bc8ae20b7113ad8c6406627fcd0`. The manifest records the exact
commands, build profile, dependency revision and input bindings.

| Release artifact | SHA-256 |
|---|---|
| Frozen helper | `6ed81cc04132676c0c9a457a9605eefe52123c92e9b5d00ccb4ed01d86da9797` |
| Build log | `be91493f241c0782ef250ad8d59c7b96d909f66e9bcca449168c2b255a85ca74` |
| Build/runtime manifest | `e8846d6bdd82ed611b0a7072353af8612b56564b7d0bc1472290a2f2aa5419c7` |
| Square-free process report | `3e34dd241d2d40a27991eee08966e2d142f3c32800d971cca70d6756e44941b8` |
| Square-free worker report | `19c8d6feef6393e25a4abb8158351f685276bf5ba1ce3227be948dfebd1f5372` |
| Full-factor process report | `40e8e5302bf85e0b4e1c3e3ac4cc8c006935abbace553737b548f134169687c8` |
| Full-factor worker report | `e5046676595d1b964d3d9338e0afe26b71a343ac0556e27e73290c17d6fbac01` |
| Exact paired comparison | `1fbc3a2a58ed472725be384b2a4de2506f1a57c97fa958ddfdaa378b282928fd` |

## Reproduction and bindings

Ignored evidence is under `artifacts/research/a377-isolation-01/`; the full
factor sidecar is under `artifacts/research/a377-deflation-02/`. Large operand
and interval artifacts are not committed.

| Artifact | SHA-256 |
|---|---|
| Signed D candidate | `0c457cfce6ed2df4545054b7ba79b76e62123e284a665ea39db12bc4124c09c2` |
| Checked complete factor sidecar | `e83563f9721a38ebf3de7fa9ff95ff2c28adb0e0056a6151d81540bcd5f20e5c` |
| Degree-881 block, native canonical spelling | `e20627b62cef2c89436f26d01bfc9f03b36404ecf6b4313995ff5b7b326e700e` |
| Frozen helper executable | `377f57c38bf0baf89dfb157b6fd67cf72abf0514f4c18863fba29b139bf696ce` |
| Degree-881 supervisor report | `79316f588bd5e87d09d37a65161de91e2951af2e0332cbc01a799f176324beb0` |
| Degree-881 worker report | `216adf694d109953b0f5977d0f300d09d6adde214f251ca73ea2cfd07c6de5cc` |
| Exact interval array | `1f30f610146348bd18a88bb36502a9a7fd2fd60b14ef0212f41a842e07776518` |
| Complete-union supervisor report | `7ebf07e2a3c2652956187bd14d35b742194b53e56ddfaf657236ec7d07fcc6ea` |
| Complete-union worker report | `4f61ff75242777dd35dc2b6257a8ecac8264e77d85db0ad5496961f47c91361d` |
| Complete-union exact interval array | `0079c78fb04625b2abadd4abb9cce6bb5c2fbb87a85eb9dfc9d9896eb90ea78e` |
| Seven-test combined log | `b0f2d083b6b14e4a2431ed125f289b448767223a20dc86956a178bc0e77e6b66` |
| Full-D factor capture | `46d5122f1f6320fcf98165b0a0382150071264923ff1eb259f83f1ed52ca744c` |
| Frozen release factor profiler | `51a26df001ba8c5c611d488ac905da7a1afa7bd3c18bb6290dab388c998c18e2` |
| 300-second factor process report | `a46b124d56c52d1c57e8944406633bebae6455a7279c435de88e4606fc23fbdc` |

The helper was compiled against production `d50794f`; its own source SHA-256
is `20ed334549f108db8fdf521e9a2517e0a72bb60f74b40c81e188666354b5f825`.
Use fresh output paths; all runs require the independently verified candidate
and exact factor sidecar:

```sh
profile_univariate_isolation discriminant.json \
  --factors square-free-residual.factors.json \
  --operand factor-block --factor-index 4 \
  --output block881.json --cpu 10 --seconds 180

profile_univariate_isolation discriminant.json \
  --factors square-free-residual.factors.json \
  --operand factor-union \
  --output factor-union.json --cpu 10 --seconds 180
```

The full-factor control is retained under
`artifacts/research/a377-discriminant-factor-02/`; it uses the original
capture from `a377-discriminant-factor-01/` and the frozen release profiler
from source `bbb701b`. Its successful 300-second-budget run supersedes any
inference of intractability from the preserved 60-second timeout.

The [first-axis support-basis proposal](BASE_UNIVARIATE_BASIS_PROPOSAL.md)
still requires a separate production design and independent checker work.
The completed union establishes that this operand's roots are computationally
accessible without a completed irreducible factorization. It does not prove
that the full original problem will complete. Exact block reconstruction alone neither bypasses
the current irreducible-registry checker nor proves original CAD coverage.
