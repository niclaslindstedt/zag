#!/usr/bin/env bash
set -euo pipefail

# Create and push a semver release tag.
#
# Usage: release.sh [patch|minor|major]
#
# If no argument is given, the bump type is auto-detected from conventional
# commits since the last tag:
#   - BREAKING CHANGE / feat!  → major
#   - feat                     → minor
#   - fix, perf, docs, test    → patch
#
# The script reads the current version from zag-cli/Cargo.toml, applies the
# bump, creates a git tag (v<version>), and pushes it to origin — which
# triggers the release CI workflow.

die() { echo "error: $*" >&2; exit 1; }

usage() {
    cat <<EOF
Usage: $(basename "$0") [patch|minor|major]

Determine the next semver version and push a release tag.

Arguments:
  patch   Force a patch bump (0.1.2 → 0.1.3)
  minor   Force a minor bump (0.1.2 → 0.2.0)
  major   Force a major bump (0.1.2 → 1.0.0)

If no argument is given, the bump type is auto-detected from conventional
commits since the last tag.
EOF
    exit 0
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# --- Parse arguments ---

BUMP="${1:-}"

if [ "$BUMP" = "--help" ] || [ "$BUMP" = "-h" ]; then
    usage
fi

# `auto` is an alias for "no explicit bump" — lets the workflow_dispatch
# input use a non-empty default while still triggering auto-detection.
if [ "$BUMP" = "auto" ]; then
    BUMP=""
fi

if [ -n "$BUMP" ] && [[ ! "$BUMP" =~ ^(patch|minor|major)$ ]]; then
    die "invalid bump type: $BUMP (expected auto, patch, minor, or major)"
fi

# --- Safety checks ---

if [ -n "$(git status --porcelain)" ]; then
    die "working tree is not clean — commit or stash changes first"
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" != "main" ]; then
    die "releases must be created from the main branch (currently on $BRANCH)"
fi

# --- Read current version ---

CARGO_TOML="$REPO_ROOT/zag-cli/Cargo.toml"
CURRENT_VERSION="$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')"

echo "$CURRENT_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' \
    || die "could not parse current version from $CARGO_TOML: $CURRENT_VERSION"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# --- Determine bump type from commits (if not explicit) ---

PREV_TAG="$(git describe --tags --abbrev=0 2>/dev/null || true)"

if [ -n "$PREV_TAG" ]; then
    COMMITS="$(git log "$PREV_TAG..HEAD" --pretty=format:"%s" --no-merges)"
    COMMIT_COUNT="$(echo "$COMMITS" | grep -c . || true)"
else
    COMMITS="$(git log --pretty=format:"%s" --no-merges)"
    COMMIT_COUNT="$(echo "$COMMITS" | grep -c . || true)"
fi

if [ -z "$BUMP" ]; then
    # Auto-detect from conventional commits
    BUMP="patch"  # default

    while IFS= read -r line; do
        [ -z "$line" ] && continue

        # Check for breaking changes: type! or BREAKING CHANGE
        if echo "$line" | grep -qE '^[a-z]+!(\([^)]*\))?:'; then
            BUMP="major"
            break
        fi
        if echo "$line" | grep -qi 'BREAKING CHANGE'; then
            BUMP="major"
            break
        fi

        # feat → at least minor
        case "$line" in
            feat\(*\):*|feat:*)
                if [ "$BUMP" != "major" ]; then
                    BUMP="minor"
                fi
                ;;
        esac
    done <<< "$COMMITS"

    echo "auto-detected bump: $BUMP (from $COMMIT_COUNT commits since ${PREV_TAG:-initial})"
else
    echo "explicit bump: $BUMP ($COMMIT_COUNT commits since ${PREV_TAG:-initial})"
fi

# --- Apply bump ---

case "$BUMP" in
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
    minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
    patch) PATCH=$((PATCH + 1)) ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
TAG="v$NEW_VERSION"

# --- Validate tag doesn't exist ---

if git rev-parse "$TAG" >/dev/null 2>&1; then
    die "tag $TAG already exists"
fi

# --- Summary ---

echo ""
echo "  current version:  v$CURRENT_VERSION"
echo "  new version:      $TAG"
echo "  bump type:        $BUMP"
echo "  commits included: $COMMIT_COUNT"
echo ""

# --- Create and push tag ---

git tag "$TAG"
echo "created tag $TAG"

git push origin "$TAG"
echo "pushed $TAG to origin — release CI triggered"
