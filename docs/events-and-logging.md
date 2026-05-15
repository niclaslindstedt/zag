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
  "exit_code": 0,
  "error_message": null,
  "total_cost_usd": 0.05,
  "usage": {
    "input_tokens": 1500,
    "output_tokens": 800
  },
  "model": "claude-sonnet-4-5-20250929",
  "provider": "claude",
  "log_path": "/home/you/.zag/projects/.../sessions/<id>.jsonl"
}
```

Access this with `zag exec -o json` or `zag exec -o json-pretty`.

| Field | Type | Description |
|-------|------|-------------|
| `agent` | string | The agent name (same as `provider`, retained for backward compatibility). |
| `session_id` | string | Unique session identifier (UUID). |
| `events` | array | The full unified event stream (see below). |
| `result` | string? | Final assistant text. Empty strings fall back to `structured_output` (Claude `--json-schema`) or the last assistant message. |
| `is_error` | bool | `true` if the session ended in a provider or subprocess error. |
| `exit_code` | i32? | Exit code from the underlying provider process, when available. Omitted from JSON when `null`. |
| `error_message` | string? | Human-readable error message from the provider. Omitted from JSON when `null`. |
| `total_cost_usd` | f64? | Session cost in USD (provider-specific, Claude surfaces this natively). |
| `usage` | Usage? | Aggregated token usage for the session. |
| `model` | string? | Concrete model reported by the provider (e.g. `claude-sonnet-4-5-20250929`). |
| `provider` | string? | Effective provider after any tier-list downgrade (see `providers.md#provider-downgrade`). |
| `log_path` | string? | Absolute path to the JSONL session log on disk, populated when the builder ran with session logging enabled (`AgentBuilder::enable_session_log(true)` or via `on_log_event` / `stream_events_to_stderr`). Omitted from JSON when `null`. |

`exec` exits with a non-zero status when `is_error == true`; pass `--exit-on-failure` to force the CLI to propagate provider failure as an exit code 1 even when the session technically completed.

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
  "tool_id": "tool_abc123",
  "input": { "command": "cargo build" },
  "result": {
    "success": true,
    "output": "Compiling zag v0.1.0...",
    "error": null
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | Name of the tool that was called (e.g. `Bash`, `Read`, `Write`). In streaming mode, Claude resolves the name from the preceding `assistant_message`'s `tool_use` block. |
| `tool_id` | string | Unique identifier linking this execution back to the `tool_use` content block that triggered it. |
| `input` | object | The JSON input passed to the tool. |
| `result` | ToolResult | Execution outcome (see below). |
| `parent_tool_use_id` | string? | Present when this execution belongs to a sub-agent; carries the `tool_use_id` of the parent `Agent` tool call that spawned it. Omitted from JSON when `null`. |

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

### UsageLimitHit / UsageLimitResumed / UsageLimitResumeFailed

zag detects when a provider hits an upstream usage / rate / weekly limit and
records the full lifecycle in the session log so you can see (in `zag listen`)
exactly when the run was paused, when it resumed, and what was injected.
Each hit is joined to its later `resumed`/`failed` event by `incident_id`.

```json
{
  "type": "usage_limit_hit",
  "provider": "claude",
  "scope": "weekly",
  "reset_at": "2026-05-20T14:32:00Z",
  "scheduled_resume_at": "2026-05-20T14:32:30Z",
  "fallback_used": false,
  "incident_id": "a4b2…",
  "raw": "Claude AI weekly usage limit reached|1747754320"
}
```

```json
{ "type": "usage_limit_resumed", "incident_id": "a4b2…",
  "resume_message": "Continue", "attempt": 1 }
```

```json
{ "type": "usage_limit_resume_failed", "incident_id": "a4b2…",
  "error": "No FIFO for session …", "attempt": 1 }
```

See `docs/usage-limits.md` for the full feature reference (detection
patterns, configuration, per-provider behaviour, manual repro).

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

> **Removed in v0.10.0**: The standalone `--json-stream` flag was removed because
> it duplicated `-o stream-json`. Use `zag exec -o stream-json` to get NDJSON
> events — the behavior is identical.

## NDJSON streaming

With `-o stream-json`, events are emitted as newline-delimited JSON:

```bash
zag exec -o stream-json --prompt "analyze the code" | while read -r line; do
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

### Live event streaming from the builder (Rust)

Rust callers using `AgentBuilder` can subscribe to per-step session log
events without shelling out to `zag listen`. Opting in starts a
`SessionLogCoordinator` for the terminal method's lifetime and invokes a
callback (or tails events to stderr) as each event is written:

```rust
use zag::{AgentBuilder, listen::ListenFormat};

// Register a typed callback
let output = AgentBuilder::new()
    .provider("claude")
    .on_log_event(|event| {
        eprintln!("event: {:?}", event.kind);
    })
    .exec("analyze the code")
    .await?;

println!("log on disk: {:?}", output.log_path);

// Or tail a pre-formatted event stream to stderr
let output = AgentBuilder::new()
    .provider("claude")
    .stream_events_to_stderr(ListenFormat::RichText)
    .stream_show_thinking(true)
    .exec("analyze the code")
    .await?;
```

Both setters implicitly enable session logging. Use
`.enable_session_log(true)` or `.session_log(SessionLogMode::Auto)` when
you want the JSONL log on disk but no live callback.

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
