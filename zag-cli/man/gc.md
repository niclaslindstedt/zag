# zag gc

Clean up old session data, logs, and process entries.

## Synopsis

    zag gc [options]
    zag gc --force [--older-than <DURATION>] [--keep-logs]

## Description

Removes stale data that accumulates over time: dead process entries, old lifecycle markers, old spawn logs, and ended session log files. By default runs in dry-run mode showing what would be cleaned. Use `--force` to actually delete.

Running and idle sessions are never touched.

## Flags

    --force              Actually delete (default is dry-run)
    --older-than <DUR>   Only clean data older than this threshold (default: 7d)
    --keep-logs          Keep session log files (only clean process/marker entries)
    --json               Output as JSON
    -r, --root <PATH>    Root directory for session resolution

## Duration Format

    7d     7 days
    30d    30 days
    24h    24 hours

## What Gets Cleaned

1. **Process entries** — Dead/exited entries from `processes.json`
2. **Lifecycle markers** — Old `.started`/`.ended` files from `~/.zag/events/`
3. **Spawn logs** — Old spawn log files from `~/.zag/logs/spawn/`
4. **Session logs** — Ended session JSONL files (unless `--keep-logs`)

## Examples

    # Dry run — show what would be cleaned
    zag gc

    # Actually delete
    zag gc --force

    # Custom threshold
    zag gc --force --older-than 30d

    # Only clean process/marker entries, keep logs
    zag gc --force --keep-logs

    # Machine-readable output
    zag gc --json

## See Also

`zag ps`, `zag status`, `zag session`
