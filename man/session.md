# zag session

List and inspect sessions.

## Synopsis

    zag session <command> [options]

## Description

Manage and inspect agent sessions tracked in `~/.zag/projects/<sanitized-path>/sessions.json`.

Sessions are automatically created when running agents with `zag run` or `zag exec`. This command provides read access to session history and the ability to import historical provider logs.

## Commands

### list

List all sessions, sorted by creation time (newest first).

    zag session list [--json] [-p provider] [-n limit] [--global] [--name NAME] [--tag TAG] [--parent SESSION_ID]

Options:
- `--json` — Output as JSON array
- `-p, --provider` — Filter by provider name (e.g., claude, codex, gemini)
- `-n, --limit` — Show only the N most recent sessions
- `--global` — List sessions across all projects (not just the current one)
- `--name` — Filter by session name (case-insensitive substring match)
- `--tag` — Filter by tag (case-insensitive exact match)
- `--parent` — Show only sessions spawned by this parent session ID

### show

Show details of a specific session.

    zag session show <id> [--json]

Options:
- `--json` — Output as JSON object

The `<id>` can be either the wrapper session ID or the provider-native session ID.

### delete

Delete a session from the store.

    zag session delete <id> [--json]

Options:
- `--json` — Output as JSON object

The session entry is removed from the store. Associated log files on disk are not deleted.

### update

Update session metadata (name, description, tags).

    zag session update <id> [--name NAME] [--description TEXT] [--tag TAG ...] [--clear-tags]

Options:
- `--name` — Set the session name
- `--description` — Set the session description
- `--tag` — Add a tag (repeatable)
- `--clear-tags` — Clear all existing tags before adding new ones

Only provided fields are updated; omitted fields remain unchanged.

### import

Import historical provider logs into the session store. Previously imported sessions are skipped.

    zag session import

Supported providers: Claude, Codex, Gemini, Copilot, Ollama (no-op today).

## Examples

    # List all sessions
    zag session list

    # List sessions as JSON
    zag session list --json

    # List only Claude sessions
    zag session list -p claude

    # Show the 5 most recent sessions
    zag session list -n 5

    # List sessions across all projects
    zag session list --global

    # Filter by session name
    zag session list --name frontend

    # Filter by tag
    zag session list --tag backend

    # Update session metadata
    zag session update abc123-def456 --name "my-agent" --tag backend
    zag session update abc123-def456 --clear-tags --tag new-tag

    # Show details of a specific session
    zag session show abc123-def456

    # Show session details as JSON
    zag session show abc123-def456 --json

    # Delete a session
    zag session delete abc123-def456

    # Import historical provider logs
    zag session import

## See Also

    zag man listen    Tail a session's log events in real-time
    zag man run       Resume a specific session with --resume
    zag man zag       Global flags and providers overview
