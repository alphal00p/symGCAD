#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

suite=${1:-fast}
if (($#)); then shift; fi
case "$suite" in
  fast)
    # Small exact algebra, decomposition, verifier and worker regressions.
    # Full external-corpus campaigns are deliberately a separate invocation.
    exec cargo test --lib --bins --tests "$@" -- --test-threads=1
    ;;
  slow)
    # Every unresolved original is retained in the checked classification.
    # The Rust supervisor enforces per-case deadlines and aggregate resources.
    exec cargo run --release --example slow_suite -- "$@"
    ;;
  *)
    printf 'Usage: scripts/test.sh [fast [cargo-options] | slow [suite-options]]\n' >&2
    exit 2
    ;;
esac
