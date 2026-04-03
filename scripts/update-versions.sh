#!/usr/bin/env bash
set -euo pipefail

# Update version strings across all project files.
#
# Usage: update-versions.sh <version>
#
# Updates versions in all Cargo.toml files, binding manifests,
# and regenerates Cargo.lock.

die() { echo "error: $*" >&2; exit 1; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

VERSION="${1:-}"
[ -n "$VERSION" ] || die "usage: update-versions.sh <version>"

# Validate semver
echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' || die "invalid semver: $VERSION"

echo "updating all version files to $VERSION"

# --- Rust crates ---

for toml in zag-cli/Cargo.toml zag-agent/Cargo.toml zag-orch/Cargo.toml bindings/rust/Cargo.toml; do
    sed -i.bak "0,/^version = \".*\"/s//version = \"$VERSION\"/" "$toml"
    rm -f "$toml.bak"
    echo "  updated $toml"
done

# Update dependency versions in bindings/rust/Cargo.toml
sed -i.bak "s/zag-agent = { version = \"[^\"]*\"/zag-agent = { version = \"$VERSION\"/" bindings/rust/Cargo.toml
sed -i.bak "s/zag-orch = { version = \"[^\"]*\"/zag-orch = { version = \"$VERSION\"/" bindings/rust/Cargo.toml
rm -f bindings/rust/Cargo.toml.bak

# --- TypeScript ---

sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" bindings/typescript/package.json
rm -f bindings/typescript/package.json.bak
echo "  updated bindings/typescript/package.json"

# --- Python ---

sed -i.bak "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" bindings/python/pyproject.toml
rm -f bindings/python/pyproject.toml.bak
echo "  updated bindings/python/pyproject.toml"

# --- C# ---

sed -i.bak "s/<Version>[^<]*<\/Version>/<Version>$VERSION<\/Version>/" bindings/csharp/src/Zag/Zag.csproj
rm -f bindings/csharp/src/Zag/Zag.csproj.bak
echo "  updated bindings/csharp/src/Zag/Zag.csproj"

# --- Regenerate Cargo.lock ---

echo "  regenerating Cargo.lock..."
cargo generate-lockfile 2>/dev/null || cargo check 2>/dev/null || true

echo "all versions updated to $VERSION"
