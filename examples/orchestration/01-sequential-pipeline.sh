#!/usr/bin/env bash
# 01-sequential-pipeline.sh — Pattern 1: Sequential Pipeline
#
# Three-stage code analysis pipeline where each stage's output feeds into the
# next via --depends-on and --inject-context:
#
#   Stage A: Analyze code structure
#   Stage B: Identify issues from the analysis
#   Stage C: Create a prioritized action plan
#
# Also demonstrates the `pipe` alternative for ad-hoc chaining.
#
# Usage:
#   ./01-sequential-pipeline.sh                   # Analyze current directory
#   ./01-sequential-pipeline.sh src/auth.rs       # Analyze a specific file
#   ZAG_PROVIDER=gemini ./01-sequential-pipeline.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-pipeline-$$"
TARGET="${1:-.}"

require_zag

# ---------------------------------------------------------------------------
header "Sequential Pipeline — Three-Stage Code Analysis"
info "Target: $TARGET"
info "Tag:    $SCRIPT_TAG"
echo

# ---------------------------------------------------------------------------
# Stage A: Analyze code structure
# ---------------------------------------------------------------------------
header "Stage A: Analyzing code structure"
sid_a=$(zag spawn $(zag_flags) \
    --name stage-parse \
    --tag "$SCRIPT_TAG" \
    "Analyze the code structure of $TARGET. List the main modules, public APIs, and key data types.")
success "Spawned stage A: $sid_a"

# ---------------------------------------------------------------------------
# Stage B: Identify issues (depends on A)
# ---------------------------------------------------------------------------
header "Stage B: Identifying issues"
sid_b=$(zag spawn $(zag_flags) \
    --depends-on "$sid_a" --inject-context \
    --name stage-issues \
    --tag "$SCRIPT_TAG" \
    "Based on the code structure analysis above, identify bugs, anti-patterns, and areas for improvement.")
success "Spawned stage B: $sid_b (depends on A)"

# ---------------------------------------------------------------------------
# Stage C: Create action plan (depends on B)
# ---------------------------------------------------------------------------
header "Stage C: Creating action plan"
sid_c=$(zag spawn $(zag_flags) \
    --depends-on "$sid_b" --inject-context \
    --name stage-plan \
    --tag "$SCRIPT_TAG" \
    "Based on the issues identified above, create a prioritized action plan with effort estimates.")
success "Spawned stage C: $sid_c (depends on B)"

# ---------------------------------------------------------------------------
# Wait for the full pipeline to finish
# ---------------------------------------------------------------------------
echo
header "Waiting for pipeline to complete"
info "The stages run sequentially via --depends-on."
info "Use 'zag listen $sid_c' in another terminal to follow progress."
zag wait "$sid_c" --timeout 10m
success "Pipeline complete"

# ---------------------------------------------------------------------------
# Show status of each stage
# ---------------------------------------------------------------------------
echo
header "Stage Status"
for sid in "$sid_a" "$sid_b" "$sid_c"; do
    status=$(zag status "$sid" 2>/dev/null || echo "unknown")
    info "$sid: $status"
done

# ---------------------------------------------------------------------------
# Print the final result
# ---------------------------------------------------------------------------
echo
header "Final Action Plan"
zag output "$sid_c"

# ---------------------------------------------------------------------------
# Show pipeline summary
# ---------------------------------------------------------------------------
echo
header "Pipeline Summary"
zag summary --tag "$SCRIPT_TAG"

# ---------------------------------------------------------------------------
# Bonus: demonstrate pipe as an alternative chaining method
# ---------------------------------------------------------------------------
echo
header "Bonus: Using 'pipe' to synthesize stages A + B"
info "pipe combines the outputs of completed sessions into a new prompt."
zag pipe $(zag_flags) "$sid_a" "$sid_b" -- \
    "Synthesize the code structure analysis and issue identification into a concise executive summary."
