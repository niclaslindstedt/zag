# listen

Tail a session's log file and output parsed events in real-time.

## Synopsis

    zag listen <session-id>
    zag listen --latest
    zag listen --active

## Description

`zag listen` tails a session's JSONL log file and outputs parsed events as they are written. This enables monitoring an active agent session from a separate terminal.

Session logs are stored under `~/.zag/projects/<sanitized-path>/logs/sessions/<session-id>.jsonl`.

## Options

### `<session-id>`

The wrapper session ID to listen to. Prefix matching is supported if the prefix is unambiguous.

### `--latest`

Listen to the most recently created session (by `started_at` in the index).

### `--active`

Listen to the most recently written-to session log file (by file modification time).

### `--json`

Output each event as a raw JSON line (NDJSON format).

### `--text`

Output events as human-readable plain text (default).

### `--rich-text`

Output events as rich text with ANSI formatting (colors, bold, dim, italic).

## Configuration

The default output format can be set in `zag.toml`:

```toml
[listen]
format = "text"       # "text", "json", or "rich-text"
```

Config key: `listen.format`

## Event Formatting

In text mode, events are formatted as:

- `[session]` — Session start/end
- `[user]` — User messages
- `[assistant]` — Assistant messages
- `[thinking]` — Reasoning/thinking blocks
- `[tool]` — Tool calls
- `[result]` — Tool results
- `[permission]` — Permission grants/denials
- `[status]` — Provider status messages
- `[stderr]` — Stderr output
- `[warning]` — Parse warnings

## Examples

    # Listen to a specific session
    zag listen abc123-def456

    # Listen to the latest session
    zag listen --latest

    # Listen to the most active session
    zag listen --active

    # JSON output for piping
    zag listen --latest --json

    # Rich text output (colors, bold, dim, italic)
    zag listen --active --rich-text

## Exit Behavior

The command exits when a `SessionEnded` event is received or when interrupted with Ctrl+C.
