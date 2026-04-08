# Swift Binding Reference

Complete API reference for the zag Swift binding. Covers every builder method, type definition, and execution pattern needed to integrate a system with this SDK. Includes both local mode (subprocess) and remote mode (HTTP/WebSocket).

## Quick Start (Local Mode)

```swift
import Zag

let output = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("write a hello world program")

print(output.result ?? "")
```

## Quick Start (Remote Mode)

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

**Package**: SPM from `https://github.com/niclaslindstedt/zag.git`
**Import**: `import Zag`
**Requires**: Swift 5.9+
**Dependencies**: None (Foundation only)

## Platform Support

| Platform | Local mode | Remote mode |
|----------|-----------|-------------|
| macOS 13+ | Subprocess | HTTP/WebSocket |
| iOS 16+ | -- | HTTP/WebSocket |
| Linux | Subprocess | HTTP/WebSocket |

- **Local mode** spawns the `zag` CLI as a subprocess. Requires the binary on `PATH` or `ZAG_BIN` env var.
- **Remote mode** connects to a `zag serve` instance via HTTP/WebSocket. No local binary needed.

### Installation

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/niclaslindstedt/zag.git", from: "0.2.4"),
],
targets: [
    .target(
        name: "YourTarget",
        dependencies: [.product(name: "Zag", package: "zag")]
    ),
]
```

## Builder API

Constructor: `ZagBuilder()`

All configuration methods are annotated `@discardableResult` and return `Self` for chaining. Uses camelCase. Boolean setters take no parameter (toggle on).

### Configuration Methods

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `bin` | `func bin(_ path: String) -> Self` | _(binding-only)_ | Override zag binary path (local mode only) |
| `provider` | `func provider(_ p: String) -> Self` | `-p, --provider` | Provider: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"` |
| `model` | `func model(_ m: String) -> Self` | `--model` | Model name or size alias (`"small"`, `"medium"`, `"large"`) |
| `systemPrompt` | `func systemPrompt(_ p: String) -> Self` | `--system-prompt` | System prompt for agent behavior |
| `root` | `func root(_ r: String) -> Self` | `--root` | Working directory for the agent |
| `autoApprove` | `func autoApprove() -> Self` | `--auto-approve` | Skip permission prompts |
| `addDir` | `func addDir(_ d: String) -> Self` | `--add-dir` | Add additional directory (repeatable) |
| `file` | `func file(_ path: String) -> Self` | `--file` | Attach a file to the prompt (repeatable) |
| `env` | `func env(_ key: String, _ value: String) -> Self` | `--env KEY=VALUE` | Add environment variable _(CLI >= 0.6.0)_ |
| `json` | `func json() -> Self` | `--json` | Request JSON output |
| `jsonSchema` | `func jsonSchema(_ s: String) -> Self` | `--json-schema` | JSON schema for validation (implies `json()`) |
| `jsonStream` | `func jsonStream() -> Self` | `--json-stream` | Enable NDJSON streaming output |
| `worktree` | `func worktree(_ name: String? = nil) -> Self` | `-w, --worktree [NAME]` | Git worktree isolation (auto-named if no arg) |
| `sandbox` | `func sandbox(_ name: String? = nil) -> Self` | `--sandbox [NAME]` | Docker sandbox isolation (auto-named if no arg) |
| `verbose` | `func verbose() -> Self` | `--verbose` | Enable verbose output |
| `quiet` | `func quiet() -> Self` | `--quiet` | Suppress non-essential output |
| `debug` | `func debug() -> Self` | `--debug` | Enable debug logging _(binding-only)_ |
| `sessionId` | `func sessionId(_ id: String) -> Self` | `--session UUID` | Pre-set session ID |
| `outputFormat` | `func outputFormat(_ f: String) -> Self` | `-o, --output` | Output format: `"text"`, `"json"`, `"json-pretty"`, `"stream-json"` |
| `inputFormat` | `func inputFormat(_ f: String) -> Self` | `-i, --input-format` | Input format: `"text"`, `"stream-json"` _(Claude only)_ |
| `replayUserMessages` | `func replayUserMessages() -> Self` | `--replay-user-messages` | Re-emit user messages on stdout _(Claude only)_ |
| `includePartialMessages` | `func includePartialMessages() -> Self` | `--include-partial-messages` | Include partial message chunks _(Claude only)_ |
| `maxTurns` | `func maxTurns(_ n: Int) -> Self` | `--max-turns` | Maximum number of agentic turns |
| `timeout` | `func timeout(_ t: String) -> Self` | `--timeout` | Timeout duration (e.g., `"30s"`, `"5m"`, `"1h"`). Kills agent if exceeded. |
| `mcpConfig` | `func mcpConfig(_ c: String) -> Self` | `--mcp-config` | MCP server config: JSON string or file path _(Claude only, CLI >= 0.6.0)_ |
| `showUsage` | `func showUsage() -> Self` | `--show-usage` | Show token usage statistics (JSON output mode) |
| `size` | `func size(_ s: String) -> Self` | `--size` | Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`) |

### Remote Mode Methods (binding-only)

| Method | Signature | Description |
|--------|-----------|-------------|
| `connection` | `func connection(_ conn: ZagConnection) -> Self` | Set a `ZagConnection` for remote mode |
| `remote` | `func remote(url: String, token: String) -> Self` | Convenience: configure remote mode from URL and token |
| `urlSession` | `func urlSession(_ session: URLSession) -> Self` | Custom `URLSession` for testing remote requests |

### Terminal Methods

| Method | Signature | Mode | Description |
|--------|-----------|------|-------------|
| `exec` | `func exec(_ prompt: String) async throws -> AgentOutput` | Local + Remote | Non-interactive execution |
| `stream` | `func stream(_ prompt: String) -> AsyncThrowingStream<Event, Error>` | Local + Remote | Stream NDJSON events |
| `execStreaming` | `func execStreaming(_ prompt: String) async throws -> StreamingSession` | Local only | Bidirectional streaming _(Claude only)_ |
| `execStreamingRemote` | `func execStreamingRemote(_ prompt: String) async throws -> ZagRemoteSession` | Remote only | Bidirectional streaming via WebSocket |
| `run` | `func run(_ prompt: String? = nil) async throws` | Local only | Interactive session (inherits stdio) |
| `resume` | `func resume(_ sessionId: String) async throws` | Local only | Resume previous session by ID |
| `continueLast` | `func continueLast() async throws` | Local only | Resume most recent session |
| `remoteClient` | `func remoteClient() throws -> ZagRemoteClient` | Remote only | Get direct access to the HTTP/WebSocket client |

## StreamingSession (Local)

Returned by `execStreaming()`. Provides bidirectional communication with the local agent process.

```swift
class StreamingSession {
    /// Send a raw NDJSON line to the agent's stdin.
    func send(_ message: String) throws

    /// Send a user message to the agent (serializes to NDJSON).
    func sendUserMessage(_ content: String) throws

    /// Close stdin to signal no more input.
    func closeInput()

    /// Async stream of parsed Event objects from stdout.
    func events() -> AsyncThrowingStream<Event, Error>

    /// Whether the child process is still running.
    var isRunning: Bool { get }

    /// Send SIGTERM to the child process.
    func terminate()

    /// Wait for the process to exit. Throws ZagError on non-zero exit.
    func wait() async throws
}
```

## Remote Client

Returned by `remoteClient()`. Full HTTP/WebSocket access to a `zag serve` instance.

```swift
class ZagRemoteClient {
    /// Spawn a new background session.
    func spawn(_ params: SpawnParams) async throws -> SpawnResult

    /// List sessions with optional filters.
    func listSessions(...) async throws -> [SessionInfo]

    /// Get session status.
    func status(_ sessionId: String) async throws -> SessionStatus

    /// Query session events.
    func events(_ sessionId: String, ...) async throws -> [Event]

    /// Get final session output.
    func output(_ sessionId: String) async throws -> AgentOutput

    /// Send a message to a running session.
    func input(_ sessionId: String, message: String) async throws

    /// Cancel a running session.
    func cancel(_ sessionId: String, reason: String? = nil) async throws

    /// Collect results from multiple sessions.
    func collect(sessionIds: [String]?, tag: String?) async throws -> [AgentOutput]

    /// Wait for sessions to complete.
    func wait(sessionIds: [String], ...) async throws -> [WaitResult]

    /// Stream events via WebSocket.
    func stream(_ sessionId: String, filter: String? = nil) -> AsyncThrowingStream<Event, Error>

    /// Subscribe to events across sessions.
    func subscribe(tag: String?, type: String?) -> AsyncThrowingStream<Event, Error>

    /// Convenience: spawn + wait + output.
    func exec(_ params: SpawnParams) async throws -> AgentOutput
}
```

### ZagConnection

```swift
struct ZagConnection {
    let url: String      // Server URL (e.g., "https://my-server:2100")
    let token: String    // Authentication token
}
```

### SpawnParams

```swift
struct SpawnParams {
    var prompt: String
    var provider: String?
    var model: String?
    var systemPrompt: String?
    var root: String?
    var autoApprove: Bool
    var maxTurns: Int?
    // ... mirrors builder configuration
}
```

## Types

### AgentOutput

```swift
struct AgentOutput: Codable {
    let agent: String                   // Agent/provider name
    let sessionId: String               // Session UUID
    let events: [Event]                 // All session events
    let result: String?                 // Final result text
    let isError: Bool                   // Whether session ended in error
    let exitCode: Int?                  // Process exit code
    let errorMessage: String?           // Error message if isError
    let totalCostUsd: Double?           // Cost in USD (if available)
    let usage: Usage?                   // Aggregate token usage
}
```

### Usage

```swift
struct Usage: Codable {
    let inputTokens: Int
    let outputTokens: Int
    let cacheReadTokens: Int?           // Claude-specific
    let cacheCreationTokens: Int?       // Claude-specific
    let webSearchRequests: Int?         // Gemini-specific
    let webFetchRequests: Int?          // Gemini-specific
}
```

### Events

Events are a Codable enum discriminated on the `type` field.

```swift
enum Event: Codable {
    case `init`(InitEvent)
    case userMessage(UserMessageEvent)
    case assistantMessage(AssistantMessageEvent)
    case toolExecution(ToolExecutionEvent)
    case result(ResultEvent)
    case error(ErrorEvent)
    case permissionRequest(PermissionRequestEvent)
}

struct InitEvent: Codable {
    let model: String                   // Model used
    let tools: [String]                 // Available tool names
    let workingDirectory: String?       // Agent working directory
    let metadata: [String: AnyCodable]  // Provider-specific metadata
}

struct UserMessageEvent: Codable {
    let content: [ContentBlock]         // User message content
}

struct AssistantMessageEvent: Codable {
    let content: [ContentBlock]         // Assistant response content
    let usage: Usage?                   // Token usage for this message
}

struct ToolExecutionEvent: Codable {
    let toolName: String                // Tool that was invoked
    let toolId: String                  // Unique invocation ID
    let input: AnyCodable?              // Tool input parameters
    let result: ToolResult              // Tool execution result
}

struct ResultEvent: Codable {
    let success: Bool                   // Whether session succeeded
    let message: String?                // Final result message
    let durationMs: Int?                // Total duration in milliseconds
    let numTurns: Int?                  // Number of agentic turns
}

struct ErrorEvent: Codable {
    let message: String                 // Error message
    let details: AnyCodable?            // Additional error details
}

struct PermissionRequestEvent: Codable {
    let toolName: String                // Tool requesting permission
    let description: String             // What the tool wants to do
    let granted: Bool                   // Whether permission was granted
}
```

### Content Blocks

```swift
enum ContentBlock: Codable {
    case text(TextBlock)
    case toolUse(ToolUseBlock)
}

struct TextBlock: Codable {
    let text: String
}

struct ToolUseBlock: Codable {
    let id: String                      // Tool use ID
    let name: String                    // Tool name
    let input: AnyCodable?              // Tool input
}
```

### ToolResult

```swift
struct ToolResult: Codable {
    let success: Bool
    let output: String?
    let error: String?
    let data: AnyCodable?
}
```

### ZagError

```swift
struct ZagError: Error {
    let message: String
    let exitCode: Int?
    let stderr: String
}
```

### Discovery Types

```swift
struct FeatureSupport: Codable {
    let supported: Bool
    let native: Bool
}

struct SessionLogSupport: Codable {
    let supported: Bool
    let native: Bool
    let completeness: String?
}

struct SizeMappings: Codable {
    let small: String
    let medium: String
    let large: String
}

struct Features: Codable {
    let interactive: FeatureSupport
    let nonInteractive: FeatureSupport
    let resume: FeatureSupport
    let resumeWithPrompt: FeatureSupport
    let sessionLogs: SessionLogSupport
    let jsonOutput: FeatureSupport
    let streamJson: FeatureSupport
    let jsonSchema: FeatureSupport
    let inputFormat: FeatureSupport
    let streamingInput: FeatureSupport
    let worktree: FeatureSupport
    let sandbox: FeatureSupport
    let systemPrompt: FeatureSupport
    let autoApprove: FeatureSupport
    let review: FeatureSupport
    let addDirs: FeatureSupport
    let maxTurns: FeatureSupport
}

struct ProviderCapability: Codable {
    let provider: String
    let defaultModel: String
    let availableModels: [String]
    let sizeMappings: SizeMappings
    let features: Features
}

struct ResolvedModel: Codable {
    let input: String
    let resolved: String
    let isAlias: Bool
    let provider: String
}
```

## Discovery API

Static async functions for querying available providers and models.

```swift
import Zag

// Function signatures
enum ZagDiscover {
    static func listProviders(bin: String? = nil) async throws -> [String]
    static func getCapability(provider: String, bin: String? = nil) async throws -> ProviderCapability
    static func getAllCapabilities(bin: String? = nil) async throws -> [ProviderCapability]
    static func resolveModel(provider: String, model: String, bin: String? = nil) async throws -> ResolvedModel
}
```

## Examples

### Local non-interactive execution

```swift
let output = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .root("/path/to/project")
    .autoApprove()
    .maxTurns(10)
    .exec("refactor the auth module")

if output.isError {
    print(output.errorMessage ?? "Unknown error")
} else {
    print(output.result ?? "")
    print("Cost: $\(output.totalCostUsd ?? 0)")
}
```

### Remote non-interactive execution

```swift
let output = try await ZagBuilder()
    .remote(url: "https://my-server:2100", token: "my-token")
    .provider("claude")
    .model("sonnet")
    .autoApprove()
    .exec("analyze the codebase")

print(output.result ?? "")
```

### Local streaming

```swift
for try await event in ZagBuilder()
    .provider("claude")
    .stream("analyze this codebase") {
    switch event {
    case .assistantMessage(let msg):
        for block in msg.content {
            if case .text(let t) = block {
                print(t.text)
            }
        }
    case .toolExecution(let tool):
        print("Tool: \(tool.toolName) -> \(tool.result.output ?? "")")
    case .result(let r):
        print("Done in \(r.durationMs ?? 0)ms")
    default:
        break
    }
}
```

### Remote streaming

```swift
let builder = ZagBuilder()
    .remote(url: "https://my-server:2100", token: "my-token")
    .provider("claude")

for try await event in builder.stream("analyze this code") {
    switch event {
    case .assistantMessage(let msg):
        print(msg.content)
    default:
        break
    }
}
```

### Bidirectional streaming (local, Claude only)

```swift
let session = try await ZagBuilder()
    .provider("claude")
    .execStreaming("start a conversation")

// Send additional messages
try session.sendUserMessage("now do something else")

// Read events
for try await event in session.events() {
    print(event)
}

// Wait for completion
try await session.wait()
```

### Direct remote client

```swift
let client = try ZagBuilder()
    .remote(url: "https://my-server:2100", token: "my-token")
    .remoteClient()

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

### JSON schema output

```swift
let output = try await ZagBuilder()
    .provider("claude")
    .jsonSchema("""
    {
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "issues": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["summary", "issues"]
    }
    """)
    .exec("analyze code quality")

if let json = output.result {
    print(json) // Structured JSON matching schema
}
```

### Error handling

```swift
import Zag

do {
    let output = try await ZagBuilder()
        .provider("claude")
        .exec("do something")
} catch let error as ZagError {
    print("Exit code: \(error.exitCode ?? -1)")
    print("Stderr: \(error.stderr)")
}
```

### Discovery

```swift
let providers = try await ZagDiscover.listProviders()
// ["claude", "codex", "gemini", "copilot", "ollama"]

let cap = try await ZagDiscover.getCapability(provider: "claude")
print(cap.defaultModel)              // "sonnet"
print(cap.availableModels)           // ["opus", "sonnet", "haiku", ...]
print(cap.features.worktree)         // FeatureSupport(supported: true, native: true)

let resolved = try await ZagDiscover.resolveModel(provider: "claude", model: "small")
// ResolvedModel(input: "small", resolved: "haiku", isAlias: true, provider: "claude")
```

## Internals

### How it works

- **Local mode**: Spawns the `zag` CLI as a subprocess via `Process` (Foundation) and parses JSON/NDJSON output into Codable Swift models.
- **Remote mode**: Communicates with a `zag serve` HTTP/WebSocket API via `URLSession`. Sessions are spawned and managed via REST endpoints; real-time events are streamed via `URLSessionWebSocketTask`.

### CLI argument construction (local mode)

Arguments are split into two groups:

**Global args** (before the subcommand): `--provider`, `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir`, `--file`, `--env`, `-w`/`--worktree`, `--sandbox`, `--verbose`, `--quiet`, `--debug`, `--session`, `--max-turns`, `--mcp-config`, `--show-usage`, `--size`

**Exec args** (after `exec`): `--json`, `--json-schema`, `--json-stream`, `-o`/`--output`, `-i`/`--input-format`, `--replay-user-messages`, `--include-partial-messages`, `--timeout`

### Default behaviors

- `exec()` automatically adds `-o json` (local) or uses POST /exec (remote) for structured output.
- `stream()` uses `--json-stream` (local) or WebSocket (remote) for NDJSON events.
- `execStreaming()` forces `-i stream-json`, `-o stream-json`, and `--replay-user-messages` for bidirectional communication.
- `run()` inherits stdin/stdout/stderr for interactive terminal use (local only).
- `resume()` dispatches to `run --resume <id>` (local only).
- `continueLast()` dispatches to `run --continue` (local only).

### Worktree and sandbox

Uses `IsolationOption` enum internally:
- `.enabled` -- flag-only, auto-generated name
- `.named(String)` -- explicit name

### Version checking

Version checks apply only to local execution mode. The SDK checks the installed `zag` CLI version (via `zag --version`) once per process. Remote mode handles compatibility on the server side.

| Method | Minimum CLI version |
|--------|-------------------|
| `env()` | 0.6.0 |
| `mcpConfig()` | 0.6.0 |
| All others | 0.2.3 |

## Provider-Specific Notes

- **Claude only**: `inputFormat()`, `replayUserMessages()`, `includePartialMessages()`, `mcpConfig()`, `execStreaming()`
- **Ollama only**: `size()`
- Size aliases (`"small"`, `"medium"`, `"large"`) are resolved by the CLI to provider-specific model names.
- Providers: `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`. Use `"auto"` for automatic provider selection.
