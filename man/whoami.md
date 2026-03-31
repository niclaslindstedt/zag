# zag whoami

Show identity of the current zag session. Designed for agent introspection — lets a running agent discover which session and process it belongs to.

## Synopsis

    zag whoami [--json]

## Description

`zag whoami` reads environment variables set by the parent `zag` process to report the current session identity. When `zag run` or `zag exec` spawns an agent, it sets `ZAG_SESSION_ID`, `ZAG_PROCESS_ID`, `ZAG_PROVIDER`, `ZAG_MODEL`, and `ZAG_ROOT` in the child process environment.

If the command is run outside of a zag session (i.e., no `ZAG_*` environment variables are set), it exits with an error.

When parent tracking information is available (from a nested zag invocation), the parent session and process IDs are also shown.

## Options

### `--json`

Output as a JSON object with all fields.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZAG_SESSION_ID` | Session UUID of the enclosing zag process |
| `ZAG_PROCESS_ID` | Process UUID of the enclosing zag process |
| `ZAG_PROVIDER` | Provider name (claude, codex, gemini, copilot, ollama) |
| `ZAG_MODEL` | Model name |
| `ZAG_ROOT` | Project root path |

## Output Fields

| Field | Description |
|-------|-------------|
| Session ID | The wrapper session UUID |
| Process ID | The zag process UUID |
| PID | OS process ID of the current process |
| Provider | Agent provider name |
| Model | Model being used |
| Root | Project root directory |
| Parent Session ID | Session ID of the parent zag process (if nested) |
| Parent Process ID | Process ID of the parent zag process (if nested) |

## Examples

    # Inside a zag session (called by an agent)
    zag whoami
    Session ID:        a1b2c3d4-...
    Process ID:        e5f6g7h8-...
    PID:               12345
    Provider:          claude
    Model:             opus
    Root:              /home/user/project

    # JSON output for machine consumption
    zag whoami --json
    {"session_id":"a1b2c3d4-...","process_id":"e5f6g7h8-...","pid":12345,...}

    # Outside a zag session
    zag whoami
    Error: Not running inside a zag session.

## Use Cases

- **Agent self-discovery**: An agent running inside a zag session can call `zag whoami` to learn its own session and process identity.
- **Inter-agent messaging**: Agents can share their session IDs to enable future cross-session communication.
- **Session hierarchy**: Nested zag invocations track parent/child relationships via `parent_session_id` and `parent_process_id`.

## See Also

    zag man ps        List and inspect agent processes
    zag man session   List and inspect sessions
