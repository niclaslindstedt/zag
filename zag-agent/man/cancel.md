# zag cancel

Gracefully cancel one or more running sessions.

## Synopsis

    zag cancel <session-ids>... [options]
    zag cancel --tag <TAG> [options]

## Description

Sends a cancellation signal to running sessions and writes a clean `SessionEnded` event to the session log with a `cancelled` reason. This ensures that `status`, `collect`, and `wait` all see the session as properly terminated rather than dead.

Unlike `zag ps stop` (which only sends SIGHUP) or `zag ps kill` (which sends SIGTERM), `cancel` also writes to the session log, making the cancellation visible to all session-aware commands.

## Arguments

    session-ids    One or more session IDs to cancel

## Flags

    --tag <TAG>        Cancel all sessions with this tag
    --reason <TEXT>    Reason for cancellation (recorded in the log)
    --json             Output as JSON
    -r, --root <PATH>  Root directory for session resolution

## Examples

    # Cancel a session gracefully
    zag cancel $sid

    # Cancel all sessions with a tag
    zag cancel --tag batch

    # Cancel with a reason
    zag cancel $sid --reason "timeout in orchestrator"

    # JSON output for scripting
    zag cancel --tag batch --json

## See Also

    zag man ps        Process management (stop/kill)
    zag man status    Check session status
    zag man wait      Wait for sessions to complete
