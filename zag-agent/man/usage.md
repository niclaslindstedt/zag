# zag-usage(1)

## Name

zag-usage — Manage in-flight usage-limit auto-resume timers.

## Synopsis

    zag usage [--json] [--root <dir>] list
    zag usage [--json] [--root <dir>] cancel <incident_id>

## Description

When a provider's CLI hits a usage / rate / weekly limit, zag detects
the limit and schedules an auto-resume timer. The timer fires at the
computed wake-up time and injects a resume message (default `Continue`)
into the live session. Until the timer fires, the resume is "pending."

`zag usage` lets you list and cancel pending resumes. See
[docs/usage-limits.md](../../docs/usage-limits.md) for the full
feature documentation, including the `[usage_limits]` config block in
`zag.toml`.

## Subcommands

`list`
:   Print every pending auto-resume timer with its session, provider,
    wake-up time, and incident id. Sorted by wake-up time. With
    `--json` (on the parent), output a JSON array of `PendingResume`
    records (see `zag-orch/src/usage_resume_store.rs` for the shape).

`cancel <incident_id>`
:   Write a tombstone for the given incident so the next rehydration
    pass (or any process re-reading the store) skips it. Does *not*
    abort an in-process timer in another running relay; if you need
    that, kill the relay process too. Errors if no pending record
    matches the id.

## State location

The store lives at `<state_dir>/scheduled_resumes.jsonl`, where
`<state_dir>` is the same project directory used by `zag session` —
`~/.zag/projects/<sanitized-root>/` for project-rooted sessions or
`~/.zag/` for global ones. Pass `--root` to operate on a specific
project's store.

## Examples

List pending resumes:

    $ zag usage list
    INCIDENT                              PROVIDER    WAKES AT (UTC)             ATTEMPT   SESSION
    c5f5d...   claude      2026-05-15 14:19:56 Z      1         abc-123

Cancel a stranded incident:

    $ zag usage cancel c5f5d...
    Cancelled pending resume c5f5d...

JSON output for scripting:

    $ zag usage --json list

## See also

`zag-listen`(1), `zag-events`(1) — surface usage-limit events from the
session log in real time. `docs/usage-limits.md` — the full feature
reference.
