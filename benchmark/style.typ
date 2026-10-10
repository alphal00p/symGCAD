#let ink = rgb("182A3A")
#let muted = rgb("566676")
#let line = rgb("DCE3E8")
#let green = rgb("DDF1E3")
#let yellow = rgb("FFF1BC")
#let red = rgb("F9DADA")
#let report-style(body) = {
  set page(paper: "a4", flipped: true, margin: (x: 11mm, top: 12mm, bottom: 12mm),
  footer: context [#text(size: 7pt, fill: muted)[symGCAD · exact computation and ordering #h(1fr) #counter(page).display("1")]])
  set text(font: "Libertinus Sans", size: 9pt, fill: ink)
  set par(leading: 0.55em)
  set heading(numbering: none)
  show heading.where(level: 1): set text(size: 23pt, weight: "bold")
  show heading.where(level: 2): set text(size: 12pt, weight: "bold")

  body
}
