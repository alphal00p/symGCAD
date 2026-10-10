#let data = json("data.json")
#let ink = rgb("182A3A")
#let muted = rgb("566676")
#let line = rgb("DCE3E8")
#let green = rgb("DDF1E3")
#let yellow = rgb("FFF1BC")
#let red = rgb("F9DADA")
#set document(title: "symGCAD — corpus benchmark", author: "Valentin Hirschi")
#set page(paper: "a4", flipped: true, margin: (x: 11mm, top: 12mm, bottom: 12mm),
  footer: context [#text(size: 7pt, fill: muted)[symGCAD · audited corpus snapshot #h(1fr) #counter(page).display("1")]])
#set text(font: "Libertinus Sans", size: 9pt, fill: ink)
#set par(leading: 0.55em)
#set heading(numbering: none)
#show heading.where(level: 1): set text(size: 23pt, weight: "bold")
#show heading.where(level: 2): set text(size: 12pt, weight: "bold")

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
  full one-hour campaigns; this snapshot must be regenerated as those runs finish.
  Missing times and cells are not zeros, and timeouts do not produce finite speedup ratios.

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
  ceilings range from 4 to 32 GiB and are retained in the source ledgers. RAM is measured resident
  memory, not the configured ceiling. Three historical supplemental verification runs retain elapsed
  time but no RSS; their RAM entries carry † and report the available solve RSS only.
  Concurrent campaigns may share the host, so single-run differences
  are not presented as precise speedups. The pending slow campaign uses bounded process supervision,
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

  *Bounded local and ordering work.* Local projection supports positive-ray certificates,
  finite witness stocks, strict Bézout pair separation and explicit resource stops. MCTS preserves
  variable groups and fixed-order obligations, uses bounded trial tiers and resumable journals,
  and admits winners only after fresh completion. Optional local progress scoring changes promotion
  priority, not mathematical proof acceptance.

  == Reproduce and inspect
  The Typst source and frozen #text(font: "Libertinus Mono")[data.json] render without private
  literature or original proof files. Regeneration from retained evidence uses the Rust example
  #text(font: "Libertinus Mono")[benchmark_report]; its source records the selection rules and rejects
  missing identities or changed audited ledgers. Run #text(font: "Libertinus Mono")[nix develop -c typst compile
  benchmark/benchmark.typ benchmark/benchmark.pdf]. Full provenance remains in the JSON snapshot
  and the repository's benchmark/reference notes.
]
