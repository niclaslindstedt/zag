# zag

A unified CLI for AI coding agents.

## Synopsis

    zag [flags] <command> [options]

## Description

`zag` provides a single interface for running multiple AI coding agents — Claude, Codex, Gemini, Copilot, and Ollama. Instead of learning five different CLIs with different flags and model names, you use one consistent command with unified options.

The CLI handles model resolution (size aliases like `small`/`medium`/`large`), configuration management, worktree isolation, Docker sandboxing, structured JSON output with schema validation, unified session logs, provider-agnostic skills, automatic provider/model selection, and remote orchestration over HTTPS via `zag serve`/`zag connect`.

## Global Flags

These flags can be used with any subcommand.

    -d, --debug                 Enable debug logging
    -q, --quiet                 Suppress all logging except agent output
    -v, --verbose               Show styled output with icons and status messages
        --no-health-check       Skip health check before proxying to a remote server
        --help-agent            Print AI-oriented reference for using this CLI

## Agent Flags

These flags apply to the agent-execution commands (`run`, `exec`, `review`, `plan`, `spawn`, `pipe`). They configure the provider, model, working tree, and subprocess environment.

    -p, --provider <NAME>       Provider: claude, codex, gemini, copilot, ollama, auto
    -m, --model <NAME>          Model name, size alias (small/medium/large), or auto
    -r, --root <PATH>           Root directory to run the agent in
    -a, --auto-approve          Skip permission prompts (auto-approve all actions)
    -s, --system-prompt <TEXT>  Custom system prompt appended to the agent's default
        --add-dir <PATH>        Additional directories to include (repeatable)
        --file <PATH>           Attach a file to the prompt (repeatable)
        --env <KEY=VALUE>       Environment variable for the agent subprocess (repeatable)
        --size <SIZE>           Model parameter size for Ollama (e.g., 2b, 9b, 35b)
        --max-turns <N>         Maximum number of agentic turns
        --mcp-config <CONFIG>   MCP server config: JSON string or path to a JSON file (Claude only)
        --show-usage            Show token usage statistics (JSON output mode only)

## Session Isolation Flags

These flags apply to `run` and `exec` and control how the session is isolated and serialized.

    -w, --worktree [NAME]       Run in an isolated git worktree (optional name)
        --sandbox [NAME]        Run inside a Docker sandbox (optional name)
        --session <UUID>        Use a specific session ID instead of auto-generating one
        --json                  Request structured JSON output from the agent
        --json-schema <SCHEMA>  Validate JSON output against a schema (file or inline)

## Session Metadata Flags

Applied to `run`, `exec`, and `spawn` for session discovery.

    --name <NAME>               Human-readable session name
    --description <TEXT>        Short description of the session's purpose
    --tag <TAG>                 Session tag (repeatable)

## Commands

Agent execution:

    run          Start an interactive session
    exec         Run non-interactively (print output and exit)
    review       Review code changes
    plan         Generate a Markdown implementation plan
    spawn        Launch a background agent session (or --interactive for long-lived)
    pipe         Chain results from completed sessions into a new session

Configuration and discovery:

    config       View or set configuration values
    capability   Show provider capability declarations
    discover     Discover available providers, models, and capabilities
    skills       Manage provider-agnostic skills
    mcp          Manage MCP servers across providers
    man          Show manual pages for commands

Session management:

    session      List and inspect sessions, import historical logs
    listen       Tail a session's log events in real-time
    ps           List, inspect, and manage agent processes
    search       Search through session logs
    input        Send a user message to a running or resumable session
    broadcast    Send a message to all sessions in the project
    whoami       Show identity of the current zag session
    wait         Block until session(s) complete
    status       Machine-readable session health check
    collect      Gather results from multiple sessions
    env          Export session environment variables
    events       Query structured events from session logs
    cancel       Gracefully cancel running sessions
    summary      Show log-based session summaries and stats
    watch        Watch session logs and execute commands on matching events
    subscribe    Subscribe to a multiplexed event stream from all sessions
    log          Append custom structured events to a session log
    output       Extract final result text from a session
    retry        Re-run failed sessions with the same configuration
    gc           Clean up old session data, logs, and process entries

Remote access:

    serve        Start the zag HTTPS/WebSocket server for remote access
    connect      Connect to a remote zag server
    disconnect   Disconnect from a remote zag server
    user         Manage user accounts on the server

Run `zag man <command>` for detailed help on each command.

## Providers

    claude    Default. Models: default, sonnet, opus, haiku (default: default)
    codex     Models: gpt-5.4, gpt-5.4-mini, gpt-5.3-codex-spark, gpt-5.3-codex,
              gpt-5-codex, gpt-5.2-codex, gpt-5.2, o4-mini, gpt-5.1-codex-max,
              gpt-5.1-codex-mini (default: gpt-5.4)
    gemini    Models: auto, gemini-3.1-pro-preview, gemini-3.1-flash-lite-preview,
              gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro,
              gemini-2.5-flash, gemini-2.5-flash-lite (default: auto)
    copilot   Models: claude-sonnet-4.6, claude-haiku-4.5, claude-opus-4.6,
              claude-sonnet-4.5, claude-opus-4.5, gpt-5.4, gpt-5.4-mini,
              gpt-5.3-codex, gpt-5.2-codex, gpt-5.2, gpt-5.1-codex-max,
              gpt-5.1-codex, gpt-5.1, gpt-5, gpt-5.1-codex-mini, gpt-5-mini,
              gpt-4.1, gemini-3.1-pro-preview, gemini-3-pro-preview
              (default: claude-sonnet-4.6)
    ollama    Local models via Ollama (default: qwen3.5:9b). Use --size to pick
              a parameter size (0.8b, 2b, 4b, 9b, 27b, 35b, 122b).

Use `zag discover` for a live summary of installed providers and their models, or `zag discover --resolve <alias>` to trace a size alias through the active provider.

## Model Size Aliases

Size aliases resolve to the appropriate model for the active provider:

    small  (s)      Lightweight, fast — haiku / gpt-5.4-mini /
                    gemini-3.1-flash-lite-preview / claude-haiku-4.5
    medium (m)      Balanced — sonnet / gpt-5.3-codex / gemini-2.5-flash /
                    claude-sonnet-4.6
    large  (l/max)  Most capable — default / gpt-5.4 / gemini-3.1-pro-preview /
                    claude-opus-4.6

`auto` (as provider or model) asks zag to pick a provider and/or model based on prompt analysis and installed CLIs.

## Configuration

Settings are stored in `~/.zag/projects/<sanitized-path>/zag.toml`. Use `zag config` to view or modify. See `zag man config` for details.

Settings priority: CLI flags > config file > agent defaults.

## Examples

    zag run                                  Interactive session with default provider
    zag -p codex exec "write tests"          Non-interactive with Codex
    zag --model large run                    Use the largest model
    zag -p auto -m auto exec "refactor"      Auto-select provider and model
    zag -w run                               Run in isolated worktree
    zag --sandbox run                        Run in Docker sandbox
    zag run --continue                       Resume latest tracked interactive session
    zag run --resume abc-123                 Resume a specific session
    zag -p ollama --size 35b exec "hello"    Ollama with large model size
    zag exec --json "list 3 colors"          Get structured JSON output
    zag plan "Add auth" -o auth-plan.md      Generate an implementation plan
    zag exec --plan auth-plan.md "Implement" Execute the plan
    zag listen --latest                      Tail the latest session's logs
    zag session list                         List all tracked sessions
    zag skills list                          List available skills
    zag discover -p claude                   Show Claude's available models
    zag --help-agent                         Print AI-oriented CLI reference

## See Also

    zag man run
    zag man exec
    zag man review
    zag man plan
    zag man config
    zag man session
    zag man listen
    zag man capability
    zag man discover
    zag man skills
    zag man mcp
    zag man ps
    zag man search
    zag man input
    zag man broadcast
    zag man whoami
    zag man spawn
    zag man wait
    zag man status
    zag man collect
    zag man env
    zag man pipe
    zag man events
    zag man cancel
    zag man summary
    zag man watch
    zag man subscribe
    zag man log
    zag man output
    zag man retry
    zag man gc
    zag man serve
    zag man connect
    zag man user
    zag man orchestration
