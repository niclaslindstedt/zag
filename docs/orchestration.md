# Orchestration

zag's orchestration system lets you coordinate multiple AI agent sessions from the shell. You can spawn agents in parallel, chain their outputs, build DAG workflows, and monitor everything in real time.

## Core primitives

These commands form the orchestration toolkit:

| Command | Purpose |
|---------|---------|
| `spawn` | Launch a background agent session, return session ID |
| `wait` | Block until session(s) complete |
| `collect` | Gather results from multiple sessions |
| `pipe` | Chain one session's output into a new agent |
| `status` | Machine-readable session health check |
| `cancel` | Gracefully stop a running session |
| `retry` | Re-run a failed session with the same config |
| `output` | Extract final result text from a session |
| `gc` | Clean up old session data |

### Communication

| Command | Purpose |
|---------|---------|
| `input` | Send a message to a running/resumable session |
| `broadcast` | Send a message to all sessions (or filtered by tag) |
| `listen` | Tail a session's log events in real time |
| `subscribe` | Multiplexed event stream from all active sessions |
| `watch` | Execute a command when a session event matches a filter |
| `log` | Append custom events to a session log |
| `env` | Export session environment variables for nested invocations |

## Spawning agents

`zag spawn` launches an agent in the background and prints its session ID:

```bash
sid=$(zag spawn -p claude "review the auth module")
echo "Session: $sid"
```

Use `--json` to get structured output including the PID and log path:

```bash
zag spawn --json -p claude "analyze performance" | jq .session_id
```

### Tags and naming

Tag and name sessions for easy discovery:

```bash
zag spawn --name reviewer --tag batch -p claude "review auth"
zag spawn --tag batch -p gemini "review tests"
```

## Waiting for completion

Block until one or more sessions finish:

```bash
# Wait for a single session
zag wait "$sid"

# Wait for all sessions with a tag
zag wait --tag batch

# With timeout
zag wait --tag batch --timeout 5m
```

## Collecting results

Gather output from multiple sessions:

```bash
# Collect by tag
zag collect --tag batch

# Collect specific sessions
zag collect "$sid1" "$sid2"

# JSON output
zag collect --tag batch --json
```

## Chaining with pipe

Feed one session's output into a new agent:

```bash
# Generate code, then review it
sid=$(zag spawn -p claude "implement a rate limiter")
zag wait "$sid"
zag pipe "$sid" -p gemini "review this implementation for bugs"
```

## Patterns

### Sequential pipeline

Agents execute in order. Each agent's output feeds the next:

```bash
s1=$(zag spawn -p claude "extract API endpoints from the codebase")
s2=$(zag spawn --depends-on "$s1" -p gemini "generate OpenAPI spec for these endpoints")
s3=$(zag spawn --depends-on "$s2" -p claude "write integration tests for this spec")
zag wait "$s3"
zag output "$s3"
```

The `--depends-on` flag creates explicit dependencies. A session won't start until its dependencies complete. Use `--inject-context` to automatically inject dependency outputs into the prompt.

### Fan-out / gather

Run the same task across multiple providers and compare:

```bash
for provider in claude codex gemini; do
  zag spawn --tag compare -p "$provider" "optimize the sort function"
done
zag wait --tag compare
zag collect --tag compare
```

### Generator and critic

One agent generates, another reviews, iterate:

```bash
gen=$(zag spawn -p claude "implement a caching layer")
zag wait "$gen"
review=$(zag pipe "$gen" -p gemini "review this code for correctness and edge cases")
zag wait "$review"
zag output "$review"
```

### Coordinator / dispatcher

A central agent delegates subtasks:

```bash
# Coordinator analyzes the task, spawns workers
coord=$(zag spawn -p claude "analyze the codebase and list the top 3 modules that need refactoring")
zag wait "$coord"

# Parse output and spawn workers
for module in $(zag output "$coord" | head -3); do
  zag spawn --tag refactor -p claude "refactor $module"
done
zag wait --tag refactor
```

## Interactive sessions

Spawn a long-lived session that stays alive for ongoing interaction:

```bash
sid=$(zag spawn --interactive --name worker -p claude)

# Send messages
zag input --name worker "analyze the auth module"
zag input --name worker "now refactor the error handling"

# Stream output
zag listen --name worker

# Cancel when done
zag cancel --name worker
```

Interactive sessions use FIFO pipes under `~/.zag/fifos/` and require the Claude provider.

## DAG workflows

Use `--depends-on` to build directed acyclic graphs of sessions:

```bash
# Parse → Analyze → Report (sequential)
s1=$(zag spawn -p claude "parse the log files")
s2=$(zag spawn --depends-on "$s1" -p gemini "analyze patterns")
s3=$(zag spawn --depends-on "$s2" -p claude "write a report")

# Or fan-out then converge
base=$(zag spawn -p claude "extract requirements")
a=$(zag spawn --depends-on "$base" --tag analysis -p claude "analyze security")
b=$(zag spawn --depends-on "$base" --tag analysis -p gemini "analyze performance")
final=$(zag spawn --depends-on "$a" --depends-on "$b" -p claude "synthesize findings")
zag wait "$final"
```

With `--inject-context`, dependency outputs are automatically prepended to the prompt.

## Monitoring

### Session status

```bash
zag status "$sid"              # running, completed, failed, dead, unknown
zag status --tag batch --json  # all sessions with a tag
```

### Real-time events

```bash
# Tail a single session
zag listen "$sid"
zag listen --name worker --rich-text --timestamps

# All sessions
zag subscribe --json

# Filtered
zag subscribe --tag batch --json | jq 'select(.type == "session_ended")'
```

### Event-driven automation

```bash
# Run a command when any session ends
zag watch --filter session_ended -- echo "A session finished"

# React to specific events
zag watch --filter tool_call --tag batch -- ./notify.sh
```

## Lifecycle management

### Cancelling sessions

```bash
zag cancel "$sid"
zag cancel --name worker
zag cancel --tag batch          # cancel all tagged sessions
```

### Retrying failed sessions

```bash
zag retry "$sid"                # re-run with same config
```

### Garbage collection

```bash
zag gc                          # clean up old session data
zag gc --older-than 7d          # only sessions older than 7 days
```

## Related

- `zag man orchestration` -- Exhaustive patterns reference with 9 orchestration topologies
- [Sessions](sessions.md) -- Session lifecycle and management
- [Events & Logging](events-and-logging.md) -- Event format and log access
- [Getting Started](getting-started.md) -- Basic orchestration example
