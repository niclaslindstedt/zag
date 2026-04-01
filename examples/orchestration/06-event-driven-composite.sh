#!/usr/bin/env bash
# 06-event-driven-composite.sh — Pattern 8+9: A2A Communication + Composite
#
# Two named agents (frontend-dev, backend-dev) work on a feature in parallel.
# The script demonstrates inter-agent messaging, broadcasting, and event-driven
# reactions via watch.
#
# Usage:
#   ./06-event-driven-composite.sh
#   ./06-event-driven-composite.sh "Add user profile page with avatar upload"
#   ZAG_PROVIDER=claude ./06-event-driven-composite.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-composite-$$"
FEATURE="${1:-Add a user profile page with avatar upload}"

require_zag

header "Event-Driven Composite — Agent Team Collaboration"
info "Feature: $FEATURE"
info "Tag:     $SCRIPT_TAG"
echo

# ===========================================================================
# Spawn the team
# ===========================================================================
header "Spawning agent team"

sid_be=$(zag spawn $(zag_flags) \
    --name backend-dev \
    --tag "$SCRIPT_TAG" \
    "You are a backend developer. Implement the backend API for: $FEATURE

Design the REST endpoints, data models, and validation logic. Document the API contract (request/response formats) clearly so the frontend team can integrate.")
success "Spawned backend-dev: $sid_be"

sid_fe=$(zag spawn $(zag_flags) \
    --name frontend-dev \
    --tag "$SCRIPT_TAG" \
    "You are a frontend developer. Implement the frontend UI for: $FEATURE

Create the React components, state management, and API integration layer. Assume a standard REST API will be available.")
success "Spawned frontend-dev: $sid_fe"

# ===========================================================================
# Inter-agent communication
# ===========================================================================
echo
header "Inter-Agent Communication"
info "Frontend asking backend about the API contract..."

zag input --name backend-dev \
    "What will the API endpoints look like? Please share the request/response format so I can build the frontend integration."
success "Message sent to backend-dev"

# ===========================================================================
# Broadcast to all agents
# ===========================================================================
echo
header "Broadcasting to all agents"
info "Sending team-wide guidance..."

zag broadcast --tag "$SCRIPT_TAG" \
    "Team reminder: ensure all implementations include proper error handling, input validation, and loading states. Follow the project's existing patterns."
success "Broadcast sent to all agents with tag '$SCRIPT_TAG'"

# ===========================================================================
# Event-driven watcher (background)
# ===========================================================================
echo
header "Setting up event watcher"
info "Watching for session completions..."

# Run watcher in background — it will print when any agent finishes
zag watch --tag "$SCRIPT_TAG" --on session_ended --once -- \
    echo "Agent completed: {session_id}" &
WATCH_PID=$!
success "Watcher running in background (PID: $WATCH_PID)"

# ===========================================================================
# Wait for both agents
# ===========================================================================
echo
header "Waiting for team to complete"
info "Use 'zag subscribe --tag $SCRIPT_TAG' in another terminal to watch events."
zag wait --tag "$SCRIPT_TAG" --timeout 10m
success "All agents completed"

# Clean up the watcher
kill $WATCH_PID 2>/dev/null || true

# ===========================================================================
# Integration review
# ===========================================================================
echo
header "Integration Review"
info "Piping both implementations into a reviewer..."
zag pipe $(zag_flags) --tag "$SCRIPT_TAG" -- \
    "Review both the frontend and backend implementations for this feature. Check that:
1. The API contract is consistent between frontend and backend
2. Error handling is aligned (frontend handles all backend error codes)
3. Data models match across the stack
4. Authentication/authorization is properly implemented
Produce a brief integration report with any issues found."

# ===========================================================================
# Summary
# ===========================================================================
echo
header "Team Summary"
zag summary --tag "$SCRIPT_TAG"
