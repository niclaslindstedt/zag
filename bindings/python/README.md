# Zag Python Binding

Python binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- Python 3.10+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Installation

```bash
pip install zag-agent
```

For development from source:

```bash
cd bindings/python
pip install -e .
```

## Quick start

```python
from zag import ZagBuilder

output = await ZagBuilder() \
    .provider("claude") \
    .model("sonnet") \
    .auto_approve() \
    .exec("write a hello world program")

print(output.result)
```

## Streaming

```python
from zag import ZagBuilder

async for event in await ZagBuilder().provider("claude").stream("analyze code"):
    print(event.type, event)
```

## Builder methods

| Method | Description |
|--------|-------------|
| `.provider(name)` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `.model(name)` | Set model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `.system_prompt(text)` | Set a system prompt |
| `.root(path)` | Set the working directory |
| `.auto_approve()` | Skip permission prompts |
| `.add_dir(path)` | Add an additional directory (chainable) |
| `.file(path)` | Attach a file to the prompt (chainable) |
| `.env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
| `.json_mode()` | Request JSON output |
| `.json_schema(schema)` | Validate output against a JSON schema (implies `.json_mode()`) |
| `.worktree(name=None)` | Run in an isolated git worktree |
| `.sandbox(name=None)` | Run in a Docker sandbox |
| `.session_id(uuid)` | Use a specific session ID |
| `.output_format(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.input_format(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replay_user_messages()` | Re-emit user messages on stdout (Claude only) |
| `.include_partial_messages()` | Include partial message chunks (Claude only) |
| `.max_turns(n)` | Set the maximum number of agentic turns |
| `.timeout(duration)` | Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. |
| `.mcp_config(config)` | MCP server config: JSON string or file path (Claude only) |
| `.show_usage()` | Show token usage statistics (JSON output mode) |
| `.size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

### Provider support for streaming / MCP flags

Four builder methods that toggle streaming I/O details and per-invocation MCP configuration are only honored by the Claude provider. Passing them to any other provider is a no-op.

| Method | Claude | Codex | Gemini | Copilot | Ollama |
|--------|--------|-------|--------|---------|--------|
| `.input_format()` | Yes | No | No | No | No |
| `.replay_user_messages()` | Yes | No | No | No | No |
| `.include_partial_messages()` | Yes | No | No | No | No |
| `.mcp_config()` | Yes | No | No | No | No |

`.exec_streaming()` is Claude-only and always sets `-i stream-json`, `-o stream-json`, and `--replay-user-messages`. By default it emits **one `assistant_message` event per complete assistant turn** — you get one event when the model finishes speaking, not a stream of token chunks. Call `.include_partial_messages(True)` to receive token-level partial `assistant_message` chunks instead. The default stays `False` so existing callers that render whole-turn bubbles are not broken.

At the end of every agent turn the session emits a **`turn_complete`** event carrying the provider's `stop_reason` (`end_turn`, `tool_use`, `max_tokens`, `stop_sequence`, or `None`), a zero-based monotonic `turn_index`, and the turn's `usage`. A per-turn `result` event fires immediately after. New code should key turn-boundary UI off `turn_complete` — it is the authoritative signal and carries richer metadata than `result`. `result` continues to fire per-turn for backward compatibility.

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncGenerator[Event]` | Stream NDJSON events |
| `.exec_streaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only). Emits one `assistant_message` event per complete turn; pair with `.include_partial_messages(True)` for token-level chunks. |
| `.run(prompt=None)` | `None` | Start an interactive session (inherits stdio) |
| `.resume(session_id)` | `None` | Resume a previous session by ID |
| `.continue_last()` | `None` | Resume the most recent session |
| `.exec_resume(session_id, prompt)` | `AgentOutput` | Resume a session non-interactively with a follow-up prompt |
| `.exec_continue(prompt)` | `AgentOutput` | Resume the most recent session non-interactively |
| `.stream_resume(session_id, prompt)` | `AsyncGenerator[Event]` | Resume a session in streaming mode |
| `.stream_continue(prompt)` | `AsyncGenerator[Event]` | Resume the most recent session in streaming mode |

## Version checking

The SDK automatically checks the installed `zag` CLI version before running commands. If you use a builder method that requires a newer CLI version than what's installed, a clear error is raised:

```
env() requires zag CLI >= 0.6.0, but the installed version is 0.5.0.
Please update the zag binary.
```

The version is detected once (by running `zag --version`) and cached for the lifetime of the process.

| Method | Minimum CLI version |
|--------|-------------------|
| `.env()` | 0.6.0 |
| `.mcp_config()` | 0.6.0 |

All other methods are available since the initial release (0.2.3).

## Discovery

Standalone async functions for discovering available providers, models, and capabilities:

```python
from zag import list_providers, get_capability, get_all_capabilities, resolve_model

providers = await list_providers()
cap = await get_capability("claude")
all_caps = await get_all_capabilities()
resolved = await resolve_model("claude", "small")  # ResolvedModel(input="small", resolved="haiku", is_alias=True)
```

| Function | Description |
|----------|-------------|
| `list_providers(bin=None)` | List available provider names |
| `get_capability(provider, bin=None)` | Get capabilities for a provider |
| `get_all_capabilities(bin=None)` | Get capabilities for all providers |
| `resolve_model(provider, model, bin=None)` | Resolve a model alias |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed dataclasses. Zero external dependencies — only the Python standard library.

## Testing

```bash
pip install pytest pytest-asyncio
pytest
```

## See also

- [TypeScript SDK](../typescript/README.md)
- [C# SDK](../csharp/README.md)
- [Rust API (zag-agent)](../../zag-agent/README.md)
- [All bindings](../README.md)

## License

[MIT](../../LICENSE)
