# zag pipe

Chain results from completed sessions into a new agent session.

## Synopsis

    zag pipe [options] <session_ids>... -- <prompt>
    zag pipe --tag <TAG> -- <prompt>

## Description

Collects the final results from one or more completed sessions and feeds them as context into a new agent session along with a user-provided prompt. This is the primary primitive for building DAG-like workflows where one session's output feeds into the next.

Results are wrapped in `<session-result>` XML tags with session ID prefixes so the receiving agent can distinguish between sources.

## Arguments

    session_ids    One or more session IDs to collect results from
    prompt         The prompt to send with the collected context (after --)

## Flags

    --tag <TAG>              Collect results from all sessions with this tag
    -p, --provider <NAME>    Provider for the new session
    -m, --model <NAME>       Model for the new session
    -o, --output <FORMAT>    Output format (text, json, json-pretty)
    --json                   Request JSON output from the agent
    -a, --auto-approve       Skip permission prompts
    -s, --system-prompt      Custom system prompt
    --add-dir <PATH>         Additional directories (repeatable)
    --size <SIZE>            Ollama model size
    --max-turns <N>          Maximum agentic turns
    -r, --root <PATH>        Root directory
    -q, --quiet              Suppress logging

## Examples

    # Pipe one session's result into a new one
    sid=$(zag spawn "analyze auth module")
    zag wait $sid
    zag pipe $sid -- "summarize the analysis"

    # Pipe multiple sessions
    zag pipe $sid1 $sid2 $sid3 -- "synthesize these analyses"

    # Pipe by tag
    zag pipe --tag batch -- "create a unified report"

    # With explicit provider/model
    zag pipe --tag batch -p claude -m opus -- "synthesize findings"

## See Also

    zag man collect    Gather raw results without re-processing
    zag man spawn      Launch background sessions
    zag man exec       Non-interactive single prompt
