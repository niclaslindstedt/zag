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
| `.json()` | Request JSON output |
| `.jsonSchema(schema)` | Validate output against a JSON schema (implies `.json()`) |
| `.jsonStream()` | Enable streaming NDJSON output |
| `.worktree(name?)` | Run in an isolated git worktree |
| `.sandbox(name?)` | Run in a Docker sandbox |
| `.sessionId(uuid)` | Use a specific session ID |
| `.outputFormat(fmt)` | Set output format (`"text"`, `"json"`, `"json-pretty"`, `"stream-json"`) |
| `.inputFormat(fmt)` | Set input format (`"text"`, `"stream-json"` — Claude only) |
| `.replayUserMessages()` | Re-emit user messages on stdout (Claude only) |
| `.includePartialMessages()` | Include partial message chunks (Claude only) |
| `.maxTurns(n)` | Set the maximum number of agentic turns |
| `.mcpConfig(config)` | MCP server config: JSON string or file path (Claude only) |
| `.showUsage()` | Show token usage statistics (JSON output mode) |
| `.size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output (suspend) |
| `.stream(prompt)` | `Flow<Event>` | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only) |
| `.run(prompt?)` | `Unit` | Start an interactive session (inherits stdio, suspend) |
| `.resume(sessionId)` | `Unit` | Resume a previous session by ID (suspend) |
| `.continueLast()` | `Unit` | Resume the most recent session (suspend) |

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
