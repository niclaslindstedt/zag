# agent session

List and inspect sessions.

## Synopsis

```
agent session <command> [options]
```

## Description

Manage and inspect agent sessions tracked in `~/.agent/projects/<sanitized-path>/sessions.json`.

Sessions are automatically created when running agents with `agent run` or `agent exec`. This command provides read access to session history and the ability to import historical provider logs.

## Commands

### list

List all sessions, sorted by creation time (newest first).

```
agent session list [--json] [-p provider] [-n limit]
```

Options:
- `--json` — Output as JSON array
- `-p, --provider` — Filter by provider name (e.g., claude, codex, gemini)
- `-n, --limit` — Show only the N most recent sessions

### show

Show details of a specific session.

```
agent session show <id> [--json]
```

Options:
- `--json` — Output as JSON object

The `<id>` can be either the wrapper session ID or the provider-native session ID.

### import

Import historical provider logs into the session store. Previously imported sessions are skipped.

```
agent session import
```

Supported providers: Claude, Codex, Gemini, Copilot, Ollama (no-op today).

## Examples

```bash
# List all sessions
agent session list

# List sessions as JSON
agent session list --json

# List only Claude sessions
agent session list -p claude

# Show the 5 most recent sessions
agent session list -n 5

# Show details of a specific session
agent session show abc123-def456

# Show session details as JSON
agent session show abc123-def456 --json

# Import historical provider logs
agent session import
```

## See Also

- `agent listen` — Tail a session's log events in real-time
- `agent run --resume` — Resume a specific session
- `agent man` — Show all available manpages
