#!/usr/bin/env bash
# Explicitly select a research backend in an isolated symGCAD working copy.
# This changes only the supplied manifest: no fetch, lock update, or build.
set -euo pipefail

usage() {
    printf 'Usage: %s /path/to/isolated/Cargo.toml /path/to/symbolica\n' "$0" >&2
}
die() { printf 'Local Symbolica selection: %s\n' "$*" >&2; exit 1; }
[[ $# == 2 ]] || { usage; exit 2; }
[[ -f $1 && ! -L $1 ]] || die 'Manifest must be an existing regular, non-symlink file'
manifest_dir=$(cd -- "$(dirname -- "$1")" && pwd -P)
manifest="$manifest_dir/$(basename -- "$1")"
[[ -d $2 ]] || die 'Symbolica directory does not exist'
backend=$(cd -- "$2" && pwd -P)
# All generated TOML paths are literal double-quoted strings. Reject unusual
# control/escape characters rather than silently changing their meaning.
case "$backend" in
    *'"'*|*'\'*) die 'Backend path contains unsupported TOML escape characters' ;;
esac
[[ ! $backend =~ [[:cntrl:]] ]] || die 'Backend path contains control characters'
for entry in symbolica numerica graphica; do
    case "$entry" in
        symbolica) package_manifest="$backend/Cargo.toml" ;;
        *) package_manifest="$backend/lib/$entry/Cargo.toml" ;;
    esac
    [[ -f $package_manifest ]] || die "Missing $package_manifest"
    awk -v expected="$entry" '
        /^\[package\][[:space:]]*$/ { inside=1; next }
        /^\[/ { inside=0 }
        inside && $1 == "name" && $2 == "=" && $3 == "\"" expected "\"" { found++ }
        END { exit found != 1 }
    ' "$package_manifest" || die "Unexpected package name in $package_manifest"
done

original=$(mktemp "$manifest_dir/.local-symbolica-original.XXXXXX")
prepared=$(mktemp "$manifest_dir/.local-symbolica-prepared.XXXXXX")
trap 'rm -f -- "$original" "$prepared"' EXIT
cp -- "$manifest" "$original"
awk -v backend="$backend" '
    /^\[/ { section=$0 }
    section == "[dependencies]" && /^symbolica[[:space:]]*=/ {
        dependencies++
        if ($0 !~ /^symbolica[[:space:]]*=[[:space:]]*\{[[:space:]]*version[[:space:]]*=[[:space:]]*"3\.0\.1",.*\}[[:space:]]*$/) bad=1
    }
    section == "[patch.crates-io]" && /^(symbolica|numerica|graphica)[[:space:]]*=/ {
        name=$1; sub(/=.*/, "", name)
        seen[name]++
        if ($0 !~ /^[^=]+=[[:space:]]*\{.*\}[[:space:]]*$/) bad=1
        path=backend
        if (name != "symbolica") path=path "/lib/" name
        print name " = { path = \"" path "\" }"
        next
    }
    { print }
    END {
        if (bad || dependencies != 1 || seen["symbolica"] != 1 || seen["numerica"] != 1 || seen["graphica"] != 1) exit 2
    }
' "$original" > "$prepared" || die 'Expected registry Symbolica 3.0.1 dependency and three unique single-line root patches; manifest left unchanged'
cmp -s -- "$manifest" "$original" || die 'Manifest changed concurrently; refusing to overwrite it'
if cmp -s -- "$manifest" "$prepared"; then
    printf 'Local backend already selected: %s\n' "$backend"
else
    chmod --reference="$manifest" "$prepared"
    mv -- "$prepared" "$manifest"
    printf 'Selected local backend: %s\n' "$backend"
fi
printf 'Only the manifest changed. Resolve the isolated lockfile before --locked builds:\n'
printf '  cargo metadata --format-version 1 --manifest-path %q > local-backend-metadata.json\n' "$manifest"
printf 'Review the lockfile diff; selecting the same revision should change only the three source identities.\n'
printf 'Then inspect cargo metadata to confirm all three manifest_path values point into this backend.\n'
printf 'Preserve the normal public-pinned manifest and lockfile outside this isolated copy.\n'
