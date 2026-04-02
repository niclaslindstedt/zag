# Configuration

zag uses TOML config files for persistent settings. CLI flags always take precedence over config values.

## Config file locations

| Scope | Path |
|-------|------|
| Project (git repo) | `~/.zag/projects/<sanitized-path>/zag.toml` |
| Global (fallback) | `~/.zag/zag.toml` |

The project path is derived from your git repo root (or `--root` flag). The path is sanitized by stripping the leading `/` and replacing `/` with `-`. For example, `/home/user/myproject` becomes `home-user-myproject`.

Outside of a git repository, the global config at `~/.zag/zag.toml` is used.

Check which config file is active:

```bash
zag config path
```

## Precedence

Settings are resolved in this order (highest priority first):

1. CLI flags (e.g., `--provider claude`)
2. Config file values
3. Built-in defaults

## Viewing and setting config

```bash
# View all config
zag config

# Get a single value
zag config provider

# Set a value
zag config provider gemini
zag config auto_approve true
zag config model.claude opus

# Unset a value (revert to default)
zag config unset provider
```

## Complete config reference

```toml
[defaults]
provider = "claude"       # Default provider: claude, codex, gemini, copilot, ollama, auto
model = "medium"          # Default model name or size alias: small, medium, large, auto
auto_approve = false      # Skip permission prompts
max_turns = 10            # Maximum number of agentic turns (optional)
system_prompt = ""        # Default system prompt appended to all sessions (optional)

[models]
# Per-provider model overrides (optional)
claude = "opus"           # Default model when using Claude
codex = "gpt-5.4"        # Default model when using Codex
gemini = "gemini-2.5-pro" # Default model when using Gemini
copilot = "claude-sonnet-4.5"  # Default model when using Copilot
ollama = "qwen3.5"       # Default model when using Ollama

[auto]
provider = "claude"       # Provider used for auto-selection decisions
model = "sonnet"          # Model used for auto-selection decisions

[ollama]
model = "qwen3.5"        # Ollama model name
size = "9b"              # Default parameter size
size_small = "2b"        # Size for the "small" alias
size_medium = "9b"       # Size for the "medium" alias
size_large = "35b"       # Size for the "large" alias

[listen]
format = "text"          # Default output format for `zag listen`: text, json, rich-text
timestamp_format = "%H:%M:%S"  # strftime-style timestamp format
```

## Valid config keys

For use with `zag config <key> [value]`:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `provider` | string | (none) | Default provider |
| `model` | string | (none) | Default model or size alias |
| `auto_approve` | bool | false | Skip permission prompts |
| `max_turns` | u32 | (none) | Max agentic turns |
| `system_prompt` | string | (none) | Default system prompt |
| `model.claude` | string | (none) | Claude-specific default model |
| `model.codex` | string | (none) | Codex-specific default model |
| `model.gemini` | string | (none) | Gemini-specific default model |
| `model.copilot` | string | (none) | Copilot-specific default model |
| `model.ollama` | string | (none) | Ollama-specific default model |
| `auto.provider` | string | claude | Auto-selection provider |
| `auto.model` | string | sonnet | Auto-selection model |
| `ollama.model` | string | qwen3.5 | Ollama model name |
| `ollama.size` | string | 9b | Default Ollama parameter size |
| `ollama.size_small` | string | 2b | Ollama small alias size |
| `ollama.size_medium` | string | 9b | Ollama medium alias size |
| `ollama.size_large` | string | 35b | Ollama large alias size |
| `listen.format` | string | (none) | Listen output format |
| `listen.timestamp_format` | string | %H:%M:%S | Listen timestamp format |

## Example configs

### Minimal

```toml
[defaults]
provider = "claude"
```

### Multi-provider setup

```toml
[defaults]
provider = "claude"
model = "medium"
auto_approve = false

[models]
claude = "sonnet"
codex = "gpt-5.4"
gemini = "gemini-2.5-flash"

[ollama]
model = "qwen3.5"
size = "9b"
```

### Team setup with auto-selection

```toml
[defaults]
provider = "auto"
model = "auto"
max_turns = 20
system_prompt = "Follow our team coding standards in CONTRIBUTING.md"

[auto]
provider = "claude"
model = "sonnet"
```

## Environment variables

zag sets these environment variables during agent sessions:

| Variable | Description | Example |
|----------|-------------|---------|
| `ZAG_SESSION_ID` | Unique session identifier | UUID string |
| `ZAG_SESSION_NAME` | Human-readable session name | `my-task` |
| `ZAG_PROVIDER` | Current provider | `claude` |
| `ZAG_MODEL` | Current model | `sonnet` |
| `ZAG_PROCESS_ID` | Process identifier (orchestration) | UUID string |
| `ZAG_ROOT` | Worktree path (if using `-w`) | `/path/to/worktree` |
| `NO_COLOR` | Disable colored output (respected by zag) | (any value) |

Provider-specific API keys (e.g., `ANTHROPIC_API_KEY`) are read by the upstream CLI tools, not by zag directly.

## Related

- `zag man config` -- Config command reference
- [Providers](providers.md) -- Provider-specific settings
