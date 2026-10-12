#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Renders the committed report (benchmark.typ reads report-data.json) and the separately
# preserved ordering/MCTS companion. Rebuilding report-data.json from fresh runs is done by
# benchmark/run_final.sh (see benchmark/README.md).
# --regenerate rebuilds the frozen 2026-10-10 audit snapshot data.json, which the report reads
# only as history; it needs the retained (ignored) campaign evidence.
if [[ "${1:-}" == --regenerate ]]; then
    completion="${2:-artifacts/completion-aggregate-303.json}"
    args=(--completion "$completion" --output benchmark/data.json --followups benchmark/followups.json)
    if [[ -f artifacts/slow-suite-one-hour-audit-09/queue-snapshot.json ]]; then
        args+=(--slow-status artifacts/slow-suite-one-hour-audit-09/queue-snapshot.json)
    elif [[ -f artifacts/slow-suite-one-hour-01/queue.json ]]; then
        args+=(--slow-status artifacts/slow-suite-one-hour-01/queue.json)
    fi
    nix develop --command cargo run --example benchmark_report -- "${args[@]}"
elif [[ $# -ne 0 ]]; then
    printf '%s\n' 'usage: benchmark/render.sh [--regenerate [completion-aggregate.json]]' >&2
    exit 2
fi
nix develop --command typst compile benchmark/benchmark.typ benchmark/benchmark.pdf
nix develop --command typst compile benchmark/ordering-mcts.typ benchmark/ordering-mcts.pdf
sha256sum benchmark/report-data.json benchmark/benchmark.typ benchmark/improvements.typ benchmark/benchmark.pdf \
    benchmark/data.json benchmark/ordering-mcts.typ benchmark/ordering-mcts.pdf
