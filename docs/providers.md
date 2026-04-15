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
| Native JSON schema | Yes | Yes | No | No | No |
| MCP servers | Yes | Yes | Yes | Yes | No |
| Worktree isolation | Yes | Yes | Yes | Yes | Yes |
| Docker sandbox | Yes | Yes | Yes | Yes | Yes |
| Max turns | Yes | Yes | No | Yes | No |
| System prompt | Yes | Yes | Yes | Yes | Yes |
| Auto-approve | Yes | Yes | Yes | Yes | N/A |
| Interactive spawn (`spawn -I`) | Yes | No | No | No | No |

### Streaming and MCP flag support

Four builder flags that toggle streaming I/O details and per-invocation MCP configuration are only honored by the Claude provider. Passing them to any other provider is a no-op.

| Flag / Builder method | Claude | Codex | Gemini | Copilot | Ollama |
|-----------------------|--------|-------|--------|---------|--------|
| `--input-format` / `inputFormat()` | Yes | No | No | No | No |
| `--replay-user-messages` / `replayUserMessages()` | Yes | No | No | No | No |
| `--include-partial-messages` / `includePartialMessages()` | Yes | No | No | No | No |
| `--mcp-config` / `mcpConfig()` | Yes | No | No | No | No |

`execStreaming()` is Claude-only and always sets `--input-format stream-json`, `--output-format stream-json`, and `--replay-user-messages`. By default it emits **one `assistant_message` event per complete assistant turn** — you get one event when the model finishes speaking, not a stream of token chunks. Call `.includePartialMessages(true)` (or pass `--include-partial-messages` on the CLI) to receive per-token partial `assistant_message` chunks instead. The default stays `false` so existing callers that render whole-turn bubbles aren't broken.

At the end of every agent turn the session emits a **`turn_complete`** event carrying the provider's `stop_reason` (`end_turn`, `tool_use`, `max_tokens`, `stop_sequence`, or `null`), a zero-based monotonic `turn_index`, and the turn's `usage`. A per-turn `result` event fires immediately after. Prefer `turn_complete` as the turn-boundary signal in new code — it is emitted in ordering-guaranteed position (after the last `assistant_message` of the turn, before `result`) and carries richer metadata than `result`. Since `execStreaming()` is Claude-only today, only Claude currently emits `turn_complete`; other providers will gain it when they grow a bidirectional streaming path. See [Events and Logging: TurnComplete](events-and-logging.md#turncomplete) for the full ordering contract.

### Notes

- **Streaming**: Claude uses `stream-json` format natively. Codex emits NDJSON. Gemini supports output format flags. Copilot and Ollama do not support streaming in a structured format.
- **Streaming input semantics**: Only Claude exposes a bidirectional `StreamingSession`. Mid-turn calls to `send_user_message` on Claude are **queued** and delivered at the next turn boundary (they do not interrupt the in-flight turn). Other providers report `streaming_input.supported == false`. See [Streaming input: mid-turn injection semantics](sessions.md#streaming-input-mid-turn-injection-semantics) for the matrix and the `semantics` field.
- **JSON schema**: Claude supports `--json-schema` and Codex supports `--output-schema` natively. For other providers, zag injects JSON instructions into the system prompt and validates the output, retrying up to 3 times on validation failure.
- **MCP**: Ollama does not support Model Context Protocol. All other providers have native MCP support that zag manages via `zag mcp`.
- **Resume**: Claude stores session state for `--resume`. Codex tracks thread IDs for resumable sessions. Gemini supports `--resume` with session ID or "latest". Copilot supports `--resume` and `--continue` for session resume.
- **Interactive spawn**: Only Claude supports long-lived interactive sessions via `zag spawn --interactive`. The session stays alive and accepts messages via `zag input` until explicitly cancelled.

## Available models

### Claude

default, sonnet, opus, haiku

### Codex

gpt-5.4, gpt-5.4-mini, gpt-5.3-codex-spark, gpt-5.3-codex, gpt-5-codex, gpt-5.2-codex, gpt-5.2, o4-mini, gpt-5.1-codex-max, gpt-5.1-codex-mini

### Gemini

auto, gemini-3.1-pro-preview, gemini-3.1-flash-lite-preview, gemini-3-pro-preview, gemini-3-flash-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite

### Copilot

claude-sonnet-4.6, claude-haiku-4.5, claude-opus-4.6, claude-sonnet-4.5, claude-opus-4.5, gpt-5.4, gpt-5.4-mini, gpt-5.3-codex, gpt-5.2-codex, gpt-5.2, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.1, gpt-5, gpt-5.1-codex-mini, gpt-5-mini, gpt-4.1, gemini-3.1-pro-preview, gemini-3-pro-preview

### Ollama

Any model from [ollama.com](https://ollama.com) with a size suffix. Available sizes: 0.8b, 2b, 4b, 9b, 27b, 35b, 122b. The default model is `qwen3.5`.

Use `zag discover -p <provider>` for a summary of any provider, `zag discover -p <provider> --models` for just the model list, or `zag discover -p <provider> --resolve large` to see which concrete model a size alias maps to. `zag discover` without a provider prints a summary table across all available providers. `zag capability -p <provider> --pretty` returns the raw capability JSON used by the bindings.

## Choosing a provider

| Use case | Recommended provider | Why |
|----------|---------------------|-----|
| General-purpose coding | claude | Best overall code quality, native JSON schema, full feature support |
| Quick tasks / cost-sensitive | claude -m small | Haiku is fast and inexpensive |
| Deep reasoning | claude -m large or gemini -m large | Most capable models |
| OpenAI ecosystem | codex | Native GPT model access |
| Local / private | ollama | Runs entirely on your machine, no API keys needed |
| Multi-provider comparison | Use `spawn` with different `-p` flags | Run the same task across providers and compare |

## Provider downgrade

When you run `zag` without `-p`, `zag` walks a tier list and falls back to the next provider if the configured or default one isn't usable. This covers two cases:

1. **Missing binary** — The provider's CLI isn't in `PATH`. zag's pre-flight check (`preflight::check_binary`) catches this.
2. **Startup probe failure** — The binary exists but can't actually start (e.g. Gemini with a broken install or missing credentials). Each provider can implement a cheap `probe()` check; Gemini runs `gemini --version` with a short timeout.

Each downgrade is logged as a warning so you can see exactly which provider ended up running and why:

```
! Downgrading provider: gemini → copilot ('gemini' CLI not found in PATH. Install: npm install -g @anthropic-ai/gemini-cli)
```

The default tier order is `claude → codex → gemini → copilot → ollama` (defined in `PROVIDER_TIER_LIST` in `zag-agent/src/factory.rs`). The requested provider is always tried first; the rest of the tier list is tried in order afterwards, skipping duplicates.

**Pinning a provider disables fallback.** If you pass `-p gemini` and the `gemini` binary is missing, zag exits with a hard error rather than silently downgrading — so `-p` means exactly what it says. Resuming a session (`zag run --continue`, `zag run --resume <id>`) also disables fallback, because the recorded provider must match.

The configured default provider (from `zag config provider <name>` / `defaults.provider` in `zag.toml`) is treated as non-explicit and may be downgraded.

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
