# Exact quadratic opening: conditional validation

The generic `quadratic_ladder` engine completes the **whole crossed A446
chamber t<4**, with the original F, masses, constraints and parameter groups
retained. It produces five cells in 0.022253026 solver seconds, followed by
successful fresh independent verification. The unrestricted original remains
unresolved: t>4 still needs a decomposition, and a composition proof must
record the t=4 exceptional section. Original coverage remains 303/331.

This is an implementation of the [coefficient theorem](POSITIVE_ENDPOINT_QUADRATIC_LADDER_PLAN.md),
not a topology-specific formula in the solver. It uses degrees, exact signed
identities and six strict whole-domain certificates. B uses a weighted-square
certificate, the quadratic constant coefficient uses a PSD-component
certificate, and four premises use shifted positive coefficients. The
independently verified base has one cell. Its five restored rows have signs
`[+,+,+,-,+]`; the two middle sectors share the original normalized F as their
x8 boundary. Discriminant, quadratic and original F seams are all retained in
the explicit exceptional set.

## Reproduction and scope

Start from the generated symbolic fixture
`0152_finiteAx12_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml`.
Keep its split expression, variables, exact `mt2=1,mz2=5/18`, groups and
constraints. Add only `t < 4` to its mathematical domain. Select:

```toml
[solver]
engine = "quadratic_ladder"
ordering = "given"
order = ["s", "t", "x1", "x3", "x5", "x7", "x4", "x2", "x8"]
merge = "none"
positivity = true
weighted_square_positivity = true
quadratic_component_positivity = true
```

Use ordinary `symgcad solve` and `symgcad verify` with 30 seconds, 2 GiB and
one worker. No homogeneous rescaling or added Feynman-coordinate positivity
premise is used. The original, conditional input and unresolved strict
complement are preserved together under `artifacts/research/full-f-ladder-01`.
An independent input/theorem audit checked their exact relation.

This run used a frozen debug CLI on CPU3. Public solve wall time was 0.07 s,
peak process RSS 12,312 KiB; fresh verification took 0.02 s and 12,360 KiB.
These single-run diagnostic measurements are outside the main benchmark.
Earlier local-engine diagnostics had different orders and transformations,
so they do not provide a matched speed ratio for this change.

The unrelated [small example](../examples/quadratic_ladder.toml) also completed
and verified five cells (0.002791398 solver seconds). Eight ladder regressions
cover unrelated rational families, exact common refinement against the
baseline, disconnected and empty bases, group/order constraints, malformed
or missing premises, proof mutations, resource limits and the unused smaller
discriminant root crossing zero. Sample checks supplement the global theorem;
they do not replace its branch inventory.

## Supplemental randomized sanity check

Using seed **20261010**, the separate `sanity_sample` helper generated two
fresh rational interior points in every exported cell, distinct from the CAD
samples. It recomputed algebraic bounds at each new coordinate prefix and
checked strict membership exactly. Its independent evaluator then evaluated
the unchanged original F and original conditional constraints, with the
specified exact masses. All **5 cells / 10 points**, **10 F evaluations** and
**90 constraint checks** passed, with **zero failures and zero skipped cells**.
The report counts 190 individual polynomial evaluations including both sides
of constraints. The worker took 0.297 s, excluded from solve timings. This is
a supplemental sanity check, not a coverage proof.

```sh
cargo build --locked --example sanity_sample
target/debug/examples/sanity_sample \
  --result artifacts/research/full-f-ladder-01/result.json \
  --original artifacts/research/full-f-ladder-01/conditional.toml \
  --output artifacts/research/full-f-ladder-01/supplemental-sanity.json \
  --seed 20261010 --points-per-cell 2 --seconds 30 --memory-mib 2048
```

## Artifact hashes

| Artifact | SHA256 |
|---|---|
| Original generated input | `ca89810d876b2bcde20ffc86b18928380eb4b26a418b6ab0f1accbb382bf1e3d` |
| Conditional input | `acf75571b9fe14cbf4c7deaa95556748f6589248aefe1683499f0e257ee3a48b` |
| Complete result | `5da6502fec4a9905ba08d853ae3ae0f2c6412a407cf07c74ee9f204ed4e36399` |
| Frozen CLI | `20f85d3b45b84074d54594017f493982cfe7f7e9b6d912a9c566f60440f05c5f` |
| Independent input/theorem audit | `61bf63856fe64ee271ab3a6ac2b66a9daf5827c0e8894f5e9bc4895dd409d7ce` |
| Supplemental report | `ded2deb3ba7e250da4bde043e618f6bb17645bba20f7056b6bf4624ed343b3f5` |
| Supplemental executable | `18f188ca70efd243869a5ab8b7d3a4942c7963fbadf71970e8bbeee8321577da` |
