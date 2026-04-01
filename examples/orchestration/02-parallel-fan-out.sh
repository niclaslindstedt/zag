#!/usr/bin/env bash
# 02-parallel-fan-out.sh — Pattern 2: Parallel Fan-Out / Gather
#
# Spawns three parallel code reviewers (security, performance, style), waits
# for all of them, collects results, and synthesizes a unified report.
#
# With --race flag: spawns two competing approaches, takes whichever finishes
# first, and cancels the rest.
#
# Usage:
#   ./02-parallel-fan-out.sh                          # Review current codebase
#   ./02-parallel-fan-out.sh "the auth module"        # Review a specific area
#   ./02-parallel-fan-out.sh --race                   # Run the race variant
#   ZAG_PROVIDER=gemini ./02-parallel-fan-out.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-fanout-$$"
RACE_MODE=false

# Parse flags
while [[ $# -gt 0 ]]; do
    case "$1" in
        --race) RACE_MODE=true; shift ;;
        *)      break ;;
    esac
done

TARGET="${1:-the current codebase}"

require_zag

if $RACE_MODE; then
    # -------------------------------------------------------------------
    # Race variant: two approaches, first one wins
    # -------------------------------------------------------------------
    RACE_TAG="orch-race-$$"
    header "Race Pattern — First Approach Wins"
    info "Spawning two competing approaches..."
    echo

    sid1=$(zag spawn $(zag_flags) \
        --name approach-a \
        --tag "$RACE_TAG" \
        "Analyze $TARGET by examining the dependency graph and module boundaries.")
    success "Spawned approach A: $sid1"

    sid2=$(zag spawn $(zag_flags) \
        --name approach-b \
        --tag "$RACE_TAG" \
        "Analyze $TARGET by tracing data flow and I/O patterns.")
    success "Spawned approach B: $sid2"

    echo
    header "Waiting for first completion"
    zag wait --tag "$RACE_TAG" --any --timeout 5m
    success "A winner emerged"

    # Find the winner (first completed session)
    echo
    header "Collecting results"
    results=$(zag collect --tag "$RACE_TAG" --json)
    winner=$(echo "$results" | jq -r '.[] | select(.status=="completed") | .session_id' | head -1)

    if [[ -n "$winner" ]]; then
        success "Winner: $winner"
        echo
        header "Winning Analysis"
        zag output "$winner"
    else
        warn "No session completed successfully."
    fi

    # Cancel the remaining session(s)
    echo
    header "Cancelling remaining sessions"
    zag cancel --tag "$RACE_TAG" 2>/dev/null || true
    success "Cleanup done"
    exit 0
fi

# ---------------------------------------------------------------------------
# Standard fan-out / gather
# ---------------------------------------------------------------------------
header "Parallel Fan-Out — Multi-Perspective Code Review"
info "Target: $TARGET"
info "Tag:    $SCRIPT_TAG"
echo

# ---------------------------------------------------------------------------
# Fan out: spawn three parallel reviewers
# ---------------------------------------------------------------------------
header "Spawning parallel reviewers"

sid_sec=$(zag spawn $(zag_flags) \
    --name security-reviewer \
    --tag "$SCRIPT_TAG" \
    "Review $TARGET for security vulnerabilities. Focus on injection attacks, authentication flaws, and data exposure risks.")
success "Spawned security reviewer: $sid_sec"

sid_perf=$(zag spawn $(zag_flags) \
    --name perf-reviewer \
    --tag "$SCRIPT_TAG" \
    "Review $TARGET for performance issues. Focus on algorithmic complexity, memory allocation patterns, and I/O bottlenecks.")
success "Spawned performance reviewer: $sid_perf"

sid_style=$(zag spawn $(zag_flags) \
    --name style-reviewer \
    --tag "$SCRIPT_TAG" \
    "Review $TARGET for code style and best practices. Focus on naming conventions, error handling patterns, and documentation gaps.")
success "Spawned style reviewer: $sid_style"

# ---------------------------------------------------------------------------
# Wait for all reviewers
# ---------------------------------------------------------------------------
echo
header "Waiting for all reviewers to complete"
info "Use 'zag subscribe --tag $SCRIPT_TAG' in another terminal to watch events."
zag wait --tag "$SCRIPT_TAG" --timeout 10m
success "All reviewers completed"

# ---------------------------------------------------------------------------
# Gather: collect raw results
# ---------------------------------------------------------------------------
echo
header "Collected Results (JSON)"
zag collect --tag "$SCRIPT_TAG" --json

# ---------------------------------------------------------------------------
# Synthesize: pipe all results into a unified report
# ---------------------------------------------------------------------------
echo
header "Synthesized Review Report"
zag pipe $(zag_flags) --tag "$SCRIPT_TAG" -- \
    "Combine these three code review perspectives (security, performance, style) into a single unified report. Organize findings by severity (critical, major, minor) and include actionable recommendations."

# ---------------------------------------------------------------------------
# Statistics
# ---------------------------------------------------------------------------
echo
header "Review Statistics"
zag summary --tag "$SCRIPT_TAG"
