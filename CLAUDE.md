# CLAUDE.md

Keep this file updated when making architectural changes to the codebase.

## Build Commands

- `make build` - Development build
- `make release` - Release build
- `make test` - Run tests
- `make fmt` - Format code
- `make clippy` - Lint

## Commit Messages

- Follow the repository's conventional commit style: `type(scope): summary`
- Use lowercase types like `feat`, `fix`, `refactor`, `docs`, or `test`
- Keep scopes lowercase and comma-separated when multiple areas changed, e.g. `refactor(codex,docs): update Codex model lineup`
- Write the summary in imperative mood and keep it specific to the change

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
| `src/session_log.rs` | Harmonized per-session log schema, storage, backfill, and live adapter wiring |
| `src/claude/mod.rs` | Claude agent implementation |
| `src/claude/models.rs` | Claude JSON output models and conversion to unified format |
| `src/codex.rs` | Codex agent implementation |
| `src/gemini.rs` | Gemini agent implementation |
| `src/copilot.rs` | Copilot agent implementation |
| `src/ollama.rs` | Ollama agent implementation (local models) |
| `src/process.rs` | Subprocess helpers: stderr capture, exit status checking, output handling |
| `src/output.rs` | Unified AgentOutput format and event formatting |
| `src/auto_selector.rs` | Auto provider/model selection via lightweight LLM call |
| `src/sandbox.rs` | Docker sandbox configuration, command building, and removal |
| `src/session.rs` | Session-worktree/sandbox mapping store (`sessions.json`) |
| `src/worktree.rs` | Git worktree creation, removal, and name generation |
| `src/json_validation.rs` | JSON and JSON Schema validation utilities |
| `man/*.md` | Embedded manpages for the `agent man` command |
| `prompts/auto-selector/*.md` | Versioned prompt templates for auto-selection (latest: 3_1) |
| `prompts/json-wrap/*.md` | Versioned prompt templates for wrapping user prompts with JSON instructions (latest: 1_0) |

## Model Size Abstraction

Instead of specifying agent-specific model names, you can use size aliases that automatically map to the appropriate model for each agent:

```bash
# Use size aliases
agent --model large run              # Uses opus (default provider: claude)
agent -p codex --model large run     # Uses gpt-5.4
agent -p gemini --model small run    # Uses gemini-2.5-flash-lite

# Or use specific model names (passthrough)
agent --model sonnet run             # Uses sonnet directly
```

### Size Mappings

Each agent implements `model_for_size()` in its `Agent` trait implementation:

| Size | Claude | Codex | Gemini | Copilot | Ollama (size) |
|------|--------|-------|--------|---------|---------------|
| `small` / `s` | haiku | gpt-5.4-mini | gemini-2.5-flash-lite | claude-haiku-4.5 | 2b |
| `medium` / `m` | sonnet | gpt-5.3-codex | gemini-2.5-flash | claude-sonnet-4.5 | 9b |
| `large` / `l` / `max` | opus | gpt-5.4 | gemini-2.5-pro | claude-opus-4.5 | 35b |

For Ollama, size aliases map to parameter sizes (not model names). The model is always `ollama.model` config (default: qwen3.5). Sizes are configurable via `ollama.size_small`, `ollama.size_medium`, `ollama.size_large`.

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

1. The CLI runs a quick non-interactive LLM call (default: Claude sonnet) with the user's prompt
2. The selector LLM analyzes task complexity and chooses the best provider/model
3. The resolved values replace `"auto"` and execution continues normally

### Configuration

The selector LLM is configurable in `agent.toml`:

```toml
[auto]
# Provider used for auto-selection (default: "claude")
# provider = "claude"
# Model used for auto-selection (default: "sonnet")
# model = "sonnet"
```

Config keys: `auto.provider`, `auto.model`

### Restrictions

- Requires a prompt to analyze (errors if used with `run` without a prompt)
- Cannot be used with `run --resume`, `run --continue`, `review`, or `config`

## Configuration

All configuration and state is stored under `~/.agent/`, never in the repository.

### Config File Location

The config location is automatically determined using this priority:

1. **Explicit `--root` flag**: `~/.agent/projects/<sanitized-root>/agent.toml`
2. **Git repository root**: `~/.agent/projects/<sanitized-repo-path>/agent.toml`
3. **Global config**: `~/.agent/agent.toml` (when not in a git repo)

The sanitized path strips the leading `/` and replaces `/` with `-` (e.g., `/Users/me/Source/app` → `Users-me-Source-app`).

This means:
- Each git repository has its own config under `~/.agent/projects/`
- No `.agent` folders in repository roots
- Global fallback for non-repository usage

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
# codex = "gpt-5.4"
# gemini = "auto"
# copilot = "claude-sonnet-4.5"

[auto]
# Settings for auto provider/model selection (-p auto / -m auto)
# provider = "claude"
# model = "haiku"

[ollama]
# Ollama-specific settings
# model = "qwen3.5"
# size = "9b"
# size_small = "2b"
# size_medium = "9b"
# size_large = "35b"
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
2. **Config file**: Settings from `agent.toml` (defaults.model, then models.<agent>)
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
| `auto.model` | Model for auto-selection LLM call (default: "sonnet") |
| `ollama.model` | Default Ollama model name (default: "qwen3.5") |
| `ollama.size` | Default Ollama parameter size (default: "9b") |
| `ollama.size_small` | Size for small alias (default: "2b") |
| `ollama.size_medium` | Size for medium alias (default: "9b") |
| `ollama.size_large` | Size for large alias (default: "35b") |

## Logging

The CLI includes professional logging and progress indicators to provide clear feedback about operations.

### Features

- **Info messages**: Professional status messages with orange `>` prefix describing operations
- **Debug logging**: Detailed logging with `[DEBUG]` prefix for troubleshooting with the `--debug` flag
- **Progress indicators**: Animated spinner for long-running operations that clears on completion
- **Success indicators**: Green checkmark (✓) for successful operations
- **Model display**: Shows the actual model name being used
- **File-based logging**: All log messages are written to session log files in `~/.agent/logs/` (always at debug level)
- **Stderr capture**: In non-interactive (exec) mode, agent subprocess stderr is captured and logged to file. On failure, stderr is included in the error message. Interactive sessions pass stderr through unchanged.

### Harmonized Session Logs

There is a second logging layer, separate from the human/debug log in `src/logging.rs`, intended for machine consumption by other tools.

- **Purpose**: One-way normalization of provider session activity into a stable per-session NDJSON format. This is not intended to reconstruct native provider logs.
- **Storage**: Per-project under `~/.agent/projects/<sanitized-root>/logs/`.
- **Primary file shape**:
  - `logs/sessions/<wrapper-session-id>.jsonl` — normalized events
  - `logs/index.json` — session metadata / lookup
  - `logs/backfill_state.json` — one-time import state
- **Normalized events** include the important parts only: session start/end, user messages, assistant messages, reasoning/thinking, tool calls, tool results, permission outcomes, provider status lines, stderr, and parse warnings.
- **Completeness is explicit**:
  - `full` when native provider storage exposes the needed detail
  - `partial` when only some event classes are available
  - `metadata_only` when only session discovery metadata exists

### Provider Log Sources

The harmonized log design is based on real local provider files inspected during implementation, plus the repo's existing session discovery/parsing code.

- **Claude**
  - Native session source: `~/.claude/projects/**/<session-id>.jsonl`
  - Observed contents: queue operations, user messages, assistant messages, thinking blocks, tool use blocks, tool result/user echo blocks, result events, cwd/session metadata
  - Sidecars may exist under provider-owned subdirectories such as `tool-results/`
  - Expected completeness: `full`

- **Codex**
  - Native prompt history: `~/.codex/history.jsonl`
  - Native live activity log: `~/.codex/log/codex-tui.log`
  - Observed contents:
    - `history.jsonl` stores `session_id`, timestamp, and prompt text
    - `codex-tui.log` stores `thread_id`-scoped live records including `ToolCall:` lines and provider/client status lines
  - Important limitation: the observed TUI log is good for tool calls and status, but not a complete canonical source for all tool results / final assistant text in every mode
  - Expected completeness: `partial` for interactive/native-log ingestion, better coverage for non-interactive JSON/NDJSON flows

- **Gemini**
  - Native session source: `~/.gemini/tmp/*/chats/session-*.json`
  - Supplemental metadata source: `~/.gemini/tmp/*/logs.json`
  - Observed contents: `sessionId`, message array, user messages, assistant messages, thoughts/reasoning entries, token/model metadata
  - File behavior appears snapshot/rewrite-oriented rather than append-only
  - Expected completeness: `full`

- **Copilot**
  - Discovered native session location: `~/.config/github-copilot/rd/chat-sessions/<session-id>/`
  - Observed storage is opaque/binary/Xodus-like (`*.xd`, blob directories), not a simple text or JSON log
  - Current assumption: provider-native metadata discovery is possible, but a proper semantic parser is not implemented until a reliable native log/export format is known
  - Expected completeness today: `metadata_only` or wrapper/live-capture fallback
  - Future work: once a real textual/session event source is found, add a provider-owned parser rather than pushing Copilot-specific logic into the shared logging layer

- **Ollama**
  - In this wrapper, no provider-native resumable session log/store was identified
  - Current assumption: harmonized logging for Ollama must rely on wrapper-observed prompt/stdout/stderr unless a native session/event source is discovered later
  - Expected completeness today: wrapper/live-capture fallback only
  - Future work: if Ollama exposes a durable native session/event log, add a provider-owned parser there too

### Implementation Notes For Future Work

- Provider parsers should remain provider-owned. Shared code should only define:
  - normalized schema
  - storage/index/backfill mechanics
  - coordinator lifecycle
- Do not assume all providers are append-only:
  - Claude looks append-only
  - Gemini chat files look rewrite/snapshot based
  - Codex currently needs correlation across more than one native file
- For Copilot and Ollama, do not fake completeness. Keep emitting `partial` / `metadata_only` until a real native source is confirmed.
- If you later find proper Copilot or Ollama logs, update this file with:
  - exact file paths
  - whether files append or rewrite
  - stable ids available for dedupe/correlation
  - whether tool calls, tool results, assistant text, and reasoning are actually present
  - expected completeness level after that parser is added

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
- Resume a previous session with `run --resume <id>` or `run --continue`
- **`review`** - Review code changes (uses Codex)
- **`config`** - View or set configuration values
- **`man`** - Show manual pages for commands

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
agent run --continue            # Resume the latest tracked session
agent run --resume <session-id> # Resume a specific session

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

# Sandbox mode (Docker sandbox microVM isolation)
agent --sandbox run                          # Auto-generated sandbox name
agent --sandbox my-sandbox run               # Named sandbox
agent --sandbox exec "write tests"           # Non-interactive in sandbox
agent -p codex --sandbox run                 # Works with any provider

# JSON output mode
agent exec --json "list 3 colors"                                        # Request JSON output
agent exec --json-schema '{"type":"object"}' "list 3 colors"             # With schema validation
agent exec --json-schema schema.json "list 3 colors"                     # Schema from file
agent exec --json-stream "list 3 colors"                                 # Stream JSON events (NDJSON)

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

# Stream JSON events (NDJSON) — convenience for -o stream-json
agent exec --json-stream "list 3 colors"

# Also works with run (when a prompt is provided)
agent run --json "list 3 colors"
```

### Behavior

- `--json-schema` implies `--json`
- `--json-stream` is mutually exclusive with `--json`/`--json-schema`
- Cannot be used with `run --resume`, `run --continue`, `review`, or `config`
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
agent [-p claude] <run|exec> [OPTIONS]
```

**Available models**: sonnet, opus, haiku
**Default**: opus

### Codex
```bash
agent -p codex <run|exec> [OPTIONS]
```

**Available models**: gpt-5.4, gpt-5.4-mini, gpt-5.3-codex, gpt-5.2-codex, gpt-5.2, gpt-5.1-codex-max, gpt-5.1-codex-mini
**Default**: gpt-5.4

### Gemini
```bash
agent -p gemini <run|exec> [OPTIONS]
```

**Available models**: auto, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite
**Default**: auto

### Copilot
```bash
agent -p copilot <run|exec> [OPTIONS]
```

**Models**: claude-sonnet-4.5 (default), claude-haiku-4.5, claude-opus-4.5, claude-sonnet-4, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.2, gpt-5.1, gpt-5, gpt-5.1-codex-mini, gpt-5-mini, gpt-4.1, gemini-3-pro-preview

### Ollama
```bash
agent -p ollama <run|exec> [OPTIONS]
```

**Default model**: qwen3.5:9b
**Available sizes**: 0.8b, 2b, 4b, 9b, 27b, 35b, 122b
**Accepts any model** from ollama.com — use `--model <name>` for the model and `--size <size>` for parameter size.

```bash
agent -p ollama run                          # qwen3.5:9b (defaults)
agent -p ollama --size 35b exec "hello"      # qwen3.5:35b
agent -p ollama --model llama3 run           # llama3:9b (default size)
agent -p ollama --model small run            # qwen3.5:2b (size alias)
```

Does not support `run --resume` or `run --continue`.

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

### Worktree Location

All providers use the same worktree path: `~/.agent/worktrees/<sanitized-repo-path>/<name>/`. The wrapper creates the worktree via `git worktree add --detach` and sets the agent's root directory to the worktree path. The sanitized path uses the same scheme as config (`/Users/me/Source/app` → `Users-me-Source-app`).

### Session Tracking & Resume

Worktree sessions are tracked in `~/.agent/projects/<sanitized-path>/sessions.json`. Each session records the session ID, provider, worktree path, and creation timestamp.

- A UUID session ID is generated for each worktree session
- `agent run --resume <session-id>` automatically resumes inside the correct worktree
- If the worktree no longer exists, the stale mapping is removed and resume proceeds without it

### Cleanup Behavior

After a worktree session ends, the CLI checks for uncommitted changes (staged, unstaged, or untracked):

- **No changes**: The worktree is automatically removed (no prompt). A message is printed: `✓ Worktree removed (no changes)`
- **Has changes (interactive `run`)**: The user is prompted whether to keep or remove the worktree
- **Has changes (`exec`)**: The worktree is kept and the resume command is printed

### Restrictions

- Cannot be used with `review` or `config` subcommands
- `--worktree` cannot be combined with `run --resume` or `run --continue`
- Requires a git repository (errors if not in one)

## Sandbox Mode

The `--sandbox` flag runs agents inside Docker sandbox microVMs for stronger isolation than git worktrees.

```bash
# Auto-generated sandbox name
agent --sandbox run

# Named sandbox
agent --sandbox my-sandbox exec "implement feature X"

# Works with any provider
agent -p codex --sandbox run
agent -p gemini --sandbox my-task exec "analyze code"
```

### How It Works

Each agent's `execute()` method checks for a `SandboxConfig`. When present, instead of running the agent binary directly, the command is wrapped in `docker sandbox run`:

```
# Without sandbox:
claude --print --model opus "hello"

# With sandbox:
docker sandbox run --name sandbox-a1b2c3d4 docker/sandbox-templates:claude-code /workspace -- --print --model opus "hello"
```

### Sandbox Templates

Each provider maps to a Docker sandbox template:

| Provider | Template |
|----------|----------|
| Claude | `docker/sandbox-templates:claude-code` |
| Codex | `docker/sandbox-templates:codex` |
| Gemini | `docker/sandbox-templates:gemini` |
| Copilot | `docker/sandbox-templates:copilot` |

### Agent-Specific Behavior in Sandbox

- **Claude**: `--dangerously-skip-permissions` is skipped (sandbox provides isolation by default). `current_dir()` is not set on the docker command.
- **Codex**: `--cd` flag is skipped (workspace handles the root directory).
- **Gemini**: `current_dir()` is not set on the docker command.
- **Copilot**: `current_dir()` is not set on the docker command.

### Session Tracking & Resume

Sandbox sessions are tracked in `~/.agent/projects/<sanitized-path>/sessions.json` with a `sandbox_name` field. Each session records the session ID, provider, workspace path, sandbox name, and creation timestamp.

- `agent run --resume <session-id>` looks up the sandbox name and re-configures the agent with `SandboxConfig`
- The sandbox is idempotent — `docker sandbox run` with the same name reuses the existing VM

### Cleanup Prompt

After interactive (`run`) sandbox sessions:

```
> Sandbox: sandbox-a1b2c3d4
> Keep sandbox? [Y/n]
```

- **Y (default)**: Keeps the sandbox and prints the resume command
- **n**: Removes the sandbox via `docker sandbox rm` and deletes the session mapping
- `exec` sessions skip the prompt (always keep)

### Restrictions

- `--sandbox` and `--worktree` are mutually exclusive
- Cannot be used with `review`, `config`, or `man` subcommands
- `--sandbox` cannot be combined with `run --resume` or `run --continue`

### Interaction Matrix

| Feature | Behavior with `--sandbox` |
|---------|--------------------------|
| `--worktree` | Mutually exclusive (error) |
| `--auto-approve` | Redundant for Claude (sandbox default), still passed for others |
| `--root` | Used as workspace path |
| `--json` / `--json-schema` | Works (flags passed through to agent inside sandbox) |
| `--system-prompt` | Works (files written to workspace, synced into sandbox) |
| `run --resume` / `run --continue` | Works via session store `sandbox_name` lookup |
| `exec` | Works, no cleanup prompt |
| `run` | Works, cleanup prompt shown |
| `review` / `config` / `man` | Not supported (error) |

## How to Implement New Features

Pattern for adding new CLI features:

1. **Add CLI flag** to `Cli` struct in `src/main.rs` (use `global = true` for cross-subcommand flags)
2. **If cross-cutting**: Handle in the appropriate sub-function of `run_agent_action()`:
   - `resolve_auto_selection()` — auto provider/model selection
   - `augment_system_prompt_for_json()` — system prompt modifications
   - `setup_worktree()` — worktree creation and session ID generation
   - `setup_sandbox()` — sandbox creation and session ID generation
   - `create_and_configure_agent()` — agent factory call and option setting
   - `execute_action()` — the run/exec dispatch
3. **If agent-specific**: Add to `Agent` trait or use the downcast pattern via `as_any_mut()` (e.g., `input_format` for Claude). Claude-specific options are consolidated in a single downcast block inside `create_and_configure_agent()`.
4. **If native in underlying binary**: Pass through the flag in the agent's `execute()` method (e.g., `--worktree` for Claude)
5. **If not native**: Implement the behavior in the wrapper before delegating to the agent (e.g., worktree creation for Codex/Gemini/Copilot)

## Development Process

Follow these steps when making changes to the codebase:

1. **Make changes** — implement the feature, fix, or refactor
2. **Write tests** — add unit tests in the corresponding `*_tests.rs` file (tests live in separate files, not inline)
3. **Build** — `make build` (must compile cleanly)
4. **Run tests** — `make test` (all tests must pass)
5. **Lint** — `make clippy` (zero warnings)
6. **Format** — `make fmt`
7. **Update README.md** — if the change affects user-facing behavior, CLI flags, supported models, or usage examples
8. **Update CLAUDE.md** — if the change affects architecture, key files, configuration, or development patterns
9. **Update manpages** — if the change adds/removes/modifies commands, flags, or behavior documented in `man/*.md`
10. **Commit** — use `/commit` to commit with conventional commit messages

## Context Window Guidelines

When using a 1M context model (e.g., Opus 4.6 1M), do NOT use exploration agents (subagent_type=Explore). The large context window can hold sufficient codebase context directly — use Glob, Grep, and Read tools instead of delegating to subagents.
