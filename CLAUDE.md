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

Cargo workspace with two crates:
- **`zag`** (binary) — Thin CLI wrapper (argument parsing, terminal logging)
- **`zag-lib`** (library) — All core logic: agent trait, provider implementations, factory, config, builder API, output types, session logs, capabilities

### Design

- **Trait-based abstraction**: Common `Agent` trait defines the interface for all agent implementations
- **Factory pattern**: `AgentFactory` creates and configures agents based on parameters
- **Builder API**: `AgentBuilder` provides ergonomic programmatic access (see below)
- **Progress handler**: `ProgressHandler` trait abstracts terminal output so the library doesn't depend on `indicatif`
- **Model validation**: Validates model names against agent-specific allowed lists with helpful error messages
- **Subprocess delegation**: Each agent spawns its respective CLI tool, passing configuration via arguments or temporary files
- **Simple execution**: Runs agent processes and waits for completion

### Programmatic API (AgentBuilder)

`zag-lib` exposes an `AgentBuilder` for driving agents from Rust code without the CLI:

```rust
use zag::builder::AgentBuilder;

// Non-interactive exec
let output = AgentBuilder::new()
    .provider("claude")
    .model("sonnet")
    .auto_approve(true)
    .exec("write a hello world program")
    .await?;

// With JSON schema validation
let output = AgentBuilder::new()
    .provider("gemini")
    .json_schema(schema)
    .exec("list 3 colors")
    .await?;

// Interactive session
AgentBuilder::new()
    .provider("claude")
    .run(Some("initial prompt"))
    .await?;

// Resume
AgentBuilder::new()
    .provider("claude")
    .resume("session-id")
    .await?;

// Streaming input/output (Claude only)
let mut session = AgentBuilder::new()
    .provider("claude")
    .replay_user_messages(true)
    .include_partial_messages(true)
    .exec_streaming("initial prompt")
    .await?;

session.send_user_message("follow-up question").await?;
while let Some(event) = session.next_event().await? {
    println!("{:?}", event);
}
session.close_input();
session.wait().await?;
```

Custom progress reporting:
```rust
use zag::progress::ProgressHandler;

struct MyProgress;
impl ProgressHandler for MyProgress {
    fn on_success(&self, msg: &str) { println!("OK: {}", msg); }
    fn on_error(&self, msg: &str) { eprintln!("ERR: {}", msg); }
}

AgentBuilder::new()
    .on_progress(Box::new(MyProgress))
    .exec("hello")
    .await?;
```

### Key Files

#### `zag-lib/` (library crate)

| File | Purpose |
|------|---------|
| `zag-lib/src/lib.rs` | Library root — re-exports all modules |
| `zag-lib/src/builder.rs` | `AgentBuilder` — high-level programmatic API |
| `zag-lib/src/streaming.rs` | `StreamingSession` — bidirectional streaming with agents (Claude only) |
| `zag-lib/src/progress.rs` | `ProgressHandler` trait and `SilentProgress` default |
| `zag-lib/src/agent.rs` | `Agent` trait definition and `ModelSize` abstraction |
| `zag-lib/src/factory.rs` | `AgentFactory` — creates and configures agents (with pre-flight binary checks) |
| `zag-lib/src/file_util.rs` | Atomic file write utilities (write-to-tmp-then-rename) |
| `zag-lib/src/preflight.rs` | CLI binary pre-flight validation: PATH scanning, install hints |
| `zag-lib/src/config.rs` | Configuration management (`zag.toml`) |
| `zag-lib/src/output.rs` | Unified `AgentOutput` format, `Event` types, and event formatting |
| `zag-lib/src/session_log.rs` | Session log schema, writer, coordinator, backfill engine, and adapter traits |
| `zag-lib/src/capability.rs` | Provider capability structs (`ProviderCapability`, `Features`, etc.) and format helpers |
| `zag-lib/src/process.rs` | Subprocess helpers: stderr capture, exit status checking, output handling |
| `zag-lib/src/process_store.rs` | Process tracking store: `ProcessEntry`, `ProcessStore`, load/save/kill helpers |
| `zag-lib/src/sandbox.rs` | Docker sandbox configuration, command building, and removal |
| `zag-lib/src/worktree.rs` | Git worktree creation, removal, and name generation |
| `zag-lib/src/session.rs` | Session-worktree/sandbox mapping store (`sessions.json`) |
| `zag-lib/src/json_validation.rs` | JSON and JSON Schema validation utilities |
| `zag-lib/src/auto_selector.rs` | Auto provider/model selection via lightweight LLM call |
| `zag-lib/src/mcp.rs` | MCP server management: per-server TOML configs, provider sync, import |
| `zag-lib/src/search.rs` | Session log search: `SearchQuery`, `SearchMatch`, `search()`, `parse_date_arg()` |
| `zag-lib/src/skills.rs` | Provider-agnostic skill management |
| `zag-lib/src/providers/claude/mod.rs` | Claude agent implementation |
| `zag-lib/src/providers/claude/models.rs` | Claude JSON output models and conversion to unified format |
| `zag-lib/src/providers/claude/logs.rs` | Claude session log adapter |
| `zag-lib/src/providers/codex.rs` | Codex agent implementation |
| `zag-lib/src/providers/gemini.rs` | Gemini agent implementation |
| `zag-lib/src/providers/copilot.rs` | Copilot agent implementation |
| `zag-lib/src/providers/ollama.rs` | Ollama agent implementation (local models) |

#### `src/` (binary crate)

The binary crate is a thin CLI wrapper. It parses arguments with clap and delegates to `zag-lib` modules.

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point — `main()`, `resolve_provider`, `capitalize`, re-exports |
| `src/cli.rs` | Clap CLI definitions: `Cli`, `Commands`, `AgentArgs`, `SessionIsolationArgs`, all subcommand enums, parsing helpers |
| `src/commands.rs` | Management command handlers: `run_config`, `run_session`, `run_skills`, `run_mcp` |
| `src/agent_action.rs` | Core agent orchestration: `run_agent_action`, session setup, agent creation, execution |
| `src/logging.rs` | Terminal logging, spinners, colored output (implements `ProgressHandler` pattern) |
| `src/listen.rs` | Listen command: session log tailing, event formatting, session resolution |
| `src/ps.rs` | `zag ps` command: list/show/kill agent processes via `ProcessStore` |
| `src/whoami.rs` | `zag whoami` command: session identity introspection via `ZAG_*` env vars |
| `src/search.rs` | `zag search` command: CLI argument wiring, human-readable and JSON output |
| `src/capability.rs` | Re-exports zag-lib capability types + provider-specific capability constructors |
| `src/output.rs` | Re-exports zag-lib output types |
| `src/session_log.rs` | Re-exports zag-lib session_log + provider-specific wiring |
| `src/auto_selector.rs` | Auto provider/model selection via lightweight LLM call |
| `src/sandbox.rs` | Docker sandbox configuration, command building, and removal |
| `src/session.rs` | Session-worktree/sandbox mapping store (`sessions.json`) |
| `src/worktree.rs` | Git worktree creation, removal, and name generation |
| `src/json_validation.rs` | JSON and JSON Schema validation utilities |
| `src/skills.rs` | Provider-agnostic skill management: parsing, loading, syncing symlinks, system prompt injection |
| `src/skills_tests.rs` | Unit tests for skills module |
| `man/*.md` | Embedded manpages for the `zag man` command |
| `prompts/auto-selector/*.md` | Versioned prompt templates for auto-selection (latest: 3_1) |
| `prompts/json-wrap/*.md` | Versioned prompt templates for wrapping user prompts with JSON instructions (latest: 1_0) |
| `man/capability.md` | Manpage for the `zag capability` command |
| `man/listen.md` | Manpage for the `zag listen` command |
| `man/skills.md` | Manpage for the `zag skills` command |
| `man/mcp.md` | Manpage for the `zag mcp` command |
| `man/ps.md` | Manpage for the `zag ps` command |
| `man/input.md` | Manpage for the `zag input` command |
| `man/whoami.md` | Manpage for the `zag whoami` command |

#### `bindings/` (language SDKs)

Native SDK packages that invoke the `zag` CLI binary and parse its JSON output. Each provides a builder API mirroring `AgentBuilder`.

| Directory | Language | Package | Key Files |
|-----------|----------|---------|-----------|
| `bindings/typescript/` | TypeScript | `zag-agent` | `src/builder.ts`, `src/types.ts`, `src/process.ts` |
| `bindings/python/` | Python | `zag-agent` | `src/zag/builder.py`, `src/zag/types.py`, `src/zag/process.py` |
| `bindings/csharp/` | C# | `Zag` | `src/Zag/ZagBuilder.cs`, `src/Zag/Models.cs`, `src/Zag/ZagProcess.cs` |

**Design**: Each binding spawns `zag exec -o json` (or `-o stream-json` for streaming) as a subprocess, parses the JSON/NDJSON output into typed models, and exposes a fluent builder API. Zero external runtime dependencies — only stdlib in each language.

**Testing**: TypeScript uses `node --test`, Python uses `pytest`, C# uses `xunit`.

## Model Size Abstraction

Instead of specifying agent-specific model names, you can use size aliases that automatically map to the appropriate model for each agent:

```bash
# Use size aliases
zag --model large run              # Uses opus (default provider: claude)
zag -p codex --model large run     # Uses gpt-5.4
zag -p gemini --model small run    # Uses gemini-2.5-flash-lite

# Or use specific model names (passthrough)
zag --model sonnet run             # Uses sonnet directly
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
zag exec -p auto "say hello"

# Auto-select model (uses configured/default provider)
zag exec -m auto "refactor the auth system"

# Auto-select both
zag exec -p auto -m auto "complex multi-file refactor"
```

### How it works

1. The CLI runs a quick non-interactive LLM call (default: Claude sonnet) with the user's prompt
2. The selector LLM analyzes task complexity and chooses the best provider/model
3. The resolved values replace `"auto"` and execution continues normally

### Configuration

The selector LLM is configurable in `zag.toml`:

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

All configuration and state is stored under `~/.zag/`, never in the repository.

### Config File Location

The config location is automatically determined using this priority:

1. **Explicit `--root` flag**: `~/.zag/projects/<sanitized-root>/zag.toml`
2. **Git repository root**: `~/.zag/projects/<sanitized-repo-path>/zag.toml`
3. **Global config**: `~/.zag/zag.toml` (when not in a git repo)

The sanitized path strips the leading `/` and replaces `/` with `-` (e.g., `/Users/me/Source/app` → `Users-me-Source-app`).

This means:
- Each git repository has its own config under `~/.zag/projects/`
- No `.agent` folders in repository roots
- Global fallback for non-repository usage

### Config File Format

```toml
# Zag CLI Configuration

[defaults]
# Default provider (claude, codex, gemini, copilot)
# provider = "claude"

# Auto-approve all actions (skip permission prompts)
# auto_approve = false

# Default model size for all agents (small, medium, large)
# Can be overridden per-agent in [models] section
model = "medium"

# Default maximum number of agentic turns
# max_turns = 10

# Default system prompt for all agents
# system_prompt = ""

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

View, set, or initialize configuration values with `zag config`:

```bash
# Print full config file
zag config

# Read a single value
zag config provider
zag config get model.claude

# Set values (space or = syntax)
zag config provider claude
zag config provider=claude
zag config model opus
zag config model.claude=opus
zag config auto_approve true

# Unset a single config key (revert to default)
zag config unset provider
zag config unset model.claude

# Initialize default config file
zag config init

# Reset config to defaults
zag config reset

# List all config keys and current values
zag config list

# Show config file path
zag config path
```

### Configuration Priority

Settings are applied in this order (later overrides earlier):

1. **Agent defaults**: Built-in defaults for each agent
2. **Config file**: Settings from `zag.toml` (defaults.model, then models.<agent>)
3. **CLI flags**: Command-line arguments (highest priority)

### Available Settings

| Key | Description |
|-----|-------------|
| `provider` | Default provider (default: "claude") |
| `auto_approve` | Skip permission prompts (default: false) |
| `model` | Default model size for all agents (default: "medium") |
| `max_turns` | Default maximum number of agentic turns |
| `system_prompt` | Default system prompt for all agents |
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
| `listen.format` | Default output format for listen command (default: "text") |
| `listen.timestamp_format` | Strftime-style timestamp format for listen output (default: "%H:%M:%S") |

## Skills

Provider-agnostic skills are stored at `~/.zag/skills/<skill-name>/` using the [Agent Skills](https://agentskills.io) open standard format (same `SKILL.md` format used by Claude, Gemini, Copilot, and Codex).

### Storage Format

```
~/.zag/skills/<skill-name>/
├── SKILL.md       (required) YAML frontmatter + markdown instructions
├── scripts/       (optional)
├── references/    (optional)
└── assets/        (optional)
```

`SKILL.md` frontmatter: `name`, `description` (required); body is markdown instructions.

### Provider Integration

| Provider | Strategy | Target |
|----------|----------|--------|
| Claude   | Symlink  | `~/.claude/skills/agent-<name>/` |
| Gemini   | Symlink  | `~/.gemini/skills/agent-<name>/` |
| Copilot  | Symlink  | `~/.copilot/skills/agent-<name>/` |
| Codex    | Symlink  | `~/.agents/skills/agent-<name>/` |
| Ollama   | System prompt injection | N/A |

Skills are synced automatically in `run_agent_action()` (after `augment_system_prompt_for_json`, before `create_and_configure_agent`). Symlinks use `agent-` prefix to avoid collisions.

## MCP Servers

MCP (Model Context Protocol) servers are managed as individual TOML files and synced into each provider's native config format.

### Storage

- **Global**: `~/.zag/mcp/<server-name>.toml`
- **Project-scoped**: `~/.zag/projects/<sanitized-path>/mcp/<server-name>.toml`

Project-scoped servers override global servers with the same name.

### Server Format

```toml
name = "github"
description = "GitHub MCP server"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"
```

### Provider Integration

Servers are injected with a `zag-` prefix into each provider's native config. User-managed entries are never touched.

| Provider | Config File | Format |
|----------|-----------|--------|
| Claude   | `~/.claude.json` | JSON `mcpServers` |
| Gemini   | `~/.gemini/settings.json` | JSON `mcpServers` |
| Copilot  | `~/.copilot/mcp-config.json` | JSON `mcpServers` |
| Codex    | `~/.codex/config.toml` | TOML `[mcp_servers]` |
| Ollama   | N/A | Not supported |

MCP servers are synced automatically in `run_agent_action()` (after skills setup, before `create_and_configure_agent`).

### Commands

```bash
zag mcp list                          # List all MCP servers
zag mcp show github                   # Show server details
zag mcp add github --command npx --args -y @modelcontextprotocol/server-github
zag mcp add github --command npx --args -y @modelcontextprotocol/server-github --env GITHUB_TOKEN='${GITHUB_TOKEN}'
zag mcp add sentry --transport http --url https://mcp.sentry.dev/sse
zag mcp add my-db --command npx --args db-mcp --global   # Global instead of project-scoped
zag mcp remove github                 # Remove server + clean provider configs
zag mcp sync                          # Sync to all providers
zag mcp sync -p claude                # Sync to specific provider
zag mcp import --from claude          # Import from provider
zag mcp import --from codex           # Import from Codex TOML config
```

## Session Identity Environment Variables

When `zag run` or `zag exec` spawns an agent subprocess, the following environment variables are set so that child processes can discover their session identity:

| Variable | Description |
|----------|-------------|
| `ZAG_SESSION_ID` | Session UUID of the enclosing zag process |
| `ZAG_PROCESS_ID` | Process UUID of the enclosing zag process |
| `ZAG_PROVIDER` | Provider name (claude, codex, gemini, copilot, ollama) |
| `ZAG_MODEL` | Model name |
| `ZAG_ROOT` | Project root path |

These are used by `zag whoami` for agent self-discovery and by nested `zag` invocations to track parent/child session hierarchies. The `ProcessEntry` in `processes.json` stores `parent_process_id` and `parent_session_id` when a nested zag process detects these env vars.

## Logging

The CLI includes professional logging and progress indicators to provide clear feedback about operations.

### Features

- **Info messages**: Professional status messages with orange `>` prefix describing operations
- **Debug logging**: Detailed logging with `[DEBUG]` prefix for troubleshooting with the `--debug` flag
- **Progress indicators**: Animated spinner for long-running operations that clears on completion
- **Success indicators**: Green checkmark (✓) for successful operations
- **Model display**: Shows the actual model name being used
- **File-based logging**: All log messages are written to session log files in `~/.zag/logs/` (always at debug level)
- **Stderr capture**: In non-interactive (exec) mode, agent subprocess stderr is captured and logged to file. On failure, stderr is included in the error message. Interactive sessions pass stderr through unchanged.

### Harmonized Session Logs

There is a second logging layer, separate from the human/debug log in `src/logging.rs`, intended for machine consumption by other tools.

- **Purpose**: One-way normalization of provider session activity into a stable per-session NDJSON format. This is not intended to reconstruct native provider logs.
- **Storage**: Per-project under `~/.zag/projects/<sanitized-root>/logs/`.
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
  - Native session source: `~/.copilot/session-state/<session-id>/events.jsonl`
  - Supplemental metadata sources: `~/.copilot/session-state/<session-id>/vscode.metadata.json` and sometimes `workspace.yaml`
  - Observed contents: `session.start`, `session.info`, `session.truncation`, `user.message`, `assistant.turn_start`, `assistant.message`, `assistant.reasoning`, `assistant.turn_end`, `tool.execution_start`, `tool.execution_complete`
  - `events.jsonl` is append-only and includes stable event ids plus native `sessionId`
  - Tool requests appear both embedded in `assistant.message.toolRequests` and as explicit tool execution lifecycle events
  - Expected completeness: `full`

- **Ollama**
  - In this wrapper, no provider-native resumable session log/store was identified
  - Current assumption: harmonized logging for Ollama must rely on wrapper-observed prompt/stdout/stderr unless a native session/event source is discovered later
  - Expected completeness: wrapper/live-capture fallback only
  - Future work: if Ollama exposes a durable native session/event log, add a provider-owned parser there too

### Implementation Notes For Future Work

- Provider parsers should remain provider-owned. Shared code should only define:
  - normalized schema
  - storage/index/backfill mechanics
  - coordinator lifecycle
- Do not assume all providers are append-only:
  - Claude looks append-only
  - Gemini chat files look rewrite/snapshot based
  - Codex needs correlation across more than one native file
- Copilot uses a native parser over `~/.copilot/session-state/<session-id>/events.jsonl`; Ollama does not have a confirmed native session source.
- For Ollama, do not fake completeness. Keep emitting `partial` / wrapper-only coverage until a real native source is confirmed.
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
zag --debug run "write a hello world program"

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
zag exec -v "write a hello world program"

# Also works with interactive mode (no behavioral change since run always shows full output)
zag -v run
```

### Quiet Mode

Disable all logging except agent output with the `--quiet` (or `-q`) flag. This applies to all modes including `run`:

```bash
# Quiet mode - only shows agent output
zag -q exec "write a hello world program"

# Useful for scripting
result=$(zag -q exec "analyze this code")

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
$ zag exec "say hello"
Hello!

# Exec mode with verbose
$ zag exec -v "say hello"
✓ Claude initialized with model opus
    ⏺ Hello!

# Interactive mode
$ zag --model sonnet run
⠋ Initializing Claude agent
✓ Claude initialized with model sonnet
> Starting interactive session
[Agent output...]
> Session terminated

# Debug mode
$ zag --model medium --debug run
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
$ zag --model haiku -a run
✓ Claude initialized with model haiku (auto approve)
> Starting interactive session
[Agent output...]
> Session terminated

# Quiet mode (only agent output, no logging)
$ zag --model sonnet -q exec "write a hello world program"
[Agent output only...]
```

## Usage

The CLI uses a subcommand structure: `zag [flags] <action> [options]`.

The provider is specified via the `--provider` (or `-p`) flag. If omitted, it defaults to the configured provider (fallback: claude).

### Actions

- **`run`** - Start an interactive session
- **`exec`** - Run non-interactively (print output and exit)
- Resume a previous session with `run --resume <id>` or `run --continue`
- **`review`** - Review code changes (uses Codex)
- **`config`** - View or set configuration values
- **`capability`** - Show provider capability declarations
- **`listen`** - Tail a session's log events in real-time
- **`session`** - List and inspect sessions, import historical provider logs
- **`man`** - Show manual pages for commands
- **`skills`** - Manage provider-agnostic skills stored in `~/.zag/skills/`
- **`mcp`** - Manage MCP servers across providers
- **`ps`** - List, inspect, and kill agent processes started by zag
- **`search`** - Search through session logs (full-text + filters)
- **`input`** - Send a user message to a running or resumable session
- **`whoami`** - Show identity of the current zag session (for agent introspection)

```bash
# Interactive mode (uses default provider, typically claude)
zag run
zag run "write a hello world program"

# With explicit provider
zag -p codex run
zag -p gemini -m large run

# Non-interactive mode (exec)
zag exec "write a hello world program"
zag exec "analyze this code" -o json

# Non-interactive mode with streaming JSON events (NDJSON format)
zag exec "complex task" -o stream-json

# Non-interactive mode with compact JSON output
zag exec "write a hello world program" -o json
zag -p gemini exec "analyze this code" --output json

# Non-interactive mode with pretty-printed JSON output
zag exec "write a hello world program" -o json-pretty

# Non-interactive mode with plain text output (no JSON parsing)
zag exec "simple task" -o text

# Non-interactive mode with native JSON output (Claude's raw JSON format)
zag exec "write a hello world program" -o native-json

# Non-interactive mode with stream-json input format (Claude only)
echo '{"type":"message","content":"hello"}' | zag exec -i stream-json "analyze"
cat input.ndjson | zag exec --input-format stream-json "process"

# Resume a session
zag run --continue            # Resume the latest tracked session
zag run --resume <session-id> # Resume a specific session

# With specific model
zag --model opus exec "complex task"
zag -p gemini --model small exec "simple task"

# With custom system prompt
zag --system-prompt "You are a Rust expert" exec "help with ownership"

# With root directory
zag --root /path/to/project run

# Auto-approve all actions
zag --auto-approve exec "write tests"

# Additional directories
zag --add-dir ../other-repo run
zag -p gemini --add-dir /path/to/docs --add-dir /path/to/specs exec "analyze"

# Enable debug logging
zag --debug exec "analyze this code"

# Enable quiet mode (suppress all logging)
zag -q exec "write tests"

# Enable verbose mode (show styled output with icons in exec)
zag -v exec "write tests"

# Worktree mode (isolated git worktree per session)
zag -w run                          # Auto-generated worktree name
zag --worktree run                  # Same as above
zag -w my-feature run               # Named worktree
zag -p codex -w run                 # Works with any provider

# Sandbox mode (Docker sandbox microVM isolation)
zag --sandbox run                          # Auto-generated sandbox name
zag --sandbox my-sandbox run               # Named sandbox
zag --sandbox exec "write tests"           # Non-interactive in sandbox
zag -p codex --sandbox run                 # Works with any provider

# JSON output mode
zag exec --json "list 3 colors"                                        # Request JSON output
zag exec --json-schema '{"type":"object"}' "list 3 colors"             # With schema validation
zag exec --json-schema schema.json "list 3 colors"                     # Schema from file
zag exec --json-stream "list 3 colors"                                 # Stream JSON events (NDJSON)

# Pre-set session ID (for agent listen)
zag --session $(uuidgen) run                    # Know the session ID before it starts
zag --session $(uuidgen) exec "complex task"    # Works with exec too

# Limit agentic turns
zag exec --max-turns 5 "fix the bug"
zag run --max-turns 10 "refactor auth"

# Combine flags
zag --debug --model opus -a exec "complex task"
zag -q exec "simple task" -o json
zag -v exec "complex task"          # Verbose exec with icons

# Provider capabilities
zag capability                              # Default provider capabilities (JSON)
zag -p ollama capability                    # Ollama capabilities
zag -p claude capability --pretty           # Pretty-printed JSON
zag -p gemini capability -f yaml            # YAML format
zag -p codex capability -f toml             # TOML format

# Listen to session logs
zag listen <session-id>             # Listen to a specific session
zag listen --latest                 # Listen to the most recently created session
zag listen --active                 # Listen to the most recently written-to session
zag listen --latest --json          # JSON output (NDJSON)
zag listen --latest --colors        # Text with ANSI colors
zag listen --ps <pid>               # Listen by OS PID (resolves to latest session for that PID)
zag listen --ps <zag-uuid>          # Listen by zag process UUID (from `zag ps list`)

# Session management
zag session list                    # List all sessions
zag session list --json             # JSON output
zag session list -p claude          # Filter by provider
zag session list -n 5               # Show 5 most recent
zag session list --global           # List sessions across all projects
zag session show <session-id>       # Show session details
zag session show <id> --json        # JSON output
zag session delete <session-id>     # Delete a session from the store
zag session import                  # Import historical provider logs

# Skills management
zag skills list                     # List all skills in ~/.zag/skills/
zag skills add commit               # Create a new skill skeleton
zag skills add commit --description "Commit code changes"  # With description
zag skills remove commit            # Remove skill and provider symlinks
zag skills sync                     # Manually sync to all providers
zag skills sync -p claude           # Sync only to Claude
zag skills import                   # Import existing Claude skills
zag skills import --from gemini     # Import from another provider

# MCP server management
zag mcp list                       # List all MCP servers
zag mcp show github                # Show server details
zag mcp add github --command npx --args -y @modelcontextprotocol/server-github
zag mcp add sentry --transport http --url https://mcp.sentry.dev/sse
zag mcp remove github              # Remove server + clean provider configs
zag mcp sync                       # Sync to all providers
zag mcp sync -p claude             # Sync only to Claude
zag mcp import --from claude       # Import from Claude
zag mcp import --from codex        # Import from Codex

# Process management
zag ps                           # List all processes (default: all)
zag ps list                      # List all processes
zag ps list --running            # Only running processes
zag ps list -n 5                 # Show 5 most recent
zag ps list -p claude            # Filter by provider
zag ps list --json               # JSON output
zag ps show <id>                 # Show process details
zag ps show <id> --json          # JSON output
zag ps stop <id>                 # Send SIGHUP to a running process (graceful stop)
zag ps kill <id>                 # Send SIGTERM to a running process (forceful)

# Search session logs
zag search "login"                         # Search current project (and sub-projects)
zag search --global "authentication"       # Search all projects
zag search --role user "refactor"          # Only user messages
zag search --tool bash --from 7d           # Bash tool calls in last 7 days
zag search --tool-kind shell "cargo test"  # By tool kind
zag search --provider claude "error"       # Filter by provider
zag search --count "TODO"                  # Count matches only
zag search --json "api key" | jq .snippet  # JSON output
zag search --regex "fn\s+\w+_handler"      # Regex search
zag search --from 2024-01-01 "deploy"      # Date range filter

# Send input to a session
zag input "hello"                          # Auto-resolve to most recent session in this project
zag input --session <session-id> "hello"   # Send a message to a specific session
zag input --latest "continue"              # Send to the most recent session
zag input --active "run tests"             # Send to the most active session
zag input --ps 12345 "status"              # Send by PID
zag input --global "hello"                 # Auto-resolve across all projects
echo "message" | zag input                 # Pipe message from stdin
zag input --stream --latest                # Stream messages interactively (Claude only)
zag input --latest "task" -o stream-json   # NDJSON event output (Claude only)

# Session identity (agent introspection)
zag whoami                                 # Show current session identity
zag whoami --json                          # JSON output for machine consumption

# Configuration
zag config                       # Print full config
zag config provider              # Read a single value
zag config get model.claude      # Read a value (explicit get)
zag config provider gemini       # Set default provider
zag config model.claude=opus     # Set claude-specific model
zag config unset provider        # Unset a config key (revert to default)
zag config init                  # Create default config file
zag config reset                 # Reset config to defaults
zag config list                  # List all keys and current values
zag config path                  # Show config file path
```

### Review Command

Top-level `zag review` command for code review (uses Codex under the hood):

```bash
# Review uncommitted changes
zag review --uncommitted

# Review against a base branch
zag review --base main

# Review a specific commit
zag review --commit abc123

# With optional title
zag review --uncommitted --title "Feature review"

# With shared flags
zag review --uncommitted --model large --auto-approve
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
zag exec --json "list 3 colors"

# Validate against inline schema
zag exec --json-schema '{"type":"object","properties":{"colors":{"type":"array"}}}' "list 3 colors"

# Validate against schema file
zag exec --json-schema schema.json "list 3 colors"

# Stream JSON events (NDJSON) — convenience for -o stream-json
zag exec --json-stream "list 3 colors"

# Also works with run (when a prompt is provided)
zag run --json "list 3 colors"
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
$ zag --model gpt-5 run
Error: Invalid model 'gpt-5' for Claude. Available models: sonnet, opus, haiku
```

Size aliases (small, medium, large) are always valid and automatically resolve to the appropriate model for each agent.

## Supported Agents

### Claude (default)
```bash
zag [-p claude] <run|exec> [OPTIONS]
```

**Available models**: sonnet, opus, haiku
**Default**: opus

### Codex
```bash
zag -p codex <run|exec> [OPTIONS]
```

**Available models**: gpt-5.4, gpt-5.4-mini, gpt-5.3-codex, gpt-5.2-codex, gpt-5.2, gpt-5.1-codex-max, gpt-5.1-codex-mini
**Default**: gpt-5.4

### Gemini
```bash
zag -p gemini <run|exec> [OPTIONS]
```

**Available models**: auto, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite
**Default**: auto

### Copilot
```bash
zag -p copilot <run|exec> [OPTIONS]
```

**Models**: claude-sonnet-4.5 (default), claude-haiku-4.5, claude-opus-4.5, claude-sonnet-4, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.2, gpt-5.1, gpt-5, gpt-5.1-codex-mini, gpt-5-mini, gpt-4.1, gemini-3-pro-preview

### Ollama
```bash
zag -p ollama <run|exec> [OPTIONS]
```

**Default model**: qwen3.5:9b
**Available sizes**: 0.8b, 2b, 4b, 9b, 27b, 35b, 122b
**Accepts any model** from ollama.com — use `--model <name>` for the model and `--size <size>` for parameter size.

```bash
zag -p ollama run                          # qwen3.5:9b (defaults)
zag -p ollama --size 35b exec "hello"      # qwen3.5:35b
zag -p ollama --model llama3 run           # llama3:9b (default size)
zag -p ollama --model small run            # qwen3.5:2b (size alias)
```

Does not support `run --resume` or `run --continue`.

### Review
```bash
zag review [--uncommitted] [--base <BRANCH>] [--commit <SHA>] [--title <TITLE>] [OPTIONS]
```

Uses Codex under the hood for code review.

## Worktree Mode

The `--worktree` (or `-w`) flag creates an isolated git worktree for the session, keeping changes separate from the main working tree.

```bash
# Auto-generated worktree name
zag -w run

# Named worktree
zag -w my-feature exec "implement feature X"

# Works with any provider
zag -p codex -w run
zag -p gemini -w my-task exec "analyze code"
```

### Worktree Location

All providers use the same worktree path: `~/.zag/worktrees/<sanitized-repo-path>/<name>/`. The wrapper creates the worktree via `git worktree add --detach` and sets the agent's root directory to the worktree path. The sanitized path uses the same scheme as config (`/Users/me/Source/app` → `Users-me-Source-app`).

### Session Tracking & Resume

Worktree sessions are tracked in `~/.zag/projects/<sanitized-path>/sessions.json`. Each session records the session ID, provider, worktree path, and creation timestamp.

- A UUID session ID is generated for each worktree session
- `zag run --resume <session-id>` automatically resumes inside the correct worktree
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
zag --sandbox run

# Named sandbox
zag --sandbox my-sandbox exec "implement feature X"

# Works with any provider
zag -p codex --sandbox run
zag -p gemini --sandbox my-task exec "analyze code"
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

Sandbox sessions are tracked in `~/.zag/projects/<sanitized-path>/sessions.json` with a `sandbox_name` field. Each session records the session ID, provider, workspace path, sandbox name, and creation timestamp.

- `zag run --resume <session-id>` looks up the sandbox name and re-configures the agent with `SandboxConfig`
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

Pattern for adding new features:

1. **Core logic goes in `zag-lib`**: Agent trait changes, provider implementations, builder options, config — all in the library crate
2. **CLI-only changes go in `src/main.rs`**: New clap flags, terminal-specific formatting
3. **For new builder options**: Add a setter to `AgentBuilder` in `zag-lib/src/builder.rs`, then wire it in `create_agent()` or the terminal methods
4. **For new CLI flags**: Add to `Cli` struct in `src/main.rs`, then map to the corresponding `AgentBuilder` setter or handle in `run_agent_action()`
5. **For agent-specific features**: Add to `Agent` trait in `zag-lib/src/agent.rs` or use the downcast pattern via `as_any_mut()` (e.g., `input_format` for Claude)
6. **For new provider support**: Add a new module under `zag-lib/src/providers/`, register in `zag-lib/src/factory.rs`

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
