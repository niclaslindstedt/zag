#!/usr/bin/env bash
set -euo pipefail

# Sync GitHub release notes from CHANGELOG.md.
#
# Usage: sync-releases.sh [version...]
#
# If no versions are given, all versions found in CHANGELOG.md are synced.
# Requires the `gh` CLI to be installed and authenticated.
#
# Examples:
#   sync-releases.sh              # sync all releases
#   sync-releases.sh 0.2.3 0.2.4  # sync specific versions

die() { echo "error: $*" >&2; exit 1; }

command -v gh >/dev/null 2>&1 || die "gh CLI is required (https://cli.github.com)"

REPO_ROOT="$(git rev-parse --show-toplevel)"
CHANGELOG="$REPO_ROOT/CHANGELOG.md"

[ -f "$CHANGELOG" ] || die "CHANGELOG.md not found at $CHANGELOG"

# Extract release notes for a given version from CHANGELOG.md.
# Returns the content between the version's ## header and the next ## header.
extract_notes() {
    local version="$1"
    awk -v ver="$version" '
        /^## \[/ {
            if (found) exit
            if (index($0, "[" ver "]")) { found = 1; next }
        }
        found { print }
    ' "$CHANGELOG" | sed '/./,$!d'
    # Trim leading blank lines; trailing newlines are stripped by command substitution
}

# Collect versions to sync
VERSIONS=()

if [ $# -gt 0 ]; then
    VERSIONS=("$@")
else
    # Extract all versions from CHANGELOG.md
    while IFS= read -r line; do
        ver="$(echo "$line" | sed -nE 's/^## \[([0-9]+\.[0-9]+\.[0-9]+)\].*/\1/p')"
        [ -n "$ver" ] && VERSIONS+=("$ver")
    done < "$CHANGELOG"
fi

[ ${#VERSIONS[@]} -gt 0 ] || die "no versions found"

echo "syncing ${#VERSIONS[@]} release(s) from CHANGELOG.md"
echo ""

SYNCED=0
SKIPPED=0
FAILED=0

for version in "${VERSIONS[@]}"; do
    tag="v$version"

    notes="$(extract_notes "$version")"
    if [ -z "$notes" ]; then
        echo "  skip $tag — no notes in CHANGELOG.md"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Check if the release exists
    if ! gh release view "$tag" >/dev/null 2>&1; then
        echo "  skip $tag — no GitHub release found"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Update the release
    if gh release edit "$tag" --notes "$notes" >/dev/null 2>&1; then
        echo "  sync $tag — updated"
        SYNCED=$((SYNCED + 1))
    else
        echo "  FAIL $tag — gh release edit failed"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "done: $SYNCED synced, $SKIPPED skipped, $FAILED failed"

[ "$FAILED" -eq 0 ] || exit 1
