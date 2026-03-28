# zag listen

Tail a session's log file and output parsed events in real-time.

## Synopsis

    zag listen <session-id>
    zag listen --latest
    zag listen --active
    zag listen --ps <pid>

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

### `--ps <PID>`

Listen to the session belonging to a process, specified by OS PID (integer) or zag process UUID (from `zag ps list`). If multiple entries share the same PID (OS PIDs are recycled), the most recently started process is used. Mutually exclusive with `<session-id>`, `--latest`, and `--active`.

### `--json`

Output each event as a raw JSON line (NDJSON format).

### `--text`

Output events as human-readable plain text (default).

### `--rich-text`

Output events as rich text with ANSI formatting (colors, bold, dim, italic). Assistant messages are rendered as styled markdown.

### `--show-thinking`

Show thinking/reasoning content. By default, reasoning blocks are hidden.

### `-r, --root <PATH>`

Root directory for session log resolution.

## Configuration

The default output format can be set in `zag.toml`:

```toml
[listen]
format = "text"       # "text", "json", or "rich-text"
```

Config key: `listen.format`

## Event Formatting

In text mode, events use Unicode icons:

- `●` — Session start/end
- `❯` — User messages
- `⏺` — Assistant messages
- `…` — Reasoning/thinking blocks (hidden unless `--show-thinking`)
- `⚡` — Tool calls (with summarized input)
- `✓` / `✗` — Tool results (success/failure)
- `🔓` / `🔒` — Permission grants/denials
- `>` — Provider status messages
- `!` — Stderr output
- `?` — Parse warnings

In rich-text mode, the same icons are used with ANSI colors and markdown rendering for assistant messages.

## Examples

    # Listen to a specific session
    zag listen abc123-def456

    # Listen to the latest session
    zag listen --latest

    # Listen to the most active session
    zag listen --active

    # JSON output for piping
    zag listen --latest --json

    # Rich text output (colors, markdown rendering)
    zag listen --active --rich-text

    # Show reasoning/thinking content
    zag listen --latest --show-thinking

    # Listen to a session by OS PID
    zag listen --ps 12345

    # Listen to a session by zag process UUID
    zag listen --ps a1b2c3d4-...

## Exit Behavior

The command exits when a `SessionEnded` event is received or when interrupted with Ctrl+C.

## See Also

    zag man session   List and inspect sessions
    zag man run       Start an interactive session
