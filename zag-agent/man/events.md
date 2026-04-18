# zag events

Query structured events from session logs.

## Synopsis

    zag events <session-id> [options]

## Description

Returns structured `AgentLogEvent` objects from a session's JSONL log with filtering by event type, sequence number range, and count. This is the low-level read API for programmatic access to session logs.

Unlike `search` (which returns text snippets), `events` returns full structured event objects suitable for machine consumption.

## Arguments

    session-id    The session to query events from

## Flags

    --type <EVENT_TYPE>    Filter by event type (session_started, user_message,
                           assistant_message, reasoning, tool_call, tool_result,
                           permission, provider_status, session_ended, etc.)
    --last <N>             Show only the last N matching events
    --after-seq <SEQ>      Show events after this sequence number (for polling)
    --before-seq <SEQ>     Show events before this sequence number
    --count                Output only the count of matching events
    --json                 Output as NDJSON (one JSON object per line)
    -r, --root <PATH>     Root directory for session log resolution

## Examples

    # Get all events from a session
    zag events $sid

    # Filter by event type
    zag events $sid --type tool_call

    # Get last 5 events
    zag events $sid --last 5

    # Get events after a sequence number (for pagination/polling)
    zag events $sid --after-seq 42

    # JSON output
    zag events $sid --type assistant_message --json

    # Count events
    zag events $sid --type tool_call --count

    # Poll for new events since last check
    last_seq=0
    while true; do
        zag events $sid --after-seq $last_seq --json | while read event; do
            last_seq=$(echo $event | jq .seq)
            echo "New event: $event"
        done
        sleep 1
    done

## See Also

    zag man listen     Real-time event tailing
    zag man search     Full-text search across sessions
    zag man summary    High-level session summary
