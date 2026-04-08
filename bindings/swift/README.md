# Zag Swift Binding

Swift binding for [zag](https://github.com/niclaslindstedt/zag) — a unified CLI for AI coding agents.

## Platform support

| Platform | Local mode | Remote mode |
|----------|-----------|-------------|
| macOS 13+ | Subprocess | HTTP/WebSocket |
| iOS 16+ | — | HTTP/WebSocket |
| Linux | Subprocess | HTTP/WebSocket |

- **Local mode** spawns the `zag` CLI as a subprocess. Requires the binary on `PATH`.
- **Remote mode** connects to a `zag serve` instance via HTTP/WebSocket. No local binary needed.

## Prerequisites

- Swift 5.9+
- **Local mode**: macOS 13+ or Linux, `zag` CLI binary installed (or set via `ZAG_BIN` env var)
- **Remote mode**: A running `zag serve` instance (any platform including iOS)

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

## Quick start (local mode)

```swift
import Zag

let output = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

print(output.result ?? "")
```

## Quick start (remote mode / iOS)

Start a `zag serve` instance on a remote machine:

```bash
zag serve --generate-token
# Server running at https://0.0.0.0:2100
# Token: <your-token>
```

Then connect from Swift (works on iOS, macOS, and Linux):

```swift
import Zag

let output = try await ZagBuilder()
    .remote(url: "https://my-server:2100", token: "my-token")
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

print(output.result ?? "")
```

## Remote streaming

```swift
import Zag

let builder = ZagBuilder()
    .remote(url: "https://my-server:2100", token: "my-token")
    .provider("claude")

for try await event in builder.stream("analyze this code") {
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

## Direct remote client

For full control over the remote API:

```swift
import Zag

let client = try ZagRemoteClient(
    connection: ZagConnection(url: "https://my-server:2100", token: "my-token")
)

// Spawn a session
let spawn = try await client.spawn(SpawnParams(prompt: "analyze code", provider: "claude"))

// Stream events via WebSocket
for try await event in client.stream(spawn.sessionId) {
    // handle events
}

// Or wait and get output
_ = try await client.wait(sessionIds: [spawn.sessionId])
let output = try await client.output(spawn.sessionId)
```

## Streaming (local mode)

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
| `.file(path)` | Attach a file to the prompt (chainable) |
| `.env(key, value)` | Add an environment variable for the agent subprocess (chainable) |
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
| `.timeout(duration)` | Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. |
| `.mcpConfig(config)` | MCP server config: JSON string or file path (Claude only) |
| `.showUsage()` | Show token usage statistics (JSON output mode) |
| `.size(size)` | Set Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |
| `.verbose()` | Enable verbose output |
| `.quiet()` | Suppress non-essential output |
| `.debug()` | Enable debug logging |
| `.bin(path)` | Override the `zag` binary path (local mode only) |
| `.connection(conn)` | Set a `ZagConnection` for remote mode |
| `.remote(url:token:)` | Convenience: configure remote mode from URL and token strings |
| `.urlSession(session)` | Set a custom `URLSession` for remote requests |

## Terminal methods

| Method | Returns | Mode | Description |
|--------|---------|------|-------------|
| `.exec(prompt)` | `AgentOutput` | Local + Remote | Run non-interactively, return structured output |
| `.stream(prompt)` | `AsyncThrowingStream<Event, Error>` | Local + Remote | Stream NDJSON events |
| `.execStreaming(prompt)` | `StreamingSession` | Local only | Bidirectional streaming (Claude only) |
| `.execStreamingRemote(prompt)` | `ZagRemoteSession` | Remote only | Bidirectional streaming via WebSocket |
| `.run(prompt?)` | `Void` | Local only | Start an interactive session (inherits stdio) |
| `.resume(sessionId)` | `Void` | Local only | Resume a previous session by ID |
| `.continueLast()` | `Void` | Local only | Resume the most recent session |
| `.remoteClient()` | `ZagRemoteClient` | Remote only | Get direct access to the HTTP client |

## Remote client methods

| Method | Description |
|--------|-------------|
| `spawn(params)` | Spawn a new background session |
| `listSessions(...)` | List sessions with optional filters |
| `status(sessionId)` | Get session status |
| `events(sessionId, ...)` | Query session events |
| `output(sessionId)` | Get final output |
| `input(sessionId, message)` | Send message to running session |
| `cancel(sessionId, reason?)` | Cancel a running session |
| `collect(sessionIds:, tag:)` | Collect results from multiple sessions |
| `wait(sessionIds:, ...)` | Wait for sessions to complete |
| `stream(sessionId, filter?)` | Stream events via WebSocket |
| `subscribe(tag:, type:)` | Subscribe to events across sessions |
| `exec(params)` | Spawn + wait + output (convenience) |

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

Version checks only apply to local execution mode. Remote mode handles compatibility on the server side.

## Discovery

Static async functions for discovering available providers, models, and capabilities:

```swift
import Zag

let providers = try await ZagDiscover.listProviders()
let cap = try await ZagDiscover.getCapability(provider: "claude")
let all = try await ZagDiscover.getAllCapabilities()
let resolved = try await ZagDiscover.resolveModel(provider: "claude", model: "small")
```

| Method | Description |
|--------|-------------|
| `ZagDiscover.listProviders(bin:)` | List available provider names |
| `ZagDiscover.getCapability(provider:bin:)` | Get capabilities for a provider |
| `ZagDiscover.getAllCapabilities(bin:)` | Get capabilities for all providers |
| `ZagDiscover.resolveModel(provider:model:bin:)` | Resolve a model alias |

## How it works

- **Local mode**: Spawns the `zag` CLI as a subprocess (`zag exec -o json` or `-o stream-json`) and parses JSON/NDJSON output into typed Swift models.
- **Remote mode**: Communicates with a `zag serve` HTTP/WebSocket API. Sessions are spawned and managed via REST endpoints; real-time events are streamed via WebSocket.

Zero external dependencies — only Foundation.

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
