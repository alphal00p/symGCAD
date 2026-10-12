#!/usr/bin/env bash
# Final benchmark in one command: cold symGCAD runs of every corpus group (solve +
# independent verify), the opt-in projective rows, the report data and the PDF.
#
# usage: benchmark/run_final.sh <symgcad-binary> [options]
#   --out DIR                 run directory (default: artifacts/bench-report/symgcad-<sha12>)
#   --cpus a,b,...            pinned CPUs, one per concurrent group (default 376,377)
#   --parallel N              concurrent groups (default 2; must be <= number of CPUs)
#   --timeout S               solve wall limit per group (default 600)
#   --repeats N               cold runs of every group (default 3); the report shows the median
#                             solve and verify times, and every repeat must verify the same cells
#   --attempt-unresolved S    also give the 28 groups unresolved in data.json a cold attempt of
#                             S seconds each (default: not attempted; their long attempts are shown)
#   --native DIR              native GCAD 300 s run directory (default: the 2026-10-12 campaign)
#   --skip-runs               only rebuild report-data.json and the PDF from existing runs
#
# Repeat k > 1 writes to <out>/symgcad-k and <out>/projective-k.  Every result.json is kept
# (about 6.5 GiB per repeat of the strict runs) so that each proof can be re-verified later.
#
# Every stage is resumable: the runner skips groups already recorded for the same
# binary sha256.  Edit <out>/symgcad/parallel.txt to change parallelism while running,
# touch <out>/symgcad/STOP to drain.  Needs the Symbolica license environment in
# .symgcad-env (main checkout) and Nix.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/.." && pwd)
[[ $# -ge 1 ]] || { sed -n '2,23p' "$0" >&2; exit 2; }
bin=$(realpath "$1"); shift
out="" cpus=376,377 parallel=2 timeout=600 timeout_unresolved=0 skip_runs=0 repeats=3
native=/common/dev/gcad/artifacts/research/claude-2026-10-12/bench/native_300s_run
while [[ $# -gt 0 ]]; do
    case $1 in
        --out) out=$2; shift 2 ;;
        --cpus) cpus=$2; shift 2 ;;
        --parallel) parallel=$2; shift 2 ;;
        --timeout) timeout=$2; shift 2 ;;
        --repeats) repeats=$2; shift 2 ;;
        --attempt-unresolved) timeout_unresolved=$2; shift 2 ;;
        --native) native=$2; shift 2 ;;
        --skip-runs) skip_runs=1; shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
sha=$(sha256sum "$bin" | cut -c1-64)
out=${out:-$repo/artifacts/bench-report/symgcad-${sha:0:12}}
mkdir -p "$out"
[[ $repeats =~ ^[1-9][0-9]*$ ]] || { echo "--repeats must be a positive integer" >&2; exit 2; }
symgcad_dirs=("$out/symgcad") projective_dirs=("$out/projective")
for ((k = 2; k <= repeats; k++)); do symgcad_dirs+=("$out/symgcad-$k"); projective_dirs+=("$out/projective-$k"); done
env_file=${SYMGCAD_ENV:-/common/dev/gcad/.symgcad-env}
py() { nix develop "$repo#reference" --command python3 -I "$@"; }

if [[ $skip_runs -eq 0 ]]; then
    only=()
    if [[ $timeout_unresolved == 0 ]]; then
        timeout_unresolved=$timeout
        mapfile -t only < <(py - "$here/campaign/groups.json" <<'EOF'
import json, sys
for e in json.load(open(sys.argv[1]))["entries"]:
    if e["status_in_data_json"] == "verified":
        print(e["index"])
EOF
)
    fi
    # Symbolica license (never printed).
    set +x
    # shellcheck disable=SC1090
    source "$env_file"
    for ((k = 0; k < repeats; k++)); do
        echo "== cold symGCAD runs, repeat $((k + 1))/$repeats -> ${symgcad_dirs[k]}"
        py "$here/campaign/symgcad_runner.py" --binary "$bin" --parallel "$parallel" --cpus "$cpus" \
            --out "${symgcad_dirs[k]}" --timeout "$timeout" --timeout-unresolved "$timeout_unresolved" \
            ${only[@]:+--only "${only[@]}"}
        echo "== opt-in projective contract rows, repeat $((k + 1))/$repeats -> ${projective_dirs[k]}"
        py "$here/campaign/symgcad_runner.py" --binary "$bin" --parallel 1 --cpus "${cpus%%,*}" \
            --out "${projective_dirs[k]}" --timeout "$timeout" --only 298 302 \
            --recipe-override "298=$here/campaign/projective/298_gluza30_positive_projective.toml" \
            --recipe-override "302=$here/campaign/projective/302_gluza33_positive_projective.toml"
    done
fi

echo "== report data -> benchmark/report-data.json"
existing() { for d in "$@"; do [[ -f $d/records.jsonl ]] && printf '%s\n' "$d"; done; }
mapfile -t symgcad_dirs < <(existing "${symgcad_dirs[@]}")
mapfile -t projective_dirs < <(existing "${projective_dirs[@]}")
py "$here/build_report.py" --symgcad "${symgcad_dirs[@]}" ${projective_dirs[@]:+--projective "${projective_dirs[@]}"} --native "$native" \
    --output "$here/report-data.json"
echo "== PDF -> benchmark/benchmark.pdf"
(cd "$repo" && nix develop --command typst compile benchmark/benchmark.typ benchmark/benchmark.pdf)
sha256sum "$here/report-data.json" "$here/benchmark.typ" "$here/benchmark.pdf"
