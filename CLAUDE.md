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
| `src/config.rs` | Configuration management with get/set support |
| `src/logging.rs` | Logging infrastructure and progress indicators |
| `src/claude.rs` | Claude agent implementation |
| `src/codex.rs` | Codex agent implementation |
| `src/gemini.rs` | Gemini agent implementation |
| `src/copilot.rs` | Copilot agent implementation |
| `src/process.rs` | Subprocess helpers for stderr capture |
| `src/auto_selector.rs` | Auto provider/model selection via lightweight LLM call |
| `src/json_validation.rs` | JSON and JSON Schema validation utilities |
| `prompts/auto-selector-2_0.md` | Versioned prompt template for auto-selection (JSON response format) |

## Model Size Abstraction

Instead of specifying agent-specific model names, you can use size aliases that automatically map to the appropriate model for each agent:

```bash
# Use size aliases
agent --model large run              # Uses opus (default provider: claude)
agent -p codex --model large run     # Uses gpt-5.1-codex-max
agent -p gemini --model small run    # Uses gemini-2.5-flash-lite

# Or use specific model names (passthrough)
agent --model sonnet run             # Uses sonnet directly
```

### Size Mappings

Each agent implements `model_for_size()` in its `Agent` trait implementation:

| Size | Claude | Codex | Gemini | Copilot |
|------|--------|-------|--------|---------|
| `small` / `s` | haiku | gpt-5.1-codex-mini | gemini-2.5-flash-lite | claude-haiku-4.5 |
| `medium` / `m` | sonnet | gpt-5.2-codex | gemini-2.5-flash | claude-sonnet-4.5 |
| `large` / `l` / `max` | opus | gpt-5.1-codex-max | gemini-2.5-pro | claude-opus-4.5 |

## Auto Provider/Model Selection

Use `-p auto` and/or `-m auto` to let a lightweight LLM call analyze your prompt and select the best provider/model:

```bash
# Auto-select provider (model uses provider's default)
agent exec -p auto "say hello"

# Auto-select model (uses configured/default provider)
agent exec -m auto "refactor the auth system"

# Auto-select both
agent exec -p auto -m auto "complex multi-file refactor"
```

### How it works

1. The CLI runs a quick non-interactive LLM call (default: Claude haiku) with the user's prompt
2. The selector LLM analyzes task complexity and chooses the best provider/model
3. The resolved values replace `"auto"` and execution continues normally

### Configuration

The selector LLM is configurable in `.agent/agent.toml`:

```toml
[auto]
# Provider used for auto-selection (default: "claude")
# provider = "claude"
# Model used for auto-selection (default: "haiku")
# model = "haiku"
```

Config keys: `auto.provider`, `auto.model`

### Restrictions

- Requires a prompt to analyze (errors if used with `run` without a prompt)
- Cannot be used with `resume`, `review`, or `config` subcommands

## Configuration

Configuration is stored in `.agent/agent.toml`, with smart location detection:

### Config File Location

The config location is automatically determined using this priority:

1. **Explicit `--root` flag**: If provided, uses `<root>/.agent/agent.toml`
2. **Git repository root**: If current directory is in a git repo, uses `<repo-root>/.agent/agent.toml`
3. **Global config**: If not in a repo, uses `~/.config/agent/.agent/agent.toml` (Linux/macOS) or `~/AppData/Roaming/agent/.agent/agent.toml` (Windows)

This means:
- Each git repository has its own config
- No scattered `.agent` folders in subdirectories
- Global fallback for non-repository usage
- `.gitignore` entry is automatically added for repository configs

### Config File Format

```toml
# Agent CLI Configuration

[defaults]
# Default provider (claude, codex, gemini, copilot)
# provider = "claude"

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

[auto]
# Settings for auto provider/model selection (-p auto / -m auto)
# provider = "claude"
# model = "haiku"
```

### Config Subcommand

View or set configuration values with `agent config`:

```bash
# Print full config file
agent config

# Set values (space or = syntax)
agent config provider claude
agent config provider=claude
agent config model opus
agent config model.claude=opus
agent config auto_approve true
```

### Configuration Priority

Settings are applied in this order (later overrides earlier):

1. **Agent defaults**: Built-in defaults for each agent
2. **Config file**: Settings from `.agent/agent.toml` (defaults.model, then models.<agent>)
3. **CLI flags**: Command-line arguments (highest priority)

### Available Settings

| Key | Description |
|-----|-------------|
| `provider` | Default provider (default: "claude") |
| `auto_approve` | Skip permission prompts (default: false) |
| `model` | Default model size for all agents (default: "medium") |
| `model.claude` | Default model for Claude agent (overrides model) |
| `model.codex` | Default model for Codex agent (overrides model) |
| `model.gemini` | Default model for Gemini agent (overrides model) |
| `model.copilot` | Default model for Copilot agent (overrides model) |
| `auto.provider` | Provider for auto-selection LLM call (default: "claude") |
| `auto.model` | Model for auto-selection LLM call (default: "haiku") |

## Logging

The CLI includes professional logging and progress indicators to provide clear feedback about operations.

### Features

- **Info messages**: Professional status messages with orange `>` prefix describing operations
- **Debug logging**: Detailed logging with `[DEBUG]` prefix for troubleshooting with the `--debug` flag
- **Progress indicators**: Animated spinner for long-running operations that clears on completion
- **Success indicators**: Green checkmark (✓) for successful operations
- **Model display**: Shows the actual model name being used
- **File-based logging**: All log messages are written to session log files in `~/.config/agent/.agent/logs/` (always at debug level)
- **Stderr capture**: In non-interactive (exec) mode, agent subprocess stderr is captured and logged to file. On failure, stderr is included in the error message. Interactive sessions pass stderr through unchanged.

### Debug Mode

Enable debug logging with the `--debug` (or `-d`) flag:

```bash
# Enable debug logging
agent --debug run "write a hello world program"

# Debug logging shows:
# - Configuration loading details
# - Model resolution (size aliases -> actual models)
# - System prompt configuration
# - Permission settings
# - Agent lifecycle events
```

### Exec Output Behavior

In `exec` mode, only the raw agent text output is shown by default — no spinners, status messages, icons, or colored formatting. This makes exec output clean for scripting and piping.

Use `--verbose` (or `-v`) to opt into the full styled output with icons, tool execution details, and status messages.

### Verbose Mode

Enable detailed formatted output with the `--verbose` (or `-v`) flag. In `exec` mode, this restores the styled output with icons (⏺, ←, ✓), colors, tool execution details, and wrapper status messages:

```bash
# Verbose exec - shows styled output with icons and status
agent exec -v "write a hello world program"

# Also works with interactive mode (no behavioral change since run always shows full output)
agent -v run
```

### Quiet Mode

Disable all logging except agent output with the `--quiet` (or `-q`) flag. This applies to all modes including `run`:

```bash
# Quiet mode - only shows agent output
agent -q exec "write a hello world program"

# Useful for scripting
result=$(agent -q exec "analyze this code")

# Quiet mode suppresses:
# - Spinner animations
# - Initialization messages (✓ Agent initialized...)
# - Session start/end messages
# - Debug logs
# - Info messages
# - Tool execution status
# - Cost and usage statistics
```

### Example Output

```bash
# Exec mode (default: clean output)
$ agent exec "say hello"
Hello!

# Exec mode with verbose
$ agent exec -v "say hello"
✓ Claude initialized with model opus
    ⏺ Hello!

# Interactive mode
$ agent --model sonnet run
⠋ Initializing Claude agent
✓ Claude initialized with model sonnet
> Starting interactive session
[Agent output...]
> Session terminated

# Debug mode
$ agent --model medium --debug run
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
$ agent --model haiku -a run
✓ Claude initialized with model haiku (auto approve)
> Starting interactive session
[Agent output...]
> Session terminated

# Quiet mode (only agent output, no logging)
$ agent --model sonnet -q exec "write a hello world program"
[Agent output only...]
```

## Usage

The CLI uses a subcommand structure: `agent [flags] <action> [options]`.

The provider is specified via the `--provider` (or `-p`) flag. If omitted, it defaults to the configured provider (fallback: claude).

### Actions

- **`run`** - Start an interactive session
- **`exec`** - Run non-interactively (print output and exit)
- **`resume`** - Resume a previous session
- **`review`** - Review code changes (uses Codex)
- **`config`** - View or set configuration values

```bash
# Interactive mode (uses default provider, typically claude)
agent run
agent run "write a hello world program"

# With explicit provider
agent -p codex run
agent -p gemini -m large run

# Non-interactive mode (exec)
agent exec "write a hello world program"
agent exec "analyze this code" -o json

# Non-interactive mode with streaming JSON events (NDJSON format)
agent exec "complex task" -o stream-json

# Non-interactive mode with compact JSON output
agent exec "write a hello world program" -o json
agent -p gemini exec "analyze this code" --output json

# Non-interactive mode with pretty-printed JSON output
agent exec "write a hello world program" -o json-pretty

# Non-interactive mode with plain text output (no JSON parsing)
agent exec "simple task" -o text

# Non-interactive mode with native JSON output (Claude's raw JSON format)
agent exec "write a hello world program" -o native-json

# Non-interactive mode with stream-json input format (Claude only)
echo '{"type":"message","content":"hello"}' | agent exec -i stream-json "analyze"
cat input.ndjson | agent exec --input-format stream-json "process"

# Resume a session
agent resume                    # Resume most recent / show picker
agent resume <session-id>       # Resume specific session
agent resume --last             # Resume most recent session

# With specific model
agent --model opus exec "complex task"
agent -p gemini --model small exec "simple task"

# With custom system prompt
agent --system-prompt "You are a Rust expert" exec "help with ownership"

# With root directory
agent --root /path/to/project run

# Auto-approve all actions
agent --auto-approve exec "write tests"

# Additional directories
agent --add-dir ../other-repo run
agent -p gemini --add-dir /path/to/docs --add-dir /path/to/specs exec "analyze"

# Enable debug logging
agent --debug exec "analyze this code"

# Enable quiet mode (suppress all logging)
agent -q exec "write tests"

# Enable verbose mode (show styled output with icons in exec)
agent -v exec "write tests"

# Worktree mode (isolated git worktree per session)
agent -w run                          # Auto-generated worktree name
agent --worktree run                  # Same as above
agent -w my-feature run               # Named worktree
agent -p codex -w run                 # Works with any provider

# JSON output mode
agent exec --json "list 3 colors"                                        # Request JSON output
agent exec --json-schema '{"type":"object"}' "list 3 colors"             # With schema validation
agent exec --json-schema schema.json "list 3 colors"                     # Schema from file

# Combine flags
agent --debug --model opus -a exec "complex task"
agent -q exec "simple task" -o json
agent -v exec "complex task"          # Verbose exec with icons

# Configuration
agent config                       # Print full config
agent config provider gemini       # Set default provider
agent config model.claude=opus     # Set claude-specific model
```

### Review Command

Top-level `agent review` command for code review (uses Codex under the hood):

```bash
# Review uncommitted changes
agent review --uncommitted

# Review against a base branch
agent review --base main

# Review a specific commit
agent review --commit abc123

# With optional title
agent review --uncommitted --title "Feature review"

# With shared flags
agent review --uncommitted --model large --auto-approve
```

### Input Formats (Claude Only)

When using `exec` with Claude, you can specify the input format with the `-i` or `--input-format` flag:

- **text** (default): Plain text input from stdin
- **stream-json**: Streaming JSON input (NDJSON format) for realtime structured input

**Note:** The `--input-format` flag only works with Claude's `exec` subcommand.

### Output Formats

When using `exec`, you can specify the output format with the `-o` or `--output` flag:

- **Default (no `-o` flag)**: Streams events and formats them as beautiful text in real-time (Claude only). Other agents use text output.
- **text**: Plain text output - bypasses JSON parsing and streams raw agent output
- **json**: Compact JSON output (single-line) - captures the full session then outputs unified AgentOutput format
- **json-pretty**: Pretty-printed JSON output - captures the full session then outputs unified AgentOutput format
- **stream-json**: Streaming JSON output in NDJSON format - each line is a unified Event as JSON
- **native-json**: Claude's raw JSON output without conversion to unified format (Claude only)

## JSON Output Mode

Use `--json` to request structured JSON output from the agent. Use `--json-schema` to additionally validate the output against a JSON schema.

```bash
# Request JSON output
agent exec --json "list 3 colors"

# Validate against inline schema
agent exec --json-schema '{"type":"object","properties":{"colors":{"type":"array"}}}' "list 3 colors"

# Validate against schema file
agent exec --json-schema schema.json "list 3 colors"

# Also works with run (when a prompt is provided)
agent run --json "list 3 colors"
```

### Behavior

- `--json-schema` implies `--json`
- Cannot be used with `resume`, `review`, or `config`
- Requires a prompt (doesn't work with interactive `run` without a prompt)
- **Claude**: Uses native `--json-schema` support when a schema is provided
- **Other agents**: Augments the system prompt with JSON instructions and schema
- **Validation**: Output is validated as JSON (and against schema if provided)
- **Retry**: On validation failure, retries up to 3 times via session resume with a correction prompt
- **Output**: The final output is the raw JSON from the agent (not wrapped in AgentOutput)

## Model Validation

The CLI validates model names to catch typos and provide helpful error messages. If you specify an invalid model, you'll get a clear error with the available options:

```bash
$ agent --model gpt-5 run
Error: Invalid model 'gpt-5' for Claude. Available models: sonnet, opus, haiku
```

Size aliases (small, medium, large) are always valid and automatically resolve to the appropriate model for each agent.

## Supported Agents

### Claude (default)
```bash
agent [-p claude] <run|exec|resume> [OPTIONS]
```

**Available models**: sonnet, opus, haiku
**Default**: opus

### Codex
```bash
agent -p codex <run|exec|resume> [OPTIONS]
```

**Available models**: gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex-mini, gpt-5.2
**Default**: gpt-5.2-codex

### Gemini
```bash
agent -p gemini <run|exec|resume> [OPTIONS]
```

**Available models**: auto, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite
**Default**: auto

### Copilot
```bash
agent -p copilot <run|exec|resume> [OPTIONS]
```

**Models**: claude-sonnet-4.5 (default), claude-opus-4.5, claude-haiku-4.5, gpt-5, gpt-5.1, gpt-5.2, gemini-3-pro-preview

### Review
```bash
agent review [--uncommitted] [--base <BRANCH>] [--commit <SHA>] [--title <TITLE>] [OPTIONS]
```

Uses Codex under the hood for code review.

## Worktree Mode

The `--worktree` (or `-w`) flag creates an isolated git worktree for the session, keeping changes separate from the main working tree.

```bash
# Auto-generated worktree name
agent -w run

# Named worktree
agent -w my-feature exec "implement feature X"

# Works with any provider
agent -p codex -w run
agent -p gemini -w my-task exec "analyze code"
```

### Behavior per Provider

| Provider | Behavior |
|----------|----------|
| Claude | Flag is passed through to `claude` binary (native `--worktree` support) |
| Codex, Gemini, Copilot | Wrapper creates worktree via `git worktree add --detach`, sets agent root to worktree path |

### Worktree Location

- **Claude**: Managed by the `claude` binary (typically `.claude/worktrees/`)
- **Other providers**: Created at `<repo>/.git/agent-worktrees/<name>/`

### Restrictions

- Cannot be used with `resume`, `review`, or `config` subcommands
- Requires a git repository (errors if not in one)
- No automatic cleanup (matches Claude's behavior)

## How to Implement New Features

Pattern for adding new CLI features:

1. **Add CLI flag** to `Cli` struct in `src/main.rs` (use `global = true` for cross-subcommand flags)
2. **If cross-cutting**: Handle in `run_agent_action()` before agent creation (e.g., worktree creation for non-Claude providers)
3. **If agent-specific**: Add to `Agent` trait or use the downcast pattern via `as_any_mut()` (e.g., `input_format` for Claude)
4. **If native in underlying binary**: Pass through the flag in the agent's `execute()` method (e.g., `--worktree` for Claude)
5. **If not native**: Implement the behavior in the wrapper before delegating to the agent (e.g., worktree creation for Codex/Gemini/Copilot)
