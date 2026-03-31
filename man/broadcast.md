# zag broadcast

Send a message to all sessions matching a tag.

## Synopsis

    zag broadcast --tag <tag> "message"
    zag broadcast --tag <tag> --global "message"
    echo "message" | zag broadcast --tag <tag>

## Description

`zag broadcast` sends a user message to all sessions that match a given tag. This is the multi-session counterpart to `zag input` — while `input` targets exactly one session, `broadcast` delivers the same message to every session with the specified tag.

The command resolves all matching sessions, sends the message to each one sequentially, and reports a summary of sent/failed counts.

If called from within a zag session (detected via `ZAG_SESSION_ID`), the message is automatically wrapped with sender metadata and reply instructions, just like `zag input`. Use `--raw` to skip wrapping.

## Options

### `<message>`

The message text to send. If omitted, the message is read from stdin.

### `--tag <TAG>`

Target all sessions with this tag. Required. Tags are matched case-insensitively.

### `--global`

Search across all projects instead of only the current project.

### `-o, --output <FORMAT>`

Output format for the broadcast result:

- **text** (default): Human-readable summary printed to stderr (e.g., `> Sent to 3 sessions (0 failed)`)
- **json**: Compact JSON with per-session results and summary
- **json-pretty**: Pretty-printed JSON

JSON output structure:

```json
{
  "results": [
    {"session_id": "abc-123", "status": "sent"},
    {"session_id": "def-456", "status": "failed", "error": "session not found"}
  ],
  "summary": {"sent": 1, "failed": 1, "total": 2}
}
```

### `-r, --root <PATH>`

Root directory for session resolution.

### `--raw`

Send the message without agent-to-agent envelope wrapping. By default, when `zag broadcast` is called from within a zag session, the message is wrapped with sender metadata. Use `--raw` to send the message verbatim.

## Examples

    # Broadcast to all sessions tagged "backend"
    zag broadcast --tag backend "report your status"

    # Cross-project broadcast
    zag broadcast --tag backend --global "deploy completed"

    # Pipe from stdin
    echo "standup time" | zag broadcast --tag team

    # JSON output with per-session results
    zag broadcast --tag backend "status" -o json

    # Pretty-printed JSON output
    zag broadcast --tag backend "status" -o json-pretty

    # Send without agent envelope wrapping
    zag broadcast --tag backend --raw "plain message"

## Provider Support

Broadcast uses each session's provider-native resume mechanism. All providers that support `run_resume_with_prompt` are supported (Claude, Codex, Gemini, Copilot). Streaming is not supported for broadcast.

## See Also

    zag man input     Send to a single session
    zag man listen    Tail session output in real-time
    zag man session   List/inspect sessions
    zag man ps        List and manage agent processes
