# zag collect

Gather results from multiple sessions.

## Synopsis

    zag collect [session-ids...] [--tag <tag>] [--json] [--root <path>]

## Description

Collects the final results from one or more completed sessions. For each session, extracts the last assistant message and session status from the JSONL log. Useful after `zag wait` to aggregate outputs from parallel agents.

## Arguments

    session-ids    One or more session IDs to collect from

## Flags

    --tag <TAG>          Collect from all sessions with this tag
    --json               Output as JSON array
    -r, --root <PATH>    Root directory for session resolution

## Output

Text output shows each session's result with a header:

    === session-name (a1b2c3d4) [completed] ===
    The analysis shows that...

JSON output produces an array of result objects:

    [
      {
        "session_id": "a1b2c3d4-...",
        "name": "analyzer",
        "provider": "claude",
        "model": "sonnet",
        "status": "completed",
        "result_text": "The analysis shows that..."
      }
    ]

Fields `name`, `error`, and `result_text` are omitted when null.

## Examples

    zag collect $sid1 $sid2 $sid3           Collect from three sessions
    zag collect --tag batch --json          Collect all "batch" sessions as JSON
    zag collect --tag batch | less          Browse results

    # Full workflow
    sid1=$(zag spawn --tag batch "analyze auth")
    sid2=$(zag spawn --tag batch "review tests")
    zag wait --tag batch --timeout 5m
    zag collect --tag batch --json > results.json

## See Also

    zag man spawn     Background session launch
    zag man wait      Block until sessions complete
    zag man status    Session health check
