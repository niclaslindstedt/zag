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
| `.env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
| `.json_mode()` | Request JSON output |
| `.json_schema(schema)` | Validate output against a JSON schema (implies `.json_mode()`) |
| `.json_stream()` | Enable streaming NDJSON output |
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

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncGenerator[Event]` | Stream NDJSON events |
| `.exec_streaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only) |
| `.run(prompt=None)` | `None` | Start an interactive session (inherits stdio) |
| `.resume(session_id)` | `None` | Resume a previous session by ID |
| `.continue_last()` | `None` | Resume the most recent session |

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
