# zag spawn

Launch an agent session in the background and return the session ID.

## Synopsis

    zag spawn [flags] [<prompt>]

## Description

Spawns a new agent session as a background process and immediately prints the session ID. By default the spawned session runs `zag exec` under the hood with stdout/stderr redirected to a log file.

With `--interactive`, spawns a long-lived FIFO-based streaming session that stays alive until killed. Interactive sessions can receive messages via `zag input` and be monitored with `zag listen`, making them ideal for `zag connect` workflows where you start a persistent session on a remote server and interact with it over time.

## Arguments

    prompt    The prompt to send to the agent (required unless --interactive)

## Flags

    -p, --provider <PROVIDER>    Provider to use (default: configured provider)
    --model <MODEL>              Model name or size alias
    -r, --root <PATH>            Root directory for the agent
    -a, --auto-approve           Skip permission prompts
    -s, --system-prompt <TEXT>   Custom system prompt
    --add-dir <PATH>             Additional directories (repeatable)
    --env <KEY=VALUE>            Environment variable for subprocess (repeatable)
    --mcp-config <CONFIG>        MCP server config: JSON string or path to a JSON file (Claude only)
    --size <SIZE>                Ollama parameter size
    --max-turns <N>              Maximum number of agentic turns
    --timeout <DURATION>         Timeout duration (e.g., 30s, 5m, 1h). Kills the agent if exceeded.
    --name <NAME>                Session name for discovery
    --description <TEXT>         Session description
    --tag <TAG>                  Session tag (repeatable)
    --show-usage                 Show token usage statistics (JSON output mode only)
    --json                       Output session info as JSON
    --depends-on <SESSION_ID>    Wait for these sessions to complete before starting (repeatable)
    --inject-context             Auto-inject dependency session results as context
    -I, --interactive            Spawn a long-lived interactive session (FIFO-based)

## Output

By default, prints the session ID to stdout:

    a1b2c3d4-e5f6-7890-abcd-ef1234567890

With `--json`, outputs:

    {"session_id":"...","pid":12345,"log_path":"~/.zag/logs/spawn/....log","interactive":false}

## Interactive Sessions

When `--interactive` is set, the session is backed by a FIFO (named pipe) at `~/.zag/fifos/<session_id>`. A background relay process streams messages between the FIFO and the agent's CLI in bidirectional NDJSON mode. The process stays alive until explicitly killed (via `zag cancel`) or the agent exits.

Interactive sessions currently require the Claude provider.

Use `zag input` to send messages and `zag listen` to monitor output:

    sid=$(zag spawn --interactive --name worker -p claude)
    zag input --name worker "analyze the auth module"
    zag listen --name worker

## Examples

    # Spawn a single background agent
    sid=$(zag spawn "analyze the auth module")

    # Spawn with metadata
    sid=$(zag spawn --name analyzer --tag batch -p claude "analyze code")

    # Spawn an interactive session (no initial prompt)
    sid=$(zag spawn --interactive --name worker -p claude)

    # Spawn an interactive session with an initial prompt
    sid=$(zag spawn --interactive --name worker -p claude "set up the project")

    # Send messages to an interactive session
    zag input --name worker "analyze the auth module"
    zag input --name worker "now check the tests"

    # Monitor an interactive session
    zag listen --name worker

    # Spawn multiple agents in parallel
    sid1=$(zag spawn --tag batch "analyze auth")
    sid2=$(zag spawn --tag batch "review tests")
    sid3=$(zag spawn --tag batch "find security issues")

    # Wait for all to complete
    zag wait --tag batch --timeout 10m

    # Collect results
    zag collect --tag batch --json

## See Also

    zag man input      Send messages to a session
    zag man listen     Tail session output in real-time
    zag man wait       Block until sessions complete
    zag man status     Session health check
    zag man collect    Gather multi-session results
    zag man exec       Non-interactive execution
