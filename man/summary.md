# zag summary

Show a log-based summary of one or more sessions.

## Synopsis

    zag summary <session-ids>... [options]
    zag summary --tag <TAG> [options]

## Description

Reads session JSONL logs and produces a structured summary: tool usage counts, files modified, duration, turn count, and result preview. No LLM call is made — this is purely log-based introspection.

## Arguments

    session-ids    One or more session IDs to summarize

## Flags

    --tag <TAG>        Summarize all sessions with this tag
    --stats            Show detailed statistics
    --json             Output as JSON
    -r, --root <PATH>  Root directory for session resolution

## Output (Text Mode)

    Session: abc123 (claude/opus) — completed in 2m 34s
    Files modified: src/auth.rs, src/main.rs
    Tools used: Bash (5), Edit (3), Read (8), Grep (2)
    Turns: 12, Events: 47
    Result: Implemented OAuth2 flow with token refresh

## Examples

    # Summarize a single session
    zag summary $sid

    # Summarize with JSON output
    zag summary $sid --json

    # Summarize all sessions with a tag
    zag summary --tag batch --json

    # Summarize multiple sessions
    zag summary $sid1 $sid2 $sid3

## See Also

    zag man events     Raw structured event access
    zag man collect    Gather final results
    zag man status     Check session health
