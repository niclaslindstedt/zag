# Sessions

Every agent invocation in zag creates a session. Sessions track the full lifecycle of an agent interaction -- its provider, model, events, output, and metadata.

## Session lifecycle

Sessions are created automatically when you run `zag exec`, `zag run`, or `zag spawn`. Each session gets a unique UUID and is stored in the project's session history:

```
~/.zag/projects/<sanitized-path>/sessions.json
```

The project path is derived from your git repo root. For example, `/home/user/myproject` becomes `home-user-myproject`.

## Naming and tagging

Give sessions human-readable names and tags for easy discovery:

```bash
# Name a session
zag exec --name auth-review "review the auth module"

# Add tags (repeatable)
zag exec --tag backend --tag review "review the API"

# Add a description
zag exec --name deploy --description "pre-release deployment check" "verify all tests pass"
```

Names, tags, and descriptions are stored in the session record and can be used for filtering.

## Listing sessions

```bash
# List all sessions
zag session list

# Filter by provider
zag session list --provider claude

# Filter by name (prefix match)
zag session list --name auth

# Filter by tag
zag session list --tag review

# Filter by parent session
zag session list --parent "$sid"
```

## Inspecting a session

```bash
# Show full session details
zag session show <session-id>

# JSON output
zag session show <session-id> --json
```

Session records include: session ID, provider, model, timestamps, name, description, tags, dependencies, worktree/sandbox paths, interactive status, and log completeness.

## Updating sessions

Update metadata on existing sessions:

```bash
zag session update <session-id> --name new-name
zag session update <session-id> --tag new-tag
zag session update <session-id> --description "updated description"
```

## Resume and continue

Resume a previous session to continue where you left off:

```bash
# Resume a specific interactive session
zag run --resume <session-id>

# Continue the most recent interactive session
zag run --continue

# One-shot resume: pick up an existing session, send one more prompt, and exit
zag exec --resume <session-id> "also add tests"
zag exec --continue "summarize what we just did"
```

Both `run` and `exec` accept `--resume <SESSION_ID>` and `--continue`. For
`exec`, zag replays the prior session into the provider, sends the new
prompt, prints the result, and exits. This is the preferred way to thread
state across CI jobs, scripts, or multi-step agent workflows without
opening an interactive shell.

Resume support varies by provider:

| Provider | Resume mechanism |
|----------|-----------------|
| Claude | Native session state |
| Codex | Thread ID tracking |
| Gemini | `--resume` with session ID or "latest" |
| Copilot | `--resume` and `--continue` flags |
| Ollama | Not supported |

## Interactive sessions

Interactive sessions stay alive for ongoing conversation. They use FIFO named pipes under `~/.zag/fifos/`:

```bash
# Start an interactive session (requires Claude provider)
sid=$(zag spawn --interactive --name worker -p claude)

# Send messages
zag input --name worker "analyze this module"
zag input --name worker "now suggest improvements"

# Tail output
zag listen --name worker
```

See [Orchestration](orchestration.md) for more on interactive sessions.

## Streaming input: mid-turn injection semantics

When a provider supports streaming input (see `features.streaming_input` in
`zag discover`), language bindings expose a `StreamingSession` whose
`send_user_message` / `sendUserMessage` method writes a user message to the
running agent over stdin. The question of *what happens when you call that
method while the agent is still producing a response on the current turn* is
provider-specific, so the capability descriptor now carries an explicit
`semantics` field.

### Values

| `semantics` | Meaning |
|-------------|---------|
| `"queue"` | The message is buffered and delivered at the **next turn boundary**. The current turn runs to completion; the new message becomes the next user turn. |
| `"interrupt"` | The message **cancels** the current turn and starts a new one with the new input. |
| `"between-turns-only"` | Mid-turn sends are an error or no-op; callers must wait for the current turn to finish before sending. |
| *(absent)* | The provider does not expose a `StreamingSession` at all. |

### Per-provider matrix

| Provider | `streaming_input.supported` | `streaming_input.semantics` |
|----------|-----------------------------|------------------------------|
| Claude   | `true` (native)             | `"queue"`                    |
| Codex    | `false`                     | *(absent)*                   |
| Gemini   | `false`                     | *(absent)*                   |
| Copilot  | `false`                     | *(absent)*                   |
| Ollama   | `false`                     | *(absent)*                   |

Claude's `"queue"` behavior comes from the Claude CLI's
`--input-format stream-json --replay-user-messages` mode: messages written
to stdin while the assistant is mid-response are not delivered immediately;
they are buffered and replayed as the next user turn once the current one
completes. If you need "interrupt" semantics on Claude, cancel the session
(drop the `StreamingSession`) and start a new one.

### Detecting turn boundaries

At the end of every agent turn a `StreamingSession` emits a `turn_complete`
event (with the provider's `stop_reason`, a zero-based `turn_index`, and
the turn's `usage`) followed immediately by a per-turn `result`. Drain
events until `turn_complete` to know the turn is over, then call
`send_user_message` to start the next turn. `turn_complete` is the
authoritative turn-boundary signal — don't key UI state off replayed
`user_message` events (they only appear when `--replay-user-messages` is
set and only fire *after* the next user message is sent). See
[Events and Logging: TurnComplete](events-and-logging.md#turncomplete) for
the full ordering contract.

### Branching on semantics

Consumers should branch on the `semantics` field rather than empirically
probing each provider. Example (TypeScript):

```ts
import { getCapability } from "zag";

const cap = await getCapability("claude");
switch (cap.features.streaming_input.semantics) {
  case "queue":
    // Safe to call sendUserMessage anytime — messages buffer between turns.
    break;
  case "interrupt":
    // sendUserMessage mid-turn will cancel the in-flight response.
    break;
  case "between-turns-only":
    // Must wait for the current turn to finish before sending.
    break;
  default:
    // Streaming input not supported on this provider.
}
```

Example (Python):

```python
from zag import get_capability

cap = await get_capability("claude")
match cap.features.streaming_input.semantics:
    case "queue":
        ...  # send any time; Claude replays at next turn boundary
    case "interrupt":
        ...
    case "between-turns-only":
        ...
    case _:
        ...  # not supported
```

## Session dependencies

Sessions can declare dependencies on other sessions for DAG workflows:

```bash
s1=$(zag spawn -p claude "extract requirements")
s2=$(zag spawn --depends-on "$s1" -p gemini "analyze requirements")
```

Dependencies are stored in the session record and enforce execution order. A session won't start until all its dependencies have completed.

## Environment variables

zag sets these environment variables during agent sessions:

| Variable | Description |
|----------|-------------|
| `ZAG_SESSION_ID` | Unique session identifier (UUID) |
| `ZAG_SESSION_NAME` | Human-readable session name (if set) |
| `ZAG_PROVIDER` | Current provider (e.g., `claude`) |
| `ZAG_MODEL` | Current model (e.g., `sonnet`) |
| `ZAG_PROCESS_ID` | Process identifier (for orchestration) |
| `ZAG_ROOT` | Worktree path (if using `-w`) |

Use `zag env` to export these for nested invocations:

```bash
eval $(zag env "$sid")
```

## Importing sessions

Import historical sessions from provider-native storage:

```bash
# Import from a specific provider
zag session import --provider claude

# Import from all providers
zag session import
```

This pulls session data from provider-specific locations (e.g., `~/.claude/projects/`, `~/.codex/history.jsonl`) into zag's unified session store.

## Deleting sessions

```bash
zag session delete <session-id>
```

## Garbage collection

Clean up old session data:

```bash
zag gc                     # default retention policy
zag gc --older-than 30d    # sessions older than 30 days
```

See `zag man gc` for details.

## Related

- [Orchestration](orchestration.md) -- Multi-agent coordination using sessions
- [Configuration](configuration.md) -- Session-related config options
- [Events & Logging](events-and-logging.md) -- Session event format and querying
- `zag man session` -- Session command reference
