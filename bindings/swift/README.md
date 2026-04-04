# Zag Swift Binding

Swift binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Prerequisites

- Swift 5.9+
- macOS 13+ or Linux
- The `zag` CLI binary installed and on your `PATH` (or set via `ZAG_BIN` env var)

## Installation

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/niclaslindstedt/zag.git", from: "0.2.4"),
],
targets: [
    .target(
        name: "YourTarget",
        dependencies: [.product(name: "Zag", package: "zag")],
        path: "Sources/YourTarget"
    ),
]
```

## Quick start

```swift
import Zag

let output = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

print(output.result ?? "")
```

## Streaming

```swift
import Zag

for try await event in ZagBuilder().provider("claude").stream("analyze code") {
    switch event {
    case .assistantMessage(let msg):
        print(msg.content)
    case .toolExecution(let tool):
        print("\(tool.toolName): \(tool.result)")
    default:
        break
    }
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
| `.showUsage()` | Show token usage statistics (JSON output mode) |
| `.size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path |

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.exec(prompt)` | `AgentOutput` | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncThrowingStream<Event, Error>` | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Bidirectional streaming (Claude only) |
| `.run(prompt?)` | `Void` | Start an interactive session (inherits stdio) |
| `.resume(sessionId)` | `Void` | Resume a previous session by ID |
| `.continueLast()` | `Void` | Resume the most recent session |

## How it works

The SDK spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses the JSON/NDJSON output into typed Swift models. Zero external dependencies — only Foundation.

## Testing

```bash
cd bindings/swift && swift test
```

## See also

- [TypeScript SDK](../typescript/README.md)
- [Python SDK](../python/README.md)
- [C# SDK](../csharp/README.md)
- [Rust API (zag-agent)](../../zag-agent/README.md)
- [All bindings](../README.md)

## License

[MIT](../../LICENSE)
