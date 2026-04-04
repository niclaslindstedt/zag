#!/usr/bin/env bash
set -euo pipefail

# Generate a Keep a Changelog entry from conventional commits.
#
# Usage: generate-changelog.sh <version> [previous-tag]
#
# If previous-tag is omitted, it is auto-detected from git tags.
# If no previous tag exists, all commits are included.

die() { echo "error: $*" >&2; exit 1; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
CHANGELOG="$REPO_ROOT/CHANGELOG.md"

VERSION="${1:-}"
PREV_TAG="${2:-}"

[ -n "$VERSION" ] || die "usage: generate-changelog.sh <version> [previous-tag]"

# Validate semver
echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' || die "invalid semver: $VERSION"

# --- Determine commit range ---

if [ -z "$PREV_TAG" ]; then
    PREV_TAG="$(git describe --tags --abbrev=0 --exclude="v$VERSION" 2>/dev/null || true)"
fi

if [ -n "$PREV_TAG" ]; then
    echo "generating changelog: $PREV_TAG..HEAD -> v$VERSION"
    COMMITS="$(git log "$PREV_TAG..HEAD" --pretty=format:"%s" --no-merges)"
else
    echo "generating changelog: all commits -> v$VERSION (no previous tag found)"
    COMMITS="$(git log --pretty=format:"%s" --no-merges)"
fi

# --- Parse and categorize commits ---

# Only include: feat, fix, docs, test, perf
# Skip: chore, refactor, ci, style, build
FEAT_LINES=""
FIX_LINES=""
DOCS_LINES=""
TEST_LINES=""
PERF_LINES=""

format_line() {
    local msg="$1"
    # Strip conventional commit prefix: type(scope): or type:
    msg="$(echo "$msg" | sed -E 's/^(feat|fix|docs|test|perf)(\([^)]*\))?:[[:space:]]*//')"
    # Capitalize first letter
    msg="$(echo "$msg" | sed -E 's/^(.)/\U\1/')"
    echo "- $msg"
}

while IFS= read -r line; do
    [ -z "$line" ] && continue

    case "$line" in
        feat\(*\):*|feat:*)
            FEAT_LINES+="$(format_line "$line")"$'\n' ;;
        fix\(*\):*|fix:*)
            FIX_LINES+="$(format_line "$line")"$'\n' ;;
        docs\(*\):*|docs:*)
            DOCS_LINES+="$(format_line "$line")"$'\n' ;;
        test\(*\):*|test:*)
            TEST_LINES+="$(format_line "$line")"$'\n' ;;
        perf\(*\):*|perf:*)
            PERF_LINES+="$(format_line "$line")"$'\n' ;;
    esac
done <<< "$COMMITS"

# --- Build the new section ---

DATE="$(date +%Y-%m-%d)"
SECTION="## [$VERSION] - $DATE"$'\n'

has_content=false

if [ -n "$FEAT_LINES" ]; then
    SECTION+=$'\n'"### Added"$'\n\n'"$FEAT_LINES"
    has_content=true
fi

if [ -n "$FIX_LINES" ]; then
    SECTION+=$'\n'"### Fixed"$'\n\n'"$FIX_LINES"
    has_content=true
fi

if [ -n "$PERF_LINES" ]; then
    SECTION+=$'\n'"### Performance"$'\n\n'"$PERF_LINES"
    has_content=true
fi

if [ -n "$DOCS_LINES" ]; then
    SECTION+=$'\n'"### Documentation"$'\n\n'"$DOCS_LINES"
    has_content=true
fi

if [ -n "$TEST_LINES" ]; then
    SECTION+=$'\n'"### Tests"$'\n\n'"$TEST_LINES"
    has_content=true
fi

if [ "$has_content" = false ]; then
    SECTION+=$'\n'"- No notable changes"$'\n'
fi

# --- Update CHANGELOG.md ---

if [ ! -f "$CHANGELOG" ]; then
    # Create fresh changelog
    cat > "$CHANGELOG" <<EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

$SECTION
EOF
    echo "created $CHANGELOG"
else
    # Prepend new section after the header (everything before first ## [)
    HEADER=""
    BODY=""
    found_first_entry=false

    while IFS= read -r line; do
        if [ "$found_first_entry" = false ] && echo "$line" | grep -qE '^## \['; then
            found_first_entry=true
        fi

        if [ "$found_first_entry" = true ]; then
            BODY+="$line"$'\n'
        else
            HEADER+="$line"$'\n'
        fi
    done < "$CHANGELOG"

    # Write: header + new section + blank line + existing body
    {
        printf '%s' "$HEADER"
        printf '%s\n' "$SECTION"
        printf '%s' "$BODY"
    } > "$CHANGELOG"

    echo "updated $CHANGELOG"
fi

echo "changelog entry for v$VERSION generated successfully"
