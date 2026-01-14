# CLAUDE.md

Keep this file updated when making architectural changes to the codebase.

## Build Commands

- `make build` - Development build
- `make release` - Release build
- `make test` - Run tests
- `make fmt` - Format code
- `make clippy` - Lint

## Architecture

Rust CLI that provides a unified interface for multiple AI coding agents (Claude, Codex, Gemini, Copilot).

### Design

- **Trait-based abstraction**: Common `Agent` trait defines the interface for all agent implementations
- **Factory pattern**: `AgentFactory` creates and configures agents based on parameters
- **Model validation**: Validates model names against agent-specific allowed lists with helpful error messages
- **Subprocess delegation**: Each agent spawns its respective CLI tool, passing configuration via arguments or temporary files
- **Simple execution**: Runs agent processes and waits for completion

### Key Files

| File | Purpose |
|------|---------|
| `src/agent.rs` | Agent trait definition and ModelSize abstraction |
| `src/factory.rs` | AgentFactory for creating and configuring agents |
| `src/main.rs` | CLI entry point with clap |
| `src/config.rs` | Configuration management |
| `src/logging.rs` | Logging infrastructure and progress indicators |
| `src/claude.rs` | Claude agent implementation |
| `src/codex.rs` | Codex agent implementation |
| `src/gemini.rs` | Gemini agent implementation |
| `src/copilot.rs` | Copilot agent implementation |

## Model Size Abstraction

Instead of specifying agent-specific model names, you can use size aliases that automatically map to the appropriate model for each agent:

```bash
# Use size aliases
agent claude --model large   # Uses opus
agent codex --model large    # Uses gpt-5.1-codex-max
agent gemini --model small   # Uses gemini-2.5-flash-lite

# Or use specific model names (passthrough)
agent claude --model sonnet  # Uses sonnet directly
```

### Size Mappings

Each agent implements `model_for_size()` in its `Agent` trait implementation:

| Size | Claude | Codex | Gemini | Copilot |
|------|--------|-------|--------|---------|
| `small` / `s` | haiku | gpt-5.1-codex-mini | gemini-2.5-flash-lite | claude-haiku-4.5 |
| `medium` / `m` | sonnet | gpt-5.2-codex | gemini-2.5-flash | claude-sonnet-4.5 |
| `large` / `l` / `max` | opus | gpt-5.1-codex-max | gemini-2.5-pro | claude-opus-4.5 |

## Configuration

Configuration is stored in `.agent/agent.toml` in the project root (or `--root` directory if specified).

### Config File Location

The config file is automatically created on first run at `.agent/agent.toml`.

### Config File Format

```toml
# Agent CLI Configuration

[defaults]
# Auto-approve all actions (skip permission prompts)
# auto_approve = false

# Default model size for all agents (small, medium, large)
# Can be overridden per-agent in [models] section
model = "medium"

[models]
# Default models for each agent (overrides defaults.model)
# Use size aliases (small, medium, large) or specific model names
# claude = "opus"
# codex = "gpt-5.2-codex"
# gemini = "auto"
# copilot = "claude-sonnet-4.5"
```

### Configuration Priority

Settings are applied in this order (later overrides earlier):

1. **Agent defaults**: Built-in defaults for each agent
2. **Config file**: Settings from `.agent/agent.toml` (defaults.model, then models.<agent>)
3. **CLI flags**: Command-line arguments (highest priority)

### Available Settings

| Section | Key | Description |
|---------|-----|-------------|
| `defaults` | `auto_approve` | Skip permission prompts (default: false) |
| `defaults` | `model` | Default model size for all agents (default: "medium") |
| `models` | `claude` | Default model for Claude agent (overrides defaults.model) |
| `models` | `codex` | Default model for Codex agent (overrides defaults.model) |
| `models` | `gemini` | Default model for Gemini agent (overrides defaults.model) |
| `models` | `copilot` | Default model for Copilot agent (overrides defaults.model) |

## Logging

The CLI includes professional logging and progress indicators to provide clear feedback about operations.

### Features

- **Info messages**: Professional status messages with orange `>` prefix describing operations
- **Debug logging**: Detailed logging with `[DEBUG]` prefix for troubleshooting with the `--debug` flag
- **Progress indicators**: Animated spinner for long-running operations that clears on completion
- **Success indicators**: Green checkmark (✓) for successful operations
- **Model display**: Shows the actual model name being used

### Debug Mode

Enable debug logging with the `--debug` (or `-d`) flag:

```bash
# Enable debug logging
agent claude --debug "write a hello world program"

# Debug logging shows:
# - Configuration loading details
# - Model resolution (size aliases -> actual models)
# - System prompt configuration
# - Permission settings
# - Agent lifecycle events
```

### Example Output

```bash
# Normal mode
$ agent claude --model sonnet
⠋ Initializing Claude agent
✓ Claude initialized with model sonnet
> Starting interactive session
[Agent output...]
> Session terminated

# Debug mode
$ agent claude --model medium --debug
[DEBUG] Debug logging enabled
[DEBUG] Model specified: medium
[DEBUG] Creating agent: claude
[DEBUG] Configuration loaded
[DEBUG] Agent instance created
[DEBUG] Model resolved from CLI: medium -> sonnet
✓ Claude initialized with model sonnet
[DEBUG] Agent configuration complete
> Starting interactive session
[Agent output...]
[DEBUG] Cleaning up agent resources
> Session terminated

# With auto-approve
$ agent claude --model haiku -a
✓ Claude initialized with model haiku (auto approve)
> Starting interactive session
[Agent output...]
> Session terminated
```

## Usage

Run any supported AI coding agent with a unified interface:

```bash
# Interactive mode (default)
agent claude
agent claude "write a hello world program"

# Non-interactive mode (print output and exit)
agent claude --print "write a hello world program"
agent codex --print "write a hello world program"

# With specific model
agent claude --model opus "complex task"
agent gemini --model small "simple task"

# With custom system prompt
agent claude --system-prompt "You are a Rust expert" "help with ownership"

# With root directory
agent claude --root /path/to/project "analyze this codebase"

# Auto-approve all actions
agent claude --auto-approve "write tests"

# Enable debug logging
agent claude --debug "analyze this code"

# Combine flags
agent claude --debug --model opus --auto-approve "complex task"
```

## Model Validation

The CLI validates model names to catch typos and provide helpful error messages. If you specify an invalid model, you'll get a clear error with the available options:

```bash
$ agent claude --model gpt-5
Error: Invalid model 'gpt-5' for Claude. Available models: sonnet, opus, haiku
```

Size aliases (small, medium, large) are always valid and automatically resolve to the appropriate model for each agent.

## Supported Agents

### Claude
```bash
agent claude [OPTIONS] [PROMPT]
```

**Available models**: sonnet, opus, haiku
**Default**: opus

### Codex
```bash
agent codex [OPTIONS] [PROMPT]
```

**Available models**: gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex-mini, gpt-5.2
**Default**: gpt-5.2-codex

### Gemini
```bash
agent gemini [OPTIONS] [PROMPT]
```

**Available models**: auto, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite
**Default**: auto

### Copilot
```bash
agent copilot [OPTIONS] [PROMPT]
```

**Models**: claude-sonnet-4.5 (default), claude-opus-4.5, claude-haiku-4.5, gpt-5, gpt-5.1, gpt-5.2, gemini-3-pro-preview
