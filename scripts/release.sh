#!/usr/bin/env bash
set -euo pipefail

REPO="niclaslindstedt/agent"
BINARY_NAME="agent"

# --- Helpers ---

die() { echo "error: $*" >&2; exit 1; }

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Build and release the agent CLI to GitHub Releases.

Options:
  --bump <major|minor|patch>   Bump version before releasing (default: none)
  --version <X.Y.Z>           Set an explicit version before releasing
  --draft                      Create release as draft
  --prerelease                 Mark release as prerelease
  --dry-run                    Build and show what would be released, but don't publish
  -h, --help                   Show this help

Examples:
  $(basename "$0") --bump patch        # 0.1.0 -> 0.1.1, build, tag, release
  $(basename "$0") --version 1.0.0     # set to 1.0.0, build, tag, release
  $(basename "$0")                     # release current version (fails if tag exists)
  $(basename "$0") --dry-run           # build only, no publish
EOF
    exit 0
}

# --- Platform detection ---

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) die "unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        armv7l)        arch="armv7" ;;
        *) die "unsupported architecture: $(uname -m)" ;;
    esac

    PLATFORM="${os}-${arch}"
}

# --- Version management ---

get_version() {
    grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

set_version() {
    local new_version="$1"

    # Validate semver format
    if ! echo "$new_version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        die "invalid semver: $new_version (expected X.Y.Z)"
    fi

    echo "setting version to $new_version"

    # Update root Cargo.toml
    sed -i.bak "0,/^version = \".*\"/s//version = \"$new_version\"/" Cargo.toml
    rm -f Cargo.toml.bak

    # Update agent-lib/Cargo.toml
    sed -i.bak "0,/^version = \".*\"/s//version = \"$new_version\"/" agent-lib/Cargo.toml
    rm -f agent-lib/Cargo.toml.bak

    # Update Cargo.lock
    cargo generate-lockfile 2>/dev/null || cargo check 2>/dev/null || true
}

bump_version() {
    local part="$1"
    local current
    current="$(get_version)"

    local major minor patch
    IFS='.' read -r major minor patch <<< "$current"

    case "$part" in
        major) major=$((major + 1)); minor=0; patch=0 ;;
        minor) minor=$((minor + 1)); patch=0 ;;
        patch) patch=$((patch + 1)) ;;
        *) die "invalid bump type: $part (expected major, minor, or patch)" ;;
    esac

    local new_version="${major}.${minor}.${patch}"
    echo "bumping version: $current -> $new_version"
    set_version "$new_version"
}

# --- Build ---

build_release() {
    echo "building release binary for $PLATFORM..."
    cargo build --release

    local src="target/release/${BINARY_NAME}"
    if [ "$PLATFORM" = *"windows"* ]; then
        src="${src}.exe"
    fi

    if [ ! -f "$src" ]; then
        die "build artifact not found: $src"
    fi

    # Create distributable archive
    local version
    version="$(get_version)"
    ASSET_NAME="${BINARY_NAME}-v${version}-${PLATFORM}"

    STAGING_DIR="$(mktemp -d)"
    trap 'rm -rf "$STAGING_DIR"' EXIT

    cp "$src" "$STAGING_DIR/${BINARY_NAME}"
    cp README.md "$STAGING_DIR/" 2>/dev/null || true
    cp LICENSE* "$STAGING_DIR/" 2>/dev/null || true

    if [[ "$PLATFORM" == windows-* ]]; then
        ASSET_PATH="${STAGING_DIR}/${ASSET_NAME}.zip"
        (cd "$STAGING_DIR" && zip -q "${ASSET_NAME}.zip" "$BINARY_NAME" README.md LICENSE* 2>/dev/null || \
         cd "$STAGING_DIR" && zip -q "${ASSET_NAME}.zip" "$BINARY_NAME")
    else
        ASSET_PATH="${STAGING_DIR}/${ASSET_NAME}.tar.gz"
        tar -czf "$ASSET_PATH" -C "$STAGING_DIR" "$BINARY_NAME"
    fi

    # Generate checksum
    CHECKSUM_PATH="${ASSET_PATH}.sha256"
    (cd "$(dirname "$ASSET_PATH")" && sha256sum "$(basename "$ASSET_PATH")" > "$CHECKSUM_PATH")

    echo "built: $ASSET_PATH"
    echo "checksum: $CHECKSUM_PATH"
}

# --- Release ---

create_release() {
    local version tag
    version="$(get_version)"
    tag="v${version}"

    echo ""
    echo "releasing $tag for $PLATFORM"
    echo "  asset: $(basename "$ASSET_PATH")"
    echo "  checksum: $(basename "$CHECKSUM_PATH")"

    if [ "${DRY_RUN:-false}" = "true" ]; then
        echo ""
        echo "[dry-run] would create release $tag and upload:"
        echo "  - $(basename "$ASSET_PATH")"
        echo "  - $(basename "$CHECKSUM_PATH")"
        return
    fi

    # Check if tag already exists on remote
    if git ls-remote --tags origin "refs/tags/$tag" | grep -q "$tag"; then
        die "tag $tag already exists on remote. Bump the version first:\n  $0 --bump patch\n  $0 --version X.Y.Z"
    fi

    # Commit version changes if any files were modified
    if ! git diff --quiet Cargo.toml agent-lib/Cargo.toml Cargo.lock 2>/dev/null; then
        git add Cargo.toml agent-lib/Cargo.toml Cargo.lock
        git commit -m "chore(release): bump version to $version"
    fi

    # Create and push tag
    git tag -a "$tag" -m "Release $tag"
    git push origin HEAD
    git push origin "$tag"

    # Create GitHub release with assets
    local gh_flags=()
    if [ "${DRAFT:-false}" = "true" ]; then
        gh_flags+=(--draft)
    fi
    if [ "${PRERELEASE:-false}" = "true" ]; then
        gh_flags+=(--prerelease)
    fi

    gh release create "$tag" \
        --repo "$REPO" \
        --title "Release $tag" \
        --generate-notes \
        "${gh_flags[@]}" \
        "$ASSET_PATH" \
        "$CHECKSUM_PATH"

    echo ""
    echo "released: https://github.com/$REPO/releases/tag/$tag"
}

# --- Main ---

main() {
    # Ensure we're in the repo root
    if [ ! -f Cargo.toml ]; then
        cd "$(git rev-parse --show-toplevel)" || die "not in a git repository"
    fi

    # Check dependencies
    command -v cargo >/dev/null 2>&1 || die "cargo not found — install Rust: https://rustup.rs"
    command -v gh >/dev/null 2>&1    || die "gh not found — install GitHub CLI: https://cli.github.com"
    command -v git >/dev/null 2>&1   || die "git not found"

    local bump="" explicit_version=""
    DRAFT="false"
    PRERELEASE="false"
    DRY_RUN="false"

    while [ $# -gt 0 ]; do
        case "$1" in
            --bump)      bump="$2"; shift 2 ;;
            --version)   explicit_version="$2"; shift 2 ;;
            --draft)     DRAFT="true"; shift ;;
            --prerelease) PRERELEASE="true"; shift ;;
            --dry-run)   DRY_RUN="true"; shift ;;
            -h|--help)   usage ;;
            *) die "unknown option: $1" ;;
        esac
    done

    if [ -n "$bump" ] && [ -n "$explicit_version" ]; then
        die "--bump and --version are mutually exclusive"
    fi

    detect_platform
    echo "platform: $PLATFORM"
    echo "current version: $(get_version)"

    # Apply version change if requested
    if [ -n "$bump" ]; then
        bump_version "$bump"
    elif [ -n "$explicit_version" ]; then
        set_version "$explicit_version"
    fi

    echo "release version: $(get_version)"
    echo ""

    build_release
    create_release
}

main "$@"
