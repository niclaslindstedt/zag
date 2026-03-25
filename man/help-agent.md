# zag CLI — AI Agent Reference

`zag` is a unified CLI wrapper for AI coding agents: Claude, Codex, Gemini, Copilot, and Ollama (local). It provides a single consistent interface across all providers with unified flags, output formats, and configuration.

This document is designed to be read by an AI agent. Use it when you need to invoke `zag` as a step in a larger workflow, script, or pipeline.

## Core Commands

```
zag exec "<prompt>"            Non-interactive: send prompt, get output, exit
zag run ["<prompt>"]           Interactive: start a full terminal session
zag run --continue             Resume the latest tracked session
zag run --resume <id>          Resume a specific session
zag review --uncommitted       Code review (uses Codex)
zag config [key] [value]       View or set configuration
zag session list               List all tracked sessions
zag listen <id>                Tail a session's log events in real-time
zag capability [-p provider]   Show provider capability declarations
zag skills list                List provider-agnostic skills
zag man [command]              Show detailed docs for a command
```

**For scripting and programmatic use, always use `exec`.** It runs non-interactively, prints output, and exits. `run` is for interactive terminal sessions only.

## Provider Selection

```
zag exec "..."                          Default provider (Claude)
zag -p claude exec "..."                Explicit Claude
zag -p codex exec "..."                Codex
zag -p gemini exec "..."               Gemini
zag -p copilot exec "..."              Copilot
zag -p ollama exec "..."               Local Ollama model
zag -p auto exec "..."                 Auto-select best provider
```

## Model Selection

Use size aliases (portable across providers) or specific model names:

```
zag --model small exec "..."           Fast/cheap: haiku / gpt-5.4-mini / gemini-2.5-flash-lite
zag --model medium exec "..."          Balanced: sonnet / gpt-5.3-codex / gemini-2.5-flash
zag --model large exec "..."           Most capable: opus / gpt-5.4 / gemini-2.5-pro
zag --model auto exec "..."            Auto-select best model for the task
zag -p auto -m auto exec "..."         Auto-select both provider and model
```

Specific model names are also accepted and passed through directly.

## Output Formats (exec)

By default, `exec` streams agent output as clean text — no spinners, no wrappers. Suitable for piping.

```
zag exec "..."                          Default: formatted text (tool indicators, clean output)
zag exec -o text "..."                 Raw text pass-through (unprocessed agent stdout)
zag exec -o json "..."                 Full session as compact JSON (AgentOutput envelope)
zag exec -o json-pretty "..."          Full session as pretty JSON
zag exec -o stream-json "..."          Streaming NDJSON — one Event per line, real-time
zag exec -o native-json "..."          Claude's raw JSON (Claude only)
```

The `-o json` format outputs the full `AgentOutput` envelope: session ID, events, tool calls, usage stats, and final result. Use it when you need metadata about the session.

## Structured JSON Output Mode

Use `--json` when you want the agent to *respond with JSON data* (not wrap the session in JSON):

```
zag exec --json "list 3 colors"
# Output: ["red","green","blue"]

zag exec --json-schema '{"type":"array","items":{"type":"string"}}' "list 3 colors"
# Output: validated JSON; retries up to 3x if schema fails

zag exec --json-schema schema.json "extract user data from the codebase"
# Schema from file

zag exec --json-stream "analyze this code"
# Stream JSON events (NDJSON, one event per line)
```

Key distinction:
- `-o json` → wraps entire session in `AgentOutput` JSON envelope (includes events, usage, etc.)
- `--json` → instructs agent to respond with JSON; outputs only the agent's response

`--json-schema` implies `--json`. On validation failure, retries automatically via session resume.

## Scripting and Integration Patterns

### Clean output for piping
```sh
# Suppress all wrapper output — only agent text is printed
zag -q exec "write a summary of this file" > summary.txt

# Pipe directly
zag exec "list all function names in this repo" | grep "auth"
```

### Capture structured data
```sh
# Get JSON response
result=$(zag exec --json "extract config keys from this codebase")
echo "$result" | jq '.keys[]'

# With schema validation
data=$(zag exec --json-schema schema.json "analyze dependencies")
```

### Multi-step pipelines
```sh
# Step 1: generate code
zag exec "write a Go HTTP handler for /health" > handler.go

# Step 2: review it
zag review --uncommitted

# Step 3: run tests via another agent step
zag exec "write unit tests for handler.go"
```

### Embed agent help in your own prompt
```sh
# Include this reference so the agent knows how to use the CLI
zag exec "Help me set up a pipeline. $(zag --help-agent)"
```

### Use in CI/scripts
```sh
#!/bin/bash
# Run analysis and capture JSON
output=$(zag -q exec -o json "analyze this PR for security issues")
severity=$(echo "$output" | jq -r '.final_result')
```

## Auto-Approve and Permissions

```sh
zag -a exec "..."                      Skip all permission prompts (auto-approve)
zag --auto-approve exec "..."          Same
```

Use `-a` in non-interactive scripts where you can't respond to prompts. Not needed with `--sandbox` (sandbox provides isolation automatically).

## Isolation: Worktrees and Sandboxes

```sh
# Git worktree isolation (keeps changes separate from main working tree)
zag -w exec "implement feature X"
zag -w my-feature run

# Docker sandbox (stronger isolation — microVM)
zag --sandbox exec "risky refactor"
zag --sandbox my-sb run

# Resume an isolated session later
zag run --resume <session-id>
```

`--worktree` and `--sandbox` are mutually exclusive. Both create resumable sessions tracked in `~/.zag/projects/<sanitized-path>/sessions.json`.

## Ollama (Local Models)

```sh
zag -p ollama exec "explain this function"         Default: qwen3.5:9b
zag -p ollama --size 35b exec "complex task"       Larger size
zag -p ollama --model llama3 run                   Different model
zag -p ollama --model small exec "quick task"      Size alias → 2b
```

## Additional Context Directories

```sh
zag --add-dir ../other-repo exec "compare implementations"
zag --add-dir /path/to/docs --add-dir /path/to/specs exec "analyze"
```

## System Prompt Override

```sh
zag --system-prompt "You are a Rust expert" exec "help with lifetimes"
```

## Session Management

```sh
zag session list                       List all tracked sessions
zag session list -p claude             Filter by provider
zag session show <id>                  Show session details
zag listen --latest                    Tail the latest session's logs
zag listen --active --rich-text        Tail the active session with colors
```

## Configuration

Settings live in `~/.zag/projects/<sanitized-path>/zag.toml` (or `~/.zag/zag.toml` globally).

```sh
zag config                             Print current config
zag config provider gemini             Set default provider
zag config model large                 Set default model size
zag config model.claude opus           Set Claude-specific model
zag config auto_approve true           Enable auto-approve by default
```

Config priority: CLI flags > `models.<agent>` > `defaults.model` > agent built-in defaults.

## Key Flags Summary

```
-p, --provider <NAME>        Provider: claude, codex, gemini, copilot, ollama, auto
-m, --model <NAME>           Model name, size alias, or auto
-s, --system-prompt <TEXT>   Custom system prompt
-r, --root <PATH>            Root directory for the agent
-a, --auto-approve           Skip permission prompts
    --add-dir <PATH>         Add extra directory (repeatable)
-w, --worktree [NAME]        Run in isolated git worktree
    --sandbox [NAME]         Run in Docker sandbox
    --size <SIZE>            Ollama parameter size (2b, 9b, 35b, etc.)
-d, --debug                  Debug logging
-q, --quiet                  Suppress all logging (clean output for scripts)
-v, --verbose                Styled output with icons (exec mode)
    --json                   Request JSON response from agent
    --json-schema <SCHEMA>   Validate JSON against schema (file or inline)
    --json-stream            Stream NDJSON events
    --session <UUID>         Pre-set session ID (for zag listen)
-o, --output <FORMAT>        exec output format: text, json, json-pretty, stream-json
```

## Detailed Documentation

For deeper documentation on any command, run:

```
zag man              General overview
zag man exec         Non-interactive execution, output formats
zag man run          Interactive sessions, worktrees, sandboxes
zag man review       Code review
zag man config       Configuration reference
zag man session      Session management
zag man listen       Real-time session log tailing
zag man capability   Provider capability declarations
zag man skills       Provider-agnostic skill management
```
