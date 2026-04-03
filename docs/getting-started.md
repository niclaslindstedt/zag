# Getting Started with zag

This guide walks you through installing zag and running your first AI agent sessions.

## Install

### From crates.io

```bash
cargo install zag-cli
```

### From source

```bash
git clone https://github.com/niclaslindstedt/zag.git
cd zag
cargo install --path zag-cli
```

### Verify

```bash
zag --version
```

## Install an agent CLI

zag delegates to upstream agent CLIs. You need at least one installed:

```bash
# Claude (recommended starting point)
curl -fsSL https://claude.ai/install.sh | bash

# Or any of these:
npm install -g @openai/codex          # Codex
npm install -g @anthropic-ai/gemini-cli  # Gemini
gh extension install github/gh-copilot   # Copilot
# Ollama: see https://ollama.com/download
```

## Your first exec

`zag exec` runs a one-shot prompt and prints the result:

```bash
zag exec "write a hello world program in Rust"
```

This uses Claude by default. The agent runs, prints its response, and exits.

## Your first interactive session

`zag run` starts an interactive session where you can have a back-and-forth conversation:

```bash
zag run
```

Type your prompts, and the agent responds. Press Ctrl+C to exit.

You can also start with an initial prompt:

```bash
zag run "help me refactor the auth module"
```

## Switch providers

Use `-p` to pick a different provider:

```bash
zag -p gemini exec "explain this function"
zag -p codex run
zag -p copilot exec "suggest improvements to main.rs"
zag -p ollama exec "what does this code do?"
```

## Use model size aliases

Instead of memorizing provider-specific model names, use size aliases:

```bash
zag -m small exec "quick question"   # fastest, cheapest
zag -m medium exec "analyze this"    # balanced
zag -m large run                     # most capable
```

Size aliases map to the right model for each provider automatically. See [Providers](providers.md) for the full mapping table.

## Auto-select the best provider

Let an LLM choose the optimal provider and model for your task:

```bash
zag -p auto -m auto exec "refactor the auth module"
```

## Your first orchestration

Spawn multiple agents in parallel and collect their results:

```bash
# Spawn two agents with a shared tag
sid1=$(zag spawn --tag review -p claude "review auth module")
sid2=$(zag spawn --tag review -p gemini "review test coverage")

# Wait for both to finish
zag wait --tag review --timeout 5m

# Collect results
zag collect --tag review
```

See `zag man orchestration` for the full orchestration guide, or the [orchestration examples](../examples/orchestration/).

## Session management

Every session gets a unique ID. You can name sessions for easy discovery:

```bash
# Create a named session
zag exec --name my-task --tag backend "implement the API"

# List sessions
zag session list
zag session list --tag backend

# Resume a session
zag run --resume <session-id>
zag run --continue  # resume the most recent
```

## JSON output

Request structured JSON output from any provider:

```bash
zag exec --json "list 3 programming languages"

# With schema validation
zag exec --json-schema '{"type":"object","required":["languages"]}' "list 3 languages"
```

## Isolation modes

Run agents in isolated environments:

```bash
# Git worktree isolation
zag -w exec "experiment with a new approach"

# Docker sandbox isolation
zag --sandbox exec "run untrusted code"
```

## Next steps

- [Providers](providers.md) -- Feature comparison and model recommendations
- [Configuration](configuration.md) -- Customize defaults with config files
- [Events & Logging](events-and-logging.md) -- Understand the NDJSON event format
- [Troubleshooting](troubleshooting.md) -- Common issues and solutions
- `zag man <command>` -- Built-in manual pages for every command
- [Examples](../examples/) -- Complete example projects
