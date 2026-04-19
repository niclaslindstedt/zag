# zag ps

List, inspect, stop, and kill agent processes started by zag.

## Synopsis

    zag ps
    zag ps list [--running] [-p <provider>] [-n <N>] [--json]
    zag ps show <id|self> [--json]
    zag ps stop <id|self>
    zag ps kill <id|self>

## Description

`zag ps` tracks every agent invocation (run, exec, review) and records its OS process ID, provider, model, command, prompt, start time, and exit status.

Process entries are stored globally in `~/.zag/processes.json` so processes from all projects are visible regardless of the current working directory.

When listing processes, entries with status `running` are checked against the OS in real-time. If the process no longer exists (e.g., was killed externally or crashed), the status is shown as `dead`.

## Subcommands

### `list` (default)

List process entries, newest first.

    zag ps list
    zag ps list --running
    zag ps list -p claude
    zag ps list -n 5

#### `--running`

Show only processes that are currently alive in the OS.

#### `-p, --provider <provider>`

Filter by provider name (e.g., claude, codex, gemini, copilot, ollama).

#### `-n, --limit <N>`

Show only the N most recent processes.

#### `--json`

Output as a JSON array. Each object includes a `live_status` field with the real-time OS status.

---

### `show <id>`

Show full details of a single process entry.

    zag ps show <id>
    zag ps show <id> --json

#### `--json`

Output as a JSON object with a `live_status` field.

---

### `stop <id>`

Send `SIGHUP` to a running process — a soft stop request. Many processes treat SIGHUP as a signal to finish current work and exit gracefully. The process status in the store is not updated immediately; use `zag ps show <id>` to check whether it has exited.

    zag ps stop <id>

Errors if the process is not currently running.

---

### `kill <id>`

Send `SIGTERM` to a running process — a forceful termination request. Updates the process status to `killed` in the store.

    zag ps kill <id>

Errors if the process is not currently running.

---

## Self-referencing

The `show`, `stop`, and `kill` subcommands accept the special value `self` in place of a process ID. When `self` is used, zag resolves it from the `ZAG_PROCESS_ID` environment variable, which is automatically set inside every zag agent session.

This allows an agent to inspect or terminate its own process:

    zag ps show self
    zag ps stop self
    zag ps kill self

`zag ps kill self` targets the **agent subprocess** (claude, codex, gemini, copilot, or ollama), not the parent `zag` process. When the agent exits the parent naturally wakes up from its `wait()` and continues — letting an orchestrator like `zig run` proceed to the next step. The process-store entry's pid is updated to the agent child's pid the moment it spawns, so the registry and `zag ps show self` always reflect the process that will actually receive the signal.

If `self` is used outside a zag session (where `ZAG_PROCESS_ID` is not set), the command will error with a descriptive message.

## Status Values

| Status    | Meaning |
|-----------|---------|
| `running` | Process is alive in the OS |
| `exited`  | Process completed normally (exit code 0) |
| `killed`  | Process was terminated via SIGTERM (exit code 1) or failed |
| `dead`    | Stored as `running` but no longer exists in the OS (crashed or killed externally) |
| `unknown` | Unrecognised stored status |

## Storage

Process entries are stored in `~/.zag/processes.json`. Entries accumulate over time and are not automatically pruned.

## Examples

    # List all processes
    zag ps

    # Show only running processes
    zag ps list --running

    # Filter by provider
    zag ps list -p claude

    # Inspect a specific process
    zag ps show a1b2c3d4-...

    # Softly ask a process to stop (SIGHUP)
    zag ps stop a1b2c3d4-...

    # Forcefully terminate a process (SIGTERM)
    zag ps kill a1b2c3d4-...

    # An agent can inspect its own process
    zag ps show self

    # An agent can terminate itself
    zag ps kill self

    # JSON output for scripting
    zag ps list --json | jq '.[] | select(.live_status == "running")'

## See Also

    zag man session   List and inspect sessions
    zag man listen    Tail a session's log events in real-time
