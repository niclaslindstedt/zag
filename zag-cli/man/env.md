# zag env

Export session environment variables.

## Synopsis

    zag env [session-id] [--shell] [--root <path>]

## Description

Outputs the `ZAG_*` environment variables for a session. Useful for orchestrators that need to construct the correct environment for nested agent invocations, or for debugging session identity.

Without a session ID, uses the current session (from `ZAG_SESSION_ID` env var) or the latest session.

## Arguments

    session-id    Session ID to look up (optional, defaults to latest)

## Flags

    --shell              Output as shell export statements (for eval)
    -r, --root <PATH>    Root directory for session resolution

## Output

Default output (key=value):

    ZAG_SESSION_ID=a1b2c3d4-e5f6-7890-abcd-ef1234567890
    ZAG_PROVIDER=claude
    ZAG_MODEL=sonnet
    ZAG_ROOT=/home/user/project

With `--shell`:

    export ZAG_SESSION_ID='a1b2c3d4-e5f6-7890-abcd-ef1234567890';
    export ZAG_PROVIDER='claude';
    export ZAG_MODEL='sonnet';
    export ZAG_ROOT='/home/user/project';

## Environment Variables

The following variables are output when available:

    ZAG_SESSION_ID      Session UUID
    ZAG_SESSION_NAME    Session name (if set)
    ZAG_PROCESS_ID      Process UUID
    ZAG_PROVIDER        Provider name
    ZAG_MODEL           Model name
    ZAG_ROOT            Project root path

## Examples

    zag env                                Show env for current/latest session
    zag env $sid                           Show env for a specific session
    eval $(zag env --shell $sid)           Set env vars in current shell

## See Also

    zag man whoami    Session identity introspection
    zag man spawn     Background session launch
