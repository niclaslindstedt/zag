#!/usr/bin/env bash
# 04-coordinator-dispatch.sh — Pattern 3: Coordinator / Dispatcher
#
# A lightweight classifier analyzes the task complexity, then routes it to the
# appropriate model size:
#   - simple   → small model (fast, cheap)
#   - moderate → default model
#   - complex  → large model (slow, thorough)
#
# Usage:
#   ./04-coordinator-dispatch.sh "Add a docstring to the main function"
#   ./04-coordinator-dispatch.sh "Redesign the authentication architecture"
#   ZAG_PROVIDER=claude ./04-coordinator-dispatch.sh "Fix the typo in README"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-dispatch-$$"
TASK="${1:?Usage: $0 \"<task description>\"}"

require_zag
require_jq

header "Coordinator / Dispatcher"
info "Task: $TASK"
echo

# ---------------------------------------------------------------------------
# Step 1: Classify the task
# ---------------------------------------------------------------------------
header "Step 1: Classifying task complexity"
info "Using a small model for fast classification..."

classification=$(zag -q exec $(zag_flags) -m small --json \
    "Classify this software engineering task into exactly one category: simple, moderate, or complex. Consider the scope of changes, number of files likely affected, and reasoning difficulty. Output a JSON object with exactly these fields: {\"category\": \"simple|moderate|complex\", \"reasoning\": \"<one sentence explanation>\"}.

Task: $TASK" 2>/dev/null || echo '{"category": "moderate", "reasoning": "classification failed, defaulting to moderate"}')

category=$(echo "$classification" | jq -r '.category // "moderate"' 2>/dev/null || echo "moderate")
reasoning=$(echo "$classification" | jq -r '.reasoning // "unknown"' 2>/dev/null || echo "unknown")

success "Category: $category"
info "Reasoning: $reasoning"

# ---------------------------------------------------------------------------
# Step 2: Route to the appropriate model
# ---------------------------------------------------------------------------
echo
header "Step 2: Executing task"

case "$category" in
    simple)
        info "Routing to small model (fast path)"
        zag exec $(zag_flags) -m small "$TASK"
        ;;
    complex)
        info "Routing to large model (thorough path)"
        sid=$(zag spawn $(zag_flags) -m large \
            --name "complex-task" \
            --tag "$SCRIPT_TAG" \
            "$TASK")
        success "Spawned background session: $sid"

        info "Waiting for completion (large model may take a while)..."
        zag wait "$sid" --timeout 15m

        echo
        header "Result"
        zag output "$sid"

        echo
        header "Summary"
        zag summary "$sid"
        ;;
    *)
        info "Routing to default model"
        zag exec $(zag_flags) "$TASK"
        ;;
esac
