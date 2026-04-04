#!/usr/bin/env bash
set -euo pipefail

# Check the current state of provider definitions against upstream CLIs.
#
# Usage: check-provider-status.sh [provider]
#
# With no arguments, checks all providers. With a provider name, checks
# only that provider. Extracts models, defaults, and size mappings from
# the Rust source, probes installed upstream CLIs, and fetches the latest
# GitHub release for open-source providers.
#
# Each section is independent — the script works even when no CLIs are
# installed (source extraction always succeeds).

die() { echo "error: $*" >&2; exit 1; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# ---------------------------------------------------------------------------
# Provider metadata
# ---------------------------------------------------------------------------

ALL_PROVIDERS=(claude codex copilot gemini ollama)

# Source files (relative to REPO_ROOT)
declare -A SRC_FILES=(
    [claude]="zag-agent/src/providers/claude/mod.rs"
    [codex]="zag-agent/src/providers/codex.rs"
    [copilot]="zag-agent/src/providers/copilot.rs"
    [gemini]="zag-agent/src/providers/gemini.rs"
    [ollama]="zag-agent/src/providers/ollama.rs"
)

# CLI binary names
declare -A CLI_BINARIES=(
    [claude]="claude"
    [codex]="codex"
    [copilot]="copilot"
    [gemini]="gemini"
    [ollama]="ollama"
)

# Alternative binary names to try if primary is missing
declare -A CLI_ALT_BINARIES=(
    [copilot]="gh copilot"
)

# GitHub repos (empty = closed source)
declare -A GITHUB_REPOS=(
    [claude]="anthropics/claude-code"
    [codex]="openai/codex"
    [copilot]=""
    [gemini]="google-gemini/gemini-cli"
    [ollama]="ollama/ollama"
)

# Help subcommands to probe per provider
declare -A HELP_COMMANDS=(
    [claude]="--help"
    [codex]="--help"
    [copilot]="--help"
    [gemini]="--help"
    [ollama]="--help"
)

# Additional help subcommands (space-separated)
declare -A EXTRA_HELP_COMMANDS=(
    [codex]="exec --help"
    [ollama]="run --help"
)

# ---------------------------------------------------------------------------
# Source extraction helpers
# ---------------------------------------------------------------------------

extract_models() {
    local file="$1"
    local const_name="${2:-AVAILABLE_MODELS}"
    # Handle both single-line and multi-line array definitions
    grep -A 20 "pub const ${const_name}" "$file" \
        | sed '/\];/q' \
        | grep -oP '"[^"]*"' | tr -d '"'
}

extract_const() {
    local file="$1"
    local name="$2"
    grep "pub const ${name}" "$file" | grep -oP '"[^"]*"' | tr -d '"' | head -1
}

extract_size_mappings() {
    local file="$1"
    local fn_name="${2:-model_for_size}"
    sed -n "/fn ${fn_name}/,/^[[:space:]]*\}/p" "$file" \
        | grep 'ModelSize::' \
        | sed 's/.*ModelSize::\([A-Za-z]*\).*"\([^"]*\)".*/\1=\2/'
}

extract_flags() {
    local file="$1"
    # Extract string literals pushed/extended into args vectors in build_*_args methods
    sed -n '/fn build_.*args/,/^[[:space:]]*fn \|^impl /p' "$file" \
        | grep -oP '"--[a-z][a-z0-9-]*"' | tr -d '"' | sort -u
}

# ---------------------------------------------------------------------------
# CLI probe helpers
# ---------------------------------------------------------------------------

find_binary() {
    local provider="$1"
    local primary="${CLI_BINARIES[$provider]}"

    if command -v "$primary" &>/dev/null; then
        echo "$primary"
        return 0
    fi

    local alt="${CLI_ALT_BINARIES[$provider]:-}"
    if [ -n "$alt" ]; then
        # For multi-word commands like "gh copilot", check the base binary
        local base="${alt%% *}"
        if command -v "$base" &>/dev/null; then
            echo "$alt"
            return 0
        fi
    fi

    return 1
}

probe_cli() {
    local provider="$1"
    local binary="$2"
    local help_cmd="${HELP_COMMANDS[$provider]}"
    local tmp_file="/tmp/zag-check-${provider}-help.txt"

    echo "    CLI binary:       $binary"

    # Try to get version
    local version
    version=$($binary --version 2>/dev/null | head -1) || version="(unknown)"
    echo "    CLI version:      $version"

    # Capture main help
    if $binary $help_cmd > "$tmp_file" 2>&1; then
        local line_count
        line_count=$(wc -l < "$tmp_file")
        echo "    Help output:      ${line_count} lines saved to $tmp_file"
    else
        echo "    Help output:      [failed to capture]"
    fi

    # Capture extra help subcommands
    local extra="${EXTRA_HELP_COMMANDS[$provider]:-}"
    if [ -n "$extra" ]; then
        local extra_file="/tmp/zag-check-${provider}-${extra// /-}-help.txt"
        if $binary $extra > "$extra_file" 2>&1; then
            local extra_lines
            extra_lines=$(wc -l < "$extra_file")
            echo "    Extra help:       $binary $extra (${extra_lines} lines -> $extra_file)"
        fi
    fi
}

# ---------------------------------------------------------------------------
# GitHub release helper
# ---------------------------------------------------------------------------

fetch_release() {
    local provider="$1"
    local repo="${GITHUB_REPOS[$provider]}"

    if [ -z "$repo" ]; then
        echo "  Latest release:   [SKIP] closed source"
        return
    fi

    if ! command -v curl &>/dev/null; then
        echo "  Latest release:   [SKIP] curl not available"
        return
    fi

    local url="https://api.github.com/repos/${repo}/releases/latest"
    local curl_args=(-sS --max-time 10)
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        curl_args+=(-H "Authorization: token ${GITHUB_TOKEN}")
    fi

    local response
    response=$(curl "${curl_args[@]}" "$url" 2>/dev/null) || {
        echo "  Latest release:   [SKIP] fetch failed"
        return
    }

    local tag published_at html_url
    tag=$(echo "$response" | grep -oP '"tag_name"\s*:\s*"\K[^"]+' | head -1) || tag=""

    if [ -z "$tag" ]; then
        echo "  Latest release:   [SKIP] no release found"
        return
    fi

    published_at=$(echo "$response" | grep -oP '"published_at"\s*:\s*"\K[^"]+' | head -1) || published_at=""
    html_url=$(echo "$response" | grep -oP '"html_url"\s*:\s*"\K[^"]+' | head -1) || html_url=""
    local date="${published_at%%T*}"

    echo "  Latest release:   $tag ($date)"
    echo "                    $html_url"
}

# ---------------------------------------------------------------------------
# Per-provider check
# ---------------------------------------------------------------------------

check_provider() {
    local provider="$1"
    local src="${REPO_ROOT}/${SRC_FILES[$provider]}"

    echo "=== $provider ==="
    echo

    if [ ! -f "$src" ]; then
        echo "  [ERROR] Source file not found: ${SRC_FILES[$provider]}"
        echo
        return
    fi

    # --- Last updated ---
    local last_updated
    last_updated=$(grep -oP '// provider-updated: \K\S+' "$src" 2>/dev/null) || last_updated="(no marker)"
    echo "  Last updated:     $last_updated"
    echo

    # --- Source state ---
    echo "  Source state (${SRC_FILES[$provider]}):"

    if [ "$provider" = "ollama" ]; then
        local default_model default_size
        default_model=$(extract_const "$src" "DEFAULT_MODEL")
        default_size=$(extract_const "$src" "DEFAULT_SIZE")
        echo "    DEFAULT_MODEL:    $default_model"
        echo "    DEFAULT_SIZE:     $default_size"

        echo -n "    AVAILABLE_SIZES:  "
        extract_models "$src" "AVAILABLE_SIZES" | tr '\n' ' '
        echo

        echo "    Size mappings:"
        extract_size_mappings "$src" "size_for_model_size" | while read -r mapping; do
            echo "      $mapping"
        done
    else
        local default_model
        default_model=$(extract_const "$src" "DEFAULT_MODEL")
        echo "    DEFAULT_MODEL:    $default_model"

        echo -n "    AVAILABLE_MODELS: "
        extract_models "$src" | tr '\n' ' '
        echo

        echo "    Size mappings:"
        extract_size_mappings "$src" | while read -r mapping; do
            echo "      $mapping"
        done
    fi

    echo -n "    Flags in args:    "
    extract_flags "$src" | tr '\n' ' '
    echo
    echo

    # --- Upstream CLI ---
    echo "  Upstream CLI:"
    local binary
    if binary=$(find_binary "$provider"); then
        probe_cli "$provider" "$binary"
    else
        echo "    [SKIP] ${CLI_BINARIES[$provider]} not installed"
    fi
    echo

    # --- GitHub release ---
    fetch_release "$provider"
    echo
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if [ $# -gt 0 ]; then
    provider="$1"
    found=false
    for p in "${ALL_PROVIDERS[@]}"; do
        if [ "$p" = "$provider" ]; then
            found=true
            break
        fi
    done
    if [ "$found" = false ]; then
        die "unknown provider: $provider (valid: ${ALL_PROVIDERS[*]})"
    fi
    targets=("$provider")
else
    targets=("${ALL_PROVIDERS[@]}")
fi

echo "Provider status report"
echo "======================"
echo

for p in "${targets[@]}"; do
    check_provider "$p"
done
