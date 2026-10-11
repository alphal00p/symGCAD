# Final Gluza33 factorization investigation

This records the final bounded push requested after the 303/331 checkpoint.
Positive Gluza33 has fixed kinematics. Its negative branch is already solved;
none of the isolated factor probes below certifies the positive decomposition.

## A structural obstruction in the selected main variable

The last 2,867-term source reaches ordinary native factorization after the
primitive-linear, sum, scaling/translation and shear paths decline. The
source-bound replay times out at 30.038 s / 13,104 KiB. Although the replay
helper predates diagonal scaling, both affine implementations reject this
source at the same 2,048-term admission gate, and the shear source is identical.
This establishes the preprocessing route, not an identical random retry trace.

An exact square-free decomposition reconstructs the complete signed source
in 0.083 s process time. Its unique three-variable component has 1,731 terms,
degrees `[18,14,10]` in `(x2,x1,x7)`, and 56-bit coefficients. The full variable
map and all scalar units/multiplicities are retained. The native trace chooses
x2 as main variable and attempts the small-leading-coefficient Wang path.

The component's leading coefficient is exactly

```text
9765625*x1^2*(5*x1+7)*(5*x1^2+73*x1+166)*(x1+x7)^2.
```

Its factors reconstruct in 0.004694 s. The current sufficient pairwise-coprime
image test cannot pass at **any integer point**: for even x1, both x1 and the
quadratic factor are even; for odd x1, both `5*x1+7` and the quadratic factor
are even. Zero or unit images fail the other necessary gates. This proves an
obstruction to that sampling rule, not irreducibility or impossibility of
factorization. More primes or larger source budgets cannot repair this axis.

The captured trace rejects all twelve small-LC candidates and falls back.
There are 175 sampling entries and 524 bivariate-factor starts in the bounded
trace; not every retry carries a full polynomial fingerprint. A separate
forced-univariate control on the exact component also times out at
60.039 s / 11,504 KiB. No factor or partial profiler output is inserted into
the solver's operation cache.

## Bounded alternative-axis experiment

The optional research patch tests two general main-variable rankings:
lowest degree, and second-smallest leading layer. Admission requires a
256–8,192-term square-free component with exactly three active variables,
degrees 3–32 and coefficient height at most 8,192 bits. The policies do not
match an input name, hash or particular coordinate. They permute active
indices without changing the source variable map; subsequent fallback uses
the chosen order, with all original algebraic acceptance checks intact.

The default production backend remains unchanged. The separate patch is
disabled unless its research feature and named environment option are enabled.
Pure policy tests cover inactive coordinates, stable ties, ranking choices,
invalid inputs and resource admission.

The successful axis changes the native fallback's main variable. All twelve
optional Wang image candidates still fail; the result is not a successful
Wang first-image assignment.

| Input and policy | Factor time | Worker wall | Peak RSS KiB | Result |
| --- | ---: | ---: | ---: | --- |
| 1,731-term component, default axis | — | 30.032942 s | 9,648 | Timeout |
| Same component, second-leading | — | 60.020913 s | 9,680 | Timeout |
| Same component, lowest-degree | 0.280055 s | 0.304347 s | 9,556 | Signed product verified |
| Full 2,867-term source, lowest-degree | 1.951155 s | 1.978713 s | 9,720 | Signed product verified |

These are single bounded diagnostic observations, with tracing, one CPU and
4 GiB, not benchmark medians or full-solve speedup ratios. The default control
uses the same binary and component as the two alternative-axis controls;
it uses CPU 9 while the other controls use CPU 8. The native backend's internal
sampling is randomized. The selected component produces 36- and 814-term
factors, both with multiplicity one. The original source reconstructs from
factors with term counts `[1,3,3,36,814]` and multiplicities `[1,1,2,1,1]`.
Independent signed replay costs 0.000367 and 0.000953 seconds respectively.
No coordinate preconditioning was applied to either source.

An independent generic control uses
`-3/7*(w+2)^2*((x+y+z^2)^7+x*y+z)*((x+y+z^2)^7+x*z+y+1)`
with full variable map `[x,unused,y,z,w]`. Both policies act on its 306-term
square-free component and reconstruct the complete signed input, retaining
the inactive coordinate, rational unit, repeated parameter-only factor and
multiplicities. Three pure policy regressions pass. A homogeneous two-cell
solve passes independent verification with the selected backend. The campaign
harness accepts 7,200 seconds and rejects 7,200.1 and NaN; its enforcement
logic is unchanged.

## Reproducing the optional backend

The [main-axis patch](../patches/experimental/symbolica-bounded-main-axis.patch)
applies after the existing bounded-Wang patch to pinned Symbolica
`75f8350094b90254ee71dc2a391fde0d14b0204a`. Clean patch replay reproduces the
compiled backend byte for byte. Use an isolated symGCAD worktree so the
ordinary dependency and build stay unchanged:

```sh
git worktree add --detach .reference/main-axis HEAD
cd .reference/main-axis
scripts/bootstrap-deps.sh
git -C DO_NOT_PUSH_REFERENCES/symbolica apply \
  ../../patches/experimental/symbolica-bounded-small-lc-wang.patch
git -C DO_NOT_PUSH_REFERENCES/symbolica apply \
  ../../patches/experimental/symbolica-bounded-main-axis.patch
scripts/use-local-symbolica.sh "$PWD/Cargo.toml" "$PWD/DO_NOT_PUSH_REFERENCES/symbolica"
nix develop
cargo metadata --format-version 1 > local-backend-metadata.json
cargo tree -i symbolica
cargo build --locked -j2 --features symbolica/research_small_lcoeff_wang \
  --bin symgcad --example hard_probe --example profile_affine_factorization
export SYMBOLICA_RESEARCH_SMALL_LC_WANG=1
export SYMBOLICA_RESEARCH_FACTOR_MAIN_AXIS=lowest-degree
```

Supply the normal Symbolica license through the environment. The experiment
used the development profile with dependencies at opt-level 2, not a release
benchmark. Use `target/debug/symgcad` for subsequent solve/verify commands.
The explicit local selection is required after the public-Cargo dependency
integration: patching an ignored checkout alone does not change Cargo's public
Git pins. Metadata resolution refreshes only this isolated lockfile; confirm
all three package paths point to that checkout before building. Add `--offline`
when all registry dependencies are already cached. Avoid `cargo update`: it
can also reselect transitive dependency versions and weaken the matched control.
Unset `SYMBOLICA_RESEARCH_WANG_TRACE` for solver timing. The feature and named
environment policy are both required; ordinary production builds are unchanged.
The optional test-only evaluator guard in `patches/experimental/` is needed
only when running Symbolica's library tests without native code generation.
Three policy tests can also be run independently with `rustc --edition=2024
--test DO_NOT_PUSH_REFERENCES/symbolica/src/poly/research_main_axis.rs -o
target/main-axis-tests` followed by `target/main-axis-tests`.

## Final original-problem campaign

The single final campaign starts from a fresh, byte-checked copy of all
27,897 inherited operations (305,478,455 bytes), with no profiler factors
injected. It preserves the original problem, variable order, scale and solver
options. Only the optional backend policy, checkpoint location and deadline
change. One CPU and 50 GiB are available, with 6,600 seconds for solving and
up to 600 seconds for independent verification within a shared 7,200-second
campaign. The worker times out after **6,603.858274935 seconds**, with
**2,010,632 KiB peak RSS** (1,963.508 MiB). Total campaign wall time is
6,604.300292846 seconds; worker user/system CPU times are 6,402.654628 and
24.558223 seconds. The small solve-deadline overshoot is included in these
observed timings. No result is exported and independent verification does not
run. Positive Gluza33 remains unresolved; the original aggregate stays 303/331.

The original worker clears the former 2,867-term factor in 1.996107155 seconds,
then continues through resultants. The terminal cache contains **32,456 records,
4,559 new**: 4,013 factorizations, 40 discriminants and 506 resultants.
Every inherited file is checked for unchanged bytes, and new
operation keys and output hashes are independently recomputed. This checks
artifact integrity, not the mathematical result of every algebra operation.
No standalone profiler factors are inserted into the cache.

The last full progress snapshot is at 879.837742911 seconds, prefix
`[2,6/13]`, with 124,218 speculative nodes and 17,355 refinements. Later
operation captures and new cache files show continued work; these counters
are neither current cell counts nor a completion estimate. The last captured
call is a resultant in `x7`, key
`c395fcc77e3ffcf7095dc236dbab6ce7a9e4763e33032358dd7d23ae835838e2`,
input hash `8ab3ce7689cc50f11ed077a54f6db59dd943877199b9778a11b4f123e7478068`.
The capture has no elapsed-call time, so the campaign duration must not be
attributed to that one operation. All campaign processes have exited.

## Evidence bindings

The route diagnostics are under `artifacts/research/gluza33-factor-route-01/`.

| Record | SHA-256 |
| --- | --- |
| Original capture | `eef02c540c1e3c33c19533df6062b5cf7e3bd991003d1eef99f817fde97f5ead` |
| Original operand | `879b4cb7d753389c91aca8553c0784da8212178f35da56bfa9944168ce3b6845` |
| Signed square-free result | `e70bc22b893848de7d780ebacbff3f498e8792ccb667fa44c451859c2d1ee400` |
| Exact derived component capture | `bd5b4170e055a2a2bcd3239e0625c71f66f31213fcca3dbdf77c32cbf76ab861` |
| Untruncated route trace | `17ff2fdc596519c76a3077e8c55d48498e3ca0797e132cae86a9b8480ab2d158` |
| Main-axis patch | `6441cf2ca55d8b4fc1220a5b6e9fa5f35b416fa68f3408a8792fd2025a9c3023` |
| Signed original factor result | `5eb140e8824afd67a1727a9db8eea83587fac9accf7fd08f0133ed12665d05ed` |
| Control report | `bd68eabe6d00f94bd47cbdd9ed013d8e011395cc097395f88de2cadfc4e01927` |
| Frozen solver | `4d4af2f11744ca267723958944987d2e4d608e293074b24d9d7b669ec798198d` |
| Frozen campaign harness | `0a1790d3a2ac2f3344d3a04fa1f3eb87127e18fc1a239c6987847f314031ef88` |
| Final worker record | `1a7519f7dd970314063d90c2623b086720f82c72a05d807637d58e11f7a34e63` |
| Final campaign provenance | `3cc6a9c5a7ec48db202b928edf651ef10b6d9a76275e956af9e309ae289d65ba` |
| Final cache manifest | `29e4f6a1bd7e25ff69d8641b30bd532155f5e393f8648ec0e25675c86c7402f7` |
| Final integrity audit | `50e02619cae6953de858c23bf8e0fb68578e4acf2400c36afe4dbe9109bece60` |
| Final active-operation capture | `c93e651825a53dba90da8e51fdfbde4095a5985e7ef4fec9b08e182cfee1f72d` |

The final campaign files are under
`artifacts/research/gluza33-final-main-axis-campaign-01/`; the audit completed
at 2026-10-11 22:43:44 UTC. Large caches and result artifacts remain outside
Git; the public record preserves their bindings and scope.
