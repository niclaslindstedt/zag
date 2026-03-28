# zag search

Search through session logs across providers and projects.

## Synopsis

    zag search [QUERY] [OPTIONS]

## Description

`zag search` scans normalized session logs (`.jsonl` files) for matching events. It searches across user messages, assistant messages, tool calls, tool results, reasoning, and other event types.

By default, the search is scoped to sessions started from the current directory and its subdirectories. Use `--global` to search all sessions across all projects.

Sessions imported via `zag session import` are included automatically.

## Arguments

### `QUERY`

Text to search for. By default this is a case-insensitive literal substring. Use `--regex` to treat it as a regular expression. If omitted, all events matching the active filters are returned.

## Options

### `-p, --provider <NAME>`

Filter by provider name (e.g. `claude`, `codex`, `gemini`, `copilot`, `ollama`).

### `--role <ROLE>`

Filter to only `user` or `assistant` message events.

### `--tool <NAME>`

Filter to tool call and tool result events whose tool name contains `NAME` (case-insensitive).

### `--tool-kind <KIND>`

Filter by normalised tool kind. Available values:

| Kind        | Description |
|-------------|-------------|
| `shell`     | Shell/command execution |
| `file-read` | File read operations |
| `file-write`| File creation/overwrite |
| `file-edit` | File modification/patching |
| `search`    | File/content search |
| `sub-agent` | Sub-agent delegation |
| `web`       | Web/network operations |
| `notebook`  | Notebook operations |
| `other`     | Unclassified |

### `--from <DATE>`

Show only events at or after this date/time. Accepts:
- ISO 8601: `2024-01-15` or `2024-01-15T10:30:00Z`
- Relative: `1h` (1 hour ago), `2d` (2 days), `3w` (3 weeks), `1m` (~30 days)

### `--to <DATE>`

Show only events at or before this date/time. Same format as `--from`.

### `--session <SESSION_ID>`

Restrict search to a specific session ID. Accepts a prefix match — you can use the first 8 characters of a UUID.

### `--global`

Search all sessions across all projects (default: current project and sub-projects only).

### `--regex`

Treat the query as a regular expression instead of a literal substring.

### `--case-sensitive`

Enable case-sensitive matching (default is case-insensitive).

### `-j, --json`

Output results as NDJSON (one JSON object per match). Each object includes the full event, session metadata, and a text snippet.

### `-c, --count`

Output only the total count of matches as a single integer.

### `-n, --limit <N>`

Stop after collecting N matches.

### `--root <DIR>`

Override the current working directory used for project scope resolution.

## Examples

    # Search for any mention of "login" in current project
    zag search login

    # Search globally (all projects)
    zag search --global "authentication"

    # Search only user messages containing "refactor"
    zag search --role user refactor

    # Search for bash tool calls in the last 7 days
    zag search --tool bash --from 7d

    # Search by tool kind
    zag search --tool-kind shell "cargo test"

    # Filter by provider
    zag search --provider claude "error handling"

    # Count matches without showing them
    zag search --count "TODO"

    # JSON output for scripting
    zag search --json "api key" | jq '.snippet'

    # Regex search
    zag search --regex "fn\s+\w+_handler"

    # Restrict to a time range
    zag search --from 2024-01-01 --to 2024-02-01 "deployment"

    # Restrict to a specific session
    zag search --session abc12345 "error"

## Scope

Without `--global`, only sessions whose `workspace_path` is the current directory or a subdirectory are searched. For example, running `zag search` from `/code/work/` will include sessions from `/code/work/projectA` and `/code/work/projectB` but not `/code/other/`.

## See Also

    zag man session   List and inspect sessions
    zag man listen    Tail a session's log events in real-time
