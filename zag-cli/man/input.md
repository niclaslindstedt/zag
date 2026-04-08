# zag input

Send a user message to a single running or resumable session.

## Synopsis

    zag input "message"
    zag input --session <session-id> "message"
    zag input --name <session-name> "message"
    zag input --latest "message"
    zag input --active "message"
    zag input --ps <pid> "message"
    zag input --global "message"
    echo "message" | zag input
    zag input --stream --latest

## Description

`zag input` sends a user message to a single existing agent session by resuming it non-interactively. This is the write counterpart to `zag listen` — while `listen` tails a session's output, `input` sends new messages into it. To send a message to multiple sessions at once, use `zag broadcast`.

The command resolves the target session, looks up the provider-native session ID, and uses the provider's resume mechanism to deliver the message. For Claude, this uses `--resume --print` with `--replay-user-messages`.

If no session selector is given, `zag input` automatically resolves to the most recently created session in the current project. Use `--global` to search across all projects instead.

## Options

### `<message>`

The message text to send. If omitted (and `--stream` is not set), the message is read from stdin.

### `--session <SESSION_ID>`

Target a specific session by ID. Supports both wrapper session IDs and provider-native session IDs. Mutually exclusive with `--latest`, `--active`, `--ps`, and `--name`.

### `--latest`

Send to the most recently created session (by `started_at` in the session store).

### `--active`

Send to the most recently active session (by log file modification time).

### `--ps <PID>`

Send to the session belonging to a process, specified by OS PID (integer) or zag process UUID (from `zag ps list`). Mutually exclusive with `--session`, `--latest`, `--active`, and `--name`.

### `--name <NAME>`

Target a session by its human-readable name (set via `--name` on `run`/`exec`). Resolves to the most recent session with the given name. Mutually exclusive with `--session`, `--latest`, `--active`, and `--ps`.

### `--global`

When auto-resolving (no explicit session selector), search across all projects instead of only the current project.

### `--stream`

Stream multiple messages from stdin. Each line of stdin is sent as a separate user message. Claude only — uses `--input-format stream-json` and `--replay-user-messages` for bidirectional streaming.

### `-o, --output <FORMAT>`

Output format for the agent's response:

- **text** (default): Plain text — prints the assistant's final response
- **json**: Compact JSON — the full `AgentOutput` structure
- **json-pretty**: Pretty-printed JSON
- **stream-json**: Streaming NDJSON — each event is a JSON line (Claude only)

### `-r, --root <PATH>`

Root directory for session resolution.

### `--raw`

Send the message without agent-to-agent envelope wrapping. By default, when `zag input` is called from within a zag session (detected via `ZAG_SESSION_ID`), the message is wrapped with sender metadata and reply instructions. Use `--raw` to send the message verbatim.

### `--file <PATH>`

Attach a file to the message (repeatable). Text files ≤50 KB are embedded inline; larger text files and binary files are included as metadata references with `@path` so the agent can access them with its own tools.

## Agent-to-Agent Messaging

When `zag input` is invoked from within a zag session (i.e., by an agent), the message is automatically wrapped in an envelope containing the sender's identity and reply instructions:

```
<agent-message>
<from session="abc123-def456" name="frontend-agent" provider="claude" model="opus"/>
<reply-with>zag input --name frontend-agent "your reply here"</reply-with>
<body>
Original message content here
</body>
</agent-message>
```

The `name` attribute is included when the sender session was created with `--name`. The `<reply-with>` command uses `--name` when available, falling back to `--session` otherwise. The receiving agent can use this command to send a response back.

Session detection uses the `ZAG_SESSION_ID` and `ZAG_SESSION_NAME` environment variables, which are automatically set when `zag run` or `zag exec` spawns an agent subprocess. When not inside a session, messages are sent without wrapping.

## Examples

    # Send a message (auto-resolves to the most recent session in this project)
    zag input "What files did you change?"

    # Send a message to the latest session globally
    zag input --global "What files did you change?"

    # Send to a specific session by ID
    zag input --session abc123-def456 "What files did you change?"

    # Send to the latest session
    zag input --latest "Continue with the next step"

    # Send to the most active session
    zag input --active "Run the tests"

    # Pipe a message from stdin
    echo "Explain this error" | zag input

    # Get JSON output
    zag input --latest "list 3 colors" -o json

    # Stream NDJSON events (Claude only)
    zag input --latest "complex task" -o stream-json

    # Stream multiple messages interactively (Claude only)
    zag input --stream --latest

    # Send to a session by OS PID
    zag input --ps 12345 "status update"

    # Send to a session by zag process UUID
    zag input --ps a1b2c3d4-... "check progress"

    # Send to a named session
    zag input --name backend-agent "check the auth module"

    # Agent-to-agent: send a message (auto-wraps with sender info when inside a session)
    zag input --session <target-session-id> "please run the tests"

    # Agent-to-agent: send without envelope wrapping
    zag input --raw --session <target-session-id> "raw message"

## Interactive Sessions

When the target session was spawned with `zag spawn --interactive`, the session is backed by a FIFO (named pipe). In this case, `zag input` writes the message directly to the FIFO instead of using the resume mechanism. This is faster and keeps the long-lived agent process alive.

Interactive sessions are detected automatically — if the FIFO exists at `~/.zag/fifos/<session_id>`, the message is sent via the pipe. No special flags are needed on the `zag input` side.

```bash
# Start an interactive session
sid=$(zag spawn --interactive --name worker -p claude)

# Send messages (automatically uses FIFO)
zag input --name worker "analyze the auth module"
zag input --name worker "now check the tests"
```

## Provider Support

- **Claude**: Full support including `--stream` and `-o stream-json`
- **Codex**: Single message mode only
- **Gemini**: Single message mode (if resume is supported)
- **Copilot**: Single message mode (if resume is supported)

## See Also

    zag man broadcast Send a message to multiple sessions by tag
    zag man listen    Tail session output in real-time
    zag man spawn     Launch background sessions (including --interactive)
    zag man run       Start an interactive session
    zag man exec      Run non-interactively
    zag man ps        List and manage agent processes
