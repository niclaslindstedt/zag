# agent

A unified CLI for AI coding agents.

## Synopsis

    agent [flags] <command> [options]

## Description

`agent` provides a single interface for running multiple AI coding agents — Claude, Codex, Gemini, and Copilot. Instead of learning four different CLIs with different flags and model names, you use one consistent command with unified options.

The CLI handles model resolution (size aliases like `small`/`medium`/`large`), configuration management, worktree isolation, structured JSON output with schema validation, and automatic provider/model selection.

## Global Flags

These flags can be used with any subcommand.

    -p, --provider <NAME>       Provider: claude, codex, gemini, copilot, ollama, auto
    -m, --model <NAME>          Model name, size alias (small/medium/large), or auto
    -s, --system-prompt <TEXT>   Custom system prompt appended to the agent's default
    -r, --root <PATH>           Root directory to run the agent in
    -a, --auto-approve          Skip permission prompts (auto-approve all actions)
        --add-dir <PATH>        Additional directories to include (repeatable)
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
        --help-agent            Print AI-oriented reference for using this CLI

## Commands

    run       Start an interactive session
    exec      Run non-interactively (print output and exit)
    resume    Resume a previous session
    review    Review code changes (uses Codex)
    config    View or set configuration values
    man       Show manual pages for commands

Run `agent man <command>` for detailed help on each command.

## Providers

    claude    Default. Models: haiku, sonnet, opus (default: opus)
    codex     Models: gpt-5.4, gpt-5.4-mini, gpt-5.3-codex, gpt-5.2-codex, gpt-5.2, gpt-5.1-codex-max, gpt-5.1-codex-mini
    gemini    Models: gemini-2.5-flash-lite, gemini-2.5-flash, gemini-2.5-pro, auto
    copilot   Models: claude-haiku-4.5, claude-sonnet-4.5, claude-opus-4.5, and more
    ollama    Local models via Ollama. Default: qwen3.5:9b. Use --size for parameter size

## Model Size Aliases

Size aliases resolve to the appropriate model for the active provider:

    small  (s)     Lightweight, fast — haiku / gpt-5.4-mini / gemini-2.5-flash-lite
    medium (m)     Balanced — sonnet / gpt-5.3-codex / gemini-2.5-flash
    large  (l/max) Most capable — opus / gpt-5.4 / gemini-2.5-pro

## Configuration

Settings are stored in `~/.agent/projects/<sanitized-path>/agent.toml`. Use `agent config` to view or modify. See `agent man config` for details.

Settings priority: CLI flags > config file > agent defaults.

## Examples

    agent run                                 Interactive session with default provider
    agent -p codex exec "write tests"         Non-interactive with Codex
    agent --model large run                   Use the largest model
    agent -p auto -m auto exec "refactor"     Auto-select provider and model
    agent -w run                              Run in isolated worktree
    agent --sandbox run                       Run in Docker sandbox
    agent -p ollama --size 35b exec "hello"   Ollama with large model size
    agent exec --json "list 3 colors"         Get structured JSON output
    agent --help-agent                        Print AI-oriented CLI reference

## See Also

    agent man run
    agent man exec
    agent man resume
    agent man review
    agent man config
