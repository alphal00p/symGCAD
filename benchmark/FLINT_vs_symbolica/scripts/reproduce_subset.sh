#!/usr/bin/env bash
# End-to-end reproduction of the committed subset (operands/subset.json):
# materialize (decompress + regenerate derived operands, sha256-checked), time
# every job of the campaign job matrix with two pinned single-threaded
# workers, collect, and compare job by job with the committed campaign
# results (exact output hashes must be identical).
#
# usage: reproduce_subset.sh <work-dir> [cpu,cpu]
# Prerequisites (from the repository root):
#   nix develop .#reference -c make -C benchmark/FLINT_vs_symbolica/flint
#   nix develop -c cargo build --release --locked --features flint-bench-hooks --example flint_vs_symbolica
# Run it inside `nix develop .#reference` (python3, GNU time, taskset) with
# SYMGCAD_ENV pointing at the Symbolica license file if <repo>/.symgcad-env is absent.
set -euo pipefail
work=$1; cpus=${2:-0,1}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
folder=$(dirname "$here")
mkdir -p "$work"
work=$(cd "$work" && pwd)
PY=${PYTHON:-python3}
started=$(date -u +%FT%TZ)
"$PY" -I "$here/materialize.py" "$folder/operands/manifest.json" "$folder/operands/sources" "$work/ops" \
  --ids "@$folder/operands/subset.json" --report "$work/materialize.json"
"$PY" -I "$here/orchestrate.py" "$folder/operands/manifest.json" "$work/ops" "$work/results" "$cpus" \
  --ids "@$folder/operands/subset.json" | tee "$work/orchestrate.log"
"$PY" -I "$here/collect.py" "$folder/operands/manifest.json" "$work/results" "$work/out" --no-typst > "$work/collect.log"
status=0
"$PY" -I "$here/compare.py" "$folder/results/per_job.json" "$work/out/per_job.json" \
  --new-summary "$work/out/summary.json" --out "$work/out/reproduction.json" || status=$?
finished=$(date -u +%FT%TZ)
printf '{"started":"%s","finished":"%s","cpus":"%s","compare_exit":%d}\n' "$started" "$finished" "$cpus" "$status" > "$work/out/run.json"
exit $status
