# zag

A unified CLI for AI coding agents.

## Synopsis

    zag [flags] <command> [options]

## Description

`zag` provides a single interface for running multiple AI coding agents — Claude, Codex, Gemini, Copilot, and Ollama. Instead of learning five different CLIs with different flags and model names, you use one consistent command with unified options.

The CLI handles model resolution (size aliases like `small`/`medium`/`large`), configuration management, worktree isolation, Docker sandboxing, structured JSON output with schema validation, unified session logs, provider-agnostic skills, and automatic provider/model selection.

## Global Flags

These flags can be used with any subcommand.

    -p, --provider <NAME>       Provider: claude, codex, gemini, copilot, ollama, auto
    -m, --model <NAME>          Model name, size alias (small/medium/large), or auto
    -s, --system-prompt <TEXT>   Custom system prompt appended to the agent's default
    -r, --root <PATH>           Root directory to run the agent in
    -a, --auto-approve          Skip permission prompts (auto-approve all actions)
        --add-dir <PATH>        Additional directories to include (repeatable)
        --env <KEY=VALUE>       Environment variable for the agent subprocess (repeatable)
    -w, --worktree [NAME]       Run in an isolated git worktree (optional name)
        --sandbox [NAME]        Run inside a Docker sandbox (optional name)
        --size <SIZE>           Model parameter size for Ollama (e.g., 2b, 9b, 35b)
    -d, --debug                 Enable debug logging
    -q, --quiet                 Suppress all logging except agent output
    -v, --verbose               Show styled output with icons and status messages
        --show-usage            Show token usage statistics (JSON output mode only)
        --json                  Request structured JSON output from the agent
        --json-schema <SCHEMA>  Validate JSON output against a schema (file or inline)
        --json-stream           Stream JSON events in NDJSON format
        --session <UUID>        Use a specific session ID instead of auto-generating one
        --help-agent              Print AI-oriented reference for using this CLI

## Commands

    run          Start an interactive session
    exec         Run non-interactively (print output and exit)
    review       Review code changes (uses Codex)
    config       View or set configuration values
    session      List and inspect sessions, import historical logs
    listen       Tail a session's log events in real-time
    capability   Show provider capability declarations
    skills       Manage provider-agnostic skills
    mcp          Manage MCP servers across providers
    ps           List, inspect, and manage agent processes
    search       Search through session logs
    input        Send a user message to a running or resumable session
    broadcast    Send a message to all sessions in the project
    whoami       Show identity of the current zag session
    spawn        Launch a background agent session (or --interactive for long-lived)
    wait         Block until session(s) complete
    status       Machine-readable session health check
    collect      Gather results from multiple sessions
    env          Export session environment variables
    pipe         Chain results from completed sessions into a new session
    events       Query structured events from session logs
    cancel       Gracefully cancel running sessions
    summary      Show log-based session summaries and stats
    watch        Watch session logs and execute commands on matching events
    subscribe    Subscribe to a multiplexed event stream from all sessions
    log          Append custom structured events to a session log
    output       Extract final result text from a session
    retry        Re-run failed sessions with the same configuration
    gc           Clean up old session data, logs, and process entries
    serve        Start the zag HTTPS/WebSocket server for remote access
    connect      Connect to a remote zag server
    disconnect   Disconnect from a remote zag server
    man          Show manual pages for commands

Run `zag man <command>` for detailed help on each command.

## Providers

    claude    Default. Models: default, haiku, sonnet, opus, sonnet-4.6, opus-4.6, haiku-4.5 (default: default)
    codex     Models: gpt-5.4, gpt-5.4-mini, gpt-5.3-codex-spark, gpt-5.3-codex, gpt-5-codex, gpt-5.2-codex, gpt-5.2, o4-mini, gpt-5.1-codex-max, gpt-5.1-codex-mini
    gemini    Models: gemini-3.1-pro-preview, gemini-3.1-flash-lite-preview, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-flash-lite, gemini-2.5-flash, gemini-2.5-pro, auto (default: auto)
    copilot   Models: claude-haiku-4.5, claude-sonnet-4.6, claude-opus-4.6, and more
    ollama    Local models via Ollama. Default: qwen3.5:9b. Use --size for parameter size

## Model Size Aliases

Size aliases resolve to the appropriate model for the active provider:

    small  (s)     Lightweight, fast — haiku / gpt-5.4-mini / gemini-3.1-flash-lite-preview
    medium (m)     Balanced — sonnet / gpt-5.3-codex / gemini-2.5-flash
    large  (l/max) Most capable — default / gpt-5.4 / gemini-3.1-pro-preview

## Configuration

Settings are stored in `~/.zag/projects/<sanitized-path>/zag.toml`. Use `zag config` to view or modify. See `zag man config` for details.

Settings priority: CLI flags > config file > agent defaults.

## Examples

    zag run                                 Interactive session with default provider
    zag -p codex exec "write tests"         Non-interactive with Codex
    zag --model large run                   Use the largest model
    zag -p auto -m auto exec "refactor"     Auto-select provider and model
    zag -w run                              Run in isolated worktree
    zag --sandbox run                       Run in Docker sandbox
    zag run --continue                      Resume latest tracked interactive session
    zag run --resume abc-123                Resume a specific session
    zag -p ollama --size 35b exec "hello"   Ollama with large model size
    zag exec --json "list 3 colors"         Get structured JSON output
    zag listen --latest                     Tail the latest session's logs
    zag session list                        List all tracked sessions
    zag skills list                         List available skills
    zag --help-agent                          Print AI-oriented CLI reference

## See Also

    zag man run
    zag man exec
    zag man review
    zag man config
    zag man session
    zag man listen
    zag man capability
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
    zag man orchestration
