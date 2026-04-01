# zag retry

Re-run a failed session with the same configuration.

## Synopsis

    zag retry <session-ids>... [options]
    zag retry --tag <TAG> --failed [options]

## Description

Re-spawns sessions using the original provider, model, prompt, and metadata from the session store and session log. The new session records a `retried_from` reference to the original session for traceability.

## Arguments

    session-ids    One or more session IDs to retry

## Flags

    --tag <TAG>        Retry all sessions with this tag
    --failed           Only retry sessions with failed or dead status
    --model <MODEL>    Override the model for the retry (upgrades/downgrades)
    --json             Output as JSON
    -r, --root <PATH>  Root directory for session resolution

## Examples

    # Retry a specific session
    zag retry $sid

    # Retry all failed sessions with a tag
    zag retry --tag batch --failed

    # Retry with a more powerful model
    zag retry $sid --model large

    # Retry multiple sessions
    zag retry $sid1 $sid2 $sid3

    # JSON output
    zag retry --tag batch --failed --json

## See Also

`zag spawn`, `zag status`, `zag wait`
