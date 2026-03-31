# zag wait

Block until one or more sessions complete.

## Synopsis

    zag wait [session-ids...] [--tag <tag>] [--latest] [--timeout <duration>] [--any] [--json]

## Description

Blocks until all specified sessions have completed (emitted a `SessionEnded` event in their log). Useful for orchestration scripts that spawn background agents and need to synchronize before collecting results.

If `--any` is set, exits as soon as the first session completes instead of waiting for all.

## Arguments

    session-ids    One or more session IDs to wait for

## Flags

    --tag <TAG>          Wait for all sessions with this tag
    --latest             Wait for the latest session
    --timeout <DURATION> Timeout duration (e.g., 30s, 5m, 1h, 1h30m)
    --any                Exit on first completed session
    --json               Output results as NDJSON (one per session)
    -r, --root <PATH>    Root directory for session resolution

## Exit Codes

    0     All sessions completed successfully
    1     One or more sessions failed
    124   Timeout reached

## Examples

    zag wait $sid1 $sid2 $sid3              Wait for three sessions
    zag wait --tag batch                    Wait for all sessions tagged "batch"
    zag wait --tag batch --timeout 10m      With a 10-minute timeout
    zag wait --tag batch --any              Exit on first completion
    zag wait --latest --json                Wait for latest session, JSON output
    zag wait $sid1 $sid2 --timeout 5m --json  JSON output with timeout

## See Also

    zag man spawn     Background session launch
    zag man status    Session health check
    zag man collect   Gather multi-session results
