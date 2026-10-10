#!/usr/bin/env bash
# Create an isolated, pinned replay tree; never mutate the active dependencies.
set -euo pipefail

package_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_dir=$(cd -- "$package_dir/../../.." && pwd -P)
symgcad_revision=$(git -C "$repo_dir" rev-parse HEAD)
symbolica_revision=75f8350094b90254ee71dc2a391fde0d14b0204a
symbolica_source="$repo_dir/DO_NOT_PUSH_REFERENCES/symbolica"

if [[ $# != 1 || $1 != /* ]]; then
    printf 'Usage: %s /absolute/path/to/new-replay-directory\n' "$0" >&2
    exit 2
fi
replay_dir=$1
[[ ! -e "$replay_dir" && ! -L "$replay_dir" ]] || {
    printf 'Replay destination must be new: %s\n' "$replay_dir" >&2
    exit 1
}
(cd -- "$package_dir" && sha256sum -c SHA256SUMS)
[[ -z $(git -C "$repo_dir" status --porcelain) ]] || {
    printf 'The symGCAD checkout must be clean; preserve or commit local work first.\n' >&2
    exit 1
}
(cd -- "$repo_dir" && sha256sum -c "$package_dir/DEPENDENCY_SHA256SUMS")
bash "$repo_dir/scripts/bootstrap-deps.sh" --check
[[ $(git -C "$symbolica_source" rev-parse HEAD) == "$symbolica_revision" \
    && -z $(git -C "$symbolica_source" status --porcelain) ]] || {
    printf 'The active pinned Symbolica checkout must be pristine; it will not be changed.\n' >&2
    exit 1
}
git -C "$repo_dir" cat-file -e "$symgcad_revision^{commit}"
git -C "$symbolica_source" cat-file -e "$symbolica_revision^{commit}"

# Failures leave their fresh directory for inspection. Existing files are never
# overwritten or reset. The public checkout's clean HEAD is recorded, avoiding
# dependence on unpublished development-history commit IDs.
mkdir -- "$replay_dir"
replay_dir=$(cd -- "$replay_dir" && pwd -P)
git -C "$repo_dir" archive "$symgcad_revision" | tar -x -C "$replay_dir"
replay_native="$replay_dir/DO_NOT_PUSH_REFERENCES/symbolica"
mkdir -p -- "$replay_native"
git -C "$symbolica_source" archive "$symbolica_revision" | tar -x -C "$replay_native"

# A private index anchors patch paths here, even when replay_dir is inside the
# main checkout. No commit or fetch is needed in this isolated backend tree.
git -C "$replay_native" init --quiet
for patch_name in symbolica-positive-axis.patch symbolica-unit-shift.patch; do
    git -C "$replay_native" apply --check "$package_dir/$patch_name"
    git -C "$replay_native" apply "$package_dir/$patch_name"
done
cp -- "$package_dir/positive_axis_probe.rs" "$replay_dir/examples/positive_axis_probe.rs"
cp -- "$package_dir/unit_shift_regression.rs" "$replay_dir/examples/unit_shift_regression.rs"
(
    cd -- "$replay_dir"
    printf 'symgcad_revision=%s\nsymbolica_revision=%s\n' "$symgcad_revision" "$symbolica_revision"
    printf 'patch_order=positive_axis,unit_shift\n'
    sha256sum examples/positive_axis_probe.rs examples/unit_shift_regression.rs \
        src/roots/integer_sign.rs src/roots.rs Cargo.toml Cargo.lock \
        DO_NOT_PUSH_REFERENCES/symbolica/src/poly/univariate.rs \
        DO_NOT_PUSH_REFERENCES/symbolica/src/poly/univariate/roots.rs \
        DO_NOT_PUSH_REFERENCES/symbolica/src/poly/univariate/tests.rs
) > "$replay_dir/REPLAY_PROVENANCE.txt"
printf 'Prepared isolated replay tree: %s\n' "$replay_dir"
printf 'No build or algebra computation was started. See the packaged README.\n'
