# agent capability

Show capability declarations for a provider.

## Synopsis

    agent [-p PROVIDER] capability [--format FORMAT] [--pretty]

## Description

Outputs a structured description of the capabilities supported by a given provider. Each capability indicates whether it is supported and whether the support is native to the provider or implemented by the wrapper.

If `--provider` is not specified, the default provider is used (from config or `claude`).

## Options

    -f, --format FORMAT    Output format: json, yaml, toml (default: json)
        --pretty           Pretty-print output (applies to JSON; yaml and toml
                           are always human-readable)

## Output Structure

The output contains:

    provider           Provider name
    default_model      Default model for the provider
    available_models   List of accepted model names (or sizes for Ollama)
    size_mappings      Mapping of small/medium/large aliases to model names
    features           Feature capability declarations

Each feature in `features` is an object with:

    supported          Whether the feature works with this provider
    native             Whether the provider implements it natively (true) or
                       the wrapper provides it (false)

The `session_logs` feature has an additional field:

    completeness       "full" or "partial" (only present when supported)

## Features

    interactive          Start an interactive session (run)
    non_interactive      Run non-interactively (exec)
    resume               Resume a previous session
    resume_with_prompt   Resume with a new prompt (for retry/correction)
    session_logs         Harmonized session log ingestion
    json_output          Structured JSON output
    stream_json          Streaming JSON events (NDJSON)
    json_schema          JSON schema validation
    input_format         Input format control (e.g., stream-json)
    worktree             Git worktree isolation
    sandbox              Docker sandbox isolation
    system_prompt        Custom system prompt
    auto_approve         Skip permission prompts
    review               Code review
    add_dirs             Additional directories

## Examples

    agent capability                          Show capabilities for default provider
    agent -p claude capability                Show Claude capabilities
    agent -p ollama capability --pretty       Pretty-print Ollama capabilities
    agent -p gemini capability -f yaml        Show Gemini capabilities as YAML
    agent -p codex capability -f toml         Show Codex capabilities as TOML

## See Also

    agent man agent    Global flags and providers overview
