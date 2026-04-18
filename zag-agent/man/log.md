# zag log

Append a custom structured event to a session log.

## Synopsis

    zag log <message> [options]
    zag log --session <ID> <message> [options]

## Description

Appends a `UserEvent` to a session's JSONL log file. This allows external tools, CI scripts, hooks, and orchestrators to annotate the session timeline with structured events.

The session is resolved from `--session` or the `ZAG_SESSION_ID` environment variable (automatically set inside `zag run`/`exec` sessions).

Events injected via `zag log` flow through `listen`, `subscribe`, `events`, `search`, and `summary` with no extra configuration.

## Arguments

    message    The log message text

## Flags

    --session <ID>     Target session ID (defaults to ZAG_SESSION_ID env var)
    --level <LEVEL>    Log level: info, warn, error, debug (default: info)
    --data <JSON>      Arbitrary JSON data to attach to the event
    -r, --root <PATH>  Root directory for session resolution

## Examples

    # Log from inside a session (auto-detects ZAG_SESSION_ID)
    zag log "deployment started"

    # Log with a specific level
    zag log --level warn "disk usage high"

    # Attach structured data
    zag log "tests passed" --data '{"count": 42, "duration_ms": 1200}'

    # Target a specific session from outside
    zag log --session $sid "build complete" --level info

## See Also

`zag listen`, `zag events`, `zag subscribe`, `zag search`
