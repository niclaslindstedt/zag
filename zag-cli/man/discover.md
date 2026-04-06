# zag discover

Discover available providers, models, and capabilities.

## Synopsis

    zag discover [OPTIONS]

## Description

Lists providers, their available models, capabilities, and size mappings.
Unlike `zag capability` (which shows detailed capability data for a single
provider), `discover` gives a high-level overview and supports querying
across all providers at once.

When invoked without flags, prints a human-readable summary table of all
providers. Use `--json` or `--format` for machine-readable output.

## Options

    -p, --provider PROVIDER    Filter to a specific provider
        --models               Show only model listings
        --resolve MODEL        Resolve a model alias (e.g. "default", "small")
        --json                 Output as JSON
    -f, --format FORMAT        Output format: json, yaml, toml
        --pretty               Pretty-print output (applies to JSON)

## Model Alias Resolution

The `--resolve` flag traces how model aliases are resolved for a given
provider. Size aliases are mapped to provider-specific model names:

    small, s             Smallest/fastest model
    medium, m, default   Balanced model (default tier)
    large, l, max        Most capable model

Non-alias names pass through unchanged.

## Examples

    zag discover                              Summary table of all providers
    zag discover -p claude                    Detailed view for Claude
    zag discover --models                     List all models across providers
    zag discover --models -p claude           Models for Claude only
    zag discover --resolve default -p claude  Resolve "default" → "sonnet"
    zag discover --resolve small -p codex     Resolve "small" → "gpt-5.4-mini"
    zag discover --json                       JSON output (all providers)
    zag discover --json --pretty              Pretty-printed JSON
    zag discover -f yaml                      YAML output

## Human-Readable Output

The default table shows:

    PROVIDER   DEFAULT MODEL                MODELS  RESUME  JSON   LOGS
    claude     default                           7  yes     yes    full
    codex      gpt-5.4                          10  yes     yes    partial
    gemini     auto                              8  yes     no     full
    copilot    claude-sonnet-4.6                19  yes     no     full
    ollama     qwen3.5                           7  no      yes    -

With `--provider`, a detailed view lists all models and features.

## See Also

    zag man capability   Detailed capability data for a single provider
    zag man zag          Global flags and providers overview
