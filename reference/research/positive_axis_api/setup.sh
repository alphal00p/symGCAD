#!/usr/bin/env bash
# Create an isolated, pinned replay tree; never mutate the active dependencies.
set -euo pipefail

package_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_dir=$(cd -- "$package_dir/../../.." && pwd -P)
# Replay the authenticated historical source, not whatever packaging/API changes
# happen to be at HEAD. The local/public commits have exactly the same tree.
package_revision=$(git -C "$repo_dir" rev-parse HEAD)
symgcad_revision=2d1b630a1170e05d3fb276bd8bc8dd0464bda4ca
historical_local_revision=6d7b9838d65d4c50bafab5fcc98428338eb0a05e
historical_tree=1e1cae62b867d7f84da47f3322309629811144f3
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
if ! git -C "$repo_dir" cat-file -e "$symgcad_revision^{commit}" 2>/dev/null; then
    if git -C "$repo_dir" cat-file -e "$historical_local_revision^{commit}" 2>/dev/null; then
        symgcad_revision=$historical_local_revision
    else
        printf 'Missing historical source. Fetch public commit %s from origin first.\n' "$symgcad_revision" >&2
        exit 1
    fi
fi
[[ $(git -C "$repo_dir" rev-parse "$symgcad_revision^{tree}") == "$historical_tree" ]] || {
    printf 'Unexpected historical source tree; refusing replay.\n' >&2
    exit 1
}
while read -r expected source_path; do
    actual=$(git -C "$repo_dir" show "$symgcad_revision:$source_path" | sha256sum)
    [[ ${actual%% *} == "$expected" ]] || {
        printf 'Historical source hash mismatch: %s\n' "$source_path" >&2
        exit 1
    }
done < "$package_dir/DEPENDENCY_SHA256SUMS"
bash "$repo_dir/scripts/bootstrap-deps.sh" --check
[[ $(git -C "$symbolica_source" rev-parse HEAD) == "$symbolica_revision" \
    && -z $(git -C "$symbolica_source" status --porcelain) ]] || {
    printf 'The active pinned Symbolica checkout must be pristine; it will not be changed.\n' >&2
    exit 1
}
git -C "$repo_dir" cat-file -e "$symgcad_revision^{commit}"
git -C "$symbolica_source" cat-file -e "$symbolica_revision^{commit}"

# Failures leave their fresh directory for inspection. Existing files are never
# overwritten or reset. The historical public commit and exact tree/hash gates
# preserve the original local-path Cargo resolution without rewriting evidence.
mkdir -- "$replay_dir"
replay_dir=$(cd -- "$replay_dir" && pwd -P)
git -C "$repo_dir" archive "$symgcad_revision" | tar -x -C "$replay_dir"
(cd -- "$replay_dir" && sha256sum -c "$package_dir/DEPENDENCY_SHA256SUMS")
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
    printf 'symgcad_revision=%s\nsymgcad_tree=%s\npackage_checkout_revision=%s\nsymbolica_revision=%s\n' \
        "$symgcad_revision" "$historical_tree" "$package_revision" "$symbolica_revision"
    printf 'patch_order=positive_axis,unit_shift\n'
    sha256sum "$package_dir/setup.sh" "$package_dir/README.md" "$package_dir/SHA256SUMS"
    sha256sum examples/positive_axis_probe.rs examples/unit_shift_regression.rs \
        src/roots/integer_sign.rs src/roots.rs Cargo.toml Cargo.lock \
        DO_NOT_PUSH_REFERENCES/symbolica/src/poly/univariate.rs \
        DO_NOT_PUSH_REFERENCES/symbolica/src/poly/univariate/roots.rs \
        DO_NOT_PUSH_REFERENCES/symbolica/src/poly/univariate/tests.rs
) > "$replay_dir/REPLAY_PROVENANCE.txt"
printf 'Prepared isolated replay tree: %s\n' "$replay_dir"
printf 'No build or algebra computation was started. See the packaged README.\n'
