#!/usr/bin/env bash
# 08-meta-bootstrap.sh — Pattern: Meta-Bootstrap (Agent-Authored Orchestration)
#
# Uses `zag --help-agent` and `zag man orchestration` to inject both the CLI
# reference and orchestration pattern guide into a prompt, then asks an agent
# to generate a working orchestration script based on your description. The
# generated script is displayed and saved — you decide when to run it.
#
# This demonstrates the "meta" pattern: agents that understand zag well enough
# to author their own multi-agent workflows.
#
# Usage:
#   ./08-meta-bootstrap.sh "spawn two reviewers (security + perf) and synthesize"
#   ./08-meta-bootstrap.sh "create a generator-critic loop that writes a README"
#   ZAG_PROVIDER=gemini ./08-meta-bootstrap.sh "build a 3-stage analysis pipeline"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-meta-$$"
TASK="${1:?Usage: $0 \"<description of the orchestration script to generate>\"}"
OUTPUT_FILE="${2:-generated-orch.sh}"

require_zag

header "Meta-Bootstrap — Agent-Authored Orchestration"
info "Task: $TASK"
info "Output: $OUTPUT_FILE"
echo

# ---------------------------------------------------------------------------
# Step 1: Capture the CLI and orchestration references
# ---------------------------------------------------------------------------
header "Step 1: Loading zag references"
info "Loading CLI reference (--help-agent)..."
HELP_REF=$(zag --help-agent)
success "Loaded $(echo "$HELP_REF" | wc -l | tr -d ' ') lines of CLI reference"

info "Loading orchestration patterns (man orchestration)..."
ORCH_REF=$(zag man orchestration 2>/dev/null || true)
if [[ -n "$ORCH_REF" ]]; then
    success "Loaded $(echo "$ORCH_REF" | wc -l | tr -d ' ') lines of orchestration patterns"
else
    warn "Could not load orchestration patterns (continuing without)"
fi

# ---------------------------------------------------------------------------
# Step 2: Ask the agent to generate an orchestration script
# ---------------------------------------------------------------------------
echo
header "Step 2: Generating orchestration script"
info "Asking the agent to write a bash script for your task..."

PROMPT="You are an expert at writing bash scripts that orchestrate AI agents using the zag CLI.

Here is the complete zag CLI reference — use it to pick the right commands, flags, and patterns:

${HELP_REF}

Here are the orchestration patterns — use them to choose the best topology for the task:

${ORCH_REF}

Write a bash orchestration script for the following task:
${TASK}

Requirements:
- Start with #!/usr/bin/env bash and set -euo pipefail
- Source lib.sh from the same directory: SCRIPT_DIR=\"\$(cd \"\$(dirname \"\$0\")\" && pwd)\" then source \"\$SCRIPT_DIR/lib.sh\"
- Set a unique SCRIPT_TAG (e.g. orch-<name>-\$\$) for cleanup
- Call require_zag (and require_jq if you use jq)
- Use \$(zag_flags) to pass through provider/model env vars
- Use header, info, and success helpers from lib.sh for output
- Use zag spawn, wait, pipe, collect, exec, output, and other primitives as appropriate
- Clean up by tagging all spawned sessions with SCRIPT_TAG (lib.sh handles cancel on exit)

Output ONLY the bash script. No markdown fences, no explanation, no commentary."

GENERATED=$(zag -q exec $(zag_flags) "$PROMPT" 2>/dev/null)

# Strip markdown fences if the agent wrapped the output anyway
GENERATED=$(echo "$GENERATED" | sed '/^```\(bash\)\{0,1\}$/d')

if [[ -z "$GENERATED" ]]; then
    error "Agent returned empty output. Try a different provider or more specific task."
    exit 1
fi

success "Script generated"

# ---------------------------------------------------------------------------
# Step 3: Display and save
# ---------------------------------------------------------------------------
echo
header "Generated Script"
echo "${_DIM}─────────────────────────────────────────────────────${_RESET}"
echo "$GENERATED"
echo "${_DIM}─────────────────────────────────────────────────────${_RESET}"

echo "$GENERATED" > "$OUTPUT_FILE"
chmod +x "$OUTPUT_FILE"

echo
success "Saved to $OUTPUT_FILE"
info "Review the script, then run it:"
info "  ./$OUTPUT_FILE"
