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

  The completed one-hour campaign (10 October 2026) timed out on all 27 symbolic ZZ
  originals and positive Gluza33. None exported a complete result or verification.
  These attempts add no completion credit; the frozen queue snapshot and all 28 ledger
  hashes are in the JSON. Targeted continuations retain completed algebra separately.

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
  are not presented as precise speedups. The ongoing slow campaign uses bounded process supervision,
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

#pagebreak()
#set text(size: 9.5pt)
#set par(leading: 0.45em, spacing: 0.65em)
#show heading.where(level: 2): set block(above: 0.85em, below: 0.45em)
= Variable ordering / bounded MCTS
#text(size: 11pt, fill: muted)[Search mechanics · 1 of 2]
#v(5pt)
#block(width: 100%, fill: rgb("EDF3F7"), radius: 5pt, inset: 10pt)[
  *The objective is the fastest completed ordering found within a budget.*
  symGCAD implements Monte Carlo tree search (MCTS) over variable-elimination prefixes.
  Exact solves measure candidates; heuristic scores guide where to spend the next trial.
  This search supplies no certificate that its chosen ordering is globally optimal.
]
#v(5pt)
#grid(
  columns: (1fr, 1fr, 1fr, 1fr), gutter: 5mm,
  ..(
    [*01 · Seed*\ Given order + inexpensive polynomial heuristics],
    [*02 · Explore*\ UCT selects a prefix; a rollout fills the remaining order],
    [*03 · Measure*\ Bounded exact solves; promising timeouts get longer trials],
    [*04 · Select*\ Fastest completed candidate; otherwise a restart suggestion],
  ).map(body => block(width: 100%, stroke: 0.5pt + line, radius: 4pt, inset: 8pt, body)),
)
#v(8pt)
#grid(columns: (1fr, 1fr), gutter: 12mm,
  [
    == Why order matters
    Each projection eliminates a variable and forms discriminants, leading coefficients
    and pairwise resultants. An unfortunate early elimination can increase degrees,
    term counts and coefficient sizes dramatically, making every later step expensive.
    Lifting cost also depends on the resulting root boundaries and number of cells.
    Consequently, the final choice uses completed solve time, including lifting.

    == The tree and its finite depth
    A node represents an elimination prefix; an edge chooses the next legal variable.
    With #emph[n] variables, a complete path has depth #emph[n]. Variable groups constrain
    the legal branches. An explicit fixed order bypasses MCTS, and a homogeneous scale
    anchor must retain its required position.

    Public orders are written in *lifting order*: an order #raw("[x, y, z]") means
    eliminate #raw("z"), then #raw("y"), then #raw("x"). The tree uses this elimination
    direction internally. For example, #raw("[z, y]") fixes the first two eliminations,
    while a rollout fills in the remaining variables.

    The implementation has *no separate depth-k lookahead setting*. It does not stop
    after k eliminations, choose one variable and restart the search at the next level.
    Its limits are elapsed time and scheduler iterations; each evaluated candidate
    is a complete ordering, although its algebra worker can time out partway through.

    == Inexpensive starting orders
    The initial portfolio contains the given order and deduplicated MODS-, T1-, Brown-
    and sparsity-based rankings. These seeds use the original input features, avoiding
    a full projection for each seed. Expansion and rollout priorities likewise use a
    fixed input ranking; they are not recomputed from every projected polynomial set.
  ],
  [
    == Selection, rollout and feedback
    Unexpanded legal choices are tried in heuristic priority order. Once all children
    exist, UCT combines mean reward with an exploration bonus and a small, decaying
    preference for the heuristic ranking. The default score for a visited child is

    $ Q_i + sqrt(2) sqrt((ln N) / n_i) + 0.1 / ((r_i + 1)(n_i + 1)) $

    Here #emph[Q] is mean reward, #emph[N] the parent visit count, #emph[n] the child
    visit count and #emph[r] its zero-based heuristic rank. Unvisited children take
    priority. A rollout follows the cheapest-ranked legal choice 80% of the time;
    otherwise it samples a legal choice uniformly. Seed 0 is the default.

    Completed solves earn the largest rewards, with faster solves scoring higher.
    A timed-out global-projection trial can earn a smaller reward from observed factor,
    term, degree and coefficient-size costs. These observations predict difficulty;
    they do not estimate how much of the solution set has been proved.

    == Successive time budgets
    Fresh orders start with a one-second worker limit. Timed-out candidates can be
    promoted to ten seconds, then sixty seconds. Promotion normally follows eight
    fresh trials, or happens sooner when all legal orders have been seen. Every fourth
    promotion favors a deeper tier so longer trials are not starved.

    Killable worker processes enforce deadlines even inside an expensive resultant
    or factorization. Each trial is clipped to the remaining search budget. The
    optimized engine can also abandon prefixes with extreme observed term growth
    and avoid repeating them during that search campaign.

    == What wins
    Only a completed exact solve can become #raw("best_order"). The comparison uses
    measured worker wall time, with fewer cells breaking an exact timing tie. With
    no completion, #raw("suggested_order") is a heuristic restart candidate.
    The public solve command then runs the selected order with the remaining campaign
    time and exports that solve's evidence.
  ],
)

#pagebreak()
= Variable ordering / use and evidence
#text(size: 11pt, fill: muted)[Budgets, reproducibility and interpretation · 2 of 2]
#v(7pt)
#grid(columns: (1fr, 1fr), gutter: 12mm,
  [
    == Enable the search
    Add these settings to a problem TOML, keeping its variables, polynomial expressions
    and domain constraints. Leave the explicit #raw("order") field unset.

    #block(width: 100%, fill: rgb("F3F5F7"), radius: 4pt, inset: 9pt)[
      #text(size: 9pt)[```toml
      [solver]
      engine = "optimized"
      ordering = "mcts"
      seed = 0
      max_rollouts = 256
      search_time_secs = 300
      checkpoint_dir = "artifacts/checkpoints"

      [limits]
      wall_time_secs = 3600
      memory_mib = 4096
      workers = 1
      ```]
    ]

    #raw("symgcad order problem.toml") runs the bounded search and reports its history.
    #raw("symgcad solve problem.toml") includes the search, then solves the selected
    ordering. #raw("symgcad verify result.json") checks the exported decomposition
    separately. MCTS is opt-in; the default ordering is #raw("given").

    == How the limits interact
    #table(
      columns: (1.05fr, 1.55fr),
      inset: 5pt,
      stroke: (left: none, right: none, top: none, bottom: 0.4pt + line),
      table.header([*Control*], [*Effective behavior*]),
      [Search wall time], [At most min(300 s, 20% of campaign); an explicit search limit can lower it.],
      [#raw("max_rollouts")], [256 scheduler iterations by default; promotions and revisits also consume iterations.],
      [Candidate tiers], [1 s → 10 s → 60 s, clipped to remaining search time.],
      [Final solve], [Receives the remaining campaign wall budget after search.],
    )

    A one-hour campaign therefore allows up to five minutes for search; a 300-second
    campaign allows up to sixty seconds. The iteration ceiling does not promise 256
    distinct exact solves. Process launch and scheduling can add small timing overheads.

    == Reuse and restart
    Compatible trials share completed exact algebra and projection-prefix caches.
    An atomic journal preserves trial history, random-generator state through replay,
    tree visits and promotion state. Context checks bind the problem, domains, groups,
    solver settings and executable identity. Restored completion claims must pass a
    fresh bounded solve before they can win; the journal never substitutes for proof.
  ],
  [
    == Soundness and the meaning of “best”
    Choosing or abandoning an ordering changes search effort only. It cannot delete
    a solution region, omit an ordinary F = 0 sheet or justify a projection reduction.
    The selected final solve does not use the search's heuristic prefix-abandonment rule.
    Its complete status still requires exhaustive algebraic construction, with an
    independently checked proof required for benchmark completion credit.

    “Best” means the fastest completed candidate observed under the recorded limits,
    engine and cache conditions. It is not an exhaustive comparison of all permutations.
    A favorable partial score, a timeout or an old journal entry cannot establish a
    solved result. If the final solve also fails to finish, the problem remains incomplete.

    == What the measurements establish
    One controlled experiment tested the optional #raw("search_local_progress = true")
    policy on a five-dimensional positive Gluza30 base using the local engine. The
    option uses polynomial inventory and refinement growth to rank timed-out candidates
    for promotion. It is disabled by default and changes neither UCT rewards nor the
    completed-winner rule.

    #table(
      columns: (1.2fr, 0.7fr, 0.9fr),
      align: (left, center, right), inset: 5pt,
      stroke: (left: none, right: none, top: none, bottom: 0.4pt + line),
      table.header([*Promotion policy*], [*Verified*], [*Median solve*]),
      [Existing scoring], [0 / 3], [313.62 s stop],
      [Local inventory scoring], [3 / 3], [65.10 s],
    )

    These paired fresh-process runs used the same release executable, seed 0, cold
    checkpoints, a sixty-second search budget, one CPU and 4 GiB per worker. Enabled
    runs produced 15,389 cells; median separate verification took 2.50 seconds. The
    reported solve time includes search. The unfinished controls do not yield an exact
    speed ratio, and this experiment compares two MCTS promotion policies, not MCTS
    against a heuristic-only solver.

    This is evidence of benefit on one reduced problem. The earlier multi-problem
    ordering study did not establish an overall MCTS advantage; the new result does
    not establish a corpus-wide improvement or explain the matched-baseline speedup.
    Search overhead and shared-cache effects must remain visible in comparisons.

    == Implementation and audit trail
    #text(size: 8.7pt)[
      #raw("src/search.rs"): tree policy, rewards, tier scheduling and prefix abandonment.\
      #raw("src/ordering.rs"): inexpensive seed portfolio and group conventions.\
      #raw("src/main.rs"): process trials, budgets and final solve.\
      #raw("src/search/checkpoint.rs"): bounded journal and restart validation.\
      #raw("docs/USAGE.md"): configuration and command usage.\
      #raw("reference/LOCAL_SEARCH_PROMOTION_BENCHMARK.md"):
      paired experiment, limits, source identities and retained evidence.
    ]
  ],
)
