# Providers

zag supports five AI coding agent providers. Each provider wraps its respective CLI tool as a subprocess.

## Provider overview

| Provider | CLI binary | Default model | Install |
|----------|-----------|---------------|---------|
| **claude** | `claude` | default | `curl -fsSL https://claude.ai/install.sh \| bash` |
| **codex** | `codex` | gpt-5.4 | `npm install -g @openai/codex` |
| **gemini** | `gemini` | auto | `npm install -g @anthropic-ai/gemini-cli` |
| **copilot** | `copilot` | claude-sonnet-4.6 | `npm install -g @github/copilot` |
| **ollama** | `ollama` | qwen3.5:9b | [ollama.com/download](https://ollama.com/download) |

## Model size aliases

Size aliases let you write `zag -m large exec "..."` and get the right model regardless of provider.

| Provider | small | medium | large |
|----------|-------|--------|-------|
| **claude** | haiku | sonnet | default |
| **codex** | gpt-5.4-mini | gpt-5.3-codex | gpt-5.4 |
| **gemini** | gemini-3.1-flash-lite-preview | gemini-2.5-flash | gemini-3.1-pro-preview |
| **copilot** | claude-haiku-4.5 | claude-sonnet-4.6 | claude-opus-4.6 |
| **ollama** | 2b | 9b | 35b |

For Claude, `default` delegates model selection to the Claude CLI itself. For Ollama, sizes refer to parameter counts and can be used with any model from the Ollama registry.

## Feature matrix

| Feature | Claude | Codex | Gemini | Copilot | Ollama |
|---------|--------|-------|--------|---------|--------|
| Interactive sessions (`run`) | Yes | Yes | Yes | Yes | Yes |
| Non-interactive (`exec`) | Yes | Yes | Yes | Yes | Yes |
| Streaming output | Yes | Yes | Yes | No | No |
| Session resume | Yes | Yes | Yes | Yes | No |
| Native JSON schema | Yes | No | No | No | No |
| MCP servers | Yes | Yes | Yes | Yes | No |
| Worktree isolation | Yes | Yes | Yes | Yes | Yes |
| Docker sandbox | Yes | Yes | Yes | Yes | Yes |
| Max turns | Yes | Yes | No | Yes | No |
| System prompt | Yes | Yes | Yes | Yes | Yes |
| Auto-approve | Yes | Yes | Yes | Yes | N/A |

### Notes

- **Streaming**: Claude uses `stream-json` format natively. Codex emits NDJSON. Gemini supports output format flags. Copilot and Ollama do not support streaming in a structured format.
- **JSON schema**: Claude supports `--json-schema` natively. For other providers, zag injects JSON instructions into the system prompt and validates the output, retrying up to 3 times on validation failure.
- **MCP**: Ollama does not support Model Context Protocol. All other providers have native MCP support that zag manages via `zag mcp`.
- **Resume**: Claude stores session state for `--resume`. Codex tracks thread IDs for resumable sessions. Gemini supports `--resume` with session ID or "latest". Copilot supports `--resume` and `--continue` for session resume.

## Available models

### Claude

default, sonnet, sonnet-4.6, opus, opus-4.6, haiku, haiku-4.5

### Codex

gpt-5.4, gpt-5.4-mini, gpt-5.3-codex-spark, gpt-5.3-codex, gpt-5-codex, gpt-5.2-codex, gpt-5.2, o4-mini, gpt-5.1-codex-max, gpt-5.1-codex-mini

### Gemini

auto, gemini-3.1-pro-preview, gemini-3.1-flash-lite-preview, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite

### Copilot

claude-sonnet-4.6, claude-haiku-4.5, claude-opus-4.6, claude-sonnet-4.5, claude-opus-4.5, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.2, gpt-5.1, gpt-5, gpt-5.1-codex-mini, gpt-5-mini, gpt-4.1, gemini-3-pro-preview

### Ollama

Any model from [ollama.com](https://ollama.com) with a size suffix. Available sizes: 0.8b, 2b, 4b, 9b, 27b, 35b, 122b. The default model is `qwen3.5`.

Use `zag capability -p <provider> --pretty` to see the current model list for any provider.

## Choosing a provider

| Use case | Recommended provider | Why |
|----------|---------------------|-----|
| General-purpose coding | claude | Best overall code quality, native JSON schema, full feature support |
| Quick tasks / cost-sensitive | claude -m small | Haiku is fast and inexpensive |
| Deep reasoning | claude -m large or gemini -m large | Most capable models |
| OpenAI ecosystem | codex | Native GPT model access |
| Local / private | ollama | Runs entirely on your machine, no API keys needed |
| Multi-provider comparison | Use `spawn` with different `-p` flags | Run the same task across providers and compare |

## Auto-selection

When you use `-p auto`, `-m auto`, or both, zag uses an LLM to analyze your task and select the best provider and/or model.

The auto-selector:

1. Takes your prompt and the list of available providers/models
2. Sends them to a selector agent (default: Claude with the Sonnet model)
3. Parses the response to extract the recommended provider and model
4. Falls back gracefully if the selector declines or returns an unparseable response

Configure the selector agent in your config file:

```toml
[auto]
provider = "claude"   # Which provider runs the selector
model = "sonnet"      # Which model runs the selector
```

## Known limitations

- **Copilot**: Does not support the `--output` flag, limiting structured output options. No streaming support.
- **Ollama**: No MCP support. System prompts must be prepended to the user prompt (no `--system` flag in the CLI). No streaming in structured format.
- **Gemini**: Session path discovery relies on scanning `~/.gemini/tmp/`.
- **Codex**: Output parsing depends on the specific NDJSON format from the Codex CLI. Log backfilling uses `~/.codex/history.jsonl`.
