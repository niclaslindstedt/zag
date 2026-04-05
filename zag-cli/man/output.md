# zag output

Extract the final result text from a session.

## Synopsis

    zag output [session-id] [options]
    zag output --latest
    zag output --name <NAME>
    zag output --tag <TAG>

## Description

Prints the last assistant message text from a session, making it easy to use session results in shell pipelines without parsing JSON.

If no targeting flag is provided, defaults to the latest session.

## Arguments

    session-id    Session ID to extract output from (optional)

## Flags

    --latest           Use the most recently created session
    --name <NAME>      Find session by name
    --tag <TAG>        Extract output from all sessions with this tag
    --json             Output as JSON with session ID metadata
    -r, --root <PATH>  Root directory for session resolution

## Examples

    # Get the result of a specific session
    zag output $sid

    # Get result of the latest session
    zag output --latest

    # Use in shell pipelines
    result=$(zag output $sid)

    # Get output from all sessions with a tag
    zag output --tag batch

    # JSON output with metadata
    zag output $sid --json

    # Find session by name
    zag output --name backend-agent

## See Also

`zag collect`, `zag pipe`, `zag events`
