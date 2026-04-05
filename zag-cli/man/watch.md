# zag watch

Watch session logs and execute a command on matching events.

## Synopsis

    zag watch <session-id> --on <EVENT_TYPE> [options] -- <command>
    zag watch --tag <TAG> --on <EVENT_TYPE> -- <command>
    zag watch --latest --on <EVENT_TYPE> -- <command>

## Description

Like `listen` but with the ability to execute a shell command when specific events match. Think of it as `listen` + `xargs` for building event-driven automation.

The command supports template variables that are expanded with event data before execution.

## Arguments

    session-id    Session ID to watch (optional if --tag or --latest used)

## Flags

    --on <EVENT_TYPE>    Event type to watch for (required): session_started,
                         session_ended, user_message, assistant_message,
                         tool_call, tool_result, etc.
    --tag <TAG>          Watch sessions with this tag
    --latest             Watch the latest session
    --filter <EXPR>      Filter expression (key=value pairs, comma-separated)
    --once               Exit after the first matching event
    --json               Output matching events as JSON
    -r, --root <PATH>    Root directory for session resolution
    command              Command to execute (after --)

## Template Variables

    {session_id}     Session ID of the event source
    {provider}       Provider name
    {event_type}     Event type name
    {seq}            Event sequence number
    {ts}             Event timestamp

## Filter Expressions

    success=true       Match successful session_ended events
    success=false      Match failed session_ended events
    tool_name=bash     Match tool_call events for a specific tool

## Examples

    # Run a command when a session completes
    zag watch $sid --on session_ended -- echo "done: {session_id}"

    # Watch for failures
    zag watch --tag batch --on session_ended --filter 'success=false' -- \
        echo "Agent {session_id} failed"

    # Exit after first completion
    zag watch $sid --on session_ended --once

    # Chain: when analysis completes, start the fix
    zag watch $sid --on session_ended --once -- \
        zag pipe {session_id} -- "fix the issues found"

## See Also

    zag man listen       Real-time event tailing
    zag man subscribe    Multiplexed event stream
    zag man events       Structured event query
