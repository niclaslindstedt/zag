#!/usr/bin/env bash
# 07-decision-arena.sh — Adversarial Debate for Better Decisions
#
# Spawns an advocate (argues FOR) and a skeptic (argues AGAINST), lets them
# exchange rebuttals via A2A messaging, then pipes everything to a judge
# that synthesizes a balanced verdict.
#
# Optionally, each debater can use a different provider — e.g., Claude as
# advocate, Gemini as skeptic — to get genuinely diverse reasoning styles.
#
# Usage:
#   ./07-decision-arena.sh                                                     # Default question
#   ./07-decision-arena.sh "Should we adopt Kubernetes for our 5-person startup?"
#   ./07-decision-arena.sh --advocate claude --skeptic gemini "Should we...?"   # Mixed providers
#   ZAG_PROVIDER=claude ZAG_MODEL=large ./07-decision-arena.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="arena-$$"

# ---------------------------------------------------------------------------
# Parse flags
# ---------------------------------------------------------------------------

ADVOCATE_PROVIDER=""
SKEPTIC_PROVIDER=""
JUDGE_PROVIDER=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --advocate)  ADVOCATE_PROVIDER="$2"; shift 2 ;;
        --skeptic)   SKEPTIC_PROVIDER="$2";  shift 2 ;;
        --judge)     JUDGE_PROVIDER="$2";    shift 2 ;;
        *)           break ;;
    esac
done

QUESTION="${1:-Should we rewrite our monolith in microservices?}"

# Build per-role flag sets (fall back to ZAG_PROVIDER / ZAG_MODEL)
_role_flags() {
    local role_provider="$1"
    local flags=()
    if [[ -n "$role_provider" ]]; then
        flags+=(-p "$role_provider")
    elif [[ -n "$ZAG_PROVIDER" ]]; then
        flags+=(-p "$ZAG_PROVIDER")
    fi
    [[ -n "$ZAG_MODEL" ]] && flags+=(--model "$ZAG_MODEL")
    echo "${flags[@]+"${flags[@]}"}"
}
advocate_flags() { _role_flags "$ADVOCATE_PROVIDER"; }
skeptic_flags()  { _role_flags "$SKEPTIC_PROVIDER"; }
judge_flags()    { _role_flags "$JUDGE_PROVIDER"; }

require_zag

# ---------------------------------------------------------------------------
header "Decision Arena — Adversarial Debate"
info "Question:  $QUESTION"
info "Tag:       $SCRIPT_TAG"
[[ -n "$ADVOCATE_PROVIDER" ]] && info "Advocate:  $ADVOCATE_PROVIDER"
[[ -n "$SKEPTIC_PROVIDER" ]]  && info "Skeptic:   $SKEPTIC_PROVIDER"
[[ -n "$JUDGE_PROVIDER" ]]    && info "Judge:     $JUDGE_PROVIDER"
echo

# ===========================================================================
# Phase 1: Opening Arguments (parallel fan-out)
# ===========================================================================
header "Phase 1: Opening Arguments"
info "Spawning advocate and skeptic in parallel..."

sid_advocate=$(zag spawn $(advocate_flags) \
    --name advocate \
    --tag "$SCRIPT_TAG" \
    "You are the ADVOCATE in a structured decision debate.

Question: $QUESTION

Build the strongest possible case FOR this decision. Structure your argument:
1. **Core thesis** — your central argument in one sentence
2. **Key benefits** (3-5) — concrete, specific advantages with evidence or reasoning
3. **Risk mitigation** — how common objections can be addressed
4. **Success criteria** — what would make this decision clearly correct in hindsight

Be persuasive but intellectually honest. Use concrete examples where possible.
Do NOT hedge or present counterarguments — that is the skeptic's job.")
success "Spawned advocate: $sid_advocate"

sid_skeptic=$(zag spawn $(skeptic_flags) \
    --name skeptic \
    --tag "$SCRIPT_TAG" \
    "You are the SKEPTIC in a structured decision debate.

Question: $QUESTION

Build the strongest possible case AGAINST this decision. Structure your argument:
1. **Core objection** — your central counterargument in one sentence
2. **Key risks** (3-5) — concrete, specific dangers, costs, or downsides
3. **Hidden costs** — what proponents typically overlook or underestimate
4. **Failure scenarios** — what would make this decision clearly wrong in hindsight

Be rigorous and incisive but intellectually honest. Use concrete examples where possible.
Do NOT hedge or present the other side — that is the advocate's job.")
success "Spawned skeptic: $sid_skeptic"

# ===========================================================================
# Phase 2: Wait for opening arguments
# ===========================================================================
echo
header "Phase 2: Waiting for Opening Arguments"
info "Both agents are working in parallel..."
info "Use 'zag listen $sid_advocate' or 'zag listen $sid_skeptic' to follow."
zag wait "$sid_advocate" "$sid_skeptic" --timeout 10m
success "Both opening arguments received"

# ===========================================================================
# Phase 3: Cross-Pollination — Rebuttals (A2A communication)
# ===========================================================================
echo
header "Phase 3: Cross-Pollination"

advocate_arg=$(zag output "$sid_advocate")
skeptic_arg=$(zag output "$sid_skeptic")

info "Sending advocate's argument to skeptic for rebuttal..."
zag input --name skeptic \
    "The advocate has made their case. Here is their argument:

---
$advocate_arg
---

Now write a REBUTTAL. Address their strongest points directly:
1. Which of their benefits are overstated or unrealistic?
2. What critical evidence or context did they ignore?
3. Why their risk mitigations are insufficient?

Be specific — reference their exact claims."
success "Rebuttal request sent to skeptic"

info "Sending skeptic's argument to advocate for rebuttal..."
zag input --name advocate \
    "The skeptic has made their case. Here is their argument:

---
$skeptic_arg
---

Now write a REBUTTAL. Address their strongest points directly:
1. Which of their risks are overstated or unlikely?
2. What critical benefits or context did they ignore?
3. Why their failure scenarios can be prevented?

Be specific — reference their exact claims."
success "Rebuttal request sent to advocate"

# ===========================================================================
# Phase 4: Wait for rebuttals
# ===========================================================================
echo
header "Phase 4: Waiting for Rebuttals"
info "Both agents are crafting rebuttals..."
zag wait "$sid_advocate" "$sid_skeptic" --timeout 10m
success "Both rebuttals received"

# ===========================================================================
# Phase 5: Judge — Balanced Synthesis
# ===========================================================================
echo
header "Phase 5: Judge's Verdict"
info "Piping all arguments and rebuttals to the judge..."

zag pipe $(judge_flags) --tag "$SCRIPT_TAG" -- \
    "You are the JUDGE in a structured decision debate.

Question: $QUESTION

You have received the complete debate record: the advocate's opening argument
and rebuttal, and the skeptic's opening argument and rebuttal.

Produce a balanced analysis:

## Strongest Points FOR
The 2-3 most compelling arguments from the advocate that survived rebuttal.

## Strongest Points AGAINST
The 2-3 most compelling arguments from the skeptic that survived rebuttal.

## Key Uncertainties
What factors would most change the answer? What information is missing?

## Recommendation
A clear verdict — PROCEED, DO NOT PROCEED, or PROCEED WITH CONDITIONS — with
a confidence level (low / medium / high) and a one-paragraph rationale.

## Decision Checklist
If proceeding, list 3-5 concrete conditions or safeguards that should be in place.

Be fair to both sides. Your job is clarity, not compromise."

# ===========================================================================
# Summary
# ===========================================================================
echo
header "Arena Summary"
zag summary --tag "$SCRIPT_TAG"
