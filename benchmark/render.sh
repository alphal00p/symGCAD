#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# A public checkout can render the committed, audited table snapshot directly.
# Regeneration additionally needs the retained (ignored) campaign evidence.
if [[ "${1:-}" == --regenerate ]]; then
    completion="${2:-artifacts/completion-aggregate-303.json}"
    args=(--completion "$completion" --output benchmark/data.json)
    if [[ -f artifacts/slow-suite-one-hour-01/queue.json ]]; then
        args+=(--slow-status artifacts/slow-suite-one-hour-01/queue.json)
    fi
    nix develop --command cargo run --example benchmark_report -- "${args[@]}"
elif [[ $# -ne 0 ]]; then
    printf '%s\n' 'usage: benchmark/render.sh [--regenerate [completion-aggregate.json]]' >&2
    exit 2
fi
nix develop --command typst compile benchmark/benchmark.typ benchmark/benchmark.pdf
sha256sum benchmark/data.json benchmark/benchmark.typ benchmark/benchmark.pdf
