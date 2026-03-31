# zag-agent (Python)

Python SDK for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- Python 3.10+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Install

```bash
pip install zag-agent
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
| `.json()` | Request JSON output |
| `.json_schema(schema)` | Validate output against a JSON schema (implies `.json()`) |
| `.json_stream()` | Enable streaming NDJSON output |
| `.worktree(name=None)` | Run in an isolated git worktree |
| `.sandbox(name=None)` | Run in a Docker sandbox |
| `.session_id(uuid)` | Use a specific session ID |
| `.output_format(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.input_format(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replay_user_messages()` | Re-emit user messages on stdout (Claude only) |
| `.include_partial_messages()` | Include partial message chunks (Claude only) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncGenerator[Event]` | Stream NDJSON events |
| `.run(prompt=None)` | `None` | Start an interactive session (inherits stdio) |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed dataclasses. Zero external dependencies — only the Python standard library.

## Testing

```bash
pip install pytest pytest-asyncio
pytest
```

## License

[MIT](../../LICENSE)
