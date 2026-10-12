// symGCAD corpus benchmark (generated data: report-data.json, built by build_report.py).
// Section 1: all 331 corpus problems, symGCAD (cold runs) vs the original native GCAD.
// Section 2: improvements over the original GCAD (improvements.typ).
// Section 3: Symbolica vs FLINT on the purely algebraic steps (summary from
//            FLINT_vs_symbolica/results/summary.json, table included from FLINT_vs_symbolica/results/table.typ).
#import "style.typ": ink, muted, line, green, yellow, red
#let data = json("report-data.json")
#let S = data.summary

#set document(title: "symGCAD — corpus benchmark", author: "Valentin Hirschi")
#set page(paper: "a4", flipped: true, margin: (x: 11mm, top: 11mm, bottom: 12mm),
  footer: context [#text(size: 7pt, fill: muted)[symGCAD · corpus benchmark, improvements over GCAD, Symbolica vs FLINT #h(1fr) #counter(page).display("1")]])
#set text(font: "Libertinus Sans", size: 9pt, fill: ink)
#set par(leading: 0.55em, justify: false)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => { set text(size: 17pt, weight: "bold"); block(above: 0.6em, below: 0.7em, it) }
#show heading.where(level: 2): set text(size: 11pt, weight: "bold")
#show raw: set text(size: 0.95em)

// ---------- helpers
#let mono(s) = text(font: "Libertinus Mono", size: 0.92em, s)
#let fmt-int(n) = {
  if n == none { "—" } else {
    let s = str(n)
    let out = ""
    let k = 0
    for i in range(s.len() - 1, -1, step: -1) {
      if k > 0 and calc.rem(k, 3) == 0 { out = "," + out }
      out = s.at(i) + out
      k += 1
    }
    out
  }
}
#let fmt(x, d: 2) = if x == none { "—" } else { str(calc.round(x, digits: d)) }
#let pct(a, b) = if b == 0 { "—" } else { str(calc.round(100 * a / b, digits: 1)) + "%" }
#let speed(r) = if r == none { "—" } else { str(calc.round(1 / r, digits: 1)) + "×" }
#let short(h) = if h == none { "—" } else { mono(h.slice(0, 12)) }
#let ident(s) = s.replace("_", "_\u{200b}").replace("/", "/\u{200b}")
#let card(title, big, body) = block(width: 100%, fill: rgb("EDF3F7"), radius: 4pt, inset: 7pt)[
  #text(size: 8pt, fill: muted, weight: "bold", upper(title))\
  #text(size: 17pt, weight: "bold", big)\
  #text(size: 8pt, body)
]

// draft detection: anything not final is listed on the title page
#let draft-reasons = {
  let r = ()
  if S.strict.verified_groups_without_cold_run > 0 { r.push([#S.strict.verified_groups_without_cold_run groups have no cold run with the report binary (data.json history shown)]) }
  if S.symgcad_binaries.len() > 1 { r.push([cold runs mix #S.symgcad_binaries.len() symGCAD binaries]) }
  if S.native.cases_recorded < S.native.cases_total { r.push([native GCAD run still in progress (#S.native.cases_recorded of #S.native.cases_total sign cases recorded)]) }
  if data.flint == none { r.push([no Symbolica-vs-FLINT results found (benchmark/FLINT_vs_symbolica/results/)]) }
  if S.opt_in.completed < S.opt_in.rows { r.push([opt-in projective rows not yet run with the report binary]) }
  r
}

// ---------- title
#text(size: 22pt, weight: "bold")[symGCAD — corpus benchmark]
#h(1fr) #text(size: 9pt, fill: muted)[generated #data.generated_utc.slice(0, 10)]
#v(-4pt)
#text(size: 10.5pt, fill: muted)[All 331 distinct problems of the GCAD corpus: exact generic CAD in Rust on Symbolica, against the original GCAD by Vitaly Magerya]
#if draft-reasons.len() > 0 [
  #v(3pt)
  #block(width: 100%, fill: rgb("FFF1BC"), radius: 4pt, inset: 6pt)[
    #text(weight: "bold")[DRAFT.] #draft-reasons.join([; ]). Regenerate with #mono("benchmark/run_final.sh <symgcad binary>") (see #mono("benchmark/README.md")).
  ]
]

= Benchmark results
#v(-2pt)
#let by = S.strict.by_scope
#let C = S.comparison
#grid(columns: (1fr, 1fr, 1fr, 1fr, 1fr), gutter: 4mm,
  card("strict contract", [#S.strict.verified / #S.strict.expected], [
    independently verified · fixed #by.fixed.verified/#by.fixed.expected · symbolic #by.symbolic.verified/#by.symbolic.expected · core #by.core.verified/#by.core.expected.
    #if S.strict.newly_verified.len() > 0 [New: #S.strict.newly_verified.len().]
    #if S.strict.lost.len() > 0 [#text(fill: rgb("A3292F"))[Lost vs data.json: #S.strict.lost.len().]]
  ]),
  card("opt-in projective", [+#S.opt_in.new_completions.len()], [
    #if S.opt_in.new_completions.len() > 0 [#S.opt_in.new_completions.map(ident).join(", ") completes] else [no new completion] only under #mono("projective_sections = true") (§1.2); #S.opt_in.completed of #S.opt_in.rows opt-in rows verified. *Not* counted in the strict total.
  ]),
  card("native GCAD, 300 s/case", [#S.native.groups.at("complete", default: 0) / #S.groups], [
    groups completed · #S.native.groups.at("timeout", default: 0) timeout · #S.native.groups.at("memory", default: 0) out of memory (8 GiB)#if S.native.groups.at("not run", default: 0) > 0 [ · #S.native.groups.at("not run") not run yet]; cases #S.native.cases_status.at("complete", default: 0)/#S.native.cases_total complete.
  ]),
  card("geometric mean ratio", [#fmt(C.geomean_solve_over_native_wall, d: 3)], [
    symGCAD solve / native time on the #C.n groups both complete (#speed(C.geomean_solve_over_native_wall) faster); #C.symgcad_faster of #C.n faster, #C.ratio_above_2 more than 2× slower.
  ]),
  card("only symGCAD completes", [#C.verified_where_native_fails], [
    groups verified by symGCAD where native GCAD timed out or ran out of memory#if C.native_completes_symgcad_not.len() > 0 [; native-only: #C.native_completes_symgcad_not.len()]. Summed times on the common set: #fmt(C.symgcad_solve_sum_s, d: 1) s vs #fmt(C.native_wall_sum_s, d: 1) s.
  ]),
)
#v(4pt)
#let T1 = C.tiers.at(0)
#block(width: 100%)[
#set text(size: 8.3pt)
#set par(leading: 0.48em)
#grid(columns: (1fr, 1fr), gutter: 8mm, [
*What is compared.* Each row is one distinct original problem (aliases merged; split polynomials kept). *symGCAD* solves the full problem in one fresh process with its recipe (engine, ordering policy, options from the 2026-10-10 audit, #mono("benchmark/campaign/symgcad_inputs/")), then #mono("symgcad verify") re-checks the exported proof in a second fresh process. Only a verified #mono("complete_generic") result counts. *Native* is the unmodified GCAD (#mono("fb6ebe3"), FLINT 3.6) through the unmodified bridge #mono("reference/adapter.py"): each sign case of the split (`F > 0` and `F < 0` added as constraints, as in the earlier parity campaign) is solved in its own process, capped at 300 s and 8 GiB (the two sign cases that hit #mono("std::bad_alloc") at 8 GiB were repeated with 24 GiB and then timed out at 300 s); the native time is the sum over sign cases and includes Python start-up and SymPy parsing. The *ratio* is symGCAD solve / native time; green < 1 (symGCAD faster), yellow 1–2, red > 2. Most corpus problems are tiny, and there the native time is dominated by Python start-up: against native internal time alone (no start-up), the geometric mean over all #C.n groups is #fmt(C.geomean_solve_over_native_internal, d: 2), i.e. about parity, because symGCAD's own process time includes its start-up and export. On the #T1.n groups where native GCAD needs at least 1 s of internal time, the geometric mean ratio is #fmt(T1.geomean_solve_over_native_wall, d: 3) against native process time and #fmt(T1.geomean_solve_over_native_internal, d: 3) against native internal time (#speed(T1.geomean_solve_over_native_internal) faster without any start-up cost on the native side).
], [
*Columns.* *Order* is the variable order of the exported result (projection eliminates from the right); *H* marks the homogeneous anchor fixed to 1 before solving. *Method* is the engine and ordering policy (#mono("mods") = GCAD's heuristic, #mono("given") = recipe order, #mono("mcts") = bounded search inside the timed solve). *Cells* are verified raw cells (no merging; GCAD merges, so cell counts are not comparable); † marks the #S.strict.cells_differ_from_history groups whose count differs from the 2026-10-10 audit (both verified; recipes and code changed since). Times are wall seconds on one pinned CPU of an AMD EPYC 9754, with cold caches (fresh checkpoint directory and #mono("TMPDIR") per run); verified rows are sorted by solve time, then the incomplete ones. *Provenance:* symGCAD binary #for (h, n) in S.symgcad_binaries [#short(h) (#n groups) ] run #if S.symgcad_run_dates != none [#S.symgcad_run_dates.at(0).slice(0, 16)–#S.symgcad_run_dates.at(1).slice(11, 16) UTC]; native launch #if data.provenance.native.len() > 0 [#data.provenance.native.at(0).launched_utc.slice(0, 16) UTC, upstream #mono(data.provenance.native.at(0).upstream_revision.slice(0, 7))]. Every row's binary hash, input hash, date and record path are in #mono("report-data.json").
])
]

== All 331 problems
#let ratio-cell(r) = {
  let c = r.display.ratio_class
  let f = if c == "fast" { green } else if c == "par" { yellow } else if c == "slow" { red } else { none }
  table.cell(fill: f)[#r.display.ratio]
}
#let status-cell(r) = {
  if r.verified {
    let k = r.symgcad.provenance.kind
    if k == "cold run" [verified] else if k == "warm long attempt" [#text(fill: muted)[verified (warm)]] else [#text(fill: muted)[verified (hist.)]]
  } else { text(fill: rgb("A3292F"), weight: "bold", r.display.status) }
}
#let native-cell(r) = {
  let n = r.native.status
  if n == "complete" [#r.display.native]
  else if n == "timeout" [#text(fill: rgb("A3292F"))[timeout]]
  else if n == "not run" [#text(fill: muted, style: "italic")[#r.display.native]]
  else [#text(fill: rgb("A3292F"))[#r.display.native]]
}
#let order-cell(r) = {
  let o = r.display.order
  if r.display.anchor != "" [#o #text(fill: muted)[· H=#r.display.anchor]] else [#o]
}
#let solve-cell(r) = if r.verified [#r.display.solve] else [#text(fill: muted)[#r.display.longest#super(r.display.longest_tag)]]
// cache tag: "cold" for every row measured by this report's runner (fresh checkpoint directory and
// TMPDIR); unresolved rows carry the tag of the attempt shown in the Solve column (legend below).
#let cache-cell(r) = {
  if r.verified {
    let k = r.symgcad.provenance.kind
    if k == "cold run" [cold] else if k == "warm long attempt" [#text(fill: rgb("A3292F"))[warm]] else [#text(fill: rgb("A3292F"))[hist.]]
  } else {
    let t = r.display.longest_tag
    if t == "c" [cold] else if t == "w" [#text(fill: muted)[warm#super[w]]] else if t == "h" [#text(fill: muted)[hist.#super[h]]] else [—]
  }
}
#let cells-cell(r) = [#r.display.cells#if r.display.cells_mark != "" [#text(fill: muted)[#r.display.cells_mark]]]
#{
  set text(size: 7pt)
  set par(leading: 0.4em)
  table(
    columns: (7mm, 63mm, 12mm, 19mm, 34mm, 50mm, 15mm, 13mm, 12mm, 10mm, 17mm, 11mm),
    align: (right, left, left, left, left, left, right, right, right, left, right, right),
    inset: (x: 2.5pt, y: 2.2pt),
    stroke: (left: none, right: none, top: none, bottom: 0.3pt + line),
    fill: (x, y) => if y == 0 { ink } else if calc.rem(y, 2) == 0 { rgb("F7F9FA") } else { white },
    table.header(..("#", "Problem", "Scope", "Status", "Method · ordering", "Order (H = anchor)", "Cells", "Solve s", "Verify s", "Cache", "Native s", "Ratio").map(x => text(fill: white, weight: "bold", x))),
    ..data.rows.map(r => (
      str(r.rank), ident(r.identifier), r.scope, status-cell(r),
      [#r.display.method #text(fill: muted)[· #r.display.ordering]],
      order-cell(r), cells-cell(r), solve-cell(r), r.display.verify, cache-cell(r), native-cell(r), ratio-cell(r),
    )).flatten(),
  )
}
#text(size: 7.5pt, fill: muted)[*Cache tags.* #emph[cold]: measured in this report's run with the report binary, in a fresh process with a fresh checkpoint directory and #mono("TMPDIR") (no algebra cache shared with any other run); every verified row is cold. The other tags occur only on unresolved rows, which this run did not re-attempt: their *Solve s* column shows the longest earlier attempt, a lower bound and not a solve time. #super[h] #emph[hist.]: the 2026-10-10 one-hour campaign (Gluza33: 7,200 s), recorded in #mono("benchmark/data.json"); #super[w] #emph[warm]: a 2026-10-12 one-hour attempt with improved local-engine recipes and a warm shared operation cache (binary #mono("c8f2c62f"), integration #mono("4dbe440")); 4 of these launches found their input withdrawn and are ignored, so those rows fall back to #super[h]. For #mono("mcts") recipes without a fixed order, the order shown is the one the 2026-10-12 attempt was working on (— where none was recorded). *Native s* is the sum over sign cases of the native GCAD process time; #emph[timeout] means at least one sign case hit the 300 s cap. Each attempt's status, wall time, peak memory and binary are in #mono("report-data.json") (#mono("rows[].attempts")).]

== Opt-in projective contract (separate table, not counted in the strict total)
These rows use the user-approved contract extension #mono("solver.projective_sections = true") (Section 2, item X1): a cell's upper boundary may be a positive root that escapes to +∞ inside the cell. They are reported separately and *do not change the strict count of #S.strict.verified / #S.strict.expected*.
#v(2pt)
#{
  set text(size: 8pt)
  table(
    columns: (58mm, 35mm, 40mm, 13mm, 12mm, 12mm, 10mm, 28mm, 14mm, 46mm),
    align: (left, left, left, right, right, right, left, left, right, left),
    inset: (x: 3pt, y: 3pt),
    stroke: (left: none, right: none, top: none, bottom: 0.3pt + line),
    fill: (x, y) => if y == 0 { ink } else { white },
    table.header(..("Problem (contract)", "Engine (base)", "Order (H = base anchor)", "Cells", "Solve s", "Verify s", "Cache", "Same problem, strict contract", "Native s", "Sanity check").map(x => text(fill: white, weight: "bold", x))),
    ..data.opt_in.map(e => {
      let c = e.at("symgcad", default: none)
      let strict = if e.strict_status == "verified" [#fmt-int(e.strict_cells) cells, #fmt(e.strict_solve_s, d: 2) s] else [unresolved]
      let nat = if e.native.status == "complete" [#fmt(e.native.wall_s, d: 2)] else [#e.native.status]
      let san = e.at("sanity", default: none)
      let ident-cells = san == none or san.at("cells_identical_to_checked", default: none)
      let sanity = if san == none [—] else [#fmt-int(san.points) fresh points in #fmt-int(san.cells_checked) cells, #san.failures failures#if not san.same_binary [ #text(fill: muted)[(checked on binary #mono(san.binary_sha256.slice(0, 8))'s result#if ident-cells == true [, whose cells are identical to this run's] else if ident-cells == false [; #text(fill: rgb("A3292F"))[cells differ from this run]])]]]
      let label = [#e.label \ #text(size: 7pt, fill: rgb("1E3F73"), weight: "bold")[opt-in contract] #text(size: 7pt, fill: muted)[#mono("projective_sections = true")]]
      if c == none {
        (label, [not run], [—], [—], [—], [—], [—], strict, nat, sanity)
      } else {
        (label, [#c.engine (#c.base_engine)],
         [#c.order.join(" ")#if c.anchor != none [ #text(fill: muted)[· H=#c.anchor]]],
         if c.verified [#fmt-int(c.cells)] else [#text(fill: rgb("A3292F"))[#c.status]],
         fmt(c.solve_s, d: 2), fmt(c.verify_s, d: 2),
         if c.provenance.kind == "cold run" [cold] else [#c.provenance.kind], strict, nat, sanity)
      }
    }).flatten(),
  )
}
#text(size: 7.5pt, fill: muted)[Projective rows: same cold protocol and binary as the strict table (#mono("benchmark/campaign/projective/")). The sanity check (seed 20261010, two fresh rational points per cell, original TOML expressions evaluated exactly) is outside all timings and supplies no proof credit.]

#[
  #set page(columns: 2)
  #set columns(gutter: 7mm)
  #set text(size: 8.2pt)
  #set par(leading: 0.48em, spacing: 0.62em)
  #set list(spacing: 0.55em)
  #show heading.where(level: 2): set block(above: 0.9em, below: 0.5em)
  #include "improvements.typ"
]

#pagebreak()
= Symbolica vs FLINT on the algebraic steps
#if data.flint == none [
  No Symbolica-vs-FLINT measurements were found when this report was built.
] else [
  #let F = data.flint
  #let kinds = F.by_kind.filter(k => k.n_ratio > 0)
  symGCAD runs all exact polynomial algebra through Symbolica; the original GCAD calls FLINT. To isolate the algebra from everything else, exact operands were captured from real symGCAD solves (Gluza30/33, A317, A5\_31, fixed A377 and others), written in a neutral format, and replayed one operation per fresh process through (i) the kernel symGCAD's production code actually uses (Symbolica, after symGCAD's own dispatch: closed-form discriminants, interpolation, binary segmentation), (ii) Symbolica's individual algorithms (Lazard–Ducos subresultants, Brown/CRT), and (iii) FLINT 3.6 (#mono("fmpz_mpoly") and #mono("fmpz_poly")). Every output pair is compared after canonicalization (signs and content); #F.outputs_compared outputs were compared with #F.mismatches mismatches. Times are the minimum over repetitions on one pinned CPU; *ratio = Symbolica time / FLINT time*, so a ratio above 1 means FLINT is faster.

  #v(2pt)
  #if kinds.len() > 0 {
    grid(columns: kinds.len(), gutter: 4mm,
      ..kinds.map(k => card(k.kind, [#fmt(k.geomean, d: 2)], [geometric-mean ratio over #k.n_ratio of #k.n operands; total-time ratio #fmt(k.total, d: 2)])))
  }
  #v(3pt)
  #let verdict(k) = if k.geomean == none [no ratio] else if k.geomean > 1.25 [FLINT faster] else if k.geomean < 0.8 [Symbolica faster] else [about even]
  *Summary.* #if kinds.len() > 0 [By geometric mean: #kinds.map(k => [#k.kind #verdict(k) (#fmt(k.geomean, d: 2))]).join([; ]).] The geometric mean weights every operand equally, so it is dominated by small, sub-millisecond operations; the total-time ratio is dominated by the few largest operands. Resultants and discriminants dominate projection cost; factorizations and GCDs build its square-free bases.
  #if F.best_for_symbolica.len() > 0 and F.best_for_flint.len() > 0 [Best case for Symbolica: #mono(F.best_for_symbolica.at(0).id) (ratio #fmt(F.best_for_symbolica.at(0).ratio, d: 2)); best case for FLINT: #mono(F.best_for_flint.at(0).id) (ratio #fmt(F.best_for_flint.at(0).ratio, d: 1)).]

  *Status of the measurements.* The numbers are the committed snapshot in #mono("benchmark/FLINT_vs_symbolica/results/") (2026-10-12, 12:57 UTC): #F.n_ops operands, #F.jobs_run of #F.n_jobs timing jobs run (#F.jobs_capped stopped at their cap, mostly exploratory 120 s kernels), #F.jobs_pending jobs on #F.ops_pending large-tier operands not yet measured. Those operands (nine of the eleven A377 pair resultants, the four Gluza33 critique discriminants, the stalled Gluza33 main-axis resultant, two operations in progress at the harvest deadlines and the large-tier segmentation encodings) are listed as #emph[pending], never as timeouts. Each job ran once; a 26-operand subset re-run twice from clean builds reproduced all 88 exact outputs and the times within 0.85–1.05×.

  *Interpretation.*
  #set list(spacing: 0.5em)
  - *Resultants and discriminants: symGCAD's production path beats FLINT on large inputs, by algorithm choice.* For the large pair resultants symGCAD packs two parameters into one integer variable (binary segmentation, item A9) and computes a single univariate integer resultant: 3–7.5× faster than FLINT's #mono("fmpz_mpoly_resultant") on Gluza33, 3.5× on A317, 10.5× on A318, 15.7× on one A377 pair, and 34 s on another A377 pair where FLINT did not finish in 600 s. Closed-form discriminants (A5) are 6–18× faster than #mono("fmpz_mpoly_discriminant") on the larger parametric ones.
  - *Like-for-like kernels are close.* Both libraries' Lazard–Ducos subresultants: Symbolica 0.74–0.88× FLINT's time on large operands, about 1.0–1.1× on small ones; the univariate integer resultant inside segmentation is at parity (0.93–1.07). FLINT's multimodular univariate resultant is 1.7× faster on an interpolation image (10.0 ms against 16.9 ms), and its default #mono("fmpz_poly_resultant") dispatcher picks a multimodular method that does not finish on the segmented operands.
  - *Factorization and GCD: FLINT is 3–22× faster on large dense bivariate inputs.* On the dense Gluza33 factor operands (13–21 k terms) FLINT's #mono("fmpz_mpoly_factor") takes 0.17–2.8 s against Symbolica's 3.2–10.6 s, and gcd(P, ∂P) 0.14–0.27 s against 1.3–5.0 s; FLINT's dense Brown-type modular GCD suits these shapes. On the sparse 4–5-variable factorizations and GCDs of the parametric problems Symbolica is mostly faster (0.17–0.77; GCDs 0.19–1.9). This is the most valuable upstream target for Symbolica.
  - *What it means for symGCAD.* Projection cost in the hard cases is dominated by large resultants and discriminants, where the production path is already faster than FLINT; the remaining kernel gap is in dense multivariate GCD and factorization. No output differed between the libraries.

  *Reproduction.* Harness (Symbolica and FLINT sides), operand manifest with sha256 and provenance, nix build/run commands, the subset re-run and six optimization ideas: #mono("benchmark/FLINT_vs_symbolica/README.md"). The Symbolica side is built with #mono("cargo build --release --locked --features flint-bench-hooks --example flint_vs_symbolica"); the feature is off by default.

  #set text(size: 7.5pt)
  #set table(inset: (x: 3pt, y: 2pt))
  #show figure: set block(breakable: true)
  #show figure.caption: set text(size: 8pt)
  #include F.table
]
