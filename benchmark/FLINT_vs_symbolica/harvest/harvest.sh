#!/usr/bin/env bash
# Harvest real symGCAD operations: run the optimized projection of one input
# with every completed algebra operation captured (SYMGCAD_CAPTURE_ALL).
#
# usage: harvest.sh <problem> <cpu> <out-dir> [timeout-s]
#   <problem>   name of harvest/inputs/<problem>.toml (see selection.json)
#   <timeout-s> defaults to the campaign's value in selection.json; g33S2 and
#               symA317 were killed at their deadline (captures then depend on
#               machine speed, so scripts/materialize.py, not a re-harvest, is
#               the exact way back to the committed operands).
# Needs the symgcad binary built with the benchmark hooks:
#   cargo build --release --locked --features flint-bench-hooks --bin symgcad
# Environment: SYMGCAD_BIN (default <repo>/target/release/symgcad),
#              SYMGCAD_ENV (license file, default <repo>/.symgcad-env).
# Output: <out-dir>/<problem>/{input.toml,capture/op-*.json,capture/active-*.json,result.json,time.txt,status.txt}
set -uo pipefail
prob=$1; cpu=$2; out=$3
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
folder=$(dirname "$here")
repo=$(cd "$folder/../.." && pwd)
secs=${4:-$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["harvest"][sys.argv[2]]["timeout_seconds"])' "$here/selection.json" "$prob")}
BIN=${SYMGCAD_BIN:-$repo/target/release/symgcad}
TIMEB=${TIME_BIN:-$(type -P time)}
input=$here/inputs/$prob.toml
[ -f "$input" ] || { echo "no input $input" >&2; exit 2; }
mkdir -p "$out"
d=$(cd "$out" && pwd)/$prob
rm -rf "$d"
mkdir -p "$d/capture" "$d/tmp" "$d/checkpoints"
# A fresh checkpoint directory: cached operations would not be recomputed and
# therefore not captured.
awk -v cp="$d/checkpoints" '{print} /^\[solver\]$/{print "checkpoint_dir = \"" cp "\""}' "$input" > "$d/input.toml"
cd "$d" || exit 2
envfile=${SYMGCAD_ENV:-$repo/.symgcad-env}
# shellcheck disable=SC1090
if [ -f "$envfile" ]; then source "$envfile"; fi
export TMPDIR="$d/tmp" SYMGCAD_CAPTURE_DIR="$d/capture" SYMGCAD_CAPTURE_ALL=1 RAYON_NUM_THREADS=1
( ulimit -v $((16 * 1024 * 1024)); "$TIMEB" -v -o "$d/time.txt" taskset -c "$cpu" timeout -s KILL "$secs" "$BIN" project "$d/input.toml" --output "$d/result.json" ) > "$d/stdout.txt" 2> "$d/stderr.txt"
echo "rc=$?" > "$d/status.txt"
echo "$prob $(cat "$d/status.txt") ops=$(find "$d/capture" -name 'op-*.json' | wc -l)"
