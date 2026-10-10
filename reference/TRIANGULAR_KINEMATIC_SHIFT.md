# Exact triangular kinematic shift experiment

`examples/triangular_shift_probe.rs` prepares a research problem under
`original_axis = new_axis + h(earlier coordinates)`. It checks the exact inverse
identity for every normalized strict constraint and split polynomial, retains
all constraints and groups, and refuses changes to an explicit/given order.
A shift depending on the target or a later coordinate is rejected. Three
focused helper tests passed.

For full symbolic A446, the order remains
`s,t,x5,x1,x2,x3,x4,x7,x8`, but the internal coordinate named `t` now means
`s + original_t`. Equivalently, preparation substitutes `t-s` for original
`t`. The outer restriction `s>4` and all positive Feynman axes are unchanged;
original `t` is unrestricted. The split polynomial shrinks from 78 to 68 terms.
This is a triangular translation, unlike the nontriangular sum-scale ordering
shortcut excluded by `TWO_COORDINATE_CHART_AUDIT.md`.

The map preserves cylindrical order in principle: at a fixed earlier prefix
every target-axis root, interval and sample is translated by the same exact
amount, preserving root order and its full-real ordinal. Later-axis boundaries
require substituting the inverse into their polynomial. Positive-only root
ordinals on the translated axis need separate recounting; they cannot be copied
blindly when a translation moves zero. Every exceptional polynomial must also
be transported. A complete implementation must independently verify this
transport and the nested transformed proof before claiming original coverage.
**This helper does not implement original-cell export or proof transport.**

The first transformed solve, using the frozen b813 debug worker, timed out after
120.039 seconds, with 114,168 KiB peak RSS. Its last snapshot reports 78,124
speculative nodes and 2,582 refinements; phase costs include 25.720 seconds for
factor handling, 1.475 for resultants and 5.570 for domain certificates. These
are exploratory measurements, with a cold transformed checkpoint store. No
matched speedup or original/transformed completion is claimed.

Artifacts are under `artifacts/research/a446-kinematic-shift-01/`. Preparation's
source SHA256 is `75a51a1b359b0cef36e4b65965bfe830df13d80f1e04babc6c58df0218667047`;
the transformed TOML SHA256 is
`df04822152e2776bf20056b80ca13729be546ba9631a83819314341f7a14cd53`.
`preparation/preparation.json` records all polynomial term counts and inverse
checks; `debug-trial/` retains worker input, process status and checkpoints.
