# Events and Logging

zag normalizes output from all providers into a unified event format. This document describes the event types, their fields, and how to work with them programmatically.

## Unified output format

Every agent session produces an `AgentOutput` structure:

```json
{
  "agent": "claude",
  "session_id": "uuid",
  "events": [ ... ],
  "result": "final text output",
  "is_error": false,
  "total_cost_usd": 0.05,
  "usage": {
    "input_tokens": 1500,
    "output_tokens": 800
  }
}
```

Access this with `zag exec -o json` or `zag exec -o json-pretty`.

## Event types

Each event in the `events` array is one of these types:

### Init

Session initialization. Emitted once at the start.

```json
{
  "type": "init",
  "model": "claude-sonnet-4-5-20250929",
  "tools": ["Bash", "Read", "Write", ...],
  "cwd": "/path/to/project",
  "session_id": "uuid"
}
```

### UserMessage

A user message (included when replaying via `--replay-user-messages`).

```json
{
  "type": "user_message",
  "content": "write a hello world"
}
```

### AssistantMessage

An assistant response with content blocks and optional usage statistics.

```json
{
  "type": "assistant_message",
  "content": [
    { "type": "text", "text": "Here's a hello world program..." },
    { "type": "tool_use", "id": "tool_1", "name": "Write", "input": { ... } }
  ],
  "usage": {
    "input_tokens": 500,
    "output_tokens": 200,
    "cache_read_tokens": 100,
    "cache_creation_tokens": 50
  }
}
```

### ToolExecution

A tool call with its input and result.

```json
{
  "type": "tool_execution",
  "tool_name": "Bash",
  "input": { "command": "cargo build" },
  "result": {
    "success": true,
    "output": "Compiling zag v0.1.0...",
    "error": null
  }
}
```

### TurnComplete

End of a single assistant turn. Fires exactly once per turn, after the
final `assistant_message` / `tool_execution` of the turn and immediately
**before** the per-turn `result`. This is the authoritative turn-boundary
signal — prefer it over `result` in new code because it carries the
provider's `stop_reason` and a zero-based monotonic `turn_index`.

```json
{
  "type": "turn_complete",
  "stop_reason": "end_turn",
  "turn_index": 0,
  "usage": {
    "input_tokens": 500,
    "output_tokens": 200
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `stop_reason` | string? | Why the turn stopped. For Claude: `end_turn`, `tool_use`, `max_tokens`, `stop_sequence`. `null` when the provider didn't surface one. |
| `turn_index` | u32 | Zero-based monotonic turn index within the streaming session. |
| `usage` | Usage? | Usage reported for the final assistant message of this turn. |

**Ordering guarantees** (in bidirectional streaming mode):

1. `turn_complete` fires after the last `assistant_message` / `tool_execution`
   of the turn.
2. `turn_complete` fires immediately before the per-turn `result`.
3. `turn_complete` is emitted exactly once per turn; `turn_index` is a
   zero-based monotonic counter.

**Per-provider coverage**: only Claude currently exposes bidirectional
streaming, so `turn_complete` is only emitted by Claude today. Other
providers will gain it when they grow a bidirectional streaming path.

**Error / interrupt handling**: if a turn ends with a provider error
(`result.success == false`), `turn_complete` still fires before `result`;
its `stop_reason` is whatever the last assistant message reported (often
`null`). On hard subprocess kills or EOF-without-result, `turn_complete`
may not fire — this is best-effort.

### Result

Session-final or per-turn result summary.

```json
{
  "type": "result",
  "success": true,
  "message": "Task completed",
  "duration_ms": 15000,
  "num_turns": 3
}
```

In single-shot `exec` (and `-o stream-json`), a `result` event is emitted once
at the end of the session.

In bidirectional streaming via `AgentBuilder::exec_streaming()` (Claude only),
a `result` event is emitted at the **end of every agent turn** — not only at
final session end, and always immediately after a `turn_complete`. After
each `result`, the session remains open and will accept another
`send_user_message()` for the next turn. `result` continues to fire per-turn
for backward compatibility, but new consumers should key turn-boundary logic
off `turn_complete` — it is the authoritative signal and carries richer
metadata. Do **not** rely on replayed `user_message` events for turn
detection, as they only appear when `--replay-user-messages` is set.

### Error

An error during the session.

```json
{
  "type": "error",
  "message": "Provider returned an error",
  "details": "..."
}
```

### PermissionRequest

A permission prompt event (when not using `--auto-approve`).

```json
{
  "type": "permission_request",
  "tool_name": "Bash",
  "description": "Run: cargo test",
  "granted": true
}
```

## Usage statistics

The `usage` field tracks token consumption:

| Field | Type | Description |
|-------|------|-------------|
| `input_tokens` | u64 | Tokens sent to the model |
| `output_tokens` | u64 | Tokens generated by the model |
| `cache_read_tokens` | u64? | Tokens read from cache (Claude only) |
| `cache_creation_tokens` | u64? | Tokens written to cache (Claude only) |
| `web_search_requests` | u32? | Web search tool calls (Claude only) |
| `web_fetch_requests` | u32? | Web fetch tool calls (Claude only) |

Use `--show-usage` to include usage statistics in the output.

## Output formats

Use `zag exec -o <format>` to control the output format:

| Format | Description |
|--------|-------------|
| *(default)* | Streamed text with formatting |
| `text` | Raw agent output, no parsing |
| `json` | Compact unified JSON (`AgentOutput`) |
| `json-pretty` | Pretty-printed unified JSON |
| `stream-json` | NDJSON event stream (unified format) |
| `native-json` | Provider's raw JSON format (Claude only) |

## NDJSON streaming

With `-o stream-json`, events are emitted as newline-delimited JSON:

```bash
zag exec -o stream-json "analyze the code" | while read -r line; do
  type=$(echo "$line" | jq -r '.type')
  echo "Event: $type"
done
```

Each line is a complete JSON object representing one event.

## Session logs

Session data is stored under `~/.zag/projects/<sanitized-path>/sessions/`. Each session includes:

- Session metadata (ID, name, tags, provider, model, timestamps)
- The full event log
- Provider-native session ID (for resume)

### Querying events

```bash
# All events from a session
zag events <session-id>

# Filter by event type
zag events <session-id> --type tool_call --json

# Last N events
zag events <session-id> --last 10

# Events after a sequence number
zag events <session-id> --after-seq 42
```

### Real-time log tailing

```bash
# Tail a session's events
zag listen <session-id>

# With rich formatting and timestamps
zag listen --latest --rich-text --timestamps

# Filter to specific event types
zag listen <session-id> --filter session_ended --filter tool_call
```

### Multiplexed event stream

Subscribe to events from all active sessions:

```bash
# All sessions
zag subscribe --json

# Filtered by tag
zag subscribe --tag batch --json | jq 'select(.type == "session_ended")'
```

## Filesystem event markers

For external orchestrators that prefer filesystem notifications over polling, zag writes lifecycle markers to `~/.zag/events/`:

- `<session-id>.started` -- Created when a session begins
- `<session-id>.ended` -- Created when a session completes

These can be watched with `inotifywait` or similar tools.

## Claude-specific JSON format

When using `-o native-json` with Claude, the output is Claude's raw JSON array of events. This includes Claude-specific fields like `subtype`, `mcp_servers`, `permissionMode`, and detailed content block structures. See `src/claude/README.md` in the source tree for the complete format reference.

## Related

- `zag man events` -- Events command reference
- `zag man listen` -- Listen command reference
- `zag man subscribe` -- Subscribe command reference
- `zag man output` -- Output command reference
