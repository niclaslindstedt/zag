# zag status

Machine-readable session health check.

## Synopsis

    zag status <session-id> [--json] [--root <path>]

## Description

Reports the current status of a session by combining information from the session store, process store, and session log. Designed for orchestration scripts that need to poll session health.

## Arguments

    session-id    Session ID (full or prefix match)

## Flags

    --json               Output as JSON
    -r, --root <PATH>    Root directory for session resolution

## Status Values

    running      Process alive, log activity within last 30 seconds
    idle         Process alive, no log activity for 30+ seconds
    completed    Session ended successfully (SessionEnded with success=true)
    failed       Session ended with failure (SessionEnded with success=false)
    dead         Process died without a clean SessionEnded event
    unknown      Session exists in store but no process or log found

## JSON Output

    {"session_id":"...","status":"running","provider":"claude","model":"sonnet","name":"my-agent","pid":12345}

Fields `name`, `pid`, and `error` are omitted when null.

## Exit Codes

    0    Status retrieved successfully
    1    Session not found or error

## Examples

    zag status $sid                     Check session status (text)
    zag status $sid --json              JSON output
    zag status $sid --json | jq .status Extract just the status string

## See Also

    zag man wait      Block until sessions complete
    zag man ps        Process management
    zag man listen    Real-time session log tailing
