# zag subscribe

Subscribe to a multiplexed event stream from all active sessions.

## Synopsis

    zag subscribe [options]

## Description

Watches all active session JSONL log files and outputs a single merged NDJSON event stream. This is the most critical read-side primitive for building orchestration on top of zag — instead of running N `listen` processes for N sessions, a single `subscribe` gives you all events.

Events are output as they arrive from any active session, with full session context in each event.

## Flags

    --tag <TAG>              Filter to sessions with this tag
    --filter <EVENT_TYPE>    Filter by event type
    --global                 Subscribe across all projects
    --json                   Output as NDJSON (default behavior)
    -r, --root <PATH>        Root directory for session resolution

## Examples

    # Subscribe to all events from all sessions
    zag subscribe

    # Filter by tag
    zag subscribe --tag batch

    # Filter by event type
    zag subscribe --filter session_ended

    # Cross-project subscription
    zag subscribe --global

    # Pipe to jq for processing
    zag subscribe --tag batch --json | jq 'select(.type == "session_ended")'

    # Monitor completions across all tagged sessions
    zag subscribe --tag batch --filter session_ended | while read event; do
        echo "Session completed: $(echo $event | jq -r .wrapper_session_id)"
    done

## See Also

    zag man listen    Tail a single session
    zag man watch     Event-driven command execution
    zag man events    Query historical events
