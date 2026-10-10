# Tropical sampling and graph structure: a plan for exact CAD

Research review, 10 October 2026. The original graph is assumed available from
the caller. This note proposes experiments; it introduces no solver change,
new completion, or performance claim.

The most promising first step is a bounded graph/support seed generator for
the existing order search, followed by graph-derived **exact identity
proposals** for the existing certificate kernels. A tropical approximation,
numerical sampler, or integral-preserving contour deformation cannot replace
the real zero set that this solver must decompose.

## What the primary literature actually supplies

| Primary source | Relevant result and its boundary |
|---|---|
| Borinsky, [*Tropical Monte Carlo quadrature for Feynman integrals*](https://arxiv.org/html/2008.12310), §§6.1, 7.1–7.2, Definition 28, Proposition 29, Algorithm 4, Theorem 32 | Generalized-permutahedron sampling uses a subset recurrence rather than enumerating every edge permutation. Its preprocessing still costs exponentially many subsets: the stated bound is O(E 2^E). Graph loop numbers and mass–momentum spanning subsets determine the relevant valuations under the stated assumptions. This suggests bounded order priors, not a polynomial-time CAD algorithm. |
| Borinsky, Munch, Tellander, [*Tropical Feynman integration in the Minkowski regime*](https://arxiv.org/html/2302.08955v2), §§2.2–2.3, 3.1, 3.3–3.4 | Theorem 3.6 extends the **support** result to generic Minkowski kinematics. It does not assert positivity or absence of real zeros. Theorem 3.1 requires complete nonvanishing for its two-sided tropical bound. The physical integration method deforms into complex space; Assumption 3.2 is not a general proof of that bound after deformation. Exceptional kinematics require further care: Conjecture 3.7 and the inclusion of Theorem 3.9 must not be promoted to unconditional support equality. |
| Borinsky, Fraaije, [*Tropical sampling from Feynman measures*](https://arxiv.org/html/2504.09613v1), §§1, 2.2–2.5, 3.1 | The sampling measure assumes effectively Euclidean positive propagators and a finite integral. Tropical Schwinger-parameter samples are combined with gamma/Gaussian sampling and a matrix factorization to sample loop momenta. Its Rust interface is useful design precedent for graph/routing metadata. Finite samples and positive Euclidean weights do not certify a signed symbolic Minkowski domain. |
| Schultka, [*Toric geometry and regularization of Feynman integrals*](https://arxiv.org/pdf/1806.01086), §4, Theorem 4.15, Proposition 4.20 | Graph polytopes and toric charts organize sector decompositions; Hepp sectors arise from complete flags. These provide explicit coordinate maps and boundary structure. A chart used by CAD still needs exact inverse/domain/order/coverage proofs; a finite list of selected charts is only a partial region. |
| Panzer, [*Hepp's bound for Feynman graphs and matroids*](https://ems.press/content/serial-article-files/39461), §§2–4, 6 | Matroid invariance, flag recurrences and polyhedral descriptions compress a tropical integral invariant. They motivate grouping equivalent order candidates. Equality of this invariant, or of the graph matroid, does not identify the full externally labelled second Symanzik polynomial. |
| Arkani-Hamed, Hillman, Mizera, [*Feynman Polytopes and the Tropical Geometry of UV and IR Divergences*](https://arxiv.org/html/2202.12296), §§IV–VIII | Subgraph and scaleless-cograph facets organize UV/IR scaling and leading divergences. This is information about boundary asymptotics. It does not classify all interior physical zeros or provide a replacement for the discriminants needed to follow them. |

Two formulas make the graph features concrete. With the convention used in
the Borinsky papers, the first Symanzik polynomial is
`U = sum_T product_(e not in T) x_e`. For generic applicable kinematics,
`z_U(S)=h1(S)` and `z_F(S)=h1(S)+delta_mm(S)`, where the latter indicator is
defined by the cograph's second polynomial vanishing identically. If integral
weights are supplied, the sampling convergence weight is
`r(S)=sum_(e in S) nu_e - D*h1(S)/2 - omega(G)*delta_mm(S)`.
The subset recurrence is
`J(S)=sum_(e in S) J(S\{e})/r(S\{e})`, with the empty-set conventions of
Definition 28. Nonpositive proper-subset denominator weights cannot define
the required sampling probabilities. These weights concern the integral, not the number of CAD
cells. [Borinsky, §§6.1, 7.2](https://arxiv.org/html/2008.12310).

## Code and local input audit

The inspected upstream revisions are pinned so this assessment is reproducible:

- [tropical-feynman-quadrature, a4c51463](https://github.com/michibo/tropical-feynman-quadrature/tree/a4c51463fe4d6711113008eaa2d84200e07bb5d4):
  `subgraph_table.hpp` allocates `1ULL << E` entries; `symanzik_polynomials.hpp`
  evaluates through a reduced Laplacian factorization. Its floating-point
  kinematic tests are not exact certificate checks.
- [momtrop, 302415aa, preprocessing.rs](https://github.com/alphal00p/momtrop/blob/302415aa23caf8e99e1dbb74a5134bec6c833bae/src/preprocessing.rs):
  `generate_stage_1` allocates `2^E` entries; mass/momentum spanning checks,
  loop counts and `recursive_fill_j_function` implement the subset machinery.
  This is a useful Rust API model, but should not become an unconditional
  exponential preprocessing dependency of every solve.
- [feyntrop, ad0d6832](https://github.com/michibo/feyntrop/tree/ad0d683274977836f5c2ab3e71496ba307413d63):
  graph, Laplacian and tropical-sampling components are separated under
  `srcs/`. Reusing ideas does not require linking a floating-point integrator
  into the exact proof kernel.

The local upstream `tests/ZZ_integrals` contains 276 `finite*.py` files.
264 carry literal sparse `U/F` arrays and original `xN` labels; 12 one-loop
E/F scripts instead carry propagator routings, external momenta and Gram
replacement rules. None contains explicit `internal_lines`/`external_lines`
connectivity. In particular, A377, A414 and A446 have the arrays but no
original edge-endpoint table. The imported `Problem` in `src/config.rs`
does not retain a graph. A polynomial support or an integral-family number
must not be silently treated as the original labelled graph.

The smallest new import is a data-only sidecar binding:

1. Original source hash and normalized mathematical-input hash.
2. Stable original edge IDs, their `xN` names, endpoints and squared masses;
   active edges and any pinching/contraction history explicitly recorded.
3. External momentum injection at vertices, exact Gram/replacement rules and
   crossing convention. Dimension and propagator powers are optional inputs
   for convergence features, never guessed from a CAD polynomial.
4. Exact reconstruction of the supplied `U` and original `F`, with a stated
   sign convention and checked scalar. Every original domain constraint,
   split, fixed order and variable group stays attached to the original input.

Use a bounded Rust data parser; do not execute fixture Python. Check maps,
momentum conservation and polynomial identities with Symbolica. A reconstruction
failure rejects the metadata. Original graph connectivity, monomial
co-occurrence, and the solver's whole-polynomial correlative graph are three
different objects. A full-support irreducible polynomial makes the last one
a clique; low graph treewidth alone authorizes no projection omission.
See [the generic CAD literature review](LITERATURE_GENERIC_GCAD.md).

## Ranked experiments and exact obligations

### 1. Legal order seeds from graph and support features

**Recommended first implementation; heuristic only.** Start with bridge and
articulation structure, biconnected components, cycle rank of edge subsets,
the number of frontier vertices in an elimination order, and separation of
external momentum/mass labels. Produce a small deterministic portfolio:
minimum-frontier contraction order, its reverse, a nested connected-subgraph
order, and support-valuation orders. Reject duplicates and illegal orders.
Compare both directions because public lifting order and projection order
are reversed in `src/ordering.rs`.

There is an immediate route without reconstructing connectivity. For any
stored polynomial support, compute
`v_P(S)=min_(alpha in support(P)) sum_(e in S) alpha_e` exactly. The existing
`U` arrays supply the matroid subset profile; `F` supplies the formal support
profile after exact parameter replacement. A377 and A414 each have six active
Feynman variables, so only 64 subsets; A446 has seven, so 128. This small
research feature extractor can precede the full sidecar. Generic formal
support may shrink on a kinematic locus, so it remains an order heuristic
unless coefficient-zero strata are separately certified.

A concrete first helper can emit **support/matroid-informed** candidates
without a learned score or a graph-certificate claim. Let `S` be the axes
already selected for projection. For each legal next axis `e`, record:

- `Delta_U=v_U(S+e)-v_U(S)` and the corresponding `Delta_F`;
- the collective exponent width `max_alpha sum_(i in S+e) alpha_i-v_P(S+e)`;
- coefficient-block interaction `C_P(S+e)=sum_b n_b^2`, grouping original
  monomials by their exponent vector on `S+e`, with `n_b` the number of
  remaining monomials in that coefficient block.

Try three explicitly separate greedy seeds: lexicographically smallest
`(Delta_U,Delta_F,C_F,C_U)`; largest first two increments with the same
interaction tie-break; and smallest `(C_F,C_U,width_F,width_U)`. Resolve
remaining ties by original variable position and reverse the final projection
sequence to obtain lifting order. Constrain every action by the original
groups; a fixed order yields no alternative. Also report each complete
feature trace. Delaying versus exposing loop closure is deliberately an
ablation, not a claim that either direction is universally better.

These descriptors measure joint original-support structure, whereas current
Brown/degree seeds use degree/term statistics and whole-polynomial sparsity
uses variable co-occurrence. `C_P` is only a cheap potential coefficient
interaction count: it does not model cancellation or the generated projection
basis. No weighted sum, trained coefficient or asserted CAD complexity bound
is needed for the first experiment. Reuse the data-only literal-array parser
in `examples/zz_rayleigh_guides.rs`; explicitly discard its dummy `eps,U,F`
axes only after checking they have zero exponents in the graph polynomials.

Use these only as extra seeds to the existing bounded MCTS. Preserve a
nonempty original `solver.order` exactly and keep outer kinematic groups
outer. Bind graph hash and seed-policy version into the search context.
No seed may remove a polynomial, prune a mathematical branch, or manufacture
a completed winner. The existing exact solve and independent verification
remain mandatory.

For larger graphs cap subset enumeration before allocation, for example at
4,096 subsets, and otherwise use bounded connected subsets and graph cuts.
This is a proposed engineering cap, not a claim from the papers. If actual
`D,nu` and convergence premises are available, a separate seed arm may use
the tropical recurrence. Without them use clearly labelled combinatorial
scores, not an invented Feynman probability measure. Graph symmetries may
deduplicate candidates only after masses, external labels, full `F`, domain
and order obligations are checked.

### 2. Graph minors as proposals for exact certificates

**Recommended second, bounded by existing kernels.** Keep determinant/minor
expressions long enough to propose short identities before expanding large
elimination polynomials. Candidate sources include deletion/contraction,
small separators, and matrix-minor identities. For a non-loop, non-bridge
edge, check the concrete convention `U_G=x_e U_(G\e)+U_(G/e)` exactly;
treat loops and bridges separately. This can expose coefficient factors or
useful sums of variables. It does not imply the same simple recurrence for
every massive `F` term.

Prefer certificates already consumable by production: exact factor/product
reconstruction, positive monomial factors, or a pair identity
`p*L+q*M=k*H^2` with `k>0` and strict domain proof for `H`. Preserve individual
delineability, positivity premises and every required ancestor guard.
Discovery can try a bounded number of small minors and decline. Replay must
derive its conclusion solely from the stored exact identity and original
domain, not from a trusted graph name or a numerical determinant.

A compact determinant evaluator is also worth testing for original-polynomial
evaluation and coefficient discovery. It must use exact arithmetic, prove
every denominator/pivot nonzero when division is used, and compare against
the original expanded expression. It is not automatically a faster method
for the large *projected* polynomials. No new determinant data structure is
justified until a captured operation benefits.

### 3. Boundary-informed diagnostic samples and local chart proposals

Use subset scaling profiles to propose exact rational points near boundary
regions, and evaluate the original signed polynomial. Opposite-sign or zero
points can reject an overstrong proposed certificate; their absence proves
nothing. Every sampled point must satisfy the original domain exactly.
This is useful for counterexamples and certificate discovery, outside
benchmark solve time unless deliberately included as preprocessing.

For an explicit sector `0<x_pi1<...<x_piE`, the map
`x_piE=r`, `x_pii=r*product_(j=i..E-1) u_j`, with `r>0` and `0<u_j<1`, has
an exact positive inverse using adjacent ratios. A future proof wrapper
could strip a proven positive monomial from the transformed *full* polynomial.
It must retain the residual polynomial, all transformed constraints and every
zero sheet. Replacing the residual by its leading boundary term is invalid.

This is a lower priority than order seeds: the full permutation atlas has
factorial size, charts may violate the required cylindrical order, and a
handful of solved charts gives no full-original completion. Required proof
work includes inverse identities, complete domain coverage with explicit
sector seams, root-bound transport, overlaps/disjointness, and original
exceptional sets. The toric literature supplies candidate maps; it does not
perform these CAD-specific checks for us.

## Consequences for the current hard cases

| Case | Existing evidence | Most useful next graph experiment |
|---|---|---|
| A377 | Two-parameter captured pair resultants respond to exact binary segmentation, but the warm continuation reaches an ordinary resultant outside its static applicability. See [the retained diagnostic](A377_BINARY_RESULTANT_DIAGNOSTIC.md). | Rank legal six-axis orders by support/subgraph profiles and measure whether they reduce projected degree/term/bit growth. Try minor-derived factoring proposals only on an exactly captured operand. Do not infer the pending operand from stale progress. |
| A414 | Three matched fixed-work pairs reduced median wall from 7.747562 to 6.916713 seconds (10.7%) after arithmetic changes, with identical counters/operation records across all six runs. The first pair alone was slower. This is capped-work timing, not a completed solve; see [the control](A414_FIXED_WORK_RELEASE.md). | Test seeds intended to reduce the still-identical frontier/refinement work. Retain node/refinement and unique-polynomial counts so an apparent timing change is not confused with a different incomplete workload. |
| A446 | Exact physical-region counterexamples disprove the proposed universal positive-common Jacobian nonvanishing theorem. See [the research record](A446_FILTERED_JACOBIAN_RESEARCH.md). | Use graph minors to propose identities on explicitly certified subdomains, or improve legal order selection. Any new filtered projection must retain the known critical point and prove its endpoint, infinity and selector obligations. |

There is also a strong existing negative control: the
[ZZ guide audit](ZZ_RAYLEIGH_GUIDE_AUDIT.md) finds exact opposing coefficient
signs on all 39 tested linear axes across 27 unresolved symbolic ZZ inputs,
for both orientations. Thus a globally positive auxiliary guide of that
tested form cannot be rescued merely by knowing that `U` is positive.
The successful Gluza graph identities are useful examples, not a universal
Symanzik monotonicity theorem.

In particular, neither finite UV/IR power counting nor a generalized
permutahedron proves the original physical `F` is nonzero. Symbolic domains
can also cross exceptional kinematic loci. Complex contour integration
computes a different representation of an integral; it does not give the
required real cylindrical decomposition. These distinctions are acceptance
conditions, not optional numerical cautions.

## Test and ablation plan

First prepare graph/support metadata and seeds without running a new CAD.
Record preprocessing time, peak memory, subset count, graph/input hashes and
every rejected identity. Initial bounds should be small and explicit, with
ordinary polynomial-only behavior on unsupported or over-budget input.

Then compare three arms on A377/A414/A446, their exact crossed partners where
available, and several already solved holdouts: existing seeds; the same
search with support-only seeds; and graph-labelled seeds. Keep the same
binary, search/promotion policy, limits, legal groups, worker count and cache
policy. Include preprocessing and all search time. Use three fresh paired
repeats when affordable, and retain every timeout. Measure verified original
completion, time to first complete candidate, total solve/verify wall, peak
RSS, nodes/refinements, and maximum projected degrees/terms/coefficient bits.
Do not form exact speed ratios from censored attempts.

Only after a useful seed result should a second experiment admit, for example,
32 bounded minor-derived certificate proposals. Count accepted identities,
ordinary fallbacks, construction/replay cost and removed algebra operations.
Independent verification must work without the graph-discovery cache.

Regression cases must include an edge-label permutation, incorrect mass or
Gram entry, missing pinched edge, exceptional coefficient cancellation,
nonpositive convergence weight, an incompatible fixed order, an interior
`F=0` sample, and the saved A446 positive critical-point example. A bad seed
may cost time; a bad graph certificate must be rejected. No sampling outcome,
partial chart, or successful integral evaluation earns CAD coverage credit.
