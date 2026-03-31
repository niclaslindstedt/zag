# zag spawn

Launch an agent session in the background and return the session ID.

## Synopsis

    zag spawn [flags] <prompt>

## Description

Spawns a new agent session as a background process and immediately prints the session ID. The spawned session runs `zag exec` under the hood with stdout/stderr redirected to a log file.

This is designed for orchestration scripts that need to launch multiple agents in parallel and later synchronize with `zag wait` or collect results with `zag collect`.

## Arguments

    prompt    The prompt to send to the agent

## Flags

    -p, --provider <PROVIDER>    Provider to use (default: configured provider)
    --model <MODEL>              Model name or size alias
    -r, --root <PATH>            Root directory for the agent
    -a, --auto-approve           Skip permission prompts
    -s, --system-prompt <TEXT>   Custom system prompt
    --add-dir <PATH>             Additional directories (repeatable)
    --size <SIZE>                Ollama parameter size
    --max-turns <N>              Maximum number of agentic turns
    --name <NAME>                Session name for discovery
    --description <TEXT>         Session description
    --tag <TAG>                  Session tag (repeatable)
    --json                       Output session info as JSON

## Output

By default, prints the session ID to stdout:

    a1b2c3d4-e5f6-7890-abcd-ef1234567890

With `--json`, outputs:

    {"session_id":"...","pid":12345,"log_path":"~/.zag/logs/spawn/....log"}

## Examples

    # Spawn a single background agent
    sid=$(zag spawn "analyze the auth module")

    # Spawn with metadata
    sid=$(zag spawn --name analyzer --tag batch -p claude "analyze code")

    # Spawn multiple agents in parallel
    sid1=$(zag spawn --tag batch "analyze auth")
    sid2=$(zag spawn --tag batch "review tests")
    sid3=$(zag spawn --tag batch "find security issues")

    # Wait for all to complete
    zag wait --tag batch --timeout 10m

    # Collect results
    zag collect --tag batch --json

## See Also

    zag man wait       Block until sessions complete
    zag man status     Session health check
    zag man collect    Gather multi-session results
    zag man exec       Non-interactive execution
