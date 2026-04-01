#!/usr/bin/env bash
# 03-generator-critic.sh — Pattern 5+6: Generator/Critic with Iterative Refinement
#
# One agent generates code, another scores it against quality criteria.
# If the score is below the threshold, feedback loops back to the generator.
# The cycle repeats until the output passes or max iterations are reached.
#
# Usage:
#   ./03-generator-critic.sh
#   ./03-generator-critic.sh "Write a thread-safe LRU cache in Rust"
#   ZAG_PROVIDER=claude ZAG_MODEL=large ./03-generator-critic.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

SCRIPT_TAG="orch-gencrit-$$"
TASK="${1:-Write a function that parses and validates email addresses in Python. Include proper error handling and RFC 5322 compliance.}"
MAX_ITERS=3
PASS_THRESHOLD=8

require_zag
require_jq

header "Generator / Critic — Iterative Refinement"
info "Task:      $TASK"
info "Threshold: score >= $PASS_THRESHOLD / 10"
info "Max iters: $MAX_ITERS"
echo

attempt=0
feedback=""

while [ $attempt -lt $MAX_ITERS ]; do
    attempt=$((attempt + 1))
    header "Iteration $attempt of $MAX_ITERS"

    # -------------------------------------------------------------------
    # Generate
    # -------------------------------------------------------------------
    if [ -z "$feedback" ]; then
        gen_prompt="$TASK"
    else
        gen_prompt="Revise your previous implementation based on this feedback: $feedback

Original task: $TASK"
    fi

    info "Generating..."
    gen_sid=$(zag spawn $(zag_flags) \
        --name "generator-$attempt" \
        --tag "$SCRIPT_TAG" \
        "$gen_prompt")
    zag wait "$gen_sid" --timeout 5m

    # -------------------------------------------------------------------
    # Critique
    # -------------------------------------------------------------------
    info "Running critic..."
    critic_result=$(zag -q exec $(zag_flags) --context "$gen_sid" --json \
        "Score this code on a scale of 1 to 10 for correctness, performance, and readability. Output a JSON object with exactly these fields: {\"score\": <number 1-10>, \"correctness\": <number 1-10>, \"performance\": <number 1-10>, \"readability\": <number 1-10>, \"suggestions\": \"<specific improvement suggestions>\"}" \
        2>/dev/null || echo '{"score": 0, "suggestions": "critic failed"}')

    # Parse the score
    score=$(echo "$critic_result" | jq -r '.score // 0' 2>/dev/null || echo "0")
    suggestions=$(echo "$critic_result" | jq -r '.suggestions // "no suggestions"' 2>/dev/null || echo "no suggestions")

    info "Score: $score / 10"
    info "Details: $(echo "$critic_result" | jq -c '{correctness, performance, readability}' 2>/dev/null || echo "N/A")"

    # -------------------------------------------------------------------
    # Check threshold
    # -------------------------------------------------------------------
    if [ "$score" -ge "$PASS_THRESHOLD" ] 2>/dev/null; then
        echo
        success "Passed on iteration $attempt (score: $score)"
        echo
        header "Final Output"
        zag output "$gen_sid"
        echo
        header "Session Summary"
        zag summary "$gen_sid"
        exit 0
    fi

    warn "Score $score < $PASS_THRESHOLD — refining..."
    info "Feedback: $suggestions"
    feedback="$suggestions"
    echo
done

# Max iterations exhausted
echo
warn "Max iterations reached without passing threshold."
header "Best Output (iteration $attempt, score: $score)"
zag output "$gen_sid"
echo
header "Session Summary"
zag summary --tag "$SCRIPT_TAG"
