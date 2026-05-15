# zag config

View or set configuration values.

## Synopsis

    zag [flags] config                    Print full config file
    zag [flags] config <key>              Get a config value
    zag [flags] config get <key>          Get a config value (explicit)
    zag [flags] config <key> <value>      Set a config value
    zag [flags] config key=value          Set a config value (equals syntax)
    zag [flags] config unset <key>        Unset a config value (revert to default)
    zag [flags] config init               Create default config file
    zag [flags] config reset              Reset config to defaults
    zag [flags] config list               List all config keys and values
    zag [flags] config path               Show config file path

## Description

Manages the `zag.toml` configuration file. When called with no arguments, prints the full config file. When called with a single key, reads and prints that value. When called with a key and value, sets that configuration option.

All configuration is stored under `~/.zag/`:

1. If `--root` is specified: `~/.zag/projects/<sanitized-root>/zag.toml`
2. If inside a git repo: `~/.zag/projects/<sanitized-repo-path>/zag.toml`
3. Otherwise: `~/.zag/zag.toml` (global)

The sanitized path is the absolute path with leading `/` stripped and `/` replaced with `-` (e.g., `/Users/me/Source/app` → `Users-me-Source-app`).

On first use, the config file and directory are created automatically.

## Arguments

    key      Config key in dot notation (see Available Keys below)
    value    Value to set

Values can be passed as two arguments (`key value`) or with equals syntax (`key=value`).

## Available Keys

    provider          Default provider: claude, codex, gemini, copilot, ollama, auto
                      Default: "claude"

    model             Default model size for all agents: small, medium, large,
                      or a specific model name. Default: "medium"

    auto_approve      Skip permission prompts by default: true, false, yes, no, 1, 0
                      Default: false

    max_turns         Default maximum number of agentic turns.
                      Must be a positive integer. No default (unlimited).

    system_prompt     Default system prompt for all agents.
                      No default (empty).

    model.claude      Default model for Claude (overrides model)
    model.codex       Default model for Codex (overrides model)
    model.gemini      Default model for Gemini (overrides model)
    model.copilot     Default model for Copilot (overrides model)
    model.ollama      Default model for Ollama (overrides model)

    auto.provider     Provider for the auto-selection LLM call
                      Default: "claude"

    auto.model        Model for the auto-selection LLM call
                      Default: "sonnet"

    ollama.model      Default Ollama model name. Default: "qwen3.5"
    ollama.size       Default Ollama parameter size. Default: "9b"
    ollama.size_small  Size for small alias. Default: "2b"
    ollama.size_medium Size for medium alias. Default: "9b"
    ollama.size_large  Size for large alias. Default: "35b"

    listen.format     Default output format for listen command: text, json, rich-text
                      Default: "text"

    listen.timestamp_format
                      Strftime-style format for timestamps in listen output.
                      Default: "%H:%M:%S"

## Configuration Priority

Settings are resolved in this order (later overrides earlier):

1. Agent built-in defaults (e.g., Claude defaults to opus)
2. `defaults.model` from config (applies to all agents)
3. `models.<agent>` from config (agent-specific override)
4. CLI `--model` flag (highest priority)

The `--provider` flag overrides `defaults.provider`, and `--auto-approve` overrides `defaults.auto_approve`.

## Config File Format

    [defaults]
    provider = "claude"
    model = "medium"
    auto_approve = false
    # max_turns = 10
    # system_prompt = ""

    [models]
    claude = "opus"
    codex = "gpt-5.4"
    gemini = "auto"
    copilot = "claude-sonnet-4.6"

    [auto]
    provider = "claude"
    model = "sonnet"

    [ollama]
    model = "qwen3.5"
    size = "9b"

    [listen]
    format = "text"
    timestamp_format = "%H:%M:%S"

    [usage_limits]
    # Detect upstream usage/rate/weekly limits and auto-resume sessions.
    enabled = true
    resume_message = "Continue"
    max_wait_secs = 86400          # 24h cap on any single wait
    default_fallback_secs = 3600   # used when the provider gave no reset time
    jitter_secs = 30
    # [usage_limits.providers.<provider>] supports `enabled`,
    # `resume_message`, `fallback_secs`, and `extra_patterns` overrides.

## Subcommands

    init     Create a default config file with commented-out settings.
             If the file already exists, prints its location without overwriting.

    reset    Delete the existing config and create a fresh default config file.

    unset    Unset a single config key, reverting it to its default value.
             The key is removed from the config file on next save.

    list     List all available config keys and their current values.
             Shows "(not set)" for unset keys.

    path     Print the resolved config file path for the current project.

    get      Read a single config value by key. Prints "(not set)" if unset.
             Equivalent to `zag config <key>` (without `get`).

## Examples

    zag config                          Print full config file
    zag config init                     Create default config file
    zag config reset                    Reset config to defaults
    zag config list                     List all keys and current values
    zag config path                     Show config file location
    zag config provider                 Read default provider value
    zag config get model.claude         Read Claude-specific model
    zag config provider gemini          Set default provider to Gemini
    zag config provider=gemini          Same (equals syntax)
    zag config model large              Set default model size
    zag config model.claude opus        Set Claude-specific model
    zag config model.claude=opus        Same (equals syntax)
    zag config auto_approve true        Enable auto-approve by default
    zag config max_turns 10             Set default max agentic turns
    zag config system_prompt "Be helpful"  Set default system prompt
    zag config unset provider           Unset default provider (revert to default)
    zag config unset model.claude       Unset Claude-specific model
    zag config auto.model haiku         Use haiku for auto-selection
    zag config ollama.model llama3      Set default Ollama model
    zag config listen.format rich-text  Set default listen output format
    zag config listen.timestamp_format "%Y-%m-%d %H:%M:%S"  Set timestamp format

## Environment Variables

| Variable | Effect |
|----------|--------|
| `ZAG_CLAUDE_ALLOW_PRINT` | Opt in to Claude's `--print` (non-interactive) mode, which consumes API tokens. Required for `zag claude` invocations that use `exec`, `--output-format`, or any other path that delegates to `claude --print`. Without it, those paths fail with a steering error pointing at `run --exit`. Recognised values: `1`, `true` (anything else, including empty / `0` / `false`, leaves the gate closed). |
| `ZAG_USER_LOG_DIR` | Override the session-log directory (set by `zag serve` in user-account mode). |
| `ZAG_PROCESS_ID` | Set inside every agent subprocess. `zag ps show self` / `zag ps kill self` resolve `self` to this value. |
| `ZAG_SESSION_ID` | Set inside every agent subprocess. Identifies the active wrapper session. |
| `ZAG_PROVIDER`, `ZAG_MODEL` | Set inside every agent subprocess so agents can introspect their own context. |

## See Also

    zag man zag      Global flags and providers overview
