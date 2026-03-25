# zag config

View or set configuration values.

## Synopsis

    zag [flags] config [key] [value]
    zag [flags] config key=value

## Description

Manages the `zag.toml` configuration file. When called with no arguments, prints the full config file. When called with a key and value, sets that configuration option.

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

    provider          Default provider: claude, codex, gemini, copilot, auto
                      Default: "claude"

    model             Default model size for all agents: small, medium, large,
                      or a specific model name. Default: "medium"

    auto_approve      Skip permission prompts by default: true, false, yes, no, 1, 0
                      Default: false

    model.claude      Default model for Claude (overrides model)
    model.codex       Default model for Codex (overrides model)
    model.gemini      Default model for Gemini (overrides model)
    model.copilot     Default model for Copilot (overrides model)

    auto.provider     Provider for the auto-selection LLM call
                      Default: "claude"

    auto.model        Model for the auto-selection LLM call
                      Default: "sonnet"

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

    [models]
    claude = "opus"
    codex = "gpt-5.4"
    gemini = "auto"
    copilot = "claude-sonnet-4.5"

    [auto]
    provider = "claude"
    model = "sonnet"

## Examples

    zag config                          Print full config file
    zag config provider gemini          Set default provider to Gemini
    zag config provider=gemini          Same (equals syntax)
    zag config model large              Set default model size
    zag config model.claude opus        Set Claude-specific model
    zag config model.claude=opus        Same (equals syntax)
    zag config auto_approve true        Enable auto-approve by default
    zag config auto.model haiku         Use haiku for auto-selection

## See Also

    zag man agent    Global flags and providers overview
