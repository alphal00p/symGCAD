#!/usr/bin/env bash
# One bounded, CPU-pinned, single-threaded measurement (benchmark/FLINT_vs_symbolica).
#
# usage: run_one.sh <op-file> <side: sym|flint> <kernel> <cpu> <timeout-s> <out-base> [min_seconds] [max_reps]
#
# Writes <out-base>.json (timing report of the harness), .canon.json (hashes of
# the exact output, see canon.py), .status (rc, wall, max RSS), .time (GNU time
# -v), .stdout and .stderr. The exact output dump itself is deleted after
# hashing. Environment overrides:
#   SYM_BIN     Symbolica harness   (default <repo>/target/release/examples/flint_vs_symbolica)
#   FLINT_BIN   FLINT harness       (default <folder>/flint/build/flint_bench)
#   SYMGCAD_ENV file exporting SYMBOLICA_LICENSE, sourced for the Symbolica side
#               (default <repo>/.symgcad-env when present)
#   PYTHON      python3 used for canon.py (default python3 on PATH)
#   TIME_BIN    GNU time (default: the `time` executable on PATH)
#   MEM_KIB     address-space limit (default 16 GiB)
set -uo pipefail
op=$1; side=$2; kernel=$3; cpu=$4; secs=$5; base=$6; mins=${7:-1.0}; reps=${8:-25}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
folder=$(dirname "$here")
repo=$(cd "$folder/../.." && pwd)
SYM=${SYM_BIN:-$repo/target/release/examples/flint_vs_symbolica}
FLINT=${FLINT_BIN:-$folder/flint/build/flint_bench}
PY=${PYTHON:-python3}
TIMEB=${TIME_BIN:-$(type -P time)}
mem_kib=${MEM_KIB:-$((16 * 1024 * 1024))}
mkdir -p "$(dirname "$base")"
rm -f "$base.json" "$base.canon.json" "$base.status" "$base.dump"
export RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1
if [ "$side" = sym ]; then
  envfile=${SYMGCAD_ENV:-$repo/.symgcad-env}
  # shellcheck disable=SC1090
  if [ -f "$envfile" ]; then source "$envfile"; fi
  cmd=("$SYM" run "$op" --kernel "$kernel" --dump "$base.dump" --json "$base.json" --min-seconds "$mins" --max-reps "$reps")
else
  cmd=("$FLINT" "$op" "$kernel" "$base.json" "$base.dump" "$mins" "$reps")
fi
start=$(date +%s.%N)
( ulimit -v "$mem_kib"; "$TIMEB" -v -o "$base.time" taskset -c "$cpu" timeout -s KILL "$secs" "${cmd[@]}" ) > "$base.stdout" 2> "$base.stderr"
rc=$?
end=$(date +%s.%N)
rss=$(grep "Maximum resident" "$base.time" 2>/dev/null | awk '{print $NF}')
canon=0
if [ $rc -eq 0 ] && [ -f "$base.dump" ]; then
  if timeout 1800 "$PY" -I "$here/canon.py" "$base.dump" > "$base.canon.json" 2>> "$base.stderr"; then canon=1; fi
fi
rm -f "$base.dump"
printf '{"rc":%d,"wall":%.3f,"max_rss_kib":%s,"canon":%d,"cpu":%s,"timeout":%s}\n' \
  "$rc" "$(awk -v a="$start" -v b="$end" 'BEGIN{printf "%.3f", b-a}')" "${rss:-null}" "$canon" "$cpu" "$secs" > "$base.status"
cat "$base.status"
