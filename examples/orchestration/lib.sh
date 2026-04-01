#!/usr/bin/env bash
# lib.sh — Shared helpers for orchestration examples.
# Source this file; do not execute it directly.

set -euo pipefail

# ---------------------------------------------------------------------------
# Provider / model resolution
# ---------------------------------------------------------------------------

# Override via environment: ZAG_PROVIDER, ZAG_MODEL
ZAG_PROVIDER="${ZAG_PROVIDER:-}"
ZAG_MODEL="${ZAG_MODEL:-}"

# Build common zag flags from env vars.
zag_flags() {
    local flags=()
    [[ -n "$ZAG_PROVIDER" ]] && flags+=(-p "$ZAG_PROVIDER")
    [[ -n "$ZAG_MODEL" ]]    && flags+=(--model "$ZAG_MODEL")
    echo "${flags[@]+"${flags[@]}"}"
}

# ---------------------------------------------------------------------------
# Color helpers (respects NO_COLOR)
# ---------------------------------------------------------------------------

if [[ -z "${NO_COLOR:-}" ]] && [[ -t 1 ]]; then
    _BOLD=$'\033[1m'
    _DIM=$'\033[2m'
    _GREEN=$'\033[32m'
    _YELLOW=$'\033[33m'
    _CYAN=$'\033[36m'
    _RED=$'\033[31m'
    _RESET=$'\033[0m'
else
    _BOLD="" _DIM="" _GREEN="" _YELLOW="" _CYAN="" _RED="" _RESET=""
fi

header()  { echo "${_BOLD}${_CYAN}==> $*${_RESET}"; }
info()    { echo "${_DIM}  > $*${_RESET}"; }
success() { echo "${_GREEN}  ✓ $*${_RESET}"; }
warn()    { echo "${_YELLOW}  ! $*${_RESET}"; }
error()   { echo "${_RED}  ✗ $*${_RESET}" >&2; }

# ---------------------------------------------------------------------------
# Prerequisite checks
# ---------------------------------------------------------------------------

require_zag() {
    if ! command -v zag &>/dev/null; then
        error "zag is not installed or not on PATH."
        error "Install it from https://github.com/niclaslindstedt/zag"
        exit 1
    fi
}

require_jq() {
    if ! command -v jq &>/dev/null; then
        error "jq is required for JSON parsing but was not found on PATH."
        error "Install it: https://jqlang.github.io/jq/download/"
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Cleanup trap
# ---------------------------------------------------------------------------

# Each script should set SCRIPT_TAG before sourcing this file or immediately
# after. The cleanup trap cancels all sessions tagged with SCRIPT_TAG on exit.
_cleanup() {
    if [[ -n "${SCRIPT_TAG:-}" ]]; then
        zag cancel --tag "$SCRIPT_TAG" 2>/dev/null || true
    fi
}

trap _cleanup EXIT INT TERM
