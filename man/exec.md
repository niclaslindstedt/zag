# zag exec

Run an agent non-interactively.

## Synopsis

    zag [flags] exec [options] <prompt>

## Description

Sends a single prompt to the agent, prints the output, and exits. This is the primary command for scripting, pipelines, and programmatic use.

By default, exec mode suppresses wrapper UI (spinners, status messages, icons) so the output is clean for piping. Use `--verbose` to restore the styled output.

## Arguments

    prompt    The prompt to send to the agent (required)

## Flags

    -o, --output <FORMAT>         Output format (see Output Formats below)
    -i, --input-format <FORMAT>   Input format: text (default), stream-json (Claude only)

All global flags apply (see `zag man zag`).

## Output Formats

Control the output format with `-o <format>`:

    (default)      Streams events as formatted text in real-time. Claude converts
                   its NDJSON stream to readable text with tool call indicators.
                   Other agents stream raw text.

    text           Plain text pass-through. Bypasses all JSON parsing and streams
                   the agent's raw stdout directly. Use when you want unprocessed
                   output or the agent doesn't support structured output.

    json           Compact JSON (single line). Captures the full session, converts
                   to the unified AgentOutput format, then outputs as minified JSON.
                   Use for programmatic parsing when you need the complete session
                   including tool calls, usage stats, and the final result.

    json-pretty    Same as json but pretty-printed with indentation. Useful for
                   debugging or inspecting structured output manually.

    stream-json    Streaming NDJSON — one unified Event per line as it happens.
                   Each line is a self-contained JSON object. Use for real-time
                   processing of agent events in pipelines or monitoring tools.

    native-json    Claude's raw JSON output without conversion to unified format.
                   Claude-only. Use when you need Claude's native event schema
                   (e.g., for direct API compatibility).

## Input Formats (Claude Only)

    text           Default. Plain text from stdin.
    stream-json    Streaming NDJSON input for structured/realtime input.

## JSON Output Mode

The `--json`, `--json-schema`, and `--json-stream` global flags provide a higher-level JSON mode designed for getting structured data from agents:

    --json               Instruct the agent to respond with JSON. Non-Claude agents
                         get an augmented system prompt with JSON instructions. The
                         output is the raw JSON from the agent (not wrapped in
                         AgentOutput), minified on a single line.

    --json-schema        Same as --json, plus validates the output against the given
                         JSON schema. Accepts a file path or inline JSON string. On
                         validation failure, retries up to 3 times by resuming the
                         session with a correction prompt.

    --json-stream        Stream JSON events (NDJSON). Mutually exclusive with --json
                         and --json-schema. Convenience shorthand for -o stream-json.

The difference between `-o json` and `--json`:
- `-o json` outputs the full AgentOutput envelope (session ID, events, usage, etc.)
- `--json` outputs only the agent's response as raw JSON (intended for structured data extraction)

## Sandbox Mode

The `--sandbox` flag runs the agent inside a Docker sandbox microVM for stronger isolation. See `zag man run` for full details.

    zag --sandbox exec "say hello"           Run in auto-named sandbox
    zag --sandbox my-name exec "say hello"   Run in named sandbox

In exec mode, the sandbox is kept after execution (no cleanup prompt). Resume with `zag run --resume <session-id>`.

## Examples

    zag exec "say hello"                              Simple prompt
    zag exec "list files" -o json                     Full session as JSON
    zag exec --json "list 3 colors"                   Raw JSON response
    zag exec --json-schema schema.json "get users"    Validated against schema
    zag exec -o stream-json "complex task"            Real-time NDJSON events
    zag exec -o text "simple question"                Raw text, no parsing
    zag -q exec "write tests" | less                  Pipe clean output
    zag -v exec "analyze code"                        Verbose with icons
    zag --sandbox exec "write tests"                  Run in Docker sandbox
    zag --session $(uuidgen) exec "say hello"          Pre-set session ID
    zag -p ollama exec "explain this code"            Ollama non-interactive
    zag -p ollama --size 35b exec "complex task"      Ollama with large size

    echo '{"data":"input"}' | agent exec -i stream-json "process"   Structured input

## See Also

    zag man run       Interactive alternative
