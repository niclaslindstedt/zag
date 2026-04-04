#!/usr/bin/env bash
set -euo pipefail

# Fetch the latest GitHub release for open-source upstream providers.
#
# Usage: fetch-upstream-releases.sh [provider]
#
# With no arguments, checks all open-source providers.
# With a provider name, checks only that provider.

die() { echo "error: $*" >&2; exit 1; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Provider -> GitHub repo mapping (closed-source providers excluded)
declare -A GITHUB_REPOS=(
    [claude]="anthropics/claude-code"
    [codex]="openai/codex"
    [gemini]="google-gemini/gemini-cli"
    [ollama]="ollama/ollama"
)

PROVIDERS_ORDERED=(claude codex gemini ollama)

fetch_release() {
    local provider="$1"
    local repo="$2"
    local url="https://api.github.com/repos/${repo}/releases/latest"
    local curl_args=(-sS --max-time 10)

    # Use GitHub token if available (avoids rate limiting)
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        curl_args+=(-H "Authorization: token ${GITHUB_TOKEN}")
    fi

    local response
    response=$(curl "${curl_args[@]}" "$url" 2>/dev/null) || {
        printf "  %-10s  %-20s  %-12s  %s\n" "$provider" "[fetch failed]" "-" "-"
        return
    }

    local tag name published_at html_url
    tag=$(echo "$response" | grep -oP '"tag_name"\s*:\s*"\K[^"]+' | head -1) || tag=""
    published_at=$(echo "$response" | grep -oP '"published_at"\s*:\s*"\K[^"]+' | head -1) || published_at=""
    html_url=$(echo "$response" | grep -oP '"html_url"\s*:\s*"\K[^"]+' | head -1) || html_url=""

    if [ -z "$tag" ]; then
        printf "  %-10s  %-20s  %-12s  %s\n" "$provider" "[no release found]" "-" "-"
        return
    fi

    # Trim date to YYYY-MM-DD
    local date="${published_at%%T*}"

    printf "  %-10s  %-20s  %-12s  %s\n" "$provider" "$tag" "$date" "$html_url"
}

# Determine which providers to check
if [ $# -gt 0 ]; then
    provider="$1"
    if [ "$provider" = "copilot" ]; then
        die "copilot is closed-source — no GitHub release to check"
    fi
    if [ -z "${GITHUB_REPOS[$provider]+x}" ]; then
        die "unknown provider: $provider (valid: ${PROVIDERS_ORDERED[*]}, copilot is closed-source)"
    fi
    targets=("$provider")
else
    targets=("${PROVIDERS_ORDERED[@]}")
fi

echo "Latest upstream releases"
echo
printf "  %-10s  %-20s  %-12s  %s\n" "PROVIDER" "TAG" "DATE" "URL"
printf "  %-10s  %-20s  %-12s  %s\n" "--------" "---" "----" "---"

for p in "${targets[@]}"; do
    fetch_release "$p" "${GITHUB_REPOS[$p]}"
done

echo
