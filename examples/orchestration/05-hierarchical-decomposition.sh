#!/usr/bin/env bash
# 05-hierarchical-decomposition.sh — Pattern 4+7: Hierarchical + Human-in-the-Loop
#
# A parent agent creates a migration plan. The user reviews and approves it.
# Then child agents execute sub-tasks in parallel, each receiving the plan
# as context. Finally, a verification step synthesizes the results.
#
# Usage:
#   ./05-hierarchical-decomposition.sh
#   ./05-hierarchical-decomposition.sh "Migrate from REST to GraphQL"
#   ZAG_PROVIDER=claude ./05-hierarchical-decomposition.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-hierarchy-$$"
TASK="${1:-Migrate the database layer from SQLite to PostgreSQL}"

require_zag

header "Hierarchical Decomposition — Migration Pipeline"
info "Task: $TASK"
info "Tag:  $SCRIPT_TAG"
echo

# ===========================================================================
# Phase 1: Plan
# ===========================================================================
header "Phase 1: Planning"
info "Spawning planner agent..."

parent=$(zag spawn $(zag_flags) \
    --name planner \
    --tag "$SCRIPT_TAG" \
    "Create a detailed migration plan for: $TASK

Break the work into exactly three parallel sub-tasks:
1. Schema migration — DDL changes, table structure, indexes
2. Data migration — data transformation, seeding, validation
3. API update — code changes to use the new database layer

For each sub-task, describe the scope, key risks, and acceptance criteria.")
success "Planner spawned: $parent"

info "Waiting for plan..."
zag wait "$parent" --timeout 10m
success "Plan ready"

echo
header "Migration Plan"
zag output "$parent"

# ===========================================================================
# Phase 2: Human approval gate
# ===========================================================================
echo
header "Phase 2: Human Review"
echo
read -rp "  Approve this plan and proceed with execution? [y/N] " answer
if [[ "${answer,,}" != "y" ]]; then
    warn "Plan rejected. Exiting."
    exit 0
fi
success "Plan approved"

# ===========================================================================
# Phase 3: Parallel execution
# ===========================================================================
echo
header "Phase 3: Parallel Execution"
info "Spawning child agents with plan context..."

child_schema=$(zag spawn $(zag_flags) \
    --depends-on "$parent" --inject-context \
    --name schema-migration \
    --tag "$SCRIPT_TAG" \
    "Execute the schema migration sub-task from the plan above. Focus on DDL changes, table structure, and indexes.")
success "Spawned schema migration: $child_schema"

child_data=$(zag spawn $(zag_flags) \
    --depends-on "$parent" --inject-context \
    --name data-migration \
    --tag "$SCRIPT_TAG" \
    "Execute the data migration sub-task from the plan above. Focus on data transformation, seeding, and validation.")
success "Spawned data migration: $child_data"

child_api=$(zag spawn $(zag_flags) \
    --depends-on "$parent" --inject-context \
    --name api-update \
    --tag "$SCRIPT_TAG" \
    "Execute the API update sub-task from the plan above. Focus on code changes to use the new database layer.")
success "Spawned API update: $child_api"

echo
info "Waiting for all child agents to complete..."
info "Use 'zag subscribe --tag $SCRIPT_TAG' to watch progress."
zag wait --tag "$SCRIPT_TAG" --timeout 15m
success "All sub-tasks completed"

# ===========================================================================
# Phase 4: Verification
# ===========================================================================
echo
header "Phase 4: Verification"
info "Synthesizing results from all agents..."
zag pipe $(zag_flags) --tag "$SCRIPT_TAG" -- \
    "Verify that all three migration sub-tasks (schema, data, API) completed successfully. Check for consistency across the changes and identify any integration gaps. Produce a final migration status report."

# ===========================================================================
# Summary
# ===========================================================================
echo
header "Migration Summary"
zag summary --tag "$SCRIPT_TAG"
