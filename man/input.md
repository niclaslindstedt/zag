# zag input

Send a user message to a running or resumable session.

## Synopsis

    zag input <session-id> "message"
    zag input --latest "message"
    zag input --active "message"
    zag input --ps <pid> "message"
    echo "message" | zag input <session-id>
    zag input --stream <session-id>

## Description

`zag input` sends a user message to an existing agent session by resuming it non-interactively. This is the write counterpart to `zag listen` — while `listen` tails a session's output, `input` sends new messages into it.

The command resolves the target session, looks up the provider-native session ID, and uses the provider's resume mechanism to deliver the message. For Claude, this uses `--resume --print` with `--replay-user-messages`.

## Options

### `<session-id>`

The session ID to send input to. Supports both wrapper session IDs and provider-native session IDs. Prefix matching is supported if the prefix is unambiguous.

### `<message>`

The message text to send. If omitted (and `--stream` is not set), the message is read from stdin.

### `--latest`

Send to the most recently created session (by `started_at` in the session store).

### `--active`

Send to the most recently active session (by log file modification time).

### `--ps <PID>`

Send to the session belonging to a process, specified by OS PID (integer) or zag process UUID (from `zag ps list`). Mutually exclusive with `<session-id>`, `--latest`, and `--active`.

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

## Examples

    # Send a message to a specific session
    zag input abc123-def456 "What files did you change?"

    # Send to the latest session
    zag input --latest "Continue with the next step"

    # Send to the most active session
    zag input --active "Run the tests"

    # Pipe a message from stdin
    echo "Explain this error" | zag input --latest

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

## Provider Support

- **Claude**: Full support including `--stream` and `-o stream-json`
- **Codex**: Single message mode only
- **Gemini**: Single message mode (if resume is supported)
- **Copilot**: Single message mode (if resume is supported)

## See Also

    zag man listen    Tail session output in real-time
    zag man run       Start an interactive session
    zag man exec      Run non-interactively
    zag man ps        List and manage agent processes
