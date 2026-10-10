# Bounded affine prefix evaluation

An A414 CPU profile attributed 9.32% self / 16.1% inclusive samples to
`algebra::prefix_value::integer_value`. Original coordinate constraints and
`s - 4` are small guards that do not need its allocated power tables.

The narrow shortcut accepts an integer `a*x_i + b` with at most two terms and
at most 16 mapped axes. It runs **after** the existing complete-support and
prefix-length checks. The original variable index, 4,096-bit coefficient cap,
positive denominator check, 16,384-bit degree-one power bound, and scratch
estimate remain enforced. It performs exact Symbolica rational arithmetic;
the coordinate case simply clones the assigned rational. Any decline uses
the unchanged integer evaluator and then the ordinary Symbolica fallback.
There is no domain, sign, or positivity assumption.

## Measurement

The retained A414 prefix is `[5, -393217, 1, 47/67, 252831860/643]` in reduced
map `[s,t,x1,x2,x3,x4,x7]` after the original `x8` anchor. The assigned original
guards `[s-4,x1,x2,x3]` evaluate to `[1,1,47/67,252831860/643]`.
The helper compares the actual production source with the retained old
evaluator and independently checks both against Symbolica substitution.

Each repetition evaluates four guards 100,000 times. Five repetitions alternate
which implementation runs first, with black-boxed inputs and outputs. CPU0
was pinned, each invocation had a 60-second / 2-GiB address-space ceiling,
and the helper was compiled with `-C opt-level=3` using dev-profile dependencies.
Other pinned research workers were running. These are evaluator measurements,
not an end-to-end solver benchmark.

| Batch | Old median | Production median | Production / old |
|---|---:|---:|---:|
| Four original affine guards | 74.560 ms | 20.461 ms | 0.274 |
| Four synthetic fallback controls at the same prefix | 117.545 ms | 116.702 ms | 0.993 |

The controls are `s^2-4*s+1`, `s*t-3`, `x1*x2+x3`, and `s/7-4/3`; they are not
claimed captured guards. Their difference is too small to support a gain.
The earlier prospective implementation and measurements remain separately
preserved; the table above uses the final production source with one support
validation, including on declined inputs.

Evidence is under `artifacts/profile/affine-prefix-01/`:

- `a414-after.json`: SHA256 `01d897eaaafa56420a96721a2daeb0df881df3acd33b61f60327a5ff390830c3`.
- `fallback-after.json`: SHA256 `007c27162d1ed9caa9b3779b3604447d53d2a8b286f7d28a52b9bc8a93b826ef`.
- Frozen helper: SHA256 `b43b2bb17f04845f9077506ffbeb329345dc350a443afd022ece0c8ba794b06b`.
- Production source: SHA256 `fabd3421bab058fefc4bce8a248a91952783c80031ce6cf1b3edcfcf40f26c6b`.
- Original guard input: SHA256 `b3762b6dc50c0765065d9de40230d953014c5aa40a36fb414dcff2c4dded77e8`;
  it binds the original worker input and saved progress hashes.

## Reproduction and checks

Build with `nix develop -c cargo rustc --example profile_affine_prefix -- -C opt-level=3`.
Its JSON input has `variables`, rational-string `prefix`, string `guards`, and
`provenance` fields. Invoke the executable with that input, `--output report.json`,
`--iterations 100000 --repeats 5` under the desired CPU/memory/wall supervisor.
The expressions and prefix above permit reconstruction without a corpus or proof.

Eight library tests pass; the helper's ten tests also include all eight production
tests. The new tests compare 1,000 reproducible affine families against the old
integer path and Symbolica, assert actual shortcut admission, and cover negative
and zero values, rational coefficients, inactive/later axes, oversized prefixes,
large coefficients/coordinates, and the independent 17-axis decline. The helper
also checks every measured value before timing. Failed early helper-compilation
logs remain retained; final logs are `artifacts/affine-prefix-focused-tests-02.log`
and `artifacts/affine-prefix-helper-tests-04.log`.
