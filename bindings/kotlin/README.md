# Zag Kotlin Binding

Kotlin binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- JDK 21+
- Gradle 8+
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Installation

**Gradle (Kotlin DSL)**

```kotlin
dependencies {
    implementation("com.github.niclaslindstedt:zag:0.1.0")
}
```

## Quick start

```kotlin
import zag.ZagBuilder

val output = ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

println(output.result)
```

## Streaming

```kotlin
import zag.ZagBuilder

ZagBuilder().provider("claude").stream("analyze code").collect { event ->
    println("${event.type}: $event")
}
```

## Builder methods

| Method | Description |
|--------|-------------|
| `.provider(name)` | Set provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `.model(name)` | Set model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `.systemPrompt(text)` | Set a system prompt |
| `.root(path)` | Set the working directory |
| `.autoApprove()` | Skip permission prompts |
| `.addDir(path)` | Add an additional directory (chainable) |
| `.file(path)` | Attach a file to the prompt (chainable) |
| `.env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
| `.json()` | Request JSON output |
| `.jsonSchema(schema)` | Validate output against a JSON schema (implies `.json()`) |
| `.worktree(name?)` | Run in an isolated git worktree |
| `.sandbox(name?)` | Run in a Docker sandbox |
| `.sessionId(uuid)` | Use a specific session ID |
| `.outputFormat(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.inputFormat(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replayUserMessages()` | Re-emit user messages on stdout (Claude only) |
| `.includePartialMessages()` | Include partial message chunks (Claude only) |
| `.maxTurns(n)` | Set the maximum number of agentic turns |
| `.timeout(duration)` | Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. |
| `.mcpConfig(config)` | MCP server config: JSON string or file path (Claude only) |
| `.showUsage()` | Show token usage statistics (JSON output mode) |
| `.size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

### Provider support for streaming / MCP flags

Four builder methods that toggle streaming I/O details and per-invocation MCP configuration are only honored by the Claude provider. Passing them to any other provider is a no-op.

| Method | Claude | Codex | Gemini | Copilot | Ollama |
|--------|--------|-------|--------|---------|--------|
| `.inputFormat()` | Yes | No | No | No | No |
| `.replayUserMessages()` | Yes | No | No | No | No |
| `.includePartialMessages()` | Yes | No | No | No | No |
| `.mcpConfig()` | Yes | No | No | No | No |

`.execStreaming()` is Claude-only and always sets `-i stream-json`, `-o stream-json`, and `--replay-user-messages`. By default it emits **one `assistant_message` event per complete assistant turn** — you get one event when the model finishes speaking, not a stream of token chunks. Call `.includePartialMessages(true)` to receive token-level partial `assistant_message` chunks instead. The default stays `false` so existing callers that render whole-turn bubbles are not broken.

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output (suspend) |
| `.stream(prompt)` | `Flow<Event>` | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only). Emits one `assistant_message` event per complete turn; pair with `.includePartialMessages(true)` for token-level chunks. |
| `.run(prompt?)` | `Unit` | Start an interactive session (inherits stdio, suspend) |
| `.resume(sessionId)` | `Unit` | Resume a previous session by ID (suspend) |
| `.continueLast()` | `Unit` | Resume the most recent session (suspend) |
| `.execResume(sessionId, prompt)` | `AgentOutput` | Resume a session non-interactively (suspend) |
| `.execContinue(prompt)` | `AgentOutput` | Resume the most recent session non-interactively (suspend) |
| `.streamResume(sessionId, prompt)` | `Flow<Event>` | Resume a session in streaming mode |
| `.streamContinue(prompt)` | `Flow<Event>` | Resume the most recent session in streaming mode |

## Version checking

The SDK automatically checks the installed `zag` CLI version before running commands. If you use a builder method that requires a newer CLI version than what's installed, a clear error is thrown:

```
env() requires zag CLI >= 0.6.0, but the installed version is 0.5.0.
Please update the zag binary.
```

The version is detected once (by running `zag --version`) and cached for the lifetime of the process.

| Method | Minimum CLI version |
|--------|-------------------|
| `.env()` | 0.6.0 |
| `.mcpConfig()` | 0.6.0 |

All other methods are available since the initial release (0.2.3).

## Discovery

Suspend functions for discovering available providers, models, and capabilities:

```kotlin
import zag.ZagDiscover

val providers = ZagDiscover.listProviders()
val cap = ZagDiscover.getCapability("claude")
val all = ZagDiscover.getAllCapabilities()
val resolved = ZagDiscover.resolveModel("claude", "small")
```

| Method | Description |
|--------|-------------|
| `ZagDiscover.listProviders(bin?)` | List available provider names |
| `ZagDiscover.getCapability(provider, bin?)` | Get capabilities for a provider |
| `ZagDiscover.getAllCapabilities(bin?)` | Get capabilities for all providers |
| `ZagDiscover.resolveModel(provider, model, bin?)` | Resolve a model alias |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed models. Uses `kotlinx.serialization` for JSON parsing and `kotlinx.coroutines` for async operations.

## Testing

```bash
./gradlew test
```

## See also

- [TypeScript SDK](../typescript/README.md)
- [Python SDK](../python/README.md)
- [C# SDK](../csharp/README.md)
- [Swift SDK](../swift/README.md)
- [Rust API (zag-agent)](../../zag-agent/README.md)
- [All bindings](../README.md)

## License

[MIT](../../LICENSE)
