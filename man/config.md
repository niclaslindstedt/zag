# agent config

View or set configuration values.

## Synopsis

    agent [flags] config [key] [value]
    agent [flags] config key=value

## Description

Manages the `.agent/agent.toml` configuration file. When called with no arguments, prints the full config file. When called with a key and value, sets that configuration option.

The config file location is determined automatically:

1. If `--root` is specified: `<root>/.agent/agent.toml`
2. If inside a git repo: `<repo-root>/.agent/agent.toml`
3. Otherwise: `~/.config/agent/.agent/agent.toml` (global)

On first use, the config file and `.agent/` directory are created automatically. If inside a git repo, `.agent/` is added to `.gitignore`.

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
    codex = "gpt-5.2-codex"
    gemini = "auto"
    copilot = "claude-sonnet-4.5"

    [auto]
    provider = "claude"
    model = "sonnet"

## Examples

    agent config                          Print full config file
    agent config provider gemini          Set default provider to Gemini
    agent config provider=gemini          Same (equals syntax)
    agent config model large              Set default model size
    agent config model.claude opus        Set Claude-specific model
    agent config model.claude=opus        Same (equals syntax)
    agent config auto_approve true        Enable auto-approve by default
    agent config auto.model haiku         Use haiku for auto-selection

## See Also

    agent man agent    Global flags and providers overview
