#!/usr/bin/env bash
# Fetch exact external sources without vendoring them into this repository.
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
deps_dir="$repo_root/DO_NOT_PUSH_REFERENCES"
with_reference=false
with_fastsecdec=false
apply_patches=false
check_only=false

usage() {
    cat <<'EOF'
Usage: scripts/bootstrap-deps.sh [--reference] [--fastsecdec] [--apply-patches] [--check]
                                [--directory PATH]

Fetch the revisions recorded in PROVENANCE.toml. The default dependency tree
is pristine and matches the recorded benchmarks. --apply-patches applies the
two optional upstream fixes, once each. Repeated calls are idempotent.
--reference also fetches the independent GCAD sources; --check changes nothing.
--fastsecdec also fetches the pinned no_deformation-branch FastSecDec producer
for optional graph-driven tests. It remains a separate external repository.
--directory overrides the optional external-source directory. Cargo fetches
its public root pins independently; bootstrap is not required for a build.
Existing checkouts at another revision or with unrelated changes are rejected;
this script never resets a checkout or deletes user changes.
EOF
}

while (($#)); do
    case "$1" in
        --reference) with_reference=true ;;
        --fastsecdec) with_fastsecdec=true ;;
        --apply-patches) apply_patches=true ;;
        --check) check_only=true ;;
        --directory)
            shift
            (($#)) || { usage >&2; exit 2; }
            deps_dir=$1 ;;
        -h|--help) usage; exit 0 ;;
        *) printf 'Unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
    shift
done

die() { printf 'Dependency setup: %s\n' "$*" >&2; exit 1; }
value() {
    # PROVENANCE contains simple quoted scalar values; never execute its text.
    awk -v section="$1" -v key="$2" '
        $0 == "[" section "]" { inside=1; next }
        /^\[/ { inside=0 }
        inside && $1 == key && $2 == "=" {
            if ($0 !~ /^[^=]+=[[:space:]]*"[^"[:cntrl:]]*"[[:space:]]*$/) exit 2
            sub(/^[^"]*"/, ""); sub(/"[[:space:]]*$/, ""); print; found++
        }
        END { if (found != 1) exit 2 }
    ' "$repo_root/PROVENANCE.toml"
}

scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT

fetch_repo() {
    local name=$1 url revision dest temporary actual_top
    url=$(value "$name" source)
    revision=$(value "$name" revision)
    [[ $revision =~ ^[0-9a-f]{40}$ ]] || die "Invalid pinned revision for $name"
    [[ $url == https://github.com/* ]] || die "Expected a public HTTPS source for $name"
    dest="$deps_dir/$name"
    if [[ ! -e $dest ]]; then
        $check_only && die "Missing $dest; run setup without --check first"
        mkdir -p -- "$deps_dir"
        temporary=$(mktemp -d "$deps_dir/.${name}-fetch.XXXXXX")
        if ! git -C "$temporary" init --quiet \
            || ! git -C "$temporary" remote add origin "$url" \
            || ! git -C "$temporary" fetch --quiet --depth 1 origin "$revision" \
            || ! git -C "$temporary" checkout --quiet --detach FETCH_HEAD; then
            printf 'Incomplete fetch retained at %s\n' "$temporary" >&2
            die "Could not fetch $name"
        fi
        mv -- "$temporary" "$dest"
    fi
    [[ -d $dest/.git || -f $dest/.git ]] || die "$dest is not a standalone checkout"
    actual_top=$(git -C "$dest" rev-parse --show-toplevel)
    [[ $(cd -- "$dest" && pwd -P) == $(cd -- "$actual_top" && pwd -P) ]] \
        || die "$dest belongs to another checkout"
    [[ $(git -C "$dest" rev-parse HEAD) == "$revision" ]] \
        || die "$name is at another revision; preserving it unchanged"
    printf '%s %s\n' "$name" "$revision"
}

symbolica_patches=(
    "$repo_root/patches/symbolica-linear-resultant-sign.patch"
    "$repo_root/patches/numerica-integer-left-shift.patch"
)

validate_symbolica() {
    local dest="$deps_dir/symbolica" revision index patch
    revision=$(value symbolica revision)
    index="$scratch/expected.index"
    rm -f -- "$index"
    GIT_INDEX_FILE="$index" git -C "$dest" read-tree "$revision"
    for patch in "${symbolica_patches[@]}"; do
        [[ -f $patch ]] || die "Missing patch $patch"
        if git -C "$dest" apply --reverse --check "$patch" 2>/dev/null; then
            GIT_INDEX_FILE="$index" git -C "$dest" apply --cached "$patch"
            printf '  applied: %s\n' "${patch##*/}"
        elif git -C "$dest" apply --check "$patch" 2>/dev/null; then
            printf '  available: %s\n' "${patch##*/}"
            if $apply_patches; then
                $check_only && die "Requested patch is not applied: ${patch##*/}"
                git -C "$dest" apply "$patch"
                GIT_INDEX_FILE="$index" git -C "$dest" apply --cached "$patch"
            fi
        else
            die "Patch does not match pinned source: ${patch##*/}"
        fi
    done
    git -C "$dest" diff --binary "$revision" -- > "$scratch/actual.diff"
    GIT_INDEX_FILE="$index" git -C "$dest" diff --cached --binary "$revision" -- > "$scratch/expected.diff"
    cmp -s "$scratch/actual.diff" "$scratch/expected.diff" \
        || die "Symbolica contains unrelated changes; preserving them unchanged"
    [[ -z $(git -C "$dest" ls-files --others --exclude-standard) ]] \
        || die "Symbolica contains untracked files; preserving them unchanged"
    for name in numerica graphica; do
        [[ $(git -C "$dest" rev-parse "$revision:lib/$name") == $(value "$name" tree) ]] \
            || die "Unexpected $name source tree"
    done
}

fetch_repo symbolica
# Validate before mutation, including exact recognition of already-applied fixes.
requested_patches=$apply_patches
apply_patches=false
validate_symbolica
apply_patches=$requested_patches
if $apply_patches; then validate_symbolica; fi

if $with_reference; then
    fetch_repo gcad
    [[ -z $(git -C "$deps_dir/gcad" status --porcelain) ]] \
        || die "GCAD has local changes; preserving them unchanged"
fi
if $with_fastsecdec; then
    fetch_repo fastSecDec
    [[ -z $(git -C "$deps_dir/fastSecDec" status --porcelain) ]] \
        || die "FastSecDec has local changes; preserving them unchanged"
fi
printf 'Dependency setup verified. External sources stay outside Git tracking.\n'
