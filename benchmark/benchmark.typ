#let data = json("data.json")
#import "style.typ": *
#show: report-style
#set document(title: "symGCAD — corpus benchmark", author: "Valentin Hirschi")

= symGCAD / corpus benchmark
#text(size: 11pt, fill: muted)[
  #data.completion_summary.complete_verified / #data.completion_summary.expected_groups distinct original problems independently verified
  #h(1em)·#h(1em) #data.completion_summary.unresolved marked slow
]

All distinct problems are listed below, fastest verified solve first; unresolved problems follow.
*Solve* and *check* are fresh-process wall seconds. *RAM* is maximum recorded resident MiB across solve and verification (†: verification RSS unavailable).
*N* is the repetition count; three-run medians are preferred when available. *D/R* identifies debug/release builds.
Cache tags: *C* = cold; *S* = shared checkpoints (may be warm); *?* = unrecorded.
For incomplete rows, *TO/stop* is the longest retained observed attempt, not a solved time.
Their RAM is the highest retained attempt RSS, which may come from an earlier run.
The *baseline* columns are a separate, matched GCAD comparison: projection + lifting + merging only,
with three fresh repetitions and identical orders. *±* means two separate sign-branch solves, not the optimized full-split solve.

#text(size: 8pt)[
  Matched ratio = symGCAD / GCAD:
  #box(fill: green, inset: 3pt)[< 1]
  #box(fill: yellow, inset: 3pt)[1–2]
  #box(fill: red, inset: 3pt)[> 2]
  #h(1em) — = unavailable or censored; TO = native timeout. Ratios use two significant digits.
]
#v(3pt)

#let ident(s) = text(size: 7.4pt, s.replace("_", "_\u{200b}").replace("/", "/\u{200b}"))
#let matched-cell(r) = {
  let x = r.baseline.ratio
  if x == none { table.cell(fill: rgb("F3F5F7"))[—] }
  else {
    let color = if x < 1 { rgb("216C3D") } else if x <= 2 { rgb("816407") } else { rgb("A3292F") }
    table.cell(fill: if x < 1 { green } else if x <= 2 { yellow } else { red })[
      #r.display.baseline_rust #text(fill: color, weight: "bold")[(×#r.display.ratio)]#if r.baseline.branches == 2 [ ±]
    ]
  }
}
#let measure(r, key) = if r.verified { r.display.at(key) } else { "—" }
#set text(size: 7.7pt)
#table(
  columns: (8mm, 83mm, 7mm, 33mm, 6mm, 16mm, 16mm, 17mm, 19mm, 30mm, 13mm),
  align: (right, left, center, left, center, right, right, right, right, right, right),
  inset: (x: 3pt, y: 2.6pt),
  stroke: (left: none, right: none, top: none, bottom: 0.3pt + line),
  fill: (x, y) => if y == 0 { ink } else if calc.rem(y, 2) == 0 { rgb("F7F9FA") } else { white },
  table.header(
    ..("#", "Original problem / representative full ID", "IDs", "Method · build", "N", "Solve s", "Check s", "RAM MiB", "Cells", "Matched Rust s (×R)", "GCAD s").map(x => text(fill: white, weight: "bold", x)),
  ),
  ..data.rows.map(r => (
    str(r.rank),
    ident(r.identifier),
    str(r.alias_count),
    if r.verified { [#r.measurement.method\ #text(fill: muted)[#r.measurement.build · #if r.measurement.cache_policy == "cold" [C] else if r.measurement.cache_policy == "shared" [S] else [?]]] }
      else { [#text(weight: "bold", fill: muted)[slow / #r.at("slow_status", default: "incomplete")]] },
    if r.verified { str(r.measurement.n) } else { "—" },
    if r.verified { measure(r, "solve") } else if r.longest_incomplete_seconds != none {
      [#if r.last_status == "timeout" [TO] else [stop] #r.display.incomplete]
    } else { "—" },
    measure(r, "verify"),
    if r.verified { [#measure(r, "rss")#if not r.measurement.memory_complete [†]] } else if r.incomplete_peak_rss_mib > 0 { str(calc.round(r.incomplete_peak_rss_mib)) } else { "—" },
    if r.verified { str(r.measurement.cells) } else { "—" },
    matched-cell(r),
    if r.baseline.complete { r.display.baseline_native } else if r.baseline.qualified_branches == 0 { [TO] } else { [partial TO] },
  )).flatten(),
)

#pagebreak()
#set text(size: 10pt)
= Measurement notes and implementation
#columns(2, gutter: 12mm)[
  == Scope and comparability
  The table contains all 331 exact polynomial/domain/order equivalence groups,
  including fixed and symbolic ZZ examples, core examples, and disabled upstream fixtures.
  “IDs” counts the retained original aliases; the accompanying #text(font: "Libertinus Mono")[data.json]
  records every alias, input identity, source report and executable hash.

  A row is complete only when the full original problem has an independently verified result.
  Completing its positive and negative branches separately does not automatically certify the full split problem.
  The current audit has #data.completion_summary.complete_verified verified groups and
  #data.completion_summary.unresolved unresolved groups. The unresolved set is marked slow and assigned
  full one-hour campaigns. The first complete campaign exhausted all 28 budgets.
  Missing times and cells are not zeros, and timeouts do not produce finite speedup ratios.

  All 124 fixed-kinematics ZZ groups are verified. The completed one-hour campaign
  (10 October 2026) timed out on all 27 unresolved symbolic ZZ originals and the
  fixed positive Gluza33 problem. None exported a complete result or verification.
  These attempts add no completion credit; the frozen queue snapshot and all 28 ledger
  hashes are in the JSON. Targeted continuations retain completed algebra separately.

  The latest original Gluza33 continuation also timed out after *7200.040 s*, with a
  50 GiB ceiling and *3902 MiB measured peak RSS*. The latest A377 continuation timed
  out after *3600.031 s*, at 892 MiB peak RSS under a 12 GiB ceiling. Both used audited
  warm native-operation caches and exported no result. Their earlier search and solve
  costs remain separate. The table includes the highest retained RSS, so A377 shows
  an earlier 1650 MiB peak. Eight hash-bound continuation records are retained in the JSON.
  Interrupted algebra experiments and restricted-domain successes add no original coverage.

  Measurements prefer verified three-repeat controls, otherwise the fastest accepted original-problem
  run. Builds are labelled. Cache tags mean cold (C), shared and possibly warm (S), or unrecorded (?).
  Solve times include in-run search, preprocessing and export. Earlier order discovery is separate;
  Gluza30 positive used a disclosed 300.026-second search. Verification has its own column.

  Baseline ratios use the frozen #text(font: "Libertinus Mono")[9025e71] release and original native GCAD,
  with identical inputs, orders, merge settings, CPU assignment and paired limits. For two sign branches,
  displayed times sum their separate three-repeat median phases; ratios compare those matched sums.
  They do not compare the optimized solve column with an unrelated native time.
  All 483 native-completing branches matched raw cell counts and independently verified; 96 native
  timeouts remain explicit. Geometric mean ratios are 0.15 for matched phases and 0.062 for whole processes.

  New reference comparisons use at most max(300 s, 3 × completed symGCAD solve-wall median)
  per native repetition, subject to shorter explicit and campaign limits. Qualification runs
  use a 300-second ceiling before Rust timings are known. Native timeouts have no exact ratio;
  the historical baseline above retains its recorded limits.

  == Machine and resource policy
  AMD EPYC 9754 128-Core Processor, Linux 6.18.45, x86-64. The task budget is 16 CPU workers and
  64 GiB aggregate memory; each measured solver process uses one CPU. Historical per-worker memory
  ceilings range from 4 to 50 GiB and are retained in the source ledgers. RAM is measured resident
  memory, not the configured ceiling. Three historical supplemental verification runs retain elapsed
  time but no RSS; their RAM entries carry † and report the available solve RSS only.
  Concurrent campaigns may share the host, so single-run differences
  are not presented as precise speedups. The completed slow attempts used bounded process supervision,
  durable attempt records and independent proof verification.

  *Supplemental sanity check.* Fixed A446 passed two fresh rational points in each of its eight
  exported cells (seed 20261010): 16 exact original-F evaluations and 112 strict-constraint checks,
  with zero failures or unchecked cells. Boundaries were recomputed at each fresh prefix.
  This separate check is outside benchmark timings and supplies no coverage-proof credit;
  full points and counterexamples from a deliberately wrong sign label are retained in
  #text(font: "Libertinus Mono")[reference/SUPPLEMENTAL_SANITY.md].

  == What changed
  *Exact algebra and reuse.* Rust uses Symbolica for polynomial arithmetic, factorization,
  resultants and rational root work. Canonical factors, deterministic algorithms and bounded caches
  avoid repeated work. Certified sum-variable preprocessing and primitive parameter-linear factors
  address measured algebra bottlenecks.

  *Smaller valid decompositions.* Positive-domain certificates, homogeneous charts, conditional
  quadratic monotone reduction and strict linear/bilinear identities can avoid irrelevant projection
  factors or restore whole fibers from a smaller verified base. Fixed A381/Ax381 and the negative
  Gluza cases now have independently verified compact decompositions. Every specialized rule carries
  exact replayable evidence; ordinary projection remains a fallback.
  New default-off quadratic-component and ladder rules preserve all interior split boundaries.
  The full crossed A446 polynomial has a verified five-cell decomposition only on the added
  region $t < 4$; its original unrestricted problem remains unresolved.

  *Bounded local and ordering work.* Local projection supports positive-ray certificates,
  finite witness stocks, strict Bézout pair separation and explicit resource stops. MCTS preserves
  variable groups and fixed orders with bounded trials, resumable journals and freshly completed
  winners. Optional progress scoring affects promotion, never acceptance. See the separately
  preserved #link("ordering-mcts.pdf")[two-page ordering and MCTS companion].

  == Recent controlled validation
  A separate release experiment completed all 66 fresh solve trials and 66 independent
  verifications: three cold repetitions per configuration, one CPU and 2 GiB, identical
  given orders and unmerged output. These synthetic and conditional cases add no original
  corpus completions and no native-GCAD comparison. Selected off/on results are:

  #table(
    columns: (1.4fr, 1fr, 1fr), inset: 4pt, stroke: 0.3pt + line,
    [*Case / selected rule*], [*Internal solve ms*], [*Raw cells*],
    [Positive endpoint / weighted], [37.160 → 9.305], [20 → 1],
    [Quadratic constant / components], [18.342 → 7.629], [8 → 1],
    [Local linear split / components], [2.374 → 0.823], [12 → 2],
  )
  These are internal phases, not the public solve column. Median process times for the
  same rows were 0.102508 → 0.061243 s, 0.081800 → 0.061391 s, and
  0.061425 → 0.061435 s respectively. Startup and supervision dominate the approximately
  61 ms group; its small differences establish no end-to-end gain. The local split keeps
  both signs and its interior zero sheet. Some certificates cost more on other cases,
  so the results support selective opt-in use.

  The conditional full crossed A446 ladder case took a 0.061135 s median public solve
  and 0.020867 s fresh verification, with five cells and 6196 KiB maximum RSS.
  Its extra $t < 4$ condition remains explicit. Two fresh interior points per cell passed
  90 original-constraint checks and 10 original-F evaluations; this supplemental check
  is outside timings and supplies no coverage proof. Detailed variants, process measurements
  and hashes are in #text(font: "Libertinus Mono")[reference/QUADRATIC_RELEASE_ABLATION.md].

  == Root-isolation research
  An isolated backend with unit-shift and positive-axis research patches compares complete
  full-real and strictly positive root inventories for the same exact polynomial. The corpus
  backend is unchanged. Ordered matches require an exact point root
  or a sign-changing interval intersection, not identical interval spelling. A positive-only
  index cannot silently replace a full-real index in an existing proof. Individual Taylor-shift
  measurements are separate from complete isolation and original-problem solve times.
  // Append the final independently checked isolation measurement here when available.

  == Reproduce and inspect
  The Typst source and frozen #text(font: "Libertinus Mono")[data.json] render without private
  literature or original proof files. Regeneration from retained evidence uses the Rust example
  #raw("benchmark_report --followups benchmark/followups.json").
  Its metadata-only overlay checks pinned ledgers,
  exact original fields, legal orders and executable hashes; it changes no completion or ratio.
  Run #text(font: "Libertinus Mono")[benchmark/render.sh] to render both PDFs. Full provenance remains in the JSON snapshot
  and the repository's benchmark/reference notes.
]
