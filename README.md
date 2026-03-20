# agent

A unified CLI for AI coding agents. Run Claude, Codex, Gemini, or Copilot through a single interface with consistent flags, model aliases, and output formats.

## Why

Each AI coding agent has its own CLI with different flags, model names, and output formats. `agent` wraps them all behind one interface so you don't need to remember four different CLIs. It also adds features that work across all providers: size-based model aliases, auto provider/model selection, worktree isolation, and structured JSON output with schema validation.

## Install

```bash
cargo install --path .
```

Requires the underlying agent CLIs to be installed separately (`claude`, `codex`, `gemini`, `copilot`).

## Quick Start

```bash
# Interactive session (default: Claude)
agent run

# Non-interactive
agent exec "write a hello world program"

# Use a different provider
agent -p codex run
agent -p gemini exec "analyze this code"

# Use model size aliases instead of provider-specific names
agent --model small exec "quick question"
agent --model large run "complex refactor"

# Auto-select provider and model based on task
agent -p auto -m auto exec "refactor the auth system"

# Code review (uses Codex)
agent review --uncommitted

# Configuration
agent config provider claude
agent config model.codex=gpt-5.1-codex-max
```

## Supported Agents

| Provider | CLI | Default Model | Models |
|----------|-----|---------------|--------|
| `claude` | `claude` | opus | sonnet, opus, haiku |
| `codex` | `codex` | gpt-5.2-codex | gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex-mini, gpt-5.2 |
| `gemini` | `gemini` | auto | auto, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite |
| `copilot` | `copilot` | claude-sonnet-4.5 | claude-sonnet-4.5, claude-opus-4.5, claude-haiku-4.5, gpt-5, gpt-5.1, gpt-5.2, gemini-3-pro-preview |
| `ollama` | `ollama` | qwen3.5:9b | Any model from ollama.com (use `--size` for parameter size) |

### Model Size Aliases

Instead of remembering provider-specific model names, use size aliases:

| Size | Claude | Codex | Gemini | Copilot |
|------|--------|-------|--------|---------|
| `small` / `s` | haiku | gpt-5.1-codex-mini | gemini-2.5-flash-lite | claude-haiku-4.5 |
| `medium` / `m` | sonnet | gpt-5.2-codex | gemini-2.5-flash | claude-sonnet-4.5 |
| `large` / `l` / `max` | opus | gpt-5.1-codex-max | gemini-2.5-pro | claude-opus-4.5 |

For Ollama, size aliases map to parameter sizes (not model names): `small`=2b, `medium`=9b, `large`=35b. These are configurable via `ollama.size_small`, `ollama.size_medium`, `ollama.size_large`.

## Commands

| Command | Description |
|---------|-------------|
| `run [prompt]` | Interactive session (optional initial prompt) |
| `exec <prompt>` | Non-interactive — print output and exit |
| `resume [id]` | Resume a previous session |
| `review` | Code review (uses Codex) |
| `config [key] [value]` | View or set configuration |
| `man [command]` | Show manual pages (`agent man exec`, etc.) |

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--provider <name>` | `-p` | Provider: claude, codex, gemini, copilot, auto |
| `--model <name>` | `-m` | Model name or size alias (small/medium/large/auto) |
| `--system-prompt` | `-s` | Custom system prompt |
| `--root <path>` | `-r` | Root directory for the agent |
| `--auto-approve` | `-a` | Skip permission prompts |
| `--add-dir <path>` | | Additional directories to include |
| `--worktree [name]` | `-w` | Run in an isolated git worktree |
| `--sandbox [name]` | | Run inside a Docker sandbox |
| `--size <size>` | | Model parameter size for Ollama (e.g., 2b, 9b, 35b) |
| `--debug` | `-d` | Enable debug logging |
| `--quiet` | `-q` | Suppress all logging except agent output |
| `--verbose` | `-v` | Show styled output with icons in exec mode |
| `--json` | | Request JSON output |
| `--json-schema <schema>` | | Validate JSON output against a schema |
| `--json-stream` | | Stream JSON events (NDJSON) |

## Configuration

Stored in `~/.agent/projects/<sanitized-path>/agent.toml` (or `~/.agent/agent.toml` globally when not in a git repo).

```toml
[defaults]
provider = "claude"
model = "medium"
auto_approve = false

[models]
claude = "opus"
codex = "gpt-5.2-codex"

[auto]
provider = "claude"
model = "sonnet"
```

Settings priority: CLI flags > config file > agent defaults.

## Features

### Auto Provider/Model Selection

Use `-p auto` and/or `-m auto` to let a lightweight LLM analyze your prompt and pick the best provider/model:

```bash
agent exec -p auto -m auto "complex multi-file refactor"
```

### Worktree Mode

Isolate sessions in a git worktree so changes don't affect your working tree:

```bash
agent -w run                    # Auto-named worktree
agent -w my-feature exec "..."  # Named worktree
```

After interactive sessions, you're prompted to keep or remove the worktree. `agent resume <id>` automatically restores the correct worktree.

### Sandbox Mode

Run agents inside Docker sandbox microVMs for stronger isolation with bidirectional file sync, network policies, and credential injection:

```bash
agent --sandbox run                    # Auto-named sandbox
agent --sandbox my-name exec "..."     # Named sandbox
agent -p codex --sandbox run           # Works with any provider
```

After interactive sessions, you're prompted to keep or remove the sandbox. `agent resume <id>` resumes inside the same sandbox. `--sandbox` and `--worktree` are mutually exclusive.

### JSON Output

```bash
agent exec --json "list 3 colors"
agent exec --json-schema '{"type":"object","required":["colors"]}' "list 3 colors"
agent exec --json-stream "complex task"
```

On validation failure, the agent retries up to 3 times via session resume.

### Output Formats

With `exec -o <format>`:

- **default** — Streamed beautiful text (Claude) or plain text (others)
- **text** — Raw text, no JSON parsing
- **json** — Compact unified JSON
- **json-pretty** — Pretty-printed unified JSON
- **stream-json** — NDJSON event stream
- **native-json** — Claude's raw JSON (Claude only)

## Architecture

```
CLI (clap) → AgentFactory → Agent trait impl → subprocess (claude/codex/gemini/copilot)
```

- **`Agent` trait** (`src/agent.rs`): Common interface for all providers — run, resume, cleanup, model resolution
- **`AgentFactory`** (`src/factory.rs`): Creates agents, resolves model aliases, validates models
- **Provider implementations** (`src/claude/`, `src/codex.rs`, `src/gemini.rs`, `src/copilot.rs`): Each spawns its respective CLI tool
- **`AgentOutput`** (`src/output.rs`): Unified output format across all providers
- **`Config`** (`src/config.rs`): TOML config management with git-root detection
- **Auto-selector** (`src/auto_selector.rs`): LLM-based provider/model routing

## Development

```bash
make build          # Dev build
make test           # Run tests
make clippy         # Lint
make fmt            # Format
make release        # Release build
```
