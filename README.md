# zag

One CLI for all your AI coding agents.

`zag` wraps Claude, Codex, Gemini, Copilot, and Ollama behind a single command so you can switch between them without learning five different CLIs. It adds cross-provider features on top: model size aliases, automatic provider/model selection, git worktree isolation, Docker sandboxing, structured JSON output with schema validation, unified session logs, and a programmatic Rust API.

## Prerequisites

- **Rust 1.85+** (edition 2024) — for building from source
- **git** — required for `--worktree` isolation
- **Docker** — required for `--sandbox` isolation (optional)
- At least one agent CLI installed (see below)

## Install

### From crates.io

```bash
cargo install zag-cli
```

### From GitHub Releases

Download a pre-built binary from [GitHub Releases](https://github.com/niclaslindstedt/zag/releases), extract it, and place it in your `PATH`.

### From source

```bash
git clone https://github.com/niclaslindstedt/zag.git
cd zag
cargo install --path .
```

### As a library

```bash
cargo add zag
```

### Agent CLIs

You need at least one underlying agent CLI installed:

| Provider | Install command | Link |
|----------|----------------|------|
| Claude | `npm install -g @anthropic-ai/claude-code` | [docs](https://docs.anthropic.com/en/docs/claude-code) |
| Codex | `npm install -g @openai/codex` | [repo](https://github.com/openai/codex) |
| Gemini | `npm install -g @anthropic-ai/gemini-cli` | [repo](https://github.com/google-gemini/gemini-cli) |
| Copilot | `gh extension install github/gh-copilot` | [docs](https://docs.github.com/en/copilot/github-copilot-in-the-cli) |
| Ollama | See [ollama.com/download](https://ollama.com/download) | [site](https://ollama.com) |

`zag` checks for the required binary before running and provides install hints if it's missing.

## Quick start

```bash
# Interactive session with Claude (the default provider)
zag run

# Non-interactive — prints the response and exits
zag exec "write a hello world program in Rust"

# Pick a different provider
zag -p gemini run
zag -p codex exec "add error handling to src/main.rs"

# Use size aliases instead of provider-specific model names
zag -m small exec "what does this function do?"   # fastest/cheapest
zag -m large run                                   # most capable

# Let an LLM pick the best provider and model for the task
zag -p auto -m auto exec "refactor the auth module"

# Code review (delegates to Codex)
zag review --uncommitted
```

## Providers

| Provider | Default model | Size aliases (small / medium / large) |
|----------|---------------|---------------------------------------|
| **claude** | default | haiku / sonnet / default |
| **codex** | gpt-5.4 | gpt-5.4-mini / gpt-5.3-codex / gpt-5.4 |
| **gemini** | auto | gemini-2.5-flash-lite / gemini-2.5-flash / gemini-2.5-pro |
| **copilot** | claude-sonnet-4.5 | claude-haiku-4.5 / claude-sonnet-4.5 / claude-opus-4.5 |
| **ollama** | qwen3.5:9b | 2b / 9b / 35b (parameter sizes, any model from ollama.com) |

Size aliases let you write `zag -m large exec "..."` and get the right model regardless of which provider you're using. For Claude, `default` delegates model selection to the Claude CLI itself.

## Commands

```
zag run [prompt]              Interactive session (optional initial prompt)
zag exec <prompt>             Non-interactive — print output and exit
zag review                    Code review (--uncommitted, --base, --commit)
zag config [key] [value]      View or set configuration
zag session list|show|import  List/inspect sessions, import historical logs
zag listen <id>               Tail a session's log events in real-time
zag ps list|show|stop|kill    List, inspect, and manage agent processes
zag search <query>            Search through session logs
zag input [message]           Send a user message to a single session
zag broadcast [message]       Send a message to all sessions in the project

zag spawn <prompt>            Launch background agent, return session ID
zag wait <id>... [--timeout]  Block until session(s) complete
zag status <id>               Machine-readable session health check
zag collect [--tag <tag>]     Gather results from multiple sessions
zag env [--session <id>]      Export session environment variables

zag capability                Show provider capability declarations
zag skills list|add|remove|sync|import   Manage provider-agnostic skills
zag mcp list|add|remove|sync|import     Manage MCP servers across providers
zag whoami                    Show current session identity (for agents)
zag man [command]             Built-in manual pages
```

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--provider <name>` | `-p` | claude, codex, gemini, copilot, ollama, auto |
| `--model <name>` | `-m` | Model name, size alias (small/medium/large), or auto |
| `--system-prompt <text>` | `-s` | Appended to the agent's system prompt |
| `--root <path>` | `-r` | Root directory for the agent |
| `--auto-approve` | `-a` | Skip permission prompts |
| `--add-dir <path>` | | Additional directories to include (repeatable) |
| `--worktree [name]` | `-w` | Run in an isolated git worktree |
| `--sandbox [name]` | | Run inside a Docker sandbox |
| `--json` | | Request structured JSON output |
| `--json-schema <schema>` | | Validate output against a JSON schema |
| `--json-stream` | | Stream JSON events (NDJSON) |
| `--session <uuid>` | | Pre-set the session ID |
| `--name <name>` | | Human-readable session name (for discovery) |
| `--description <text>` | | Short description of the session's purpose |
| `--tag <tag>` | | Session tag (repeatable, for discovery/filtering) |
| `--max-turns <n>` | | Maximum number of agentic turns |
| `--size <size>` | | Ollama parameter size (e.g., 2b, 9b, 35b) |
| `--show-usage` | | Show token usage statistics (JSON output mode) |
| `--debug` | `-d` | Debug logging |
| `--quiet` | `-q` | Suppress all output except the agent's response |
| `--verbose` | `-v` | Styled output with icons in exec mode |

## Session management

Every interactive session gets a session ID. You can name and tag sessions for discovery, resume them, and `zag` tracks provider-native session IDs automatically.

```bash
# Create sessions with metadata for discovery
zag exec --name "backend-agent" --tag backend "implement API"
zag run --name "frontend-agent" --tag frontend --description "CSS work"

# Resume a specific session
zag run --resume <session-id>

# Resume the most recent session
zag run --continue

# List and filter sessions
zag session list
zag session list --tag backend     # filter by tag
zag session list --name frontend   # filter by name

# Send messages by name
zag input --name backend-agent "check the auth module"

# Broadcast to all sessions in the project (or filter by tag)
zag broadcast "report status"
zag broadcast --tag backend "report status"

# Update session metadata
zag session update <id> --tag new-tag

# Tail a session's logs in real-time (from another terminal)
zag listen <session-id>
zag listen --latest --rich-text
zag listen --ps <pid>             # by OS PID or zag process UUID
```

## Orchestration

`zag` provides primitives for launching, synchronizing, and collecting results from multiple agent sessions. These are building blocks — not an orchestration engine — designed for shell scripts and pipelines.

```bash
# Spawn parallel agents
sid1=$(zag spawn --name analyzer --tag batch -p claude "analyze auth module")
sid2=$(zag spawn --name reviewer --tag batch -p gemini "review test coverage")
sid3=$(zag spawn --name scanner --tag batch -p codex "find security issues")

# Check health
zag status $sid1                          # → running | idle | completed | failed | dead

# Block until all finish (exit code reflects success)
zag wait --tag batch --timeout 10m

# Collect results
zag collect --tag batch --json > results.json

# Feed a session's result into a new agent
zag exec --context $sid1 "summarize the analysis and suggest fixes"

# Propagate agent failure as a non-zero exit code
zag exec --exit-on-failure "fix the bug" || echo "Agent reported failure"

# Export session env for nested invocations
eval $(zag env --shell --session $sid1)

# Query parent-child process trees
zag ps list --children $PARENT_SESSION_ID
zag session list --parent $PARENT_SESSION_ID

# Filter listen to specific event types
zag listen $sid1 --filter session_ended --filter tool_call
```

Filesystem lifecycle markers are written to `~/.zag/events/` (`.started` and `.ended` files) for external non-Rust orchestrators that prefer `inotifywait` over polling.

## Worktree and sandbox isolation

```bash
# Worktree: isolated git worktree per session
zag -w run                        # auto-named
zag -w my-feature exec "..."      # named

# Sandbox: Docker microVM isolation
zag --sandbox run                  # auto-named
zag --sandbox my-sandbox exec "..."

# Both track sessions — resume restores the correct workspace
zag run --resume <id>
```

After interactive sessions, you're prompted to keep or remove the workspace. Exec sessions with changes are kept automatically with a resume command printed.

## JSON output

```bash
# Request JSON output
zag exec --json "list 3 programming languages"

# Validate against a schema (inline or file path)
zag exec --json-schema '{"type":"object","required":["languages"]}' "list 3 languages"

# Stream events as NDJSON
zag exec --json-stream "complex task"
```

Claude uses its native `--json-schema` support. Other providers get JSON instructions injected into the system prompt. On validation failure, `zag` retries up to 3 times via session resume.

### Output formats

With `exec -o <format>`:

| Format | Description |
|--------|-------------|
| *(default)* | Streamed text — beautiful formatting for Claude, plain for others |
| `text` | Raw agent output, no parsing |
| `json` | Compact unified JSON (AgentOutput) |
| `json-pretty` | Pretty-printed unified JSON |
| `stream-json` | NDJSON event stream (unified format) |
| `native-json` | Claude's raw JSON format (Claude only) |

## Configuration

Per-project config lives at `~/.zag/projects/<sanitized-path>/zag.toml`. Falls back to `~/.zag/zag.toml` outside of git repos.

```bash
zag config                          # Print current config
zag config provider gemini          # Set default provider
zag config model.claude=opus        # Set per-agent model default
zag config auto_approve true        # Skip permission prompts by default
zag config max_turns 10             # Set default max agentic turns
zag config system_prompt "Be concise" # Set default system prompt
zag config unset provider           # Unset a config key (revert to default)
```

```toml
[defaults]
provider = "claude"
model = "medium"
auto_approve = false
# max_turns = 10
# system_prompt = ""

[models]
claude = "opus"
codex = "gpt-5.4"

[auto]
provider = "claude"
model = "sonnet"

[ollama]
model = "qwen3.5"
size = "9b"
```

Settings priority: CLI flags > config file > agent defaults.

## Skills

`zag` supports provider-agnostic skills using the [Agent Skills](https://agentskills.io) open standard. Skills are stored in `~/.zag/skills/` and automatically synced to each provider's native skill directory via symlinks.

```bash
zag skills list                     # List all skills
zag skills add commit               # Create a new skill
zag skills import --from claude     # Import existing Claude skills
zag skills sync                     # Sync to all providers
```

## MCP Servers

Manage MCP (Model Context Protocol) servers across all providers from a single place. Each server is stored as an individual TOML file in `~/.zag/mcp/` (global) or `~/.zag/projects/<path>/mcp/` (project-scoped), and synced into each provider's native config format with a `zag-` prefix.

```bash
zag mcp add github --command npx --args -y @modelcontextprotocol/server-github
zag mcp add sentry --transport http --url https://mcp.sentry.dev/sse
zag mcp list                        # List all MCP servers
zag mcp sync                        # Sync to all providers
zag mcp import --from claude        # Import from provider config
zag mcp remove github               # Remove + clean provider configs
```

Supported providers: Claude (`~/.claude.json`), Gemini (`~/.gemini/settings.json`), Copilot (`~/.copilot/mcp-config.json`), Codex (`~/.codex/config.toml`).

## Programmatic API

The `zag-lib` crate exposes an `AgentBuilder` for driving agents from Rust code:

```rust
use zag::builder::AgentBuilder;

let output = AgentBuilder::new()
    .provider("claude")
    .model("sonnet")
    .auto_approve(true)
    .exec("write a hello world program")
    .await?;

println!("{}", output.result.unwrap_or_default());
```

See the [`zag-lib` crate](zag-lib/) for the full API including JSON schema validation, custom progress handlers, and interactive sessions.

### Language bindings

SDK packages are available for TypeScript, Python, and C#. Each wraps the `zag` CLI and exposes a fluent builder API with typed output models.

**TypeScript** (`bindings/typescript/`)

```typescript
import { ZagBuilder } from "zag-agent";

const output = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .autoApprove()
  .exec("write a hello world program");

console.log(output.result);

// Streaming
for await (const event of new ZagBuilder().provider("claude").stream("analyze code")) {
  console.log(event.type);
}
```

**Python** (`bindings/python/`)

```python
from zag import ZagBuilder

output = await ZagBuilder() \
    .provider("claude") \
    .model("sonnet") \
    .auto_approve() \
    .exec("write a hello world program")

print(output.result)

# Streaming
async for event in await ZagBuilder().provider("claude").stream("analyze code"):
    print(event.type)
```

**C#** (`bindings/csharp/`)

```csharp
using Zag;

var output = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .AutoApprove()
    .ExecAsync("write a hello world program");

Console.WriteLine(output.Result);

// Streaming
await foreach (var evt in new ZagBuilder().Provider("claude").StreamAsync("analyze code"))
{
    Console.WriteLine(evt.Type);
}
```

## Examples

The `examples/` directory contains complete projects demonstrating `zag` usage:

- **[cv-review](examples/cv-review/)** — A Rust program that uses the `zag` library crate to review CVs against job descriptions using parallel agent invocations
- **[react-claude-interface](examples/react-claude-interface/)** — A React web app that provides a Claude Code-like chat interface powered by `zag exec` and `zag input` with streaming NDJSON events over Server-Sent Events

## Troubleshooting

**"CLI not found in PATH"** — The agent CLI binary isn't installed or isn't in your `PATH`. Install it using the commands in the [Agent CLIs](#agent-clis) table above.

**"Invalid model 'X' for Y"** — You specified a model name that the provider doesn't recognize. Use `zag capability -p <provider> --pretty` to see available models, or use size aliases (`small`, `medium`, `large`).

**`--worktree` fails** — You must be inside a git repository. The worktree is created under `~/.zag/worktrees/`.

**`--sandbox` fails** — Docker must be installed and running. Sandbox mode uses `docker sandbox run` for microVM isolation.

**Config not taking effect** — Check which config file is being used with `zag config path`. Config is per-project (based on git repo root). CLI flags always override config.

## Architecture

```
zag (binary crate)
  CLI parsing (clap) → AgentFactory → Agent trait → subprocess
  Session logs, worktree/sandbox lifecycle, JSON mode, auto-selection

zag-lib (library crate)
  Agent trait, provider implementations, AgentBuilder API
  Config, output types, session logs, skills, process helpers
```

Each provider implementation spawns the respective CLI tool as a subprocess. The `Agent` trait defines the common interface (run, resume, cleanup, model resolution). `AgentOutput` normalizes output from all providers into a unified event stream.

## Development

```bash
make build          # Dev build
make test           # Run tests
make clippy         # Lint (zero warnings)
make fmt            # Format
make release        # Release build
```

## License

[MIT](LICENSE)
